# Migration from cortex-engine

When to use: import a compatible cortex-engine SQLite or JSON store into a separate HyperMind actor while preserving source identity and evidence distinctions.

Do not use: a live destination writer, observed authority for dream-generated text, source deletion before verification, or count-only checks as proof of embedding/edge equivalence.

```sh
hm import --config .hypermind/hypermind.conf --input /path/to/cortex-store.json --json
```

The importer owns the destination actor directly; stop its daemon first. Preserve a recoverable input/key copy and use a separate destination initially. Validation follows the current importer’s declared formats, not arbitrary JSON assumptions. The source is read-only.

Observations retain user-asserted authority; dream-minted memories remain derived inference. IDs, relationships, and compatible embedding identity must be preserved or explicitly reported unsupported. Review actual counts, mappings, and verification output. A partial/failed import is not complete. Query representative observations and derived memories against the retained source before switching clients.

The [type catalog](../reference/generated/rust-types.md) contains current migration declarations; the [CLI catalog](../reference/generated/cli-declarations.md) contains its parser. Import qualification and the final release journey are separate requirements.
