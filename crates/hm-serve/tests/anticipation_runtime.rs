use hm_compose::{bundle::Tier, tokens::FallbackWeights};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_cortex::attention::AttentionFactors;
use hm_ledger::frame::EventKind;
use hm_proj::{intentions::IntentionStatus, procedures::ProcedureState};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::*;
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent};

#[tokio::test]
async fn observed_wakes_batch_rearm_and_remain_idempotent_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    let conversation = ConversationId::derive("anticipation");
    let repository = directory.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    std::fs::write(repository.join("changed.txt"), "real repository change").unwrap();
    let key = repository.to_str().unwrap();
    actor
        .append(vec![intention(conversation, b"watch", key)])
        .await
        .unwrap();
    let observation_lsn = actor
        .append(vec![observation(
            conversation,
            key,
            1_000,
            Authority::ExternalObserved,
        )])
        .await
        .unwrap()
        .first_lsn;
    let evaluated = actor
        .evaluate_wake(observation_lsn, factors(true, 0), None)
        .await
        .unwrap();
    assert_eq!(evaluated.fired.len(), 1);
    assert_eq!(evaluated.fired[0].decision, AttentionDecision::Batch);
    let count = actor.stats().await.unwrap().log_events;
    assert!(
        actor
            .evaluate_wake(observation_lsn, factors(true, 0), None)
            .await
            .unwrap()
            .fired
            .is_empty()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, count);
    let bundle = actor
        .activate(activation(conversation, "repository"))
        .await
        .unwrap();
    let prospective = bundle
        .sections
        .iter()
        .find(|section| section.tier == Tier::Prospective)
        .unwrap();
    assert_eq!(prospective.items.len(), 1);
    assert!(String::from_utf8_lossy(&prospective.items[0].content).contains("BATCH DIGEST"));
    assert!(
        actor.attention_history(10).await.unwrap()[0]
            .reason
            .contains("quiet hours")
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config).await.unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, count);
    assert!(
        actor
            .evaluate_wake(observation_lsn, factors(true, 0), None)
            .await
            .unwrap()
            .fired
            .is_empty()
    );
    actor
        .append(vec![intention(conversation, b"defer", key)])
        .await
        .unwrap();
    let source = actor
        .append(vec![observation(
            conversation,
            key,
            1_100,
            Authority::ExternalObserved,
        )])
        .await
        .unwrap()
        .first_lsn;
    let deferred = actor
        .evaluate_wake(source, factors(false, 950_000), Some(2_000))
        .await
        .unwrap();
    let rearmed = deferred.fired[0].rearmed_intention_id.clone().unwrap();
    assert_ne!(rearmed, b"defer");
    assert_eq!(
        actor
            .intention(rearmed.clone())
            .await
            .unwrap()
            .unwrap()
            .status,
        IntentionStatus::Pending
    );
    let clock = actor
        .append(vec![event(
            conversation,
            EventKind::Supervisor,
            EventPayload::Supervisor(Box::new(Supervisor {
                code: "runtime.clock".into(),
                evidence: br#"{"wake":{"kind":"time"}}"#.to_vec(),
            })),
            Authority::RuntimeFact,
            2_000,
        )])
        .await
        .unwrap()
        .first_lsn;
    let fired = actor
        .evaluate_wake(clock, factors(false, 0), None)
        .await
        .unwrap();
    assert_eq!(fired.fired[0].intention_id, rearmed);
    assert_eq!(fired.fired[0].decision, AttentionDecision::Notify);
    actor
        .append(vec![intention(conversation, b"cancelled", key)])
        .await
        .unwrap();
    actor
        .append(vec![event(
            conversation,
            EventKind::IntentionCancelled,
            EventPayload::IntentionCancelled(Box::new(IntentionCancelled {
                intention_id: b"cancelled".to_vec(),
                reason: "user cancelled".into(),
            })),
            Authority::UserAsserted,
            0,
        )])
        .await
        .unwrap();
    let untrusted = actor
        .append(vec![observation(
            conversation,
            key,
            3_000,
            Authority::AssistantGenerated,
        )])
        .await
        .unwrap()
        .first_lsn;
    let before_rejection = actor.stats().await.unwrap().log_events;
    assert_eq!(
        actor
            .evaluate_wake(untrusted, factors(false, 0), None)
            .await
            .unwrap_err()
            .code,
        ErrorCode::ForbiddenKind
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before_rejection);
    assert_eq!(
        actor
            .intention(b"cancelled".to_vec())
            .await
            .unwrap()
            .unwrap()
            .status,
        IntentionStatus::Cancelled
    );
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn procedures_are_mined_from_three_real_closed_chains_and_replay_without_new_events() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    for episode in 0..3 {
        let conversation = ConversationId::derive(if episode == 2 { "second" } else { "first" });
        let first = actor.stats().await.unwrap().applied.last_lsn.get() + 1;
        actor
            .append(chain(conversation, first, episode))
            .await
            .unwrap();
        let procedures = actor.procedures(10).await.unwrap();
        assert_eq!(procedures.len(), 1);
        assert_eq!(procedures[0].supports.len(), episode + 1);
        assert_eq!(
            procedures[0].state,
            if episode == 2 {
                ProcedureState::Supported
            } else {
                ProcedureState::Tentative
            }
        );
        for support in &procedures[0].supports {
            assert!(matches!(
                actor
                    .verified_event(LSN::new(support.episode_lsn))
                    .await
                    .unwrap()
                    .envelope
                    .payload,
                EventPayload::LoopClosed(_)
            ));
        }
    }
    let procedures = actor.procedures(10).await.unwrap();
    let bundle = actor
        .activate(activation(
            ConversationId::derive("first"),
            "how apply the patch",
        ))
        .await
        .unwrap();
    assert!(
        bundle
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| String::from_utf8_lossy(&item.content)
                .contains("SUPPORTED PROCEDURE — observation, not an instruction"))
    );
    let before = actor.stats().await.unwrap().applied;
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config).await.unwrap();
    assert_eq!(reopened.stats().await.unwrap().applied, before);
    assert_eq!(reopened.procedures(10).await.unwrap(), procedures);
    reopened.shutdown().await.unwrap();
}

