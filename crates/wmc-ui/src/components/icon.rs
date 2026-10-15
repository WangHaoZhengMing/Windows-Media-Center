use gpui_kit::{prelude::*, *};
use wmc_assets::Icon;

pub(crate) use wmc_controls::image_button as resource_button;

pub(crate) fn icon(kind: Icon, size: Pixels) -> Img {
    img(kind.path()).size(size).object_fit(ObjectFit::Contain)
}
