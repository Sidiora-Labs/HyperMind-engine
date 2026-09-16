# LoCoMo scorer source

`upstream/evaluation.py` and `upstream/LICENSE.txt` are unmodified copies from
[snap-research/locomo](https://github.com/snap-research/locomo/tree/3eb6f2c585f5e1699204e3c3bdf7adc5c28cb376)
at revision `3eb6f2c585f5e1699204e3c3bdf7adc5c28cb376`, licensed under
CC BY-NC 4.0 as reproduced in `upstream/LICENSE.txt`. These vendored files are
not covered by HyperMind's Apache-2.0 license.

`score.py` loads only the original QA/F1 functions; it does not import or run
the unrelated BERTScore and ROUGE paths. Its three scorer dependencies match
the versions in the upstream environment. No QA scoring function is altered.

Install the isolated scorer environment with:

```sh
uv venv --python 3.11 target/locomo-scorer-venv
uv pip install --python target/locomo-scorer-venv/bin/python -r eval/fixtures/locomo/requirements.txt
```

The runner defaults to that interpreter; `LOCOMO_SCORER_PYTHON` overrides its
path. The adapter checks dependency versions and the upstream source digest.
