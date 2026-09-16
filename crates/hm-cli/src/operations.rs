use anyhow::{Context, Result, ensure};
use base64::Engine as _;
use hm_ledger::keyring::{KeyHierarchy, OsEntropy};
use hm_schema::wire::{RequestPayload, ResponsePayload, Stats, ToolRequest};
use serde_json::{Value, json};
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub async fn doctor(path: &Path, models_directory: Option<&Path>) -> Result<Value> {
    let config = hm_serve::config::load(path)?;
    let socket_ready = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::UnixStream::connect(&config.socket_path),
    )
    .await
    .is_ok_and(|value| value.is_ok());
    let mut actors = Vec::new();
    let mut healthy = socket_ready;
    for capability in &config.actors {
        let directory = config.actor_directory(capability.actor);
        let keys = match KeyHierarchy::open_or_create(
            &directory,
            hm_core::ActorId::new(capability.actor),
            config.user,
            &config.kek,
            &mut OsEntropy,
            false,
        ) {
            Ok(_) => "verified",
            Err(_) if directory.join("keys/DELETION_RECEIPT").exists() => "shredded",
            Err(_) => "unavailable_or_invalid",
        };
        let index = if socket_ready {
            match tokio::time::timeout(Duration::from_secs(5), async {
                let mut client = crate::DaemonClient::connect_admin(&config).await?;
                client
                    .request(RequestPayload::Stats(Box::new(Stats {
                        actor: capability.actor,
                    })))
                    .await
            })
            .await
            {
                Ok(Ok(ResponsePayload::StatsResult(stats))) => {
                    let current = stats
                        .projection_stats
                        .iter()
                        .all(|p| p.applied_lsn == stats.log_events);
                    healthy &= current;
                    json!({"status":if current {"current"} else {"lagging"},"log_events":stats.log_events,"projections":stats.projection_stats.iter().map(|p| json!({"name":p.name,"applied_lsn":p.applied_lsn})).collect::<Vec<_>>()})
                }
                _ => {
                    healthy = false;
                    json!({"status":"unavailable"})
                }
            }
        } else {
            json!({"status":"not_checked_daemon_offline","files_present":directory.join("projections/data.mdb").is_file()})
        };
        healthy &= keys == "verified";
        actors.push(json!({"actor":capability.actor,"path":directory,"present":directory.is_dir(),"keys":keys,"indexes":index}));
    }
    let default_models = path
        .parent()
        .context("configuration directory missing")?
        .join("models");
    let models = crate::models::status(models_directory.unwrap_or(&default_models))?;
    let toolchain = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    Ok(
        json!({"ok":true,"healthy":healthy,"toolchain":toolchain,"socket_ready":socket_ready,"actor_directories":actors,
        "missing_models":models["models"].as_array().unwrap().iter().filter(|model| model["present"] != true).map(|model| model["id"].clone()).collect::<Vec<_>>(),"model_health":models,
        "note":"ok means diagnostics completed; healthy covers daemon/key/index checks, models have separate verified status"}),
    )
}

pub async fn bench(path: &Path, query: &str, iterations: usize) -> Result<Value> {
    ensure!(
        (1..=10000).contains(&iterations),
        "iterations must be between 1 and 10000"
    );
    let config = hm_serve::config::load(path)?;
    let mut client = crate::DaemonClient::connect(&config, crate::first_actor(&config)?).await?;
    let mut samples = Vec::with_capacity(iterations);
    let mut hits = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let reply = client
            .request(RequestPayload::Recall(Box::new(hm_schema::wire::Recall {
                query: query.as_bytes().to_vec(),
                limit: 32,
                mode: hm_schema::wire::RecallMode::ListWindows,
                level: 1,
                start_ns: 0,
                end_ns: 0,
            })))
            .await?;
        let ResponsePayload::RecallResult(result) = reply else {
            anyhow::bail!("unexpected recall response")
        };
        samples.push(u64::try_from(start.elapsed().as_nanos())?);
        hits += result.members.unwrap_or_default().len();
    }
    samples.sort_unstable();
    Ok(
        json!({"ok":true,"benchmark":"daemon_lexical_recall","encoder":"lexical_only","iterations":iterations,"hits":hits,
        "p50_ns":samples[(iterations-1)/2],"p99_ns":samples[((iterations*99).div_ceil(100)-1).min(iterations-1)],"includes_transport":true,"quality_score":null,"paid_calls":0}),
    )
}

pub struct TuiOptions<'a> {
    pub config: &'a Path,
    pub conversation: &'a str,
    pub query: &'a str,
    pub budget_tokens: usize,
    pub interval_ms: u64,
    pub frames: Option<usize>,
    pub json: bool,
}

pub async fn tui(options: TuiOptions<'_>) -> Result<()> {
    ensure!(
        options.interval_ms >= 100 && options.frames != Some(0),
        "interval must be >=100ms and frames positive"
    );
    ensure!(
        options.json || std::io::stdout().is_terminal(),
        "non-terminal TUI output requires --json"
    );
    let config = hm_serve::config::load(options.config)?;
    let mut client = crate::DaemonClient::connect(&config, crate::first_actor(&config)?).await?;
    let mut interval = tokio::time::interval(Duration::from_millis(options.interval_ms));
    let mut number = 0;
    loop {
        tokio::select! { _ = tokio::signal::ctrl_c() => break, _ = interval.tick() => {} }
        let reply = client.request(RequestPayload::ToolRequest(Box::new(ToolRequest {
            verb:"activate".into(),arguments_json:serde_json::to_vec(&json!({"conversation":options.conversation,"query":options.query,"budget_tokens":options.budget_tokens}))?,
        }))).await?;
        let ResponsePayload::BytesResult(reply) = reply else {
            anyhow::bail!("unexpected activation response")
        };
        let mut frame: Value = serde_json::from_slice(&reply.bytes)?;
        ensure!(frame["ok"] == true, "live activation failed: {}", frame);
        if let Some(items) = frame["items"].as_array_mut() {
            for item in items {
                if let Some(encoded) = item["content_base64"].as_str() {
                    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
                    if let Ok(content) = String::from_utf8(bytes) {
                        item["content"] = Value::String(content);
                    }
                    item["trust"] = Value::String("untrusted_memory".into());
                }
            }
        }
        if options.json {
            println!("{}", serde_json::to_string(&frame)?);
        } else {
            print!(
                "\x1b[2J\x1b[HHyperMind live activation · Ctrl-C to stop\n{}\n",
                serde_json::to_string_pretty(&frame)?
            );
        }
        std::io::stdout().flush()?;
        number += 1;
        if options.frames.is_some_and(|frames| number >= frames) {
            break;
        }
    }
    Ok(())
}
