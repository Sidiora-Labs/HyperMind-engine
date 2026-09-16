# Fly.io

This template builds the repository's amd64 distroless image, attaches one volume, and runs one writer. No Fly resources have been created or tested by this repository's local checks.

From the repository root, choose an unused app name and create its storage:

```sh
fly apps create YOUR_APP
fly volumes create hypermind_data --app YOUR_APP --region fra --size 10
```

Prepare a server certificate whose SAN includes `YOUR_APP.fly.dev` (or your actual DNS name). For evaluation only, generate a private development CA and separate server/client credentials:

```sh
deploy/tls/create-dev-certs.sh /absolute/new/private/tls-dir YOUR_APP.fly.dev
```

Use managed PKI for production. Set the app secrets `HM_TLS_CERT_PEM`, `HM_TLS_KEY_PEM`, and `HM_TLS_CLIENT_CA_PEM` to the complete multiline contents of `server/server.pem`, `server/server.key`, and `server/client-ca.pem`. Use the Fly secrets interface or protected stdin; do not place PEMs in `fly.toml`, build arguments, shell history, or source control. Keep `client/` with the client and keep the CA signing key offline. [Fly secrets](https://fly.io/docs/apps/secrets/)

Deploy exactly one Machine, disabling Fly's extra standby Machine:

```sh
fly deploy --config deploy/fly/fly.toml --app YOUR_APP --ha=false
fly scale count 1 --app YOUR_APP
fly checks list --app YOUR_APP
fly ssh console --app YOUR_APP --command '/usr/local/bin/hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json'
```

The template deliberately uses raw TCP with no TLS/HTTP handlers: gRPC is exposed on 443 and REST on 8444, with mTLS terminating inside HyperMind. IPv6 works with Fly's dedicated IPv6 address; IPv4 clients require an explicitly allocated **dedicated**, billed IPv4 (`fly ips allocate-v4 --app YOUR_APP`). A shared IPv4/TLS handler is not a substitute—it terminates the client's TLS before HyperMind. The TCP check proves a listener is open, not authenticated engine health; `hm doctor` supplies the latter. [Fly networking](https://fly.io/docs/networking/services/), [health checks](https://fly.io/docs/reference/health-checks/)

The manifest explicitly builds with `HM_RUNTIME_UID=0` and `HM_RUNTIME_GID=0`, solely to initialize a fresh externally mounted volume; this cloud variant runs as container root, not as the default image's UID 65532. It adds no privileged mode or host mounts. For non-root operation, have an operator prepare the mounted state and all existing private files for UID/GID 65532, stop the writer, remove these two build overrides, and redeploy. Image directory ownership alone does not initialize a new volume's permissions. Startup initializes only an absent `hypermind.conf` and refuses to invent replacement keys for existing data. [Fly configuration and mounts](https://fly.io/docs/reference/configuration/)

Clients need the trusted server CA, client certificate/key, and the **actor** capability from the private persisted configuration's `actor=ID:TOKEN` entry. Retrieve that entry through authorized administrative filesystem access without exporting/logging the full configuration (which also contains the KEK and admin token). Never send the admin capability to actor endpoints. Use encrypted, stopped-writer volume backups; never scale this store to multiple independent Machines or detach/delete its volume during upgrades.