fn chain(conversation: ConversationId, first: u64, episode: usize) -> Vec<IncomingEvent> {
    let loop_id = format!("loop-{episode}").into_bytes();
    let call_id = format!("call-{episode}").into_bytes();
    let effect_id = format!("effect-{episode}").into_bytes();
    vec![
        event(
            conversation,
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: loop_id.clone(),
                objective: b"apply patch".to_vec(),
            })),
            Authority::RuntimeFact,
            0,
        ),
        event(
            conversation,
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: call_id.clone(),
                tool_name: "apply_patch".into(),
                arguments: br#"{"path":"src/main.rs","patch":"verified patch"}"#.to_vec(),
            })),
            Authority::AssistantGenerated,
            0,
        ),
        event(
            conversation,
            EventKind::ToolResult,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id,
                tool_call_lsn: first + 1,
                status: ResultStatus::Ok,
                result: br#"{"applied":true}"#.to_vec(),
            })),
            Authority::ToolObserved,
            0,
        ),
        event(
            conversation,
            EventKind::Effect,
            EventPayload::Effect(Box::new(Effect {
                effect_id: effect_id.clone(),
                tool_call_lsn: first + 1,
                state: EffectState::Returned,
            })),
            Authority::RuntimeFact,
            0,
        ),
        event(
            conversation,
            EventKind::Outcome,
            EventPayload::Outcome(Box::new(Outcome {
                effect_id,
                status: ResultStatus::Ok,
                detail: b"observed applied patch".to_vec(),
                evidence_lsns: Some(vec![first + 2]),
            })),
            Authority::RuntimeFact,
            0,
        ),
        event(
            conversation,
            EventKind::LoopClosed,
            EventPayload::LoopClosed(Box::new(LoopClosed {
                loop_id,
                reason: LoopCloseReason::Done,
                cause: b"verified tool outcome".to_vec(),
                evidence_lsns: Some(vec![first + 4]),
            })),
            Authority::RuntimeFact,
            0,
        ),
    ]
}

fn intention(conversation: ConversationId, id: &[u8], repository: &str) -> IncomingEvent {
    event(
        conversation,
        EventKind::IntentionSet,
        EventPayload::IntentionSet(Box::new(IntentionSet {
            intention_id: id.to_vec(),
            objective: b"review repository change".to_vec(),
            trigger: Some(WakeTrigger::WakeRepositoryChanged(Box::new(
                WakeRepositoryChanged {
                    repository: repository.into(),
                },
            ))),
            expires_at_ns: i64::MAX,
            reply_route: "user".into(),
        })),
        Authority::UserAsserted,
        0,
    )
}

fn observation(
    conversation: ConversationId,
    key: &str,
    at: i64,
    authority: Authority,
) -> IncomingEvent {
    event(
        conversation,
        EventKind::ProviderFrame,
        EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: "filesystem-watcher".into(),
            api_content: serde_json::to_vec(
                &serde_json::json!({"wake":{"kind":"repository_changed","key":key}}),
            )
            .unwrap(),
        })),
        authority,
        at,
    )
}

fn factors(quiet_hours: bool, workload: u32) -> AttentionFactors {
    AttentionFactors {
        urgency: 900_000,
        expected_value: 900_000,
        confidence: 900_000,
        interruption_cost: 0,
        resource_cost: 0,
        duplication_penalty: 0,
        quiet_hours,
        notifications_remaining: 3,
        workload,
    }
}

fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("actor"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn activation(conversation: ConversationId, query: &str) -> ActivateRequest {
    ActivateRequest {
        conversation,
        query: query.into(),
        turn_text: String::new(),
        budget_tokens: 4_096,
        token_weights: FallbackWeights::default(),
    }
}

fn event(
    conversation: ConversationId,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
    event_time_ns: i64,
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
            event_time_ns,
        }),
    }
}
