#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::slice1::Metric;
use hm_embed::{
    Embedder, HashFeatureEmbedder, HttpFetcher, ModelKind, ModelStore, OnnxEmbedder,
    QuantizedEmbedding, quantize,
};
use hm_index::simd::simd_dot;
use std::path::Path;

const DIMENSIONS: usize = 128;
const PROBES: usize = 20;
const PREFILTER_CANDIDATES: usize = 128;

pub struct EncoderRun {
    pub encoder: String,
    pub metrics: Vec<Metric>,
}

#[derive(Clone, Debug)]
struct Record {
    lsn: u64,
    vector: Vec<i8>,
    prefilter: u128,
}

pub(crate) struct SeededVectorCorpus {
    embedder: HashFeatureEmbedder,
    records: Vec<Record>,
}

impl SeededVectorCorpus {
    pub(crate) fn build(count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let embedder = HashFeatureEmbedder::new(DIMENSIONS)?;
        let mut records = Vec::with_capacity(count);
        seed_range(&embedder, &mut records, 0, count)?;
        Ok(Self { embedder, records })
    }

    pub(crate) fn query_index(
        &self,
        index: usize,
        limit: usize,
    ) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
        let query = quantize(&self.embedder.embed_query(&anchors(index))?)?;
        search(&self.records, &query, limit)
    }
}

pub fn run() -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
    let embedder = HashFeatureEmbedder::new(DIMENSIONS)?;
    let mut records = Vec::with_capacity(100_000);
    let mut metrics = Vec::new();
    for (start, end) in [(0, 1_000), (1_000, 10_000), (10_000, 100_000)] {
        seed_range(&embedder, &mut records, start, end)?;
        let (recall, mrr) = measure(&embedder, &records)?;
        metrics.push(ratio_metric(format!("recall_at_10_{end}"), recall));
        metrics.push(ratio_metric(format!("mrr_{end}"), mrr));
    }
    Ok(metrics)
}

pub fn run_local() -> Result<EncoderRun, Box<dyn std::error::Error>> {
    let spec = ModelKind::BgeSmallEnV15.spec();
    let model_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/hm-models");
    let embedder = std::thread::spawn(move || {
        OnnxEmbedder::download(
            ModelKind::BgeSmallEnV15,
            &ModelStore::new(model_root),
            &HttpFetcher,
        )
    })
    .join()
    .map_err(|_| "local encoder initialization panicked")??;
    let documents: Vec<_> = SEMANTIC_PAIRS.iter().map(|pair| pair.0).collect();
    let prototypes = embedder
        .embed_documents(&documents)?
        .into_iter()
        .map(|embedding| quantize(&embedding))
        .collect::<Result<Vec<_>, _>>()?;
    let mut metrics = Vec::new();
    for count in [1_000, 10_000, 100_000] {
        let records: Vec<_> = (0..count)
            .map(|index| SemanticRecord {
                topic: index % prototypes.len(),
                prefilter: semantic_prefilter(&prototypes[index % prototypes.len()]),
            })
            .collect();
        let mut recalled = 0_usize;
        let mut reciprocal_rank = 0.0;
        for (topic, (_, query)) in SEMANTIC_PAIRS.iter().enumerate() {
            let query = quantize(&embedder.embed_query(query)?)?;
            let hits = semantic_search(&records, &prototypes, &query, 10)?;
            if let Some(rank) = hits.iter().position(|candidate| *candidate == topic) {
                recalled += 1;
                reciprocal_rank += 1.0 / (rank + 1) as f64;
            }
        }
        let denominator = SEMANTIC_PAIRS.len() as f64;
        metrics.push(ratio_metric(
            format!("semantic_recall_at_10_{count}"),
            recalled as f64 / denominator,
        ));
        metrics.push(ratio_metric(
            format!("semantic_mrr_{count}"),
            reciprocal_rank / denominator,
        ));
    }
    Ok(EncoderRun {
        encoder: format!("{}@{}", embedder.report_label(), spec.revision),
        metrics,
    })
}

#[derive(Clone, Copy)]
struct SemanticRecord {
    topic: usize,
    prefilter: u128,
}

fn semantic_search(
    records: &[SemanticRecord],
    prototypes: &[QuantizedEmbedding],
    query: &QuantizedEmbedding,
    limit: usize,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let query_prefilter = semantic_prefilter(query);
    let mut best_distance = vec![u32::MAX; prototypes.len()];
    for record in records {
        let distance = (query_prefilter ^ record.prefilter).count_ones();
        best_distance[record.topic] = best_distance[record.topic].min(distance);
    }
    let mut candidates: Vec<_> = best_distance.into_iter().enumerate().collect();
    candidates.sort_unstable_by_key(|(topic, distance)| (*distance, *topic));
    candidates.truncate(PREFILTER_CANDIDATES.min(candidates.len()));
    let mut scored = candidates
        .into_iter()
        .map(|(topic, _)| {
            simd_dot(&query.values, &prototypes[topic].values)
                .map(|score| (score, topic))
                .ok_or_else(|| "semantic vector dimension mismatch".into())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    scored.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored.dedup_by_key(|(_, topic)| *topic);
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(_, topic)| topic).collect())
}

