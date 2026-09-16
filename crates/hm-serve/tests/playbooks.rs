#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_proj::procedures::ProcedureState;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::*;
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};

const PLAYBOOK_ID: &[u8] = b"playbook-release-checks";
const SECOND_ID: &[u8] = b"playbook-second";
const PROPOSAL_ID: &[u8] = b"proposal-release-checks-1";
const INSTRUCTIONS: &[u8] = b"1. run the checks\n2. publish the release\n";
const DIGEST: [u8; 32] = [7; 32];

#[tokio::test]
async fn imported_playbooks_and_proposals_are_admitted_read_and_survive_restart() {
    let directory = tempfile::tempdir().unwrap();
    let config = actor_config(directory.path());
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    let conversation = ConversationId::derive("playbooks");

    let call_lsn = actor
        .append(vec![event(
            conversation,
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
            conversation,
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

    let import_lsn = actor
        .append(vec![import(conversation, PLAYBOOK_ID)])
        .await
        .unwrap()
        .first_lsn
        .get();
    let imported = actor
        .procedure(PLAYBOOK_ID.to_vec())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(imported.state, ProcedureState::Imported);
    assert_eq!(imported.version_lsn, import_lsn);
    assert_eq!(imported.adopted_lsn, 0);
    assert!(imported.supports.is_empty());
    assert_eq!(imported.strategy, "follow the release playbook");

    let metadata = actor.playbook(PLAYBOOK_ID.to_vec()).await.unwrap().unwrap();
    assert_eq!(metadata.name, "release checks");
    assert_eq!(metadata.source_uri, "file:///playbooks/release-checks.md");
    assert_eq!(metadata.source_digest, DIGEST.to_vec());
    assert_eq!(metadata.playbook_version, 3);
    assert_eq!(metadata.instruction_bytes, INSTRUCTIONS.len() as u64);
    assert_eq!(metadata.imported_lsn, import_lsn);
    assert_eq!(
        metadata.declared_tools,
        vec!["run_checks".to_owned(), "publish".to_owned()]
    );
    assert_eq!(
        actor
            .playbook_instructions(PLAYBOOK_ID.to_vec())
            .await
            .unwrap()
            .unwrap(),
        INSTRUCTIONS.to_vec()
    );

    let events_before_duplicate = actor.stats().await.unwrap().log_events;
    assert_eq!(
        actor
            .append(vec![import(conversation, PLAYBOOK_ID)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::AlreadyExists
    );
    assert_eq!(
        actor.stats().await.unwrap().log_events,
        events_before_duplicate
    );

    assert_eq!(
        actor
            .append(vec![proposal(
                conversation,
                PROPOSAL_ID,
                failure_lsn,
                failure_lsn
            )])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        actor.stats().await.unwrap().log_events,
        events_before_duplicate
    );

    let proposal_lsn = actor
        .append(vec![proposal(
            conversation,
            PROPOSAL_ID,
            import_lsn,
            failure_lsn,
        )])
        .await
        .unwrap()
        .first_lsn
        .get();
    assert_eq!(
        actor
            .procedure(PLAYBOOK_ID.to_vec())
            .await
            .unwrap()
            .unwrap(),
        imported
    );
    let proposals = actor
        .procedure_proposals(PLAYBOOK_ID.to_vec(), 16)
        .await
        .unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].proposal_id, PROPOSAL_ID.to_vec());
    assert_eq!(proposals[0].base_lsn, import_lsn);
    assert_eq!(proposals[0].proposed_lsn, proposal_lsn);
    assert_eq!(proposals[0].adopted_lsn, 0);
    assert_eq!(proposals[0].failure_lsns, vec![failure_lsn]);
    assert_eq!(
        proposals[0].rationale,
        "the observed run failed the release checks before publishing"
    );

    let head_adoption_lsn = actor
        .append(vec![adopt(conversation, PLAYBOOK_ID, import_lsn)])
        .await
        .unwrap()
        .first_lsn
        .get();
    let adopted = actor
        .procedure(PLAYBOOK_ID.to_vec())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(adopted.state, ProcedureState::Adopted);
    assert_eq!(adopted.version_lsn, import_lsn);
    assert_eq!(adopted.adopted_lsn, head_adoption_lsn);
    assert_eq!(
        actor
            .append(vec![adopt(conversation, PLAYBOOK_ID, import_lsn)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::OrderingViolation
    );

    let proposal_adoption_lsn = actor
        .append(vec![adopt(conversation, PLAYBOOK_ID, proposal_lsn)])
        .await
        .unwrap()
        .first_lsn
        .get();
    let improved = actor
        .procedure(PLAYBOOK_ID.to_vec())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(improved.state, ProcedureState::Adopted);
    assert_eq!(improved.strategy, "run the checks twice before publishing");
    assert_eq!(improved.version_lsn, proposal_adoption_lsn);
    assert_eq!(improved.previous_lsn, import_lsn);
    assert_eq!(improved.adopted_lsn, proposal_adoption_lsn);
    assert_eq!(
        actor
            .append(vec![adopt(conversation, PLAYBOOK_ID, proposal_lsn)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::OrderingViolation
    );

    let events_before_batches = actor.stats().await.unwrap().log_events;
    assert_eq!(
        actor
            .append(vec![
                import(conversation, SECOND_ID),
                import(conversation, SECOND_ID),
            ])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        actor
            .append(vec![
                proposal(
                    conversation,
                    PROPOSAL_ID,
                    proposal_adoption_lsn,
                    failure_lsn
                ),
                proposal(
                    conversation,
                    PROPOSAL_ID,
                    proposal_adoption_lsn,
                    failure_lsn
                ),
            ])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        actor.stats().await.unwrap().log_events,
        events_before_batches
    );
    assert!(actor.procedure(SECOND_ID.to_vec()).await.unwrap().is_none());

    let applied = actor.stats().await.unwrap().applied;
    actor.shutdown().await.unwrap();

    let reopened = ActorEngine::open(config).await.unwrap();
    assert_eq!(reopened.stats().await.unwrap().applied, applied);
    assert_eq!(
        reopened
            .procedure(PLAYBOOK_ID.to_vec())
            .await
            .unwrap()
            .unwrap(),
        improved
    );
    assert_eq!(
        reopened.playbook(PLAYBOOK_ID.to_vec()).await.unwrap(),
        Some(metadata)
    );
    assert_eq!(
        reopened
            .playbook_instructions(PLAYBOOK_ID.to_vec())
            .await
            .unwrap()
            .unwrap(),
        INSTRUCTIONS.to_vec()
    );
    let replayed = reopened
        .procedure_proposals(PLAYBOOK_ID.to_vec(), 16)
        .await
        .unwrap();
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].adopted_lsn, proposal_adoption_lsn);
    assert_eq!(replayed[0].failure_lsns, vec![failure_lsn]);
    assert_eq!(replayed[0].rationale, proposals[0].rationale);
    assert!(
        reopened
            .procedure(SECOND_ID.to_vec())
            .await
            .unwrap()
            .is_none()
    );
    reopened.shutdown().await.unwrap();
}

fn import(conversation: ConversationId, procedure_id: &[u8]) -> IncomingEvent {
    event(
        conversation,
        EventKind::ProcedureImported,
        EventPayload::ProcedureImported(Box::new(ProcedureImported {
            procedure_id: procedure_id.to_vec(),
            name: "release checks".into(),
            strategy: "follow the release playbook".into(),
            expected_outcomes: vec!["the release is published".into()],
            preconditions: vec!["the checks are green".into()],
            instructions: INSTRUCTIONS.to_vec(),
            declared_tools: vec!["run_checks".into(), "publish".into()],
            source_uri: "file:///playbooks/release-checks.md".into(),
            source_digest: DIGEST.to_vec(),
            playbook_version: 3,
        })),
        Authority::ExternalObserved,
    )
}

fn proposal(
    conversation: ConversationId,
    proposal_id: &[u8],
    base_lsn: u64,
    failure_lsn: u64,
) -> IncomingEvent {
    event(
        conversation,
        EventKind::ProcedureImprovementProposed,
        EventPayload::ProcedureImprovementProposed(Box::new(ProcedureImprovementProposed {
            proposal_id: proposal_id.to_vec(),
            procedure_id: PLAYBOOK_ID.to_vec(),
            base_lsn,
            strategy: "run the checks twice before publishing".into(),
            expected_outcomes: vec!["the release is published".into()],
            preconditions: vec!["the checks are green".into()],
            rationale: "the observed run failed the release checks before publishing".into(),
            failure_lsns: vec![failure_lsn],
        })),
        Authority::DerivedInference,
    )
}

fn adopt(conversation: ConversationId, procedure_id: &[u8], procedure_lsn: u64) -> IncomingEvent {
    event(
        conversation,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: procedure_id.to_vec(),
            procedure_lsn,
        })),
        Authority::UserAsserted,
    )
}

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("actor"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn event(
    conversation: ConversationId,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
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
