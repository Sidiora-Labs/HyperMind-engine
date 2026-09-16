# Operations and recovery

When to use: operate one actor owner, diagnose missing results, and recover interrupted requests without duplicate external effects.

Do not use: a second writer, manual LMDB edits, log truncation, or a new request identity to bypass an unknown mutation result.

Run `hm doctor --config .hypermind/hypermind.conf --json` for checks implemented by the installed CLI. Use `inspect` for ledger/projection positions, integrity, attention, and calibration. Confirm the selected actor. A socket file alone does not establish readiness; health does not prove an external effect.

Keep a stable connection identity and monotonically increasing client sequence for idempotent append. Exact replay returns the original receipt. After a lost acknowledgement, recover the sequence from the handshake and reconcile observed outcomes before repeating an external action. `unknown` means neither failure nor permission to repeat.

For missing memory, check actor, authority, filters, active generation, retractions/fading, projection position, encoder identity, and gaps. Change retrieval thresholds only with measured evaluation, not to fit one example.

Stop the owner before a simple filesystem backup of actor data and configuration. Protect keys, capabilities, decrypted exports, and provider caches. Retain externally trusted checkpoint public keys/checkpoints for rollback detection. Restore into an isolated directory and verify before reconnecting clients.

Crypto-shred deletes keys, not ordinary relevance. Receipts cannot erase external backups/provider copies. Read [sealing](../internals/sealing.md), [recovery](../internals/recovery.md), and [security](../security/threat-model.md) before destructive administration.

## Authorized web sources

Web ingestion is off until an operator names the hosts it may reach. `HM_WEB_SOURCE_HOSTS` is a comma-separated allowlist matched case-insensitively against the exact host of every request, with no suffix matching; when it is unset or empty there is no web-source surface at all and the attempt reports `kOperationUnavailable`. `HM_WEB_SOURCE_MAX_BYTES` bounds a single response (default 2097152, hard cap 8388608, applied to the declared content length and again to the bytes actually read). `HM_WEB_SOURCE_MAX_REDIRECTS` bounds the redirect chain (default 3, hard cap 8). `HM_WEB_SOURCE_MIN_INTERVAL_MS` is the minimum wall-clock gap between two requests to the same host (default 1000). `HM_WEB_SOURCE_TIMEOUT_MS` is the per-request timeout (default 15000). `HM_WEB_SOURCE_CROSS_HOST_REDIRECT` accepts `1` or `true` to permit a redirect that changes host; it is off by default, so a redirect off the original host is refused. An unparsable numeric value is rejected as `kInvalidArgument` rather than silently replaced.

Redirects are not followed by the HTTP client. Each `Location` is resolved against the current URL and re-validated against the full policy, so a hop into an unlisted host, a non-`http(s)` scheme, a URL carrying credentials, or a private address is refused instead of followed. Before every request the destination host is resolved and each returned address is checked; unspecified, loopback, private, shared, link-local, benchmarking, documentation and broadcast IPv4 ranges and unique-local, link-local and IPv4-mapped IPv6 forms are refused. The host allowlist alone does not stop DNS rebinding, because a name may resolve differently between two requests; the per-request address check is what closes that window, and it narrows rather than eliminates it. Treat an allowlisted host as trusted infrastructure, not as a sandbox.
