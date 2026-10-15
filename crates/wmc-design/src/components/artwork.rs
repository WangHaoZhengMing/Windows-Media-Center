use gpui_kit::{prelude::*, *};
use wmc_assets::{ALBUM_PLACEHOLDER, Icon, VIDEO_PLACEHOLDER};
use wmc_domain::MediaKind as LibraryKind;

/// Gallery thumbnails retain the original placeholder art, without a hero panel.
pub fn artwork(kind: LibraryKind, size: Pixels) -> Img {
    let path = match kind {
        LibraryKind::Music => ALBUM_PLACEHOLDER,
        LibraryKind::Photos => Icon::Folder.path(),
        LibraryKind::Videos => VIDEO_PLACEHOLDER,
    };
    img(path)
        .size(size)
        .object_fit(if kind == LibraryKind::Music {
            ObjectFit::Cover
        } else {
            ObjectFit::Contain
        })
}
