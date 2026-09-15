# Recovery

Recovery treats the ledger as truth and assumes that an acknowledgement can be
lost after durable commit. It restores local structure; it never guesses an
external effect's outcome.

## Daemon boot

For one actor, boot performs these steps in order:

1. Open the key hierarchy and segment log. Segment recovery validates framing,
   CRC32C, manifest state, and contiguous LSNs; it truncates only a torn tail
   and refuses interior corruption.
2. Inspect the final decrypted event. If it is a partial idempotent batch,
   truncate the whole trailing batch. No member of a logical batch is exposed
   alone.
3. Read and decrypt the retained log. Validate schemas, ordering references,
   and the applied-state digest in LSN order.
4. Resume every projection from its transactional checkpoint. Because each
   projection advances its checkpoint even for irrelevant events, the minimum
   checkpoint identifies the first frame any projection still needs.
5. Rebuild the per-connection deduplication table from retained batch metadata.
   Only after these steps does the actor accept requests.

Use normal boot after an unclean daemon exit. Do not delete LMDB, manifests, or
segments to force startup: an interior-corruption or cryptographic error is an
integrity incident, not a stale-cache condition.

## Lost acknowledgement

A client MUST keep the same 16-byte connection ID while reconnecting and MUST
retain the exact encoded pending mutation until its outcome is known. After the
version-3 handshake, compare `Welcome.next_client_seq` with the pending
sequence:

- If they are equal, the server has not admitted the mutation; send the exact
  pending bytes at that sequence.
- If `next_client_seq = pending + 1`, the mutation is durable; replay the exact
  bytes. The server returns the original LSN range with `duplicate=true`.
- Any other difference is an unknown recovery history and MUST stop automatic
  replay.

Use this algorithm for transport loss during an append or checkpoint. Do not
construct a replacement payload at the same sequence, skip the pending write,
or retry an external side effect from the transport error alone. A rejected
mutation with `effect_state=not_dispatched` or `rejected` may be corrected as a
new sequence; `effect_state=unknown` requires reconciliation.

## Turn checkpoints

`Checkpoint` stores an opaque nonempty blob under a turn ID as an idempotent
ledger event. `LatestCheckpoint(turn_id)` derives the checkpoint conversation
from that ID and scans backward, up to 65,536 records, returning the latest
matching blob and its LSN.

Use checkpoints for a runtime cursor needed to resume a partially completed
turn. Do not treat a checkpoint as evidence that an external action happened,
as an authority-bearing memory, or as a replacement for intent, bindings, and
work-ledger events.

## Loop and effect resumption

After reconnect or process restart, activate the original conversation. The
required intent, bindings, and work-ledger tiers reconstruct the objective,
open loop, exact task bindings, and unresolved operations from one snapshot.
For each work item with `requires_reconciliation=true`, inspect the external
system using observed evidence. Append a returned result or outcome only when
that evidence exists. Until then, preserve `outcome_unknown`.

Use work-ledger resumption to decide what needs reconciliation. Do not infer
success from a checkpoint or memory, infer failure from a missing response, or
dispatch the operation again without an operation-specific idempotency key or
external observation.

## Subscription catch-up

`Subscribe(since_lsn, conversation?)` registers the live receiver before
reading the bounded replay, returns historical frames after `since_lsn`, and
then forwards newer frames. The replay tail prevents duplicates at the handoff.
Output queues and subscription counts are bounded; a saturated subscriber is
disconnected rather than allowed to lag silently.

Use subscription catch-up to observe commits after a known LSN. Do not use it
as durable storage or assume an open connection guarantees delivery. Persist
the last processed LSN and reconnect from it; process event LSNs idempotently.
