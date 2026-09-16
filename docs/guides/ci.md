# Continuous integration

When to use: reproduce a failing check, review a workflow change, or configure required status checks for this repository.

Do not use: a workflow file, skipped job, local test, or uploaded document as evidence that hosted CI or release qualification passed. CI does not authorize provider spending, publishing packages, or deploying cloud resources.

## Checks

The [main workflow](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/.github/workflows/ci.yml) runs on pushes, pull requests, and manual dispatches:

| Check | What it executes |
| --- | --- |
| Quality | Workflow lint/policy, authored Rust formatting, and Clippy with warnings denied across all targets |
| Rust | Locked workspace build/tests, actual slice journeys, and HNSW tests on Linux x86-64 and aarch64 |
| SDKs | TypeScript native/client/render/migration tests, a built Python extension plus mTLS tests, and Go race-enabled tests against the real daemon |
| Documentation | Generated API coverage, local links including community policies, ten READMEs, and an mdBook build artifact |
| Dependencies | Cargo advisory/license/source policy and high-severity npm audits for both workspaces |
| Internal evaluation | Separate slice-1–6 gates, absolute thresholds, and available default-branch baseline comparison |
| Public evaluation | Complete cached LongMemEval/LoCoMo replay with slice-7 acceptance thresholds; no live provider calls |
| Required CI result | Fails unless every prerequisite succeeds, including jobs that fail, are cancelled, or are skipped |

The [deployment workflow](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/.github/workflows/deployment.yml) additionally validates manifests, native Linux/macOS startup, Windows launcher plans, and an actual non-root Docker remember/restart/recall lifecycle. The [fuzz workflow](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/.github/workflows/fuzz.yml) runs bounded fuzz smoke, the actual SIMD test under Miri, and explicitly native storage checks. Native LMDB coverage is not described as Miri coverage.

Changes to generated schema bindings are checked through their canonical source and behavior tests; rustfmt does not rewrite generated output or carried donor fixtures. The Go historical protected-constraint mismatch remains a real failing compatibility gate, documented in the [Go compatibility note](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/COMPATIBILITY.md). It is not excluded or converted into a permitted failure.

## Reproducible setup and boundaries

External actions use full commit SHA pins. The compiler is Rust 1.93.0, Node is pinned in [.nvmrc](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/.nvmrc), Go follows its module declaration, and Python CI dependencies are pinned in [requirements-ci.txt](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/requirements-ci.txt). Standalone CI tools have versioned archives and checked SHA-256 digests. Dependency updates are proposed through Dependabot; they are not automatically merged.

Workflows use read-only default permissions, non-persistent checkout credentials, bounded runtimes, and cancellation of superseded runs. They do not use `pull_request_target` to execute contributed code. Rust caches contain dependency downloads, not credentials, provider responses, or reusable compiled executables. Documentation and measured reports are explicitly selected for artifact upload; private state and `.env` are not uploaded.

Hosted images, package registries, and operating-system package repositories can still change. These pins improve traceability; they do not establish reproducible-release certification. CI creates no release tag, published image, package, cluster, or cloud deployment.

## Public evidence must be configured explicitly

Set repository variables `SLICE7_EVIDENCE_RUN_ID` and `SLICE7_EVIDENCE_SHA256` only after reviewing a complete, trusted `slice7-public-evidence` artifact. The digest is the artifact SHA-256 without the `sha256:` prefix. The evidence-producing run must have succeeded on this repository's default branch, triggered by a push or manual dispatch; fork/PR-produced evidence is refused. Artifact expiration or any digest mismatch fails closed.

After extraction into `eval/`, the artifact must supply the exact public datasets under `datasets/` and matching reader/judge response and accounting caches under `cache/slice7-gateway/`. Review the contents for secrets, unrelated responses, licenses, and complete coverage before publishing any evidence. Scores alone cannot replace response caches. Never upload the workspace `.env` or credentials.

No evidence-producing workflow or artifact is assumed to exist merely because the replay consumer is configured. The current local live campaign is separate; until its full evidence has been reviewed and published through an approved default-branch workflow, the public replay job deliberately fails. It never falls back to paid calls, synthetic answers, or a partial benchmark. `CENTRA_GATEWAY_URL` may be set as a non-secret repository variable for matching cache identity; no provider API key is used by CI.

## Local commands and branch protection

The root [Makefile](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/Makefile) exposes individual checks. Run the one relevant to your change; do not treat `make help` as an aggregate release gate. Python tests require the real native extension to be installed first, as described in the [Python SDK guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/README.md).

An administrator can require `Required CI result` and the relevant deployment/fuzz checks using repository rulesets after the checks have appeared. This change does not modify GitHub branch protection, enable private security reporting, or supply missing benchmark evidence. Review actual job results before marking specification tasks done.
