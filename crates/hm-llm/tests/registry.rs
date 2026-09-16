#![forbid(unsafe_code)]

use hm_llm::cost::{PINNED_PRICE_TABLE_VERSION, RunBudget, RunCost, pricing_for};
use hm_llm::registry::PromptRegistry;
use hm_llm::{LlmError, Usage};
use std::path::Path;

#[test]
fn prompt_wording_is_frozen_per_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    let registry = PromptRegistry::load(&root).unwrap();
    assert_eq!(registry.len(), 9);
    let snapshots = [
        (
            "abstract-synthesis",
            1,
            "e99a380afb2b9a12ee0ce293e9817ea9247208cb8658f512438c33e8b46dffae",
        ),
        (
            "abstract-synthesis",
            2,
            "e601ec458a7701d106a107c53abfaea7ae689b39ee9c4d12b8bed22a83b24657",
        ),
        (
            "connect-long-context",
            1,
            "85c003c86ff84d861d520a7d8f35583a41131f70c58c3e3c8984a4f2d2585ce7",
        ),
        (
            "edge-discover",
            1,
            "a92c3c3a5f9b03fc86cf61b17cf1c2ab8e1674c10a3a55132000a63d83392609",
        ),
        (
            "hindsight-review",
            1,
            "851e8930b922217af99f05c7a4648a11e0621ede09c39b0e8b278bcf40eda42a",
        ),
        (
            "hindsight-review",
            3,
            "3c5c6da7ef1c8ef6e4e6ea05b1261430f84c7d05a1dea290791b359c4d43f6a7",
        ),
        (
            "merge-cluster",
            1,
            "6acd857b60f0ba8a1513575ba1183d6b29f82b8ab22862b0350023719f227a2c",
        ),
        (
            "refine-definition",
            1,
            "e8301da5e5b32c745d483662960b3b320fb259e560ca90f6b3bfe91e0ce9d71a",
        ),
        (
            "supersession",
            1,
            "ca1b54b5b68ec6432862da53c6e606ef80f57152d7c97da2c6469f768ed8a806",
        ),
    ];
    for (id, version, expected) in snapshots {
        assert_eq!(
            encode_digest(registry.get(id, version).unwrap().digest),
            expected,
            "snapshot for {id}@{version}"
        );
        let expected = decode_digest(expected);
        registry.verify_snapshot(id, version, expected).unwrap();
    }
    assert_eq!(registry.latest("abstract-synthesis").unwrap().version, 2);
    assert_eq!(registry.latest("hindsight-review").unwrap().version, 3);
}

fn encode_digest(value: [u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn pinned_prices_account_per_call_and_enforce_run_budgets() {
    assert_eq!(PINNED_PRICE_TABLE_VERSION, "2026-09-01");
    let pricing = pricing_for("gemini-2.5-flash").unwrap();
    let first = Usage {
        input_tokens: 1_000_000,
        output_tokens: 100_000,
        cache_read_tokens: 50_000,
        cache_write_tokens: 0,
        cost_microusd: 0,
    }
    .with_cost(pricing)
    .unwrap();
    assert_eq!(first.cost_microusd, 550_000);
    let budget = RunBudget {
        max_calls: 2,
        max_tokens: 2_200_000,
        max_microusd: 1_100_000,
    };
    let accumulated = budget.admit(RunCost::default(), first).unwrap();
    let accumulated = budget.admit(accumulated, first).unwrap();
    assert_eq!(accumulated.calls, 2);
    assert_eq!(accumulated.tokens(), 2_200_000);
    assert!((accumulated.usd() - 1.1).abs() < f64::EPSILON);
    assert_eq!(budget.admit(accumulated, first), Err(LlmError::Capacity));
}

fn decode_digest(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64, "replace snapshot placeholder with BLAKE3");
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
