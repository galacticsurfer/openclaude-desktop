// Keep the console window hidden on Windows release builds; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    openclaude_lib::run()
}
