# Evaluation datasets

Large public benchmark inputs and judge responses are not committed. The slice 4 gate looks for the cleaned LongMemEval-S release at `eval/datasets/longmemeval/longmemeval_s_cleaned.json` and no-consolidation predictions at `eval/datasets/longmemeval/wave4-no-consolidation.jsonl`. The paths can be overridden with `LONGMEMEVAL_DATASET` and `LONGMEMEVAL_PREDICTIONS`.

The slice 6 gate uses the same author dataset with consolidation-enabled predictions at `eval/datasets/longmemeval/wave6-consolidation.jsonl`. Override that path with `LONGMEMEVAL_CONSOLIDATED_PREDICTIONS`. When predictions are available, the gate requires at least 80 percent judged accuracy.

Use the cleaned LongMemEval-S artifact published by the benchmark authors. Record its content digest alongside a run. Judge responses live under `eval/datasets/longmemeval/judge-cache/`; each filename hashes the pinned judge, prompt version, question, reference, and candidate answer. Cache files and downloaded datasets remain local.

If either dataset or predictions are absent, the gate records zero LongMemEval coverage and zero accuracy. It does not substitute examples or claim a benchmark score.
