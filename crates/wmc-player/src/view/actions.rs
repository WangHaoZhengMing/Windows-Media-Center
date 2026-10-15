//! Key bindings, pointer actions and navigation behavior.
use super::{PlayerEvent, PlayerView};
use gpui_kit::{prelude::*, *};
use std::time::Instant;

actions!(
    music_player,
    [
        TogglePlayback,
        Back,
        SeekBack,
        SeekForward,
        PreviousLine,
        NextLine,
        ToggleMotion,
        ToggleMute,
        VolumeDown,
        VolumeUp,
        Fullscreen,
        SearchLyrics,
        SearchCover,
        TranslateLyrics
    ]
);

pub fn bind_keys(cx: &mut App) {
    wmc_controls::init(cx);
    cx.bind_keys([
        KeyBinding::new("space", TogglePlayback, Some("MusicPlayer")),
        KeyBinding::new("enter", TogglePlayback, Some("MusicPlayer")),
        KeyBinding::new("backspace", Back, Some("MusicPlayer")),
        KeyBinding::new("escape", Back, Some("MusicPlayer")),
        KeyBinding::new("escape", Back, Some("PlayerSettings")),
        KeyBinding::new("left", SeekBack, Some("MusicPlayer")),
        KeyBinding::new("right", SeekForward, Some("MusicPlayer")),
        KeyBinding::new("up", PreviousLine, Some("MusicPlayer")),
        KeyBinding::new("down", NextLine, Some("MusicPlayer")),
        KeyBinding::new("b", ToggleMotion, Some("MusicPlayer")),
        KeyBinding::new("m", ToggleMute, Some("MusicPlayer")),
        KeyBinding::new("ctrl-down", VolumeDown, Some("MusicPlayer")),
        KeyBinding::new("ctrl-up", VolumeUp, Some("MusicPlayer")),
        KeyBinding::new("f11", Fullscreen, Some("MusicPlayer")),
        KeyBinding::new("ctrl-l", SearchLyrics, Some("MusicPlayer")),
        KeyBinding::new("ctrl-shift-c", SearchCover, Some("MusicPlayer")),
        KeyBinding::new("ctrl-t", TranslateLyrics, Some("MusicPlayer")),
    ]);
}

impl PlayerView {
    pub(super) fn seek(&mut self, seconds: f64, cx: &mut Context<Self>) {
        if self.settings.embedding {return;}
        self.motion.seek();
        self.lyric_freeze = None;
        self.status = self
            .audio
            .seek(seconds)
            .err()
            .map(|e| format!("无法跳转：{e}"));
        self.sync_system_media();
        cx.notify();
    }

    pub(super) fn seek_lyric(&mut self, seconds: f64, cx: &mut Context<Self>) {
        self.seek(seconds, cx);
        if self.status.is_none() {
            self.lyric_freeze = Some((seconds, Instant::now()));
        }
    }

    pub(super) fn seek_line(&mut self, direction: isize, cx: &mut Context<Self>) {
        if self.lyrics.lines.is_empty() {
            return;
        }
        let current = self.lyrics.active_line(self.audio.position()).unwrap_or(0);
        let next = current
            .saturating_add_signed(direction)
            .min(self.lyrics.lines.len() - 1);
        self.seek_lyric(self.lyrics.lines[next].start, cx);
    }

    pub(super) fn back(&mut self, cx: &mut Context<Self>) {
        if self.translation.open { self.translation.open = false; cx.notify(); return; }
        if self.settings.info_open { self.settings.info_open=false;cx.notify();return; }
        if self.show_more { self.show_more = false; cx.notify(); return; }
        if self.cover_open {
            self.close_cover_search();
            cx.notify();
            return;
        }
        if self.online_open {
            self.online_open = false;
            cx.notify();
            return;
        }
        self.online_generation += 1;
        self._online_task = None;
        self.cover_generation += 1;
        self._cover_task = None;
        self._asset_task = None;
        self.translation.stop();
        if self.standalone {
            self.pause();
            self.detach_system_media();
            cx.quit();
        } else {
            cx.emit(PlayerEvent::Back);
        }
    }
}

impl PlayerView {
    pub(super) fn root(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("music-player")
            .key_context(if self.online_open || self.cover_open || self.settings.info_open || self.translation.open {"PlayerSettings"} else {"MusicPlayer"})
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x101014))
            .text_color(rgb(0xeef5ff))
            .font_family("Microsoft YaHei UI")
            .on_action(cx.listener(|this, _: &TogglePlayback, _, cx| {
                this.toggle_playback(cx);
            }))
            .on_action(cx.listener(|this, _: &Back, _, cx| this.back(cx)))
            .on_action(cx.listener(|this, _: &SearchLyrics, _, cx| this.search_online_lyrics(cx)))
            .on_action(cx.listener(|this, _: &SearchCover, _, cx| this.search_online_cover(cx)))
            .on_action(cx.listener(|this, _: &TranslateLyrics, window, cx| this.open_translation(window, cx)))
            .on_action(
                cx.listener(|this, _: &SeekBack, _, cx| this.seek(this.audio.position() - 5., cx)),
            )
            .on_action(
                cx.listener(|this, _: &SeekForward, _, cx| {
                    this.seek(this.audio.position() + 5., cx)
                }),
            )
            .on_action(cx.listener(|this, _: &PreviousLine, _, cx| this.seek_line(-1, cx)))
            .on_action(cx.listener(|this, _: &NextLine, _, cx| this.seek_line(1, cx)))
            .on_action(cx.listener(|this, _: &ToggleMotion, _, cx| {
                this.animate_background = !this.animate_background;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleMute, _, cx| {
                this.toggle_mute(cx);
            }))
            .on_action(cx.listener(|this, _: &VolumeDown, _, cx| {
                this.change_volume(-5, cx);
            }))
            .on_action(cx.listener(|this, _: &VolumeUp, _, cx| {
                this.change_volume(5, cx);
            }))
            .on_action(cx.listener(|_, _: &Fullscreen, window, _| window.toggle_fullscreen()))
    }
}
