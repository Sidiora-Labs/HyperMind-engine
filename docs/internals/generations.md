# Generations

Each consolidation run owns a monotonically numbered generation and records the generation it expects to replace. Memory, graph, review, and document records are written with the run ID while the run is open. Their projections — memories, graph, fsrs, and documents — record staging markers at the same ledger positions, and the publish compare-and-swap verifies every one of them.

`ConsolidationClosed` publishes only when:

- the expected parent is still active;
- the declared derived-record count matches the staged set; and
- every staged record is present in its owning projection.

The active-generation pointer changes in the same projection transaction that records publication. A crash before close therefore leaves the parent readable. Replaying close after a lost acknowledgement is idempotent and cannot create a second generation.

Readers follow the active generation's ancestry. A lease pins the generation number for stable reads, but retraction is an override: once a run is retracted, its records are excluded even for a lease created before rollback. The pointer returns to the parent immediately, and physical cleanup may happen later.

The generation evaluation suite exercises these boundaries with real encrypted actor directories and OS process termination: it kills a writer after staging, restarts and publishes, kills another writer after commit but before acknowledgement, replays the batch, and then retracts while checking both active and previously leased views.

The daemon's graph and memory reads follow the same rule. Both resolve the active generation first and then walk its published lineage, so a neighbourhood only ever names memory records and edges that the active ancestry publishes. Retracting a run therefore removes its nodes and edges from the next read immediately, without any separate invalidation step, and the anchor's own record, the edges touching it, and the records of the far endpoints all come from a single snapshot so that one neighbourhood is internally consistent.
