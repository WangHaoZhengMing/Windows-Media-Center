//! UI bridge for system master-volume commands and asynchronous state updates.
use super::PlayerView;
use crate::system_volume::{Command, Controller};
use gpui_kit::*;
use std::time::Instant;

impl PlayerView {
    pub(super) fn start_system_volume(cx: &mut Context<Self>) -> (Controller, Task<()>) {
        let (controller, updates) = Controller::start();
        let task = cx.spawn(async move |this, cx| {
            while let Ok(update) = updates.recv().await {
                if this
                    .update(cx, |this, cx| {
                        match update.result {
                            Ok(state) => {
                                this.volume_state = Some(state);
                                this.volume_error = None;
                            }
                            Err(error) => {
                                this.volume_state = None;
                                this.volume_error = Some(error);
                            }
                        }
                        if update.interacted {
                            this.volume_changed = Some(Instant::now());
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        (controller, task)
    }

    pub(super) fn change_volume(&mut self, amount: i16, cx: &mut Context<Self>) {
        self.volume_command(Command::Change(amount), cx);
    }

    pub(super) fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.volume_command(Command::ToggleMute, cx);
    }

    fn volume_command(&mut self, command: Command, cx: &mut Context<Self>) {
        if let Err(error) = self.system_volume.send(command) {
            self.volume_error = Some(error);
            self.volume_changed = Some(Instant::now());
            cx.notify();
        }
    }
}
