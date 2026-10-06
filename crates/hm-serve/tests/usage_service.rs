use hm_context::{Scope, digest_bytes, maintenance::JobKind};
use hm_core::ActorId;
use hm_llm::provider_usage::{ObservationFormat, ObservationMetadata, ProviderUsageSnapshot};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    usage_service::*,
};
use std::collections::BTreeSet;
fn scope() -> Scope {
    Scope {
        owner_id: "usage-owner".into(),
        project_id: "measurements".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(62),
        user: [8; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn request(id: &str, job: &str, tokens: u64, model: &str, prompt: &str) -> ReservationRequest {
    ReservationRequest {
        request_id: id.into(),
        attribution: UsageAttribution {
            job_id: job.into(),
            worker_id: "native-worker".into(),
            session_id: "measurement-session".into(),
            turn_id: "measurement-turn".into(),
            provider_id: "local-generation".into(),
            model_id: model.into(),
            source_ids: BTreeSet::from(["vessel-observation".into()]),
        },
        kind: JobKind::Extraction,
        reserved_tokens: tokens,
        input_digest: digest_bytes(prompt.as_bytes()),
    }
}
fn query() -> RollupQuery {
    RollupQuery {
        session_id: None,
        turn_id: None,
        job_id: None,
        provider_id: None,
        start_ms: None,
        end_ms: None,
    }
}
#[test]
fn absent_and_explicit_zero_are_distinct_evidence() {
    let metadata = ObservationMetadata {
        provider_id: "local-generation".into(),
        source: "native-provider-response".into(),
        evidence_id: "parser-case".into(),
        observed_at_ns: 1,
        expires_at_ns: None,
        format: ObservationFormat::Ollama,
    };
    let absent = ProviderUsageSnapshot::parse(b"{\"done\":true}", metadata.clone()).unwrap();
    let zero =
        ProviderUsageSnapshot::parse(b"{\"prompt_eval_count\":0,\"eval_count\":0}", metadata)
            .unwrap();
    assert_eq!(absent.tokens.input, None);
    assert_eq!(zero.tokens.input, Some(0));
    assert_eq!(absent.tokens.output, None);
    assert_eq!(zero.tokens.output, Some(0));
    assert_ne!(absent.original_bytes, zero.original_bytes);
    assert_ne!(absent.fingerprint, zero.fingerprint);
}
#[tokio::test]
async fn trusted_actual_observation_reservation_rollup_unknown_and_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let owner = scope();
    configure(
        &actor,
        &owner,
        &owner,
        UsageLimits {
            total_tokens: 1000,
            hourly_tokens: 600,
            daily_tokens: 700,
            job_tokens: 256,
            concurrency: 1,
            lease_ms: 60_000,
        },
    )
    .await
    .unwrap();
    let prompt = "Reply with the vessel serial Zircon-58 only.";
    let initial = request("actual-call", "extraction-job", 256, &model, prompt);
    let reservation = reserve(&actor, &owner, &owner, initial.clone())
        .await
        .unwrap();
    assert_eq!(
        reserve(&actor, &owner, &owner, initial).await.unwrap().id,
        reservation.id
    );
    assert!(
        reserve(
            &actor,
            &owner,
            &owner,
            request("overlap", "parallel-job", 1, &model, prompt)
        )
        .await
        .is_err()
    );
    let observer =
        ProviderObserver::ollama(endpoint, "local-generation".into(), model.clone()).unwrap();
    assert!(
        observer
            .observe(
                &actor,
                &owner,
                &owner,
                &reservation.id,
                "substituted prompt",
                None
            )
            .await
            .is_err()
    );
    let catalog = hm_llm::catalog::ModelCatalog::parse(
        &serde_json::to_vec(&serde_json::json!({"data":[{"id":model,"pricing":{}}]})).unwrap(),
    )
    .unwrap();
    let observation = observer
        .observe(
            &actor,
            &owner,
            &owner,
            &reservation.id,
            prompt,
            Some(&catalog.models[0]),
        )
        .await
        .unwrap();
    assert_eq!(
        observation.catalog_estimate.as_ref().unwrap().basis,
        "catalog_tariff_estimate"
    );
    assert_eq!(
        observation.catalog_estimate.as_ref().unwrap().nanodollars,
        None
    );
    assert!(
        !observation
            .catalog_estimate
            .as_ref()
            .unwrap()
            .unknown
            .is_empty()
    );
    assert_eq!(
        observation.snapshot.observation.format,
        ObservationFormat::Ollama
    );
    let actual =
        observation.snapshot.tokens.input.unwrap() + observation.snapshot.tokens.output.unwrap();
    assert!(actual > 0 && actual <= 256);
    assert_eq!(observation.attribution, reservation.attribution);
    assert_eq!(observation.snapshot.raw["model"], model);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&observation.snapshot.original_bytes).unwrap(),
        observation.snapshot.raw
    );
    let replay = observer
        .observe(&actor, &owner, &owner, &reservation.id, prompt, None)
        .await
        .unwrap();
    assert_eq!(replay.id, observation.id);
    reconcile_unknown(&actor, &owner, &owner, &reservation.id, &observation.id)
        .await
        .unwrap();
    let initial_rollup = rollup(&actor, &owner, &owner, query()).await.unwrap();
    assert_eq!(initial_rollup.known_tokens, actual);
    assert_eq!(initial_rollup.observed_calls, 1);
    assert_eq!(initial_rollup.held_tokens, 0);
    assert_eq!(
        inspect(&actor, &owner, &owner)
            .await
            .unwrap()
            .accounting
            .spent,
        actual
    );
    assert!(
        reserve(
            &actor,
            &owner,
            &owner,
            request("per-job-ceiling", "extraction-job", 257, &model, prompt)
        )
        .await
        .is_err()
    );
    let unknown = reserve(
        &actor,
        &owner,
        &owner,
        request("interrupted-call", "unknown-job", 256, &model, prompt),
    )
    .await
    .unwrap();
    mark_unknown(&actor, &owner, &owner, &unknown.id)
        .await
        .unwrap();
    assert!(
        reconcile_unknown(
            &actor,
            &owner,
            &owner,
            &unknown.id,
            "request-authored-actual-0"
        )
        .await
        .is_err()
    );
    assert!(
        reconcile_unknown(&actor, &owner, &owner, &unknown.id, &observation.id)
            .await
            .is_err()
    );
    let second_tokens = (600 - actual - 256).min(256);
    let second = reserve(
        &actor,
        &owner,
        &owner,
        request(
            "second-interruption",
            "second-unknown-job",
            second_tokens,
            &model,
            prompt,
        ),
    )
    .await
    .unwrap();
    mark_unknown(&actor, &owner, &owner, &second.id)
        .await
        .unwrap();
    let held = 256 + second_tokens;
    let overflow = 600 - actual - held + 1;
    assert!(
        reserve(
            &actor,
            &owner,
            &owner,
            request("hour-ceiling", "hour-job", overflow, &model, prompt)
        )
        .await
        .is_err()
    );
    let rollup_before = rollup(&actor, &owner, &owner, query()).await.unwrap();
    assert_eq!(rollup_before.known_tokens, actual);
    assert_eq!(rollup_before.held_tokens, held);
    assert_eq!(rollup_before.unknown_reservations, 2);
    assert_eq!(rollup_before.observed_calls, 1);
    assert_eq!(rollup_before.attributions.len(), 3);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let restored = inspect(&actor, &owner, &owner).await.unwrap();
    assert_eq!(
        restored.observations[&observation.id]
            .snapshot
            .original_bytes,
        observation.snapshot.original_bytes
    );
    assert_eq!(restored.accounting.spent, actual);
    assert_eq!(
        restored.accounting.unknown_usage.values().sum::<u64>(),
        held
    );
    let after = rollup(&actor, &owner, &owner, query()).await.unwrap();
    assert_eq!(after.known_tokens, actual);
    assert_eq!(after.held_tokens, held);
    assert_eq!(after.observed_calls, 1);
    let mut turn = query();
    turn.turn_id = Some("foreign-turn".into());
    assert_eq!(
        rollup(&actor, &owner, &owner, turn)
            .await
            .unwrap()
            .known_tokens,
        0
    );
    actor.shutdown().await.unwrap();
    let daily_dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(daily_dir.path())).await.unwrap();
    configure(
        &actor,
        &owner,
        &owner,
        UsageLimits {
            total_tokens: 1000,
            hourly_tokens: 1000,
            daily_tokens: 300,
            job_tokens: 1000,
            concurrency: 1,
            lease_ms: 60_000,
        },
    )
    .await
    .unwrap();
    assert!(
        reserve(
            &actor,
            &owner,
            &owner,
            request("daily-ceiling", "day-job", 301, &model, prompt)
        )
        .await
        .is_err()
    );
    actor.shutdown().await.unwrap();
}
