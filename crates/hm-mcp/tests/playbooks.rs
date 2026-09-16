#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_mcp::{Envelope, InspectInput, IntendAction, IntendInput, McpServer};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ResultStatus, Retention, Sensitivity, ToolCall,
    ToolResult,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::Value;

const SOURCE_URI: &str = "file:///playbooks/release-checks.md";
const DOCUMENT: &str = "%%\nname: release checks\nstrategy: follow the release playbook\noutcomes: the release is published\npreconditions: the checks are green\ntools: run_checks; publish\nversion: 3\n%%\n1. run the checks\n2. publish the release\n";
const INSTRUCTIONS: &str = "1. run the checks\n2. publish the release\n";
const PROPOSAL_ID: &str = "proposal-release-checks-1";
const IMPROVED_STRATEGY: &str = "run the checks twice before publishing";
const RATIONALE: &str = "the observed run failed the release checks before publishing";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn event(kind: EventKind, payload: EventPayload, authority: Authority) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("playbooks"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

async fn intend(server: &McpServer, action: IntendAction) -> Value {
    let envelope = server
        .intend_envelope(IntendInput {
            conversation: "playbooks".to_owned(),
            action,
        })
        .await;
    assert!(envelope.ok, "intend failed: {:?}", envelope.items);
    envelope.items[0].clone()
}

async fn inspect(server: &McpServer, uri: &str) -> Envelope {
    server
        .inspect_envelope(InspectInput {
            uri: Some(uri.to_owned()),
            ..InspectInput::default()
        })
        .await
}

async fn listing(server: &McpServer) -> Vec<Value> {
    let envelope = inspect(server, "hm://7/procedures").await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    envelope.items[0]["procedures"]
        .as_array()
        .expect("a procedure listing")
        .clone()
}

async fn procedure(server: &McpServer, procedure_id: &str) -> Value {
    let envelope = inspect(server, &format!("hm://7/procedures/{procedure_id}")).await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    envelope.items[0]["procedure"].clone()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn playbooks_import_disclose_progressively_and_improve_through_review() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let call_lsn = actor
        .append(vec![event(
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"call-release".to_vec(),
                tool_name: "run_checks".into(),
                arguments: br#"{"suite":"release"}"#.to_vec(),
            })),
            Authority::AssistantGenerated,
        )])
        .await
        .unwrap()
        .first_lsn
        .get();
    let failure_lsn = actor
        .append(vec![event(
            EventKind::ToolResult,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"call-release".to_vec(),
                tool_call_lsn: call_lsn,
                status: ResultStatus::Error,
                result: br#"{"failed":"the release checks timed out"}"#.to_vec(),
            })),
            Authority::ToolObserved,
        )])
        .await
        .unwrap()
        .first_lsn
        .get();

    let imported = intend(
        &server,
        IntendAction::ImportPlaybook {
            source_uri: SOURCE_URI.to_owned(),
            document: DOCUMENT.to_owned(),
        },
    )
    .await;
    assert_eq!(imported["action"].as_str(), Some("import_playbook"));
    let procedure_id = imported["procedure_id"].as_str().unwrap().to_owned();
    assert_eq!(procedure_id.len(), 64);
    let import_lsn = imported["lsn"].as_u64().unwrap();
    assert!(import_lsn > failure_lsn);

    let listed = listing(&server).await;
    assert_eq!(listed.len(), 1);
    let entry = listed[0].as_object().unwrap();
    assert_eq!(entry["procedure_id"].as_str(), Some(procedure_id.as_str()));
    assert_eq!(entry["state"].as_str(), Some("imported"));
    assert_eq!(
        entry["strategy"].as_str(),
        Some("follow the release playbook")
    );
    assert_eq!(
        strings(&entry["expected_outcomes"]),
        ["the release is published"]
    );
    assert_eq!(strings(&entry["preconditions"]), ["the checks are green"]);
    assert_eq!(entry["version_lsn"].as_u64(), Some(import_lsn));
    assert_eq!(entry["adopted_lsn"].as_u64(), Some(0));
    assert_eq!(entry["name"].as_str(), Some("release checks"));
    assert_eq!(entry["source_uri"].as_str(), Some(SOURCE_URI));
    assert_eq!(entry["source_digest"].as_str().unwrap().len(), 64);
    assert_eq!(entry["playbook_version"].as_u64(), Some(3));
    assert_eq!(strings(&entry["declared_tools"]), ["run_checks", "publish"]);
    assert_eq!(
        entry["instruction_bytes"].as_u64(),
        Some(INSTRUCTIONS.len() as u64)
    );
    assert!(
        !entry.contains_key("instructions"),
        "a listing must not carry an instruction body"
    );
    let serialized = serde_json::to_string(&listed[0]).unwrap();
    assert!(!serialized.contains("publish the release"), "{serialized}");
    assert!(!serialized.contains("1. run the checks"), "{serialized}");

    let disclosed = procedure(&server, &procedure_id).await;
    assert_eq!(disclosed["state"].as_str(), Some("imported"));
    assert_eq!(disclosed["name"].as_str(), Some("release checks"));
    assert_eq!(disclosed["instructions"].as_str(), Some(INSTRUCTIONS));
    assert!(disclosed["proposals"].as_array().unwrap().is_empty());

    let malformed = inspect(&server, "hm://7/procedures/not-hex").await;
    assert!(!malformed.ok, "a malformed identifier must be refused");
    assert_eq!(
        malformed.items[0]["error"].as_str(),
        Some("kInvalidArgument")
    );
    let unknown = inspect(&server, &format!("hm://7/procedures/{}", "00".repeat(32))).await;
    assert!(!unknown.ok, "an unknown procedure must be refused");
    assert_eq!(unknown.items[0]["error"].as_str(), Some("kInvalidArgument"));

    let adopted = intend(
        &server,
        IntendAction::AdoptProcedure {
            procedure_id: procedure_id.clone(),
            procedure_lsn: import_lsn,
        },
    )
    .await;
    assert_eq!(adopted["action"].as_str(), Some("adopt_procedure"));
    let adoption_lsn = adopted["lsn"].as_u64().unwrap();
    let listed = listing(&server).await;
    assert_eq!(listed[0]["state"].as_str(), Some("adopted"));
    assert_eq!(listed[0]["version_lsn"].as_u64(), Some(import_lsn));
    assert_eq!(listed[0]["adopted_lsn"].as_u64(), Some(adoption_lsn));

    let proposed = intend(
        &server,
        IntendAction::ProposeProcedureImprovement {
            proposal_id: PROPOSAL_ID.to_owned(),
            procedure_id: procedure_id.clone(),
            strategy: IMPROVED_STRATEGY.to_owned(),
            expected_outcomes: vec!["the release is published".to_owned()],
            preconditions: vec!["the checks are green".to_owned()],
            rationale: RATIONALE.to_owned(),
            failure_lsns: vec![failure_lsn],
        },
    )
    .await;
    assert_eq!(
        proposed["action"].as_str(),
        Some("propose_procedure_improvement")
    );
    assert_eq!(
        proposed["procedure_id"].as_str(),
        Some(procedure_id.as_str())
    );
    assert_eq!(proposed["proposal_id"].as_str(), Some(PROPOSAL_ID));
    let proposal_lsn = proposed["lsn"].as_u64().unwrap();

    let reviewed = procedure(&server, &procedure_id).await;
    assert_eq!(
        reviewed["strategy"].as_str(),
        Some("follow the release playbook")
    );
    assert_eq!(reviewed["version_lsn"].as_u64(), Some(import_lsn));
    assert_eq!(reviewed["adopted_lsn"].as_u64(), Some(adoption_lsn));
    let proposals = reviewed["proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0]["rationale"].as_str(), Some(RATIONALE));
    assert_eq!(proposals[0]["strategy"].as_str(), Some(IMPROVED_STRATEGY));
    assert_eq!(proposals[0]["base_lsn"].as_u64(), Some(import_lsn));
    assert_eq!(proposals[0]["proposed_lsn"].as_u64(), Some(proposal_lsn));
    assert_eq!(proposals[0]["adopted_lsn"].as_u64(), Some(0));
    assert_eq!(
        proposals[0]["failure_lsns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap())
            .collect::<Vec<_>>(),
        [failure_lsn]
    );

    let improved = intend(
        &server,
        IntendAction::AdoptProcedure {
            procedure_id: procedure_id.clone(),
            procedure_lsn: proposal_lsn,
        },
    )
    .await;
    let improved_lsn = improved["lsn"].as_u64().unwrap();
    let head = procedure(&server, &procedure_id).await;
    assert_eq!(head["state"].as_str(), Some("adopted"));
    assert_eq!(head["strategy"].as_str(), Some(IMPROVED_STRATEGY));
    assert_eq!(head["version_lsn"].as_u64(), Some(improved_lsn));
    assert_eq!(head["previous_lsn"].as_u64(), Some(import_lsn));
    assert_eq!(head["adopted_lsn"].as_u64(), Some(improved_lsn));
    assert_eq!(head["instructions"].as_str(), Some(INSTRUCTIONS));
    let proposals = head["proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0]["adopted_lsn"].as_u64(), Some(improved_lsn));

    actor.shutdown().await.unwrap();
}
