# Contributing to Flow Web

Flow Web is an unofficial browser port of [Flow Desktop](https://github.com/Flow-Tube/Flow-Desktop). Contributions are welcome, especially browser compatibility, media delivery, deployment, documentation and tests.

## Getting started

See [docs/WEB.md](docs/WEB.md) for setup and [docs/UPSTREAM.md](docs/UPSTREAM.md) for the branch and export workflow.

Develop source changes against `integration/desktop`. The public `main` branch is generated from a pinned integration commit; do not make manual changes to generated files there. Keep changes focused and search existing pull requests and issues first.

## Design and engineering

- Reuse the shared React/TypeScript UI and Rust services; follow existing Flow design tokens, localization and component conventions.
- Isolate web-specific behavior in `web-server/` and narrow frontend transport/player adapters. Avoid broad changes to native Tauri behavior.
- Validate untrusted input and network destinations. Never expose arbitrary host files, credentials, or unrestricted media proxy endpoints.
- Keep upstream synchronization separate from web features so future Flow updates remain manageable.
- Preserve the original license, copyright and attribution.

## Validation

From the integration or web branch, with the prerequisites in the web runtime guide:

```sh
pnpm install --frozen-lockfile
pnpm test
pnpm build --mode web
cargo check --manifest-path web-server/Cargo.toml --all-targets
cargo test --manifest-path web-server/Cargo.toml
```

For browser/player changes, include the browser version, OS, server architecture and actual playback or interaction results. Note skipped tests and unverified behavior. Smoke-test native behavior where shared code changes. Add screenshots or recordings for visible changes, with sensitive data removed.

Report web-specific bugs in this repository when issues are enabled. For security concerns, follow [SECURITY.md](SECURITY.md). For unrelated native Flow bugs, use the [upstream repository](https://github.com/Flow-Tube/Flow-Desktop).

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md) and use clear, focused commit messages.
