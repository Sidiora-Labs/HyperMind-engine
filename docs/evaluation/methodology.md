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

## Probe-set benchmark

`hm-eval adapt beam [ARTIFACT_PATH]` normalizes an operator-supplied long-context artifact into a probe set at `eval/datasets/beam/probe-set.json`; `hm-eval bench beam` runs it and writes `eval/results/slice7-beam.json`. `HM_BEAM_PROBE_SET` overrides the probe-set path. Reports name the encoder `lexical_only`, because no embedding runtime is configured for this pipeline.

The judge-free block is always present and never involves a provider. It reports, per probe, how many of the annotated evidence messages the ledger returned, the lexical recall rank of the first returned evidence citation, the number of citations packed into the reader context and the characters that context occupied. Summed over the probe set it publishes evidence recall, mean reciprocal rank of the first evidence citation, the count of evidence-bearing probes for which nothing annotated was retrieved, and probe counts per kind. Abstention probes carry no annotated evidence and are counted but excluded from recall and rank means.

The judged block is optional and appears only when a gateway pass actually answered and graded. Grading is criterion by criterion: each criterion of a probe is judged on its own and the grade keeps the criterion text, the judge's evidence quote and a digest of the raw judge response. A criterion the judge failed to grade records the failure and leaves compliance unset; it is never read as a zero, and a probe with any judge failure is reported as unscored rather than folded into a mean. Event-ordering probes are scored without a judge at all, by aligning the observed events to the reference events by term overlap and combining coverage with a rank correlation; those probes are counted separately from judge-scored probes inside the judged summary.

A run over the in-repo fixture at `eval/fixtures/beam/probe-set.json` is never a benchmark claim. Such a run reports `complete: false` and records a failure saying the probe set is the in-repo fixture and not a published benchmark artifact, so a wiring check cannot be read as a score. A judge-free run over a real artifact is likewise incomplete: it records that no judged pass ran.

## Cross-SDK behavioural contract

`hm-eval contract` runs one fixed scenario through every SDK leg the machine can actually run and writes `eval/results/cross-sdk-contract.json`. The scenario remembers a user turn and an assistant turn, recalls them lexically, submits one rejected argument and one rejected mutation, inspects the first ledger event, then crosses a SIGKILL restart of the daemon to recall again and fade the first record. The in-process Rust leg is always run; the TypeScript leg runs over the Unix socket; the Python leg runs over mutual-TLS gRPC.

Comparison is on behaviour, not structure. Each leg records raw envelopes and one normalization in Rust reduces them to item identifiers, provenance URIs stripped of clock-bound query fields, authority classes, error codes, effect states, health keys, gap kinds and warning kinds. Legs agree when those reduced observations match step for step.

Runtimes that are absent are recorded, never stubbed. Every leg the harness cannot run appears in `unavailable` with the reason it could not run, and is excluded from the agreement claim rather than substituted. The Go leg is unverified here: the Go toolchain is absent and no Go contract leg is written, so the Go SDK is declared unavailable by inspection and is never reported as agreeing. The Python leg runs only where its SDK dependencies are installed; where `import hypermind` fails, the interpreter's own error is recorded as the reason. A step an SDK does not expose is listed in `unsupported` and is not counted as a divergence. The run exits non-zero when fewer than two legs participated or when the participating legs diverged.

## Controlled retrieval comparison

`hm-eval sweep retrieval` answers whether one retrieval change helps and writes `eval/results/retrieval-sweep.json`. A variant names exactly four knobs: `lexical_limit`, `minimum_term_overlap`, `session_cap` and `recency_weight`. Only `lexical_limit` reaches the kernel, as the limit of the real lexical recall call against a real actor engine. The other three are post-filters the sweep applies to the returned recall items — a term-overlap floor against the probe question, a cap on hits kept per session, and a re-rank that blends normalized lexical score with normalized ledger position. They are properties of the comparison, not configurable engine settings, and enabling one changes nothing about how the engine indexes or scores.

A variant must differ from its baseline in exactly one field. A variant that changes two fields, changes none, repeats another variant's name, carries an empty name, or carries the baseline's own name is refused before any retrieval runs, so a reported delta always belongs to a single isolated change.

The metric is judge-free and no provider is opened. Each probe-set conversation is ingested into its own actor engine, one event per message in message order, so every annotated evidence message id maps onto a known ledger position. Per probe the report records how many annotated evidence messages survived to the final ranked list and the rank of the first one; summed over the evidence-bearing probes it publishes evidence recall and the mean reciprocal rank of the first evidence hit. Abstention probes carry no annotated evidence and are counted but excluded from both means. Clock-bound values the engine returns are discarded, so two runs over the same probe set agree exactly.

Each variant's probe outcomes are cached under a directory named by the variant identity: a digest over the sweep format tag, the probe-set digest, the whole variant record and the sweep implementation itself. Every cache read re-derives that identity from the file's own contents and fails with an identity mismatch when it disagrees, so a result recorded for a different probe set, a different variant or an older implementation is never silently reused.

The report names the baseline, then each variant with the single field it changed and that field's baseline and variant values, and lists the variants that improved and the variants that regressed against the baseline. Evidence recall decides the direction and the reciprocal rank breaks a tie; a variant that moves neither appears in neither list. A run over the in-repo fixture measures wiring, not a benchmark, exactly as the probe-set benchmark section describes.
