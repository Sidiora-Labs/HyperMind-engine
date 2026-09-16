# Deployment verification

Validation is layered. Rendering a manifest is not the same as running a daemon, and running locally is not proof of a successful cloud deployment.

## Offline configuration checks

Use Python 3.11+, Docker Compose, and Helm. From the repository root:

```sh
python3 -m venv target/deployment-venv
target/deployment-venv/bin/pip install -r deploy/requirements.txt
target/deployment-venv/bin/python deploy/verify.py --require-tools
python3 -m unittest discover -s deploy/macos -p 'test_*.py' -v
pwsh -NoProfile -File deploy/windows/Test-Launcher.ps1
```

`verify.py` checks actual Compose and Helm rendering, persistent mounts, single-writer defaults, non-root local/Kubernetes settings, explicit cloud UID exceptions, end-to-end TLS configuration, runtime secret references, systemd settings, and macOS plist escaping. It creates only temporary local configuration-check directories. It does not contact a cloud API, start services, create a cluster, or read your credentials. Without `--require-tools`, absent Docker/Helm checks are visibly skipped rather than counted as passed.

Railway's source contract checks do not replace SDK evaluation; its [guide](railway/README.md) describes the pinned local SDK and account-backed plan/apply boundary.

## Real local runtime tests

```sh
cargo test -p hm-cli --test cloud_start --lib
cargo test -p hm-cli --test operations
cargo test -p hm-serve --test remote
docker build -t hypermind:local -f deploy/Dockerfile .
```

The Rust tests exercise real files, sockets, TLS identities, initialization, restart, capability rejection, and shutdown. The container build verifies packaging; run the [Docker remember/restart/recall workflow](docker/README.md) against that built image before using it. The [deployment workflow](../.github/workflows/deployment.yml) provides CI checks and a container lifecycle test; a committed workflow is not evidence that hosted CI has already run.

## Platform qualification

Windows PowerShell plan checks and Linux-run macOS plist tests do not establish successful execution under Docker Desktop or launchd. Test those routes on their actual operating systems. Similarly, Helm lint/template is not a Kubernetes installation, and provider configuration checks are not a Fly.io, Railway, or Render rollout.

For each target, record the source revision, built image digest/binary, actual OS/provider, storage identity, mTLS/capability rejection tests, authenticated doctor result, and a remember/restart/recall result. Keep keys, capabilities, private records, and provider credentials out of reports. Do not claim production or release qualification until its independent gates pass.

## Local evidence — 2026-09-16

The Linux amd64 image built from the engine source at `f7a2fef4dec46a4f3a4f5256063b5474df1033a7` passed the static-ELF guard and returned `hm 0.1.0`. Its local immutable image ID was `sha256:8f65fce68cfeb162f10549a148956dda283366913592d02fc34ce5344b7e45ce`; this is not a published registry digest.

The real container ran as UID/GID 65532, with a read-only root filesystem, dropped capabilities, no network, and no published ports. Initialization created private configuration with mode 0600. An authenticated daemon appended a unique observation at LSN 1 and recalled `[1]`. All nine doctor-reported projection checkpoints matched the one-event log. SIGINT shut down the daemon with exit code 0.

After removing the stopped container, a new container from the same image mounted the same isolated named volume. It retained the exact configuration bytes, healthy checkpoints, recall `[1]`, and untrusted-memory labels in rendered activation. It also stopped with exit code 0. Only the test-owned containers and labeled volume were removed afterwards.

Compose and Helm rendering, cloud secret/storage contracts, Linux shell checks, macOS plist tests, Windows launcher plans, the Railway local SDK graph, development certificate chains, and focused native TLS-bootstrap tests passed locally. This does not establish a native Windows/macOS service installation, an actual Kubernetes/cloud rollout, hosted CI success, or a qualified release. No image was published and no cloud resources were provisioned by these checks.
