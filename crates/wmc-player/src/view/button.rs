//! Player controls share the same Base Button and WMC artwork as the home page.
use gpui_kit::base::Button;
use wmc_assets::Icon;

pub(super) fn button(id: &'static str, icon: Icon, width: f32, height: f32, scale: f32) -> Button {
    wmc_controls::image_button(id, icon, width, height, true, scale)
}
