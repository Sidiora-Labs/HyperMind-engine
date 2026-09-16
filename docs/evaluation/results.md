# Results and qualification status

This checkout is under active development. **v1.0.0 is not qualified or released.** Targets are acceptance criteria, not achieved scores.

| Requirement | Target | Current claim |
| --- | --- | --- |
| LongMemEval | 500 questions; accuracy ≥ 0.90 | Complete local live run: 459/500 correct (91.8%); lexical-only retrieval. |
| LoCoMo | Full coverage; non-adversarial F1 ≥ 0.75 | Live run in progress; no final score claimed. |
| Attention | Suppression precision ≥ 0.90 | Declared gate required; fixtures are not a user study. |
| HNSW | Recall parity ≥ 0.98 | Index parity is separate from semantic quality. |
| Recall | Recall@10 ≥ 0.95 at 10k | Encoder/cohort-specific reports required. |
| Activation | Warm p99 < 10 ms at 100k | Release-scale qualification required. |
| Integrity/continuity | No laundering/lossy rewrites; full continuity | Individual journeys do not establish release qualification. |
| Determinism | Identical x86-64/aarch64 bundles | Cross-architecture release gate pending. |

## LongMemEval local result — 2026-09-16

The [complete machine-readable report](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/eval/results/2026-09-16/longmemeval.json) records 500 generated answers, 500 graded answers, 459 correct, and no run failure. This clears the 90% accuracy target locally; it is not a hosted CI or release result.

Both reader and judge used `openrouter/openai/gpt-5.6-luna` through Centra, with medium reasoning and total output caps of 4,096 and 2,048 tokens respectively. Retrieval was `lexical_only`. The run reused 19 matching reader and 19 matching judge cache entries; the remaining requests were live. The model route is a catalog alias, not an immutable dated snapshot.

Dataset revision: `98d7416c24c778c2fee6e6f3006e7a073259d48f`; dataset SHA-256: `d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442`. The unchanged official category rubrics are pinned to scorer revision `9e0b455f4ef0e2ab8f2e582289761153549043fc`.

| Category | Correct / evaluated |
| --- | --- |
| Abstention | 28 / 30 |
| Knowledge update | 70 / 72 |
| Multi-session | 101 / 121 |
| Single-session assistant | 55 / 56 |
| Single-session preference | 24 / 30 |
| Single-session user | 64 / 64 |
| Temporal reasoning | 117 / 127 |

The archived report's cost fields reflect conservative reservations at completion, before the shared campaign ledger was reconciled to user-confirmed upstream token rates. **They are not actual billed dollars.** Reported usage was 12,089,000 input tokens and 57,670 output tokens; the 33,597 reasoning tokens are already included in output usage. Do not count them twice or treat the shared campaign balance as this benchmark's isolated invoice.

## Remaining qualification

The task tracker distinguishes `implemented` from `done`: the slice-7 journey and wave-8 remote APIs, SDKs, migration, CLI/deployment, and documentation have implementation evidence, but their dependency and qualification gates are not all green. The original Go protected-constraint compatibility test remains blocked by the documented authority-policy mismatch. The stricter all-target Clippy check also exposed outstanding test-cast, function-size, and error-documentation diagnostics. These failures are not excluded from CI.

The focused local slice-7 journey passed: real filesystem observations, quiet-hours batching, contradicted predictions, three independent procedure episodes across two conversations, restart/idempotency, MCP/UDS activation, and recall. Wave 7 remains open while its public benchmark gate is incomplete.

When to use: distinguish implementation evidence, diagnostics, full benchmarks, and published CI artifacts.

Do not use: temporary partial scores as final results, targets as marketing evidence, or diagnostic pass rates as population accuracy. Inspect actual machine-readable `eval/results` reports and their identities before numerical claims. Never publish provider credentials.
