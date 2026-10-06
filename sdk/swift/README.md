# HyperMind Swift package

A source-only SwiftPM package that wraps the C application binary interface
published in [hypermind.h](../../crates/hm-capi/include/hypermind.h). The
`CHyperMind` system-library target imports that header directly out of the
checkout and links `hypermind`; the `HyperMind` target adds an async/await
surface over it.

```swift
let credentials = HyperMindCredentials(
    actor: 7,
    userHex: userIdentityHex,
    kekHex: keyEncryptionKeyHex
)
let engine = try HyperMindEngine(credentials: credentials, stateDirectory: path)
let envelope = try await engine.recall(argumentsJSON: #"{"mode":"lexical","query":"region","limit":8}"#)
try engine.close()
```

`HyperMindEngine` exposes one thin method per verb the kernel already
dispatches. The package mirrors that inventory rather than widening it: there
is no verb in this package that the kernel does not expose, and every wrapper
delegates to `call(verb:argumentsJSON:)`.

## Ownership

An engine owns exactly one handle. `init` obtains it, `deinit` frees it exactly
once, and `close()` is a separate, idempotent shutdown that releases the actor
directory without freeing the handle. Strings handed to the C callback are
borrowed for the duration of that callback, so the result is copied into a
Swift `String` before the callback returns. The continuation crossing the
`void *` user-data pointer is retained once before the call is issued and
released exactly once: inside the callback when the call was accepted, and on
the calling path when it was refused. The boundary returns success only when
the callback will fire exactly once, which is what makes that pairing safe.

## Cancellation

Cancellation is cooperative at the call boundary. `call(verb:argumentsJSON:)`
checks for cancellation before issuing the call and again after it resolves.
The kernel round trip itself is not interruptible, so cancelling a task that is
already in flight does not stop work the actor has accepted; it only prevents
the result from being consumed. This limitation is deliberate rather than
hidden: interrupting an accepted mutation would make the ledger and the caller
disagree about what happened.

## Errors

`HyperMindError` carries the boundary `status` from the header, the kernel
error discriminant in `kernelCode` when that status is `HM_STATUS_KERNEL`, and
the stable kernel error name in `message`. For synchronous refusals the message
is the reason the boundary recorded on the calling thread.

## Typed context

`HyperMindContextClient` uses the existing engine handle and existing verbs.
Supply `contextScope` when opening the engine; the C boundary validates that
scope against the configured actor. A context client requires an explicit
scope, session, actor and context owner. Host-owned activation defers engine
reductions. Caller-provided required source IDs and UTC offsets remain explicit.

```swift
let scope = try ContextScope(ownerID: "owner", projectID: "project")
let engine = try HyperMindEngine(
    credentials: credentials, stateDirectory: path, contextScope: scope
)
let context = try HyperMindContextClient(
    engine: engine, scope: scope, sessionID: "session", actorID: 7,
    ownership: .hypermind
)
let source = try ContextSourceMessage(
    id: "reading", ordinal: 0, role: .user,
    parts: [.text("Temperature 20 C")], recordedAtNS: 1791288000123456789,
    authority: .userAsserted
)
let receipt = try await context.ingest(source, originalBytes: originalBytes)
let report = try await context.activate(
    query: "temperature",
    budget: ContextTokenBudget(contextTokens: 2048, reservedOutputTokens: 128)
)
let recovered = try await context.recover(sourceID: source.id)
```

Nanosecond fields use canonical signed decimal JSON strings. Source digests
bind the immutable typed source, while expansion returns the exact original
host bytes. Relations, forks, import bundles and receipts, historian and
maintenance jobs, notes and memory calls all reach the actual C ABI.
`ContextClientError` describes client validation, response identity and cursor
failures; `HyperMindError` retains boundary and kernel refusals. Cancellation
retains the engine's cooperative call-boundary behavior.

## Platform qualification

The package requires Swift 5.9 or newer and a matching native HyperMind library.
The source-contract gate `crates/hm-capi/tests/swift_binding.rs` checks C symbols
and the fourteen-verb inventory. Actual compilation and runtime tests require
a Swift toolchain and the native library; this textual gate does not establish
runtime qualification. Linux compilation is available with Swift 6.2.4.
Apple runtime and SDK qualification require separate runs on those platforms.

Build the native library and run the focused context suite with its library
directory on the linker and runtime search paths:

```sh
cargo build -p hm-capi
LD_LIBRARY_PATH="$PWD/target/debug" swift test --package-path sdk/swift \
  -Xlinker -L -Xlinker "$PWD/target/debug" --filter Context
```

On macOS use `DYLD_LIBRARY_PATH` for a dynamic library, or build a static
archive and provide its directory to the linker. The target path must match
`CARGO_TARGET_DIR` when that variable is configured.

See the [SDK guide](../../docs/reference/sdks.md) for how this package sits
beside the other language surfaces.
