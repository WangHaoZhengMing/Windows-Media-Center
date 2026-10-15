use gpui_kit::{prelude::*, *};
use wmc_assets::Icon;

pub use crate::image_button as resource_button;

pub fn icon(kind: Icon, size: Pixels) -> Img {
    img(kind.path()).size(size).object_fit(ObjectFit::Contain)
}
