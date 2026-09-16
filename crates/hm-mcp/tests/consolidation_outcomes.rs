#![forbid(unsafe_code)]

use hm_core::{ActorId, LSN};
use hm_cortex::nrem::cluster::ObservationCluster;
use hm_cortex::nrem::merge::{MERGE_OUTPUT_TOKENS, merge_request};
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest, WireFixture,
    WireRequest, WireResponse,
};
use hm_mcp::{
    ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, ConsolidationRuntime,
    McpServer, RememberInput, RememberKind,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const ENDPOINT: &str = "https://fixture.invalid/v1/chat/completions";
const MODEL: &str = "fixture-merge-model";
const CONVERSATIONS: [&str; 2] = ["kestrel-rotation-a", "kestrel-rotation-b"];
const DEFINITION: &str = "Kestrel deploy tokens rotate on every release and each rotation failure is written to the release log.";

fn actor_config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 64 * 1024 * 1024,
    }
}

async fn seeded_actor(path: &Path) -> ActorEngine {
    let actor = ActorEngine::open(actor_config(path)).await.unwrap();
    let writer = McpServer::new(actor.clone());
    for index in 0..6_usize {
        let remembered = writer
            .remember_envelope(RememberInput {
                conversation: CONVERSATIONS[index % CONVERSATIONS.len()].to_owned(),
                content: format!("turn {index:02}. {DEFINITION}"),
                kind: RememberKind::User,
                chunk_bytes: None,
                anchor: None,
                retention: None,
                sensitivity: None,
                vocabulary: None,
                source: None,
                derive: None,
                source_delivery: None,
                source_settlement: None,
                document: None,
                source_sync: None,
            })
            .await;
        assert!(remembered.ok, "{remembered:?}");
        assert_eq!(
            remembered.items[0]["first_lsn"],
            u64::try_from(index + 1).unwrap()
        );
    }
    drop(writer);
    actor
}

fn mint_response(cluster: &ObservationCluster) -> Value {
    let first = &cluster.observations[0];
    let second = cluster
        .observations
        .iter()
        .find(|candidate| candidate.source.conversation != first.source.conversation)
        .expect("cluster spanned a single conversation");
    let third = cluster
        .observations
        .iter()
        .find(|candidate| {
            candidate.source.lsn != first.source.lsn && candidate.source.lsn != second.source.lsn
        })
        .expect("cluster held fewer than three observations");
    let citations = [first, second, third]
        .into_iter()
        .map(|observation| {
            let text = std::str::from_utf8(&observation.source.content).unwrap();
            let start = text
                .find(DEFINITION)
                .expect("observation lost the sentence");
            json!({
                "lsn": observation.source.lsn,
                "byte_start": start,
                "byte_end": start + DEFINITION.len(),
                "quote": DEFINITION,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "action": "mint",
        "target": null,
        "name": "Kestrel token rotation",
        "definition": DEFINITION,
        "tags": ["kestrel", "release", "operations"],
        "salience_micros": 900_000,
        "citations": citations,
    })
}

fn recorded(request: &StructuredRequest, maximum_output_tokens: u32, body: Value) -> WireFixture {
    WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: ENDPOINT.to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": MODEL,
                "messages": [
                    {"role": "system", "content": request.system},
                    {"role": "user", "content": request.prompt}
                ],
                "max_tokens": maximum_output_tokens,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": request.prompt_id,
                        "strict": true,
                        "schema": request.json_schema
                    }
                }
            }),
        },
        response: WireResponse { status: 200, body },
    }
}

fn answered(choice: &Value) -> Value {
    json!({
        "choices": [choice],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 25,
            "prompt_tokens_details": {"cached_tokens": 0}
        }
    })
}

fn completed(output: &Value) -> Value {
    answered(&json!({"message": {"content": output.to_string()}}))
}

