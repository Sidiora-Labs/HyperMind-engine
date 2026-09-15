use hm_core::ErrorCode;
use hm_index::bm25::{Q32_ONE, fixed_ln_ratio, score_q32};
use hm_index::tokenize::{MAXIMUM_TERM_BYTES, tokenize};

#[test]
fn tantivy_tokenization_is_unicode_lowercase_and_bounded() {
    let too_long = "x".repeat(MAXIMUM_TERM_BYTES + 1);
    let terms = tokenize(&format!(
        "Hello, WORLD! naïve café a 42 snake_case {too_long}"
    ));
    assert_eq!(
        terms,
        ["hello", "world", "naïve", "café", "42", "snake", "case"]
    );
}

#[test]
fn fixed_ln_ratio_matches_neocortex_q32_vectors() {
    assert_eq!(fixed_ln_ratio(1, 1).expect("ln one"), 0);
    assert_eq!(fixed_ln_ratio(2, 1).expect("ln two"), 2_977_044_471);
    assert_eq!(fixed_ln_ratio(4, 1).expect("ln four"), 5_954_088_942);
    let ln_three = fixed_ln_ratio(3, 1).expect("ln three");
    assert!(ln_three > Q32_ONE);
    assert!(ln_three < fixed_ln_ratio(4, 1).expect("ln four"));
    assert_eq!(
        fixed_ln_ratio(1, 2).expect_err("negative logarithm").code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn integer_bm25_rewards_frequency_and_short_documents() {
    let base = score_q32(100, 10, 1_000, 10, 1).expect("base score");
    let repeated = score_q32(100, 10, 1_000, 10, 3).expect("frequency score");
    let long = score_q32(100, 10, 1_000, 100, 1).expect("long score");
    assert!(repeated > base);
    assert!(base > long);
}
