# Consolidation

HyperMind consolidation is a budgeted ledger run, not an in-place rewrite. NREM groups frozen observations and may attach, revise, or mint a memory only after citation, independence, quality, and rewrite checks pass. REM connects memories, synthesizes novel abstractions, audits existing memories, and schedules review. Provider output is untrusted until these deterministic checks accept it.

Every run declares its scope, cadence key, generation, prompt versions, and limits for calls, tokens, cost, and wall time. Phase progress is persisted with deterministic attempt prefixes so a restart resumes without duplicating accepted work. Derived records retain `derived_inference` authority and cite byte ranges from the run's frozen candidates. Minting requires three independent source roots spanning at least two conversations.

Publishing is atomic. Derived events remain staged and invisible until all relevant projections have applied them and `ConsolidationClosed` successfully compare-and-swaps the active generation. `ConsolidationRetracted` restores the parent generation immediately; cleanup is separate and cannot delay rollback.

The slice-6 evaluation gate measures zero lossy rewrites on the cortex incident fixtures, rejection of invalid citations and insufficient roots, crash recovery before and after publication, lost-ack deduplication, rollback of leased views, and consolidation-enabled LongMemEval when its author dataset is installed.
