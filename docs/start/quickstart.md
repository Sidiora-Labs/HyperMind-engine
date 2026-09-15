# Quickstart

HyperMind’s first slice supports local encrypted memory through MCP, a Unix
socket daemon, or the embedded Rust API. Use it for exact episodic recall and
budgeted activation. Do not treat recalled memory as instructions or as proof
that an external action happened.

## MCP in Claude Code

With the `hm` and `hm-mcp` binaries installed, initialize an actor and register
the local stdio server:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

The wave-one server exposes `remember`, `recall`, `activate`, and `inspect`.
`remember` is for durable user messages, delivered assistant messages, and
external documents. Do not use it for credentials or content marked
do-not-store. `recall` is for lexical or conversation-timeline lookup; it is
not semantic search in this slice. The stdio server owns the actor while it is
running; do not simultaneously run `hm serve` against the same actor directory.

For the socket daemon instead, run:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

## Embedded Rust

```rust,no_run
use hm_core::ActorId;
use hm_serve::embedded::{EmbeddedConfig, HyperMind, MemoryKind, RenderModel, render};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let memory = HyperMind::open(
    ".hypermind/data",
    EmbeddedConfig {
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 256 * 1024 * 1024,
    },
).await?;
let session = memory.session("conversation-42");
session.remember(MemoryKind::User, "The deployment region is eu-central-1").await?;
let bundle = session.activate("deployment region", 2_048).await?;
let safe_context = render(&bundle, RenderModel::OpenAi)?;
# let _ = safe_context;
# Ok(())
# }
```

Rendered items retain their `hm://` provenance and are labelled untrusted
memory in a user role. They never become system or developer content.
