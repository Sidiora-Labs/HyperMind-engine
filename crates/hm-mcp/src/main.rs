#![forbid(unsafe_code)]

use hm_core::ActorId;
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::load;
use std::{ffi::OsStr, path::PathBuf};

const USAGE: &str = "usage: hm-mcp --config PATH [--context-scope PATH]";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let mut config_path = None;
    let mut context_scope_path = None;
    while let Some(argument) = arguments.next() {
        match argument.as_os_str() {
            flag if flag == OsStr::new("--config") && config_path.is_none() => {
                config_path = Some(PathBuf::from(arguments.next().ok_or(USAGE)?));
            }
            flag if flag == OsStr::new("--context-scope") && context_scope_path.is_none() => {
                context_scope_path = Some(PathBuf::from(arguments.next().ok_or(USAGE)?));
            }
            flag if (flag == OsStr::new("--help") || flag == OsStr::new("-h"))
                && config_path.is_none()
                && context_scope_path.is_none() =>
            {
                println!("{USAGE}\n\n--config PATH         Private server configuration.\n--context-scope PATH  Owner-only version-1 JSON actor/scope mapping for context operations.");
                return Ok(());
            }
            _ => return Err(USAGE.into()),
        }
    }
    let config = load(config_path.ok_or(USAGE)?)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    let context = context_scope_path
        .map(|path| hm_serve::context_config::load(path, &config, capability.actor))
        .transpose()?;
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?;
    let mut server = hm_mcp::McpServer::configured(actor, Some(config.admin_token)).await?;
    if let Some(context) = context {
        server = server.with_context_scope(context.scope);
    }
    hm_mcp::serve_stdio(server).await
}
