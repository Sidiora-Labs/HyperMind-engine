# Architecture

The ledger is the durable authority; projections and indexes are rebuildable read models. Each actor owns one writer, encryption keys, an LMDB environment, indexes, and monotonically increasing LSNs. Actor identities are security boundaries, not search filters.

```text
MCP / CLI / SDK / authenticated remote request
                    |
          admission + actor writer
                    |
       encrypted append-only event ledger
                    |
       checkpointed projections and indexes
                    |
       recall / activation / safe rendering
```

Schema and core types define boundaries; the ledger provides recovery and proof. Projections materialize beliefs, work, intentions, predictions, procedures, generations, and retrieval data. Cognition performs bounded background work; composition orders and trims context. Surfaces preserve the same authority/error contracts. Evaluation qualifies behavior independently of a feature’s existence.

Vertical slices include the event, projection, consumer, surface, restart journey, and evaluation that make each capability usable. The current contract and task status live in `spec/hypermind-01/spec.kvx`; an implemented interface does not imply release qualification.

When to use: choose an actor for an independently authorized memory domain and a conversation for continuity within it.

Do not use: a shared actor directory as a multi-process database, projections as authoritative write surfaces, or unit tests as proof of a complete release.
