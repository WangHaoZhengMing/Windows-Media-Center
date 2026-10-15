use crate::navigation::{CATEGORIES, DestinationKind, Direction, EntryId};
use crate::shell::AppShell;
use gpui_kit::{prelude::*, *};
use std::time::Instant;
use wmc_design::components::{
    artwork,
    menu_skin::{MenuSkin, menu_skin},
};
use wmc_design::theme::{Layout, color};
use wmc_motion::context_menu::{
    ContextMenuPose, context_menu_focus_glow, context_menu_hide, context_menu_show,
};

pub(crate) struct MenuState {
    pub target: Option<usize>,
    pub position: Point<Pixels>,
    pub selected: usize,
    pub anchor: Option<Bounds<Pixels>>,
    pub opened: Instant,
    pub closing: Option<(Instant, ContextMenuPose)>,
    pub pending: Option<MenuAction>,
    pub focus_epoch: Instant,
    pub lost_focus: Option<Instant>,
}

impl MenuState {
    pub(crate) fn pose(&self, reduced: bool) -> ContextMenuPose {
        if let Some((epoch, from)) = self.closing {
            context_menu_hide(epoch.elapsed().as_secs_f32(), from, reduced)
        } else {
            context_menu_show(self.opened.elapsed().as_secs_f32(), reduced)
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum MenuAction {
    Open,
    Enqueue,
    Remove,
    Details,
    Small,
    List,
    Manage,
    Settings,
}

impl AppShell {
    pub(crate) fn menu_actions(&self, target: Option<usize>) -> Vec<(MenuAction, &'static str)> {
        let mut actions = Vec::new();
        if let Some(item) = target.and_then(|index| self.library.items.get(index)) {
            actions.push((
                MenuAction::Open,
                if item.file.kind == wmc_library::MediaKind::Photos {
                    "查看"
                } else {
                    "播放"
                },
            ));
            if item.file.kind == wmc_library::MediaKind::Music {
                actions.push((MenuAction::Enqueue, "添加到播放列表"));
            }
            actions.push((MenuAction::Details, "详细信息"));
            actions.push((MenuAction::Remove, "从本次列表移除"));
        } else if self.home_page.navigation.opened().is_none() && !self.queue_visible {
            actions.push((MenuAction::Open, "打开当前项目"));
        }
        if matches!(self.destination_state.kind, DestinationKind::Library(_))
            && self.home_page.navigation.opened().is_some()
        {
            actions.extend([
                (MenuAction::Small, "小图标视图"),
                (MenuAction::List, "列表视图"),
            ]);
        }
        actions.extend([
            (MenuAction::Manage, "管理媒体库"),
            (MenuAction::Settings, "设置"),
        ]);
        actions
    }

    pub(crate) fn show_context_menu(
        &mut self,
        target: Option<usize>,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.details.is_some() {
            cx.stop_propagation();
            return;
        }
        self.home_page.launch_dismissed = true;
        self.context_menu = Some(MenuState {
            target,
            position,
            selected: 0,
            anchor: target.and_then(|index| self.library.bounds.get(&index).copied()),
            opened: Instant::now(),
            closing: None,
            pending: None,
            focus_epoch: Instant::now(),
            lost_focus: None,
        });
        window.focus(&self.focus, cx);
        cx.stop_propagation();
        cx.notify();
    }

    pub(crate) fn navigate_menu(&mut self, direction: Direction, cx: &mut Context<Self>) -> bool {
        let Some(menu) = &self.context_menu else {
            return false;
        };
        if menu.closing.is_some() {
            return true;
        }
        let count = self.menu_actions(menu.target).len();
        let menu = self.context_menu.as_mut().unwrap();
        menu.selected = match direction {
            Direction::Up => (menu.selected + count - 1) % count,
            Direction::Down => (menu.selected + 1) % count,
            _ => menu.selected,
        };
        menu.focus_epoch = Instant::now();
        cx.notify();
        true
    }

    pub(crate) fn run_menu_action(
        &mut self,
        action: MenuAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu.take().and_then(|menu| menu.target);
        match action {
            MenuAction::Open => {
                if let Some(index) = target {
                    self.open_media(index, window, cx);
                } else {
                    self.open(window, cx);
                }
            }
            MenuAction::Enqueue => {
                if let Some(item) = target.and_then(|index| self.library.items.get(index)) {
                    self.app.enqueue(item.file.path.clone(), item.title.clone());
                }
            }
            MenuAction::Remove => {
                if let Some(index) = target.filter(|&index| index < self.library.items.len()) {
                    self.library.items.remove(index);
                    self.library.selection = [0; 3];
                    self.library.pages = [0; 3];
                }
            }
            MenuAction::Details => {
                if let Some(index) = target {
                    self.show_details(index, cx);
                }
            }
            MenuAction::Small => self.library.list = false,
            MenuAction::List => self.library.list = true,
            MenuAction::Manage | MenuAction::Settings => {
                let id = if matches!(action, MenuAction::Manage) {
                    EntryId::MediaLocations
                } else {
                    EntryId::Appearance
                };
                self.home_page.navigation.back();
                if let Some((category, entry)) =
                    CATEGORIES.iter().enumerate().find_map(|(category, row)| {
                        row.entries
                            .iter()
                            .position(|entry| entry.id == id)
                            .map(|entry| (category, entry))
                    })
                {
                    self.home_page.navigation.select_category(category);
                    self.home_page.navigation.select_entry(entry);
                    self.open(window, cx);
                }
            }
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(crate) fn close_context_menu(
        &mut self,
        action: Option<MenuAction>,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = &mut self.context_menu
            && menu.closing.is_none()
        {
            let pose = menu.pose(cx.reduce_motion());
            menu.closing = Some((Instant::now(), pose));
            menu.pending = action;
            cx.notify();
        }
    }

    pub(crate) fn advance_context_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = &self.context_menu else {
            return;
        };
        if let Some((epoch, _)) = menu.closing {
            let deadline = if menu.anchor.is_some() { 0.6 } else { 0.35 };
            if cx.reduce_motion() || epoch.elapsed().as_secs_f32() >= deadline {
                if let Some(action) = menu.pending {
                    self.run_menu_action(action, window, cx);
                } else {
                    self.context_menu = None;
                }
            } else {
                window.request_animation_frame();
            }
        } else if menu.opened.elapsed().as_secs_f32() < 0.5 || !cx.reduce_motion() {
            window.request_animation_frame();
        }
    }

    pub(crate) fn context_menu_overlay(
        &self,
        layout: Layout,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let menu = self.context_menu.as_ref().unwrap();
        let actions = self.menu_actions(menu.target);
        let unique_count = if menu.target.is_some() {
            actions
                .iter()
                .take_while(|(action, _)| {
                    matches!(
                        action,
                        MenuAction::Open
                            | MenuAction::Enqueue
                            | MenuAction::Remove
                            | MenuAction::Details
                    )
                })
                .count()
        } else {
            0
        };
        let shared_count = actions.len() - unique_count;
        let group_height =
            |count: usize| 18. + count as f32 * 51. + count.saturating_sub(1) as f32 * 4.;
        let top_height = group_height(unique_count);
        let bottom_height = group_height(shared_count);
        let base_width = 287.;
        let base_height = 10. + top_height + bottom_height;
        let scale = layout.scale;
        let menu_size = size(px(base_width * scale), px(base_height * scale));
        let viewport = window.viewport_size();
        let origin = contextual_origin(menu.anchor, viewport, menu_size, scale, menu.position);
        let pose = menu.pose(cx.reduce_motion());
        let sx = scale * pose.scale[0];
        let sy = scale * pose.scale[1];
        let width = base_width * sx;
        let height = base_height * sy;
        let left = origin.x + (menu_size.width - px(width)) / 2.;
        let top = origin.y + (menu_size.height - px(height)) / 2.;
        let mut panel = div()
            .id("context-menu-panel")
            .absolute()
            .left(left)
            .top(top)
            .w(px(width))
            .h(px(height))
            .opacity(pose.alpha)
            .occlude()
            .font_family("Segoe Media Center Semibold")
            .text_color(color(0xf2f2f2))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(menu_skin(MenuSkin::Outline, width, height, [sx, sy]));
        {
            panel = panel.child(div().absolute().left(px(5. * sx)).top(px(5. * sy)).child(
                menu_skin(
                    if unique_count > 0 {
                        MenuSkin::TopLight
                    } else {
                        MenuSkin::TopDark
                    },
                    (base_width - 10.) * sx,
                    top_height * sy,
                    [sx, sy],
                ),
            ));
        }
        if shared_count > 0 {
            panel = panel.child(
                div()
                    .absolute()
                    .left(px(5. * sx))
                    .top(px((5. + top_height) * sy))
                    .child(menu_skin(
                        if shared_count > 0 {
                            MenuSkin::BottomDark
                        } else {
                            MenuSkin::BottomLight
                        },
                        (base_width - 10.) * sx,
                        bottom_height * sy,
                        [sx, sy],
                    )),
            );
        }
        for (index, (action, title)) in actions.into_iter().enumerate() {
            let selected = menu.selected == index;
            let (slot, group_top) = if index < unique_count {
                (index, 5.)
            } else {
                (index - unique_count, 5. + top_height)
            };
            let y = group_top + 9. + slot as f32 * 55. * pose.items_position;
            if slot > 0 {
                panel = panel.child(
                    img("wmc/context-menu/OVERLAYS.DIVIDER.HORIZONTAL.PNG")
                        .absolute()
                        .left(px(16. * sx))
                        .top(px((group_top
                            + 9.
                            + (slot as f32 * 55. - 3.) * pose.items_position)
                            * sy))
                        .w(px(224. * sx))
                        .h(px(2. * sy))
                        .object_fit(ObjectFit::Fill)
                        .opacity(pose.items_alpha),
                );
            }
            panel = panel.child(
                div()
                    .id(("context-menu-item", index))
                    .absolute()
                    .left(px(14. * sx))
                    .top(px(y * sy))
                    .w(px(250. * sx))
                    .h(px(51. * sy))
                    .opacity(pose.items_alpha)
                    .cursor_pointer()
                    .when(selected, |row| {
                        row.child(menu_skin(MenuSkin::Focus, 250. * sx, 51. * sy, [sx, sy]))
                            .child(
                                div()
                                    .absolute()
                                    .opacity(if cx.reduce_motion() {
                                        0.
                                    } else {
                                        context_menu_focus_glow(
                                            menu.focus_epoch.elapsed().as_secs_f32(),
                                        )
                                    })
                                    .child(menu_skin(
                                        MenuSkin::FocusOverlay,
                                        250. * sx,
                                        51. * sy,
                                        [sx, sy],
                                    )),
                            )
                    })
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .flex()
                            .items_center()
                            .pl(px(16. * sx))
                            .text_size(px((20. * sy * 96. / 72.).floor()))
                            .child(title),
                    )
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if let Some(menu) = &mut this.context_menu
                            && menu.closing.is_none()
                            && menu.selected != index
                        {
                            menu.selected = index;
                            menu.focus_epoch = Instant::now();
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.close_context_menu(Some(action), cx);
                    })),
            );
        }
        let mut overlay = div().absolute().size_full().child(
            div()
                .id("context-menu-dismiss")
                .absolute()
                .size_full()
                .occlude()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.close_context_menu(None, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, _, cx| {
                        this.close_context_menu(None, cx);
                        cx.stop_propagation();
                    }),
                ),
        );
        if let Some(anchor) = menu.anchor
            && let Some(item) = menu.target.and_then(|index| self.library.items.get(index))
        {
            let w = anchor.size.width * pose.thumbnail_scale;
            let h = anchor.size.height * pose.thumbnail_scale;
            let image = if item.file.kind == wmc_library::MediaKind::Photos {
                if let Some(image) = self.photo_image(
                    &item.file.path,
                    crate::pages::photos::image_cache::THUMB,
                    cx,
                ) {
                    img(image)
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    artwork(crate::navigation::LibraryKind::Photos, w).into_any_element()
                }
            } else if let Some(cover) = &item.cover {
                img(cover.clone())
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element()
            } else {
                let kind = match item.file.kind {
                    wmc_library::MediaKind::Music => crate::navigation::LibraryKind::Music,
                    _ => crate::navigation::LibraryKind::Videos,
                };
                artwork(kind, w).into_any_element()
            };
            overlay = overlay.child(
                div()
                    .absolute()
                    .left(anchor.origin.x + (anchor.size.width - w) / 2.)
                    .top(anchor.origin.y + (anchor.size.height - h) / 2.)
                    .w(w)
                    .h(h)
                    .opacity(pose.contextual_alpha)
                    .child(image),
            );
        }
        overlay
            .child(
                div()
                    .absolute()
                    .left(left)
                    .top(top)
                    .opacity(pose.alpha)
                    .child(menu_skin(
                        MenuSkin::Shadow,
                        width + 37. * sx,
                        height + 45. * sy,
                        [sx, sy],
                    )),
            )
            .child(panel)
    }
}

