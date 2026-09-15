#![allow(clippy::missing_errors_doc)]

use crate::bundle::RetrievalLane;
use hm_core::{Error, ErrorCode};
use hm_index::entity_rules::{EntityKind, extract_query_entities};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecallMode {
    Semantic,
    Lexical,
    Entity,
    Temporal,
    Graph,
    AsOf,
    Timeline,
    Reconstruct,
    Near,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryShape {
    Identifier,
    Temporal,
    Explanatory,
    Semantic,
    Anchored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorFacet {
    Path,
    Symbol,
    Url,
    Entity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Anchor {
    pub facet: AnchorFacet,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanePlan {
    pub lane: RetrievalLane,
    pub weight_q16: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPlan {
    pub shape: QueryShape,
    pub lanes: Vec<LanePlan>,
}

pub fn plan(
    query: &str,
    turn_text: &str,
    mode: RecallMode,
    anchor: Option<&Anchor>,
) -> Result<QueryPlan, Error> {
    if query.len() > 1024 * 1024 || turn_text.len() > 1024 * 1024 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    if let Some(anchor) = anchor {
        validate_anchor(anchor)?;
    }
    let lower = query.to_lowercase();
    let shape = if mode == RecallMode::Near || anchor.is_some() {
        QueryShape::Anchored
    } else if extract_query_entities(query, turn_text)
        .iter()
        .any(|entity| entity.kind != EntityKind::ProperNoun || entity.canonical.contains(' '))
    {
        QueryShape::Identifier
    } else if contains_temporal_phrase(&lower) {
        QueryShape::Temporal
    } else if lower.starts_with("why ") || lower.starts_with("how ") {
        QueryShape::Explanatory
    } else {
        QueryShape::Semantic
    };
    let lanes = match mode {
        RecallMode::Semantic => vec![
            weighted(RetrievalLane::Vector, 5),
            weighted(RetrievalLane::Lexical, 3),
        ],
        RecallMode::Lexical => vec![weighted(RetrievalLane::Lexical, 1)],
        RecallMode::Entity => vec![weighted(RetrievalLane::Entity, 1)],
        RecallMode::Temporal => vec![weighted(RetrievalLane::Temporal, 1)],
        RecallMode::Graph => vec![weighted(RetrievalLane::Graph, 1)],
        RecallMode::AsOf => vec![weighted(RetrievalLane::Belief, 1)],
        RecallMode::Timeline => vec![weighted(RetrievalLane::Timeline, 1)],
        RecallMode::Reconstruct => vec![weighted(RetrievalLane::Reconstruct, 1)],
        RecallMode::Near => vec![
            weighted(RetrievalLane::Entity, 6),
            weighted(RetrievalLane::Vector, 4),
            weighted(RetrievalLane::Temporal, 2),
        ],
    };
    let lanes = if mode == RecallMode::Semantic {
        match shape {
            QueryShape::Identifier => vec![
                weighted(RetrievalLane::Entity, 6),
                weighted(RetrievalLane::Lexical, 4),
                weighted(RetrievalLane::Vector, 3),
            ],
            QueryShape::Temporal => vec![
                weighted(RetrievalLane::Temporal, 6),
                weighted(RetrievalLane::Vector, 4),
                weighted(RetrievalLane::Lexical, 3),
            ],
            QueryShape::Explanatory => vec![
                weighted(RetrievalLane::Vector, 5),
                weighted(RetrievalLane::Lexical, 4),
                weighted(RetrievalLane::Graph, 2),
            ],
            QueryShape::Semantic => lanes,
            QueryShape::Anchored => vec![
                weighted(RetrievalLane::Entity, 6),
                weighted(RetrievalLane::Vector, 4),
                weighted(RetrievalLane::Temporal, 2),
            ],
        }
    } else {
        lanes
    };
    Ok(QueryPlan { shape, lanes })
}

pub fn validate_anchor(anchor: &Anchor) -> Result<(), Error> {
    if anchor.value.is_empty() || anchor.value.len() > 4096 {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(())
    }
}

const fn weighted(lane: RetrievalLane, weight: u32) -> LanePlan {
    LanePlan {
        lane,
        weight_q16: weight << 16,
    }
}

fn contains_temporal_phrase(query: &str) -> bool {
    query
        .split_whitespace()
        .any(|word| matches!(word, "yesterday" | "today" | "tomorrow" | "when" | "since"))
        || ["last week", "last month", "before ", "after "]
            .iter()
            .any(|phrase| query.contains(phrase))
        || query.starts_with("when ")
        || query.starts_with("before ")
        || query.starts_with("after ")
}
