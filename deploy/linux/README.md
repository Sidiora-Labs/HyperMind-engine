![HyperMind](../../spec/readme_img.png)

# Linux

Use the native systemd service on a Linux host, or the [container deployment](../README.md#docker). The native route needs systemd, a C/C++ build toolchain, CMake, Perl, pkg-config, Git, and the Rust toolchain pinned in the repository.

## Build and install

From the repository root:

```sh
cargo build --locked --release -p hm-cli
bash deploy/linux/install.sh --check target/release/hm
sudo bash deploy/linux/install.sh target/release/hm
sudo systemctl enable --now hypermind.service
sudo -u hypermind /usr/local/bin/hm doctor \
  --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

The installer creates a non-root `hypermind` account, installs the binary and [service unit](../hypermind.service), and initializes mode-0600 credentials inside `/var/lib/hypermind`. It does not start the service, overwrite an existing configuration, or replace a customized service unit. `--check` performs a read-only preflight. The build uses the host architecture; this guide does not imply a published package or binary.

The default endpoint is a Unix socket. Run CLI commands as the service account so they can read the private configuration:

```sh
sudo -u hypermind hm remember --config /var/lib/hypermind/hypermind.conf \
  --conversation deployment --content 'The service is installed on this host.' --json
sudo -u hypermind hm recall --config /var/lib/hypermind/hypermind.conf \
  --query 'service installed' --json
sudo journalctl -u hypermind.service -f
```

Do not start an embedded engine or MCP owner against the same actor directory while this daemon is running. CLI clients are fine.

## Remote access

Provision a server certificate with the host's DNS name in its SAN, its private key, and the CA that signs authorized client certificates. Install them as `/etc/hypermind/tls/server.pem`, `server.key`, and `client-ca.pem`, readable only by root and the service account. Then install the supplied drop-in:

```sh
sudo install -d -m 0755 /etc/systemd/system/hypermind.service.d
sudo install -m 0644 deploy/linux/remote.conf \
  /etc/systemd/system/hypermind.service.d/remote.conf
sudo systemctl daemon-reload
sudo systemctl restart hypermind.service
```

This enables actor gRPC on 8443 and actor REST on 8444, both with end-to-end mutual TLS. Only open ports required by your clients in the host firewall. No admin network listener is exposed. Client capabilities are separate from certificates; never hand clients the complete server configuration.

Optional Centra provider variables go in `/etc/hypermind/providers.env`, owned by root and mode 0600. The system service manager reads this file; there is no reason to make provider keys public or include them in the unit. Use the documented feature opt-ins in the [configuration reference](../../docs/reference/config.md).

## Stop, upgrade, and recover

```sh
sudo systemctl stop hypermind.service
sudo bash deploy/linux/install.sh /absolute/path/to/new/hm
sudo systemctl start hypermind.service
sudo -u hypermind hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

Take an encrypted backup of the entire state directory, including `hypermind.conf` and keyrings, while stopped before upgrading. Keep a known-good binary and backup together: a binary rollback alone is not a schema downgrade. Never copy a live LMDB directory or run two writers against restored state. See the [operations checklist](../README.md#operations).

Disabling the service with `systemctl disable --now hypermind.service` leaves all state and keys intact. The suite deliberately has no data-purging uninstall command.

The unit's `StateDirectory`, restrictive umask, filesystem sandbox, non-root identity, and SIGINT shutdown are defined by [systemd execution settings](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml) and [kill settings](https://github.com/systemd/systemd/blob/main/man/systemd.kill.xml). Syntax/preflight validation is distinct from actually installing this service on a host.
