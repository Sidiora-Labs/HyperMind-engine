use hm_compose::rerank::{
    CrossEncoder, DeploymentConfig, DeploymentReranker, EvaluationEvidence, MODEL_REVISION,
    MODEL_SHA256, RerankError, TOKENIZER_SHA256, TOP_K,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[test]
fn disabled_deployment_needs_neither_model_nor_evaluation() {
    let reranker = DeploymentReranker::open(
        &DeploymentConfig::default(),
        Path::new("absent-model.onnx"),
        Path::new("absent-tokenizer.json"),
    )
    .unwrap();
    assert!(!reranker.enabled());
    assert_eq!(
        reranker.rank("", &["one", "two", "three"]).unwrap(),
        [0, 1, 2]
    );
}

#[test]
fn activation_requires_deployment_bound_measured_gain() {
    let mut config = DeploymentConfig {
        enabled: true,
        deployment_id: "deployment-a".to_owned(),
        evidence: None,
    };
    let check = |config: &DeploymentConfig| {
        assert!(matches!(
            DeploymentReranker::open(config, Path::new("absent"), Path::new("absent")),
            Err(RerankError::UnqualifiedDeployment)
        ));
    };
    check(&config);
    config.evidence = Some(EvaluationEvidence {
        producer: "hm-eval".to_owned(),
        deployment_id: config.deployment_id.clone(),
        model_sha256: MODEL_SHA256.to_owned(),
        tokenizer_sha256: TOKENIZER_SHA256.to_owned(),
        evaluation_id: "negative-policy-test".to_owned(),
        evaluated_queries: 10,
        baseline_mrr_at_10: 0.75,
        reranked_mrr_at_10: 0.75,
    });
    check(&config);
    let evidence = config.evidence.as_mut().unwrap();
    evidence.reranked_mrr_at_10 = f64::NAN;
    check(&config);
    let evidence = config.evidence.as_mut().unwrap();
    evidence.reranked_mrr_at_10 = 0.9;
    evidence.deployment_id = "deployment-b".to_owned();
    check(&config);
    let evidence = config.evidence.as_mut().unwrap();
    evidence.deployment_id = "deployment-a".to_owned();
    evidence.model_sha256 = "another-model".to_owned();
    check(&config);
    let evidence = config.evidence.as_mut().unwrap();
    evidence.model_sha256 = MODEL_SHA256.to_owned();
    evidence.evaluated_queries = 0;
    check(&config);
}

#[test]
fn pinned_cross_encoder_ranks_real_passages_and_preserves_tail() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/hm-models/cross-encoder--ms-marco-MiniLM-L6-v2")
        .join(MODEL_REVISION);
    std::fs::create_dir_all(&directory).unwrap();
    let model_path = directory.join("model_quint8_avx2.onnx");
    let tokenizer_path = directory.join("tokenizer.json");
    artifact(&model_path, "onnx/model_quint8_avx2.onnx", MODEL_SHA256);
    artifact(&tokenizer_path, "tokenizer.json", TOKENIZER_SHA256);
    let model = CrossEncoder::open_for_evaluation(&model_path, &tokenizer_path).unwrap();
    let mut passages = vec!["Ripe oranges are stored in the kitchen."; TOP_K + 2];
    passages[7] =
        "The engine stores its durable append-only event log on disk before acknowledging a write.";
    let order = model
        .rank_top_32(
            "When does the engine acknowledge durable writes?",
            &passages,
        )
        .unwrap();
    assert_eq!(order[0], 7);
    assert_eq!(&order[TOP_K..], &[TOP_K, TOP_K + 1]);
    assert_eq!(order.len(), passages.len());
    let expected_rest = (0..TOP_K).filter(|index| *index != 7).collect::<Vec<_>>();
    assert_eq!(&order[1..TOP_K], expected_rest.as_slice());
    assert_eq!(
        model.rank_top_32("", &passages),
        Err(RerankError::InvalidArgument)
    );
    let long_passage = "This passage discusses the storage of fruit. ".repeat(1_000);
    assert_eq!(
        model
            .rank_top_32("How is fruit stored?", &[&long_passage])
            .unwrap(),
        [0]
    );
}

fn artifact(path: &Path, name: &str, digest: &str) {
    if !path.exists() {
        let url = format!(
            "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L6-v2/resolve/{MODEL_REVISION}/{name}"
        );
        let bytes = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), digest);
        std::fs::write(path, bytes).unwrap();
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
        digest
    );
}
