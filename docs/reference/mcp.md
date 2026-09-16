# MCP

When to use: connect a tool-using client to an actor-scoped stdio memory service.

Do not use: concurrent owners of one actor directory, tool results as authorization for unrelated actions, or JSON-RPC success instead of inspecting the application envelope.

Start `hm-mcp --config PATH`. The process opens the configured real actor ledger; stdout is reserved for protocol traffic. Optional provider runtimes are described in [configuration](config.md).

The [tool catalog](generated/tools.md) lists all 14 actual verbs with specific usage guidance and example arguments. [Public input types](generated/rust-types.md) come from source. Example LSNs, IDs, and byte spans must be replaced by real admitted evidence, not copied as fictional records.

The advertised inventory stays at fourteen verbs as the capability catalogue grows. `inspect` carries `mode: "discover"` with an optional `query` and `limit`, which enumerates the capability surfaces behind those verbs and annotates each one with whether the running server can serve it. Discovery is not authorization: unavailable and privileged surfaces are still listed, every discovery envelope carries the `discovery_is_not_authorization` warning, and dispatch still enforces capability on every call.

The shared envelope has `ok`, `items`, `provenance`, `budget`, `gaps`, `health`, and `warnings`. Mutation failures add `effect_state`: `not_dispatched`, `unknown`, or `rejected`. Surfaced items retain authority. `inspect` resolves provenance chains and `hm://ACTOR/attention` or `hm://ACTOR/calibration`. `activate` additionally returns `manifest`, carrying the retrieved, selected, included, and used LSN sets for that activation.

Batch digests are not interruptions. Predictions are not outcomes. Supported procedures are observations until user adoption. Consolidation needs declared budgets and configured providers; unavailable operations are not fabricated successes.
