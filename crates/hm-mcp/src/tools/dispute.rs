use crate::Envelope;
use crate::tools::believe::{BeliefClaimInput, append, assertion};
use hm_core::{Error, ErrorCode};
use hm_cortex::adjudicate::{AdjudicationError, AdjudicationOutcome, dispute};
use hm_cortex::nli::{NliModel, NliVerdict};
use hm_ledger::frame::EventKind;
use hm_llm::{LlmProvider, ModelTier};
use hm_schema::events::{Authority, BeliefType, EventPayload, ProposedAssertion};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct DisputeRuntime {
    nli: Arc<NliModel>,
    provider: Option<Arc<dyn LlmProvider>>,
    minimum_tier: ModelTier,
}

impl DisputeRuntime {
    #[must_use]
    pub fn new(
        nli: Arc<NliModel>,
        provider: Option<Arc<dyn LlmProvider>>,
        minimum_tier: ModelTier,
    ) -> Self {
        Self {
            nli,
            provider,
            minimum_tier,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct DisputeInput {
    pub conversation: String,
    pub existing: BeliefClaimInput,
    pub incoming: BeliefClaimInput,
}

pub async fn run(
    actor: &ActorEngine,
    runtime: Option<&DisputeRuntime>,
    input: DisputeInput,
) -> Result<Envelope, Error> {
    if input.conversation.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let existing = assertion(&input.existing);
    let incoming = assertion(&input.incoming);
    let local;
    let runtime = if let Some(runtime) = runtime {
        runtime
    } else {
        local = default_runtime()?;
        &local
    };
    let report = dispute(
        &runtime.nli,
        runtime.provider.as_deref(),
        runtime.minimum_tier,
        &existing,
        &incoming,
    )
    .map_err(|error| adjudication_error(&error))?;
    let mut envelope = Envelope::empty();
    let (action, events) = apply_outcome(
        actor,
        &input.conversation,
        &existing,
        &incoming,
        report.outcome,
    )
    .await?;
    for lsn in &events {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    }
    envelope.items.push(json!({
        "verdict": verdict_name(report.nli.verdict),
        "confidence": report.nli.confidence,
        "suggested_action": action,
        "resulting_events": events,
        "model_id": report.model_id,
        "usage": {
            "input_tokens": report.usage.input_tokens,
            "output_tokens": report.usage.output_tokens,
            "cost_microusd": report.usage.cost_microusd,
        },
    }));
    envelope.health = json!({"projection": "ready", "nli": "ready"});
    Ok(envelope)
}

async fn apply_outcome(
    actor: &ActorEngine,
    conversation: &str,
    existing: &hm_schema::events::Assertion,
    incoming: &hm_schema::events::Assertion,
    outcome: AdjudicationOutcome,
) -> Result<(&'static str, Vec<u64>), Error> {
    let result = match outcome {
        AdjudicationOutcome::Replace { retract, assertion } if protected(assertion.belief_type) => {
            let proposal = ProposedAssertion {
                belief_id: assertion.belief_id,
                belief_type: assertion.belief_type,
                canonical_identity: assertion.canonical_identity,
                value: assertion.value,
                valid_from_ns: assertion.valid_from_ns,
                valid_to_ns: assertion.valid_to_ns,
                provenance: assertion.provenance,
                conflict_domain: assertion.conflict_domain,
                claim: assertion.claim,
            };
            let outcome = append(
                actor,
                conversation,
                EventKind::ProposedAssertion,
                EventPayload::ProposedAssertion(Box::new(proposal)),
                Authority::DerivedInference,
                Some(dispute_run_id(existing, incoming)),
            )
            .await?;
            let _ = retract;
            ("propose_protected", vec![outcome.first_lsn.get()])
        }
        AdjudicationOutcome::Replace { retract, assertion } => {
            let run_id = dispute_run_id(existing, incoming);
            let first = append(
                actor,
                conversation,
                EventKind::Retract,
                EventPayload::Retract(Box::new(retract)),
                Authority::DerivedInference,
                Some(run_id.clone()),
            )
            .await?;
            let second = append(
                actor,
                conversation,
                EventKind::Assertion,
                EventPayload::Assertion(Box::new(assertion)),
                Authority::DerivedInference,
                Some(run_id),
            )
            .await?;
            (
                "replace",
                vec![first.first_lsn.get(), second.first_lsn.get()],
            )
        }
        AdjudicationOutcome::Conflict(_) => {
            let outcome = append(
                actor,
                conversation,
                EventKind::Assertion,
                EventPayload::Assertion(Box::new(incoming.clone())),
                Authority::DerivedInference,
                Some(dispute_run_id(existing, incoming)),
            )
            .await?;
            ("surface_conflict", vec![outcome.first_lsn.get()])
        }
        AdjudicationOutcome::UnverifiedTension => ("review", Vec::new()),
        AdjudicationOutcome::NoChange => ("keep_existing", Vec::new()),
    };
    Ok(result)
}

fn default_runtime() -> Result<DisputeRuntime, Error> {
    let cache = std::env::var_os("HYPERMIND_MODEL_CACHE")
        .map_or_else(|| PathBuf::from("target/hm-models"), PathBuf::from);
    NliModel::download(&cache)
        .map(|model| DisputeRuntime::new(Arc::new(model), None, ModelTier::Capable))
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))
}

fn adjudication_error(error: &AdjudicationError) -> Error {
    match error {
        AdjudicationError::InvalidUtf8 | AdjudicationError::InvalidBelief => {
            Error::new(ErrorCode::InvalidArgument)
        }
        AdjudicationError::Nli(_)
        | AdjudicationError::Llm(_)
        | AdjudicationError::InvalidStructuredOutput => Error::new(ErrorCode::OperationUnavailable),
    }
}

fn dispute_run_id(
    existing: &hm_schema::events::Assertion,
    incoming: &hm_schema::events::Assertion,
) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.dispute.v1\0");
    hasher.update(&existing.belief_id);
    hasher.update(&incoming.belief_id);
    hasher.finalize().as_bytes().to_vec()
}

const fn protected(belief_type: BeliefType) -> bool {
    matches!(
        belief_type,
        BeliefType::Identity | BeliefType::Constraint | BeliefType::Preference
    )
}

const fn verdict_name(verdict: NliVerdict) -> &'static str {
    match verdict {
        NliVerdict::Genuine => "genuine",
        NliVerdict::Tension => "tension",
        NliVerdict::Complementary => "complementary",
        NliVerdict::Unrelated => "unrelated",
    }
}
