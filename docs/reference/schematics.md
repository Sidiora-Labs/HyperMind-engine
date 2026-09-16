# Schematics

This reference describes the implemented schema-v2 ledger and protocol-v3
boundaries. “MUST” and “MUST NOT” describe compatibility and integrity
requirements. The [generated schema catalog](generated/schema-types.md) copies
every current event, wire type, union, discriminator, and field from source.

When to use: implement a compatible reader, writer, or verified adapter.

Do not use: hand-maintained numeric lists or unvalidated generated buffers as a
substitute for the canonical schema and admission rules.

## 1. Event taxonomy

Every event MUST be an `NCEV` FlatBuffer envelope with a nonzero schema version
no newer than version 2. Event families include observations and tool/effect
chains; intent, loops and bindings; embeddings and beliefs; memory, graph and
review records; consolidation generations; and intentions, attention,
predictions and procedures. The schema catalog is the complete current list,
including additive event discriminants that earlier slices did not admit.

Authority is one of `user_asserted`, `external_observed`,
`tool_observed`, `runtime_fact`, `assistant_generated`, or
`derived_inference`. A derived value MUST NOT acquire observed authority merely
by being copied through another surface.

## 2. Ordering rules

LSNs MUST begin at one and increase by exactly one within an actor. The actor
MUST validate every member of a batch before sealing or appending any member.
Readers MUST use one pinned projection snapshot.

A `ToolResult` or `Effect` MUST cite an earlier `ToolCall` LSN. An `Outcome`
MUST name an effect already present in the work ledger. A `LoopClosed` MUST
name an open loop in the same conversation; loop identifiers are unique across
the actor and a loop can close only once. A `LoopClosed{reason=done}` and every
`Outcome` with evidence MUST cite one or more ledger events carrying
`tool_observed`, `external_observed`, or `runtime_fact` authority. Memory,
summary, and reconstruction records MUST NOT satisfy this evidence rule.

A `Binding` MUST select exactly one of `task` and `scope`. Its entity, property,
revision, evidence LSN, and freshness requirement MUST be nonempty or nonzero.
There are at most 4,096 open loops per actor and at most 128 bindings per
target.

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
accepted by the local server; continuity requests require version 3.

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
when `applied_lsn = checkpoint + 1`. Every registered projection MUST advance
for every event, including events that do not affect its materialized values.
This includes retrieval, belief/time, generations, attention, predictions,
procedures, continuity, and work-ledger projections.

Rebuild MUST start from the minimum projection checkpoint and MUST produce the
same canonical dump as continuous application of the same log prefix. Intent
holds the current objective and open loops. Bindings resolve independently of
ranking and are never shed. Work ledger entries retain unreconciled calls and
effects. `LatestCheckpoint` searches at most 65,536 timeline records for the
requested turn and fails with `kCapacityExceeded` instead of silently claiming
absence after that bound. The belief database remains behind a protected write
gate.

## 6. Bundle contract

An activation bundle has ten ordered tiers: resident, intent, bindings,
work-ledger, prospective, conversation, entity, conflicts, fused, and temporal.
The first four are required. Optional tiers shed from the tail; conversation
items may be coarsened before required tiers. If required content alone exceeds
the budget, activation MUST narrow the active subtask and emit a
`narrowed_subtask` gap. Missing, stale, or conflicting required bindings and
every dropped optional tier MUST also be visible in `gaps`.

The encoded form MUST begin `HMA1`, include the snapshot epoch, exact token
budget and spend, retrieval manifest, visible gaps, per-subsystem health, and a
BLAKE3 bundle hash.

Every surfaced item MUST have at least one provenance LSN and an `hm://` URI.
Renderers MUST drop incomplete provenance, MUST NOT emit raw bytes, and MUST
place memory only in a user-role untrusted-memory section. Memory MUST NOT be
rendered as system or developer instructions.

## 7. Idempotent append

An idempotent batch is identified by `(connection_id, client_seq)` and a digest
over every member's kind, conversation, and complete encoded envelope. A new
connection MUST begin at sequence 1. Later sequences MUST be exactly one more
than the last durable sequence. A batch contains 1 through 256 members, and
each envelope MUST repeat the connection, sequence, zero-based member index,
and common member count.

An exact replay of the latest sequence MUST return the original first and last
LSNs with `duplicate=true`; it MUST NOT append or reapply projections. Reusing
that sequence with different content MUST fail with `kIdempotencyConflict`.
Skipping or rewinding a sequence MUST fail with `kSequenceViolation`. On boot,
the daemon rebuilds the deduplication table from durable envelope metadata. A
version-3 `Welcome.next_client_seq` reports the sequence the server will admit.

## 8. Torn-batch rollback

Segment recovery first removes an incomplete physical frame tail. The daemon
then decrypts the last complete frame and examines its batch metadata. If its
member index is not `member_count - 1`, the entire trailing logical batch—from
member zero through the durable partial suffix—MUST be truncated. A malformed
tail whose indices cannot identify that boundary MUST stop recovery with an
integrity error. Deduplication and projections MUST rebuild only after this
rollback, so no partial batch becomes visible or reserves a sequence.

## 9. Work-ledger transitions

Calls enter as `dispatched`. Effects record `committed`, `outcome_unknown`, or
`returned`; tool results and outcomes can move a record to `returned`.
Transitions MUST follow this table, including idempotent self-transitions:

| Current | Allowed next states |
|---|---|
| `dispatched` | `dispatched`, `committed`, `outcome_unknown`, `returned` |
| `committed` | `committed`, `outcome_unknown`, `returned` |
| `outcome_unknown` | `outcome_unknown`, `returned` |
| `returned` | `returned` |

Every state except `returned` has `requires_reconciliation=true`. Recovery MUST
therefore surface interrupted effects as `outcome_unknown`; it MUST NOT infer
that an external operation failed or repeat it merely because an acknowledgement
was lost. The operational algorithms are specified in
[Recovery](../internals/recovery.md).
