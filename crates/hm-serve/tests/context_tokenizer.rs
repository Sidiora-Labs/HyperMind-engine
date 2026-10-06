use hm_context::{history::SourceHistory, *};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_projection::{self, ProjectionRequest, SummaryLevel},
    context_tokenizer::*,
};

#[tokio::test]
async fn pinned_tokenizer_real_provider_and_native_restart_invalidation() {
    let path = std::env::var("HYPERMIND_CONTEXT_TOKENIZERS")
        .expect("owner-installed tokenizer configuration required");
    let config: OwnedTokenizerConfig =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let installed = config
        .tokenizers
        .iter()
        .find(|entry| entry.model_id == "qwen2.5:3b")
        .unwrap()
        .clone();
    assert_eq!(
        installed.tokenizer_revision,
        "aa8e72537993ba99e69dfaafa59ed015b17504d1"
    );
    assert_eq!(
        installed.tokenizer_sha256,
        "c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539"
    );
    let first_identity = identity(&installed.model_id).unwrap();
    assert_eq!(first_identity.model_revision, installed.model_revision);
    assert_eq!(first_identity.generation, 1);
    let counter = counter_for_model(&installed.model_id).unwrap();
    assert_eq!(counter.count(b"").unwrap(), 0);
    assert!(counter.count(&[0xff]).is_err());
    assert!(counter_for_model("unconfigured/model").is_err());
    assert!(count_provider_input(&installed.model_id, 0, b"hello").is_err());
    let mut corrupt = installed.clone();
    corrupt.tokenizer_sha256 = "0".repeat(64);
    assert!(install_owned(corrupt, 1).is_err());
    assert_eq!(identity(&installed.model_id).unwrap(), first_identity);

    let endpoint = std::env::var("HYPERMIND_TOKENIZER_TEST_ENDPOINT")
        .expect("real provider endpoint required");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .unwrap();
    let tags: serde_json::Value = client
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let actual = tags["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == installed.model_id)
        .unwrap();
    assert_eq!(actual["digest"].as_str().unwrap(), installed.model_revision);
    for prompt in [
        "The exact input contains punctuation, whitespace\n and 12345.",
        "UTF-8: 你好 — café 👋",
        "<|im_start|>user\nSay yes.<|im_end|>\n<|im_start|>assistant\n",
    ] {
        let counted = count_provider_input(&installed.model_id, 1, prompt.as_bytes()).unwrap();
        let measured: serde_json::Value = client.post(format!("{endpoint}/api/generate")).json(&serde_json::json!({"model":installed.model_id,"prompt":prompt,"raw":true,"stream":false,"options":{"num_predict":1,"temperature":0}})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
        assert_eq!(measured["done"], true);
        assert_eq!(
            measured["prompt_eval_count"].as_u64().unwrap(),
            counted.tokens
        );
        assert!(measured["eval_count"].as_u64().unwrap() <= 1);
    }

    let text = installed.ollama_text.as_ref().unwrap();
    let observed: serde_json::Value = client
        .post(format!("{endpoint}/api/show"))
        .json(&serde_json::json!({"model":installed.model_id}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(observed["system"].as_str().unwrap(), text.system);
    assert_eq!(
        observed["template"].as_str().unwrap().as_bytes(),
        std::fs::read(&text.template_path).unwrap()
    );
    let final_content = "Give one short word. Exact user payload 你好.";
    let framed = count_ollama_user_input(&installed.model_id, 1, final_content).unwrap();
    let measured: serde_json::Value = client.post(format!("{endpoint}/api/chat")).json(&serde_json::json!({"model":installed.model_id,"messages":[{"role":"user","content":final_content}],"stream":false,"options":{"num_predict":1,"temperature":0}})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    assert_eq!(measured["done"], true);
    assert_eq!(
        measured["prompt_eval_count"].as_u64().unwrap(),
        framed.tokens
    );
    assert!(measured["eval_count"].as_u64().unwrap() <= 1);

    let scope = Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    };
    let mut history = SourceHistory::new(scope.clone(), "session").unwrap();
    let mut source = SourceMessage {
        id: "source".into(),
        ordinal: 1,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: "Preserve original context 你好.".into(),
        }],
        occurred_at_ns: None,
        recorded_at_ns: 1,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    source.source_digest = source.computed_digest().unwrap();
    history.ingest(source, vec![0xff, 0, 0x80]).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let actor_config = || ActorConfig {
        actor_directory: directory.path().into(),
        actor: ActorId::new(18),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    };
    let request = ProjectionRequest {
        model_id: installed.model_id.clone(),
        policy_revision: "policy-1".into(),
        permission_revision: "permission-1".into(),
        required_blocks: vec![],
        required_message_ids: vec![],
        tier: SummaryLevel::Detailed,
        defer_reductions: false,
    };
    let actor = ActorEngine::open(actor_config()).await.unwrap();
    let first = context_projection::assemble(
        &actor,
        &history,
        request.clone(),
        actor.stats().await.unwrap().applied.last_lsn,
    )
    .await
    .unwrap();
    assert!(counter.count(&first.bytes).unwrap() > 0);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(actor_config()).await.unwrap();
    assert_eq!(
        context_projection::current(&actor, &scope, "session")
            .await
            .unwrap()
            .unwrap()
            .bytes,
        first.bytes
    );
    let changed = install_owned(installed.clone(), 1).unwrap();
    assert_eq!(changed.generation, 2);
    assert!(counter.ensure_current().is_err());
    assert!(count_provider_input(&installed.model_id, 1, b"hello").is_err());
    assert!(
        context_projection::current(&actor, &scope, "session")
            .await
            .is_err()
    );
    let rebuilt = context_projection::assemble(
        &actor,
        &history,
        request,
        actor.stats().await.unwrap().applied.last_lsn,
    )
    .await
    .unwrap();
    assert!(rebuilt.generation > first.generation);
    assert_ne!(rebuilt.fence, first.fence);
    assert_eq!(rebuilt.bytes, first.bytes);
    assert_eq!(
        context_projection::expand(&actor, &scope, "session", &rebuilt.coverage[0])
            .await
            .unwrap(),
        vec![0xff, 0, 0x80]
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(actor_config()).await.unwrap();
    assert_eq!(
        context_projection::current(&actor, &scope, "session")
            .await
            .unwrap()
            .unwrap()
            .generation,
        rebuilt.generation
    );
    actor.shutdown().await.unwrap();
}
