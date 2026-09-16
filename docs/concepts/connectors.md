# Source connectors

When to use: let an external source — a repository host, a workspace host — push signed change notifications into an actor's ledger, or pull revision heads from one, under an authorization the user granted explicitly and can revoke.

Do not use: as a credential store for unrelated services, as a way to make a provider's payload count as something the user asserted, or as a scheduler; nothing on this path retries by itself.

A source connector is a durable binding between one actor and one external source provider. It is identified by a sixteen-byte `connector_id` together with a provider string, and like every other authority in HyperMind it exists only as ledger records. There is no connector table, no side file of bindings, and no way to reach a connected source without an append that admission already accepted. Four records carry the whole lifecycle, and one projection, `source_connectors`, replays them into the state that admission and the surfaces read.

## Four records and their pinned authority

`SourceConnectorBound` is the binding itself: the connector id, the provider, the external account, the redeemed consent nonce and its expiry, the active credential version, the signature scheme, the granted scopes, and a state of `bound` or `revoked`. It must be `user_asserted`. Only a person can bind, rotate or revoke a connector.

`SourceDeliveryAccepted` records that a signed delivery verified: its connector, its `delivery_id`, the scheme and credential version that verified it, the `signed_at_ns` the sender signed, the blake3 digest of the body, the body length, and the provider's own event name. It must be `external_observed`, because a delivery is an artefact an outside system produced.

`SourceDeliverySettled` records what the caller did with an accepted delivery: the accepted LSN, the attempt number, a state of `applied`, `failed` or `abandoned`, the earliest instant a further attempt is due, and a short detail string. It must be `runtime_fact`. It is the engine's own bookkeeping, not a claim by anyone.

`SourceRevisionObserved` records a revision head: the connector, the `source_id`, the revision, the content digest, and when it was observed. It must be `external_observed`.

Authority is pinned per kind in envelope validation, so nothing else is admissible. A record carrying the wrong class is refused with `kProtectedTypeWrite` before it reaches the log, which means no consolidation run, no model output and no derived record can mint a connector, admit a delivery, or move a revision head. None of the four is model-derived, none requires a run id, and none carries model provenance.

## Consent is a single-use grant

Binding is two calls, not one, so the act of granting is itself recorded. `bind` with `action: "consent"` mints a state string for one provider and one connector id and returns it: a sixteen-byte nonce, an absolute expiry, and a tag computed under a consent key derived from the actor's own data key. The secret never leaves the writer thread; the caller receives only the encoded state.

`bind` with `action: "bind"` hands that state back. The tag is verified before any field in the state is believed, the expiry is checked against the actor clock, and the nonce is then carried into the `SourceConnectorBound` record. Admission writes redeemed nonces into the registry and refuses a second bind carrying a nonce that is already there with `kIdempotencyConflict`, so a captured state string cannot bind a second connector. An expired or unrecognised state is `kCapabilityDenied`.

The state is bound to the provider and connector it was minted for. Replaying it against a different provider or a different connector id fails the tag check, because both are in the signed material.

## Credentials are versioned and rotatable

A connector's signing secret lives in the credential vault under the actor's existing key hierarchy — no second key system. Each version is sealed with XChaCha20-Poly1305 under the data key, with associated data binding the credential domain, the actor, the user, the version, the provider and the connector id, so a secret sealed for one connector cannot be opened as another. Versions are separate files under the actor directory, written temp-then-rename and fsynced, and storing a version that already exists is refused with `kAlreadyExists` rather than overwriting a secret in place.

`bind` with `action: "rotate"` stores the next version and appends a `SourceConnectorBound` naming it while the connector stays `bound`. Older versions remain openable, so deliveries signed with the version a sender has not yet stopped using still verify. `bind` with `action: "revoke"` appends a `SourceConnectorBound` with state `revoked`; from that append on, admission refuses a delivery or a revision for that connector with `kCapabilityDenied`.

Secrets never cross the actor command channel. The writer loads a version, uses it, and drops it; the surfaces receive verification results, never key material.

## Signed delivery admission

