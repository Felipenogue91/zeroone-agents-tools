// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = claude_code_tool_manager::run_cli(&args) {
        std::process::exit(code);
    }
    // WebKitGTK's DMABUF renderer crashes inside the NVIDIA driver
    // (libnvidia-eglcore) as soon as a page draws WebGL, e.g. the Code Forest
    // tab. The fallback renderer keeps GPU acceleration and does not crash.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    claude_code_tool_manager::run()
}
