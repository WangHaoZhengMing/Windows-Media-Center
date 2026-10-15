use gpui_kit::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, KeyBinding, KeyDownEvent,
    KeyUpEvent, Keystroke, Modifiers, ParentElement, Render, Styled, TestAppContext, Window, div,
    point, px,
};
use std::{cell::Cell, rc::Rc};

gpui_kit::actions!(test_player, [TogglePlayback]);

struct Harness {
    disabled: bool,
    clicks: Rc<Cell<usize>>,
    page_actions: Rc<Cell<usize>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let page_actions = self.page_actions.clone();
        div()
            .id("test-page")
            .key_context("MusicPlayer")
            .size(px(100.))
            .on_action(move |_: &TogglePlayback, _, _| page_actions.set(page_actions.get() + 1))
            .child(
                super::button("test-button", "播放")
                    .disabled(self.disabled)
                    .size_full()
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
    }
}

fn initialize(cx: &mut App) {
    gpui_kit::init(cx);
    super::init(cx);
    cx.bind_keys([
        KeyBinding::new("enter", TogglePlayback, Some("MusicPlayer")),
        KeyBinding::new("space", TogglePlayback, Some("MusicPlayer")),
    ]);
}

#[gpui_kit::test]
fn focused_button_activates_once_without_triggering_page_shortcuts(cx: &mut TestAppContext) {
    cx.update(initialize);
    let clicks = Rc::new(Cell::new(0));
    let page_actions = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let clicks = clicks.clone();
        let page_actions = page_actions.clone();
        move |_, _| Harness {
            disabled: false,
            clicks,
            page_actions,
        }
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1);
    for key in ["enter", "space"] {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
    }
    assert_eq!(clicks.get(), 3);
    assert_eq!(page_actions.get(), 0);
}

#[gpui_kit::test]
fn disabled_button_ignores_activation(cx: &mut TestAppContext) {
    cx.update(initialize);
    let clicks = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let clicks = clicks.clone();
        move |_, _| Harness {
            disabled: true,
            clicks,
            page_actions: Rc::new(Cell::new(0)),
        }
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 0);
}

struct TabHarness {
    first: FocusHandle,
    disabled: FocusHandle,
    last: FocusHandle,
}

impl Render for TabHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size(px(150.))
            .child(
                super::button("first", "播放")
                    .track_focus(&self.first)
                    .w_full()
                    .h(px(40.)),
            )
            .child(
                super::button("disabled", "停止")
                    .track_focus(&self.disabled)
                    .disabled(true)
                    .w_full()
                    .h(px(40.)),
            )
            .child(
                super::button("last", "下一首")
                    .track_focus(&self.last)
                    .w_full()
                    .h(px(40.)),
            )
    }
}

#[gpui_kit::test]
fn tab_navigation_skips_disabled_controls(cx: &mut TestAppContext) {
    cx.update(initialize);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let first = cx.focus_handle();
        window.focus(&first, cx);
        TabHarness {
            first,
            disabled: cx.focus_handle(),
            last: cx.focus_handle(),
        }
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| assert!(view.read(cx).last.is_focused(window)));
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| assert!(view.read(cx).first.is_focused(window)));
}
