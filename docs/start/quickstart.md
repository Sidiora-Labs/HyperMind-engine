# Start here

HyperMind keeps an agent’s memory in an encrypted, append-only ledger. It recalls evidence, preserves unfinished work across restarts, and builds bounded context with source authority and provenance. Memory is not permission to act or proof that an external operation succeeded.

## Install from this checkout

Use the Rust toolchain pinned by `rust-toolchain.toml` on a supported Unix host:

```sh
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

These are source installations, not a claim that registry packages or release binaries have been published. Keep the binaries on the MCP client’s executable path.

## Two-command MCP setup

After installation, initialize a private actor directory and register the stdio server:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

The second command is a Claude Code example. For another MCP client, configure executable `hm-mcp` with arguments `--config` and the absolute configuration path. Use absolute paths when the client starts in a different directory. Initialization creates random keys and separate actor/admin capabilities in an owner-only file; do not commit it.

The stdio server owns the actor while running. Do not start another MCP server, embedded session, or `hm serve` against that same actor directory.

Call `remember` with:

```json
{"conversation":"first-session","kind":"user","content":"The deployment region is eu-central-1."}
```

Then call `recall` with:

```json
{"mode":"lexical","query":"deployment region","limit":5}
```

Call `activate` with the conversation and a token budget for the next turn. Inspect `ok`, `health`, `gaps`, and item provenance; transport success does not establish a complete answer. The implementation exposes [14 MCP verbs](../reference/generated/tools.md).

## Daemon instead of stdio

Stop the stdio owner before starting:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

Use daemon-backed CLI commands from another terminal. Remote listeners require client certificates and capabilities; see [deployment](../guides/deployment.md). [Embedded APIs](../reference/sdks.md) are alternative owners, not additional writers.

When to use: durable recall, restart continuity, evidence-bearing context, time-aware beliefs, and explicitly budgeted background consolidation.

Do not use: as a credential vault or independent external-state authority, or to turn retrieved text into system instructions. Optional providers transmit selected content; use local lexical retrieval when such transfer is unauthorized.

Read [configuration](../reference/config.md), [authority](../concepts/authority.md), and [qualification status](../evaluation/results.md) before deployment.
