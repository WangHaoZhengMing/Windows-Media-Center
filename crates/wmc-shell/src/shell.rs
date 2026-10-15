use crate::{
    actions,
    overlays::{context_menu, media_details as details},
    pages::{home::launch, library as media_library, photos::gallery as photo_gallery},
};

use crate::navigation::{CATEGORIES, DestinationState, Direction, Entry};
use actions::*;
use gpui_kit::{
    base::motion::{Transition, transition},
    prelude::*,
    *,
};
use std::time::{Duration, Instant};
use wmc_motion::{HomeArrival, LAUNCH_DURATION};
use {
    wmc_design::components::background,
    wmc_design::theme::{INK, Layout, WHITE, color},
};

pub struct AppShell {
    pub(crate) home_page: crate::pages::home::state::HomePageState,
    pub(crate) photo_page: crate::pages::photos::PhotoPageState,
    pub(crate) player_page: crate::presenter::PlayerPageState,
    pub(crate) hide_cursor: std::rc::Rc<dyn Fn()>,
    pub(crate) app: wmc_application::Application,
    pub(crate) focus: FocusHandle,
    pub(crate) ambient_motion: bool,
    pub(crate) preview: Option<&'static Entry>,
    pub(crate) queue_visible: bool,
    pub(crate) queue_scroll: UniformListScrollHandle,
    pub(crate) destination_state: DestinationState,
    pub(crate) library: media_library::LibraryState,
    pub(crate) context_menu: Option<context_menu::MenuState>,
    pub(crate) _menu_focus_subscriptions: Vec<Subscription>,
    pub(crate) details: Option<details::DetailsState>,
}

