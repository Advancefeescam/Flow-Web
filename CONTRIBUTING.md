# Contributing to Flow Web

Flow Web is an unofficial browser port of [Flow Desktop](https://github.com/Flow-Tube/Flow-Desktop). Contributions are welcome, especially browser compatibility, media delivery, deployment, documentation and tests.

## Getting started

The web runtime is currently developed on [`feat/web-runtime`](https://github.com/Advancefeescam/Flow-Web/tree/feat/web-runtime). See [docs/WEB.md](https://github.com/Advancefeescam/Flow-Web/blob/feat/web-runtime/docs/WEB.md) for setup, the parity tracker and upstream update instructions.

For web changes, branch from the active web branch and target it in your pull request until PR #1 is merged. For changes confined to the default branch's documentation, target main. Keep changes focused and search existing pull requests and issues first.

## Design and engineering

- Reuse the shared React/TypeScript UI and Rust services; follow existing Flow design tokens, localization and component conventions.
- Isolate web-specific behavior in `web-server/` and narrow frontend transport/player adapters. Avoid broad changes to native Tauri behavior.
- Validate untrusted input and network destinations. Never expose arbitrary host files, credentials, or unrestricted media proxy endpoints.
- Keep upstream synchronization separate from web features so future Flow updates remain manageable.
- Preserve the original license, copyright and attribution.

## Validation

From the active web branch, with the prerequisites in the web runtime guide:

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
