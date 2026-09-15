use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};

pub fn conversation(first: u8) -> ConversationId {
    let mut bytes = [0_u8; 16];
    bytes[0] = first;
    ConversationId::new(bytes)
}

pub fn message_frame(
    lsn: u64,
    conversation: ConversationId,
    kind: EventKind,
    content: &str,
) -> Frame {
    let payload = match kind {
        EventKind::UserMsg => EventPayload::UserMsg(Box::new(UserMsg {
            content: content.as_bytes().to_vec(),
        })),
        EventKind::DeliveredMsg => EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
            content: content.as_bytes().to_vec(),
        })),
        _ => panic!("message frame requires a message kind"),
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
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).expect("timestamp") * 1_000),
            actor: ActorId::new(19),
            conversation,
        },
        sealed_payload: encode_event_envelope(&envelope),
    }
}

pub fn workload() -> (ConversationId, Vec<Frame>) {
    let current = conversation(0x33);
    let previous = conversation(0x44);
    (
        current,
        vec![
            message_frame(
                1,
                current,
                EventKind::UserMsg,
                "current question\nwith a deliberately long continuation that can be coarsened",
            ),
            message_frame(
                2,
                current,
                EventKind::DeliveredMsg,
                "current answer\nwith another long continuation for the trim law",
            ),
            message_frame(
                3,
                previous,
                EventKind::DeliveredMsg,
                "past alpha result from another conversation",
            ),
            message_frame(
                4,
                previous,
                EventKind::UserMsg,
                "alpha rare fact with a stable provenance",
            ),
        ],
    )
}
