#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use hm_compose::bundle::RetrievalLane;
use hm_compose::fusion::{FusedHit, LaneRanking, Q16_ONE, RankedCandidate, fuse};
use hm_compose::geometry::{
    AlignmentBoost, BASIS_DIRECTIONS, GEOMETRY_VERSION, GeometryConfig, GeometryEvidence,
};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN};
use hm_embed::{Embedder, HashFeatureEmbedder, quantize};
use hm_index::geometry::{Q16_ONE as FACTOR_ONE, alignment_factor_q16, coordinates_q16};
use hm_index::simd::scalar_dot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEPLOYMENT_ID: &str = "hm-eval/geometry-alignment-ablation";
const DIMENSIONS: usize = 256;
const CUTOFF: usize = 10;

#[derive(Deserialize)]
struct Fixture {
    documents: Vec<String>,
    queries: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    query: String,
    relevant_index: usize,
}

#[derive(Debug, Serialize)]
pub struct GeometryResult {
    pub fixture_hash: String,
    pub queries: u64,
    pub baseline_mrr_at_10: f64,
    pub boosted_mrr_at_10: f64,
    pub enabled_by_default: bool,
}

struct Measurement {
    query: Vec<i8>,
    relevant: Vec<u8>,
    hits: Vec<FusedHit>,
}

pub fn run() -> Result<GeometryResult, Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../../../../eval/fixtures/geometry/alignment.json");
    let fixture: Fixture = serde_json::from_slice(bytes)?;
    if fixture.documents.is_empty() || fixture.queries.is_empty() {
        return Err(Box::new(Error::new(ErrorCode::SchemaInvalid)));
    }
    let embedder = HashFeatureEmbedder::new(DIMENSIONS)?;
    let documents = fixture
        .documents
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut vectors = Vec::with_capacity(documents.len());
    for embedding in embedder.embed_documents(&documents)? {
        vectors.push(quantize(&embedding)?.values);
    }
    let corpus = vectors
        .iter()
        .enumerate()
        .map(|(index, vector)| (canonical_id(index), vector.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut measurements = Vec::with_capacity(fixture.queries.len());
    let mut baseline = 0.0;
    let mut measured = 0.0;
    for case in &fixture.queries {
        if case.relevant_index >= vectors.len() {
            return Err(Box::new(Error::new(ErrorCode::SchemaInvalid)));
        }
        let query = quantize(&embedder.embed_query(&case.query)?)?.values;
        let hits = rank(&query, &vectors)?;
        let relevant = canonical_id(case.relevant_index);
        baseline += reciprocal_rank(hits.iter().map(|hit| &hit.canonical_id), &relevant);
        measured += reciprocal_rank(alignment_order(&query, &corpus, &hits).iter(), &relevant);
        measurements.push(Measurement {
            query,
            relevant,
            hits,
        });
    }
    let queries = measurements.len() as f64;
    let baseline_mrr_at_10 = baseline / queries;
    let mut boosted_mrr_at_10 = measured / queries;

    let evidence = GeometryEvidence {
        producer: "hm-eval".to_owned(),
        deployment_id: DEPLOYMENT_ID.to_owned(),
        version: GEOMETRY_VERSION,
        evaluation_id: format!(
            "geometry-alignment-ablation/{}",
            blake3::hash(bytes).to_hex()
        ),
        evaluated_queries: measurements.len() as u64,
        baseline_mrr_at_10,
        boosted_mrr_at_10,
    };
    if evidence.justifies(DEPLOYMENT_ID, GEOMETRY_VERSION) {
        let boost = AlignmentBoost::open(&GeometryConfig {
            enabled: true,
            deployment_id: DEPLOYMENT_ID.to_owned(),
            version: GEOMETRY_VERSION,
            evidence: Some(evidence),
        })
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
        let mut boosted = 0.0;
        for measurement in &measurements {
            let mut hits = measurement.hits.clone();
            boost
                .apply(&measurement.query, &corpus, &mut hits)
                .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
            boosted += reciprocal_rank(
                hits.iter().map(|hit| &hit.canonical_id),
                &measurement.relevant,
            );
        }
        boosted_mrr_at_10 = boosted / queries;
    }

    Ok(GeometryResult {
        fixture_hash: blake3::hash(bytes).to_hex().to_string(),
        queries: measurements.len() as u64,
        baseline_mrr_at_10,
        boosted_mrr_at_10,
        enabled_by_default: AlignmentBoost::open(&GeometryConfig::default())
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .enabled(),
    })
}

fn canonical_id(index: usize) -> Vec<u8> {
    format!("document-{index:04}").into_bytes()
}

fn rank(query: &[i8], vectors: &[Vec<i8>]) -> Result<Vec<FusedHit>, Error> {
    let mut order = (0..vectors.len()).collect::<Vec<_>>();
    let scores = vectors
        .iter()
        .map(|vector| scalar_dot(query, vector).unwrap_or(0))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| scores[*right].cmp(&scores[*left]).then(left.cmp(right)));
    let candidates = order
        .iter()
        .map(|index| {
            RankedCandidate::neutral(
                canonical_id(*index),
                LSN::new(u64::try_from(*index).unwrap_or(u64::MAX).saturating_add(1)),
            )
        })
        .collect::<Vec<_>>();
    fuse(
        ActorId::new(7),
        ConversationId::derive("hypermind.geometry-ablation"),
        &[LaneRanking {
            lane: RetrievalLane::Vector,
            weight_q16: Q16_ONE,
            candidates,
        }],
        vectors.len(),
    )
}

fn alignment_order(
    query: &[i8],
    corpus: &BTreeMap<Vec<u8>, Vec<i8>>,
    hits: &[FusedHit],
) -> Vec<Vec<u8>> {
    let basis = hits
        .iter()
        .filter_map(|hit| corpus.get(&hit.canonical_id).map(Vec::as_slice))
        .take(BASIS_DIRECTIONS)
        .collect::<Vec<_>>();
    let query_coordinates = coordinates_q16(query, &basis);
    let mut ranked = hits
        .iter()
        .map(|hit| {
            let score = corpus
                .get(&hit.canonical_id)
                .map_or(hit.score_q32, |vector| {
                    let factor =
                        alignment_factor_q16(&coordinates_q16(vector, &basis), &query_coordinates);
                    let scaled = u128::from(hit.score_q32) * u128::from(factor.unsigned_abs())
                        / u128::from(FACTOR_ONE.unsigned_abs());
                    u64::try_from(scaled).unwrap_or(u64::MAX)
                });
            (score, hit.canonical_id.clone())
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    ranked.into_iter().map(|(_, id)| id).collect()
}

fn reciprocal_rank<'ids>(order: impl Iterator<Item = &'ids Vec<u8>>, relevant: &[u8]) -> f64 {
    order
        .take(CUTOFF)
        .position(|id| id.as_slice() == relevant)
        .map_or(0.0, |index| 1.0 / (index + 1) as f64)
}
