use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};

pub fn frames(count: usize) -> Vec<Frame> {
    (0..count)
        .map(|index| {
            let lsn = u64::try_from(index).expect("test index") + 1;
            let mut conversation = [0_u8; 16];
            conversation[0] = u8::try_from(index % 13).expect("conversation byte");
            conversation[1] = 0x4d;
            let text = if index % 2 == 0 {
                format!("alpha durable memory item {index}")
            } else {
                format!("beta delivered response {index}")
            };
            let (kind, payload) = if index % 2 == 0 {
                (
                    EventKind::UserMsg,
                    EventPayload::UserMsg(Box::new(UserMsg {
                        content: text.into_bytes(),
                    })),
                )
            } else {
                (
                    EventKind::DeliveredMsg,
                    EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                        content: text.into_bytes(),
                    })),
                )
            };
            let envelope = EventEnvelope {
                schema_version: 2,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 0,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::UserAsserted,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Public,
                event_time_ns: 0,
            };
            Frame {
                header: FrameHeader {
                    lsn: LSN::new(lsn),
                    kind,
                    wall_timestamp_ns: UtcNanos::new(
                        5_000_000 + i64::try_from(index).expect("timestamp"),
                    ),
                    actor: ActorId::new(41),
                    conversation: ConversationId::new(conversation),
                },
                sealed_payload: encode_event_envelope(&envelope),
            }
        })
        .collect()
}
