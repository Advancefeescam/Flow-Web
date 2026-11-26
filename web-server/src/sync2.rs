//! Compile the original protocol and server transport in the headless runtime.
//! Membership, database application and the background service are deliberately
//! not exposed until their Tauri dependencies have a safe server implementation.
#![allow(dead_code)]
#[path = "../../src-tauri/src/sync2/error.rs"] pub mod error;
#[path = "../../src-tauri/src/sync2/curve.rs"] pub mod curve;
#[path = "../../src-tauri/src/sync2/kdf.rs"] pub mod kdf;
#[path = "../../src-tauri/src/sync2/keys.rs"] pub mod keys;
#[path = "../../src-tauri/src/sync2/sealed.rs"] pub mod sealed;
#[path = "../../src-tauri/src/sync2/envelope.rs"] pub mod envelope;
#[path = "../../src-tauri/src/sync2/epoch.rs"] pub mod epoch;
#[path = "../../src-tauri/src/sync2/roster.rs"] pub mod roster;
#[path = "../../src-tauri/src/sync2/paths.rs"] pub mod paths;
#[path = "../../src-tauri/src/sync2/http.rs"] pub mod http;
#[path = "../../src-tauri/src/sync2/mailbox.rs"] pub mod mailbox;
#[path = "../../src-tauri/src/sync2/server.rs"] pub mod server;
#[path = "../../src-tauri/src/sync2/store.rs"] pub mod store;
#[path = "../../src-tauri/src/sync2/read_budget.rs"] pub mod read_budget;
#[path = "../../src-tauri/src/sync2/wrap.rs"] pub mod wrap;
#[path = "../../src-tauri/src/sync2/payload.rs"] pub mod payload;
pub use error::{Error, Result};
fn array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    out.copy_from_slice(bytes);
    out
}
