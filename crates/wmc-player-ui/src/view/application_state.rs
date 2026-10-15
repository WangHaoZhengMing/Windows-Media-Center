use super::PlayerView;
use gpui_kit::*;
use wmc_platform::volume::Command;
impl PlayerView {
    pub(super) fn refresh_application_state(&mut self) {
        self.app.poll();
        self.volume_state = self.app.volume_state();
        self.volume_error = self.app.volume_error();
        self.volume_changed = self.app.volume_changed();
        if self.settings.embedding && !self.app.embedding() {
            self.settings.embedding = false;
            self.media_identity = self.app.media_identity();
            if let Some(message) = self.app.embed_message() {
                self.status = Some(message.clone());
                self.settings.set_message(message);
            }
        }
    }
    pub(super) fn change_volume(&mut self, amount: i16, cx: &mut Context<Self>) {
        self.volume_command(Command::Change(amount), cx);
    }
    pub(super) fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.volume_command(Command::ToggleMute, cx);
    }
    fn volume_command(&mut self, command: Command, cx: &mut Context<Self>) {
        if let Err(error) = self.app.volume_command(command) {
            self.volume_error = Some(error);
            self.volume_changed = Some(std::time::Instant::now());
            cx.notify();
        }
    }
}
