use base64::Engine as _;
use hm_core::{ActorId, ErrorCode};
use hm_docs::identity::document_identity;
use hm_mcp::{Envelope, McpServer, RememberDocument, RememberInput, RememberKind};
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::uds::ToolDispatcher;
use serde_json::Value;

const DAMAGED_PAGE: &[u8] = include_bytes!("../../hm-docs/tests/fixtures/damaged-page.pdf");

const PROSE: &str = "Alpha one two.\n\nBeta three four.\n\nGamma five six.\n";
const REVISED_PROSE: &str = "Alpha one two.\n\nBeta three four rewritten.\n\nGamma five six.\n";
const RECORDS: &str = "name,role,note\nada,engineer,short\ngrace,admiral,long\n";

fn actor_config(path: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join(actor.to_string()),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn document(
    name: &str,
    media_type: &str,
    bytes: &[u8],
    token_budget: u32,
    plan_only: bool,
) -> RememberInput {
    RememberInput {
        conversation: "library".to_owned(),
        content: String::new(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: None,
        source_delivery: None,
        source_settlement: None,
        document: Some(RememberDocument {
            name: name.to_owned(),
            media_type: media_type.to_owned(),
            content_base64: encode(bytes),
            loader: None,
            token_budget: Some(token_budget),
            plan_only,
        }),
        source_sync: None,
    }
}

fn chunks(envelope: &Envelope) -> &Vec<Value> {
    envelope.items[0]["chunks"]
        .as_array()
        .expect("chunk preview")
}

fn identifiers(envelope: &Envelope) -> Vec<String> {
    chunks(envelope)
        .iter()
        .map(|chunk| chunk["chunk_id"].as_str().expect("chunk id").to_owned())
        .collect()
}

fn changes(envelope: &Envelope) -> Vec<String> {
    chunks(envelope)
        .iter()
        .map(|chunk| chunk["change"].as_str().expect("change").to_owned())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
    }
    text
}

async fn published_ids(actor: &ActorEngine, name: &str) -> Vec<String> {
    let state = actor
        .document_state(document_identity(name, &[]).to_vec())
        .await
        .expect("document state read")
        .expect("a published document");
    state
        .chunks
        .iter()
        .map(|record| hex(&record.chunk_id))
        .collect()
}

#[tokio::test]
async fn plan_only_previews_without_appending() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 41))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let before = actor.stats().await.unwrap().log_events;
    let envelope = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            true,
        ))
        .await;

    assert!(envelope.ok, "{:?}", envelope.items);
    assert_eq!(envelope.items[0]["published"], Value::Bool(false));
    assert_eq!(envelope.items[0]["loader"], "text");
    assert_eq!(envelope.items[0]["extraction_version"], 1);
    assert_eq!(envelope.items[0]["generation"], Value::Null);
    assert_eq!(envelope.items[0]["run_id"], Value::Null);
    assert_eq!(envelope.items[0]["plan"]["added"], 3);
    assert_eq!(envelope.items[0]["plan"]["retained"], 0);
    assert_eq!(changes(&envelope), vec!["added", "added", "added"]);
    assert_eq!(chunks(&envelope)[0]["byte_start"], 0);
    assert_eq!(
        chunks(&envelope)[2]["byte_end"],
        Value::from(u64::try_from(PROSE.len()).unwrap())
    );

    assert_eq!(actor.stats().await.unwrap().log_events, before);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn publishing_a_document_makes_its_chunks_readable() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 42))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let preview = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            true,
        ))
        .await;
    let envelope = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            false,
        ))
        .await;

    assert!(envelope.ok, "{:?}", envelope.items);
    assert_eq!(envelope.items[0]["published"], Value::Bool(true));
    assert_eq!(envelope.items[0]["generation"], 1);
    assert!(envelope.items[0]["run_id"].is_string());
    assert_eq!(envelope.items[0]["first_lsn"], 1);
    assert!(envelope.items[0]["last_lsn"].as_u64().unwrap() > 1);
    assert_eq!(identifiers(&envelope), identifiers(&preview));

    assert_eq!(
        published_ids(&actor, "handbook.md").await,
        identifiers(&envelope)
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn re_ingesting_identical_bytes_retains_every_chunk() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 43))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let first = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            false,
        ))
        .await;
    assert!(first.ok, "{:?}", first.items);

    let second = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            false,
        ))
        .await;
    assert!(second.ok, "{:?}", second.items);
    assert_eq!(changes(&second), vec!["retained", "retained", "retained"]);
    assert_eq!(second.items[0]["plan"]["retained"], 3);
    assert_eq!(second.items[0]["plan"]["added"], 0);
    assert_eq!(identifiers(&second), identifiers(&first));
    assert_eq!(second.items[0]["generation"], 1);
    assert_eq!(
        published_ids(&actor, "handbook.md").await,
        identifiers(&first)
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn revising_one_paragraph_keeps_the_untouched_chunk_ids() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 44))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let first = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            PROSE.as_bytes(),
            4,
            false,
        ))
        .await;
    assert!(first.ok, "{:?}", first.items);
    let original = identifiers(&first);

    let revised = server
        .remember_envelope(document(
            "handbook.md",
            "text/markdown",
            REVISED_PROSE.as_bytes(),
            4,
            false,
        ))
        .await;
    assert!(revised.ok, "{:?}", revised.items);
    let kinds = changes(&revised);
    assert_eq!(kinds[0], "retained");
    assert_eq!(kinds[2], "retained");
    assert!(
        kinds[1] == "replaced" || kinds[1] == "added",
        "the edited run is republished, not retained: {kinds:?}"
    );

    let updated = identifiers(&revised);
    assert_eq!(updated[0], original[0]);
    assert_eq!(updated[2], original[2]);
    assert_ne!(updated[1], original[1]);
    assert_eq!(revised.items[0]["generation"], 2);
    assert_eq!(published_ids(&actor, "handbook.md").await, updated);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn table_documents_chunk_by_row() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 45))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let envelope = server
        .remember_envelope(document(
            "roster.csv",
            "text/csv",
            RECORDS.as_bytes(),
            64,
            false,
        ))
        .await;

    assert!(envelope.ok, "{:?}", envelope.items);
    assert_eq!(envelope.items[0]["loader"], "table");
    let rows = chunks(&envelope);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["row"], 0);
    assert_eq!(rows[1]["row"], 1);
    assert_eq!(rows[2]["row"], 2);
    for row in rows {
        assert_eq!(row["cut"], "row_end");
    }
    assert_eq!(
        published_ids(&actor, "roster.csv").await,
        identifiers(&envelope)
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn partial_extraction_is_surfaced_as_a_gap() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 46))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let envelope = server
        .remember_envelope(document(
            "atlas.pdf",
            "application/pdf",
            DAMAGED_PAGE,
            64,
            false,
        ))
        .await;

    assert!(envelope.ok, "{:?}", envelope.items);
    assert_eq!(envelope.items[0]["loader"], "pdf");
    assert_eq!(envelope.items[0]["published"], Value::Bool(true));
    assert_eq!(envelope.gaps.len(), 1);
    assert_eq!(envelope.gaps[0]["kind"], "partial_extraction");
    assert_eq!(envelope.gaps[0]["loader"], "pdf");
    assert_eq!(envelope.gaps[0]["failed_units"], serde_json::json!([2]));
    assert!(
        envelope.gaps[0]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty())
    );
    assert_eq!(envelope.warnings.len(), 1);
    assert!(envelope.warnings[0].contains("unextracted"));

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn rejections_carry_effect_state() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 47))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let mut both = document("handbook.md", "text/markdown", PROSE.as_bytes(), 4, true);
    both.content = "a stray string".to_owned();
    let conflicting = server.remember_envelope(both).await;
    assert!(!conflicting.ok);
    assert_eq!(
        conflicting.items[0]["error"],
        ErrorCode::InvalidArgument.as_str()
    );
    assert!(conflicting.effect_state.is_some());

    let mut oversized = document("huge.md", "text/markdown", PROSE.as_bytes(), 4, true);
    oversized.document.as_mut().unwrap().content_base64 = "A".repeat(11_184_816);
    let refused = server.remember_envelope(oversized).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"],
        ErrorCode::CapacityExceeded.as_str()
    );
    assert!(refused.effect_state.is_some());

    let archive = server
        .remember_envelope(document(
            "bundle.zip",
            "application/zip",
            b"PK\x03\x04 not a document",
            4,
            true,
        ))
        .await;
    assert!(!archive.ok);
    assert_eq!(
        archive.items[0]["error"],
        ErrorCode::OperationUnavailable.as_str()
    );
    assert!(archive.effect_state.is_some());

    assert_eq!(actor.stats().await.unwrap().log_events, 0);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn tool_inventory_is_unchanged() {
    assert_eq!(hm_serve::rest::VERBS.len(), 14);

    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path(), 48))
        .await
        .unwrap();
    let dispatcher = hm_mcp::dispatcher::McpToolDispatcher::default();

    let mut resolved = 0;
    for verb in hm_serve::rest::VERBS {
        let outcome = dispatcher
            .dispatch(actor.clone(), verb.to_owned(), b"{}".to_vec())
            .await;
        let denied = outcome
            .as_ref()
            .err()
            .is_some_and(|error| error.code == ErrorCode::CapabilityDenied);
        assert!(!denied, "{verb} must resolve to a tool");
        resolved += 1;
    }
    assert_eq!(resolved, 14);

    let unknown = dispatcher
        .dispatch(actor.clone(), "document".to_owned(), b"{}".to_vec())
        .await
        .expect_err("an unlisted verb is not dispatched");
    assert_eq!(unknown.code, ErrorCode::CapabilityDenied);

    actor.shutdown().await.unwrap();
}