/// ContextMenuLayout::ILayout.Layout IL_0317..IL_0407: right/left flip,
/// vertical centering, then clamp inside the padded host.
fn contextual_origin(
    anchor: Option<Bounds<Pixels>>,
    viewport: Size<Pixels>,
    menu: Size<Pixels>,
    scale: f32,
    _pointer: Point<Pixels>,
) -> Point<Pixels> {
    // Include the panel's 10px horizontal margins inside the host padding.
    let safe_left = px(40. * scale);
    let safe_top = px(34. * scale);
    let max_x = (viewport.width - px(68. * scale) - menu.width).max(safe_left);
    let max_y = (viewport.height - px(58. * scale) - menu.height).max(safe_top);
    let Some(anchor) = anchor else {
        return point(max_x, max_y);
    };
    let right = anchor.origin.x + anchor.size.width + px(35. * scale);
    let x = if right + menu.width < viewport.width - px(68. * scale) {
        right
    } else {
        anchor.origin.x - px(35. * scale) - menu.width
    };
    let y = anchor.origin.y + (anchor.size.height - menu.height) / 2.;
    point(x.clamp(safe_left, max_x), y.clamp(safe_top, max_y))
}

#[cfg(test)]
mod tests {
    use super::contextual_origin;
    use gpui_kit::{Bounds, point, px, size};

    #[test]
    fn il_layout_flips_to_the_left_and_centers_on_the_context_item() {
        let viewport = size(px(1280.), px(720.));
        let menu = size(px(287.), px(423.));
        let anchor = |x| {
            Some(Bounds::new(
                point(px(x), px(200.)),
                size(px(100.), px(100.)),
            ))
        };
        assert_eq!(
            contextual_origin(anchor(200.), viewport, menu, 1., point(px(0.), px(0.))),
            point(px(335.), px(38.5))
        );
        assert_eq!(
            contextual_origin(anchor(1000.), viewport, menu, 1., point(px(0.), px(0.))),
            point(px(678.), px(38.5))
        );
    }

    #[test]
    fn uncontextual_menu_uses_bottom_right_host_padding() {
        assert_eq!(
            contextual_origin(
                None,
                size(px(1280.), px(720.)),
                size(px(287.), px(423.)),
                1.,
                point(px(50.), px(50.))
            ),
            point(px(925.), px(239.))
        );
    }
}
