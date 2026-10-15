//! WMC visuals over GPUI Base's keyboard, focus and disabled behavior.
use gpui_kit::{base::Button, prelude::*, *};
use wmc_assets::{Icon, IconState};

#[cfg(test)]
mod tests;

gpui_kit::actions!(wmc_controls, [FocusNext, FocusPrevious]);

pub fn init(cx: &mut App) {
    // Let GPUI synthesize a keyboard click instead of triggering page actions.
    cx.bind_keys([
        KeyBinding::new("enter", NoAction, Some("WmcButton")),
        KeyBinding::new("space", NoAction, Some("WmcButton")),
        KeyBinding::new("tab", FocusNext, Some("WmcButton")),
        KeyBinding::new("shift-tab", FocusPrevious, Some("WmcButton")),
    ]);
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id)
        .accessibility_label(label)
        .key_context("WmcButton")
        .on_action(|_: &FocusNext, window, cx| window.focus_next(cx))
        .on_action(|_: &FocusPrevious, window, cx| window.focus_prev(cx))
}

pub fn image_button(
    id: &'static str,
    icon: Icon,
    width: f32,
    height: f32,
    available: bool,
    scale: f32,
) -> Button {
    let state = if available {
        IconState::Normal
    } else {
        IconState::Disabled
    };
    button(id, icon.label())
        .disabled(!available)
        .group(id)
        .relative()
        .flex_shrink_0()
        .w(px(width * scale))
        .h(px(height * scale))
        .focus_visible(|style| style.bg(rgb(0x317aa6).opacity(0.5)).rounded(px(4. * scale)))
        .when(available, |button| button.cursor_pointer())
        .child(
            img(icon.image(state))
                .size_full()
                .object_fit(ObjectFit::Contain),
        )
        .when(available, |button| {
            button.child(
                div()
                    .absolute()
                    .inset_0()
                    .opacity(0.)
                    .group_hover(id, |style| style.opacity(1.))
                    .child(
                        img(icon.image(IconState::Focused))
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    ),
            )
        })
}
