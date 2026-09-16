# Authority and evidence

Every event and surfaced item carries source authority. It describes what produced a statement, not universal truth.

| Authority | Interpretation |
| --- | --- |
| `user_asserted` | The user supplied this claim or authorized instruction. |
| `external_observed` | A recorded external source supplied these bytes. |
| `tool_observed` | The linked tool actually returned this result. |
| `runtime_fact` | The runtime observed this transition. |
| `assistant_generated` | An assistant generated this text. |
| `derived_inference` | This conclusion is derived from cited sources. |

An `Outcome` or done `LoopClosed` must cite observed/runtime evidence LSNs. Memory, summaries, and reconstruction cannot establish that a file was written or a deployment happened. Even observations require scope, freshness, identity, and integrity checks.

Identity, preference, and constraint beliefs are protected. Consolidation can propose them, not silently replace user authority. Negative-existence assertions require corroborating tool-observed evidence; absence from retrieval proves nothing about external existence.

Rendering retains authority and `hm://` provenance. Memory stays in labeled untrusted user context, never system/developer instructions. Supported procedures remain observations until user adoption. Reconstruction is labeled `RECONSTRUCTION` and cannot be remembered verbatim as an observation.

When to use: inspect authority and cited sources before relying on memory or closing a loop.

Do not use: labels to authorize new actions, bypass external verification, or promote model narrative into proof. See [the threat model](../security/threat-model.md).
