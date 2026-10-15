//! Apply OS commands on the GPUI thread and publish state even while minimized.
use super::PlayerView;
use crate::media::{Command, MediaSession, Metadata};
use gpui_kit::*;
use std::time::Duration;

/// Set the desktop app identity before opening windows. Other platforms are unchanged.
pub fn initialize_app_identity(cx: &App) {
    #[cfg(target_os = "windows")]
    cx.set_app_identity(crate::media::APP_ID, crate::media::APP_NAME);
    #[cfg(not(target_os = "windows"))]
    let _ = cx;
}

impl PlayerView {
    pub(super) fn attach_system_media(
        &mut self,
        album: &str,
        cover: &image::RgbaImage,
        cx: &mut Context<Self>,
    ) {
        if self.audio.error.is_some() {
            return;
        }
        // Also cover embedders that did not invoke the startup helper.
        initialize_app_identity(cx);
        let (sender, receiver) = crate::media::channel();
        self.system_media = match MediaSession::attach(
            Metadata {
                title: &self.title,
                artist: &self.artist,
                album,
                cover,
            },
            sender,
        ) {
            Ok(session) => session,
            Err(error) => {
                eprintln!("系统媒体控制不可用，继续本地播放：{error}");
                return;
            }
        };
        if !self.system_media.is_attached() {
            return;
        }
        self.sync_system_media();
        self.media_tasks.push(cx.spawn(async move |this, cx| {
            while let Ok(command) = receiver.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if this.system_media.is_attached() {
                            this.apply_media_command(command, cx);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.media_tasks.push(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                match this.update(cx, |this, _| {
                    this.sync_system_media();
                    this.system_media.is_attached()
                }) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        }));
    }

    pub(super) fn sync_system_media(&mut self) {
        if self.system_media.is_attached() {
            let _ = self.system_media.publish(self.audio.snapshot().into());
        }
    }

    pub(super) fn detach_system_media(&mut self) {
        self.system_media.detach();
        self.media_tasks.clear();
    }

    pub(super) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.settings.embedding {
            return;
        }
        self.audio.toggle();
        self.sync_system_media();
        cx.notify();
    }

    pub(super) fn stop(&mut self, cx: &mut Context<Self>) {
        if self.settings.embedding {
            return;
        }
        self.audio.stop();
        self.motion.seek();
        self.lyric_freeze = None;
        self.sync_system_media();
        cx.notify();
    }

    fn apply_media_command(&mut self, command: Command, cx: &mut Context<Self>) {
        match command {
            Command::Play => self.play(),
            Command::Pause => self.pause(),
            Command::Stop => self.stop(cx),
            Command::Restart => self.seek(0., cx),
            Command::SeekTo(seconds) if seconds.is_finite() => self.seek(seconds, cx),
            Command::SeekBy(seconds) if seconds.is_finite() => {
                self.seek(self.audio.position() + seconds, cx)
            }
            _ => {}
        }
        cx.notify();
    }
}
