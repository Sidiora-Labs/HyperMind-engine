# Render

The Blueprint is a **paid private service**, not a public web service. Render private networking carries arbitrary protocols, so clients in the same workspace and region can preserve end-to-end mTLS to HyperMind. Render's managed public HTTPS frontend terminates TLS and is not a supported direct ingress for these listeners. Host a caller on the private network or choose Fly/Railway raw TCP for external access; do not weaken the daemon's client-certificate requirement. [Private services](https://render.com/docs/private-services), [private networking](https://render.com/docs/private-network), [managed TLS](https://render.com/docs/tls)

Create a Blueprint from your repository using `deploy/render/render.yaml`. It builds `deploy/Dockerfile`, requests one instance and a 10-GiB persistent disk, and disables automatic redeploys. Account resource creation incurs charges and must be performed by the operator. Local validation has not deployed this Blueprint. [Blueprint reference](https://render.com/docs/blueprint-spec)

Supply the three prompted secrets as complete multiline PEM values: `HM_TLS_CERT_PEM`, `HM_TLS_KEY_PEM`, and `HM_TLS_CLIENT_CA_PEM`. For evaluation, first generate bootstrap credentials for `localhost` in a new private directory:

```sh
deploy/tls/create-dev-certs.sh /absolute/new/private/bootstrap-tls localhost
```

Use only the files in `server/` for the service's secrets. After creation, copy the real private hostname from Render's **Connect → Internal** address, generate a separate evaluation identity with that hostname as the DNS SAN, and replace all three secrets before connecting clients:

```sh
deploy/tls/create-dev-certs.sh /absolute/new/private/service-tls ACTUAL_PRIVATE_HOSTNAME
```

The bootstrap identity is a real localhost-only certificate, not a usable remote identity; do not skip replacement or disable hostname checks. Clients receive only `client/` from the final identity, plus their actor capability. Keep `ca/ca.key` offline. Production should issue the actual hostname through managed PKI rather than the development helper. TLS is parsed in memory by `--tls-from-env`; no private PEM is written to the image or persisted store. [Runtime secrets](https://render.com/docs/configure-environment-variables)

Connect over `https://ACTUAL_PRIVATE_HOSTNAME:8443` for gRPC or `:8444` for REST with the trusted CA and client certificate. Obtain only `actor=ID:TOKEN` from the private persisted config via authorized administrative filesystem access; never distribute the KEK/admin token or whole config. No admin TCP listener is enabled. No unauthenticated HTTP health path is supplied; use an authorized direct process execution of `/usr/local/bin/hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json` where available, plus an authenticated client request. The distroless image has no shell; shell-only platform consoles are not a health-check mechanism.

The Blueprint explicitly sets non-secret build args `HM_RUNTIME_UID=0`/`HM_RUNTIME_GID=0` so a fresh mounted disk can be initialized. This **cloud image runs as container root**; the default image remains UID 65532. Render translates these variables to build args; the Dockerfile never declares the secret PEM/provider variables as build args. For non-root operation, have an operator prepare all mounted state for UID/GID 65532 while stopped and change both values to 65532 before rebuilding. [Docker environment handling](https://render.com/docs/docker)

Persistent disks disable zero-downtime deploys and horizontal scaling. Keep the same disk and private configuration during updates, budget for brief downtime, and maintain encrypted stopped-writer backups. Never delete/recreate the disk to resolve a startup error. [Persistent disks](https://render.com/docs/disks)
