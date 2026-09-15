#![forbid(unsafe_code)]

use hm_embed::{
    Artifact, ArtifactFetcher, CachedEmbedder, Distance, EmbedError, Embedder, Embedding,
    HashFeatureEmbedder, InputRole, ModelKind, ModelSpec, ModelStore, Normalization, Provider,
    QuantizedEmbedding, RecordedTransport, RemoteConfig, RemoteEmbedder, SpaceIdentity, quantize,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn hash_feature_is_deterministic_and_permanently_lexical_only() {
    let embedder = HashFeatureEmbedder::new(32).unwrap();
    let first = embedder.embed_query("alpha beta alpha").unwrap();
    let second = embedder.embed_query("alpha beta alpha").unwrap();
    assert_eq!(first, second);
    assert_eq!(first.space.encoder_id, "lexical_only");
    assert_eq!(first.space.input_role, InputRole::Query);
    assert_eq!(embedder.report_label(), "lexical_only");
    assert!((norm(&first.values) - 1.0).abs() < 1e-6);
}

#[test]
fn quantization_derives_int8_and_one_bit_prefilter() {
    let embedding = Embedding {
        space: identity(InputRole::Document, 5),
        values: vec![-1.0, -0.5, 0.0, 0.5, 1.0],
    };
    let QuantizedEmbedding {
        values,
        binary_prefilter,
        inverse_scale,
        ..
    } = quantize(&embedding).unwrap();
    assert_eq!(values, [-127, -64, 0, 64, 127]);
    assert_eq!(binary_prefilter, [0b0001_1100]);
    assert!((inverse_scale - 1.0 / 127.0).abs() < f32::EPSILON);
}

#[test]
fn content_hash_cache_batches_misses_and_preserves_roles() {
    let calls = Arc::new(AtomicUsize::new(0));
    let embedder = CountingEmbedder {
        calls: calls.clone(),
    };
    let cached = CachedEmbedder::new(embedder, 2).unwrap();
    let first = cached
        .embed_documents(&["one", "two", "three", "one"])
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(first[0], first[3]);
    assert_eq!(cached.cached_items(), 3);
    let second = cached.embed_documents(&["three", "one"]).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(second[0], first[2]);
    cached.embed_query("one").unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert_eq!(cached.cached_items(), 4);
}

#[test]
fn model_store_downloads_once_and_rejects_digest_changes() {
    let temporary = tempfile::tempdir().unwrap();
    let fetcher = FixtureFetcher::default();
    let spec = fixture_spec();
    let store = ModelStore::new(temporary.path());
    let first = store.ensure(spec, &fetcher).unwrap();
    assert_eq!(std::fs::read(&first.model).unwrap(), b"model");
    assert_eq!(std::fs::read(&first.tokenizer).unwrap(), b"tokenizer");
    assert_eq!(fetcher.calls.load(Ordering::Relaxed), 2);
    assert_eq!(store.ensure(spec, &fetcher).unwrap(), first);
    assert_eq!(fetcher.calls.load(Ordering::Relaxed), 2);
    std::fs::write(first.model, b"tampered").unwrap();
    assert!(matches!(
        store.ensure(spec, &fetcher),
        Err(EmbedError::ArtifactDigest { .. })
    ));
}

#[test]
fn pinned_model_spaces_are_disjoint_even_at_the_same_role() {
    let nomic = ModelKind::NomicEmbedTextV15.spec();
    let bge = ModelKind::BgeSmallEnV15.spec();
    assert_ne!(nomic.encoder_id, bge.encoder_id);
    assert_ne!(nomic.revision, bge.revision);
    assert_eq!(nomic.dimensions, 768);
    assert_eq!(bge.dimensions, 384);
    assert_eq!(nomic.model.sha256.len(), 64);
    assert_eq!(bge.model.sha256.len(), 64);
}

#[test]
fn every_remote_provider_matches_its_recorded_wire_fixture() {
    for (provider, fixture) in [
        (Provider::OpenAi, include_str!("fixtures/openai.json")),
        (Provider::Voyage, include_str!("fixtures/voyage.json")),
        (Provider::Vertex, include_str!("fixtures/vertex.json")),
        (Provider::Ollama, include_str!("fixtures/ollama.json")),
    ] {
        let transport = RecordedTransport::from_json(fixture).unwrap();
        let embedder = RemoteEmbedder::new(provider, remote_config(), transport).unwrap();
        let embeddings = embedder.embed_documents(&["alpha", "beta"]).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].values, [0.6, 0.8, 0.0]);
        assert_eq!(embeddings[1].values, [0.0, 0.0, 1.0]);
        assert_eq!(embeddings[0].space.input_role, InputRole::Document);
        assert_eq!(embedder.transport().remaining(), 0);
    }
}

fn identity(role: InputRole, dimensions: usize) -> SpaceIdentity {
    SpaceIdentity {
        encoder_id: "test".to_owned(),
        revision: "v1".to_owned(),
        dimensions,
        distance: Distance::Cosine,
        normalization: Normalization::L2,
        input_role: role,
    }
}

fn norm(vector: &[f32]) -> f32 {
    vector.iter().map(|value| value * value).sum::<f32>().sqrt()
}

struct CountingEmbedder {
    calls: Arc<AtomicUsize>,
}

impl Embedder for CountingEmbedder {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        identity(role, 2)
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(Embedding {
            space: self.identity(InputRole::Query),
            values: values(query),
        })
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(documents
            .iter()
            .map(|document| Embedding {
                space: self.identity(InputRole::Document),
                values: values(document),
            })
            .collect())
    }
}

fn values(text: &str) -> Vec<f32> {
    let digest = blake3::hash(text.as_bytes());
    vec![
        f32::from(digest.as_bytes()[0]),
        f32::from(digest.as_bytes()[1]),
    ]
}

#[derive(Default)]
struct FixtureFetcher {
    calls: AtomicUsize,
    responses: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl ArtifactFetcher for FixtureFetcher {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, EmbedError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if let Some(response) = self.responses.lock().unwrap().get(url).cloned() {
            return Ok(response);
        }
        match url {
            "fixture://model" => Ok(b"model".to_vec()),
            "fixture://tokenizer" => Ok(b"tokenizer".to_vec()),
            _ => Err(EmbedError::Network("unknown fixture URL".to_owned())),
        }
    }
}

const fn fixture_spec() -> ModelSpec {
    ModelSpec {
        kind: ModelKind::BgeSmallEnV15,
        encoder_id: "fixture/model",
        revision: "fixture-v1",
        dimensions: 3,
        maximum_tokens: 8,
        model: Artifact {
            name: "model.onnx",
            url: "fixture://model",
            sha256: "9372c470eeadd5ecd9c3c74c2b3cb633f8e2f2fad799250a0f70d652b6b825e4",
        },
        tokenizer: Artifact {
            name: "tokenizer.json",
            url: "fixture://tokenizer",
            sha256: "5f97e3774c51edd1d63706c2ec3826c564a067794770cdab0f8c4797971cacf9",
        },
    }
}

fn remote_config() -> RemoteConfig {
    RemoteConfig {
        endpoint: "https://fixture.invalid/embeddings".to_owned(),
        api_key: Some("fixture-key".to_owned()),
        model: "fixture-model".to_owned(),
        revision: "fixture-v1".to_owned(),
        dimensions: 3,
        maximum_batch: 16,
    }
}