HyperMind defines its own signature envelope rather than implementing any provider's. Scheme `hmac_sha256_v0` is HMAC-SHA256 over the domain string `hypermind.source-delivery.v0`, a zero byte, the connector id, a zero byte, the delivery id, a zero byte, `signed_at_ns` as a little-endian signed 64-bit integer, a zero byte, and the body. The tag is compared in constant time, and the signed timestamp must fall inside a freshness window of three hundred seconds around the actor clock. An edge adapter maps a particular provider's header set into this envelope; a provider-specific scheme would be a further member of `SourceSignatureScheme`, not a change to this one.

A tag mismatch is `kSignatureInvalid` and a timestamp outside the window is `kOrderingViolation`. Both are refusals, and both append nothing.

`remember` with `source_delivery` admits a verified delivery as two events in one batch: the `SourceDeliveryAccepted` record, and a `ProviderFrame` carrying the verified body itself. That second append is what keeps the content addressable — the delivery is not a bare digest in the log, it is the bytes the provider sent, stored as externally observed evidence with its own LSN. The reply names both LSNs.

There is no network on this path. The body arrives as an argument that the caller already holds, so admission is a verification, never a fetch.

## The durable inbox

A delivery's idempotency key is its connector id together with its `delivery_id`. A repeat is not an error: the tool reads the registry first and returns the earlier `accepted_lsn` with `duplicate` set, and the writer's own precondition refuses a genuine second append of the same key with `kIdempotencyConflict`, including within a single batch. That makes a sender's at-least-once retry safe without a second ledger record.

`remember` with `source_settlement` closes an attempt. The attempt number must advance — a settlement at or below the attempt already recorded is `kOrderingViolation` — and settling a delivery that was never accepted is refused the same way. A `failed` settlement computes the next attempt instant from a bounded schedule: one second for the first retry, doubling per attempt, capped at sixty seconds. Past five attempts the state becomes `abandoned` regardless of what the caller reported, and `next_attempt_at_ns` is zero.

The schedule is advice the engine records, not a timer it runs. Nothing in HyperMind wakes up to retry a delivery; the record tells a caller when it may try again and when it should stop.

## Revision heads

`remember` with `source_sync` is the pull half. It refuses with `kOperationUnavailable` unless a source runtime is configured, so a daemon that was never given a transport cannot reach the network at all. It then checks that the named connector is bound, lists revisions through the outbound seam, and appends one `SourceRevisionObserved` per source whose revision differs from the stored head. A source whose revision already matches is reported as unchanged and appends nothing, so a sync that finds nothing new is a read.

The listing contract is HyperMind's own: a JSON object with a `sources` array of `source_id`, `revision` and `content_digest`, and an optional `next_cursor`. Every field is validated defensively, paging is bounded, and a repeated cursor is `kSchemaInvalid` rather than a loop.

## Reading connectors back

Three `inspect` resources read the registry, each returning provenance URIs for the records it reports. `hm://{actor}/connectors` lists the bindings with their state, active credential version, scopes and bound LSN. `hm://{actor}/inbox` lists accepted deliveries newest first with their state, attempt, next attempt instant, body digest and settled LSN. `hm://{actor}/revisions` lists the current revision head per source.

None of these is a live view of the provider. They report what the ledger holds, which is what verified, not what is true at the far end right now.

## Errors

The whole surface reuses existing error codes. Signature mismatch is `kSignatureInvalid`; a stale or future timestamp is `kOrderingViolation`; a duplicate delivery id or a redeemed consent nonce is `kIdempotencyConflict`; an unbound connector, a revoked one, or an expired consent is `kCapabilityDenied`; an unknown credential version is `kInvalidArgument`; re-storing an existing version is `kAlreadyExists`; a malformed payload is `kSchemaInvalid`. No new code was added, so the numeric wire contract and its generated SDK mirrors are unchanged.

## Stated limitations

Two things on this path are not verified here, and this page says so rather than implying otherwise.

The checked-in Go and TypeScript FlatBuffers event trees were not regenerated for the four new records. The canonical schema and the Rust bindings carry them; those two language trees still describe the earlier union and must be regenerated before either is used to read a ledger containing source records.

The HTTP source transport has no test in this environment, because testing it would require a live service. Everything reachable without a network — signature verification, consent, the vault, admission, settlement, the listing parser and the sync driver — is exercised against a recorded transport that asserts exact request equality. The live transport's behaviour against a real host is unqualified.
