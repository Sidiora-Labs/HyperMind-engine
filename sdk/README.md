![HyperMind](../spec/readme_img.png)

# HyperMind SDKs

The Rust kernel is currently available through `hm-serve::embedded`. Open a
`HyperMind`, create a conversation `Session`, and call `remember`, `recall`, or
`activate` without running the daemon. Activation bundles can be passed to
`render`; rendered memory is always labelled `UntrustedMemory`, retains its
`hm://` provenance URI, and is never emitted with a system or developer role.

TypeScript, Python, and Go source packages now live in this directory.
The TypeScript client supports the Unix socket and mutually authenticated gRPC;
its focused client suite passed. Python provides a native embedded engine and
an asynchronous gRPC client. Go provides Unix-socket and gRPC clients under the
existing module name `centra/core/cortexclient`.

The `hm-capi` crate publishes a C application binary interface over one
embedded actor, and `swift/` holds a source-only SwiftPM package that wraps
that header with an async/await surface. The Swift package is built and tested
only on Apple platforms; this repository's Linux gate proves binding fidelity
against the header, not a Swift test run. See
[C ABI and Swift](../docs/reference/sdks.md#c-abi-and-swift).

These are source-level interfaces, not published npm/PyPI packages, wheels, or
qualified cross-language releases. The Go donor compatibility limitation is
recorded in [COMPATIBILITY.md](go/COMPATIBILITY.md); passing current focused tests
does not erase that historical mismatch. See the
[SDK guide](../docs/reference/sdks.md) and
[source-derived API catalog](../docs/reference/generated/sdk-api.md) for actual
entry points. Use an embedded owner or a daemon client for an actor, never
concurrent owners of the same state directory.
