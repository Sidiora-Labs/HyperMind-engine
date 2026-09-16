#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

#[path = "../src/harness.rs"]
#[allow(dead_code)]
mod harness;

use base64::Engine as _;
use harness::{HarnessResult, JourneyHarness};
use hm_core::LSN;
use hm_cortex::nrem::merge::merge_request;
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_mcp::{ConsolidationRuntime, McpServer};
use hm_schema::event::{self, Boundary};
use hm_schema::events::EventPayload;
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const CONVERSATIONS: [&str; 2] = ["slice6-orchid-a", "slice6-orchid-b"];
const DEFINITION: &str =
    "Orchid refresh tokens rotate after every use and rotation failures go to the ops log.";

#[tokio::main]
async fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [mode, config] if mode == "mcp" => harness::run_mcp_daemon(Path::new(config)).await,
        [mode, config] if mode == "mcp_fixture" => fixture_mcp(Path::new(config)).await,
        [command, directory, marker, mode] if command == "__generation_child" => {
            generation_child(Path::new(directory), Path::new(marker), mode).await
        }
        [] => journey().await,
        _ => Err("invalid slice6 journey helper invocation".into()),
    };
    if let Err(error) = result {
        eprintln!("slice6 journey failed: {error}");
        std::process::exit(1);
    }
}

