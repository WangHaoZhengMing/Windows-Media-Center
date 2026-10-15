//! Routes, shell chrome and page-specific presentation state.
mod actions;
mod chrome;
mod navigation;
mod overlays;
mod pages;
mod presenter;
mod shell;
pub use shell::AppShell;
pub fn bind_keys(cx: &mut gpui_kit::App) {
    actions::bind_keys(cx);
    wmc_player_ui::bind_keys(cx);
}
