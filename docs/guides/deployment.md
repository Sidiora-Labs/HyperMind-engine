# Deployment

When to use: run a single durable actor owner for authenticated clients, locally or on a persistent host.

Do not use: multiple processes writing the same actor directory, ephemeral-only storage for durable memory, public unauthenticated listeners, or a successful template build as proof of production qualification.

## Choose a deployment route

The [deployment suite](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/README.md) contains platform-specific installation and operations instructions. These are source-build artifacts, not a claim that public images, native installers, or managed services have been released.

| Target | Guide | Operating boundary |
| --- | --- | --- |
| Docker / Compose | [Container guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/docker/README.md) | Non-root Linux amd64 container; named persistent volume; optional loopback mTLS ports. |
| Linux | [Linux guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/linux/README.md) | Native source build and a dedicated systemd service account. |
| macOS | [macOS guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/macos/README.md) | Native source build with a per-user launchd agent, or Docker; Apple Silicon containers use amd64 emulation. |
| Windows | [Windows guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/windows/README.md) | WSL2 / Docker Desktop running Linux containers, not a native Windows daemon. |
| Kubernetes | [Kubernetes guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/kubernetes/README.md) | One replica, one persistent claim, an existing TLS Secret, and an image you build and provide. |
| Fly.io | [Fly.io guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/fly/README.md) | Persistent state and raw TCP forwarding so HyperMind terminates mutual TLS. |
| Railway | [Railway guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/railway/README.md) | Persistent volume and private networking or a raw TCP proxy; follow the supplied current IaC setup. |
| Render | [Render guide](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/render/README.md) | Private service with a persistent disk; no public web TLS-termination path. |

The actor protocol uses Unix sockets locally; Windows support therefore means running the Linux implementation through the documented compatibility/container route. The portable container does not bundle an ONNX Runtime shared library. Downloaded model files alone do not enable local inference.

Platform-specific commands and provisioning requirements belong to the linked guides. Template parsing, local container tests, native host installation, and real cloud deployment are separate evidence levels. No live cloud deployment is claimed here.

## Establish one durable owner

Initialize private state with `hm init --path /absolute/state/path`, then run `hm serve --config /absolute/state/path/hypermind.conf`. First-run deployment helpers use `hm init --if-missing` or `hm serve --init-if-missing` to preserve existing credentials; this is not permission to initialize state from a second running owner. The serve option requires a `hypermind.conf` filename and refuses to create replacement keys when existing actor data has lost its configuration.

Choose a daemon, an MCP process, or an embedded engine for each actor directory. CLI/SDK clients may connect to the daemon; competing embedded owners may not. Keep the state directory and its configuration on persistent storage. A container rebuild, process restart, or deployment change must not silently select a fresh volume.

The generated configuration contains encryption keys and actor/admin capabilities. Restrict it to the owning account. Give clients only the actor capability they need through a secure channel, never a copy of the complete server configuration. Protect keyrings, projections, exports, logs, and backups as well as the encrypted ledger.

## Remote listeners

Four listeners are opt-in: `--grpc-bind`, `--grpc-admin-bind`, `--rest-bind`, and `--rest-admin-bind`. Every enabled listener requires a complete TLS identity, supplied either through all three file options (`--tls-cert`, `--tls-key`, `--tls-client-ca`) or through `--tls-from-env`. The environment route reads the complete PEM values in `HM_TLS_CERT_PEM`, `HM_TLS_KEY_PEM`, and `HM_TLS_CLIENT_CA_PEM` in memory. The two routes cannot be mixed. Provision trusted certificates with the names clients actually verify; do not reuse test keys.

For local evaluation, the [development certificate helper](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/deploy/tls/README.md) creates separate CA, server, and client identities in a new private directory. It is not production PKI. Mount only the server identity on the daemon, never the CA signing key or client private key.

```sh
hm serve --config /srv/hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /srv/hypermind/tls/server.pem \
  --tls-key /srv/hypermind/tls/server-key.pem \
  --tls-client-ca /srv/hypermind/tls/client-ca.pem --json
```

Both a valid client certificate and the correct capability token are required. Actor and admin listeners reject the other role. Keep admin listeners private; the deployment examples do not need an exposed admin endpoint.

Preserve TLS end to end. A public HTTP proxy that terminates TLS before reaching HyperMind is not equivalent to the required client-certificate authentication. The cloud guides select raw TCP or private-network paths accordingly; do not turn off verification to make a proxy work.

gRPC carries NCPR envelopes; REST wraps MCP arguments/results. Subscription queues are bounded: after a disconnect, reconnect after the last handled LSN. See the [protocol contracts](../reference/protocol.md), [SDK guidance](../reference/sdks.md), and [CLI declarations](../reference/generated/cli-declarations.md).

## Readiness and routine operation

Run the authenticated health command as the service account or inside the owning container:

```sh
hm doctor --config /absolute/state/path/hypermind.conf --require-healthy --json
```

A socket file or open TCP port alone is not readiness. Inspect the diagnostic result and process exit status. Health does not certify recall quality, external tool effects, or release-scale performance.

Use `hm remember` and `hm recall` as daemon clients to exercise storage. Daemon recall returns ledger IDs; use an SDK renderer, MCP, or `hm tui --frames 1 --json` with its required configuration/conversation arguments for activation content. See [CLI operations](../reference/cli.md).

Keep replica count at one for a given state volume. Use graceful shutdown and allow pending writes to finish. Do not horizontally scale a shared actor directory or allow an upgrade to create overlapping owners.

## Providers and models

Lexical operation requires no remote provider. Inject optional provider credentials at runtime through the platform's secret mechanism; never place them in image layers, checked-in manifests, public logs, or a client configuration shared with others.

Centra routing uses `CENTRA_GATEWAY_URL` and `CENTRA_GATEWAY_API_KEY`. Explicit embedding, reconstruction, and consolidation feature opt-ins are described in [configuration](../reference/config.md). Reconstruction and consolidation use `openrouter/openai/gpt-5.6-luna`; embedding uses `openrouter/openai/text-embedding-3-large`. Authorize any content transfer and define a provider spend budget before enabling these paths.

Model cache verification and runtime inference availability are different checks. Use `hm doctor` to inspect the installed environment rather than assuming an artifact download makes semantic inference available.

## Backup, upgrade, and recovery

1. Stop the owner before a simple filesystem backup. Keep an encrypted copy of the entire state directory, configuration, keyrings, and relevant trusted checkpoints.
2. Build the replacement binary/image and retain the previous artifact. A binary rollback is not a schema downgrade; keep a compatible backup.
3. Upgrade using the same persistent storage and protected configuration. Do not regenerate credentials or mount the state into two active owners.
4. Start one owner, run authenticated readiness, and inspect recovery/projection status before restoring client traffic.
5. Test a restore in an isolated directory with its own owner before relying on backups.

Stopping or removing a service is different from deleting its state. Container volume deletion, PVC deletion, provider disk deletion, and actor crypto-shredding can destroy access to stored memory. The platform guides preserve state by default; destructive cleanup must be deliberate and separately authorized.

See [operations and recovery](operations.md), [recovery internals](../internals/recovery.md), and the [threat model](../security/threat-model.md). Deployment templates do not close the outstanding benchmark, cross-architecture, reproducibility, or release-qualification gates; consult [measured status](../evaluation/results.md).
