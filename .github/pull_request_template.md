## Change and scope

Describe the behavior changed, the motivating issue, and any relevant `spec/hypermind-01/spec.kvx` task or acceptance criteria. List remaining work explicitly.

## Verification

List exact commands, environments, and observed results. Distinguish local tests from hosted CI and real runtime tests from configuration rendering. Record failures, skipped checks, and untested platforms; do not mark an unmet gate complete.

## Compatibility and safety

Describe any wire/schema, persisted-state, SDK, migration, actor-isolation, authority/provenance, or provider-cost impact. State “not applicable” where appropriate. Call out protected-path changes for human review and explain rollback or data recovery for operational changes.

## Review checklist

- [ ] The change stays within the requested scope, and unrelated edits are excluded.
- [ ] Tests exercise real code paths; original donor vectors and fixtures remain byte-for-byte intact.
- [ ] Generated files were regenerated with their source/tooling or left untouched.
- [ ] Documentation and public contracts match the actual implementation and qualification status.
- [ ] No credentials, private keys, capability tokens, private memory, or sensitive configuration are included.

For vulnerabilities, use the [security policy](https://github.com/Sidiora-Labs/HyperMind-engine/security/policy) instead of a public issue or pull request containing exploit details.
