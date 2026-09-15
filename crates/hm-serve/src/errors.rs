use hm_core::{Error, ErrorCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MutationEffectState {
    NotDispatched = 1,
    Unknown = 2,
    Rejected = 3,
}

#[must_use]
pub const fn mutation_effect_state(error: Error) -> MutationEffectState {
    match error.code {
        ErrorCode::InvalidArgument
        | ErrorCode::InvalidLength
        | ErrorCode::InvalidKind
        | ErrorCode::SchemaInvalid
        | ErrorCode::SchemaVersion
        | ErrorCode::ForbiddenKind
        | ErrorCode::ProtocolInvalid
        | ErrorCode::ProtocolVersion
        | ErrorCode::CapabilityDenied => MutationEffectState::NotDispatched,
        ErrorCode::AlreadyExists
        | ErrorCode::SequenceViolation
        | ErrorCode::OrderingViolation
        | ErrorCode::BeliefWriteGate
        | ErrorCode::BeliefIdConflict
        | ErrorCode::BeliefNotFound
        | ErrorCode::NegativeExistenceUncorroborated
        | ErrorCode::LoopNotFound
        | ErrorCode::WorkItemNotFound
        | ErrorCode::IdempotencyConflict
        | ErrorCode::ProtectedTypeWrite
        | ErrorCode::CitationInvalid
        | ErrorCode::Tripwire => MutationEffectState::Rejected,
        _ => MutationEffectState::Unknown,
    }
}

#[must_use]
pub const fn mutation_effect_state_name(state: MutationEffectState) -> &'static str {
    match state {
        MutationEffectState::NotDispatched => "not_dispatched",
        MutationEffectState::Unknown => "unknown",
        MutationEffectState::Rejected => "rejected",
    }
}
