# Operations and recovery

When to use: operate one actor owner, diagnose missing results, and recover interrupted requests without duplicate external effects.

Do not use: a second writer, manual LMDB edits, log truncation, or a new request identity to bypass an unknown mutation result.

Run `hm doctor --config .hypermind/hypermind.conf --json` for checks implemented by the installed CLI. Use `inspect` for ledger/projection positions, integrity, attention, and calibration. Confirm the selected actor. A socket file alone does not establish readiness; health does not prove an external effect.

Keep a stable connection identity and monotonically increasing client sequence for idempotent append. Exact replay returns the original receipt. After a lost acknowledgement, recover the sequence from the handshake and reconcile observed outcomes before repeating an external action. `unknown` means neither failure nor permission to repeat.

For missing memory, check actor, authority, filters, active generation, retractions/fading, projection position, encoder identity, and gaps. Change retrieval thresholds only with measured evaluation, not to fit one example.

Stop the owner before a simple filesystem backup of actor data and configuration. Protect keys, capabilities, decrypted exports, and provider caches. Retain externally trusted checkpoint public keys/checkpoints for rollback detection. Restore into an isolated directory and verify before reconnecting clients.

Crypto-shred deletes keys, not ordinary relevance. Receipts cannot erase external backups/provider copies. Read [sealing](../internals/sealing.md), [recovery](../internals/recovery.md), and [security](../security/threat-model.md) before destructive administration.
