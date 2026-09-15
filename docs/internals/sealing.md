# Sealing and key lifecycle

Every durable event payload is sealed before it enters a segment. HyperMind refuses an actor directory without a keyring as legacy plaintext unless key creation was explicitly requested.

## Key hierarchy

The configured 256-bit key-encryption key wraps a randomly generated per-user key. The user key wraps a randomly generated per-actor data key. Both wrapping operations use XChaCha20-Poly1305. Their associated data contains a versioned domain, actor identifier, and user identifier, so wrapped keys cannot be moved between identities. The keyring is created with mode `0600`, synced, and its directory is synced before use. Decrypted keys are held in zeroizing containers.

The data key seals event payloads with XChaCha20-Poly1305. Each payload's associated data binds the actor, user, LSN, and event kind. Changing metadata, moving ciphertext to another actor or position, or modifying ciphertext causes authentication to fail.

## Rotation

Key rotation opens and authenticates the existing hierarchy, generates new wrapping material, and atomically re-wraps the same data key. Ledger segments do not change, so their MMR leaf hashes and signed checkpoints remain stable. Rotation is an admin operation; interrupted replacement must leave either the prior valid keyring or the complete replacement.

## Destruction

Crypto-shred derives the data-key fingerprint, binds it to the current checkpoint root and deletion LSN, and signs a deletion receipt. It writes and syncs a pending receipt, unlinks the keyring, syncs the key directory, promotes the receipt, and syncs again. The in-memory key hierarchy is then dropped and zeroized. Either a pending or final deletion receipt prevents key recreation and makes later opens return `kKeyDestroyed`.

This operation destroys HyperMind's key material, not copies made by the operating system, backups, clients, or external providers. Those require independent deletion controls.
