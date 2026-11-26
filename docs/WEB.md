# Web runtime

Flow Web reuses the original Flow frontend, extraction, streaming and SQLite code. The web runtime is under active development. Native desktop builds remain on the integration branch.

## Sign-in and security

Open the instance in a regular browser. Sign in as `flow` with the existing `FLOW_WEB_PASSWORD` (at least 16 characters, ideally a long random secret). HTTP Basic authentication is no longer accepted for API or media requests.

In **Settings → Account & security**, configure optional TOTP using a six-digit authenticator app with a 30-second period. Start enrollment with the administrator password, copy the setup key into your authenticator, then confirm with the password and a valid code. Enrollment expires after five minutes. Enabling or disabling TOTP revokes other sessions. Codes cannot be reused, including immediately after enrollment; wait for the next code before testing sign-in. Sessions last 12 hours and are invalidated on server restart. The browser's session cookie is HttpOnly, SameSite=Strict, and Secure when `FLOW_WEB_ORIGIN` is HTTPS. Login and sensitive actions are rate limited across the instance.

This is single-administrator authentication, not separate user accounts. Password changes remain an environment configuration change followed by a restart. Passkeys, recovery codes, QR-based enrollment and isolated per-user data are not implemented. The setup key is stored in the separate private `web-auth.json` in the configured data directory; back it up securely and do not share it. A trusted server owner can recover from a lost authenticator by stopping the service, securely moving `web-auth.json` out of the data directory, and restarting with the existing password. This disables TOTP, so re-enroll immediately. Never remove the main Flow database. Use one server process per data directory.

For testing on localhost, set `FLOW_WEB_ORIGIN=http://127.0.0.1:8780`. Remote access requires a trusted HTTPS reverse proxy and the exact external HTTPS origin. Do not publicly expose this experimental build before security and media tests have passed.

## Source layout

The web export retains shared Flow Desktop source in `vendor/flow-desktop/`. Its root `src-tauri` symlink and the migration symlink require a Linux/WSL checkout with symlinks enabled. The integration branch keeps the original upstream layout.

## Development

- Install Node 24, the repository's pnpm version, stable Rust, and Linux build dependencies.
- Run `pnpm install --frozen-lockfile` and `pnpm build --mode web` from the repository root.
- Set `FLOW_WEB_PASSWORD` to a private password of at least 16 characters; set `FLOW_WEB_ORIGIN` to the exact browser origin.
- Run `cd web-server && cargo run`.
- Open http://127.0.0.1:8780. Sign in on the Flow Web login screen using username `flow` and your configured password. Use HTTPS on a trusted reverse proxy for remote access, and never publish this test server openly.

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
| Import/export, local media | Theme, playlist, equalizer, watch-history, and NewPipe subscription exports download browser files. A user-selected text/JSON import can restore supported legacy settings, subscriptions, and history. Collection-backed Flow restore, ZIP/DB import, automatic backup, and local media remain unavailable; no arbitrary host file access |
| Desktop window, tray, updater, global OS shortcuts | Native-only; web equivalents where applicable |
| PWA install and static asset cache | Manifest and service worker added; iPad install testing pending |

At the last pushed commit, GitHub Actions passed the workflow, headless runtime,
and desktop quality checks. The desktop platform matrix is skipped on this web
branch. For the current branch, `pnpm build --mode web`, 22 focused frontend
tests, and 598 web-server tests passed (6 ignored). The broad frontend suite was
stopped after it stalled, so it has no current pass result. The browser loaded the
onboarding, home, watch page, and download dialog against a local server. Remote
YouTube extraction and actual file transfer could not be checked in this environment
because the outbound proxy refused the connection. No complete playback, iPad/Safari,
container startup, or ARM64 deployment has been verified. Keep the PR draft until
media and device behavior have been checked.

## Playback transport and seeking

On the main watch player, non-live synthetic DASH can try a direct Google Video CDN manifest. The same-origin authenticated proxy manifest stays available with identical representation IDs; a direct playback failure switches back at the recovery position before another extraction. The browser avoids direct attempts for five minutes after a failure (per page runtime). CORS, CDN client/URL restrictions and different client IPs may prevent direct playback. Set `FLOW_WEB_DIRECT_MEDIA=0` on the server to disable the experiment. Other playback surfaces, SABR, live HLS and downloads continue to use the existing transport. Dragging the browser's non-live seek bar previews locally and commits one seek on pointer release; touch is included. This does not download the full video or guarantee instant unbuffered seeks.

Test direct versus proxy using DevTools Network (media requests to `*.googlevideo.com` versus `/media/stream/`); compare first frame and a long unbuffered seek. Check 1080p/4K, quality changes, audio, pause, touch, pointer cancellation and fallback. Never share signed media URLs in logs. Actual CDN and Safari/iPad results are still unverified.

## Updating from Flow Desktop

See [UPSTREAM.md](UPSTREAM.md) for the integration branch and reproducible export procedure.
