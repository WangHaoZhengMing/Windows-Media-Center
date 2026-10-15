//! Playback controls and metadata shared with the containing application.
use super::PlayerView;
use gpui_kit::*;
use std::sync::Arc;
use wmc_core::MediaCommand;

impl PlayerView {
    pub fn leave_page(&mut self) {
        self.dragging = false;
    }

    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn artist(&self) -> &str {
        &self.artist
    }
    pub fn cover(&self) -> Arc<RenderImage> {
        self.cover.clone()
    }
    pub fn is_playing(&self) -> bool {
        self.audio.playing()
    }
    pub fn is_finished(&self) -> bool {
        self.audio.is_finished()
    }
    pub fn is_muted(&self) -> bool {
        self.volume_state.is_some_and(|state| state.muted)
    }

    pub fn deactivate(&mut self) {
        self.pause();
        self.detach_system_media();
    }

    pub fn dispatch(&mut self, command: MediaCommand, cx: &mut Context<Self>) {
        match command {
            MediaCommand::PlayPause => self.toggle_playback(cx),
            MediaCommand::Stop => self.stop(cx),
            MediaCommand::Rewind => self.seek(self.audio.position() - 5.0, cx),
            MediaCommand::FastForward => self.seek(self.audio.position() + 5.0, cx),
            MediaCommand::ToggleMute => self.toggle_mute(cx),
            MediaCommand::VolumeDown => self.change_volume(-5, cx),
            MediaCommand::VolumeUp => self.change_volume(5, cx),
            MediaCommand::Previous | MediaCommand::Next => {}
        }
        cx.notify();
    }
}
