# Evaluation methodology

When to use: reproduce a declared gate or compare a relevant change under the same data/model/encoder identity.

Do not use: fixtures, selected diagnostics, missing rows, or partial live coverage as a full benchmark pass. Never adjust a scorer to conceal a failure.

Reports separate judge-free/judged metrics, coverage, encoder, model route, prompt version, and cost basis. Lexical/hash-feature pipelines are labeled `lexical_only`. Synthetic vector parity tests index mechanics, not semantic quality. Fixture-driven and live-provider claims remain separate.

Seeded recall/latency cohorts measure bounded mechanics at 1k/10k/100k records. Duplicated paraphrases are not open-domain generalization. Report cold/warm latency and query-embedding inclusion; corpus construction is outside timing. An unmet target is not silently relabeled achieved.

The geometry alignment ablation ranks a committed 28-document, 14-query fixture (`eval/fixtures/geometry/alignment.json`) twice with the hash-feature embedder and no network: once by plain int8 dot product and once with the experimental alignment boost applied to the fused hits. Slice 4 publishes `geometry_baseline_mrr_at_10`, `geometry_boosted_mrr_at_10` and `geometry_boost_enabled` as judge-free metrics under the suite name `geometry_alignment_ablation`, with no threshold: a lower boosted number is reported, not failed. The boost stays off by default and may only be enabled for a deployment on the strength of this measured pair, which qualifies only when the boosted MRR@10 exceeds the baseline.

## Public benchmarks

LongMemEval uses the pinned 500-question cleaned S split and unchanged category-specific upstream judge prompts. LoCoMo uses all 1,986 questions and the unchanged upstream Python F1 scorer/license. Non-adversarial and all-category metrics stay separate; optional judged accuracy is never blended into F1. Reader inputs contain only question metadata and real ledger-retrieved content. References and annotated evidence are scoring-only.

The current reader/judge use exact route `openrouter/openai/gpt-5.6-luna` through `CENTRA_GATEWAY_URL`. Medium reasoning and total completion-token caps are part of cache identity. The gateway catalog supplies no immutable dated snapshot; reports do not pretend otherwise. Historical older-model caches remain separate.

The real lexical pipeline packs complete role-aware passages with session diversity under an 80,000-character serialized excerpt limit. It retains source byte ranges and included/omitted coverage. It is not a semantic encoder or a guarantee of full evidence retrieval.

## Grading

Manual review inspects question, reference, complete answer, official category rubric, original evidence, and actual reader request. A correct count can conceal an incorrect explanation; a permissive rubric can accept intermediate steps despite a conflicting headline. Report strict answer quality separately from official rubric score. Do not replace the answer key or extrapolate selected examples to all questions.

`hm-eval diagnose longmemeval --questions ID,ID` and `rejudge longmemeval ID PREDICTION_PATH` write diagnostic reports. `--live` requires explicit spend authorization; offline runs require matching content-hash caches. Missing evidence fails closed.

## Cost and qualification

Reserve budget before dispatch; unknown calls retain reservations and are not automatically retried. Gateway-reported charges, conservative reservations, and usage-based accounting at user-confirmed upstream rates are different labeled bases. Reasoning tokens are included in total output usage; do not bill twice or reset the limit by deleting/changing caches.

The current authorized campaign uses a shared $49 ledger ceiling plus $1 reserved for earlier provider checks. This is not authorization for future campaigns. Credentials/private responses are not documentation artifacts.

`cargo run -p hm-eval -- gate slice7` requires full coverage, LongMemEval ≥ 0.90 accuracy, LoCoMo ≥ 0.75 non-adversarial F1, attention ≥ 0.90 suppression precision, and HNSW ≥ 0.98 parity. CI replay needs its explicitly pinned evidence artifact. Local success does not prove hosted CI or release qualification.
