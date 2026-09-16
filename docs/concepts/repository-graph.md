# Repository graph

A repository graph is a read model HyperMind derives from a document the caller wrote. The caller describes one repository as a newline-delimited JSON snapshot, hands it to `remember`, and asks `consolidate` to turn it into nodes and relationships that `recall` can walk. Every step is deterministic, cited to bytes the ledger already holds, and published as a retractable generation.

## What the engine does not do

HyperMind never runs, downloads, installs or configures a program that produces a snapshot. There is no subprocess, no fetch, no extractor path, and no configuration key anywhere on this path. The caller produces the snapshot however it likes — a script, a build step, a language server, a person typing JSON — and the engine reads only the bytes it was given. A repository the engine was never handed a snapshot for has no graph, and asking for one refuses with `kOperationUnavailable` rather than going and looking.

## The snapshot document

A snapshot is UTF-8 text, one JSON object per line. The first non-empty line is the header and declares `contract`, which must be the literal string `hypermind.repository-graph.v1`, the `repository` name, and `fact_count`, the number of fact lines that follow. A header that declares any other contract, or a document whose fact lines do not match the declared count, refuses with `kSchemaInvalid`.

Every line after the header is one repository fact. A fact names a `kind`, which is one of `file`, `symbol`, `dependency`, `route`, `test` or `storage`, and a `name` unique within that kind. It may also carry `path`, `line`, `end_line`, a string map of `attributes`, and a list of `relations`, each naming a `relation` and a `target`. Relation names are exactly `contains`, `imports`, `calls`, `defines`, `routes_to`, `covers`, `reads` and `writes`. A fact of an unknown kind, a relation with an unknown name, and a target that resolves to no fact or to more than one are each counted and dropped; none of them is fatal. A snapshot carries at most 4096 facts and resolves to at most 8192 relationships.

Snapshots are ingested through `remember` with `kind: "repository_snapshot"` into the conversation `repository:<name>`. The content is split on line boundaries into shards, each stored as an externally observed provider frame, so a fact line is never cut in half and the byte range of every line stays addressable by LSN.

## Identity

Node and relationship identity are content-addressed, so the same fact in a later snapshot addresses the same record rather than creating a second one.

A node id is `blake3` over the domain string `hypermind.repository-node.v1`, a zero byte, and then the repository name, the kind string and the fact name, each prefixed with its little-endian length. A relationship id is `blake3` over the domain string `hypermind.repository-edge.v1`, a zero byte, the source node id, the target node id, and the length-prefixed relation name. A snapshot digest is `blake3` over the domain string `hypermind.repository-snapshot.v1`, a zero byte, and the shard contents concatenated in ascending LSN order.

Relationship weight is counted evidence, not a model's opinion: it is 250000 micros per distinct declaring fact, capped at 1000000.

## Deterministic derivation

`consolidate` with `action: "run"`, `mode: "repository"` and `scope` set to the repository name reads the newest complete snapshot in that repository's conversation, resolves it, and stages one derived event per node and per relationship. No model is called on any path, and the run succeeds on a server built with no consolidation runtime at all.

Because the derivation is deterministic, its provenance is pinned rather than measured: `model_id` and `prompt_id` name the extraction contract `repository-graph-extract`, the temperature is zero, the call id is the snapshot digest, and every token count and the cost are honestly zero. The run id is `repository-graph/` followed by the digest in lowercase hex. Every derived event carries `derived_inference` authority and one citation naming the LSN and byte range of the snapshot line it came from, so a graph answer can always be read back to the observed bytes.

## Generations

A repository extraction is an ordinary consolidation run. It opens with a single `Publish` phase and a declared budget it does not spend, stages its derived events in batches so the protocol batch limit holds, and closes with the compare-and-swap that publishes the generation. Re-running the same snapshot is detected through the run history as a duplicate and appends nothing. A later, changed snapshot publishes a new generation in which a node already visible is revised against its current version rather than minted again, and its relationships are re-asserted; the newest lineage entry wins. A published repository generation is retractable exactly like any other run.

## Reading the graph

`recall` with `mode: "graph"` walks the validated relationships of the published generation from an anchor node. `filters.anchor` is either 64 lowercase hex characters, which is a node id as the extraction run and this mode's own items report it, or a repository name, in which case `query` holds the node name and the surface resolves it by trying the candidate node id for each of the six fact kinds and taking the one that has a visible memory record.

Each item names the `relation`, the `direction`, the endpoint's `node_id` and `name`, the evidence-counted `weight_micros`, the `valid_from_ns` and `valid_to_ns` validity interval, and the `edge_lsn` of the asserting event, with `derived_inference` authority and an `hm://` uri. The envelope's provenance carries that uri and one uri per cited snapshot LSN, and its health reports the active generation and the resolved anchor. `limit` is clamped to 256 neighbours, and `filters.temporal_from_ns`, when supplied, is the validity instant, so an as-of traversal is possible. An anchor that matches no visible node returns an ok envelope with no items and a `graph_anchor_unresolved` gap, never silence.

## Stated limitation

Facts a newer snapshot no longer declares are not retracted. The older node and relationship records stay visible through the generation lineage, and both the extraction envelope and this page say so rather than claiming a sweep. Removing a fact from a repository graph today means retracting the generation that declared it.
