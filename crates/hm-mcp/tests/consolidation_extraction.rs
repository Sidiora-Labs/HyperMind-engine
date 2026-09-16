#![forbid(unsafe_code)]

use hm_core::{ActorId, LSN};
use hm_cortex::nrem::cluster::ObservationCluster;
use hm_cortex::nrem::merge::merge_request;
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_mcp::{
    ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, ConsolidationRuntime,
    McpServer, RememberInput, RememberKind,
};
use hm_schema::event::{self, Boundary};
use hm_schema::events::EventPayload;
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

const TOPICS: [&str; 4] = ["Orchid", "Beacon", "Lantern", "Quartz"];
const CONVERSATIONS: [&str; 3] = ["extraction-a", "extraction-b", "extraction-c"];
const NOTES_PER_TOPIC: usize = 5;
const ENDPOINT: &str = "https://fixture.invalid/v1/chat/completions";
const MODEL: &str = "fixture-merge-model";
const PARALLEL: &str = "HM_CONSOLIDATION_PARALLEL";
const PROVIDER: &str = "HM_CONSOLIDATION_PROVIDER";
const API_KEY: &str = "CENTRA_GATEWAY_API_KEY";
const CHILD: &str = "consolidation_parallelism_under_the_current_environment";

fn actor_config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 64 * 1024 * 1024,
    }
}

fn definition(topic: &str) -> String {
    format!(
        "{topic} rotation tokens refresh after every use and rotation failures reach the ops log."
    )
}

async fn seed(server: &McpServer, topics: &[&str]) {
    let mut index = 0_usize;
    for topic in topics {
        for note in 0..NOTES_PER_TOPIC {
            let envelope = server
                .remember_envelope(RememberInput {
                    conversation: CONVERSATIONS[index % CONVERSATIONS.len()].to_owned(),
                    content: format!("{topic} note {note:02}. {}", definition(topic)),
                    kind: RememberKind::User,
                    chunk_bytes: None,
                    anchor: None,
                    retention: None,
                    sensitivity: None,
                    vocabulary: None,
                    source: None,
                    derive: None,
                })
                .await;
            assert!(envelope.ok, "{envelope:?}");
            index += 1;
        }
    }
}

fn run_input(cadence_key: &str) -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Nrem),
        scope: Some("consolidation-extraction".to_owned()),
        cadence_key: Some(cadence_key.to_owned()),
        budget: Some(ConsolidateBudget {
            max_llm_calls: 16,
            max_tokens: 16_000,
            max_microusd: 50_000,
            max_wall_ms: 30_000,
        }),
        run_id: None,
        reason: None,
    }
}

fn topic_of(cluster: &ObservationCluster) -> String {
    let content = std::str::from_utf8(&cluster.observations[0].source.content).unwrap();
    content.split_whitespace().next().unwrap().to_owned()
}

fn minted_name(cluster: &ObservationCluster, index: usize) -> String {
    format!("{} memory {index}", topic_of(cluster))
}

fn mint_response(cluster: &ObservationCluster, index: usize) -> Value {
    let definition = definition(&topic_of(cluster));
    let first = &cluster.observations[0];
    let second = cluster
        .observations
        .iter()
        .find(|candidate| candidate.source.conversation != first.source.conversation)
        .unwrap();
    let third = cluster
        .observations
        .iter()
        .find(|candidate| {
            candidate.source.lsn != first.source.lsn && candidate.source.lsn != second.source.lsn
        })
        .unwrap();
    let citations = [first, second, third]
        .into_iter()
        .map(|observation| {
            let content = std::str::from_utf8(&observation.source.content).unwrap();
            let start = content.find(definition.as_str()).unwrap();
            json!({
                "lsn": observation.source.lsn,
                "byte_start": start,
                "byte_end": start + definition.len(),
                "quote": definition
            })
        })
        .collect::<Vec<_>>();
    json!({
        "action": "mint",
        "target": null,
        "name": minted_name(cluster, index),
        "definition": definition,
        "tags": ["rotation", "operations"],
        "salience_micros": 900_000,
        "citations": citations
    })
}

fn fixture(cluster: &ObservationCluster, index: usize) -> WireFixture {
    let request = merge_request(cluster, &[]).unwrap();
    WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: ENDPOINT.to_owned(),
            headers: BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
            body: json!({
                "model": MODEL,
                "messages": [
                    {"role": "system", "content": request.system},
                    {"role": "user", "content": request.prompt}
                ],
                "max_tokens": request.maximum_output_tokens,
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
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": mint_response(cluster, index).to_string()}}],
                "usage": {"prompt_tokens": 120, "completion_tokens": 30}
            }),
        },
    }
}

fn fixtures(clusters: &[ObservationCluster]) -> Vec<WireFixture> {
    clusters
        .iter()
        .enumerate()
        .map(|(index, cluster)| fixture(cluster, index))
        .collect()
}

fn provider(fixtures: Vec<WireFixture>) -> Arc<OpenAiCompatible<RecordedTransport>> {
    Arc::new(
        OpenAiCompatible::new(
            ProviderConfig {
                endpoint: ENDPOINT.to_owned(),
                api_key: None,
                model: MODEL.to_owned(),
                tier: ModelTier::Standard,
                pricing: Pricing {
                    input_microusd_per_million_tokens: 1_000_000,
                    output_microusd_per_million_tokens: 1_000_000,
                },
            },
            RecordedTransport::new(fixtures),
        )
        .unwrap(),
    )
}

