use super::PlayerView;
use gpui_kit::*;
impl PlayerView {
    pub(super) fn sync_system_media(&mut self) {
        self.app.poll();
    }
    pub(super) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.settings.embedding {
            return;
        }
        self.app.toggle();
        cx.notify();
    }
    pub(super) fn stop(&mut self, cx: &mut Context<Self>) {
        self.app.stop();
        self.motion.seek();
        self.lyric_freeze = None;
        cx.notify();
    }
}
