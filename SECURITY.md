# Security policy

HyperMind stores sensitive application memory and encryption keys. Read the [threat model](docs/security/threat-model.md) and [deployment contract](deploy/README.md) before using it with private data or remote access.

## Version status

This repository is in active development; v1.0.0 is not a qualified release. There is no published long-term-support or backport commitment here. Include the exact commit or image digest in a report. A passing local test or benchmark is not a security certification.

## Report privately

Do not put vulnerability details, exploit payloads, credentials, or affected memory in public issues, pull requests, logs, or discussion threads.

Open the repository's [Security / Advisories page](https://github.com/Sidiora-Labs/HyperMind-engine/security/advisories). If **Report a vulnerability** is available, use it to submit a private report. This policy does not claim that private reporting is enabled. If it is unavailable and no private contact is published, open an issue asking only for a private security contact; omit technical details until a private channel is agreed. This follows [GitHub's reporting guidance](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

Include privately:

- The affected commit, component, deployment mode, and operating system.
- Expected behavior, actual behavior, and the security impact.
- Minimal reproduction steps using your own test actor and non-sensitive data.
- Whether authentication, a capability, filesystem access, or a provider is required.
- Relevant redacted errors, and any mitigation you have verified.

Do not send real keys or another person's memory as proof. Keep reports limited to information needed to reproduce the issue. Test only systems and data you are authorized to access; stop if testing could expose other actors' data or damage a live store.

The project has no stated response deadline, guaranteed fix date, or bounty program. Coordinate disclosure and any public advisory through the agreed private channel; do not interpret an unanswered report as permission to attack a deployment.

## Relevant security boundaries

Reports of cross-actor access, capability bypass, authority laundering, unsafe activation rendering, tamper-verification bypass, key disclosure, or incorrect crypto-shred guarantees are especially relevant. Remote gRPC and REST require both mutual TLS and capabilities; an edge proxy terminating TLS is not an equivalent replacement for client-certificate checks.

The OS account, kernel, process memory, key-encryption key, and trusted checkpoint key are inside the current trust boundary. Ledger encryption does not mean projections, exports, logs, SDK buffers, or backups are encrypted. Crypto-shredding cannot erase plaintext or keys copied elsewhere. These limitations are not permission to leak data; report behavior that violates the documented boundary or claims.

## Operational precautions

Protect the entire state directory and keep a single writer. Keep config files, capability tokens, TLS keys, provider keys, and private responses out of Git, container images, build arguments, and public diagnostics. Use production PKI for production; the included certificate helper is for development only.

Enabling a remote provider sends selected content outside the process and can incur charges. Configure explicit opt-ins and budgets only with authorization. Recalled text remains untrusted content even when transported securely.

If a credential is exposed, restrict access and replace it through the relevant issuer or supported administration path. Preserve an encrypted, access-controlled incident copy if needed; do not destroy the only recoverable state while investigating. Key rotation and crypto-shred are different operations, and backups require their own retention controls.
