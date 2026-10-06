use hm_context::{Scope, digest_bytes, maintenance::JobKind};
use hm_core::ActorId;
use hm_llm::provider_usage::{ObservationFormat, ObservationMetadata, ProviderUsageSnapshot};
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_config::TrustedContextConfig,
    uds::ToolDispatcher,
    usage_service::{self, ReservationRequest, UsageAttribution, UsageLimits, UsageRollupWire},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
const LARGE: u64 = 9_007_199_254_740_993;
fn scope() -> Scope {
    Scope {
        owner_id: "precision-owner".into(),
        project_id: "usage".into(),
        workspace_id: None,
    }
}
fn config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("ledger/7"),
        actor: ActorId::new(7),
        user: [0x11; 16],
        kek: [0x22; 32],
        projection_map_bytes: 64 * 1024 * 1024,
    }
}
async fn public(actor: &ActorEngine) -> Value {
    let dispatcher =
        McpToolDispatcher::from_env()
            .unwrap()
            .with_context_scope(TrustedContextConfig {
                version: 1,
                actor: 7,
                scope: scope(),
            });
    let bytes = dispatcher
        .dispatch(
            actor.clone(),
            "inspect".into(),
            serde_json::to_vec(&json!({"uri":"hm://7/context-usage"})).unwrap(),
        )
        .await
        .unwrap();
    let reply: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reply["ok"], true, "{reply}");
    reply["items"][0].clone()
}
#[tokio::test]
async fn actual_native_mcp_usage_quantities_preserve_exact_public_decimals() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("HM_USAGE_CONSUMER_FIXTURE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    std::fs::create_dir_all(&root).unwrap();
    let actor = ActorEngine::open(config(&root)).await.unwrap();
    usage_service::configure(
        &actor,
        &scope(),
        &scope(),
        UsageLimits {
            total_tokens: u64::MAX,
            hourly_tokens: u64::MAX,
            daily_tokens: u64::MAX,
            job_tokens: u64::MAX,
            concurrency: 2,
            lease_ms: 60000,
        },
    )
    .await
    .unwrap();
    let reservation = usage_service::reserve(
        &actor,
        &scope(),
        &scope(),
        ReservationRequest {
            request_id: "exact-reservation".into(),
            attribution: UsageAttribution {
                job_id: "precision-job".into(),
                worker_id: "precision-worker".into(),
                session_id: "precision-session".into(),
                turn_id: "precision-turn".into(),
                provider_id: "native-development".into(),
                model_id: "pending-model".into(),
                source_ids: BTreeSet::from(["measurement-record".into()]),
            },
            kind: JobKind::Extraction,
            reserved_tokens: LARGE,
            input_digest: digest_bytes(b"Read the measurement record."),
        },
    )
    .await
    .unwrap();
    usage_service::mark_unknown(&actor, &scope(), &scope(), &reservation.id)
        .await
        .unwrap();
    let reply = public(&actor).await;
    assert_eq!(reply["version"], 1);
    assert_eq!(reply["rollup"]["held_tokens"], LARGE.to_string());
    assert_eq!(reply["rollup"]["known_tokens"], "0");
    assert_eq!(reply["rollup"]["unknown_reservations"], "1");
    assert_eq!(reply["rollup"]["observed_calls"], "0");
    assert_eq!(
        reply["state"]["limits"]["total_tokens"],
        u64::MAX.to_string()
    );
    assert_eq!(
        reply["state"]["reservations"][0]["reserved_tokens"],
        LARGE.to_string()
    );
    assert_eq!(reply["state"]["reservations"][0]["id"], reservation.id);
    assert!(reply["state"]["reservations"][0]["settled_tokens"].is_null());
    assert!(
        reply["state"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!reply.to_string().contains("original_bytes"));
    assert!(!reply.to_string().contains("raw_json"));
    let typed: UsageRollupWire = serde_json::from_value(reply["rollup"].clone()).unwrap();
    assert_eq!(typed.held_tokens, LARGE);
    let mut legacy = reply["rollup"].clone();
    legacy["held_tokens"] = json!(42);
    assert_eq!(
        serde_json::from_value::<UsageRollupWire>(legacy.clone())
            .unwrap()
            .held_tokens,
        42
    );
    legacy["held_tokens"] = json!(LARGE);
    assert!(serde_json::from_value::<UsageRollupWire>(legacy).is_err());
    for malformed in ["09007199254740993", "+1", "18446744073709551616", "1.0"] {
        let mut bad = reply["rollup"].clone();
        bad["held_tokens"] = json!(malformed);
        assert!(serde_json::from_value::<UsageRollupWire>(bad).is_err());
    }
    let vectors = parser_vectors();
    std::fs::write(
        root.join("usage_expected.json"),
        serde_json::to_vec(&reply).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("usage_parser_vectors.json"),
        serde_json::to_vec(&vectors).unwrap(),
    )
    .unwrap();
    let manifest = json!({"scope":scope(),"actor":7,"user_hex":"11".repeat(16),"kek_hex":"22".repeat(32),"actor_directory":root.join("ledger/7"),"expected":{"held_tokens":LARGE.to_string(),"unknown_reservations":"1","known_tokens":"0","reservation_id":reservation.id}});
    let manifest_path = root.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&manifest_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(&root)).await.unwrap();
    assert_eq!(public(&actor).await, reply);
    actor.shutdown().await.unwrap();
}
fn parser_vectors() -> Value {
    let metadata = ObservationMetadata {
        provider_id: "parser-vector".into(),
        source: "recorded-parser-input".into(),
        evidence_id: "precision-vector".into(),
        observed_at_ns: 9_007_199_254_740_993,
        expires_at_ns: None,
        format: ObservationFormat::Ollama,
    };
    let tokens=ProviderUsageSnapshot::parse(br#"{"prompt_eval_count":"9007199254740993","eval_count":0,"usage":{"cost":"9007199.254740993"},"message":{"content":"excluded-response-content"}}"#,metadata.clone()).unwrap();
    assert_eq!(tokens.tokens.input, Some(LARGE));
    assert_eq!(tokens.tokens.output, Some(0));
    assert_eq!(tokens.tokens.cache_read, None);
    let token_wire = usage_service::public_provider_snapshot(&tokens);
    assert_eq!(token_wire["tokens"]["input"], LARGE.to_string());
    assert_eq!(token_wire["tokens"]["output"], "0");
    assert_eq!(
        token_wire["reported_charge"]["magnitude"]["nanodollar_numerator"],
        LARGE.to_string()
    );
    assert!(!token_wire.to_string().contains("excluded-response-content"));
    assert!(!token_wire.to_string().contains("original_bytes"));
    assert!(
        ProviderUsageSnapshot::parse(
            br#"{"prompt_eval_count":"18446744073709551616"}"#,
            metadata.clone()
        )
        .is_err()
    );
    let mut account_metadata = metadata;
    account_metadata.format = ObservationFormat::AccountQuota;
    let quota=ProviderUsageSnapshot::parse(br#"{"data":{"balance":"9007199.254740993","quota_windows":[{"name":"capacity","unit":"tokens","limit":"18446744073709551615","remaining":0,"starts_at_ns":"-9007199254740993","resets_at_ns":"9223372036854775807","refill":{"amount":"9007199254740993","interval_ms":"9007199254740993"}}],"funding":{"total_credits":0}}}"#,account_metadata).unwrap();
    let quota_wire = usage_service::public_provider_snapshot(&quota);
    assert_eq!(
        quota_wire["quota_windows"][0]["limit"]["value"],
        u64::MAX.to_string()
    );
    assert_eq!(quota_wire["quota_windows"][0]["remaining"]["value"], "0");
    assert!(quota_wire["quota_windows"][0]["used"].is_null());
    json!({"origin":"recorded_parser_vectors_only","billed":false,"tokens":token_wire,"quota":quota_wire})
}
