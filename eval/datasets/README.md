![HyperMind](../../spec/readme_img.png)

# Evaluation datasets

Large public benchmark inputs and judge responses are not committed. The slice 4 gate looks for the cleaned LongMemEval-S release at `eval/datasets/longmemeval/longmemeval_s_cleaned.json` and no-consolidation predictions at `eval/datasets/longmemeval/wave4-no-consolidation.jsonl`. The paths can be overridden with `LONGMEMEVAL_DATASET` and `LONGMEMEVAL_PREDICTIONS`.

The slice 6 gate uses the same author dataset with consolidation-enabled predictions at `eval/datasets/longmemeval/wave6-consolidation.jsonl`. Override that path with `LONGMEMEVAL_CONSOLIDATED_PREDICTIONS`. When predictions are available, the gate requires at least 80 percent judged accuracy.

Use the cleaned LongMemEval-S artifact published by the benchmark authors. Record its content digest alongside a run. Judge responses live under `eval/datasets/longmemeval/judge-cache/`; each filename hashes the pinned judge, prompt version, question, reference, and candidate answer. Cache files and downloaded datasets remain local.

If either dataset or predictions are absent, the gate records zero LongMemEval coverage and zero accuracy. It does not substitute examples or claim a benchmark score.

The long-context probe adapter reads an operator-supplied published artifact at `eval/datasets/beam/conversations.json`, overridable with `HM_BEAM_ARTIFACT`. `hm-eval` normalizes that artifact into its own probe set and writes it to `eval/datasets/beam/probe-set.json`, overridable with `HM_BEAM_PROBE_SET`. A probe survives normalization only when it carries a question, a reference answer and grading criteria, and, unless it is an abstention probe, evidence message ids that resolve inside its own conversation. If the published artifact is absent, the run records zero coverage for it and claims no benchmark score; the checked-in probe fixture at `eval/fixtures/beam/probe-set.json` is HyperMind-authored material for testing the adapter and the pipeline bridge, and it is never substituted for the published artifact or reported as a benchmark result.
