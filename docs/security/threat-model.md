# Threat model

HyperMind protects an actor's durable memory from accidental cross-actor access, undetected log modification, authority laundering, and loss of confidentiality after an intentional crypto-shred. The operating-system account, kernel, process memory, configured key-encryption key, and checkpoint trust anchor remain inside the trusted computing base.

## Trust boundaries

Actor data crosses four boundaries. Clients cross the Unix-socket or remote transport boundary and must authenticate before issuing requests. Ingest crosses the evidence boundary, where source kind determines authority and a caller cannot promote model output to observed evidence. Sealed frames cross the storage boundary; projections and indexes are reproducible caches, while the append-only ledger is authoritative. Rendered activation crosses the model boundary and is always labelled user content with authority and provenance, never system or developer instructions.

Remote model and embedding providers receive only data explicitly selected for that provider. They are not part of log integrity and cannot create observed authority. Filesystem permissions and transport encryption are deployment responsibilities.

## Capabilities

Actor and admin tokens are separate capabilities. An actor connection can mutate or read only its selected actor. Admin transport operations include health, statistics, integrity verification, projection rebuild, and crypto-shred. Key rotation uses its explicitly supported administration interface, not an invented wire request. Actor requests on an admin connection and admin requests on an actor connection are rejected with `kCapabilityDenied`. Multi-actor sharing requires an explicit bridge that appends a new event with its origin actor.

Remote gRPC and REST listeners require mutual TLS with a trusted client CA as well as the appropriate capability. Certificates do not replace capability checks. Actor/admin listeners are separate, request bodies and output queues are bounded, and a saturated subscriber is disconnected. Secure transport does not make retrieved model-authored text trustworthy.

Possession of a token grants its documented operations; it does not establish the truth of submitted content. Authority is assigned from the authenticated source and admission rules.

## What verification proves

Offline verification recomputes MMR leaves from stored frame headers and sealed payload bytes, reconstructs the root, and validates an Ed25519 checkpoint against a trusted public key. A successful result proves that the inspected prefix has the same ordered sealed bytes committed by that checkpoint and that its signature is valid. Range proofs establish inclusion in a particular root.

Verification does not decrypt content, prove that the original observation was true, establish who controlled a signing key, detect compromise before a checkpoint was signed, or prove completeness beyond the checkpointed prefix. Operators must obtain and pin the public key through a separate trusted channel and retain checkpoints externally when rollback detection matters.

## Crypto-shred guarantees

Crypto-shred signs a receipt containing the actor, user, deletion LSN, data-key fingerprint, and checkpoint root, durably removes the keyring, then exposes the final receipt. Reopening the actor returns `kKeyDestroyed`. With no surviving copy of the key-encryption, user, or data keys, the sealed payloads cannot be decrypted by HyperMind.

The receipt proves that this implementation completed its deletion protocol for the named key fingerprint; it cannot erase backups, snapshots, swap, crash dumps, provider copies, plaintext previously exported by a client, or keys copied outside HyperMind. Operators must apply matching retention and destruction controls to those systems.

## Reporting vulnerabilities

Do not disclose a suspected vulnerability in a public issue. Use the repository's private GitHub security-advisory channel and include affected versions, reproduction steps, impact, and any suggested mitigation. Maintainers should acknowledge the report privately, coordinate a fix and release, and publish an advisory after users have a reasonable upgrade path. Avoid accessing data that is not yours while validating a report.
