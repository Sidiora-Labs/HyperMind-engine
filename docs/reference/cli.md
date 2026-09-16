# Command-line interface

When to use: initialize/inspect a deployment, issue explicit memory operations, manage consolidation, or perform controlled offline migration/verification.

Do not use: embedded commands against a live owner, destructive operations without authority, or roadmap commands as if already installed.

`hm --help` and `hm COMMAND --help` describe the installed parser. `--json` is global. [Generated declarations](generated/cli-declarations.md) reproduce current command/options source and fail the docs gate on drift.

| Command | Use and boundary |
| --- | --- |
| `init --path PATH [--actor ID]` | Create private storage/configuration; no silent overwrite. |
| `serve --config PATH` | Own storage and serve clients; remote listeners require mTLS. |
| `doctor --config PATH` | Inspect implemented health checks, not release qualification. |
| `remember --config PATH --conversation ID --content TEXT` | Append a message; `--embedded` owns storage directly. |
| `recall --config PATH --query TEXT` | Retrieve using the selected CLI mode; MCP exposes richer filters. |
| `activate --config PATH --conversation ID --budget-tokens N` | Produce bounded context; retain health and provenance. |
| `consolidate run/list/retract` | Budget, inspect, or retract generations; CLI opens the actor directly. |
| `import --config PATH --input PATH` | Migrate a supported source; stop other destination writers. The input may be a native archive as well as a raw event stream. |
| `archive pack/unpack/verify` | Package the native transfer stream with a signed manifest over an owned-member allowlist; verification needs no decryption key and accepts a pinned public key. `unpack --input FILE --output DIR` verifies the whole archive first, writes only owned members into an existing directory, and never overwrites a file that is already there. |
| `verify ACTOR_DIRECTORY ACTOR CHECKPOINT PUBLIC_KEY` | Offline sealed-log proof; pin the public key independently. |

`serve --telemetry-file PATH` is the only way to turn span export on; there is no environment variable, no configuration key, and no default that enables it. When the flag is present the daemon appends one OpenTelemetry OTLP/JSON record per line to that owner-only file, optionally labelled by `--telemetry-service NAME`, and flushes it before the listeners stop. Span attributes are metadata only by construction: keys and string values are fixed compile-time constants, so no memory content, query text, conversation identifier, capability token, model name, or filesystem path can appear in an exported span.

Provider-dependent operations may be unavailable without a configured runtime. Packaging commands are documented after their parser exists; source presence is not a release claim.
