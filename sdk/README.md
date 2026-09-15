# HyperMind SDKs

The Rust kernel is currently available through `hm-serve::embedded`. Open a
`HyperMind`, create a conversation `Session`, and call `remember`, `recall`, or
`activate` without running the daemon. Activation bundles can be passed to
`render`; rendered memory is always labelled `UntrustedMemory`, retains its
`hm://` provenance URI, and is never emitted with a system or developer role.

The TypeScript, Python, and Go packages are delivered in later dependency
waves after the local protocol and continuity slice are closed.
