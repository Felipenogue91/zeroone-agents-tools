// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = claude_code_tool_manager::run_cli(&args) {
        std::process::exit(code);
    }
    claude_code_tool_manager::run()
}