fn semantic_prefilter(embedding: &QuantizedEmbedding) -> u128 {
    u128::from_ne_bytes(
        embedding.binary_prefilter[..16]
            .try_into()
            .expect("BGE has at least 128 dimensions"),
    )
}

const SEMANTIC_PAIRS: [(&str, &str); 12] = [
    (
        "The automobile needs petrol from a service station.",
        "Where can the car be refueled?",
    ),
    (
        "A physician prescribed medicine for the illness.",
        "What treatment did the doctor order?",
    ),
    (
        "The laptop battery is charged with a USB-C adapter.",
        "How does the notebook computer receive power?",
    ),
    (
        "The train departs the railway platform before sunrise.",
        "When does the locomotive leave the station?",
    ),
    (
        "The puppy sleeps beside the fireplace every evening.",
        "Where does the young dog rest at night?",
    ),
    (
        "Fresh vegetables are stored in the refrigerator drawer.",
        "Where are the produce items kept cold?",
    ),
    (
        "The software defect was corrected in the latest release.",
        "Which version repaired the application bug?",
    ),
    (
        "The attorney filed the contract with the county clerk.",
        "Who submitted the legal agreement to the local office?",
    ),
    (
        "Heavy rainfall flooded the road near the old bridge.",
        "What weather made the street by the crossing impassable?",
    ),
    (
        "The musician tuned the guitar before the concert.",
        "What instrument did the performer prepare for the show?",
    ),
    (
        "The spacecraft entered orbit around the distant planet.",
        "What began circling the remote world?",
    ),
    (
        "The baker kneaded dough for the morning bread.",
        "What did the cook prepare for the breakfast loaves?",
    ),
];

fn seed_range(
    embedder: &HashFeatureEmbedder,
    records: &mut Vec<Record>,
    start: usize,
    end: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for batch_start in (start..end).step_by(512) {
        let batch_end = (batch_start + 512).min(end);
        let documents: Vec<_> = (batch_start..batch_end).map(document).collect();
        let references: Vec<_> = documents.iter().map(String::as_str).collect();
        for (offset, embedding) in embedder
            .embed_documents(&references)?
            .into_iter()
            .enumerate()
        {
            let quantized = quantize(&embedding)?;
            records.push(Record {
                lsn: (batch_start + offset + 1) as u64,
                vector: quantized.values,
                prefilter: packed_prefilter(&quantized.binary_prefilter)?,
            });
        }
    }
    Ok(())
}

fn measure(
    embedder: &HashFeatureEmbedder,
    records: &[Record],
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let mut recalled = 0_usize;
    let mut reciprocal_rank = 0.0;
    for index in probe_indices(records.len()) {
        let query = quantize(&embedder.embed_query(&anchors(index))?)?;
        let hits = search(records, &query, 10)?;
        if let Some(rank) = hits.iter().position(|lsn| *lsn == index as u64 + 1) {
            recalled += 1;
            reciprocal_rank += 1.0 / (rank + 1) as f64;
        }
    }
    Ok((
        recalled as f64 / PROBES as f64,
        reciprocal_rank / PROBES as f64,
    ))
}

fn search(
    records: &[Record],
    query: &QuantizedEmbedding,
    limit: usize,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let query_prefilter = packed_prefilter(&query.binary_prefilter)?;
    let mut buckets: [Vec<usize>; DIMENSIONS + 1] = std::array::from_fn(|_| Vec::new());
    for (index, record) in records.iter().enumerate() {
        let distance = (query_prefilter ^ record.prefilter).count_ones() as usize;
        buckets[distance].push(index);
    }
    let mut candidates = Vec::with_capacity(PREFILTER_CANDIDATES);
    for bucket in buckets {
        let remaining = PREFILTER_CANDIDATES.saturating_sub(candidates.len());
        candidates.extend(bucket.into_iter().take(remaining));
        if candidates.len() == PREFILTER_CANDIDATES {
            break;
        }
    }
    let mut scored = candidates
        .into_iter()
        .map(|index| {
            let record = &records[index];
            simd_dot(&query.values, &record.vector)
                .map(|score| (score, record.lsn))
                .ok_or_else(|| "vector dimension mismatch".into())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    scored.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(_, lsn)| lsn).collect())
}

fn packed_prefilter(bytes: &[u8]) -> Result<u128, Box<dyn std::error::Error>> {
    Ok(u128::from_ne_bytes(bytes.try_into()?))
}

fn probe_indices(count: usize) -> impl Iterator<Item = usize> {
    (0..PROBES).map(move |probe| {
        let mixed = 0x5eed_u64
            .wrapping_add(u64::try_from(probe).expect("probe fits u64"))
            .wrapping_mul(6_364_136_223_846_793_005)
            .rotate_left(23);
        usize::try_from(mixed % u64::try_from(count).expect("count fits u64"))
            .expect("reduced probe fits usize")
    })
}

fn anchors(index: usize) -> String {
    (0..16)
        .map(|facet| format!("topic{index}facet{facet}hypermind"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn document(index: usize) -> String {
    anchors(index)
}

fn ratio_metric(name: String, value: f64) -> Metric {
    Metric {
        name,
        value,
        unit: "ratio".to_owned(),
        tolerance: 0.01,
        higher_is_better: true,
        judged: false,
    }
}
