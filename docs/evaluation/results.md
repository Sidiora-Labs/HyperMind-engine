# Results and qualification status

This checkout is under active development. **v1.0.0 is not qualified or released.** Targets are acceptance criteria, not achieved scores.

| Requirement | Target | Current claim |
| --- | --- | --- |
| LongMemEval | 500 questions; accuracy ≥ 0.90 | Full live Luna run in progress; no final score claimed. |
| LoCoMo | Full coverage; non-adversarial F1 ≥ 0.75 | Live qualification pending. |
| Attention | Suppression precision ≥ 0.90 | Declared gate required; fixtures are not a user study. |
| HNSW | Recall parity ≥ 0.98 | Index parity is separate from semantic quality. |
| Recall | Recall@10 ≥ 0.95 at 10k | Encoder/cohort-specific reports required. |
| Activation | Warm p99 < 10 ms at 100k | Release-scale qualification required. |
| Integrity/continuity | No laundering/lossy rewrites; full continuity | Individual journeys do not establish release qualification. |
| Determinism | Identical x86-64/aarch64 bundles | Cross-architecture release gate pending. |

The focused local slice-7 journey passed: real filesystem observations, quiet-hours batching, contradicted predictions, three independent procedure episodes across two conversations, restart/idempotency, MCP/UDS activation, and recall. Wave 7 remains open while its public benchmark gate is incomplete.

When to use: distinguish implementation evidence, diagnostics, full benchmarks, and published CI artifacts.

Do not use: temporary partial scores as final results, targets as marketing evidence, or diagnostic pass rates as population accuracy. Inspect actual machine-readable `eval/results` reports and their identities before numerical claims. Never publish provider credentials.
