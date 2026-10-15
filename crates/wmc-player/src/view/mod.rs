mod actions;
mod host;
mod translation;
mod auto_hide;
mod background;
mod button;
mod chrome;
mod controls;
mod frame;
mod layout;
mod lyrics;
mod metadata;
mod more;
mod settings;
mod settings_ui;
mod wmc_skin;
mod online_lyrics;
mod online_covers;
mod media_cache;
mod resources;
mod system_media;
mod system_volume;
mod text;

use crate::{audio::AudioPlayer, lyrics::Lyrics, presentation::LyricsMotion, track::PlayerConfig};
use crate::{diagnostics as perf, track::PreparedTrack};
use gpui_kit::{prelude::*, *};
use gpui_kit::base::slider::{SliderEvent, SliderState};
use std::{sync::Arc, time::Instant};

pub use actions::bind_keys;
pub use system_media::initialize_app_identity;
#[derive(Clone, Copy)]
pub enum PlayerEvent {
    Back,
    Queue,
    Previous,
    Next,
}

/// Self-contained GPUI page. Host owns navigation/chrome; the player owns audio.
pub struct PlayerView {
    focus: FocusHandle,
    audio: AudioPlayer,
    lyrics: Lyrics,
    title: String,
    artist: String,
    cover: Arc<RenderImage>,
    background: background::BackgroundState,
    motion: LyricsMotion,
    lyric_freeze: Option<(f64, Instant)>,
    lyric_images: lyrics::cache::LineImageCache,
    retired_lyrics: Vec<lyrics::cache::LineImageCache>,
    layouts: lyrics::layout::LayoutCache,
    last_frame: Instant,
    epoch: Instant,
    animate_background: bool,
    standalone: bool,
    dragging: bool,
    progress_slider: Entity<SliderState>,
    progress_focus: FocusHandle,
    _progress_subscription: Subscription,
    status: Option<String>,
    show_queue: bool,
    volume_changed: Option<Instant>,
    system_volume: crate::system_volume::Controller,
    volume_state: Option<crate::system_volume::State>,
    volume_error: Option<String>,
    _volume_task: Task<()>,
    _font_loader: Task<()>,
    show_more: bool,
    settings: settings::Settings,
    system_media: crate::media::MediaSession,
    media_tasks: Vec<Task<()>>,
    online_query: crate::online_lyrics::SearchQuery,
    online_candidates: Vec<crate::online_lyrics::Candidate>,
    online_open: bool,
    online_busy: bool,
    online_generation: u64,
    online_message: String,
    lyric_source: Option<String>,
    _online_task: Option<Task<()>>,
    cover_candidates: Vec<(crate::online_covers::CoverCandidate, Option<Arc<RenderImage>>)>,
    cover_open: bool,
    cover_busy: bool,
    cover_generation: u64,
    cover_art_generation: u64,
    cover_message: String,
    cover_source: Option<String>,
    _cover_task: Option<Task<()>>,
    media_identity: Option<crate::media_cache::MediaIdentity>,
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
    pub fn demo(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::from_prepared(
            PlayerConfig::default()
                .prepare()
                .expect("embedded demo assets"),
            false,
            window,
            cx,
        )
    }

