use hm_eval::bench::beam::{self, ProbeKind, ProbeSet};
use hm_eval::bench::pipeline::MemoryPipeline;
use std::collections::BTreeSet;

type Mutation = (&'static str, fn(&mut ProbeSet));

#[test]
fn fixture_probe_set_covers_every_probe_kind() {
    let set = beam::fixture_probe_set().unwrap();
    assert_eq!(set.format, beam::PROBE_SET_FORMAT);
    assert_eq!(set.conversations.len(), 2);
    let messages: usize = set
        .conversations
        .iter()
        .flat_map(|conversation| conversation.sessions.iter())
        .map(|session| session.messages.len())
        .sum();
    assert_eq!(messages, 40);
    let probes: usize = set
        .conversations
        .iter()
        .map(|conversation| conversation.probes.len())
        .sum();
    assert_eq!(probes, 12);
    for conversation in &set.conversations {
        assert_eq!(conversation.sessions.len(), 3);
    }
    let kinds: BTreeSet<ProbeKind> = set
        .conversations
        .iter()
        .flat_map(|conversation| conversation.probes.iter())
        .map(|probe| probe.kind)
        .collect();
    assert_eq!(kinds.len(), 10);
}

#[test]
fn probe_set_validation_rejects_each_malformed_shape() {
    let base = beam::fixture_probe_set().unwrap();
    let mutations: [Mutation; 7] = [
        ("wrong format tag", |set| {
            set.format = "hypermind.beam-probe-set.v0".to_owned();
        }),
        ("duplicate probe id", |set| {
            let duplicate = set.conversations[0].probes[0].id.clone();
            set.conversations[1].probes[0].id = duplicate;
        }),
        ("duplicate message id", |set| {
            let duplicate = set.conversations[0].sessions[0].messages[0].id;
            set.conversations[0].sessions[2].messages[0].id = duplicate;
        }),
        ("dangling evidence id", |set| {
            set.conversations[0].probes[0].evidence_message_ids = vec![9_999];
        }),
        ("empty criteria", |set| {
            set.conversations[0].probes[0].criteria.clear();
        }),
        ("abstention probe with evidence", |set| {
            let message = set.conversations[1].sessions[0].messages[0].id;
            let probe = set.conversations[1]
                .probes
                .iter_mut()
                .find(|probe| probe.kind == ProbeKind::Abstention)
                .expect("the fixture carries an abstention probe");
            probe.evidence_message_ids = vec![message];
        }),
        ("non-abstention probe without evidence", |set| {
            set.conversations[0].probes[0].evidence_message_ids.clear();
        }),
    ];
    for (defect, mutate) in mutations {
        let mut set = base.clone();
        mutate(&mut set);
        let bytes = serde_json::to_vec(&set).unwrap();
        let error = beam::parse_probe_set(&bytes)
            .err()
            .unwrap_or_else(|| panic!("{defect} was accepted"));
        assert!(!error.to_string().is_empty(), "{defect} was not named");
    }
}

#[test]
fn probe_set_digest_is_stable_and_content_bound() {
    let set = beam::fixture_probe_set().unwrap();
    let digest = beam::probe_set_digest(&set).unwrap();
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    );
    let round_trip = beam::parse_probe_set(&serde_json::to_vec(&set).unwrap()).unwrap();
    assert_eq!(beam::probe_set_digest(&round_trip).unwrap(), digest);
    let mut changed = set.clone();
    changed.conversations[0].probes[0].criteria[0].push('.');
    assert_ne!(beam::probe_set_digest(&changed).unwrap(), digest);
}

