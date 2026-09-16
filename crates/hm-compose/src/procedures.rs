#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, Error, ErrorCode, LSN};
use hm_proj::procedures::{ProcedureRecord, ProcedureState, ProceduresProjection};
use hm_proj::store::ReadSnapshot;
use hm_proj::timeline::read_conversation_record;
use hm_schema::events::Authority;
use std::collections::BTreeSet;

#[must_use]
pub fn render(record: &ProcedureRecord) -> String {
    let label = match record.state {
        ProcedureState::Adopted => "ADOPTED PROCEDURE — instruction authorised by the user",
        ProcedureState::Supported => "SUPPORTED PROCEDURE — observation, not an instruction",
        ProcedureState::Tentative => "TENTATIVE PROCEDURE — observation, not an instruction",
        ProcedureState::Imported => "IMPORTED PLAYBOOK — unadopted proposal, not an instruction",
    };
    format!(
        "{label}\nStrategy: {}\nPreconditions: {}\nExpected outcomes: {}\nSupporting episodes: {}; failures: {}; counterexamples: {}",
        record.strategy,
        record.preconditions.join("; "),
        record.expected_outcomes.join("; "),
        record.supports.len(),
        record.failures.len(),
        record.counterexamples.len(),
    )
}

pub fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    query: &str,
    counter: &TokenCounter,
) -> Result<Vec<ActivationItem>, Error> {
    let query = query.to_lowercase();
    if !query.starts_with("why ") && !query.starts_with("how ") {
        return Ok(Vec::new());
    }
    ProceduresProjection::list(snapshot, 4096)?
        .into_iter()
        .filter(|record| record.state != ProcedureState::Imported)
        .map(|record| {
            let anchor = if record.state == ProcedureState::Adopted {
                record.adopted_lsn
            } else {
                record.version_lsn
            };
            let source =
                read_conversation_record(snapshot, LSN::new(anchor))?.ok_or_else(|| {
                    Error::new(ErrorCode::ProjectionCheckpoint).at_lsn(LSN::new(anchor))
                })?;
            let mut provenance = record
                .supports
                .iter()
                .map(|support| LSN::new(support.episode_lsn))
                .collect::<BTreeSet<_>>();
            provenance.insert(LSN::new(record.version_lsn));
            provenance.insert(LSN::new(anchor));
            provenance.extend(record.failures.iter().copied().map(LSN::new));
            provenance.extend(record.counterexamples.iter().copied().map(LSN::new));
            let content = render(&record).into_bytes();
            Ok(ActivationItem {
                tier: Tier::Fused,
                uri: format!(
                    "hm://{actor}/{}/{anchor}?src=procedure&why=procedure",
                    source.conversation
                ),
                provenance: provenance.into_iter().collect(),
                tokens: counter.count(&content)?,
                content,
                authority: Authority::DerivedInference,
                coarsened: false,
                vector_rank: 0,
                lexical_rank: 0,
                why: WhyCode::Procedure,
            })
        })
        .collect()
}