impl AppShell {
    pub fn new(
        app: wmc_application::Application,
        hide_cursor: std::rc::Rc<dyn Fn()>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let menu_blur = cx.on_blur(&focus, window, |this, window, cx| {
            let Some(menu) = &mut this.context_menu else {
                return;
            };
            let epoch = Instant::now();
            menu.lost_focus = Some(epoch);
            // MenuControl::.ctor and OnLoseDeepKeyFocus: 2 seconds, reset on regain.
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let _ = this.update_in(cx, |this, _, cx| {
                    if this
                        .context_menu
                        .as_ref()
                        .is_some_and(|menu| menu.lost_focus == Some(epoch))
                    {
                        this.close_context_menu(None, cx);
                    }
                });
            })
            .detach();
        });
        let menu_focus = cx.on_focus(&focus, window, |this, _, _| {
            if let Some(menu) = &mut this.context_menu {
                menu.lost_focus = None;
            }
        });

        let mut view = Self {
            home_page: crate::pages::home::state::HomePageState::new(),
            photo_page: crate::pages::photos::PhotoPageState::new(cx),
            player_page: crate::presenter::PlayerPageState::default(),
            hide_cursor,
            app,
            focus,
            ambient_motion: true,
            preview: None,
            queue_visible: false,
            queue_scroll: UniformListScrollHandle::new(),
            destination_state: DestinationState::default(),
            library: media_library::LibraryState::new(),
            context_menu: None,
            _menu_focus_subscriptions: vec![menu_blur, menu_focus],
            details: None,
        };
        view.rescan_library(cx);
        view.start_playback_monitor(window, cx);
        view
    }
    pub(crate) fn navigate(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if self.navigate_details(direction, cx) {
            return;
        }
        if self.navigate_menu(direction, cx) {
            return;
        }
        self.library.pointer_navigation = false;
        self.library.hovered = None;
        if self.photo_page.path.is_some() {
            let kind = crate::navigation::LibraryKind::Photos;
            let indices = self.library.indices(kind, self.destination_state.tab);
            let slot = media_library::media_kind(kind).index();
            let selected = self.library.selection[slot];
            let next = match direction {
                Direction::Left => selected.saturating_sub(1),
                Direction::Right => (selected + 1).min(indices.len().saturating_sub(1)),
                _ => selected,
            };
            if let Some(&index) = indices.get(next) {
                self.library.selection[slot] = next;
                self.library.pages[slot] = next / media_library::PAGE_SIZE;
                self.photo_page.path = Some(self.library.items[index].file.path.clone());
                self.photo_page.scroll.scroll_to_item(
                    photo_gallery::photo_row(
                        next,
                        indices.len(),
                        photo_gallery::Grid::new(self.photo_page.zoom).columns,
                    ),
                    ScrollStrategy::Nearest,
                );
            }
            cx.notify();
            return;
        }
        self.home_page.launch_dismissed = true;
        if self.queue_visible || self.home_page.navigation.opened().is_some() {
            if let crate::navigation::DestinationKind::Library(kind) = self.destination_state.kind
                && self.destination_state.focus == crate::navigation::DestinationFocus::Content
            {
                let indices = self.library.indices(kind, self.destination_state.tab);
                let selected = &mut self.library.selection[media_library::media_kind(kind).index()];
                let columns = if kind == crate::navigation::LibraryKind::Photos {
                    photo_gallery::Grid::new(self.photo_page.zoom).columns
                } else if !self.library.list {
                    5
                } else {
                    1
                };
                let next = match direction {
                    Direction::Left => selected.saturating_sub(1),
                    Direction::Right => selected.saturating_add(1),
                    Direction::Down => selected.saturating_add(columns),
                    Direction::Up if *selected >= columns => selected.saturating_sub(columns),
                    _ => {
                        self.destination_state.navigate(direction);
                        cx.notify();
                        return;
                    }
                };
                *selected = next.min(indices.len().saturating_sub(1));
                self.library.pages[media_library::media_kind(kind).index()] =
                    *selected / media_library::PAGE_SIZE;
                if kind == crate::navigation::LibraryKind::Photos {
                    self.photo_page.scroll.scroll_to_item(
                        photo_gallery::photo_row(*selected, indices.len(), columns),
                        ScrollStrategy::Nearest,
                    );
                }
                cx.notify();
                return;
            }
            self.destination_state.navigate(direction);
            cx.notify();
            return;
        }
        self.home_page.navigation.navigate(direction);
        cx.notify();
    }
    pub(crate) fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.details.is_some() {
            self.activate_details(window, cx);
            return;
        }
        if let Some(menu) = &self.context_menu {
            let action = self.menu_actions(menu.target)[menu.selected].0;
            self.close_context_menu(Some(action), cx);
            return;
        }
        self.home_page.launch_dismissed = true;
        if self.queue_visible || self.home_page.navigation.opened().is_some() {
            if let crate::navigation::DestinationKind::Library(kind) = self.destination_state.kind
                && self.destination_state.focus == crate::navigation::DestinationFocus::Content
            {
                let indices = self.library.indices(kind, self.destination_state.tab);
                if let Some(&index) =
                    indices.get(self.library.selection[media_library::media_kind(kind).index()])
                {
                    self.open_media(index, window, cx);
                }
                return;
            }
            self.activate_destination(cx);
            return;
        }
        self.queue_visible = false;
        self.preview = Some(self.home_page.navigation.selected_entry());
        self.destination_state.reset(
            self.home_page.navigation.selected_entry().id,
            self.ambient_motion,
        );
        self.home_page.navigation.activate();
        if self.destination_state.kind == crate::navigation::DestinationKind::MediaLocations {
            self.library.draft = self.library.config.clone();
        }
        cx.notify();
    }
    pub(crate) fn back(&mut self, cx: &mut Context<Self>) {
        if self.player_page.visible {
            self.return_from_player(cx);
            return;
        }
        if self.details.is_some() {
            self.close_details(cx);
            return;
        }
        if self.context_menu.is_some() {
            self.close_context_menu(None, cx);
            return;
        }
        self.app.cancel_load();
        self.player_page.entering = None;
        if self.photo_page.path.take().is_some() {
            cx.notify();
            return;
        }
        self.home_page.launch_dismissed = true;
        self.queue_visible = false;
        self.home_page.navigation.back();
        cx.notify();
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.advance_player_exit(window, cx);
        self.advance_context_menu(window, cx);
        self.advance_details(window, cx);
        self.sync_photos(window, cx);
        let layout = Layout::for_window(window);
        if let Some(path) = &self.photo_page.path {
            return div()
                .id("photo-viewer")
                .size_full()
                .relative()
                .bg(color(INK))
                .child(
                    if let Some(image) = self.photo_image(path, Self::photo_size(window), cx) {
                        img(image)
                            .absolute()
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .into_any_element()
                    } else {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                self.photo_error(path, Self::photo_size(window), cx)
                                    .unwrap_or_else(|| "正在加载照片…".into()),
                            )
                            .into_any_element()
                    },
                )
                .track_focus(&self.focus)
                .key_context("MediaCenter")
                .on_action(cx.listener(|this, _: &Back, _, cx| this.back(cx)))
                .on_action(
                    cx.listener(|this, _: &MoveLeft, _, cx| this.navigate(Direction::Left, cx)),
                )
                .on_action(
                    cx.listener(|this, _: &MoveRight, _, cx| this.navigate(Direction::Right, cx)),
                )
                .on_action(cx.listener(|_, _: &Fullscreen, window, _| window.toggle_fullscreen()))
                .child(self.header(layout, window, cx))
                .into_any_element();
        }
        if let Some(player) = &self.player_page.view
            && self.player_page.visible
            && self.player_page.exiting.is_none()
        {
            window.request_animation_frame();
            let chrome_alpha = player.update(cx, |player, cx| player.controls_opacity(window, cx));
            return div()
                .id("media-center-player")
                .relative()
                .size_full()
                .overflow_hidden()
                .child(
                    div()
                        .size_full()
                        .opacity(self.player_exit_alpha(cx.reduce_motion()))
                        .child(player.clone()),
                )
                .when(chrome_alpha > 0.001, |root| {
                    root.child(self.header(layout, window, cx).opacity(chrome_alpha))
                })
                .into_any_element();
        }
        if let Some(started) = self.player_page.library_return {
            if !cx.reduce_motion()
                && started.elapsed().as_secs_f32() < wmc_motion::player_return::RETURN_DURATION
            {
                window.request_animation_frame();
            } else {
                self.player_page.library_return = None;
            }
        }
        let song_alpha = self.player_page.entering.map_or(1., |started| {
            let elapsed = started.elapsed().as_secs_f32();
            if elapsed < wmc_motion::player_entry::LIBRARY_EXIT_SECONDS && !cx.reduce_motion() {
                window.request_animation_frame();
            }
            wmc_motion::player_entry::library_exit(elapsed, cx.reduce_motion())
        });
        let launch_time = self
            .home_page
            .launch_epoch
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f32();
        let launching = !self.home_page.launch_dismissed
            && !cx.reduce_motion()
            && launch_time < LAUNCH_DURATION;
        let arrival = HomeArrival::sample(
            launch_time,
            self.home_page.navigation.category_index(),
            CATEGORIES.len(),
            self.home_page.launch_dismissed || cx.reduce_motion(),
        );
        if launching || arrival.moving {
            window.request_animation_frame();
        }
        let now = self.home_page.epoch.elapsed();
        self.home_page
            .motion
            .retarget(self.home_page.navigation.category_index(), now);
        let motion = self.home_page.motion.sample(now, cx.reduce_motion());
        if motion.moving {
            window.request_animation_frame();
        }
        let page = transition(
            "destination-presence",
            if self.home_page.navigation.opened().is_some() || self.queue_visible {
                1.0_f32
            } else {
                0.0
            },
            Transition::new(Duration::from_millis(320)),
            window,
            cx,
        );
        let mut root = div()
            .id("media-center")
            .key_context("MediaCenter")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(color(INK))
            .text_color(color(WHITE))
            .font_family("Microsoft YaHei UI")
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.show_context_menu(None, event.position, window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &MoveUp, _, cx| this.navigate(Direction::Up, cx)))
            .on_action(cx.listener(|this, _: &MoveDown, _, cx| this.navigate(Direction::Down, cx)))
            .on_action(cx.listener(|this, _: &MoveLeft, _, cx| this.navigate(Direction::Left, cx)))
            .on_action(
                cx.listener(|this, _: &MoveRight, _, cx| this.navigate(Direction::Right, cx)),
            )
            .on_action(cx.listener(|this, _: &Open, window, cx| this.open(window, cx)))
            .on_action(cx.listener(|this, _: &DetailsNext, window, cx| {
                if !this.navigate_details(Direction::Right, cx) {
                    window.focus_next(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &DetailsPrevious, window, cx| {
                if !this.navigate_details(Direction::Left, cx) {
                    window.focus_prev(cx);
                }
            }))
            .on_action(cx.listener(move |this, _: &ContextMenu, window, cx| {
                let target = if let crate::navigation::DestinationKind::Library(kind) =
                    this.destination_state.kind
                    && this.home_page.navigation.opened().is_some()
                    && this.destination_state.focus == crate::navigation::DestinationFocus::Content
                {
                    this.library
                        .indices(kind, this.destination_state.tab)
                        .get(this.library.selection[media_library::media_kind(kind).index()])
                        .copied()
                } else {
                    None
                };
                this.show_context_menu(
                    target,
                    point(layout.unit(400.), layout.unit(220.)),
                    window,
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &Back, _, cx| this.back(cx)))
            .on_action(cx.listener(|_, _: &Fullscreen, window, _| window.toggle_fullscreen()))
            .child(background(
                layout,
                self.ambient_motion && !cx.reduce_motion(),
            ))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .opacity(
                        (1. - page)
                            * song_alpha
                            * self
                                .context_menu
                                .as_ref()
                                .map_or(1., |menu| menu.pose(cx.reduce_motion()).page_alpha),
                    )
                    .child(self.rail(layout, &motion, &arrival, cx))
                    .child(self.tiles(layout, &motion, &arrival, window, cx)),
            );
        if let Some(entry) = self.preview.filter(|_| page > 0.001) {
            root = root.child(
                self.destination(entry, layout, page, window, cx)
                    .opacity(song_alpha),
            );
        }
        if launching {
            root = root.child(launch::launch_overlay(
                layout,
                window.viewport_size(),
                launch_time,
            ));
        }
        root = root.child(self.transport_bar(layout, cx));
        root = root.child(self.now_playing(layout, cx));
        if self.details.is_some() {
            root = root.child(self.details_overlay(layout, window, cx).opacity(song_alpha));
        }
        if self.player_page.exiting.is_some()
            && let Some(player) = &self.player_page.view
        {
            let alpha = self.player_exit_alpha(cx.reduce_motion());
            if alpha > 0. {
                root = root.child(
                    div()
                        .absolute()
                        .size_full()
                        .opacity(alpha)
                        .child(player.clone()),
                );
            }
        }
        root = root.child(self.header(layout, window, cx));
        if self.context_menu.is_some() {
            root = root.child(self.context_menu_overlay(layout, window, cx));
        }
        root.into_any_element()
    }
}
