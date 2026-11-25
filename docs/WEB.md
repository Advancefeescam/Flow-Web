# Experimental web runtime

This branch keeps the original Flow frontend, extraction, streaming and SQLite code. It is not yet a validated release. The desktop app remains available through the existing build path.

## Development

- Install Node 24, the repository's pnpm version, stable Rust, and Linux build dependencies.
- Run `pnpm install --frozen-lockfile` and `pnpm build --mode web` from the repository root.
- Set `FLOW_WEB_PASSWORD` to a private password of at least 16 characters; set `FLOW_WEB_ORIGIN` to the exact browser origin.
- Run `cd web-server && cargo run`.
- Open http://127.0.0.1:8780. The experimental instance uses HTTP Basic authentication. Use HTTPS on a trusted reverse proxy for remote access, and never publish this test server openly.

## Container

The Containerfile builds the web UI and headless Rust server without desktop GUI packages. It is intended for Linux amd64 and arm64, with architecture-specific native builds. Set the password and origin in your shell before running `podman compose -f compose.web.yml up --build`. The compose configuration publishes only to loopback; for iPad access configure a private HTTPS reverse proxy. Use a separate data volume from Flow Desktop and Flow Sync. Do not put credentials into Git.

## Parity tracker

| Area | Current branch |
| --- | --- |
| Video search, metadata, channels, playlists, comments, explore | HTTP command adapters implemented; runtime verification pending |
| Video/audio streaming, range requests, DASH, SABR | Relay implementation; browser/codec tests pending |
| Live HLS | Not exposed until manifest and segment relay is hardened against SSRF |
| YouTube Music | Browse/search/queue/lyrics/stream adapters implemented; verification pending |
| History, preferences, library data | Shared SQLite migrations and storage adapters; integration tests pending |
| SponsorBlock, DeArrow, RYD | Read adapters implemented; custom upstreams and submissions pending |
| FlowNeuro, homepage and personalized Shorts | Pending; web currently opens Explore |
| Flow Sync, linked playback, profiles | Pending; single private instance only |
| Downloads, import/export, local media | Pending browser/server equivalents; no arbitrary host file access |
| Desktop window, tray, updater, global OS shortcuts | Native-only; web equivalents where applicable |
| Home-screen manifest | Added; offline caching and iPad testing pending |

No complete build, playback or ARM64 deployment has been verified yet. Do not merge until tests pass and security/media behavior has been reviewed.
