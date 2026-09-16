![HyperMind](../../spec/readme_img.png)

# Development certificates

For local evaluation, generate a fresh CA, a server identity, and a separate client identity with OpenSSL 3 or later:

```sh
bash deploy/tls/create-dev-certs.sh /absolute/private/new-certificate-directory localhost
```

The output directory must not exist. Keys are created with private permissions, never printed, and never overwritten. Certificates expire after seven days. The server certificate's DNS SAN is the hostname passed as the second argument: use the actual endpoint hostname for a cloud evaluation, not `localhost`.

| Directory | Use |
| --- | --- |
| `ca/` | Development CA certificate and private signing key; keep offline/private |
| `server/` | `server.pem`, `server.key`, `client-ca.pem`; only this directory belongs on the daemon |
| `client/` | `client.pem`, `client.key`, `server-ca.pem`; provision to the evaluation client |

For Compose, set `HM_TLS_DIRECTORY` to the `server/` subdirectory. The daemon's UID must be able to read that directory and its key; private file ownership is preferable to relaxing permissions. Never mount the CA signing key or client private key into the server container.

Certificates authenticate the transport, not the actor. Every client also needs its own actor capability. Do not share `hypermind.conf`, which additionally contains admin credentials and encryption keys.

This helper is deliberately not a production PKI. Use your organization's certificate authority, securely distribute trust roots, track expiration, and arrange revocation and renewal before exposing a production service. Restart the daemon with the replacement identity; certificate hot reload is not claimed.
