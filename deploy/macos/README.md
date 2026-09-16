# macOS: user agent or Docker Desktop

Choose one route per store. The native route builds on the Mac and installs a
per-user launchd agent; the Docker route uses its own persistent named volume.
Neither route needs `sudo`, changes system-wide services, nor deletes stored data.
Native macOS/launchd execution has not been tested in this Linux workspace.

## Native source build and launchd

Install Apple's command-line developer tools, the repository's pinned Rust
toolchain, and Python 3. Keep this checkout and the selected Python installation
at stable paths: launchd records absolute paths. The `portable` build does not
bundle ONNX Runtime; model artifact presence is not a claim of local inference
availability. [Apple developer tools](https://developer.apple.com/xcode/resources/).

From the repository root:

```sh
xcode-select --install
python3 deploy/macos/hypermind.py build
python3 deploy/macos/hypermind.py render
python3 deploy/macos/hypermind.py install
python3 deploy/macos/hypermind.py start
python3 deploy/macos/hypermind.py doctor
python3 deploy/macos/hypermind.py status
python3 deploy/macos/hypermind.py logs
python3 deploy/macos/hypermind.py stop
```

Default state is `~/Library/Application Support/HyperMind`; the plist is
`~/Library/LaunchAgents/ag.centra.hypermind.plist`. `install` initializes with
`hm init --path STATE --if-missing` and does not start the agent. State/log
directories are mode 0700, configuration/logs/plist mode 0600. An existing
insecure configuration is rejected, not silently replaced or relaxed. Existing
configuration and its credentials are never overwritten. Existing different
plists are refused unless `install --replace` is explicitly requested; replacement
preserves a timestamped backup. Stop before replacing a loaded agent.

For custom paths, pass the same options to each lifecycle command:

```sh
python3 deploy/macos/hypermind.py install \
  --state "$HOME/Library/Application Support/My Memory" \
  --binary "$HOME/src/HyperMind-engine/target/release/hm"
python3 deploy/macos/hypermind.py start \
  --state "$HOME/Library/Application Support/My Memory" \
  --binary "$HOME/src/HyperMind-engine/target/release/hm"
```

Arguments remain separate plist array elements; XML-sensitive characters are
escaped by the property-list serializer. launchd manages a foreground supervisor
that owns an exclusive lock on this managed state and runs
`hm serve --config CONFIG`. On `bootout`/SIGTERM it forwards SIGINT to `hm`, waits
up to 30 seconds, and only then kills a non-responsive child. Do not manually run
another daemon against this directory; the launcher lock is not a kernel-wide
actor-store lock. Apple documents per-user LaunchAgents, argument arrays, private
ownership, and SIGTERM handling in its
[launchd guide](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html).

`restart` stops then starts the managed agent. `uninstall` unloads it and renames
the plist to a timestamped `.disabled` file; configuration, keys, logs, and data
remain intact. Native services run in the login user's GUI session, not as a
system daemon while the user is logged out. Back up the store and key material
before upgrades and reinstall the plist if executable paths change.

## Docker Desktop

Install and start Docker Desktop for your Mac. The current image is amd64-only;
Apple Silicon uses `linux/amd64` emulation. Native arm64 container builds and
Apple Silicon runtime performance remain unqualified. Follow Docker's current
macOS requirements and optional Rosetta guidance.
[Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/).

```sh
python3 deploy/macos/hypermind.py docker --action start --build
python3 deploy/macos/hypermind.py docker --action doctor
python3 deploy/macos/hypermind.py docker --action status
python3 deploy/macos/hypermind.py docker --action logs
python3 deploy/macos/hypermind.py docker --action stop
```

The launcher uses stable project `hypermind`, initializes only when no daemon
container is running, and enforces one replica. `stop` preserves the named volume;
no volume-removal command exists. Use `--project NAME` consistently to select an
independent store. The default Compose file exposes no TCP ports. See the parent
[deployment guide](../README.md) for mTLS and capability-token configuration.

## Portable checks

```sh
python3 -m unittest discover -s deploy/macos -p 'test_*.py' -v
python3 deploy/macos/hypermind.py docker --action start --render-plan
```

These check real plist serialization, argument boundaries, private-file checks,
non-overwrite behavior, the actual OS lock, and Docker command construction.
They do not install a service or claim native macOS/Docker Desktop qualification.
