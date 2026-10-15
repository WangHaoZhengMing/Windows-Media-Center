mod actions;
mod chrome;
mod context_menu;
mod destination;
mod details;
mod launch;
mod media_library;
mod photo_gallery;
mod photos;
mod player_transition;
mod playback;
mod rail;
mod tiles;
mod transport;

use crate::{
    components::background,
    theme::{INK, Layout, WHITE, color},
};
pub(crate) use actions::bind_keys;
use actions::*;
use gpui_kit::{
    base::motion::{Transition, transition},
    prelude::*,
    *,
};
use std::time::{Duration, Instant};
use wmc_core::{CATEGORIES, DestinationState, Direction, Entry, HomeState, TransportState};
use wmc_motion::{CategoryMotion, HomeArrival, LAUNCH_DURATION};

pub(crate) struct HomeView {
    state: HomeState,
    focus: FocusHandle,
    ambient_motion: bool,
    preview: Option<&'static Entry>,
    queue_visible: bool,
    queue_scroll: UniformListScrollHandle,
    transport: TransportState,
    category_motion: CategoryMotion<{ CATEGORIES.len() }>,
    motion_epoch: Instant,
    destination_state: DestinationState,
    player: Option<Entity<wmc_player::PlayerView>>,
    player_visible: bool,
    queue_index: Option<usize>,
    queue_played: Vec<usize>,
    playback_finished: bool,
    player_subscription: Option<Subscription>,
    song_transition: Option<Instant>,
    player_exit: Option<Instant>,
    library_return: Option<Instant>,
    launch_epoch: Option<Instant>,
    launch_dismissed: bool,
    library: media_library::LibraryState,
    photo: Option<std::path::PathBuf>,
    photos: Entity<photos::PhotoImages>,
    _photo_subscription: Subscription,
    photo_scroll: UniformListScrollHandle,
    photo_visible: std::ops::Range<usize>,
    photo_grid_generation: Option<u64>,
    photo_zoom: f32,
    context_menu: Option<context_menu::MenuState>,
    library_list: bool,
    song_queue: Vec<(std::path::PathBuf, String)>,
    media_bounds: std::collections::HashMap<usize, Bounds<Pixels>>,
    _menu_focus_subscriptions: Vec<Subscription>,
    details: Option<details::DetailsState>,
}

