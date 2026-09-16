# HyperMind deployment suite

Run HyperMind on your workstation, a Linux server, Kubernetes, or a persistent cloud service. These files build from this checkout; they do not assume a published image, package, or qualified v1 release.

## Choose a platform

| Platform | Included deployment | Access and persistence |
| --- | --- | --- |
| [Docker](docker/README.md) | Distroless image, Compose, optional mTLS override | Private Unix socket by default; named volume |
| [Windows](windows/README.md) | PowerShell launcher; Docker Desktop / WSL2 guide | Linux containers; persistent named volume |
| [macOS](macos/README.md) | Native user launch agent and Docker guide | Private user-owned state |
| [Linux](linux/README.md) | Preflight, installer, hardened systemd unit, mTLS drop-in | Dedicated account and `/var/lib/hypermind` |
| [Kubernetes](kubernetes/README.md) | Helm chart, single-replica StatefulSet, PVC, TLS Secret, probes | Internal mTLS service and persistent volume |
| [Fly.io](fly/README.md) | Fly configuration and persistent-volume setup | TCP passthrough to engine mTLS |
| [Railway](railway/README.md) | Provider configuration and volume/secret setup | Private network or TCP proxy; persistent volume |
| [Render](render/README.md) | Blueprint for a private service with a persistent disk | Private mTLS endpoint, not a public HTTP web service |

Windows support means Docker Desktop with Linux containers or WSL2. The Unix-socket daemon is not a native Windows service. The container currently targets **Linux amd64**; Apple Silicon uses emulation for this image. Native macOS builds use the host architecture.

## Docker

From the repository root:

```sh
docker compose -p hypermind -f deploy/compose.yaml build
docker compose -p hypermind -f deploy/compose.yaml run --rm hypermind \
  init --path /var/lib/hypermind --if-missing --json
docker compose -p hypermind -f deploy/compose.yaml up -d
docker compose -p hypermind -f deploy/compose.yaml exec hypermind \
  hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

This publishes no host port. The [Docker guide](docker/README.md) covers SDK access over mTLS, lifecycle commands, and volumes. Do not use `down --volumes` unless you intend to destroy the deployment's state and keys.

## Shared deployment contract

**One owner, persistent state.** Run one daemon for each state directory. Keep configuration, keyrings, ledger, and projections on durable storage. Do not scale replicas against the same volume or mount one actor directory into multiple owners. A container restart is not a reason to regenerate keys.

**Private by default.** Remote gRPC and REST require mutual TLS and capability tokens. A client certificate alone grants no actor authority. Templates do not publish an admin listener. Use raw TCP passthrough or private networking: a provider's HTTPS terminator cannot transparently replace the engine's client-certificate authentication.

**Explicit initialization.** `hm init --if-missing` preserves valid existing configuration. Cloud commands use `serve --init-if-missing` to initialize fresh storage at runtime. Missing TLS inputs or malformed configuration fail instead of silently creating replacement identities.

**Runtime secrets.** Keep TLS private keys, capabilities, encryption keys, and provider credentials out of Git, image layers, build arguments, logs, and public deployment output. Use private files or provider secret stores. Never distribute the full server configuration to clients.

**Volume ownership.** Compose and Kubernetes use the non-root image. Cloud templates requiring container root for provider-managed volume ownership declare that exception in their guide. This is not privileged mode; operator-prepared writable volumes offer a non-root alternative. Do not add host mounts or make private data world-readable to fix permissions.

## Certificates and providers

For evaluation, [generate short-lived development certificates](tls/README.md) for the exact DNS endpoint used by your client. The helper separates the CA signing key, server identity, and client identity. Production needs managed PKI, renewal, and revocation.

File-based TLS uses `--tls-cert`, `--tls-key`, and `--tls-client-ca`. The explicit `--tls-from-env` alternative reads PEM contents directly from these runtime secrets:

```text
HM_TLS_CERT_PEM
HM_TLS_KEY_PEM
HM_TLS_CLIENT_CA_PEM
```

These values are PEM contents, not filenames. Do not combine environment-TLS mode with file-TLS arguments.

Lexical operation requires no provider. Optional embedding, reconstruction, and consolidation use the [documented opt-ins and budgets](../docs/reference/config.md). Centra routing uses `CENTRA_GATEWAY_URL` and `CENTRA_GATEWAY_API_KEY`; reconstruction and consolidation use `openrouter/openai/gpt-5.6-luna`. Deploying the service or supplying a key does not itself enable provider features.

## Operations

Before serving application traffic:

1. Confirm durable storage is mounted and only one owner is running.
2. Run authenticated `hm doctor --require-healthy --json` inside the deployment or as its local service account.
3. Remember a unique test observation, restart the daemon, and recall it. A TCP health check alone does not prove storage continuity.
4. Test remote access with a real client identity and capability, then confirm a missing certificate or wrong capability is rejected.

For backups and upgrades:

1. Stop the daemon gracefully and wait for it to exit. SIGINT and SIGTERM request orderly shutdown.
2. Back up the complete state directory to encrypted storage, including configuration and keyrings. Ledger-only backups are insufficient; projections and exports may contain plaintext.
3. Record the image digest or binary revision and test restoration in a separate, isolated deployment.
4. Install the new build, start one owner, and repeat doctor and recall checks. Retain the previous binary and matching backup until validation finishes.

A binary rollback is not a schema migration. Never restore over a live owner or copy live LMDB files as an ordinary filesystem backup. Crypto-shredding cannot erase keys retained in your separate backups; include snapshot retention in your deletion policy.

## Validation and limits

See [deployment verification](verification.md) for checks that do not provision cloud resources and the distinction between configuration validation, local runtime tests, and platform qualification.

The portable image supports lexical memory and configured remote providers; it does not bundle ONNX Runtime. Downloading model artifacts does not prove local inference readiness. Base image digests and Cargo dependencies are pinned, but distribution build packages are not individually locked: a successful image build is not reproducible-release certification.

Applying cloud manifests can create billable resources. This suite does not automatically deploy to your accounts, purchase plans, register domains, publish images, or manage production certificates. Review each platform's storage, networking, permissions, and cost implications before applying its configuration.
