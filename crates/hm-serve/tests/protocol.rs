use hm_core::ErrorCode;
use hm_serve::protocol::{FrameParser, encode_frame};

#[test]
fn ncpr_frame_matches_donor_layout_and_streams() {
    let first = encode_frame(b"abc").unwrap();
    assert_eq!(&first[..4], &[3, 0, 0, 0]);
    assert_eq!(&first[4..8], &crc32c::crc32c(b"abc").to_le_bytes());
    assert_eq!(&first[8..], b"abc");

    let second = encode_frame(b"defgh").unwrap();
    let joined = [first, second].concat();
    let mut parser = FrameParser::default();
    assert!(parser.push(&joined[..5]).unwrap().is_empty());
    assert_eq!(
        parser.push(&joined[5..]).unwrap(),
        vec![b"abc".to_vec(), b"defgh".to_vec()]
    );
}

#[test]
fn ncpr_frame_rejects_corruption_and_oversize_declarations() {
    let mut corrupt = encode_frame(b"abc").unwrap();
    corrupt[8] ^= 1;
    assert_eq!(
        FrameParser::default().push(&corrupt).unwrap_err().code,
        ErrorCode::ChecksumMismatch
    );
    let invalid = [0_u8; 8];
    assert_eq!(
        FrameParser::default().push(&invalid).unwrap_err().code,
        ErrorCode::InvalidLength
    );
}
