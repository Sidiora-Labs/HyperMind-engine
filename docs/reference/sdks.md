# SDKs

When to use: embed the kernel in one owning process or connect to the authenticated daemon without recreating memory semantics.

Do not use: competing embedded owners, direct storage writes, unsafe renderers, or source availability as a claim of package publication/qualification.

## Rust

`hm_serve::embedded::{HyperMind, EmbeddedConfig, Session}` provides the embedded path. Open `HyperMind`, create a conversation session, then call its actual `remember`, `recall`, and `activate` methods. `render` retains provenance and untrusted-memory labeling. The actor API exposes broader kernel contracts; do not invent missing Session methods.

```rust,no_run
use hm_core::ActorId;
use hm_serve::embedded::{EmbeddedConfig, HyperMind, MemoryKind, RenderModel, render};
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let memory = HyperMind::open(".hypermind/data", EmbeddedConfig {
    actor: ActorId::new(1), user: [1; 16], kek: [2; 32],
    projection_map_bytes: 256 * 1024 * 1024,
}).await?;
let session = memory.session("example");
session.remember(MemoryKind::User, "The region is eu-central-1").await?;
let bundle = session.activate("region", 2048).await?;
let context = render(&bundle, RenderModel::PlainText)?;
# let _ = context;
# let actor = memory.actor();
# drop(session);
# drop(memory);
# actor.shutdown().await?;
# Ok(())
# }
```

Repeated sample keys are documentation-only. Production must use securely provisioned identities/keys, never these example bytes.

## TypeScript

The workspace contains `engine` (native embedded), `client` (protocol and sequence recovery), `render` (safe rendering), and `migrate`. Use checked-in package scripts under `sdk/typescript`; generated wire bindings are not hand-edited. Prebuild publication is a separate release activity.

## Python and Go

Reach-wave interfaces are being implemented and qualified separately; no wheels, registry publication, or cross-language release journey is claimed here. The [generated API catalog](generated/sdk-api.md) discovers authored declarations as sources are added. Use confirmed package instructions rather than inferred installation commands.

All SDKs must preserve envelope/error semantics, sequence recovery, authority, and safe rendering. Shared schema alone does not establish parity.
