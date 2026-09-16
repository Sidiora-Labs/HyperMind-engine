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

## Apple-platform gate

The Swift package is compiled and tested only on Apple platforms; no Swift
toolchain exists in this repository's Linux CI, so the checked-in Linux gate
proves only that the Swift sources bind symbols and status values that the C
header actually declares. No Swift build or test run is claimed anywhere in
this repository, and the XCTest suite under `Tests/HyperMindTests` is never
executed here.

The checked-in gate is `crates/hm-capi/tests/swift_binding.rs`. It reads the
module map, the Swift sources and the header as text and fails if the Swift
code names a symbol, a status enumerator or a verb that does not exist.

To build and run the suite yourself on macOS, produce the static archive and
then run the tests:

```sh
cargo rustc -p hm-capi --crate-type staticlib --release
swift test
```

The archive lands under the workspace target directory; point the linker at it
with `-L` when `swift test` cannot find `libhypermind.a` on its default search
path.

See the [SDK guide](../../docs/reference/sdks.md) for how this package sits
beside the other language surfaces.
