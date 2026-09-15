# Merkle Mountain Range

HyperMind commits to ciphertext so an auditor can verify ledger integrity without decryption keys. For each durable frame, the leaf hash covers a frame domain byte, LSN, event kind, timestamp, actor, conversation, sealed-payload length, and sealed payload. It therefore commits to both ordering metadata and the exact ciphertext.

## Nodes and roots

Adjacent equal-height nodes are combined with a distinct node domain. The current peaks are folded into a root hash with a separate root domain and the total leaf count. These domains prevent a leaf, internal node, and root from being interpreted as one another. The append receipt returns leaf count, leaf hash, and root.

The persisted MMR store records leaves and internal nodes. A range proof supplies only boundary nodes outside the requested interval. Verification rebuilds the covered subtrees, rejects malformed, overlapping, unused, or out-of-bounds boundary nodes, reconstructs the peaks, and requires the calculated root to equal the proof's expected root.

## Checkpoints and offline verification

A checkpoint records actor, LSN, leaf count, root, Ed25519 public key, and signature. The signature covers a versioned domain plus actor, LSN, leaf count, and root. Checkpoint files also carry a CRC for storage corruption detection. The verifier scans frame boundaries, hashes the stored sealed bytes, rebuilds the MMR, checks the requested prefix and root, then verifies the checkpoint signature against the trusted public key. It does not need the sealing key.

The public key must be pinned outside the actor directory for meaningful authenticity. A locally replaced checkpoint and locally replaced key are otherwise indistinguishable from a new history.

## Recovery

On open, HyperMind compares persisted MMR state with complete ledger frames. Missing trailing nodes can be rebuilt from sealed frames. A bounded repair may truncate only an incomplete tail; a mismatch inside a complete prefix is interior corruption and is refused. Repair never invents plaintext or rewrites a valid frame, and the recovered root must match every retained checkpoint for its prefix.
