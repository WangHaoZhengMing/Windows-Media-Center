use super::icon;
use crate::theme::{Layout, SHORTCUT_HEIGHT, SHORTCUT_WIDTH};
use gpui_kit::{prelude::*, *};
use wmc_assets::{Icon, Shortcut, ShortcutImages};
use wmc_core::Artwork;

pub(super) enum ShortcutVisual {
    Original(ShortcutImages),
    Glyph(Icon),
}

/// Extra destinations reuse original glyphs; they do not invent WMC focus art.
pub(super) fn visual(kind: Artwork) -> ShortcutVisual {
    let original = match kind {
        Artwork::Landscape => Shortcut::Photos,
        Artwork::Slideshow | Artwork::Heart => Shortcut::PhotoFavorites,
        Artwork::Cinema | Artwork::History => Shortcut::Videos,
        Artwork::Record => Shortcut::Music,
        Artwork::Albums => Shortcut::MusicFavorites,
        Artwork::About => Shortcut::LearnMore,
        Artwork::Calendar => return ShortcutVisual::Glyph(Icon::Calendar),
        Artwork::Folder => return ShortcutVisual::Glyph(Icon::Folder),
        Artwork::Queue => return ShortcutVisual::Glyph(Icon::Queue),
        Artwork::Display => return ShortcutVisual::Glyph(Icon::Display),
    };
    ShortcutVisual::Original(original.images())
}

pub(crate) fn shortcut(kind: Artwork, focus: f32, layout: Layout) -> Div {
    let (selected, idle) = match visual(kind) {
        ShortcutVisual::Original(images) => (
            img(images.focused)
                .size_full()
                .object_fit(ObjectFit::Fill)
                .into_any_element(),
            img(images.idle)
                .w(layout.unit(150.))
                .h(layout.unit(84.3))
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
        ),
        ShortcutVisual::Glyph(glyph) => (
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(icon(glyph, layout.unit(82.)))
                .into_any_element(),
            icon(glyph, layout.unit(52.)).into_any_element(),
        ),
    };
    div()
        .relative()
        .w(layout.unit(SHORTCUT_WIDTH))
        .h(layout.unit(SHORTCUT_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(div().size_full().opacity(focus).child(selected))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .opacity(1. - focus)
                .child(idle),
        )
}
