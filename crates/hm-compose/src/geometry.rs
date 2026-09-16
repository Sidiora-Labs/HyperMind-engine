#![allow(clippy::missing_errors_doc)]

use crate::fusion::FusedHit;
use hm_index::geometry::{Q16_ONE, alignment_factor_q16, coordinates_q16};
use std::collections::BTreeMap;

pub const GEOMETRY_VERSION: u16 = 1;
pub const BASIS_DIRECTIONS: usize = 8;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeometryConfig {
    pub enabled: bool,
    pub deployment_id: String,
    pub version: u16,
    pub evidence: Option<GeometryEvidence>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryEvidence {
    pub producer: String,
    pub deployment_id: String,
    pub version: u16,
    pub evaluation_id: String,
    pub evaluated_queries: u64,
    pub baseline_mrr_at_10: f64,
    pub boosted_mrr_at_10: f64,
}

impl GeometryEvidence {
    #[must_use]
    pub fn justifies(&self, deployment_id: &str, version: u16) -> bool {
        self.producer == "hm-eval"
            && !deployment_id.is_empty()
            && self.deployment_id == deployment_id
            && version == GEOMETRY_VERSION
            && self.version == version
            && !self.evaluation_id.is_empty()
            && self.evaluated_queries > 0
            && (0.0..=1.0).contains(&self.baseline_mrr_at_10)
            && (0.0..=1.0).contains(&self.boosted_mrr_at_10)
            && self.boosted_mrr_at_10 > self.baseline_mrr_at_10
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeometryError {
    UnqualifiedDeployment,
    InvalidArgument,
    DimensionMismatch,
    CapacityExceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlignmentReport {
    pub applied: usize,
    pub neutral: usize,
    pub minimum_factor_q16: i64,
    pub maximum_factor_q16: i64,
}

pub struct AlignmentBoost {
    enabled: bool,
}

impl AlignmentBoost {
    pub fn open(config: &GeometryConfig) -> Result<Self, GeometryError> {
        if !config.enabled {
            return Ok(Self { enabled: false });
        }
        if !config
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.justifies(&config.deployment_id, config.version))
        {
            return Err(GeometryError::UnqualifiedDeployment);
        }
        Ok(Self { enabled: true })
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn apply(
        &self,
        query: &[i8],
        vectors: &BTreeMap<Vec<u8>, Vec<i8>>,
        hits: &mut Vec<FusedHit>,
    ) -> Result<AlignmentReport, GeometryError> {
        if !self.enabled {
            return Ok(AlignmentReport {
                applied: 0,
                neutral: hits.len(),
                minimum_factor_q16: Q16_ONE,
                maximum_factor_q16: Q16_ONE,
            });
        }
        if query.is_empty() {
            return Err(GeometryError::InvalidArgument);
        }
        if vectors.values().any(|vector| vector.len() != query.len()) {
            return Err(GeometryError::DimensionMismatch);
        }
        let basis = hits
            .iter()
            .filter_map(|hit| vectors.get(&hit.canonical_id).map(Vec::as_slice))
            .take(BASIS_DIRECTIONS)
            .collect::<Vec<_>>();
        let query_coordinates = coordinates_q16(query, &basis);
        let mut factors = Vec::new();
        let mut neutral = 0;
        let mut boosted = Vec::with_capacity(hits.len());
        for hit in hits.iter() {
            let Some(vector) = vectors.get(&hit.canonical_id) else {
                neutral += 1;
                boosted.push(hit.clone());
                continue;
            };
            let coordinates = coordinates_q16(vector, &basis);
            let factor = alignment_factor_q16(&coordinates, &query_coordinates);
            let score_q32 = rescale(hit.score_q32, factor)?;
            factors.push(factor);
            boosted.push(FusedHit {
                score_q32,
                uri: format!("{}&boost={factor}", hit.uri),
                ..hit.clone()
            });
        }
        boosted.sort_by(|left, right| {
            right
                .score_q32
                .cmp(&left.score_q32)
                .then_with(|| left.canonical_id.cmp(&right.canonical_id))
        });
        let report = AlignmentReport {
            applied: factors.len(),
            neutral,
            minimum_factor_q16: factors.iter().copied().min().unwrap_or(Q16_ONE),
            maximum_factor_q16: factors.iter().copied().max().unwrap_or(Q16_ONE),
        };
        *hits = boosted;
        Ok(report)
    }
}

fn rescale(score: u64, factor: i64) -> Result<u64, GeometryError> {
    let scaled = u128::from(score)
        .checked_mul(u128::from(factor.unsigned_abs()))
        .ok_or(GeometryError::CapacityExceeded)?
        / u128::from(Q16_ONE.unsigned_abs());
    u64::try_from(scaled).map_err(|_| GeometryError::CapacityExceeded)
}
