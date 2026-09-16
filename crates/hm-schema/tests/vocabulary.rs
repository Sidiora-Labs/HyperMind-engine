#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_schema::event::{
    Boundary, EventKind, MAXIMUM_VOCABULARY_ALIASES, MAXIMUM_VOCABULARY_NAME_BYTES,
    MAXIMUM_VOCABULARY_TERMS, encode_event_envelope, verify_event,
};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Retention, Sensitivity, VocabularyCategory,
    VocabularyImported, VocabularyTerm,
};

#[test]
fn vocabulary_import_round_trips_and_is_bounded() {
    let accepted = verify_event(
        &encode_event_envelope(&envelope(imported(), Authority::UserAsserted)),
        EventKind::VocabularyImported,
        Boundary::Disk,
    )
    .unwrap();
    assert_eq!(accepted.kind, EventKind::VocabularyImported);
    assert_eq!(accepted.envelope.authority, Authority::UserAsserted);
    let EventPayload::VocabularyImported(payload) = &accepted.envelope.payload else {
        panic!("vocabulary import decodes to its own payload");
    };
    assert_eq!(payload.version, 1);
    assert_eq!(payload.source_digest.len(), 32);
    assert_eq!(payload.terms.len(), 3);
    assert_eq!(payload.terms[2].category, VocabularyCategory::Relation);
    assert_eq!(
        payload.terms[1].parent_term_id.as_deref(),
        Some("term-vehicle")
    );
    assert_eq!(payload.ignored_triples, 4);

    verify_event(
        &encode_event_envelope(&envelope(imported(), Authority::UserAsserted)),
        EventKind::VocabularyImported,
        Boundary::Socket,
    )
    .unwrap();

    for (label, invalid) in rejected_imports() {
        for boundary in [Boundary::Disk, Boundary::Socket] {
            let error = verify_event(
                &encode_event_envelope(&envelope(invalid.clone(), Authority::UserAsserted)),
                EventKind::VocabularyImported,
                boundary,
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::SchemaInvalid,
                "{label} at {boundary:?}"
            );
        }
    }

    for authority in [
        Authority::DerivedInference,
        Authority::AssistantGenerated,
        Authority::ToolObserved,
        Authority::ExternalObserved,
        Authority::RuntimeFact,
    ] {
        for boundary in [Boundary::Disk, Boundary::Socket] {
            let error = verify_event(
                &encode_event_envelope(&envelope(imported(), authority)),
                EventKind::VocabularyImported,
                boundary,
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::ProtectedTypeWrite,
                "{authority:?} at {boundary:?}"
            );
        }
    }
}

#[test]
fn ledger_frames_carry_the_vocabulary_kind() {
    assert_eq!(
        hm_ledger::frame::EventKind::try_from(44u8).unwrap(),
        hm_ledger::frame::EventKind::VocabularyImported
    );
    assert_eq!(
        hm_ledger::frame::EventKind::try_from(52u8)
            .unwrap_err()
            .code,
        ErrorCode::InvalidKind
    );
    assert_eq!(
        EventKind::try_from(44u8).unwrap(),
        EventKind::VocabularyImported
    );
    assert!(EventKind::try_from(52u8).is_err());
    assert!(EventKind::VocabularyImported.is_wave_seven());
    assert_eq!(
        hm_cortex::authority::authority_for_event(EventKind::VocabularyImported),
        Authority::UserAsserted
    );
}

fn rejected_imports() -> Vec<(&'static str, EventPayload)> {
    let mut cases = Vec::new();

    cases.push((
        "empty vocabulary_id",
        mutate(|value| value.vocabulary_id.clear()),
    ));
    cases.push(("zero version", mutate(|value| value.version = 0)));
    cases.push(("empty source_uri", mutate(|value| value.source_uri.clear())));
    cases.push((
        "empty source_media_type",
        mutate(|value| value.source_media_type.clear()),
    ));
    cases.push((
        "short source_digest",
        mutate(|value| value.source_digest.truncate(31)),
    ));
    cases.push(("empty terms", mutate(|value| value.terms.clear())));
    cases.push((
        "too many terms",
        mutate(|value| {
            value.terms = (0..=MAXIMUM_VOCABULARY_TERMS)
                .map(|index| term(&format!("term-{index:06}"), VocabularyCategory::EntityType))
                .collect();
        }),
    ));
    cases.push((
        "empty term_id",
        mutate(|value| value.terms[0].term_id.clear()),
    ));
    cases.push((
        "empty canonical_name",
        mutate(|value| value.terms[0].canonical_name.clear()),
    ));
    cases.push((
        "oversized canonical_name",
        mutate(|value| {
            value.terms[0].canonical_name = "n".repeat(MAXIMUM_VOCABULARY_NAME_BYTES + 1);
        }),
    ));
    cases.push((
        "oversized alias",
        mutate(|value| {
            value.terms[0].aliases = Some(vec!["a".repeat(MAXIMUM_VOCABULARY_NAME_BYTES + 1)]);
        }),
    ));
    cases.push((
        "too many aliases",
        mutate(|value| {
            value.terms[0].aliases = Some(
                (0..=MAXIMUM_VOCABULARY_ALIASES)
                    .map(|index| format!("alias-{index}"))
                    .collect(),
            );
        }),
    ));
    cases.push((
        "empty alias",
        mutate(|value| value.terms[0].aliases = Some(vec![String::new()])),
    ));
    cases.push(("unsorted term_ids", mutate(|value| value.terms.swap(0, 2))));
    cases.push((
        "duplicate term_ids",
        mutate(|value| value.terms[1].term_id = value.terms[0].term_id.clone()),
    ));

    cases
}

fn mutate(apply: impl FnOnce(&mut VocabularyImported)) -> EventPayload {
    let EventPayload::VocabularyImported(mut value) = imported() else {
        panic!("the fixture is a vocabulary import");
    };
    apply(&mut value);
    EventPayload::VocabularyImported(value)
}

fn imported() -> EventPayload {
    EventPayload::VocabularyImported(Box::new(VocabularyImported {
        vocabulary_id: b"fleet-vocabulary".to_vec(),
        version: 1,
        source_uri: "file:///vocabularies/fleet.nt".to_owned(),
        source_media_type: "application/n-triples".to_owned(),
        source_digest: vec![0x5a; 32],
        terms: vec![
            term("term-truck", VocabularyCategory::EntityInstance),
            VocabularyTerm {
                parent_term_id: Some("term-vehicle".to_owned()),
                ..term("term-van", VocabularyCategory::EntityType)
            },
            term("term-works-for", VocabularyCategory::Relation),
        ],
        ignored_triples: 4,
    }))
}

fn term(term_id: &str, category: VocabularyCategory) -> VocabularyTerm {
    VocabularyTerm {
        term_id: term_id.to_owned(),
        canonical_name: term_id.replace('-', " "),
        category,
        parent_term_id: None,
        aliases: Some(vec![format!("{term_id}-alias")]),
    }
}

fn envelope(payload: EventPayload, authority: Authority) -> EventEnvelope {
    EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 7,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    }
}
