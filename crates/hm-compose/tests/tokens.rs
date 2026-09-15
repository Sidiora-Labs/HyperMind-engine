use hm_compose::tokens::{FallbackWeights, TokenCounter};

#[test]
fn tiktoken_counts_for_model_exactly() {
    let counter = TokenCounter::for_model("gpt-4o", None, FallbackWeights::default())
        .expect("tiktoken counter");
    assert_eq!(counter.count(b"hello world").expect("count"), 2);
}

#[test]
fn hugging_face_tokenizer_json_is_keyed_by_model() {
    let tokenizer = br#"{
      "version":"1.0","truncation":null,"padding":null,"added_tokens":[],
      "normalizer":null,"pre_tokenizer":{"type":"Whitespace"},
      "post_processor":null,"decoder":null,
      "model":{"type":"WordLevel","vocab":{"[UNK]":0,"hello":1,"world":2},"unk_token":"[UNK]"}
    }"#;
    let counter = TokenCounter::for_model(
        "local-word-level",
        Some(tokenizer),
        FallbackWeights::default(),
    )
    .expect("HF counter");
    assert_eq!(counter.model_id(), "local-word-level");
    assert_eq!(counter.count(b"hello world").expect("count"), 2);
}

#[test]
fn neocortex_q8_byte_weights_are_the_offline_fallback() {
    let mut weights = FallbackWeights {
        per_byte_q8: [0; 256],
        item_overhead: 1,
    };
    weights.per_byte_q8[usize::from(b'a')] = 128;
    let counter = TokenCounter::for_model("unknown-model", None, weights).expect("fallback");
    assert_eq!(counter.count(b"aa").expect("count"), 2);
    assert_eq!(counter.count(b"").expect("minimum count"), 1);
}