async fn minted_names(actor: &ActorEngine) -> Vec<String> {
    let mut names = Vec::new();
    for frame in actor
        .frames_since(LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
    {
        if frame.header.kind != EventKind::MemoryMinted {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::MemoryMinted,
            Boundary::Disk,
        )
        .unwrap();
        let EventPayload::MemoryMinted(memory) = verified.envelope.payload else {
            panic!("MemoryMinted decoded as a different payload");
        };
        names.push(memory.name);
    }
    names
}

#[tokio::test]
async fn nrem_extraction_records_every_candidate_in_source_order() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    seed(&server, &TOPICS).await;
    let clusters = hm_mcp::tools::consolidate::nrem_clusters(&actor)
        .await
        .unwrap();
    assert_eq!(clusters.len(), TOPICS.len());
    let recorded = provider(fixtures(&clusters));
    let server = server.with_consolidation_runtime(
        ConsolidationRuntime::new(recorded.clone()).with_parallelism(1),
    );
    let envelope = server.consolidate_envelope(run_input("night-serial")).await;

    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(
        envelope.items[0]["extraction"],
        json!({"extracted": clusters.len(), "failed": 0, "skipped": 0, "aborted": 0})
    );
    assert_eq!(
        envelope.items[0]["stats"]["derived_records"],
        json!(clusters.len())
    );
    assert_eq!(
        envelope.items[0]["cost"]["llm_calls"],
        json!(clusters.len())
    );
    assert_eq!(recorded.transport().remaining(), 0);
    let expected = clusters
        .iter()
        .enumerate()
        .map(|(index, cluster)| minted_name(cluster, index))
        .collect::<Vec<_>>();
    assert_eq!(minted_names(&actor).await, expected);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_failing_candidate_stops_dispatch_and_the_run_still_reports() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    seed(&server, &TOPICS).await;
    let clusters = hm_mcp::tools::consolidate::nrem_clusters(&actor)
        .await
        .unwrap();
    assert_eq!(clusters.len(), TOPICS.len());
    let mut recorded_fixtures = fixtures(&clusters);
    recorded_fixtures.remove(0);
    let recorded = provider(recorded_fixtures);
    let server = server.with_consolidation_runtime(
        ConsolidationRuntime::new(recorded.clone()).with_parallelism(1),
    );
    let envelope = server
        .consolidate_envelope(run_input("night-refused"))
        .await;

    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items[0]["extraction"]["failed"], json!(1));
    assert_eq!(
        envelope.items[0]["extraction"]["skipped"],
        json!(clusters.len() - 1)
    );
    assert_eq!(envelope.items[0]["extraction"]["extracted"], json!(0));
    assert_eq!(envelope.items[0]["extraction"]["aborted"], json!(0));
    assert_eq!(envelope.items[0]["stats"]["derived_records"], json!(0));
    assert!(
        envelope.items[0]["stats"]["dropped_candidates"]
            .as_u64()
            .unwrap()
            >= u64::try_from(clusters.len() - 1).unwrap()
    );
    assert!(minted_names(&actor).await.is_empty());

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn parallel_extraction_does_not_change_the_outcome() {
    let serial = single_cluster_run(1).await;
    let parallel = single_cluster_run(4).await;
    assert_eq!(serial.0, parallel.0);
    assert_eq!(serial.1, parallel.1);
    assert_eq!(
        serial.0,
        json!({"extracted": 1, "failed": 0, "skipped": 0, "aborted": 0})
    );
    assert_eq!(serial.1["derived_records"], json!(1));
}

async fn single_cluster_run(maximum_parallel: usize) -> (Value, Value) {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    seed(&server, &TOPICS[..1]).await;
    let clusters = hm_mcp::tools::consolidate::nrem_clusters(&actor)
        .await
        .unwrap();
    assert_eq!(clusters.len(), 1);
    let recorded = provider(fixtures(&clusters));
    let runtime = ConsolidationRuntime::new(recorded.clone()).with_parallelism(maximum_parallel);
    assert_eq!(runtime.maximum_parallel(), maximum_parallel);
    let server = server.with_consolidation_runtime(runtime);
    let envelope = server.consolidate_envelope(run_input("night-single")).await;

    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(recorded.transport().remaining(), 0);
    let outcome = (
        envelope.items[0]["extraction"].clone(),
        envelope.items[0]["stats"].clone(),
    );
    drop(server);
    actor.shutdown().await.unwrap();
    outcome
}

#[test]
fn the_parallel_ceiling_comes_from_the_environment_or_the_default() {
    assert_child_parallelism(&[], 4);
    assert_child_parallelism(&[(PARALLEL, "1")], 1);
    assert_child_parallelism(&[(PARALLEL, "8")], 8);
    assert_child_parallelism(&[(PARALLEL, "0")], 4);
    assert_child_parallelism(&[(PARALLEL, "")], 4);
    assert_child_parallelism(&[(PARALLEL, "many")], 4);
    assert_child_parallelism(&[(PARALLEL, "512")], 64);
}

#[test]
#[ignore = "re-executed with a prepared environment by the_parallel_ceiling_comes_from_the_environment_or_the_default"]
fn consolidation_parallelism_under_the_current_environment() {
    let runtime = ConsolidationRuntime::from_env().unwrap().unwrap();
    println!("consolidation-parallelism {}", runtime.maximum_parallel());
}

fn assert_child_parallelism(variables: &[(&str, &str)], expected: usize) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "--ignored", "--nocapture", CHILD])
        .env_remove(PARALLEL)
        .env(PROVIDER, "centra")
        .env(API_KEY, "fixture-key");
    for (name, value) in variables {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reported = String::from_utf8(output.stdout).unwrap();
    let wanted = format!("consolidation-parallelism {expected}");
    assert!(
        reported.lines().any(|line| line == wanted),
        "wanted {wanted}, got {reported}"
    );
}