fn recorded_provider(fixtures: Vec<WireFixture>) -> OpenAiCompatible<RecordedTransport> {
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: ENDPOINT.to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: MODEL.to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

async fn truncated_then_repaired(
    actor: &ActorEngine,
) -> (
    Arc<OpenAiCompatible<RecordedTransport>>,
    ConsolidationRuntime,
) {
    let clusters = hm_mcp::tools::consolidate::nrem_clusters(actor)
        .await
        .unwrap();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].observations.len(), 6);
    let request = merge_request(&clusters[0], &[]).unwrap();
    assert_eq!(request.maximum_output_tokens, MERGE_OUTPUT_TOKENS);
    let provider = Arc::new(recorded_provider(vec![
        recorded(
            &request,
            MERGE_OUTPUT_TOKENS,
            answered(&json!({"finish_reason": "length", "message": {"content": "{\"action\":"}})),
        ),
        recorded(
            &request,
            MERGE_OUTPUT_TOKENS.saturating_mul(2),
            completed(&mint_response(&clusters[0])),
        ),
    ]));
    let runtime = ConsolidationRuntime::new(Arc::clone(&provider) as Arc<dyn hm_llm::LlmProvider>);
    (provider, runtime)
}

fn run_input(cadence_key: &str, budget: ConsolidateBudget) -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Nrem),
        scope: Some("actor".to_owned()),
        cadence_key: Some(cadence_key.to_owned()),
        budget: Some(budget),
        run_id: None,
        reason: None,
    }
}

async fn minted_memories(actor: &ActorEngine) -> usize {
    actor
        .frames_since(LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
        .iter()
        .filter(|frame| frame.header.kind == EventKind::MemoryMinted)
        .count()
}

#[tokio::test]
async fn a_truncated_consolidation_answer_is_repaired_inside_the_declared_budget() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = seeded_actor(temporary.path()).await;
    let (provider, runtime) = truncated_then_repaired(&actor).await;
    let server = McpServer::new(actor.clone()).with_consolidation_runtime(runtime);
    let envelope = server
        .consolidate_envelope(run_input(
            "kestrel-repair-cycle",
            ConsolidateBudget {
                max_llm_calls: 4,
                max_tokens: 100_000,
                max_microusd: 1_000_000,
                max_wall_ms: 600_000,
            },
        ))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items[0]["stats"]["derived_records"], 1);
    assert_eq!(envelope.items[0]["stats"]["dropped_candidates"], 0);
    let budget = envelope
        .budget
        .clone()
        .expect("fresh run reported no budget");
    assert_eq!(budget["version"], 1);
    assert_eq!(budget["max_llm_calls"], 4);
    assert_eq!(budget["llm_calls"], 2);
    assert_eq!(budget["max_tokens"], 100_000);
    assert_eq!(budget["tokens"], 250);
    assert_eq!(budget["max_microusd"], 1_000_000);
    assert_eq!(budget["microusd"], 300);
    assert_eq!(budget["retries"], 1);
    assert_eq!(minted_memories(&actor).await, 1);
    assert_eq!(provider.transport().remaining(), 0);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_retry_cannot_be_bought_outside_the_run_budget() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = seeded_actor(temporary.path()).await;
    let (provider, runtime) = truncated_then_repaired(&actor).await;
    let server = McpServer::new(actor.clone()).with_consolidation_runtime(runtime);
    let envelope = server
        .consolidate_envelope(run_input(
            "kestrel-single-call-cycle",
            ConsolidateBudget {
                max_llm_calls: 1,
                max_tokens: 100_000,
                max_microusd: 1_000_000,
                max_wall_ms: 600_000,
            },
        ))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items[0]["stats"]["derived_records"], 0);
    assert_eq!(envelope.items[0]["stats"]["dropped_candidates"], 1);
    let budget = envelope
        .budget
        .clone()
        .expect("fresh run reported no budget");
    assert_eq!(budget["version"], 1);
    assert_eq!(budget["max_llm_calls"], 1);
    assert_eq!(budget["llm_calls"], 1);
    assert_eq!(budget["tokens"], 125);
    assert_eq!(budget["microusd"], 150);
    assert_eq!(budget["retries"], 0);
    assert_eq!(minted_memories(&actor).await, 0);
    assert_eq!(provider.transport().remaining(), 1);

    drop(server);
    actor.shutdown().await.unwrap();
}
