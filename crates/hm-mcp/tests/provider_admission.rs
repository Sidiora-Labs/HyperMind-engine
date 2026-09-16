use hm_compose::reconstruct::{ReconstructionAnchor, reconstruction_request};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_llm::admission::{AdmissionLimits, CallAdmission};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest,
};
use hm_mcp::{ConsolidationRuntime, RecallFilters, RecallInput, RecallMode, ReconstructionRuntime};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ResultStatus, Retention, Sensitivity, ToolCall,
    ToolResult, UserMsg,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

const IN_FLIGHT: &str = "HM_PROVIDER_MAX_IN_FLIGHT";
const PER_ACTOR: &str = "HM_PROVIDER_MAX_IN_FLIGHT_PER_ACTOR";
const WAIT_MS: &str = "HM_PROVIDER_ADMISSION_WAIT_MS";
const CHILD: &str = "admission_limits_under_the_current_environment";

fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn provider() -> Arc<OpenAiCompatible<RecordedTransport>> {
    Arc::new(
        OpenAiCompatible::new(
            ProviderConfig {
                endpoint: "https://gateway.centra.ag/v1/chat/completions".to_owned(),
                api_key: None,
                model: "openrouter/openai/gpt-4o-mini".to_owned(),
                tier: ModelTier::Economy,
                pricing: Pricing {
                    input_microusd_per_million_tokens: 150_000,
                    output_microusd_per_million_tokens: 600_000,
                },
            },
            RecordedTransport::from_json(include_str!("fixtures/reconstruction-centra.json"))
                .unwrap(),
        )
        .unwrap(),
    )
}

fn single_slot() -> Arc<CallAdmission> {
    Arc::new(CallAdmission::new(AdmissionLimits {
        maximum_in_flight: 1,
        maximum_in_flight_per_actor: 1,
        maximum_wait_ms: 0,
    }))
}

fn event(kind: EventKind, payload: EventPayload, authority: Authority) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::new([0; 16]),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            authority,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 1,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

async fn seed(actor: &ActorEngine, directory: &Path) {
    actor
        .append(vec![event(
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"At 09:00 release 7 was queued.".to_vec(),
            })),
            Authority::UserAsserted,
        )])
        .await
        .unwrap();
    let receipt = directory.join("release.receipt");
    std::fs::write(&receipt, "At 09:05 receipt confirms release 7 completed.").unwrap();
    actor
        .append(vec![event(
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"release-check".to_vec(),
                tool_name: "read-receipt".to_owned(),
                arguments: receipt.to_str().unwrap().as_bytes().to_vec(),
            })),
            Authority::RuntimeFact,
        )])
        .await
        .unwrap();
    let result = std::fs::read(&receipt).unwrap();
    actor
        .append(vec![event(
            EventKind::ToolResult,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"release-check".to_vec(),
                tool_call_lsn: 2,
                status: ResultStatus::Ok,
                result,
            })),
            Authority::ToolObserved,
        )])
        .await
        .unwrap();
}

fn recall() -> RecallInput {
    RecallInput {
        mode: RecallMode::Reconstruct,
        query: "release 7".to_owned(),
        conversation: String::new(),
        limit: 32,
        since_lsn: 0,
        filters: RecallFilters {
            anchor_lsns: vec![1, 3],
            maximum_output_tokens: Some(256),
            ..RecallFilters::default()
        },
    }
}

fn anchors() -> Vec<ReconstructionAnchor> {
    vec![
        ReconstructionAnchor {
            lsn: LSN::new(1),
            uri: "hm://7/00000000000000000000000000000000/1".to_owned(),
            content: "At 09:00 release 7 was queued.".to_owned(),
            authority: Authority::UserAsserted,
        },
        ReconstructionAnchor {
            lsn: LSN::new(3),
            uri: "hm://7/00000000000000000000000000000000/3".to_owned(),
            content: "At 09:05 receipt confirms release 7 completed.".to_owned(),
            authority: Authority::ToolObserved,
        },
    ]
}

fn anchor_request() -> StructuredRequest {
    reconstruction_request(&anchors(), 256).unwrap()
}

