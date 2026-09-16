# HyperMind

Durable, evidence-aware memory for AI agents. Remember across sessions, recover after restarts, and build bounded context without confusing recalled text with authority.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Active development: **v1.0.0 is not qualified or released**. Source is available; benchmark targets, local test results, and release qualification are different claims. See [current evidence and limitations](docs/evaluation/results.md).

## Why HyperMind?

Agent context windows are temporary; useful memory must survive process exits without losing where information came from. HyperMind is a Rust memory engine with local storage, explicit provenance, and interfaces for both embedded applications and long-running agents.

- Durable events: an encrypted append-only ledger, replayable projections, checkpoints, and restart continuity.
- Evidence-bearing recall: lexical retrieval, optional embeddings, time-aware beliefs, disputes, and source references.
- Bounded activation: assemble relevant context within a token budget and retain untrusted-memory labels.
- Observed follow-up: intentions, predictions, outcomes, quiet-hours attention batches, and supported procedures.
- Multiple surfaces: stdio MCP, a Unix-socket daemon, authenticated gRPC/REST, and source SDKs.

## How it fits together

```text
MCP / CLI / SDK / gRPC / REST
              |
       actor + capability
              |
     append-only event ledger
              |
     projections + indexes
              |
  recall -> activation -> safe rendering
```

The ledger is the source of truth; projections and indexes are derived views. Remembering a claim does not verify it. Activation produces evidence-labeled context, not executable instructions. [Architecture](docs/concepts/architecture.md) · [Authority model](docs/concepts/authority.md)

## Build and install from source

Use a Unix development host with Git, rustup, and a native C/C++ toolchain (compiler/linker, make, CMake, Perl, and pkg-config). The repository pins Rust 1.93.0. Rust dependencies and the protobuf compiler are resolved by the build; dependency downloads require network access.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

These commands install this checkout, not published registry packages. Put Cargo’s binary directory on your client’s PATH. The default lexical path needs no model download or provider key. See the [installation guide](docs/start/quickstart.md).

## Connect an MCP client

After installation, initialize private state and register the stdio server. This two-command example uses Claude Code:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

For other clients, use an equivalent MCP server entry. Replace the configuration path with an absolute path and ensure the client can find `hm-mcp`:

```json
{
  "mcpServers": {
    "hypermind": {
      "command": "hm-mcp",
      "args": ["--config", "/absolute/path/.hypermind/hypermind.conf"]
    }
  }
}
```

Try `remember` with `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}`, then `recall` with `{"mode":"lexical","query":"region","limit":5}`. The [14-tool catalog](docs/reference/generated/tools.md) covers remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect, and forget.

Run **one owner per actor directory**: MCP, daemon, or an embedded engine—not concurrent writers. The generated configuration contains keys and actor/admin capabilities; keep it private and out of version control.

## Use the daemon and CLI

Stop the MCP owner first. Start the daemon in one terminal:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

In another terminal, append a memory, retrieve ledger IDs, and request a context bundle:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

Daemon `recall` currently returns `lsns`, not a rendered answer; daemon `activate` returns a base64-encoded HMA1 bundle. Use the MCP interface or an SDK renderer for memory text. CLI `--embedded` is an alternative only after the daemon has stopped. [CLI contracts](docs/reference/cli.md)

## Remote access requires mutual TLS

Remote listeners are opt-in. Provide a server certificate/key, a trusted client CA, and a valid capability token in the client. Run this instead of the local-only daemon; the paths below refer to certificates you provision:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

Keep actor and admin listeners separate; a client certificate alone does not grant actor authority. See [remote deployment](docs/guides/deployment.md), [protocol contracts](docs/reference/protocol.md), and [Docker, Compose, systemd, and Helm setup](deploy/README.md). No prepublished image is implied.

## SDK entry points

SDKs live in this repository. Package publication and cross-language release qualification are separate work; source availability is not a promise of npm, PyPI, or prebuilt binary availability.

| Language | Source | Entry points |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

Choose an embedded owner or a daemon client, not both for the same actor. Python requires 3.10+; Go declares 1.25.0. TypeScript build scripts live in its workspace. See [SDK guidance](docs/reference/sdks.md) and the [source-derived API catalog](docs/reference/generated/sdk-api.md) for current signatures.

## Optional providers through Centra

Local lexical memory works without a remote provider. Enable only the features you need in the environment of the owning process; this example explicitly enables all three provider-backed paths:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

Embedding uses `openrouter/openai/text-embedding-3-large`. Reconstruction and consolidation use **`openrouter/openai/gpt-5.6-luna` through `CENTRA_GATEWAY_URL`**. These opt-ins do not change the independent benchmark configuration. Historical recorded fixtures are not new provider calls.

Never commit real keys or put them in MCP JSON. Provider use sends selected content outside the process and can incur charges; obtain data-transfer authorization and set an explicit spend budget first. Model artifact downloads are opt-in and do not, by themselves, enable local ONNX inference. [Configuration](docs/reference/config.md)

## Evidence, authority, and limits

- Recalled memory is untrusted data, never a system/developer instruction or permission to act.
- Claims, attestations, observed tool outcomes, and derived procedures retain distinct evidence roles.
- Quiet hours and attention policy control follow-up; a prediction is not proof that its outcome happened.
- Ledger encryption does not imply every projection, export, log, or SDK buffer is encrypted. Protect the whole state directory.
- Single-writer ownership matters. This is not a distributed multi-writer database or a credential vault.
- Inspect `ok`, `health`, `gaps`, and provenance; transport success alone does not mean complete or correct memory.

Read the [threat model](docs/security/threat-model.md) before exposing a service or importing untrusted history.

## Benchmarks: targets are not results

The specification sets these acceptance targets:

| Gate | Target—not a measured claim |
| --- | --- |
| LongMemEval | 500 questions; accuracy ≥ 0.90 |
| LoCoMo | Full coverage; non-adversarial F1 ≥ 0.75 |
| Recall@10 | ≥ 0.95 at 10k items |
| Warm activation | p99 < 10 ms at 100k items |

The full LongMemEval run is in progress; LoCoMo qualification is pending. A focused real slice-7 journey passed, but neither that journey nor a selected diagnostic establishes release qualification. Consult [measured status](docs/evaluation/results.md) and [evaluation methodology](docs/evaluation/methodology.md), not partial scores.

## Develop and check documentation

From the repository root, use the focused journey and documentation checks below. Documentation tooling additionally needs Node.js and mdBook:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

The catalog command regenerates documented public surfaces from actual source. `docs-gate` rejects stale coverage and broken local links; it does not certify prose accuracy or external URLs. Follow [AGENTS.md](AGENTS.md) and the [task specification](spec/hypermind-01/spec.kvx) for scoped work and qualification.

## Repository map

| Path | Contents |
| --- | --- |
| `crates/` | Rust kernel, storage, cognition, surfaces, CLI, and evaluation tools |
| `schemas/` | Canonical FlatBuffers and protobuf contracts |
| `sdk/` | TypeScript, Python, and Go source packages |
| `docs/` | mdBook, generated reference catalogs, and ADR-001–010 |
| `eval/` | Dataset tooling, benchmark definitions, and evidence reports |
| `deploy/` | Source-build container and deployment manifests |
| `spec/` | Requirements, design, workflow, and task state |

Start with the [documentation index](docs/SUMMARY.md).

## License

Apache-2.0, as declared by the Rust workspace. See [LICENSE](LICENSE). Review upstream model and dataset licenses separately; the project license does not relicense those assets.
