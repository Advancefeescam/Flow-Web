# Vendored Flow Desktop source

The generated web tree contains the original shared Rust source, resources, SQLite migrations, sidecar scripts and tests in `flow-desktop/`. The full desktop source remains on the `integration/desktop` branch.

The root `src-tauri` entry is a symbolic link to `vendor/flow-desktop`, retained for existing Rust paths, frontend references and SQLx migrations. It is not a second copy of the source. Clone on Linux/WSL or enable Git symlink support on platforms that support it. Container builds use the vendor path explicitly.

Android/iOS launcher assets and Linux native packaging metadata are not exported. Remaining native code is retained conservatively until dependency audits and build tests justify further removal.

Do not edit this generated subtree directly. Make source changes on the integration branch and regenerate the web export. Upstream attribution and the root GPL-3.0 license apply.
