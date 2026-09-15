use crate::bundle::{BundleHealth, HealthStatus};
use hm_core::LSN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncoderState {
    Unconfigured,
    Loading,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthInput {
    pub encoder: EncoderState,
    pub embedding_backlog: usize,
    pub projection_lsn: LSN,
    pub ledger_lsn: LSN,
    pub semantic_included: bool,
    pub lexical_included: bool,
}

#[must_use]
pub const fn evaluate(input: HealthInput) -> BundleHealth {
    BundleHealth {
        encoder: match input.encoder {
            EncoderState::Unconfigured => HealthStatus::Unavailable,
            EncoderState::Loading => HealthStatus::SemanticLagging,
            EncoderState::Ready => HealthStatus::SemanticReady,
        },
        backlog: if input.embedding_backlog == 0 {
            HealthStatus::SemanticReady
        } else {
            HealthStatus::SemanticLagging
        },
        projection: if input.projection_lsn.get() >= input.ledger_lsn.get() {
            HealthStatus::SemanticReady
        } else {
            HealthStatus::SemanticLagging
        },
        inclusion: if input.semantic_included {
            HealthStatus::SemanticReady
        } else if input.lexical_included {
            HealthStatus::LexicalOnly
        } else {
            HealthStatus::Unavailable
        },
    }
}
