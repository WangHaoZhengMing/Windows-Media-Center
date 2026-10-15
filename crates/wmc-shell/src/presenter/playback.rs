use crate::navigation::{CATEGORIES, EntryId, MediaCommand};
use crate::shell::AppShell;
use gpui_kit::{prelude::*, *};
use std::time::Duration;
use wmc_design::theme::{Layout, WHITE, color};
impl AppShell {
    pub(crate) fn start_playback_monitor(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| {
                        this.app.poll();
                        if this.app.take_finished() {
                            this.step_queue(true, window, cx);
                        }
                        if this.app.has_track() {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
    pub(crate) fn step_queue(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.app.queue_target(forward) {
            self.load_queue_song(index, self.player_page.visible, window, cx);
        }
    }
    pub(crate) fn clear_queue(&mut self, cx: &mut Context<Self>) {
        self.app.clear_queue();
        self.player_page.entering = None;
        cx.notify();
    }
    pub(crate) fn show_queue(&mut self, cx: &mut Context<Self>) {
        if self.player_page.visible {
            self.return_from_player(cx);
        }
        self.preview = CATEGORIES
            .iter()
            .flat_map(|c| c.entries)
            .find(|e| e.id == EntryId::PlayQueue);
        self.queue_visible = true;
        self.destination_state
            .reset(EntryId::PlayQueue, self.ambient_motion);
        cx.notify();
    }
    pub(crate) fn media_command(
        &mut self,
        command: MediaCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if command == MediaCommand::Stop {
            self.app.cancel_load();
            self.player_page.entering = None;
        }
        match command {
            MediaCommand::Next => self.step_queue(true, window, cx),
            MediaCommand::Previous => self.step_queue(false, window, cx),
            MediaCommand::PlayPause if !self.app.has_track() => {
                if !self.app.queue().entries().is_empty() {
                    self.load_queue_song(0, false, window, cx);
                }
            }
            _ => self.app.dispatch(command),
        }
        cx.notify();
    }
    pub(crate) fn ensure_player(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.player_page.view.is_some() {
            return;
        }
        let Some(track) = self.app.current_track() else {
            return;
        };
        let app = self.app.clone();
        let player = cx.new(|cx| {
            wmc_player_ui::PlayerView::new(track, app, self.hide_cursor.clone(), window, cx)
        });
        self.player_page.subscription = Some(cx.subscribe_in(
            &player,
            window,
            |this, _, event: &wmc_player_ui::PlayerEvent, window, cx| match event {
                wmc_player_ui::PlayerEvent::Back => this.return_from_player(cx),
                wmc_player_ui::PlayerEvent::Queue => this.show_queue(cx),
                wmc_player_ui::PlayerEvent::Previous => this.step_queue(false, window, cx),
                wmc_player_ui::PlayerEvent::Next => this.step_queue(true, window, cx),
            },
        ));
        self.player_page.view = Some(player);
    }
    pub(crate) fn now_playing(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let Some((title, artist)) = self.app.track_summary() else {
            return div();
        };
        div()
            .absolute()
            .left(layout.unit(24.))
            .bottom(layout.unit(16.))
            .child(
                wmc_design::button("home-now-playing", "打开正在播放")
                    .flex()
                    .items_center()
                    .justify_start()
                    .gap(layout.unit(12.))
                    .max_w(layout.unit(410.))
                    .cursor_pointer()
                    .text_color(color(WHITE))
                    .child(
                        img(self
                            .player_page
                            .cover
                            .clone()
                            .expect("current track thumbnail"))
                        .size(layout.unit(64.))
                        .rounded(layout.unit(64. * 0.03))
                        .object_fit(ObjectFit::Cover),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(layout.unit(19.))
                                    .text_ellipsis()
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(layout.unit(15.))
                                    .text_color(color(WHITE).opacity(0.7))
                                    .text_ellipsis()
                                    .child(artist),
                            ),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.ensure_player(window, cx);
                        this.player_page.exiting = None;
                        this.player_page.visible = true;
                        if let Some(player) = &this.player_page.view {
                            window.focus(&player.focus_handle(cx), cx);
                        }
                        cx.notify();
                    })),
            )
    }
}

impl AppShell {
    pub(crate) fn load_song(
        &mut self,
        path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .library
            .items
            .iter()
            .find(|item| item.file.path == path)
            .map(|item| item.title.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        let index = self.app.find_or_enqueue(path, title);
        self.load_queue_song(index, true, window, cx);
    }
    pub(crate) fn load_queue_song(
        &mut self,
        index: usize,
        foreground: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(load) = self.app.begin_load(index) else {
            return;
        };
        let started = std::time::Instant::now();
        self.player_page.entering = foreground.then_some(started);
        self.player_page.library_return = None;
        self.library.error = None;
        let reduced = cx.reduce_motion();
        cx.spawn_in(window, async move |this, cx| {
            let path = load.path.clone();
            let prepared = cx
                .background_executor()
                .spawn(async move {
                    wmc_application::PlayerConfig {
                        audio: Some(path),
                        ..Default::default()
                    }
                    .prepare()
                })
                .await;
            if foreground && !reduced && prepared.is_ok() {
                let duration =
                    Duration::from_secs_f32(wmc_motion::player_entry::LIBRARY_EXIT_SECONDS);
                if let Some(remaining) = duration.checked_sub(started.elapsed()) {
                    cx.background_executor().timer(remaining).await;
                }
            }
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.app.accepts(&load) {
                    return;
                }
                this.player_page.entering = None;
                match prepared {
                    Ok(mut track) => {
                        this.player_page.subscription = None;
                        if let Some(player) = this.player_page.view.take() {
                            player.update(cx, |player, _| player.release(window));
                        }
                        if let Some(cover) = this.player_page.cover.take() {
                            let _ = window.drop_image(cover);
                        }
                        if !this.app.install(&load, &mut track) {
                            return;
                        }
                        let cover = track.cover.unwrap_or_else(|| {
                            image::RgbaImage::from_pixel(1, 1, image::Rgba([20, 40, 60, 255]))
                        });
                        this.player_page.cover = Some(wmc_design::images::render_image(
                            image::imageops::thumbnail(&cover, 64, 64),
                        ));
                        this.app.play();
                        this.player_page.visible = foreground || this.player_page.visible;
                        if this.player_page.visible {
                            this.player_page.exiting = None;
                            this.ensure_player(window, cx);
                        } else {
                            window.focus(&this.focus, cx);
                        }
                    }
                    Err(error) => {
                        this.app.fail_load(&load);
                        this.library.error = Some(format!("歌曲加载失败：{error:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
