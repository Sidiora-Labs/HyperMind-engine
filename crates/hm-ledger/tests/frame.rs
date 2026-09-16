use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{
    EventKind, FRAME_HEADER_SIZE, Frame, FrameHeader, MAXIMUM_FRAME_BYTES, decode, encode,
};

fn donor_frame(kind: u8) -> Frame {
    let mut conversation = [0_u8; 16];
    conversation[0] = 0x31;
    Frame {
        header: FrameHeader {
            lsn: LSN::new(u64::from(kind)),
            kind: EventKind::try_from(kind).expect("donor event kind"),
            wall_timestamp_ns: UtcNanos::new(-123_456_789 + i64::from(kind)),
            actor: ActorId::new(7),
            conversation: ConversationId::new(conversation),
        },
        sealed_payload: b"sealed-payload".to_vec(),
    }
}

#[test]
fn neocortex_vectors_round_trip_all_legacy_kinds() {
    for kind in 1..=21 {
        let frame = donor_frame(kind);
        let encoded = encode(&frame).expect("encode donor vector");
        assert_eq!(encoded.len(), FRAME_HEADER_SIZE + b"sealed-payload".len());
        assert_eq!(decode(&encoded).expect("decode donor vector"), frame);

        let mut corrupt = encoded.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0x80;
        assert_eq!(
            decode(&corrupt).expect_err("checksum must fail").code,
            ErrorCode::ChecksumMismatch
        );
        assert_eq!(
            decode(&encoded[..FRAME_HEADER_SIZE - 1])
                .expect_err("short frame must fail")
                .code,
            ErrorCode::Truncated
        );
    }
}

#[test]
fn hypermind_event_kind_extensions_round_trip_without_renumbering() {
    for kind in 22..=47 {
        let frame = donor_frame(kind);
        assert_eq!(decode(&encode(&frame).unwrap()).unwrap(), frame);
        assert_eq!(frame.header.kind as u8, kind);
    }
    assert!(EventKind::try_from(48).is_err());
}

#[test]
fn encoding_is_byte_exact_with_donor_fixture() {
    let expected: Vec<u8> = include_str!("../fixtures/frame_kind_01.hex")
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                .expect("valid hex byte")
        })
        .collect();
    let actual = encode(&donor_frame(1)).expect("encode fixture");
    assert_eq!(actual, expected);
}

#[test]
fn invalid_headers_and_lengths_fail_closed() {
    let mut frame = donor_frame(1);
    frame.header.lsn = LSN::new(0);
    assert_eq!(
        encode(&frame).expect_err("zero LSN").code,
        ErrorCode::SequenceViolation
    );

    frame.header.lsn = LSN::new(1);
    frame.sealed_payload = vec![0; MAXIMUM_FRAME_BYTES - FRAME_HEADER_SIZE + 1];
    assert_eq!(
        encode(&frame).expect_err("oversize").code,
        ErrorCode::InvalidLength
    );

    let mut encoded = encode(&donor_frame(1)).expect("valid frame");
    encoded[16] = 0;
    let checksum = crc32c::crc32c(&encoded[8..]);
    encoded[4..8].copy_from_slice(&checksum.to_le_bytes());
    assert_eq!(
        decode(&encoded).expect_err("unknown kind").code,
        ErrorCode::InvalidKind
    );
}
