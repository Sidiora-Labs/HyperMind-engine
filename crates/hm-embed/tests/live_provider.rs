#![forbid(unsafe_code)]

use hm_embed::{
    Embedder, HttpTransport, InputRole, Provider, RemoteConfig, RemoteEmbedder, quantize,
};

#[test]
#[ignore = "requires CENTRA_GATEWAY_API_KEY and incurs a provider request"]
fn centra_openai_compatible_embedding_lifecycle() {
    let api_key = std::env::var("CENTRA_GATEWAY_API_KEY").expect("CENTRA_GATEWAY_API_KEY");
    let mut endpoint = std::env::var("CENTRA_GATEWAY_URL")
        .unwrap_or_else(|_| "https://gateway.centra.ag".to_owned());
    endpoint = endpoint.trim_end_matches('/').to_owned();
    if !endpoint.ends_with("/v1/embeddings") {
        if endpoint.ends_with("/v1") {
            endpoint.push_str("/embeddings");
        } else {
            endpoint.push_str("/v1/embeddings");
        }
    }
    let embedder = RemoteEmbedder::new(
        Provider::OpenAi,
        RemoteConfig {
            endpoint,
            api_key: Some(api_key),
            model: "openrouter/openai/text-embedding-3-large".to_owned(),
            revision: "centra-openrouter-live".to_owned(),
            dimensions: 3072,
            maximum_batch: 16,
        },
        HttpTransport::default(),
    )
    .unwrap();
    let query = embedder
        .embed_query("The quick brown fox jumps over the lazy dog")
        .unwrap();
    let documents = embedder
        .embed_documents(&[
            "A fox jumps over a dog.",
            "A database checkpoint commits a log prefix.",
        ])
        .unwrap();
    assert_eq!(query.space.input_role, InputRole::Query);
    assert_eq!(documents.len(), 2);
    assert_eq!(query.values.len(), 3072);
    assert_eq!(documents[0].values.len(), 3072);
    let quantized = quantize(&query).unwrap();
    assert_eq!(quantized.values.len(), 3072);
    assert_eq!(quantized.binary_prefilter.len(), 384);
}
