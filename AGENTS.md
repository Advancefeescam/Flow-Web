# Working on Flow Web as an AI agent

Flow Web is the browser and headless-server edition of Flow, a privacy-first YouTube and YouTube Music client. This repository also contains Flow Desktop (Tauri 2) and the Android companion. Web work reuses Flow's original React interface, Innertube extraction, streaming, SQLite schema, FlowNeuro, and MusicBrain. Preserve desktop behavior while giving browser users real equivalents where the browser cannot call an operating-system API.

## Repository map

- src/ is the React 19 and TypeScript interface shared by web and desktop. Backend calls belong in src/lib/api/ or focused hooks, global state in src/store/, and UI composition in src/components/ and src/pages/.
- web-server/ is the standalone Rust HTTP server. main.rs wires routes and state; rpc.rs dispatches an explicit command allowlist; media.rs and hls.rs expose authenticated, constrained media; db.rs opens the shared SQLite schema.
- src-tauri/src/ contains the desktop command layer and reusable extraction, streaming, recommendation, sync, and database modules. Reuse domain logic where possible; do not make the web server depend on a Tauri window or host-file dialog.
- src-tauri/migrations/ is shared by both runtimes. Existing migrations are immutable. Add a new numbered migration for schema changes; inspect the directory for the next number.
- docs/WEB.md describes the current web runtime and parity gaps. Keep it accurate when a feature or its verification status changes.
- FlowApp_mobile/ contains Android code and protocol references. Read its AGENTS.md before changing that subtree.

If Design.md exists, read it before changing React or Tailwind code. If graphify-out/graph.json exists, use graphify query for codebase questions and graphify update . after edits. Neither file is guaranteed to be present.

## Agent workflow

1. Inspect the working tree, the relevant source, and current main before editing. Fetch main when available, compare it with the working branch, and preserve all existing user changes. Do not reset or overwrite a dirty tree.
2. Turn broad requests into a concrete parity checklist from the actual code and user-visible flows. Work through it end to end. Prefer a working vertical slice with error handling and browser verification over disconnected stubs. Do not label a feature complete because a command returns a placeholder or a build succeeds.
3. Search for existing implementations before adding logic. Keep changes focused, remove dead code, and comment only when the reason is not apparent from the code.
4. Make routine implementation decisions independently. Ask only when a missing product choice or external access truly blocks the next step. Keep working on independent tasks while waiting for an answer.
5. After each meaningful slice, run its focused tests and build, then exercise the actual web route or UI. Fix failures before continuing. State exactly which behavior was observed and which behavior still needs a device or network.
6. For user-requested implementation on the current feature branch, agents may commit and push reviewed, tested changes so the PR stays reviewable. Use type(scope): summary commit messages. Never force-push, rewrite history, delete branches, merge, deploy, publish, or mark a draft PR ready without explicit user instruction. Do not push secrets or code known to be broken.

## Web parity and security

- A browser Download action must deliver a real file to the browser. Server-side storage alone does not satisfy it. Report raw direct-stream downloads separately from muxed, tagged, offline-library downloads.
- Browser file import must start from a file the user selected. Never accept arbitrary host paths through an HTTP request. Limit upload size and file type, and keep per-profile data isolated.
- Flow Sync must retain Android protocol compatibility, authenticated pairing, deterministic CRDT merges, and explicit conflict handling. Read src-tauri/tests/sync_*.rs before touching shared sync code; keep its golden and fuzz tests green.
- Profiles must isolate settings, history, downloads, recommendations, and sync state. A single-profile response is a compatibility state, not multi-profile support.
- Media relays must authenticate requests, validate session tokens, restrict remote hosts and redirect targets, bound manifest sizes, preserve range semantics, and rewrite every HLS playlist, segment, and key URL served to the browser. Do not create a caller-controlled open proxy or an SSRF path.
- Keep FLOW_WEB_PASSWORD private, require the configured browser origin for state-changing requests, and use HTTPS through a trusted proxy outside loopback. Never log passwords, auth headers, poTokens, pairing keys, or media signatures.
- Treat YouTube restrictions as typed user-facing failures: private, age-restricted, geo-blocked, paid, bot-check, unavailable, and retryable network errors. Optional SponsorBlock, DeArrow, and RYD failures must not block core playback.
- Preserve separate FlowNeuro video and MusicBrain music state. Shared watch-history queries feeding video recommendations must exclude music rows.
- Hide or explain desktop-only controls in web mode. Browser equivalents include native downloads, file pickers, Picture-in-Picture, Fullscreen, Media Session, PWA install, and WebAudio equalizer where supported.

## Verification

- Frontend: use the repository's pnpm version. From the root, run pnpm install --frozen-lockfile when dependencies change, pnpm build --mode web, and focused pnpm exec vitest run paths for changed hooks, stores, and flows.
- Web server: from web-server/, run cargo check --all-targets and cargo test. The web crate uses its own build.rs headless configuration; do not run its commands from src-tauri/.
- Desktop changes: from src-tauri/, run cargo checks and tests with its .cargo/config.toml. Run pnpm build for desktop frontend changes. Keep existing desktop features working.
- When a broad suite stalls, diagnose its process and run focused tests. Report that the broad suite was not completed; never infer a pass from silence.
- For UI changes, start the web server with separate test data and exercise first run, the normal path, an error path, and a browser download where reachable. For playback, also verify the intended Safari/iPad and ARM64 devices when they are available. A blocked external service or missing device is an unverified test, not a successful one.
- Keep the PR draft until the required playback and device checks pass. Record new parity and validation evidence in docs/WEB.md without claiming release readiness early.

## Compatibility rules

- Do not edit or renumber existing SQLite migrations. Do not bump app versions unless the owner asks.
- Use existing theme tokens, accessible labels, focus states, and getString localization for interface changes. Avoid raw hard-coded colors and unrelated UI redesigns.
- Keep React components small, domain logic in hooks or services, and Rust handlers thin. Do not duplicate the extractor or create a new HTTP client per request.
- Backend failures should reach the UI as structured error kinds. Optimistic state changes must roll back when persistence fails.
- Update documentation when behavior materially changes. Keep design notes factual and distinguish implemented, tested, and pending work.
