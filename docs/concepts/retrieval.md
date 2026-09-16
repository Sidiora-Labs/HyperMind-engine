# Recall and activation

Recall reads indexed evidence; activation composes context for a conversation and question. Both expose provenance and health rather than pretending missing sources were available.

The planner combines lexical, vector, entity, temporal, graph, relation, and belief lanes where available. Identifiers favor entity lookup; temporal questions use windows; how/why queries can include procedures. The relation lane ranks relationships directly from their own embedding lane, alongside document and entity retrieval; the graph lane keeps its relation-term gating and follows an edge only when its relation name shares a literal token with the query or resolves, by exact normalized name, to the same relation term as the query in an imported vocabulary version, so reviewed alias names widen matching while fuzzy similarity never gates retrieval. Reciprocal-rank fusion combines rankings, not incompatible raw scores. Embedding-space identity includes model, revision, dimensions, and role.

The ten activation tiers are resident, intent, bindings, work ledger, prospective, conversation, entity, conflicts, fused, and temporal. The first four are required. Optional context shrinks before required bindings. If required content exceeds budget, the engine narrows the subtask and reports a gap. Deadline fallback is explicitly degraded with staleness information.

The core activation path performs index reads and no model calls. Optional query embedding is distinct from rebuilding document embeddings. Explicit reconstruction is a separately requested model-backed operation, not ordinary recall or observed evidence.

An experimental geometric alignment boost can rescale already-fused scores: candidate vectors and the query are projected onto a basis taken from the highest-ranked candidates that have a stored vector, and each score is multiplied by the resulting alignment factor. It is off by default, it never runs in the core activation path, and candidates without a stored vector stay neutral rather than being dropped. Enabling it for a deployment requires an explicit deployment identifier, a declared geometry version, and evidence produced by hm-eval whose measured boosted MRR@10 exceeds its baseline; an unqualified deployment is refused, not silently ignored. This is the same rule the optional cross-encoder rerank already follows.

When to use: `recall` for bounded evidence lookup, `activate` for whole-turn continuity and token limits, and timeline/as-of access for history.

Do not use: lexical-only output as proof of semantic retrieval, omit gaps from your client, or confuse ranking with evidence authority. Preserve actual encoder/retrieval identity in benchmarks.
