# Configuration

When to use: provision storage, actor identity, capabilities, queue limits, and explicitly authorized providers before opening an actor.

Do not use: checked-in credentials, admin tokens as actor tokens, unrestricted file permissions, or provider settings that transmit private context without approval.

`hm init --path DIRECTORY` creates random keys/capabilities in `hypermind.conf`. The loader requires an owner-only regular file, rejects unknown keys, duplicate actors/tokens, zero capabilities, and actor/admin token reuse. Prefer absolute deployment paths.

| Key | Meaning | Default or constraint |
| --- | --- | --- |
| `socket` | Unix-socket path | Required |
| `data` | Actor data root | Required |
| `user` | User identity | Required, 16-byte hex |
| `kek` | Key-encryption key | Required, secret 32-byte hex |
| `admin_token` | Admin capability | Required, nonzero secret 32-byte hex |
| `actor` | `ACTOR_ID:TOKEN` | One or more; nonzero u16 ID, unique 32-byte token |
| `maximum_connections` | Connection cap | 128; range 1–4096 |
| `maximum_output_frames` | Queued-frame cap | 256; range 1–4096 |
| `maximum_output_bytes` | Queued-byte cap | 64 MiB; range 1 MiB–1 GiB |
| `projection_map_bytes` | LMDB map capacity | 256 MiB; minimum 1 MiB |

Remote listener/certificate paths are CLI options, not config-file keys. See [deployment](../guides/deployment.md).

For platforms that inject secret values rather than files, `hm serve --tls-from-env` reads complete PEM values from `HM_TLS_CERT_PEM`, `HM_TLS_KEY_PEM`, and `HM_TLS_CLIENT_CA_PEM`. All three are required, must be nonempty, and are each limited to 1 MiB. This option requires a remote listener and cannot be mixed with the three TLS file options. It does not disable certificate verification or replace client capabilities. Keep these runtime secrets out of image build arguments and checked-in manifests.

`HM_EMBEDDING_PROVIDER=centra` enables the Centra embedding runtime with `CENTRA_GATEWAY_API_KEY`. `CENTRA_GATEWAY_URL` selects the base URL, defaulting to `https://gateway.centra.ag/v1`. Current embedding route: `openrouter/openai/text-embedding-3-large`, 3072 dimensions, batch maximum 16. Without this setting the local path does not claim semantic embeddings.

`HM_RECONSTRUCTION_PROVIDER=centra` separately opts into reconstruction through the same gateway URL and exact route `openrouter/openai/gpt-5.6-luna`. Its small output budget uses no reasoning; benchmark reader/judge settings remain separate. Reconstruction stays assistant-generated and must not be remembered verbatim. `HYPERMIND_MODEL_CACHE` selects the local NLI cache.

`HM_CONSOLIDATION_PROVIDER=centra` separately enables provider-backed consolidation through `CENTRA_GATEWAY_URL` and `CENTRA_GATEWAY_API_KEY`, using the exact route `openrouter/openai/gpt-5.6-luna`. A key alone does not activate this path. A consolidation run has explicit limits for LLM calls, tokens, micro-USD, and wall time (`max_llm_calls`, `max_tokens`, `max_microusd`, `max_wall_ms` in the MCP budget). Set these within the authorized spend/data-transfer scope; enabling the runtime is not authorization for an unbounded run. These controls do not change independent benchmark settings or turn derived memories into observed evidence.

Benchmark reader/judge routing is `openrouter/openai/gpt-5.6-luna` through `CENTRA_GATEWAY_URL`, not directly to OpenRouter. A configured key is not spend authorization. See [evaluation](../evaluation/methodology.md).