async fn generation_child(
    directory: &Path,
    marker: &Path,
    mode: &std::ffi::OsStr,
) -> HarnessResult<()> {
    let mode = mode.to_str().ok_or("generation mode was not UTF-8")?;
    hm_eval::suites::generations::child(directory, marker, mode)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn journey() -> HarnessResult<()> {
    let temporary = tempfile::tempdir()?;
    let harness = JourneyHarness::create(temporary.path(), std::env::current_exe()?)?;
    let writer = harness.start_mcp().await?;
    for index in 0..20 {
        let content = format!("observation {index:02}. {DEFINITION}");
        let result = writer
            .call(
                "remember",
                json!({
                    "conversation": CONVERSATIONS[index % CONVERSATIONS.len()],
                    "content": content,
                    "kind": "user"
                }),
            )
            .await?;
        if result["items"][0]["first_lsn"] != u64::try_from(index + 1)? {
            return Err("observation ledger order was not deterministic".into());
        }
    }
    writer.kill().await?;

    let fixture = harness.start_mcp_mode("mcp_fixture").await?;
    let request = consolidate_args();
    let consolidated = fixture.call("consolidate", request.clone()).await?;
    let run_id = consolidated["items"][0]["run_id"]
        .as_str()
        .ok_or("consolidate omitted run id")?
        .to_owned();
    if consolidated["items"][0]["stats"]["derived_records"] != 1
        || consolidated["items"][0]["cost"]["llm_calls"] != 1
        || consolidated["items"][0]["duplicate"] != false
    {
        return Err(
            format!("fixture consolidation did not mint one memory: {consolidated}").into(),
        );
    }
    fixture.kill().await?;

    verify_cited_memory(harness.config_path(), &run_id).await?;

    let reader = harness.start_mcp().await?;
    let activated = activate(&reader).await?;
    if !contains_memory(&activated)? {
        return Err("published memory was absent from the fused tier after restart".into());
    }
    let retracted = reader
        .call(
            "consolidate",
            json!({
                "action": "retract",
                "run_id": run_id,
                "reason": "slice6 rollback verification"
            }),
        )
        .await?;
    if retracted["items"][0]["active_generation"] != 0 {
        return Err("retraction did not switch directly to the parent generation".into());
    }
    if contains_memory(&activate(&reader).await?)? {
        return Err("retracted memory remained visible on the next activation".into());
    }
    let events_before_replay = reader.call("inspect", json!({})).await?["items"][0]["log_events"]
        .as_u64()
        .ok_or("inspect omitted ledger event count")?;
    let replay = reader.call("consolidate", request).await?;
    if replay["items"][0]["run_id"] != run_id
        || replay["items"][0]["duplicate"] != true
        || replay["items"][0]["status"] != "retracted"
        || replay["items"][0]["cost"] != consolidated["items"][0]["cost"]
        || replay["items"][0]["stats"] != consolidated["items"][0]["stats"]
    {
        return Err(format!("idempotent rerun changed the recorded result: {replay}").into());
    }
    let events_after_replay = reader.call("inspect", json!({})).await?["items"][0]["log_events"]
        .as_u64()
        .ok_or("inspect omitted replay event count")?;
    if events_after_replay != events_before_replay {
        return Err("idempotent rerun appended duplicate ledger events".into());
    }
    reader.close().await?;

    let gate = hm_eval::slice6::run()
        .await
        .map_err(|error| error.to_string())?;
    if gate
        .judge_free
        .iter()
        .find(|metric| metric.name == "generation_rollback_override")
        .is_none_or(|metric| metric.value != 1.0)
    {
        return Err("slice6 rollback eval gate was not green".into());
    }
    Ok(())
}

fn consolidate_args() -> Value {
    json!({
        "action": "run",
        "mode": "nrem",
        "scope": "actor",
        "cadence_key": "slice6-fixture-cycle",
        "budget": {
            "max_llm_calls": 1,
            "max_tokens": 4096,
            "max_microusd": 1000,
            "max_wall_ms": 30000
        }
    })
}

async fn activate(client: &harness::McpClient) -> HarnessResult<Value> {
    client
        .call(
            "activate",
            json!({
                "conversation": "slice6-recall",
                "query": "Orchid rotation",
                "turn_text": "",
                "budget_tokens": 8192
            }),
        )
        .await
}

fn contains_memory(envelope: &Value) -> HarnessResult<bool> {
    for item in envelope["items"]
        .as_array()
        .ok_or("activation items were not an array")?
    {
        if item["tier"] != "fused" {
            continue;
        }
        let Some(encoded) = item["content_base64"].as_str() else {
            continue;
        };
        let content = base64::engine::general_purpose::STANDARD.decode(encoded)?;
        if content == DEFINITION.as_bytes() {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn fixture_mcp(config_path: &Path) -> HarnessResult<()> {
    let actor = open_actor(config_path).await?;
    let clusters = hm_mcp::tools::consolidate::nrem_clusters(&actor).await?;
    if clusters.len() != 1 || clusters[0].observations.len() != 20 {
        return Err("fixture observations did not form one complete NREM cluster".into());
    }
    let request = merge_request(&clusters[0], &[])
        .map_err(|reason| format!("could not build merge fixture request: {reason:?}"))?;
    let response = fixture_response(&clusters[0])?;
    let endpoint = "https://fixture.invalid/v1/chat/completions";
    let model = "fixture-merge-model";
    let wire = WireRequest {
        method: "POST".to_owned(),
        url: endpoint.to_owned(),
        headers: BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
        body: json!({
            "model": model,
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
    };
    let transport = RecordedTransport::new(vec![WireFixture {
        request: wire,
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": response.to_string()}}],
                "usage": {"prompt_tokens": 120, "completion_tokens": 30}
            }),
        },
    }]);
    let provider = OpenAiCompatible::new(
        ProviderConfig {
            endpoint: endpoint.to_owned(),
            api_key: None,
            model: model.to_owned(),
            tier: ModelTier::Standard,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 1_000_000,
            },
        },
        transport,
    )
    .map_err(|error| format!("fixture provider configuration failed: {error:?}"))?;
    let runtime = ConsolidationRuntime::new(Arc::new(provider));
    hm_mcp::serve_stdio(McpServer::new(actor).with_consolidation_runtime(runtime))
        .await
        .map_err(|error| error.to_string().into())
}

fn fixture_response(
    cluster: &hm_cortex::nrem::cluster::ObservationCluster,
) -> HarnessResult<Value> {
    let first = &cluster.observations[0];
    let second = cluster
        .observations
        .iter()
        .find(|candidate| candidate.source.conversation != first.source.conversation)
        .ok_or("cluster did not span two conversations")?;
    let third = cluster
        .observations
        .iter()
        .find(|candidate| {
            candidate.source.lsn != first.source.lsn && candidate.source.lsn != second.source.lsn
        })
        .ok_or("cluster did not contain three independent observations")?;
    let citations = [first, second, third]
        .into_iter()
        .map(|observation| {
            let text = std::str::from_utf8(&observation.source.content)?;
            let start = text
                .find(DEFINITION)
                .ok_or("fixture definition was absent")?;
            Ok(json!({
                "lsn": observation.source.lsn,
                "byte_start": start,
                "byte_end": start + DEFINITION.len(),
                "quote": DEFINITION
            }))
        })
        .collect::<HarnessResult<Vec<_>>>()?;
    Ok(json!({
        "action": "mint",
        "target": null,
        "name": "Orchid token rotation",
        "definition": DEFINITION,
        "tags": ["orchid", "authentication", "operations"],
        "salience_micros": 900000,
        "citations": citations
    }))
}

async fn verify_cited_memory(config_path: &Path, expected_run_id: &str) -> HarnessResult<()> {
    let actor = open_actor(config_path).await?;
    let frames = actor.frames_since(LSN::new(0), None, usize::MAX).await?;
    let memory_frame = frames
        .iter()
        .find(|frame| frame.header.kind == EventKind::MemoryMinted)
        .ok_or("ledger omitted MemoryMinted")?;
    let verified = event::verify_event(
        &memory_frame.sealed_payload,
        event::EventKind::MemoryMinted,
        Boundary::Disk,
    )?;
    let EventPayload::MemoryMinted(memory) = verified.envelope.payload else {
        return Err("MemoryMinted decoded as a different payload".into());
    };
    if memory.definition != DEFINITION.as_bytes()
        || memory.citations.len() != 3
        || verified
            .envelope
            .model_provenance
            .as_ref()
            .is_none_or(|model| model.model_id != "fixture-merge-model")
        || verified.envelope.run_id.as_deref().map(hex).as_deref() != Some(expected_run_id)
    {
        return Err("minted memory lost its definition, run, model, or citations".into());
    }
    for citation in &memory.citations {
        let source = frames
            .iter()
            .find(|frame| frame.header.lsn.get() == citation.first_lsn)
            .ok_or("citation source LSN was absent")?;
        let source_kind = event::EventKind::try_from(source.header.kind as u8)
            .map_err(|()| "citation source kind was invalid")?;
        let source = event::verify_event(&source.sealed_payload, source_kind, Boundary::Disk)?;
        let EventPayload::UserMsg(message) = source.envelope.payload else {
            return Err("citation did not point to a user observation".into());
        };
        let start = usize::try_from(citation.byte_start)?;
        let end = usize::try_from(citation.byte_end)?;
        if message.content.get(start..end) != Some(DEFINITION.as_bytes()) {
            return Err("citation byte range did not reproduce the minted claim".into());
        }
    }
    actor.shutdown().await?;
    Ok(())
}

async fn open_actor(config_path: &Path) -> HarnessResult<ActorEngine> {
    let config = hm_serve::config::load(config_path)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    Ok(ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: hm_core::ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?)
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
