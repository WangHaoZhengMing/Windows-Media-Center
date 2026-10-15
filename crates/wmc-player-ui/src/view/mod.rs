mod panels;
use panels::{settings, translation};
mod actions;
mod auto_hide;
mod host;
use crate::backdrop as background;
mod button;
mod controls;
pub(crate) mod frame;
mod layout;
mod lyrics;
mod media_cache;
mod metadata;
mod wmc_skin;
use wmc_design::images as resources;
mod application_state;
mod playback;
mod text;

use crate::diagnostics as perf;
use gpui_kit::base::slider::{SliderEvent, SliderState};
use gpui_kit::{prelude::*, *};
use std::{sync::Arc, time::Instant};
use wmc_application::{Application, TrackPresentation};
use wmc_lyrics::Lyrics;
use wmc_motion::lyrics::scroll::LyricsMotion;

pub use actions::bind_keys;
#[derive(Clone, Copy)]
pub enum PlayerEvent {
    Back,
    Queue,
    Previous,
    Next,
}

/// GPUI presentation of an application-owned playback session.
pub struct PlayerView {
    focus: FocusHandle,
    pub(crate) app: Application,
    hide_cursor: std::rc::Rc<dyn Fn()>,
    pub(crate) lyrics: Lyrics,
    title: String,
    artist: String,
    cover: Arc<RenderImage>,
    pub(crate) background: background::BackgroundState,
    motion: LyricsMotion,
    lyric_freeze: Option<(f64, Instant)>,
    lyric_images: lyrics::cache::LineImageCache,
    retired_lyrics: Vec<lyrics::cache::LineImageCache>,
    layouts: lyrics::layout::LayoutCache,
    last_frame: Instant,
    epoch: Instant,
    pub(crate) animate_background: bool,
    dragging: bool,
    progress_slider: Entity<SliderState>,
    progress_focus: FocusHandle,
    _progress_subscription: Subscription,
    status: Option<String>,
    volume_changed: Option<Instant>,
    volume_state: Option<wmc_platform::volume::State>,
    volume_error: Option<String>,
    _font_loader: Task<()>,
    show_more: bool,
    settings: settings::Settings,
    online_query: wmc_enrichment::lyrics::TrackQuery,
    lyrics_search: panels::lyrics_search::SearchState,
    cover_search: panels::cover_search::SearchState,
    media_identity: Option<wmc_enrichment::cache::MediaIdentity>,
    _asset_task: Option<Task<()>>,
    auto_hide: auto_hide::AutoHide,
    translation: translation::TranslationState,
}