#[tokio::test]
async fn a_saturated_gate_refuses_reconstruction_without_spending_a_call() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    seed(&actor, directory.path()).await;
    let recorded = provider();
    let mut runtime = ReconstructionRuntime::new(recorded.clone());
    runtime.admission = single_slot();
    let admission = Arc::clone(&runtime.admission);
    let held = admission.admit(7).unwrap();
    let refused = hm_mcp::tools::reconstruct::run(&actor, Some(&runtime), recall())
        .await
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
    assert_eq!(recorded.transport().remaining(), 1);
    assert_eq!(admission.refusals(), 1);
    drop(held);
    assert_eq!(admission.in_flight(), 0);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_open_gate_passes_reconstruction_through_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    seed(&actor, directory.path()).await;
    let recorded = provider();
    let runtime = ReconstructionRuntime::new(recorded.clone());
    let envelope = hm_mcp::tools::reconstruct::run(&actor, Some(&runtime), recall())
        .await
        .unwrap();
    assert_eq!(envelope.items[0]["label"], "RECONSTRUCTION");
    assert_eq!(envelope.items[0]["authority"], "assistant_generated");
    assert!(envelope.warnings.iter().any(|warning| {
        warning
            == "Assistant-generated reconstruction is not evidence and cannot be remembered verbatim."
    }));
    assert_eq!(recorded.transport().remaining(), 0);
    assert_eq!(runtime.admission.in_flight(), 0);
    actor.shutdown().await.unwrap();
}

#[test]
fn provider_for_takes_and_releases_a_permit_for_that_actor() {
    let request = anchor_request();

    let reconstruction_recorded = provider();
    let mut reconstruction = ReconstructionRuntime::new(reconstruction_recorded.clone());
    reconstruction.admission = single_slot();
    let narrated = reconstruction
        .provider_for(7)
        .generate_structured(&request)
        .unwrap();
    assert!(narrated.value.get("narrative").is_some());
    assert_eq!(reconstruction_recorded.transport().remaining(), 0);
    assert_eq!(reconstruction.admission.high_water_mark(), 1);
    assert_eq!(reconstruction.admission.in_flight(), 0);

    let consolidation_recorded = provider();
    let mut consolidation = ConsolidationRuntime::new(consolidation_recorded.clone());
    consolidation.admission = single_slot();
    let admitted = consolidation.provider_for(7);
    let held = Arc::clone(&consolidation.admission).admit(7).unwrap();
    let refused = admitted.generate_structured(&request).unwrap_err();
    assert!(matches!(refused, LlmError::Admission(_)));
    assert_eq!(consolidation_recorded.transport().remaining(), 1);
    drop(held);
    assert!(admitted.generate_structured(&request).is_ok());
    assert_eq!(consolidation_recorded.transport().remaining(), 0);
    assert_eq!(consolidation.admission.in_flight(), 0);
}

#[test]
fn admission_limits_come_from_the_environment_or_the_defaults() {
    assert_child_limits(&[], AdmissionLimits::default());
    assert_child_limits(
        &[(IN_FLIGHT, "3"), (PER_ACTOR, "1"), (WAIT_MS, "10")],
        AdmissionLimits {
            maximum_in_flight: 3,
            maximum_in_flight_per_actor: 1,
            maximum_wait_ms: 10,
        },
    );
    assert_child_limits(
        &[
            (IN_FLIGHT, ""),
            (PER_ACTOR, "not-a-number"),
            (WAIT_MS, "10"),
        ],
        AdmissionLimits {
            maximum_in_flight: AdmissionLimits::default().maximum_in_flight,
            maximum_in_flight_per_actor: AdmissionLimits::default().maximum_in_flight_per_actor,
            maximum_wait_ms: 10,
        },
    );
}

#[test]
#[ignore = "re-executed with a prepared environment by admission_limits_come_from_the_environment_or_the_defaults"]
fn admission_limits_under_the_current_environment() {
    let limits = hm_mcp::admission::admission_from_env().limits();
    println!(
        "admission-limits {} {} {}",
        limits.maximum_in_flight, limits.maximum_in_flight_per_actor, limits.maximum_wait_ms
    );
}

fn assert_child_limits(variables: &[(&str, &str)], expected: AdmissionLimits) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "--ignored", "--nocapture", CHILD])
        .env_remove(IN_FLIGHT)
        .env_remove(PER_ACTOR)
        .env_remove(WAIT_MS);
    for (name, value) in variables {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reported = String::from_utf8(output.stdout).unwrap();
    let wanted = format!(
        "admission-limits {} {} {}",
        expected.maximum_in_flight, expected.maximum_in_flight_per_actor, expected.maximum_wait_ms
    );
    assert!(
        reported.lines().any(|line| line == wanted),
        "wanted {wanted}, got {reported}"
    );
}
