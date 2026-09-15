#![forbid(unsafe_code)]

use hm_cortex::adjudicate::{AdjudicationOutcome, SUPERSESSION_PROMPT, dispute};
use hm_cortex::nli::{NliModel, NliVerdict};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_schema::events::{Assertion, AssertionClaim, BeliefType, ProvenanceRange};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn model() -> NliModel {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/hm-models");
    NliModel::download(&cache).expect("pinned in-process NLI model")
}

fn assertion(id: &[u8], identity: &str, value: &str, valid_from_ns: i64) -> Assertion {
    Assertion {
        belief_id: id.to_vec(),
        belief_type: BeliefType::Fact,
        canonical_identity: identity.to_owned(),
        value: value.as_bytes().to_vec(),
        valid_from_ns,
        valid_to_ns: 0,
        provenance: vec![ProvenanceRange {
            first_lsn: 7,
            last_lsn: 7,
            byte_start: 0,
            byte_end: u32::try_from(value.len()).expect("value length"),
        }],
        conflict_domain: Some("deployment:region".to_owned()),
        claim: AssertionClaim::Affirmative,
    }
}

fn prompt(existing: &Assertion, incoming: &Assertion) -> String {
    format!(
        "existing_identity: {}\nexisting_valid: {}..{}\nexisting_value: {}\n\nincoming_identity: {}\nincoming_valid: {}..{}\nincoming_value: {}",
        existing.canonical_identity,
        existing.valid_from_ns,
        existing.valid_to_ns,
        std::str::from_utf8(&existing.value).unwrap(),
        incoming.canonical_identity,
        incoming.valid_from_ns,
        incoming.valid_to_ns,
        std::str::from_utf8(&incoming.value).unwrap(),
    )
}

fn provider(
    tier: ModelTier,
    existing: &Assertion,
    incoming: &Assertion,
    supersedes: bool,
) -> OpenAiCompatible<RecordedTransport> {
    let schema = json!({
        "type": "object",
        "properties": {
            "supersedes": {"type": "boolean"},
            "reason": {"type": "string"}
        },
        "required": ["supersedes", "reason"],
        "additionalProperties": false
    });
    let fixture = WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: "https://fixture.invalid/v1/chat/completions".to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": "fixture-capable",
                "messages": [
                    {"role": "system", "content": SUPERSESSION_PROMPT},
                    {"role": "user", "content": prompt(existing, incoming)}
                ],
                "max_tokens": 256,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": "supersession_1",
                        "strict": true,
                        "schema": schema
                    }
                }
            }),
        },
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": serde_json::to_string(&json!({
                    "supersedes": supersedes,
                    "reason": "the incoming claim has a later validity interval"
                })).unwrap()}}],
                "usage": {"prompt_tokens": 30, "completion_tokens": 12}
            }),
        },
    };
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-capable".to_owned(),
            tier,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(vec![fixture]),
    )
    .unwrap()
}

#[test]
fn in_process_nli_runs_both_directions_and_separates_unrelated_claims() {
    let nli = model();
    let contradictory = nli
        .classify_both(
            "The deployment region is Europe.",
            "The deployment region is America.",
        )
        .expect("bidirectional NLI");
    assert_eq!(contradictory.verdict, NliVerdict::Genuine);
    assert!(contradictory.confidence >= 0.8);
    let unrelated = nli
        .classify_both(
            "The deployment region is Europe.",
            "The preferred editor color is blue.",
        )
        .expect("unrelated NLI");
    assert_ne!(unrelated.verdict, NliVerdict::Genuine);
}

#[test]
fn genuine_dispute_makes_exactly_one_tier_capped_supersession_call() {
    let nli = model();
    let existing = assertion(
        b"old",
        "deployment:region",
        "The deployment region is Europe.",
        10,
    );
    let incoming = assertion(
        b"new",
        "deployment:region",
        "The deployment region is America.",
        20,
    );

    let no_provider =
        dispute(&nli, None, ModelTier::Capable, &existing, &incoming).expect("no-provider dispute");
    assert_eq!(no_provider.outcome, AdjudicationOutcome::UnverifiedTension);
    assert_eq!(no_provider.usage.output_tokens, 0);

    let low_tier = provider(ModelTier::Economy, &existing, &incoming, true);
    let gated = dispute(
        &nli,
        Some(&low_tier),
        ModelTier::Capable,
        &existing,
        &incoming,
    )
    .expect("tier-gated dispute");
    assert_eq!(gated.outcome, AdjudicationOutcome::UnverifiedTension);
    assert_eq!(low_tier.transport().remaining(), 1);

    let capable = provider(ModelTier::Capable, &existing, &incoming, true);
    let replaced = dispute(
        &nli,
        Some(&capable),
        ModelTier::Capable,
        &existing,
        &incoming,
    )
    .expect("supersession dispute");
    let AdjudicationOutcome::Replace { retract, assertion } = replaced.outcome else {
        panic!("expected replacement");
    };
    assert_eq!(retract.belief_id, b"old");
    assert_eq!(assertion.belief_id, b"new");
    assert_eq!(replaced.usage.input_tokens, 30);
    assert_eq!(replaced.usage.output_tokens, 12);
    assert_eq!(replaced.usage.cost_microusd, 54);
    assert_eq!(capable.transport().remaining(), 0);
}

#[test]
fn failed_supersession_keeps_an_obligated_conflict_edge() {
    let nli = model();
    let existing = assertion(
        b"old",
        "deployment:region:europe",
        "The deployment region is Europe.",
        10,
    );
    let incoming = assertion(
        b"new",
        "deployment:region:america",
        "The deployment region is America.",
        20,
    );
    let capable = provider(ModelTier::Capable, &existing, &incoming, false);
    let report = dispute(
        &nli,
        Some(&capable),
        ModelTier::Standard,
        &existing,
        &incoming,
    )
    .expect("conflict dispute");
    let AdjudicationOutcome::Conflict(conflict) = report.outcome else {
        panic!("expected conflict");
    };
    assert!(conflict.obligated_surfacing);
    assert_eq!(conflict.left_belief_id, b"old");
    assert_eq!(conflict.right_belief_id, b"new");
    assert_eq!(capable.transport().remaining(), 0);
}
