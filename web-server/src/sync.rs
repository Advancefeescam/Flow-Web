//! Shared FLOW-SYNC/1 wire primitives used by the FLOW-SYNC/2 transport.
pub const PROTOCOL_VERSION: u8 = 1;
#[path = "../../src-tauri/src/sync/canonical.rs"] pub mod canonical;
#[path = "../../src-tauri/src/sync/crypto.rs"] pub mod crypto;
#[path = "../../src-tauri/src/sync/qr.rs"] pub mod qr;
