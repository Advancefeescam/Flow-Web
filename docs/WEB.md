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
| Video search, metadata, channels, playlists, comments, explore | HTTP command adapters and web UI are connected; remote/browser verification pending |
| Video/audio streaming, range requests, DASH, SABR | Relay and browser player integration are present; Safari and device playback remain unverified |
| Live HLS | Not exposed until manifest and segment relay are hardened against SSRF |
| YouTube Music | Browse/search/queue/lyrics/stream adapters are connected; remote verification pending |
| History, preferences, library data | Shared SQLite migrations and storage adapters are connected; multi-user and recovery behavior need validation |
| SponsorBlock, DeArrow, RYD | Read adapters are connected; custom upstreams and submissions remain incomplete |
| FlowNeuro | Watch signals, onboarding, preference actions, taste overview and channel tools are connected to the headless service; full browser verification remains |
| Personalized Home and Shorts | Original ranking and feed services are reused through web RPC; actual YouTube responses and browser sessions remain unverified |
| Flow Sync and linked devices | Not yet ported to the headless runtime |
| Profiles | Server-side profile switching and isolation are not yet ported |
| Downloads, import/export, local media | Browser download and user-selected file equivalents are not yet implemented; no arbitrary host file access |
| Desktop window, tray, updater, global OS shortcuts | Native-only; web equivalents where applicable |
| Home-screen manifest | Added; offline caching and iPad testing pending |

The full frontend suite (504 files, 2,998 tests) and
`pnpm build --mode web` pass locally and in the latest web runtime workflow. Rust formatting and compilation are being checked in GitHub Actions; desktop CI has exposed a missing `StreamVariant` import that is being corrected. No complete browser playback or ARM64 deployment has been verified yet. Do not merge until the server compiles, media behavior has been reviewed, and playback has been tested on the intended devices.
