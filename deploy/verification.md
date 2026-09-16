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