impl HomeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        let photos = photos::PhotoImages::new(cx);
        let photo_subscription = cx.observe(&photos, |_, _, cx| cx.notify());
        let mut view = Self {
            state: HomeState::default(),
            focus,
            ambient_motion: true,
            preview: None,
            queue_visible: false,
            queue_scroll: UniformListScrollHandle::new(),
            transport: TransportState::default(),
            category_motion: CategoryMotion::new(0),
            motion_epoch: Instant::now(),
            destination_state: DestinationState::default(),
            player: None,
            player_visible: false,
            queue_index: None,
            queue_played: Vec::new(),
            playback_finished: false,
            player_subscription: None,
            song_transition: None,
            player_exit: None,
            library_return: None,
            launch_epoch: None,
            launch_dismissed: false,
            library: media_library::LibraryState::new(),
            photo: None,
            photos,
            _photo_subscription: photo_subscription,
            photo_scroll: UniformListScrollHandle::new(),
            photo_visible: 0..0,
            photo_grid_generation: None,
            photo_zoom: 1.,
            context_menu: None,
            library_list: false,
            song_queue: Vec::new(),
            media_bounds: std::collections::HashMap::new(),
            _menu_focus_subscriptions: vec![menu_blur, menu_focus],
            details: None,
        };
        view.rescan_library(cx);
        view.start_playback_monitor(window, cx);
        view
    }
    fn navigate(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if self.navigate_details(direction, cx) {
            return;
        }
        if self.navigate_menu(direction, cx) {
            return;
        }
        self.library.pointer_navigation = false;
        self.library.hovered = None;
        if self.photo.is_some() {
            let kind = wmc_core::LibraryKind::Photos;
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
                self.photo = Some(self.library.items[index].file.path.clone());
                self.photo_scroll.scroll_to_item(
                    photo_gallery::photo_row(
                        next,
                        indices.len(),
                        photo_gallery::Grid::new(self.photo_zoom).columns,
                    ),
                    ScrollStrategy::Nearest,
                );
            }
            cx.notify();
            return;
        }
        self.launch_dismissed = true;
        if self.queue_visible || self.state.opened().is_some() {
            if let wmc_core::DestinationKind::Library(kind) = self.destination_state.kind
                && self.destination_state.focus == wmc_core::DestinationFocus::Content
            {
                let indices = self.library.indices(kind, self.destination_state.tab);
                let selected = &mut self.library.selection[media_library::media_kind(kind).index()];
                let columns = if kind == wmc_core::LibraryKind::Photos {
                    photo_gallery::Grid::new(self.photo_zoom).columns
                } else if !self.library_list {
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
                if kind == wmc_core::LibraryKind::Photos {
                    self.photo_scroll.scroll_to_item(
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
        self.state.navigate(direction);
        cx.notify();
    }
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.details.is_some() {
            self.activate_details(window, cx);
            return;
        }
        if let Some(menu) = &self.context_menu {
            let action = self.menu_actions(menu.target)[menu.selected].0;
            self.close_context_menu(Some(action), cx);
            return;
        }
        self.launch_dismissed = true;
        if self.queue_visible || self.state.opened().is_some() {
            if let wmc_core::DestinationKind::Library(kind) = self.destination_state.kind
                && self.destination_state.focus == wmc_core::DestinationFocus::Content
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
        self.preview = Some(self.state.selected_entry());
        self.destination_state
            .reset(self.state.selected_entry().id, self.ambient_motion);
        self.state.activate();
        if self.destination_state.kind == wmc_core::DestinationKind::MediaLocations {
            self.library.draft = self.library.config.clone();
        }
        cx.notify();
    }
    fn back(&mut self, cx: &mut Context<Self>) {
        if self.player_visible {
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
        self.library.play_generation += 1;
        self.library.loading_track = false;
        self.song_transition = None;
        if self.photo.take().is_some() {
            cx.notify();
            return;
        }
        self.launch_dismissed = true;
        self.queue_visible = false;
        self.state.back();
        cx.notify();
    }
}

impl Focusable for HomeView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.advance_player_exit(window, cx);
        self.advance_context_menu(window, cx);
        self.advance_details(window, cx);
        self.sync_photos(window, cx);
        let layout = Layout::for_window(window);
        if let Some(path) = &self.photo {
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
        if let Some(player) = &self.player
            && self.player_visible
            && self.player_exit.is_none()
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
        if let Some(started) = self.library_return {
            if !cx.reduce_motion()
                && started.elapsed().as_secs_f32() < wmc_motion::player_return::RETURN_DURATION
            {
                window.request_animation_frame();
            } else {
                self.library_return = None;
            }
        }
        let song_alpha = self.song_transition.map_or(1., |started| {
            let elapsed = started.elapsed().as_secs_f32();
            if elapsed < wmc_motion::player_entry::LIBRARY_EXIT_SECONDS && !cx.reduce_motion() {
                window.request_animation_frame();
            }
            wmc_motion::player_entry::library_exit(elapsed, cx.reduce_motion())
        });
        let launch_time = self
            .launch_epoch
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f32();
        let launching =
            !self.launch_dismissed && !cx.reduce_motion() && launch_time < LAUNCH_DURATION;
        let arrival = HomeArrival::sample(
            launch_time,
            self.state.category_index(),
            CATEGORIES.len(),
            self.launch_dismissed || cx.reduce_motion(),
        );
        if launching || arrival.moving {
            window.request_animation_frame();
        }
        let now = self.motion_epoch.elapsed();
        self.category_motion
            .retarget(self.state.category_index(), now);
        let motion = self.category_motion.sample(now, cx.reduce_motion());
        if motion.moving {
            window.request_animation_frame();
        }
        let page = transition(
            "destination-presence",
            if self.state.opened().is_some() || self.queue_visible {
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
                if !this.navigate_details(Direction::Right, cx) { window.focus_next(cx); }
            }))
            .on_action(cx.listener(|this, _: &DetailsPrevious, window, cx| {
                if !this.navigate_details(Direction::Left, cx) { window.focus_prev(cx); }
            }))
            .on_action(cx.listener(move |this, _: &ContextMenu, window, cx| {
                let target = if let wmc_core::DestinationKind::Library(kind) =
                    this.destination_state.kind
                    && this.state.opened().is_some()
                    && this.destination_state.focus == wmc_core::DestinationFocus::Content
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
        if self.player_exit.is_some()
            && let Some(player) = &self.player
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
