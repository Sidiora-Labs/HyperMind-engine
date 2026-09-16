# MCP

When to use: connect a tool-using client to an actor-scoped stdio memory service.

Do not use: concurrent owners of one actor directory, tool results as authorization for unrelated actions, or JSON-RPC success instead of inspecting the application envelope.

Start `hm-mcp --config PATH`. The process opens the configured real actor ledger; stdout is reserved for protocol traffic. Optional provider runtimes are described in [configuration](config.md).

The [tool catalog](generated/tools.md) lists all 14 actual verbs with specific usage guidance and example arguments. [Public input types](generated/rust-types.md) come from source. Example LSNs, IDs, and byte spans must be replaced by real admitted evidence, not copied as fictional records.

The shared envelope has `ok`, `items`, `provenance`, `budget`, `gaps`, `health`, and `warnings`. Mutation failures add `effect_state`: `not_dispatched`, `unknown`, or `rejected`. Surfaced items retain authority. `inspect` resolves provenance chains and `hm://ACTOR/attention` or `hm://ACTOR/calibration`.

Batch digests are not interruptions. Predictions are not outcomes. Supported procedures are observations until user adoption. Consolidation needs declared budgets and configured providers; unavailable operations are not fabricated successes.
