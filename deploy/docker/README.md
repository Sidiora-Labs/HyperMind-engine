![HyperMind](../../spec/readme_img.png)

# Docker and Compose

The supplied image builds HyperMind from source and runs a static Linux amd64 binary as UID/GID 65532 in a shell-free distroless runtime. Docker Desktop provides the Linux VM on Windows and macOS. Apple Silicon currently needs amd64 emulation; no multi-architecture image is published by this repository.

## Local daemon

From the repository root, with Docker Engine and Compose available:

```sh
docker compose -p hypermind -f deploy/compose.yaml build
docker compose -p hypermind -f deploy/compose.yaml run --rm hypermind \
  init --path /var/lib/hypermind --if-missing --json
docker compose -p hypermind -f deploy/compose.yaml up -d
docker compose -p hypermind -f deploy/compose.yaml exec hypermind \
  hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

Initialization preserves existing credentials. The named volume persists state across container recreation. Always use the same Compose project name; a different name selects a different volume. Do not initialize or modify the volume from a second running daemon.

The default service publishes no host ports. Talk to it through the CLI in its container:

```sh
docker compose -p hypermind -f deploy/compose.yaml exec hypermind \
  hm remember --config /var/lib/hypermind/hypermind.conf \
  --conversation demo --content 'HyperMind is running in Docker.' --json
docker compose -p hypermind -f deploy/compose.yaml exec hypermind \
  hm recall --config /var/lib/hypermind/hypermind.conf --query Docker --json
```

`recall` returns ledger IDs. Use an SDK renderer for activation content or the container's `hm tui` command for an activation view.

## Host SDK access over mTLS

Provision a server certificate valid for `localhost`, its private key, and a trusted client CA. Place `server.pem`, `server.key`, and `client-ca.pem` in a private directory readable by container UID 65532. Keep client private keys outside that directory and outside the repository. On Linux, host bind-mount ownership must permit UID 65532 to read the files; do not solve permissions by making keys world-readable.

```sh
export HM_TLS_DIRECTORY=/absolute/private/path/to/server-tls
docker compose -p hypermind -f deploy/compose.yaml -f deploy/compose.mtls.yaml config --quiet
docker compose -p hypermind -f deploy/compose.yaml -f deploy/compose.mtls.yaml up -d
```

The override publishes gRPC at `localhost:8443` and REST at `localhost:8444`, bound only to loopback. It exposes no admin endpoint. SDKs require the client certificate, key, CA, and an actor capability. Provision that capability through a secure administrator channel; the full `hypermind.conf` also contains encryption keys and the admin capability and must never be distributed to clients.

## Operations

```sh
docker compose -p hypermind -f deploy/compose.yaml ps
docker compose -p hypermind -f deploy/compose.yaml logs --tail=100 -f
docker compose -p hypermind -f deploy/compose.yaml stop
docker compose -p hypermind -f deploy/compose.yaml start
```

Include the mTLS override for `up` when using remote access. Stop uses SIGINT with a grace period. `down` removes containers and their network but retains the named volume; **do not use `down --volumes`** unless deliberately deleting all stored state and keys. Follow the [backup and upgrade checklist](../README.md#operations) before rebuilding/recreating containers.

The portable image supports lexical operation and configured remote providers. It does not bundle an ONNX Runtime shared library. Base images and Cargo dependencies are pinned; distribution build packages are not individually locked, so an image build alone is not reproducible-release certification.