impl EventEmitter<PlayerEvent> for PlayerView {}
impl Focusable for PlayerView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PlayerView {
    pub fn new(
        track: TrackPresentation,
        app: Application,
        hide_cursor: std::rc::Rc<dyn Fn()>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let online_query = wmc_enrichment::lyrics::TrackQuery {
            title: track.title.clone(),
            artist: if track.artist == "未知歌手" {
                String::new()
            } else {
                track.artist.clone()
            },
            album: track.album.clone(),
            duration_ms: Some((track.duration * 1000.).round() as u64),
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let progress_focus = cx.focus_handle();
        let progress_slider = cx.new(|_| SliderState::new().max(1.0).step(0.0001));
        let progress_subscription = cx.subscribe_in(
            &progress_slider,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                if this.settings.embedding {
                    return;
                }
                match event {
                    SliderEvent::Change(_) => {
                        this.dragging = true;
                        window.focus(&this.progress_focus, cx);
                    }
                    SliderEvent::Release(value) => {
                        this.dragging = false;
                        this.seek(f64::from(value.end()) * this.app.duration(), cx);
                    }
                }
                cx.notify();
            },
        );
        let lyric_text: String = track.lyrics.lines.iter().map(|line| line.text()).collect();
        let font_loader = cx.spawn(async move |this, cx| {
            let font = cx
                .background_executor()
                .spawn(async move { lyrics::font::SongFont::load(&lyric_text) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.lyric_images.set_font(font);
                cx.notify();
            });
        });
        let cover = track.cover.unwrap_or_else(|| {
            let bytes = wmc_assets::load(wmc_assets::Shortcut::MusicFavorites.images().focused)
                .expect("embedded fallback artwork");
            let art = image::load_from_memory(&bytes)
                .expect("valid fallback artwork")
                .to_rgba8();
            image::imageops::crop_imm(&art, 48, 22, 82, 91).to_image()
        });
        let backdrop = crate::backdrop::Backdrop::from_cover(&cover);
        let background_image = backdrop.render(0., 256, 144);
        let mut view = Self {
            focus,
            app: app.clone(),
            hide_cursor,
            title: track.title,
            artist: track.artist,
            motion: LyricsMotion::new(track.lyrics.lines.len()),
            lyric_freeze: None,
            lyric_images: lyrics::cache::LineImageCache::new(None, track.lyrics.lines.len()),
            retired_lyrics: Vec::new(),
            layouts: lyrics::layout::LayoutCache::default(),
            lyrics: track.lyrics,
            cover: resources::render_image(cover),
            background: background::BackgroundState::new(
                backdrop,
                resources::background_image(background_image),
            ),
            last_frame: Instant::now(),
            epoch: Instant::now(),
            animate_background: true,
            dragging: false,
            progress_slider,
            progress_focus,
            _progress_subscription: progress_subscription,
            status: None,
            volume_changed: None,
            volume_state: None,
            volume_error: None,
            _font_loader: font_loader,
            show_more: false,
            settings: settings::Settings::new(
                &online_query,
                track.lyric_content,
                track.media_path.clone(),
                window,
                cx,
            ),
            online_query,
            lyrics_search: panels::lyrics_search::SearchState::default(),
            cover_search: panels::cover_search::SearchState::default(),
            media_identity: None,
            _asset_task: None,
            auto_hide: auto_hide::AutoHide::new(Instant::now()),
            translation: translation::TranslationState::new(&app, window, cx),
        };
        view.restore_translation_cache(cx);
        if track.media_path.is_some() {
            view.resolve_cached_assets(
                track.media_path,
                track.lyrics_cache_allowed,
                track.cover_cache_allowed,
                track.cover_online_needed,
                track.local_sources,
                cx,
            );
        }
        view
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_application_state();
        let _profile = perf::Timer::frame(self.last_frame.elapsed());
        window.request_animation_frame();
        let chrome_alpha = self.update_chrome_visibility(window, cx);
        let frame = self.frame(window, cx);
        self.update_background(&frame, window, cx);
        self.prepare_lyric_images(&frame, window, cx);
        let scale = frame.scale;
        let columns = &frame.columns;
        let arrival = frame.arrival;
        let unit = |v| px(v * scale);
        let mut root = self
            .root(cx)
            .child(self.background_layer(&frame))
            .child(
                div()
                    .absolute()
                    .left(unit(columns.cover_left))
                    .top(unit(columns.cover_top))
                    .size(unit(columns.cover_size))
                    .opacity(arrival.cover_alpha)
                    .rounded(unit(columns.cover_size * 0.03))
                    .overflow_hidden()
                    .shadow(vec![
                        BoxShadow::new(px(0.), px(12.), rgb(0x000000).opacity(100. / 255.).into())
                            .blur_radius(px(30.)),
                    ])
                    .child(
                        img(self.cover.clone())
                            .size_full()
                            .rounded(unit(columns.cover_size * 0.03))
                            .object_fit(ObjectFit::Contain),
                    ),
            )
            .child(self.metadata(&frame, cx).opacity(arrival.details_alpha));
        root = root
            .child(self.lyric_view(&frame, cx))
            .child(
                self.progress(&frame, window, cx)
                    .opacity(arrival.details_alpha),
            )
            .when(chrome_alpha > 0.001, |root| {
                root.child(
                    self.transport(frame.chrome_scale, frame.playing, cx)
                        .opacity(arrival.background_alpha * chrome_alpha),
                )
            })
            .when(
                self.status.is_some() || self.app.error().is_some(),
                |root| {
                    root.child(
                        div()
                            .absolute()
                            .left(unit(columns.cover_left))
                            .bottom(unit(24.))
                            .text_size(unit(12.))
                            .text_color(rgb(0xc5d9e8).opacity(0.55))
                            .child(if let Some(status) = &self.status {
                                status.clone()
                            } else if self.app.error().is_some() {
                                "声音设备不可用 · 仅预览歌词".into()
                            } else {
                                String::new()
                            }),
                    )
                },
            );

        if self.show_more {
            root = root.child(self.more_menu(&frame, cx));
        }
        if self.lyrics_search.open {
            root = root.child(self.online_panel(window, cx));
        }
        if self.cover_search.open {
            root = root.child(self.cover_panel(window, cx));
        }
        if self.settings.info_open {
            root = root.child(self.lyric_info_panel(window, cx));
        }
        if self.translation.open {
            root = root.child(self.translation_panel(window, cx));
        }
        root
    }
}