    pub fn from_config(
        config: PlayerConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<Self> {
        Ok(Self::from_prepared(config.prepare()?, false, window, cx))
    }

    pub fn from_prepared(
        track: PreparedTrack,
        standalone: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let online_query = crate::online_lyrics::SearchQuery {
            title: track.title.clone(),
            artist: if track.artist == "未知歌手" {
                String::new()
            } else {
                track.artist.clone()
            },
            album: track.album.clone(),
            duration_ms: Some((track.audio.duration() * 1000.).round() as u64),
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let progress_focus = cx.focus_handle();
        let progress_slider = cx.new(|_| SliderState::new().max(1.0).step(0.0001));
        let progress_subscription = cx.subscribe_in(
            &progress_slider,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                if this.settings.embedding { return; }
                match event {
                    SliderEvent::Change(_) => {
                        this.dragging = true;
                        window.focus(&this.progress_focus, cx);
                    }
                    SliderEvent::Release(value) => {
                        this.dragging = false;
                        this.seek(f64::from(value.end()) * this.audio.duration(), cx);
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
        // SMTC only needs a small thumbnail, not a second full-resolution cover.
        let media_cover = image::imageops::thumbnail(&track.cover, 256, 256);
        let (system_volume, volume_task) = Self::start_system_volume(cx);
        let mut view = Self {
            focus,
            audio: {
                let mut audio = AudioPlayer::new(track.audio);
                if perf::enabled() {
                    audio.toggle_mute();
                    if let Ok(start) = std::env::var("WMC_PLAYER_PROFILE_START")
                        && let Ok(seconds) = start.parse::<f64>()
                        && seconds.is_finite()
                    {
                        let _ = audio.seek(seconds);
                    }
                    audio.toggle();
                }
                audio
            },
            title: track.title,
            artist: track.artist,
            motion: LyricsMotion::new(track.lyrics.lines.len()),
            lyric_freeze: None,
            lyric_images: lyrics::cache::LineImageCache::new(None, track.lyrics.lines.len()),
            retired_lyrics: Vec::new(),
            layouts: lyrics::layout::LayoutCache::default(),
            lyrics: track.lyrics,
            cover: resources::render_image(track.cover),
            background: background::BackgroundState::new(track.backdrop, resources::background_image(track.background)),
            last_frame: Instant::now(),
            epoch: Instant::now(),
            animate_background: true,
            standalone,
            dragging: false,
            progress_slider,
            progress_focus,
            _progress_subscription: progress_subscription,
            status: None,
            show_queue: false,
            volume_changed: None,
            system_volume,
            volume_state: None,
            volume_error: None,
            _volume_task: volume_task,
            _font_loader: font_loader,
            show_more: false,
            settings: settings::Settings::new(&online_query,track.lyric_content,track.media_path.clone(),window,cx),
            system_media: crate::media::MediaSession::default(),
            media_tasks: Vec::new(),
            online_query,
            online_candidates: Vec::new(),
            online_open: false,
            online_busy: false,
            online_generation: 0,
            online_message: String::new(),
            lyric_source: None,
            _online_task: None,
            cover_candidates: Vec::new(),
            cover_open: false,
            cover_busy: false,
            cover_generation: 0,
            cover_art_generation: 0,
            cover_message: String::new(),
            cover_source: None,
            _cover_task: None,
            media_identity: None,
            _asset_task: None,
            auto_hide: auto_hide::AutoHide::new(Instant::now()),
            translation: translation::TranslationState::new(window, cx),
        };
        view.attach_system_media(&track.album, &media_cover, cx);
        view.restore_translation_cache(cx);
        if track.media_path.is_some() {
            view.resolve_cached_assets(track.media_path, track.lyrics_cache_allowed, track.cover_cache_allowed, track.cover_online_needed, track.local_sources, cx);
        }
        view
    }

    pub fn pause(&mut self) {
        self.audio.pause();
        self.sync_system_media();
    }

    pub fn play(&mut self) {
        if self.settings.embedding {return;}
        self.audio.play();
        self.sync_system_media();
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                        BoxShadow::new(px(0.), px(12.), rgb(0x000000).opacity(100./255.).into())
                            .blur_radius(px(30.)),
                    ])
                    .child(
                        img(self.cover.clone())
                            .size_full()
                            .rounded(unit(columns.cover_size * 0.03))
                            .object_fit(ObjectFit::Contain),
                    ),
            )
            .child(
                self.metadata(&frame, cx)
                    .opacity(arrival.details_alpha),
            );
        root = root
            .child(self.lyric_view(&frame, cx))
            .child(
                self.progress(
                    scale,
                    columns.info_left,
                    columns.info_top + 50.,
                    columns.info_width,
                    frame.seconds,
                    frame.duration,
                    window,
                    cx,
                )
                .opacity(arrival.details_alpha),
            )
            .when(chrome_alpha > 0.001, |root| root.child(
                self.transport(frame.chrome_scale, frame.playing, cx)
                    .opacity(arrival.background_alpha * chrome_alpha),
            ))
            .when(
                self.status.is_some() || self.audio.error.is_some(),
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
                            } else if self.audio.error.is_some() {
                                "声音设备不可用 · 仅预览歌词".into()
                            } else {
                                String::new()
                            }),
                    )
                },
            );
        if self.standalone && chrome_alpha > 0.001 {
            root = root.child(self.chrome(frame.chrome_scale, window, cx).opacity(chrome_alpha));
        }
        if self.show_queue {
            root = root.child(
                div()
                    .absolute()
                    .right(unit(36.))
                    .bottom(unit(68.))
                    .w(unit(380.))
                    .p(unit(22.))
                    .bg(rgb(0x071c35).opacity(0.95))
                    .border_1()
                    .border_color(rgb(0xd9edff).opacity(0.4))
                    .child(
                        div()
                            .text_size(unit(16.))
                            .text_color(rgb(0x91c5de))
                            .child("播放队列 · 1 首"),
                    )
                    .child(
                        div()
                            .id("player-queue-track")
                            .mt(unit(12.))
                            .text_size(unit(22.))
                            .cursor_pointer()
                            .child(self.title.clone())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.seek(0., cx);
                                this.show_queue = false;
                                cx.notify();
                            })),
                    ),
            );
        }
        if self.show_more { root = root.child(self.more_menu(&frame, cx)); }
        if self.online_open {
            root = root.child(self.online_panel(window, cx));
        }
        if self.cover_open { root = root.child(self.cover_panel(window, cx)); }
        if self.settings.info_open {root=root.child(self.lyric_info_panel(window,cx));}
        if self.translation.open { root = root.child(self.translation_panel(window, cx)); }
        root
    }
}
