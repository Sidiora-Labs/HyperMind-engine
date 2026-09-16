use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::procedures::{ImprovementDraft, ProcedureHead};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, IntentSet, LoopCloseReason, LoopClosed, LoopOpened,
    Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntendCloseReason {
    Done,
    Abandoned,
    HandedOff,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntendAction {
    SetIntention {
        intention_id: String,
        objective: String,
        trigger: WakeTriggerInput,
        expires_at_ns: i64,
        reply_route: String,
    },
    CancelIntention {
        intention_id: String,
        reason: String,
    },
    EvaluateWake {
        observation_lsn: u64,
        factors: AttentionFactorsInput,
        #[serde(default)]
        rearm_at_ns: Option<i64>,
    },
    AdoptProcedure {
        procedure_id: String,
        procedure_lsn: u64,
    },
    ImportPlaybook {
        source_uri: String,
        document: String,
    },
    ProposeProcedureImprovement {
        proposal_id: String,
        procedure_id: String,
        strategy: String,
        expected_outcomes: Vec<String>,
        preconditions: Vec<String>,
        rationale: String,
        failure_lsns: Vec<u64>,
    },
    SetObjective {
        objective: String,
    },
    OpenLoop {
        loop_id: String,
        objective: String,
    },
    CloseLoop {
        loop_id: String,
        reason: IntendCloseReason,
        #[serde(default)]
        cause: String,
        #[serde(default)]
        evidence_lsns: Vec<u64>,
    },
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WakeTriggerInput {
    At { at_ns: i64 },
    Schedule { schedule: String },
    ChildTerminal { child_id: String },
    ProcessExit { process_id: String },
    FileChanged { path: String },
    RepositoryChanged { repository: String },
    ChannelMessage { channel: String },
    ExternalCondition { condition: String },
    UserResponse { reply_to: String },
    EntityMentioned { entity_id: String },
    LoopClosed { loop_id: String },
    PredictionResolved { prediction_id: String },
    BeliefChanged { canonical_identity: String },
}

impl From<WakeTriggerInput> for hm_schema::events::WakeTrigger {
    fn from(value: WakeTriggerInput) -> Self {
        use hm_schema::events::{
            WakeAt, WakeBeliefChanged, WakeChannelMessage, WakeChildTerminal, WakeEntityMentioned,
            WakeExternalCondition, WakeFileChanged, WakeLoopClosed, WakePredictionResolved,
            WakeProcessExit, WakeRepositoryChanged, WakeSchedule, WakeUserResponse,
        };
        match value {
            WakeTriggerInput::At { at_ns } => Self::WakeAt(Box::new(WakeAt { at_ns })),
            WakeTriggerInput::Schedule { schedule } => {
                Self::WakeSchedule(Box::new(WakeSchedule { schedule }))
            }
            WakeTriggerInput::ChildTerminal { child_id } => {
                Self::WakeChildTerminal(Box::new(WakeChildTerminal {
                    child_id: child_id.into_bytes(),
                }))
            }
            WakeTriggerInput::ProcessExit { process_id } => {
                Self::WakeProcessExit(Box::new(WakeProcessExit {
                    process_id: process_id.into_bytes(),
                }))
            }
            WakeTriggerInput::FileChanged { path } => {
                Self::WakeFileChanged(Box::new(WakeFileChanged { path }))
            }
            WakeTriggerInput::RepositoryChanged { repository } => {
                Self::WakeRepositoryChanged(Box::new(WakeRepositoryChanged { repository }))
            }
            WakeTriggerInput::ChannelMessage { channel } => {
                Self::WakeChannelMessage(Box::new(WakeChannelMessage { channel }))
            }
            WakeTriggerInput::ExternalCondition { condition } => {
                Self::WakeExternalCondition(Box::new(WakeExternalCondition { condition }))
            }
            WakeTriggerInput::UserResponse { reply_to } => {
                Self::WakeUserResponse(Box::new(WakeUserResponse {
                    reply_to: reply_to.into_bytes(),
                }))
            }
            WakeTriggerInput::EntityMentioned { entity_id } => {
                Self::WakeEntityMentioned(Box::new(WakeEntityMentioned {
                    entity_id: entity_id.into_bytes(),
                }))
            }
            WakeTriggerInput::LoopClosed { loop_id } => {
                Self::WakeLoopClosed(Box::new(WakeLoopClosed {
                    loop_id: loop_id.into_bytes(),
                }))
            }
            WakeTriggerInput::PredictionResolved { prediction_id } => {
                Self::WakePredictionResolved(Box::new(WakePredictionResolved {
                    prediction_id: prediction_id.into_bytes(),
                }))
            }
            WakeTriggerInput::BeliefChanged { canonical_identity } => {
                Self::WakeBeliefChanged(Box::new(WakeBeliefChanged { canonical_identity }))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
pub struct AttentionFactorsInput {
    pub urgency: u32,
    pub expected_value: u32,
    pub confidence: u32,
    pub interruption_cost: u32,
    pub resource_cost: u32,
    pub duplication_penalty: u32,
    pub quiet_hours: bool,
    pub notifications_remaining: u32,
    pub workload: u32,
}

impl From<AttentionFactorsInput> for hm_cortex::attention::AttentionFactors {
    fn from(value: AttentionFactorsInput) -> Self {
        Self {
            urgency: value.urgency,
            expected_value: value.expected_value,
            confidence: value.confidence,
            interruption_cost: value.interruption_cost,
            resource_cost: value.resource_cost,
            duplication_penalty: value.duplication_penalty,
            quiet_hours: value.quiet_hours,
            notifications_remaining: value.notifications_remaining,
            workload: value.workload,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct IntendInput {
    pub conversation: String,
    pub action: IntendAction,
}

#[allow(clippy::too_many_lines)]
pub async fn run(actor: &ActorEngine, input: IntendInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    if let IntendAction::EvaluateWake {
        observation_lsn,
        factors,
        rearm_at_ns,
    } = &input.action
    {
        let report = actor
            .evaluate_wake(LSN::new(*observation_lsn), (*factors).into(), *rearm_at_ns)
            .await?;
        let mut envelope = Envelope::empty();
        envelope
            .items
            .push(serde_json::to_value(&report).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?);
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{observation_lsn}", actor.actor()));
        for item in &report.fired {
            envelope
                .provenance
                .push(format!("hm://{}/lsn/{}", actor.actor(), item.fired_lsn));
            envelope
                .provenance
                .push(format!("hm://{}/lsn/{}", actor.actor(), item.decision_lsn));
        }
        return Ok(envelope);
    }
    let (kind, payload, action, loop_id, proposal_id, authority) = match input.action {
        IntendAction::SetIntention {
            intention_id,
            objective,
            trigger,
            expires_at_ns,
            reply_route,
        } => (
            EventKind::IntentionSet,
            EventPayload::IntentionSet(Box::new(hm_schema::events::IntentionSet {
                intention_id: intention_id.clone().into_bytes(),
                objective: objective.into_bytes(),
                trigger: Some(trigger.into()),
                expires_at_ns,
                reply_route,
            })),
            "set_intention",
            Some(intention_id),
            None,
            Authority::UserAsserted,
        ),
        IntendAction::CancelIntention {
            intention_id,
            reason,
        } => (
            EventKind::IntentionCancelled,
            EventPayload::IntentionCancelled(Box::new(hm_schema::events::IntentionCancelled {
                intention_id: intention_id.clone().into_bytes(),
                reason,
            })),
            "cancel_intention",
            Some(intention_id),
            None,
            Authority::UserAsserted,
        ),
        IntendAction::AdoptProcedure {
            procedure_id,
            procedure_lsn,
        } => (
            EventKind::ProcedureAdopted,
            EventPayload::ProcedureAdopted(Box::new(hm_schema::events::ProcedureAdopted {
                procedure_id: identifier(&procedure_id),
                procedure_lsn,
            })),
            "adopt_procedure",
            Some(procedure_id),
            None,
            Authority::UserAsserted,
        ),
        IntendAction::EvaluateWake { .. } => unreachable!("handled above"),
        IntendAction::ImportPlaybook {
            source_uri,
            document,
        } => {
            let imported = hm_cortex::playbook::parse(&source_uri, document.as_bytes())?;
            let procedure_id = hex(&imported.procedure_id);
            (
                EventKind::ProcedureImported,
                EventPayload::ProcedureImported(Box::new(imported)),
                "import_playbook",
                Some(procedure_id),
                None,
                Authority::ExternalObserved,
            )
        }
        IntendAction::ProposeProcedureImprovement {
            proposal_id,
            procedure_id,
            strategy,
            expected_outcomes,
            preconditions,
            rationale,
            failure_lsns,
        } => {
            if proposal_id.is_empty() || procedure_id.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let record = actor
                .procedure(identifier(&procedure_id))
                .await?
                .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
            let head = ProcedureHead {
                procedure_id: record.procedure_id,
                version_lsn: record.version_lsn,
                strategy: record.strategy,
                expected_outcomes: record.expected_outcomes,
                preconditions: record.preconditions,
            };
            let proposed = hm_cortex::procedures::propose_improvement(
                &identifier(&proposal_id),
                &head,
                ImprovementDraft {
                    strategy,
                    expected_outcomes,
                    preconditions,
                    rationale,
                },
                &failure_lsns,
            )?;
            (
                EventKind::ProcedureImprovementProposed,
                EventPayload::ProcedureImprovementProposed(Box::new(proposed)),
                "propose_procedure_improvement",
                Some(procedure_id),
                Some(proposal_id),
                Authority::DerivedInference,
            )
        }
        IntendAction::SetObjective { objective } => {
            if objective.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            (
                EventKind::IntentSet,
                EventPayload::IntentSet(Box::new(IntentSet {
                    objective: objective.into_bytes(),
                })),
                "set_objective",
                None,
                None,
                Authority::UserAsserted,
            )
        }
        IntendAction::OpenLoop { loop_id, objective } => {
            if loop_id.is_empty() || objective.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let item_loop_id = loop_id.clone();
            (
                EventKind::LoopOpened,
                EventPayload::LoopOpened(Box::new(LoopOpened {
                    loop_id: loop_id.into_bytes(),
                    objective: objective.into_bytes(),
                })),
                "open_loop",
                Some(item_loop_id),
                None,
                Authority::UserAsserted,
            )
        }
        IntendAction::CloseLoop {
            loop_id,
            reason,
            cause,
            evidence_lsns,
        } => {
            if loop_id.is_empty()
                || matches!(reason, IntendCloseReason::Done) && evidence_lsns.is_empty()
            {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let item_loop_id = loop_id.clone();
            let reason = match reason {
                IntendCloseReason::Done => LoopCloseReason::Done,
                IntendCloseReason::Abandoned => LoopCloseReason::Abandoned,
                IntendCloseReason::HandedOff => LoopCloseReason::HandedOff,
                IntendCloseReason::Superseded => LoopCloseReason::Superseded,
            };
            (
                EventKind::LoopClosed,
                EventPayload::LoopClosed(Box::new(LoopClosed {
                    loop_id: loop_id.into_bytes(),
                    reason,
                    cause: cause.into_bytes(),
                    evidence_lsns: (!evidence_lsns.is_empty()).then_some(evidence_lsns),
                })),
                "close_loop",
                Some(item_loop_id),
                None,
                Authority::UserAsserted,
            )
        }
    };
    let conversation = ConversationId::derive(&input.conversation);
    let outcome = actor
        .append(vec![IncomingEvent {
            kind,
            conversation,
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?;
    let uri = format!("hm://{}/lsn/{}", actor.actor(), outcome.first_lsn.get());
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "action": action,
        "loop_id": if matches!(action, "open_loop" | "close_loop") { loop_id.as_deref() } else { None },
        "intention_id": if matches!(action, "set_intention" | "cancel_intention") { loop_id.as_deref() } else { None },
        "procedure_id": if matches!(
            action,
            "adopt_procedure" | "import_playbook" | "propose_procedure_improvement"
        ) { loop_id.as_deref() } else { None },
        "proposal_id": proposal_id.as_deref(),
        "lsn": outcome.first_lsn.get(),
    }));
    envelope.provenance.push(uri);
    Ok(envelope)
}

const PROCEDURE_DIGEST_HEX: usize = 64;

fn identifier(value: &str) -> Vec<u8> {
    if value.len() != PROCEDURE_DIGEST_HEX || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return value.as_bytes().to_vec();
    }
    (0..value.len() / 2)
        .filter_map(|index| u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}
