#![forbid(unsafe_code)]

use hm_core::ActorId;
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::load;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--config")) {
        return Err("usage: hm-mcp --config PATH".into());
    }
    let path = arguments.next().ok_or("usage: hm-mcp --config PATH")?;
    if arguments.next().is_some() {
        return Err("usage: hm-mcp --config PATH".into());
    }
    let config = load(path)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?;
    hm_mcp::serve_stdio(hm_mcp::McpServer::new_with_admin(actor, config.admin_token)).await
}
