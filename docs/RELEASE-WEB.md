# Flow Web release gates

Do not tag a stable release until the gates below have evidence from a clean build and real devices. The first target is a private alpha/beta, not a claim of full desktop parity.

| Gate | Required evidence |
| --- | --- |
| Build | Clean locked pnpm and Cargo builds, full frontend and server test suites, no unexpected CI failures |
| Deploy | Linux amd64 and arm64 container smoke tests; persistent volume, restart, non-root runtime; actual Podman test on Miatoll |
| Authentication | HTTPS reverse proxy, session renewal/expiry, TOTP/recovery, logout and no secret leakage |
| Playback | Firefox, Chromium and real Safari/iPad; video/audio, direct-to-relay fallback, live, 4K, seek, quality changes, subtitles, fullscreen and PiP |
| Mobile UI | Watch/Shorts sheets, navigation, touch, theme contrast (light/dark/AMOLED/custom), landscape, safe areas, music queue and PWA |
| Flow Sync | Stable server-side identity and key storage, original FLOW-SYNC/2 pairing, encrypted two-way collection merge, conflict/recovery tests, restart and Android/Desktop interoperability |
| Library | Subscription/playlist/history edits and imports/exports; download of actual media; verify all advertised features and hide or label unsupported features |
| Updates | Upstream merge and export, migration and backup/restore test, rollback guide; no manual build-time patches |

## Release packaging

Build and smoke-test the container on both architectures with the **Web container smoke tests** workflow before publication. Publish only tested immutable version tags with matching source revision and release notes; do not promote `latest` before the release gate. Keep a persistent data volume, backup before upgrades, and never share it with Flow Desktop or Flow Sync.

## Current limitations

The current web server is a single-administrator instance. Flow Sync currently provides a read-only server readiness check, not pairing or collection sync. Physical Safari/iPad, ARM64 hosting and complete feature parity have not been established. CI and device results must be recorded before changing those statements.
