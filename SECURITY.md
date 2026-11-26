# Security policy

Flow Web is an experimental, independently maintained port of Flow Desktop. There is no stable web release or guaranteed security support window yet.

## Reporting vulnerabilities

For vulnerabilities specific to the web runtime, use [private vulnerability reporting for Flow Web](https://github.com/Advancefeescam/Flow-Web/security/advisories/new) when enabled. If it is unavailable, contact the repository maintainer [Advancefeescam](https://github.com/Advancefeescam) to arrange a private reporting channel. Do not publish exploit details in issues, pull requests or discussions.

Include the affected commit, server OS and architecture, browser version, reproduction steps, impact and possible mitigations. Redact passwords, cookies, signed media URLs, authentication headers and personal data.

For vulnerabilities in the original native application, follow [Flow Desktop's security policy](https://github.com/Flow-Tube/Flow-Desktop/security/policy).

## Deployment

Use a strong private password, an exact configured origin, HTTPS for remote access, and a separate data volume. Keep the experimental runtime on a trusted network or behind a trusted reverse proxy; do not expose it directly to the public internet. The current runtime is not designed for multiple untrusted users.
