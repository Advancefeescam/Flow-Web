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
| Live HLS | Authenticated playlist, segment, and key relay is present with remote host and redirect restrictions; live playback and Safari/iPad verification pending |
| YouTube Music | Browse/search/queue/lyrics/stream adapters are connected; remote verification pending |
| History, preferences, library data | Shared SQLite migrations and storage adapters are connected; multi-user and recovery behavior need validation |
| SponsorBlock, DeArrow, RYD | Read adapters are connected; custom upstreams and submissions remain incomplete |
| FlowNeuro | Watch signals, onboarding, preference actions, taste overview and channel tools are connected to the headless service; full browser verification remains |
| Personalized Home and Shorts | Original ranking and feed services are reused through web RPC; actual YouTube responses and browser sessions remain unverified |
| Flow Sync and linked devices | Not yet ported to the headless runtime |
| Profiles | The default profile has a compatibility view; creating, switching, and isolating multiple profiles are not yet ported |
| Downloads | The browser can request a real file for an available direct video/audio stream. Adaptive muxing, tags, an offline library, and an actual remote stream test are pending |
| User subtitle files | The player can read a chosen `.srt` or `.vtt` file in browser memory; select it again after reloading the page |
| Import/export, local media | Theme, playlist, equalizer, watch-history, and NewPipe subscription exports have browser file paths where implemented. Full backup export/restore, other-app import, and local media need browser equivalents; no arbitrary host file access |
| Desktop window, tray, updater, global OS shortcuts | Native-only; web equivalents where applicable |
| PWA install and static asset cache | Manifest and service worker added; iPad install testing pending |

At the last pushed commit, GitHub Actions passed the workflow, headless runtime,
and desktop quality checks. The desktop platform matrix is skipped on this web
branch. For the current branch, `pnpm build --mode web`, 15 focused frontend
tests, and 598 web-server tests passed (6 ignored). The broad frontend suite was
stopped after it stalled, so it has no current pass result. The browser loaded the
onboarding, home, watch page, and download dialog against a local server. Remote
YouTube extraction and actual file transfer could not be checked in this environment
because the outbound proxy refused the connection. No complete playback, iPad/Safari,
container startup, or ARM64 deployment has been verified. Keep the PR draft until
media and device behavior have been checked.
