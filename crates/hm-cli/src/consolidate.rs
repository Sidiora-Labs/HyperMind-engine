use anyhow::{Result, anyhow};
use clap::{Subcommand, ValueEnum};
use hm_core::{ActorId, ErrorCode};
use hm_mcp::{ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, McpServer};
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::{ServerConfig, load};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    Run {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, value_enum)]
        mode: Mode,
        #[arg(long, default_value = "actor")]
        scope: String,
        #[arg(long)]
        cadence_key: String,
        #[arg(long)]
        max_llm_calls: u64,
        #[arg(long)]
        max_tokens: u64,
        #[arg(long)]
        max_microusd: u64,
        #[arg(long)]
        max_wall_ms: u64,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    Retract {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Mode {
    Nrem,
    Rem,
    Both,
}

impl From<Mode> for ConsolidateMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Nrem => Self::Nrem,
            Mode::Rem => Self::Rem,
            Mode::Both => Self::Both,
        }
    }
}

pub async fn execute(command: Command) -> Result<Value> {
    let (config_path, input) = match command {
        Command::Run {
            config,
            mode,
            scope,
            cadence_key,
            max_llm_calls,
            max_tokens,
            max_microusd,
            max_wall_ms,
        } => (
            config,
            ConsolidateInput {
                action: ConsolidateAction::Run,
                mode: Some(mode.into()),
                scope: Some(scope),
                cadence_key: Some(cadence_key),
                budget: Some(ConsolidateBudget {
                    max_llm_calls,
                    max_tokens,
                    max_microusd,
                    max_wall_ms,
                }),
                run_id: None,
                reason: None,
            },
        ),
        Command::List { config } => (
            config,
            ConsolidateInput {
                action: ConsolidateAction::List,
                mode: None,
                scope: None,
                cadence_key: None,
                budget: None,
                run_id: None,
                reason: None,
            },
        ),
        Command::Retract {
            config,
            run_id,
            reason,
        } => (
            config,
            ConsolidateInput {
                action: ConsolidateAction::Retract,
                mode: None,
                scope: None,
                cadence_key: None,
                budget: None,
                run_id: Some(run_id),
                reason: Some(reason),
            },
        ),
    };
    let config = load(&config_path)?;
    run(&config, input).await
}

async fn run(config: &ServerConfig, input: ConsolidateInput) -> Result<Value> {
    let capability = config
        .actors
        .first()
        .ok_or_else(|| anyhow!(ErrorCode::InvalidArgument))?;
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?;
    let envelope = McpServer::new(actor.clone())
        .consolidate_envelope(input)
        .await;
    actor.shutdown().await?;
    if !envelope.ok {
        return Err(anyhow!(
            envelope
                .items
                .first()
                .and_then(|item| item.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("consolidation failed")
                .to_owned()
        ));
    }
    Ok(serde_json::to_value(envelope)?)
}
