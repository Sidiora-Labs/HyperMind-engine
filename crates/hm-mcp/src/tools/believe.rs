use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::event::{Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope, verify_event};
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, EventEnvelope, EventPayload,
    ProposedAssertion, ProvenanceRange, ResultStatus, Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BeliefTypeInput {
    Fact,
    Preference,
    Constraint,
    Goal,
    Identity,
}

impl From<BeliefTypeInput> for BeliefType {
    fn from(value: BeliefTypeInput) -> Self {
        match value {
            BeliefTypeInput::Fact => Self::Fact,
            BeliefTypeInput::Preference => Self::Preference,
            BeliefTypeInput::Constraint => Self::Constraint,
            BeliefTypeInput::Goal => Self::Goal,
            BeliefTypeInput::Identity => Self::Identity,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimInput {
    #[default]
    Affirmative,
    NegativeExistence,
}

impl From<ClaimInput> for AssertionClaim {
    fn from(value: ClaimInput) -> Self {
        match value {
            ClaimInput::Affirmative => Self::Affirmative,
            ClaimInput::NegativeExistence => Self::NegativeExistence,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ProvenanceInput {
    pub first_lsn: u64,
    pub last_lsn: u64,
    #[serde(default)]
    pub byte_start: u32,
    pub byte_end: u32,
}

impl From<&ProvenanceInput> for ProvenanceRange {
    fn from(value: &ProvenanceInput) -> Self {
        Self {
            first_lsn: value.first_lsn,
            last_lsn: value.last_lsn,
            byte_start: value.byte_start,
            byte_end: value.byte_end,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BeliefClaimInput {
    pub belief_id: String,
    pub belief_type: BeliefTypeInput,
    pub canonical_identity: String,
    pub value: String,
    #[serde(default)]
    pub valid_from_ns: i64,
    #[serde(default)]
    pub valid_to_ns: i64,
    pub provenance: Vec<ProvenanceInput>,
    #[serde(default)]
    pub conflict_domain: Option<String>,
    #[serde(default)]
    pub claim: ClaimInput,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BelieveInput {
    pub conversation: String,
    #[serde(flatten)]
    pub belief: BeliefClaimInput,
    #[serde(default)]
    pub run_id: Option<String>,
}

pub async fn run(actor: &ActorEngine, input: BelieveInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty()
        || input.run_id.as_ref().is_some_and(String::is_empty)
        || input.belief.belief_id.is_empty()
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let assertion = assertion(&input.belief);
    if assertion.claim == AssertionClaim::NegativeExistence {
        require_tool_observation(actor, &assertion.provenance).await?;
    }
    let protected = matches!(
        assertion.belief_type,
        BeliefType::Identity | BeliefType::Constraint | BeliefType::Preference
    );
    let run_id = input.run_id.map(String::into_bytes);
    let (kind, payload, authority) = if run_id.is_some() && protected {
        (
            EventKind::ProposedAssertion,
            EventPayload::ProposedAssertion(Box::new(proposal(&assertion))),
            Authority::DerivedInference,
        )
    } else {
        (
            EventKind::Assertion,
            EventPayload::Assertion(Box::new(assertion.clone())),
            if run_id.is_some() {
                Authority::DerivedInference
            } else {
                Authority::UserAsserted
            },
        )
    };
    let outcome = append(actor, &input.conversation, kind, payload, authority, run_id).await?;
    if protected && authority != Authority::UserAsserted {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite).at_lsn(outcome.first_lsn));
    }
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "belief_id": input.belief.belief_id,
        "belief_type": belief_type_name(assertion.belief_type),
        "canonical_identity": assertion.canonical_identity,
        "valid_from_ns": assertion.valid_from_ns,
        "valid_to_ns": assertion.valid_to_ns,
        "lsn": outcome.first_lsn.get(),
    }));
    envelope.provenance.push(format!(
        "hm://{}/lsn/{}",
        actor.actor(),
        outcome.first_lsn.get()
    ));
    Ok(envelope)
}

pub(crate) fn assertion(input: &BeliefClaimInput) -> Assertion {
    Assertion {
        belief_id: input.belief_id.as_bytes().to_vec(),
        belief_type: input.belief_type.into(),
        canonical_identity: input.canonical_identity.clone(),
        value: input.value.as_bytes().to_vec(),
        valid_from_ns: input.valid_from_ns,
        valid_to_ns: input.valid_to_ns,
        provenance: input.provenance.iter().map(Into::into).collect(),
        conflict_domain: input.conflict_domain.clone(),
        claim: input.claim.into(),
    }
}

pub(crate) async fn append(
    actor: &ActorEngine,
    conversation: &str,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
    run_id: Option<Vec<u8>>,
) -> Result<hm_serve::actor::AppendOutcome, Error> {
    actor
        .append(vec![IncomingEvent {
            kind,
            conversation: ConversationId::derive(conversation),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: actor.actor().get(),
                run_id,
                model_provenance: None,
                authority,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await
}

fn proposal(assertion: &Assertion) -> ProposedAssertion {
    ProposedAssertion {
        belief_id: assertion.belief_id.clone(),
        belief_type: assertion.belief_type,
        canonical_identity: assertion.canonical_identity.clone(),
        value: assertion.value.clone(),
        valid_from_ns: assertion.valid_from_ns,
        valid_to_ns: assertion.valid_to_ns,
        provenance: assertion.provenance.clone(),
        conflict_domain: assertion.conflict_domain.clone(),
        claim: assertion.claim,
    }
}

async fn require_tool_observation(
    actor: &ActorEngine,
    provenance: &[ProvenanceRange],
) -> Result<(), Error> {
    for range in provenance {
        let maximum = range
            .last_lsn
            .checked_sub(range.first_lsn)
            .and_then(|span| span.checked_add(1))
            .and_then(|count| usize::try_from(count).ok())
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        for frame in actor
            .frames_since(LSN::new(range.first_lsn.saturating_sub(1)), None, maximum)
            .await?
        {
            if frame.header.lsn.get() > range.last_lsn || frame.header.kind != EventKind::ToolResult
            {
                continue;
            }
            let verified = verify_event(
                &frame.sealed_payload,
                hm_schema::event::EventKind::ToolResult,
                Boundary::Disk,
            )?;
            if verified.envelope.authority == Authority::ToolObserved
                && matches!(
                    verified.envelope.payload,
                    EventPayload::ToolResult(ref result) if result.status == ResultStatus::Ok
                )
            {
                return Ok(());
            }
        }
    }
    Err(Error::new(ErrorCode::NegativeExistenceUncorroborated))
}

pub(crate) const fn belief_type_name(value: BeliefType) -> &'static str {
    match value {
        BeliefType::Fact => "fact",
        BeliefType::Preference => "preference",
        BeliefType::Constraint => "constraint",
        BeliefType::Goal => "goal",
        BeliefType::Identity => "identity",
    }
}
