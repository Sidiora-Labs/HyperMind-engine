# Railway

Use current Railway Infrastructure as Code, not a new `railway.json`/`railway.toml` deployment: Railway has deprecated that format for new services. This directory pins the authoring SDK; the Railway CLI must be at least 5.42.1. No account or deployed service was contacted by local validation. [Railway IaC](https://docs.railway.com/infrastructure-as-code), [official SDK](https://github.com/railwayapp/railway-ts-sdk)

Create/select an otherwise empty project environment. In its secure **shared variables**, set `HM_TLS_CERT_PEM`, `HM_TLS_KEY_PEM`, and `HM_TLS_CLIENT_CA_PEM`. They are runtime multiline PEM values, never build arguments. For private-network use, generate an evaluation certificate for the stable service hostname:

```sh
deploy/tls/create-dev-certs.sh /absolute/new/private/tls-dir hypermind.railway.internal
```

Upload only `server/server.pem`, `server/server.key`, and `server/client-ca.pem` as those three values. Keep `client/` private with clients and the CA signing key offline; production requires managed PKI.

From `deploy/railway`:

```sh
npm ci
railway login
railway link
export HYPERMIND_SOURCE_REPO=YOUR_ORG/YOUR_REPOSITORY
railway config plan
railway config apply
```

Review the plan before applying. This file owns the whole selected environment; do not apply it to an unrelated populated project, because omitted resources can be deleted. It declares one 10-GiB volume, one replica, no deployment overlap, and a gRPC TCP proxy on internal port 8443. Initialization runs at process startup when the volume is mounted, not as a pre-deploy command. [IaC reference](https://docs.railway.com/infrastructure-as-code/reference)

Private clients in the same environment connect to `hypermind.railway.internal:8443` (gRPC) or `:8444` (REST) using mTLS and an actor capability. Public clients use the generated TCP proxy hostname and assigned external port; **before using that route**, issue/reissue the server certificate with that actual hostname in its SAN, update the PEM variables, and restart. Alternatively, choose your own DNS name in advance, issue its certificate, and configure a DNS-only CNAME to the generated proxy hostname, keeping Railway's assigned port. Do not enable Railway's HTTPS domain proxy for these listeners. [TCP proxy](https://docs.railway.com/networking/tcp-proxy), [private networking](https://docs.railway.com/networking/private-networking)

`RAILWAY_RUN_UID=0` is an explicit container-root exception for Railway's fresh-volume ownership; default Docker/Compose/Kubernetes images remain non-root. An operator may instead prepare **all** persisted state for UID 65532 while stopped, then set `RAILWAY_RUN_UID=65532`. Do not assume image ownership survives an external mount. The manifest disables HTTP health checks because they cannot supply the required client identity. Check authenticated engine health with:

```sh
railway ssh --service hypermind -- /usr/local/bin/hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

This runs a binary directly; the image has no interactive shell. A platform "running" status alone is not proof of engine health. [Volume caveats](https://docs.railway.com/volumes/reference), [SSH command execution](https://docs.railway.com/cli/ssh)

Obtain the actor capability through authorized administrative access to only the private configuration's `actor=ID:TOKEN` entry. Never log/copy the full configuration to clients, and never use the admin capability on actor ports. Keep encrypted backups of the stopped writer's volume, avoid replicas/autoscaling, and preserve the same volume/configuration during upgrades. Optional Centra settings belong in runtime secrets and must be added explicitly to the IaC environment-variable map; a key alone does not enable consolidation.
