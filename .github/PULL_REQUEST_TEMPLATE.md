## Summary

<!-- What changed and why? Link relevant issues. -->

## Validation

<!-- Report actual results; leave untested items unchecked. -->

- [ ] `pnpm test`
- [ ] `pnpm build --mode web`
- [ ] `cargo check --manifest-path web-server/Cargo.toml --all-targets`
- [ ] `cargo test --manifest-path web-server/Cargo.toml`
- [ ] Browser/device smoke test where applicable
- [ ] Native desktop smoke test where shared behavior changes

Browser, OS and server architecture:

## Screenshots or recordings

<!-- For visible changes; remove sensitive details. -->

## Compatibility and security

- [ ] Web-specific changes are isolated where practical.
- [ ] No secrets, signed URLs, telemetry or broad permissions introduced.
- [ ] Documentation, migration and compatibility notes updated where needed.

<!-- List known limitations, skipped tests and follow-up work. -->
