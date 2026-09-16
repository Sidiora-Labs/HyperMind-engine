#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_index::vocabulary::{
    AliasSuggestion, DEFAULT_SIMILARITY_THRESHOLD_Q16, MAXIMUM_CANDIDATES, MAXIMUM_NAME_CHARS,
    Q16_ONE, normalize_name, normalized_segments, similarity_q16, suggest_aliases,
};

#[test]
fn normalization_is_unicode_aware_and_idempotent() {
    let cases = [
        ("Works For", "works_for"),
        ("works_for", "works_for"),
        ("  --Café Client--  ", "café_client"),
        ("ÄÖÜ", "äöü"),
        ("---", ""),
    ];
    for (input, expected) in cases {
        let normalized = normalize_name(input);
        assert_eq!(normalized, expected, "normalizing {input:?}");
        assert_eq!(
            normalize_name(&normalized),
            normalized,
            "normalization of {input:?} is idempotent"
        );
    }
    assert_eq!(
        normalized_segments("  Café  --  Client Café ", 8),
        ["café", "client"]
    );
    assert_eq!(
        normalized_segments("beta alpha gamma", 2),
        ["alpha", "beta"]
    );
    assert!(normalized_segments("---", 8).is_empty());
}

#[test]
fn similarity_is_symmetric_fixed_point_and_character_counted() {
    let pairs = [
        ("Works For", "works_for", Q16_ONE),
        ("vehicles", "vehicles", Q16_ONE),
        ("vehicles", "vehicle", 60_494),
        ("car", "cart", 52_428),
        ("", "vehicle", 0),
        ("---", "vehicle", 0),
    ];
    for (left, right, expected) in pairs {
        assert_eq!(similarity_q16(left, right), expected, "{left:?} {right:?}");
        assert_eq!(
            similarity_q16(right, left),
            expected,
            "symmetry for {left:?} {right:?}"
        );
    }
    assert_eq!(Q16_ONE, 65_536);

    let multi_byte = similarity_q16("café", "cafés");
    let single_byte = similarity_q16("cafe", "cafes");
    assert_eq!(multi_byte, single_byte);
    assert_eq!(multi_byte, 56_173);

    let over_length = "é".repeat(MAXIMUM_NAME_CHARS + 1);
    assert_eq!(over_length.chars().count(), MAXIMUM_NAME_CHARS + 1);
    assert_eq!(similarity_q16(&over_length, &over_length), 0);
    assert_eq!(similarity_q16(&over_length, "vehicle"), 0);
    let at_length = "é".repeat(MAXIMUM_NAME_CHARS);
    assert_eq!(similarity_q16(&at_length, &at_length), Q16_ONE);
}

#[test]
fn suggestions_are_thresholded_ordered_and_never_merge() {
    let candidates = ["vehicle", "car", "vehicles"];
    assert_eq!(
        suggest_aliases("vehicles", &candidates, DEFAULT_SIMILARITY_THRESHOLD_Q16, 8)
            .expect("default threshold suggestions"),
        [
            AliasSuggestion {
                candidate: "vehicles".to_owned(),
                similarity_q16: 65_536,
            },
            AliasSuggestion {
                candidate: "vehicle".to_owned(),
                similarity_q16: 60_494,
            },
        ]
    );
    assert_eq!(
        suggest_aliases("vehicles", &candidates, Q16_ONE, 8).expect("exact threshold suggestions"),
        [AliasSuggestion {
            candidate: "vehicles".to_owned(),
            similarity_q16: 65_536,
        }]
    );
    assert!(
        suggest_aliases("cart", &["car"], DEFAULT_SIMILARITY_THRESHOLD_Q16, 8)
            .expect("below threshold suggestions")
            .is_empty()
    );

    assert_eq!(
        suggest_aliases("vehicles", &candidates, DEFAULT_SIMILARITY_THRESHOLD_Q16, 0)
            .expect_err("zero limit")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        suggest_aliases("vehicles", &candidates, 0, 8)
            .expect_err("zero threshold")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        suggest_aliases("vehicles", &candidates, Q16_ONE + 1, 8)
            .expect_err("threshold above one")
            .code,
        ErrorCode::InvalidArgument
    );
    let too_many = vec!["vehicle"; MAXIMUM_CANDIDATES + 1];
    assert_eq!(
        suggest_aliases("vehicles", &too_many, DEFAULT_SIMILARITY_THRESHOLD_Q16, 8)
            .expect_err("too many candidates")
            .code,
        ErrorCode::InvalidArgument
    );
}
