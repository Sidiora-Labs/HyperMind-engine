# Donor compatibility

The sequence-recovery implementation and original test vectors come from
`neocortex-engine/cortexclient`. Unmodified test sources are retained in
`testdata/donor/*.original`; executable copies use the current generated protocol
and `hm serve` process configuration. The module and package names remain
`centra/core/cortexclient` and `cortexclient` for source compatibility.

`TestLoopSeamContractAgainstRealCortexd` is an outstanding historical compatibility
gate: it expects a derived consolidation to install a protected constraint as
resident memory. HyperMind requires direct user authority for protected
constraints. The client deliberately preserves `derived_inference` authority,
and does not make that assertion pass by promoting evidence or weakening the
test. `TestCurrentProtectedConstraintPolicy` exercises the current safe behavior.

Legacy LSN-only consolidation citations are resolved against actual transcript
records into byte ranges before submission. The legacy `all` rebuild spelling
maps to the daemon's `bm25` route, which rebuilds the complete projection stream
and returns its applied checkpoint. gRPC uses the same recovery machine as UDS;
unsequenced mutating tool calls are never automatically retried.
