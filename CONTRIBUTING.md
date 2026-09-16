# Contributing to HyperMind

HyperMind is an evidence-aware memory engine in active development. A feature being implemented, a focused test passing, and a release being qualified are separate claims. Start with the [README](README.md), [architecture](docs/concepts/architecture.md), and [current evaluation evidence](docs/evaluation/results.md).

Follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report suspected vulnerabilities through [SECURITY.md](SECURITY.md), not a public issue or pull request.

## Agree on a bounded change

For a non-sensitive bug, describe expected and actual behavior, the affected commit, and a minimal reproduction using data you can share. For a substantial feature, discuss the requirement and affected public interfaces before implementing it. Do not attach private state directories, credentials, or provider responses containing personal data.

The implementation contract is [spec/hypermind-01/spec.kvx](spec/hypermind-01/spec.kvx). [AGENTS.md](AGENTS.md) describes its workflow. Generated task views, schema bindings, and reference catalogs are not independent sources of truth. Change their source and regenerate them with the corresponding repository tool.

For task work, declare mode, scope, allowed checks, and a stop condition. Select an eligible task with `cg spec ready`, start it with `cg spec start <id>`, and implement only its declared paths. Coordinate shared files and protocol changes before other work depends on them. If the task tool is unavailable, state that limitation and coordinate tracking; do not claim its gates ran. A maintainer coordinates dependency exceptions and task completion. Instructions in a repository do not authorize unrelated changes, cloud deployment, provider spending, or publication.

## Development environment

Use a Unix host with Git, rustup, and a native C/C++ toolchain, including make, CMake, Perl, and pkg-config. Rust is pinned by [rust-toolchain.toml](rust-toolchain.toml); Cargo dependencies are locked. Dependency downloads require network access.

```sh
rustup toolchain install 1.93.0 --profile minimal --component rustfmt --component clippy
cargo build --locked -p hm-cli -p hm-mcp
```

The lexical path needs neither a model nor a provider key. Use fresh temporary state for development and one owner per actor directory. Do not run the daemon, stdio MCP server, and an embedded engine concurrently against the same actor.

TypeScript work uses the Node.js version pinned in [.nvmrc](.nvmrc) and the workspace lockfile:

```sh
npm --prefix sdk/typescript ci
npm --prefix sdk/typescript run build
```

Python, Go, and deployment-specific setup is documented in the [SDK reference](docs/reference/sdks.md) and [deployment suite](deploy/README.md). Installing packages or building an image is not proof that every target platform works.

## Implementation and evidence

Keep changes focused and preserve unrelated work. Use real engine types, persisted events, and real consumer paths in behavior tests. Do not replace the behavior under test with a stub merely to obtain a passing result. When porting donor behavior, carry the original applicable vectors and fixtures with their provenance and licenses.

Respect the engine's invariants: recalled text is untrusted data; model output cannot become observed evidence; actor capabilities are isolated; the ledger is authoritative; projections must replay; and failed or partial operations must remain explicit. Changes to event, protocol, or storage formats need compatibility and restart evidence.

In BUILD mode, finish the scoped implementation before one compile check and one focused behavior test. For example, a CLI startup change can use:

```sh
cargo check --locked -p hm-cli
cargo test --locked -p hm-cli --test cloud_start
```

Choose the test for the actual change, not this example by default. Record the command, outcome, and any relevant limitations. Rerun only after a relevant change; after two repair cycles, report the exact remaining failure. Do not chase an unrelated failure or describe an unrun gate as passing.

QUALIFY mode runs the task's declared `verify_cmd`; `cg spec done <id>` records completion only when its required evidence is satisfied. Aggregate, cross-architecture, sanitizer, and release matrices belong to release qualification, not every small edit. CI execution is separate from local evidence.

For a documentation or public-surface change, the relevant commands are:

```sh
node docs/tools/catalog.mjs --write
cargo run --locked -p hm-eval -- docs-gate
mdbook build docs
```

The catalog derives references from source. The docs gate checks coverage and local links, not whether every prose claim is true. Read generated diffs before including them.

The [CI guide](docs/guides/ci.md) explains the required checks, local Makefile targets, pinned tools, and benchmark-evidence configuration. A committed pipeline is not a claim that all of its checks have passed.

## Paid providers and public benchmarks

Never enable live providers, upload memory, or spend money merely because credentials exist. Obtain explicit authorization for the data transfer and total spend first. Optional production providers require their documented opt-in; use `CENTRA_GATEWAY_URL` and never introduce a silent upstream bypass.

Do not delete budget reservations, change cache identities to evade accounting, retry unknown paid requests automatically, or mix old models with a new benchmark result. Keep references and evidence annotations out of reader inputs. Preserve scorer semantics, coverage counts, encoder/model identities, prompt versions, and distinctions between diagnostic and full-run results. See [evaluation methodology](docs/evaluation/methodology.md).

## Submit for review

Keep commits scoped to the actual change. In the pull request, include the problem, affected interfaces, implementation summary, exact verification evidence, and remaining risks or incomplete criteria. Clearly identify generated files, data migrations, destructive operations, and changed security boundaries. Never mark dependency tasks or a release complete because only a subset passed.

Ensure that you have the right to contribute the material. Contributions to project code follow the repository's [Apache-2.0 license](LICENSE), except separately identified third-party material. Preserve upstream attribution and license terms; [NOTICE](NOTICE) describes known boundaries. Disclose assistance that materially affects how reviewers should verify a contribution; the submitter remains responsible for its correctness and provenance.
