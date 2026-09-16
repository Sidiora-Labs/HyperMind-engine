# Protocol v3 and remote surfaces

When to use: implement a capability-scoped client that preserves receipts, idempotency, evidence, and mutation uncertainty.

Do not use: unchecked buffers, plaintext remote transport, role mixing, or fresh sequence numbers to hide uncertain appends.

## Local protocol

Unix frames have an eight-byte prefix (little-endian payload length and CRC32C), then an `NCPR` FlatBuffer `WireEnvelope`. `Hello` carries version, stable connection ID, and capability; `Welcome` reports actor and durable next sequence. Version 3 is current; supported version-2 envelopes remain compatible. Additive members retain numeric discriminants.

Requests carry IDs. Append carries a sequence and bounded event batch with matching metadata. Exact replay returns the original receipt; changed content under the same sequence conflicts. Preserve stable error codes and `effect_state`.

## gRPC

`hypermind.v3.HyperMind` exposes `Connect`, `Exchange`, and streaming `Subscribe`. `Envelope.ncpr` holds an NCPR envelope **without** Unix length/CRC framing. `ExchangeRequest` includes Hello and request bytes. Preserve connection identity across RPCs. Rust clients use a connected transport channel plus `HyperMindClient::new`, retaining the RPC name `Connect`.

`Subscribe` first returns an acknowledgement, then unchanged NCPR Event envelopes. Resume after the last handled LSN. Bounded-queue saturation terminates instead of silently dropping events. Transport errors retain dispatch uncertainty; application errors stay in `ErrorDetail`.

## REST

Actor tools use `POST /v1/VERB` for the same [14 verbs](generated/tools.md), mutual TLS, and `Authorization: Bearer TOKEN` containing a 32-byte capability encoded as 64 hex characters.

```json
{"connection_id":"0123456789abcdef0123456789abcdef","request_id":1,"arguments":{"mode":"lexical","query":"region","limit":8}}
```

Generate a unique stable connection ID for a real client; this example is illustrative. Results preserve the MCP envelope. Separate admin listeners serve `/v1/admin/VERB`; actor tokens are rejected. `/openapi.json` and `/openapi.yaml` expose the source-generated description on TLS listeners requiring client authentication.

The [schema catalog](generated/schema-types.md) contains every canonical wire type. [Schematics](schematics.md) covers durable frames and activation bytes; remote wrappers do not change those contracts.
