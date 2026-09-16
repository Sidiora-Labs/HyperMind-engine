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

The workspace contains `engine` (native embedded), `client` (protocol and sequence recovery), `render` (safe rendering), `migrate`, and `console`. `@hypermind/console` builds the read-only inspection views — overview, activity, sources and entities, evidence path, access, removal preview, domain profile and upload sessions — from daemon envelopes it obtains through `@hypermind/client` in Node or `fetch` in a browser; it is described in the [console guide](../guides/console.md). Use checked-in package scripts under `sdk/typescript`; generated wire bindings are not hand-edited. Prebuild publication is a separate release activity.

## Python and Go

Reach-wave interfaces are being implemented and qualified separately; no wheels, registry publication, or cross-language release journey is claimed here. The [generated API catalog](generated/sdk-api.md) discovers authored declarations as sources are added. Use confirmed package instructions rather than inferred installation commands.

## C ABI and Swift

When to use: embed the kernel from a language that speaks C, or from Swift on an Apple platform, without running the daemon or reimplementing memory semantics.

Do not use: a second owner of an actor directory, a handle freed while a call is in flight, a borrowed callback string retained after the callback returns, or a returned envelope treated as proof of an external effect.

`crates/hm-capi` publishes [hypermind.h](../../crates/hm-capi/include/hypermind.h). A handle is opened from a strict JSON configuration object, may be cloned by reference count, and is freed exactly once per handle; shutting the actor down is the separate, idempotent close step. One call entry point forwards a verb and its JSON arguments to the same dispatcher the MCP transport uses and hands the serialized envelope back through a callback; the boundary reports success only when that callback will fire exactly once, and the callback's strings are borrowed for its duration. Boundary faults travel as a status enumerator, while a kernel refusal carries the numeric error discriminant and its stable name.

The [Swift package](../../sdk/swift/README.md) wraps that header through a system-library target and bridges each call into `withCheckedThrowingContinuation`, mirroring the existing verb inventory rather than widening it. The Swift package is compiled and tested only on Apple platforms; no Swift toolchain exists in this repository's Linux CI, so the checked-in Linux gate proves only that the Swift sources bind symbols and status values that the C header actually declares. No Swift build or test run is claimed here.

All SDKs must preserve envelope/error semantics, sequence recovery, authority, and safe rendering. Shared schema alone does not establish parity.
