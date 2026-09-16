# Getting help

HyperMind is under active development. Source availability, local verification, hosted CI, package publication, and release qualification are different states. Consult the [current evidence](docs/evaluation/results.md) before relying on a claimed capability. No support SLA or guaranteed response time is offered here.

## Start with the relevant guide

- [Installation and first memory](docs/start/quickstart.md).
- [CLI contracts](docs/reference/cli.md), [configuration](docs/reference/config.md), and [SDKs](docs/reference/sdks.md).
- [Deployment suite](deploy/README.md), including Docker, desktop launchers, Linux, Kubernetes, Fly.io, Railway, and Render.
- [Authority and evidence](docs/concepts/authority.md) and the [threat model](docs/security/threat-model.md).
- [Evaluation methodology](docs/evaluation/methodology.md) for benchmark coverage, cost, and cache questions.

For non-sensitive bugs and questions, use the [repository issue tracker](https://github.com/Sidiora-Labs/HyperMind-engine/issues). Search existing issues first. Suspected vulnerabilities belong in the private process described by [SECURITY.md](SECURITY.md), not a public support thread. Contribution proposals follow [CONTRIBUTING.md](CONTRIBUTING.md).

## Include useful, safe diagnostics

Provide the commit or image digest, platform and architecture, installation method, exact redacted command, expected result, actual result, and a minimal reproduction with data you may share. Include whether you use MCP, a Unix-socket daemon, an embedded engine, or a remote client, and whether optional providers are enabled. Never attach your full environment or configuration.

These commands identify the source and local binary without reading memory:

```sh
git rev-parse HEAD
hm --version
```

For a running daemon, authenticated health inspection is:

```sh
hm doctor --config /private/path/hypermind.conf --require-healthy --json
```

Review diagnostic output before posting it: paths, actor identifiers, and operational metadata can be sensitive. Replace them consistently. Never post keyrings, the state directory, TLS private keys, capabilities, provider keys, raw memory exports, or private provider response caches.

## Common checks

Only one process may own an actor directory. Stop the existing MCP, daemon, or embedded owner before opening that directory through another mode. Do not delete lock, ledger, projection, or key files merely to bypass an error.

Daemon CLI `recall` returns ledger IDs; `activate` returns a canonical bundle. Use a supported renderer or `hm tui --json` for readable activation content. A nonempty response is not proof that all requested evidence was available: inspect health, gaps, and provenance.

Remote clients need a certificate trusted by the server, the correct server name, and the appropriate actor capability. Admin and actor capabilities are not interchangeable. Public HTTPS termination on a hosting platform does not preserve end-to-end client-certificate authentication; follow that platform's deployment guide.

Lexical operation requires no provider key. Merely placing a key in the environment does not enable embedding, reconstruction, or consolidation. Model artifact verification also does not prove that ONNX inference is configured. Do not enable paid calls to troubleshoot without permission and an explicit budget.

For suspected corruption or data loss, stop writes and retain the complete state and matching binary revision in protected storage. Ledger-only copies may be insufficient. Do not run crypto-shred, overwrite keys, or restore a backup over a running owner. Share a redacted reproduction or arrange a private channel instead of uploading the original state.

Native Windows daemon support is not claimed; the documented Windows paths use Linux containers or WSL2. Cloud configuration validation does not mean an account was provisioned or a deployment was tested there.
