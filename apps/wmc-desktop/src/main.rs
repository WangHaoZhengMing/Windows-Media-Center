#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod bootstrap;
mod desktop_host;
fn main() {
    bootstrap::run();
}