#[test]
fn published_artifact_adapter_normalizes_rows_into_probes() {
    let artifact = serde_json::json!([
        {
            "conversation_id": "hm-adapter-check",
            "chat": [
                [
                    {"id": 11, "role": "user", "content": "The Fernwell rain gauge logs every tipping bucket pulse.", "time_anchor": "2031-04-01T08:00:00Z"},
                    {"id": 12, "role": "assistant", "content": "Recorded: the Fernwell rain gauge logs tipping bucket pulses."}
                ],
                [
                    {"id": 13, "role": "user", "content": "Priya Raman took over the Fernwell gauge maintenance rota in April."},
                    {"id": 14, "role": "assistant", "content": "Noted: Priya Raman owns the Fernwell maintenance rota."},
                    {"role": "user", "content": "   "}
                ]
            ],
            "probing_questions": {
                "information_extraction": [
                    {
                        "question": "Who owns the Fernwell gauge maintenance rota?",
                        "ideal_response": "Priya Raman owns it.",
                        "rubric": ["Names Priya Raman."],
                        "difficulty": "easy",
                        "source_chat_ids": [13, 14]
                    },
                    {
                        "question": "Which probe has no rubric and is therefore dropped?",
                        "answer": "Dropped.",
                        "source_chat_ids": [11]
                    }
                ],
                "abstention": [
                    {
                        "question": "What is the Fernwell snow depth?",
                        "ideal_answer": "The conversation records no snow depth.",
                        "rubric": "Declines to state a snow depth.",
                        "source_chat_ids": [11]
                    }
                ],
                "unmapped_category": [
                    {"question": "Ignored.", "answer": "Ignored.", "rubric": ["Ignored."], "source_chat_ids": [11]}
                ]
            }
        }
    ]);
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("conversations.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let set = beam::adapt_published_artifact(&bytes).unwrap();
    assert_eq!(set.format, beam::PROBE_SET_FORMAT);
    assert_eq!(set.source_digest, blake3::hash(&bytes).to_hex().to_string());
    assert_eq!(set.conversations.len(), 1);
    let conversation = &set.conversations[0];
    assert_eq!(conversation.id, "hm-adapter-check");
    assert_eq!(conversation.sessions.len(), 2);
    assert_eq!(conversation.sessions[1].messages.len(), 2);
    assert_eq!(conversation.probes.len(), 2);
    let extraction = conversation
        .probes
        .iter()
        .find(|probe| probe.kind == ProbeKind::InformationExtraction)
        .unwrap();
    assert_eq!(extraction.reference_answer, "Priya Raman owns it.");
    assert_eq!(extraction.evidence_message_ids, vec![13, 14]);
    assert_eq!(extraction.difficulty, "easy");
    let abstention = conversation
        .probes
        .iter()
        .find(|probe| probe.kind == ProbeKind::Abstention)
        .unwrap();
    assert!(abstention.evidence_message_ids.is_empty());
    assert_eq!(abstention.criteria, vec!["Declines to state a snow depth."]);
    let embedded = serde_json::json!([
        {
            "conversation_id": "hm-adapter-check",
            "chat": artifact[0]["chat"].clone(),
            "probing_questions": serde_json::to_string(&artifact[0]["probing_questions"]).unwrap()
        }
    ]);
    let embedded = beam::adapt_published_artifact(&serde_json::to_vec(&embedded).unwrap()).unwrap();
    assert_eq!(
        embedded.conversations[0].probes.len(),
        conversation.probes.len()
    );
}

#[tokio::test]
async fn fixture_conversations_retrieve_their_own_evidence() {
    let set = beam::fixture_probe_set().unwrap();
    let digest = beam::probe_set_digest(&set).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let mut probes = 0;
    let mut grounded = 0;
    for conversation in &set.conversations {
        let documents = beam::to_documents(conversation);
        assert_eq!(documents.len(), 20);
        let mut identities = BTreeSet::new();
        for (document, message) in documents.iter().zip(
            conversation
                .sessions
                .iter()
                .flat_map(|session| &session.messages),
        ) {
            assert_eq!(document.id, format!("{}:{}", conversation.id, message.id));
            assert_eq!(document.date, message.occurred_at);
            assert!(!document.id.is_empty());
            assert!(identities.insert(document.id.clone()));
        }
        let pipeline =
            MemoryPipeline::open(temporary.path(), &digest, &conversation.id, &documents)
                .await
                .unwrap();
        for probe in &conversation.probes {
            probes += 1;
            let context = pipeline
                .retrieve_context(&beam::to_question(conversation, probe))
                .await
                .unwrap();
            for citation in &context.provenance {
                assert!(!citation.uri.is_empty());
                assert!(citation.lsn > 0);
            }
            let evidence: BTreeSet<String> = beam::evidence_document_ids(conversation, probe)
                .into_iter()
                .collect();
            if context.provenance.iter().any(|citation| {
                citation
                    .document_ids
                    .iter()
                    .any(|document| evidence.contains(document))
            }) {
                grounded += 1;
            }
        }
        pipeline.close().await.unwrap();
    }
    assert_eq!(probes, 12);
    assert!(
        grounded >= 10,
        "only {grounded} of {probes} fixture probes retrieved their own evidence"
    );
}
