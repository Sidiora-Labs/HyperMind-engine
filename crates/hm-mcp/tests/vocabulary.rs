#![forbid(unsafe_code)]

use hm_core::{ActorId, LSN};
use hm_mcp::{McpServer, RememberInput, RememberKind, RetentionInput, VocabularyInput};
use hm_schema::event::{Boundary, EventKind, verify_event};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};

const DOCUMENT: &str = concat!(
    "# acme-crm terms\n",
    "<https://acme.example/crm#Account> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n",
    "<https://acme.example/crm#Account> <http://www.w3.org/2000/01/rdf-schema#label> \"Account\" .\n",
    "<https://acme.example/crm#billsFor> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .\n",
    "<https://acme.example/crm#billsFor> <http://www.w3.org/2004/02/skos/core#altLabel> \"invoices\" .\n",
    "<https://acme.example/crm#northwind> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#NamedIndividual> .\n",
    "<https://acme.example/crm#northwind> <http://purl.org/dc/terms/created> \"2026-02-02T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .\n",
);

const MALFORMED: &str = "Account is a class in the acme-crm vocabulary.\n";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn vocabulary() -> VocabularyInput {
    VocabularyInput {
        vocabulary_id: "acme-crm".to_owned(),
        version: 1,
        source_uri: "file:///vocab/acme-crm.nt".to_owned(),
    }
}

fn import(content: &str, vocabulary: Option<VocabularyInput>) -> RememberInput {
    RememberInput {
        conversation: "ops".to_owned(),
        content: content.to_owned(),
        kind: RememberKind::Vocabulary,
        chunk_bytes: Some(16),
        anchor: None,
        retention: Some(RetentionInput::Durable),
        sensitivity: None,
        vocabulary,
        source: None,
        derive: None,
        source_delivery: None,
        source_settlement: None,
        document: None,
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[tokio::test]
async fn remember_imports_a_versioned_vocabulary() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let envelope = server
        .remember_envelope(import(DOCUMENT, Some(vocabulary())))
        .await;
    assert!(envelope.ok, "{:?}", envelope.items);
    assert!(envelope.effect_state.is_none());
    let item = &envelope.items[0];
    let first_lsn = item["first_lsn"].as_u64().unwrap();
    assert_eq!(item["last_lsn"].as_u64(), Some(first_lsn));
    assert_eq!(item["count"].as_u64(), Some(1));
    assert_eq!(item["vocabulary_id"].as_str(), Some("acme-crm"));
    assert_eq!(item["version"].as_u64(), Some(1));
    assert_eq!(
        item["source_uri"].as_str(),
        Some("file:///vocab/acme-crm.nt")
    );
    assert_eq!(
        item["source_media_type"].as_str(),
        Some("application/n-triples")
    );
    let digest = item["source_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert_eq!(digest, hex(blake3::hash(DOCUMENT.as_bytes()).as_bytes()));
    assert_eq!(item["term_count"].as_u64(), Some(3));
    assert_eq!(item["ignored_triples"].as_u64(), Some(1));
    assert_eq!(envelope.provenance, vec![format!("hm://7/lsn/{first_lsn}")]);
    assert_eq!(envelope.warnings.len(), 1);
    assert!(envelope.warnings[0].contains("merges no existing identity"));

    assert_eq!(
        actor.stats().await.unwrap().applied.last_lsn.get(),
        first_lsn
    );

    let frames = actor
        .frames_since(LSN::new(first_lsn - 1), None, 1)
        .await
        .unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].header.lsn.get(), first_lsn);
    assert_eq!(
        frames[0].header.kind,
        hm_ledger::frame::EventKind::VocabularyImported
    );
    let verified = verify_event(
        &frames[0].sealed_payload,
        EventKind::VocabularyImported,
        Boundary::Disk,
    )
    .unwrap();
    assert_eq!(verified.envelope.authority, Authority::UserAsserted);
    let EventPayload::VocabularyImported(value) = verified.envelope.payload else {
        panic!("the appended event must carry a vocabulary payload")
    };
    assert_eq!(value.vocabulary_id, b"acme-crm".to_vec());
    assert_eq!(value.version, 1);
    assert_eq!(value.terms.len(), 3);
    assert_eq!(value.ignored_triples, 1);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn rejects_invalid_vocabulary_arguments_without_appending() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let baseline = actor.stats().await.unwrap().applied.last_lsn.get();

    let empty_id = VocabularyInput {
        vocabulary_id: String::new(),
        ..vocabulary()
    };
    let zero_version = VocabularyInput {
        version: 0,
        ..vocabulary()
    };
    let cases = [
        (import(DOCUMENT, None), "kInvalidArgument"),
        (import(DOCUMENT, Some(empty_id)), "kInvalidArgument"),
        (import(DOCUMENT, Some(zero_version)), "kInvalidArgument"),
        (import(MALFORMED, Some(vocabulary())), "kSchemaInvalid"),
    ];

    for (input, expected) in cases {
        let envelope = server.remember_envelope(input).await;
        assert!(!envelope.ok);
        assert_eq!(envelope.items[0]["error"].as_str(), Some(expected));
        assert_eq!(envelope.effect_state.as_deref(), Some("not_dispatched"));
        assert!(envelope.provenance.is_empty());
        assert_eq!(
            actor.stats().await.unwrap().applied.last_lsn.get(),
            baseline
        );
    }

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn do_not_store_vocabulary_returns_a_receipt_without_a_ledger_event() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let baseline = actor.stats().await.unwrap().applied.last_lsn.get();

    let mut input = import(DOCUMENT, Some(vocabulary()));
    input.retention = Some(RetentionInput::DoNotStore);
    let envelope = server.remember_envelope(input).await;
    assert!(envelope.ok, "{:?}", envelope.items);
    assert_eq!(envelope.items[0]["stored"].as_bool(), Some(false));
    assert_eq!(
        envelope.items[0]["retention"].as_str(),
        Some("do_not_store")
    );
    assert_eq!(
        envelope.items[0]["receipt"].as_str(),
        Some(hex(blake3::hash(DOCUMENT.as_bytes()).as_bytes()).as_str())
    );
    assert_eq!(
        actor.stats().await.unwrap().applied.last_lsn.get(),
        baseline
    );

    actor.shutdown().await.unwrap();
}
