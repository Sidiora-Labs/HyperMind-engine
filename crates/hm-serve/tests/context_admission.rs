use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};

fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("actor"),
        actor: ActorId::new(23),
        user: [6; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn message(content: &str) -> IncomingEvent {
    IncomingEvent {
        kind: EventKind::UserMsg,
        conversation: ConversationId::derive("admission"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::UserMsg(Box::new(UserMsg {
                content: content.as_bytes().to_vec(),
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::UserAsserted,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

#[tokio::test]
async fn stale_tail_rejects_entire_batch_without_ledger_or_projection_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let captured = actor.stats().await.unwrap().applied.last_lsn;
    assert_eq!(captured, LSN::new(0));
    let native = message("native source update");
    let native_outcome = actor.append(vec![native.clone()]).await.unwrap();
    let before = actor.stats().await.unwrap();
    let integrity = actor.verification_status().await.unwrap();
    let mut subscription = actor.subscribe();
    let error = actor
        .append_if_tail(
            captured,
            vec![message("obsolete result"), message("obsolete followup")],
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::SequenceViolation);
    assert_eq!(error.lsn, native_outcome.last_lsn);
    assert_eq!(actor.stats().await.unwrap(), before);
    assert_eq!(actor.verification_status().await.unwrap(), integrity);
    assert!(matches!(
        subscription.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    let frames = actor.frames_since(LSN::new(0), None, 16).await.unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].sealed_payload, native.payload);
    let fresh = actor
        .append_if_tail(before.applied.last_lsn, vec![message("fresh result")])
        .await
        .unwrap();
    assert_eq!(fresh.first_lsn.get(), 2);
    assert_eq!(subscription.try_recv().unwrap().header.lsn, fresh.last_lsn);
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(directory.path())).await.unwrap();
    assert_eq!(reopened.stats().await.unwrap().log_events, 2);
    assert_eq!(
        reopened
            .frames_since(LSN::new(0), None, 16)
            .await
            .unwrap()
            .len(),
        2
    );
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_conditional_batches_have_one_winner_and_loser_can_retry() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let expected = actor.stats().await.unwrap().applied.last_lsn;
    let left = message("left candidate");
    let right = message("right candidate");
    let (left_result, right_result) = tokio::join!(
        actor.append_if_tail(expected, vec![left.clone()]),
        actor.append_if_tail(expected, vec![right.clone()])
    );
    let (winner, loser, winner_event, loser_event) = match (left_result, right_result) {
        (Ok(winner), Err(loser)) => (winner, loser, left, right),
        (Err(loser), Ok(winner)) => (winner, loser, right, left),
        results => panic!("one conditional append must win: {results:?}"),
    };
    assert_eq!(loser.code, ErrorCode::SequenceViolation);
    assert_eq!(loser.lsn, winner.last_lsn);
    assert_eq!(actor.stats().await.unwrap().log_events, 1);
    let retry = actor
        .append_if_tail(winner.last_lsn, vec![loser_event.clone()])
        .await
        .unwrap();
    assert_eq!(retry.first_lsn.get(), 2);
    let native = message("subsequent native write");
    actor.append(vec![native.clone()]).await.unwrap();
    let frames = actor.frames_since(LSN::new(0), None, 16).await.unwrap();
    assert_eq!(frames.len(), 3);
    assert_eq!(
        frames
            .iter()
            .map(|frame| &frame.sealed_payload)
            .collect::<Vec<_>>(),
        [&winner_event.payload, &loser_event.payload, &native.payload]
    );
    let before = actor.stats().await.unwrap();
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(directory.path())).await.unwrap();
    assert_eq!(reopened.stats().await.unwrap().applied, before.applied);
    assert_eq!(
        reopened.frames_since(LSN::new(0), None, 16).await.unwrap(),
        frames
    );
    reopened.shutdown().await.unwrap();
}
