# Schematics

This reference is normative for the implemented wave-one surface. “MUST” and
“MUST NOT” describe compatibility and integrity requirements.

## 1. Event taxonomy

Every event MUST be an `NCEV` FlatBuffer envelope with a nonzero schema version
no newer than version 2. Wave one admits these event kinds:

| Discriminant | Kind | Authority normally assigned at ingest |
|---:|---|---|
| 1 | `UserMsg` | `user_asserted` or `external_observed` |
| 2 | `DeliveredMsg` | `assistant_generated` |
| 3 | `ToolCall` | source authority |
| 4 | `ToolResult` | `tool_observed` |
| 5 | `Reasoning` | `assistant_generated` |
| 21 | `Attestation` | `runtime_fact` |

The remaining stable discriminants, 6 through 20, are reserved by schema v2
and MUST be rejected until their dependency wave enables them. Authority is
one of `user_asserted`, `external_observed`, `tool_observed`, `runtime_fact`,
`assistant_generated`, or `derived_inference`. A derived value MUST NOT acquire
an observed authority merely by being copied through another surface.

## 2. Ordering rules

LSNs MUST begin at one and increase by exactly one within an actor. A
`ToolResult` MUST cite an earlier LSN whose kind is `ToolCall`. Batch metadata
MUST either be absent (`client_event_count = 0`) or use a zero-based index less
than the declared count. The actor writer validates the complete batch before
sealing or appending it. Readers MUST use one pinned projection snapshot.

## 3. Frame layout

The durable frame header is exactly 43 bytes, all integer fields little-endian:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 4 | total frame length |
| 4 | 4 | CRC32C over bytes 8 through end |
| 8 | 8 | LSN |
| 16 | 1 | event-kind discriminant |
| 17 | 8 | wall timestamp, signed nanoseconds |
| 25 | 2 | actor namespace |
| 27 | 16 | conversation identifier |
| 43 | variable | sealed payload |

Payloads on disk MUST begin with `NCSEAL01`. XChaCha20-Poly1305 additional data
binds actor, LSN, kind, and user. A plaintext durable record MUST be rejected as
`kLegacyPlaintext`.

Local protocol messages use an independent eight-byte prefix: payload length
and CRC32C, followed by one `NCPR` `WireEnvelope`. Protocol versions 2 and 3 are
accepted by the wave-one local server.

## 4. Error codes

Numeric values are stable. New codes MUST be appended and existing values MUST
NOT be renumbered.

| Range | Codes |
|---|---|
| 0–14 | argument, length, kind, sequence, file I/O, truncation, checksum, corruption, manifest, backend, and segment failures |
| 15–26 | proof, signature, checkpoint, existence, writer, process, durability, projection, map, cryptographic, deletion, and legacy-plaintext failures |
| 27–44 | schema and ordering gates plus belief, entity, vector, lexical, temporal, intent, loop, work-ledger, and invariant failures |
| 45–54 | protocol, version, capability, capacity, availability, idempotency, protected-write, citation, deadline, and tripwire failures |

The canonical symbolic names are defined by `hm_core::ErrorCode::as_str`.
Errors crossing MCP MUST retain the code and operational location. Mutation
errors MUST also carry `effect_state` as `not_dispatched`, `unknown`, or
`rejected`.

## 5. Projection contract

Each actor owns one LMDB environment. Every projection mutation and its
checkpoint MUST commit in the same write transaction. An apply is valid only
when `applied_lsn = checkpoint + 1`. Timeline and BM25 projections MUST advance
for every event, including events that produce no index terms. Rebuild MUST
resume at the minimum checkpoint and MUST produce the same canonical dump as a
continuous application of the same log prefix. The belief database is behind a
protected write gate.

## 6. Bundle contract

An activation bundle has ten ordered tiers: resident, intent, bindings,
work-ledger, prospective, conversation, entity, conflicts, fused, and temporal.
The first four are required. Optional tiers shed from the tail; conversation
items may be coarsened before required tiers. The encoded form MUST begin
`HMA1`, include the snapshot epoch, exact token budget and spend, retrieval
manifest, visible gaps, per-subsystem health, and a BLAKE3 bundle hash.

Every surfaced item MUST have at least one provenance LSN and an `hm://` URI.
Renderers MUST drop incomplete provenance, MUST NOT emit raw bytes, and MUST
place memory only in a user-role untrusted-memory section. Memory MUST NOT be
rendered as system or developer instructions.
