fn main() {
    // Reused desktop services select the shared migration directory and omit Tauri-only
    // implementations when compiled by the standalone web runtime.
    println!("cargo:rustc-check-cfg=cfg(flow_web_headless)");
    println!("cargo:rustc-cfg=flow_web_headless");
}
