fn main() {
    // Reused desktop modules omit Tauri-only implementations in the standalone web runtime.
    println!("cargo:rustc-check-cfg=cfg(flow_web_headless)");
    println!("cargo:rustc-cfg=flow_web_headless");
}
