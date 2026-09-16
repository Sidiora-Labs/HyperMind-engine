#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId};
use hm_mcp::{
    ExpectedPredicateInput, InspectInput, McpServer, OutcomeInput, PredicateKindInput, PredictInput,
};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ResultStatus, Retention, Sensitivity, ToolCall,
    ToolResult, UserMsg,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::{Value, json};

fn prediction(id: &str, expected: &str) -> PredictInput {
    PredictInput {
        conversation: "predictions".to_owned(),
        prediction_id: id.to_owned(),
        revision: 1,
        task_id: Some("write-file".to_owned()),
        attempt_id: Some(id.to_owned()),
        operation_id: Some(id.to_owned()),
        mechanism: "filesystem-write".to_owned(),
        predicates: vec![ExpectedPredicateInput {
            kind: PredicateKindInput::RevisionEquals,
            scope: "repository".to_owned(),
            property: Some("revision".to_owned()),
            expected: Some(json!(expected)),
        }],
        deadline_ns: i64::MAX,
        uncertainty: "The filesystem operation may fail.".to_owned(),
    }
}

fn outcome(id: &str, lsns: Vec<u64>) -> OutcomeInput {
    OutcomeInput {
        conversation: "predictions".to_owned(),
        prediction_id: id.to_owned(),
        revision: 1,
        observation_lsns: lsns,
    }
}

async fn append(
    actor: &ActorEngine,
    kind: hm_ledger::frame::EventKind,
    payload: EventPayload,
    authority: Authority,
) -> u64 {
    actor
        .append(vec![IncomingEvent {
            kind,
            conversation: ConversationId::derive("predictions"),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await
        .unwrap()
        .first_lsn
        .get()
}

async fn observed(actor: &ActorEngine, id: &str, observation: Value) -> u64 {
    let call_lsn = append(
        actor,
        hm_ledger::frame::EventKind::ToolCall,
        EventPayload::ToolCall(Box::new(ToolCall {
            call_id: id.as_bytes().to_vec(),
            tool_name: "filesystem-inspect".to_owned(),
            arguments: b"{}".to_vec(),
        })),
        Authority::RuntimeFact,
    )
    .await;
    append(
        actor,
        hm_ledger::frame::EventKind::ToolResult,
        EventPayload::ToolResult(Box::new(ToolResult {
            call_id: id.as_bytes().to_vec(),
            tool_call_lsn: call_lsn,
            status: ResultStatus::Ok,
            result: serde_json::to_vec(&json!({"schema_version": 1,"observations": [observation]}))
                .unwrap(),
        })),
        Authority::ToolObserved,
    )
    .await
}

fn revision(value: Option<&str>, executed: bool, resolvable: bool) -> Value {
    json!({"kind": "revision_equals", "scope": "repository", "property": "revision", "value": value, "executed": executed, "resolvable": resolvable})
}

#[tokio::test]
async fn real_observations_resolve_immutable_predictions_and_calibration_survives_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let config = ActorConfig {
        actor_directory: temporary.path().join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    };
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    let server = McpServer::new(actor.clone());
    let file = temporary.path().join("revision");
    std::fs::write(&file, "v2").unwrap();
    let actual_revision = std::fs::read_to_string(&file).unwrap();

    let mut pending = prediction("pending", "v2");
    pending.predicates.push(ExpectedPredicateInput {
        kind: PredicateKindInput::ObjectExists,
        scope: "revision-file".to_owned(),
        property: None,
        expected: Some(json!(true)),
    });
    assert!(server.predict_envelope(pending.clone()).await.ok);
    assert_eq!(
        server.predict_envelope(pending.clone()).await.items[0]["duplicate"],
        true
    );
    pending.predicates[0].expected = Some(json!("redefined-result"));
    let before = actor.stats().await.unwrap().log_events;
    assert!(!server.predict_envelope(pending).await.ok);
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    let partial = observed(
        &actor,
        "partial",
        revision(Some(&actual_revision), true, true),
    )
    .await;
    let response = server
        .outcome_envelope(outcome("pending", vec![partial]))
        .await;
    assert!(response.ok, "{response:?}");
    assert_eq!(response.items[0]["assessment"], "pending");
    let exists = observed(&actor, "exists", json!({"kind":"object_exists","scope":"revision-file","value":file.exists(),"executed":true,"resolvable":true})).await;
    let response = server
        .outcome_envelope(outcome("pending", vec![exists]))
        .await;
    assert!(response.ok, "{response:?}");
    assert_eq!(response.items[0]["assessment"], "supported");
    assert_eq!(
        server
            .outcome_envelope(outcome("pending", vec![exists]))
            .await
            .items[0]["duplicate"],
        true
    );

    for index in 0..3 {
        let id = format!("wrong-{index}");
        assert!(server.predict_envelope(prediction(&id, "v3")).await.ok);
        let lsn = observed(&actor, &id, revision(Some(&actual_revision), true, true)).await;
        let response = server.outcome_envelope(outcome(&id, vec![lsn])).await;
        assert!(response.ok, "{response:?}");
        assert_eq!(response.items[0]["assessment"], "contradicted");
        if index == 2 {
            assert_eq!(response.gaps[0]["kind"], "revision_required");
        }
    }
    assert!(
        server
            .predict_envelope(prediction("missing", "v2"))
            .await
            .ok
    );
    let unresolved = std::fs::read_to_string(temporary.path().join("absent"));
    assert!(unresolved.is_err());
    let lsn = observed(&actor, "missing", revision(None, true, false)).await;
    assert_eq!(
        server
            .outcome_envelope(outcome("missing", vec![lsn]))
            .await
            .items[0]["assessment"],
        "unresolvable"
    );
    assert!(
        server
            .predict_envelope(prediction("cancelled", "v2"))
            .await
            .ok
    );
    let lsn = observed(&actor, "cancelled", revision(None, false, false)).await;
    assert_eq!(
        server
            .outcome_envelope(outcome("cancelled", vec![lsn]))
            .await
            .items[0]["assessment"],
        "not_executed"
    );

    assert!(
        server
            .predict_envelope(prediction("untrusted", "v2"))
            .await
            .ok
    );
    let lsn = append(
        &actor,
        hm_ledger::frame::EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: serde_json::to_vec(&revision(Some("v2"), true, true)).unwrap(),
        })),
        Authority::DerivedInference,
    )
    .await;
    let before = actor.stats().await.unwrap().log_events;
    let response = server
        .outcome_envelope(outcome("untrusted", vec![lsn]))
        .await;
    assert!(!response.ok);
    assert_eq!(response.items[0]["error"], "kCitationInvalid");
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    let report = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/calibration".to_owned()),
        })
        .await;
    assert!(report.ok);
    let counts = report.items[0]["calibration"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["predicate_kind"] == "revision_equals")
        .unwrap();
    assert_eq!(counts["supported"], 1);
    assert_eq!(counts["pending"], 0);
    assert_eq!(counts["contradicted"], 3);
    assert_eq!(counts["unresolvable"], 1);
    assert_eq!(counts["not_executed"], 1);
    let expected = report.items[0]["calibration"].clone();
    drop(server);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config).await.unwrap();
    let server = McpServer::new(actor.clone());
    let report = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/calibration".to_owned()),
        })
        .await;
    assert!(report.ok);
    assert_eq!(report.items[0]["calibration"], expected);
    let attention = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/attention".to_owned()),
        })
        .await;
    assert!(attention.ok);
    assert!(attention.items[0]["attention"].as_array().unwrap().len() <= 256);
    actor.shutdown().await.unwrap();
}
