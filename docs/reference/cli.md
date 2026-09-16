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
| `import --config PATH --input PATH` | Migrate a supported source; stop other destination writers. |
| `archive pack/verify` | Package the native transfer stream with a signed manifest over an owned-member allowlist; verification needs no decryption key and accepts a pinned public key. |
| `verify ACTOR_DIRECTORY ACTOR CHECKPOINT PUBLIC_KEY` | Offline sealed-log proof; pin the public key independently. |

Provider-dependent operations may be unavailable without a configured runtime. Packaging commands are documented after their parser exists; source presence is not a release claim.
