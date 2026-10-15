//! Master output volume, independent of rodio and system media transport controls.
//! volumecontrol selects WASAPI / CoreAudio / PulseAudio at compile time.
use std::{sync::mpsc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct State {
    pub percent: u8,
    pub muted: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Command {
    Change(i16),
    ToggleMute,
}

pub(crate) struct Update {
    pub result: Result<State, String>,
    pub interacted: bool,
}

pub(crate) struct Controller {
    commands: mpsc::SyncSender<Command>,
}

impl Controller {
    pub fn start() -> (Self, async_channel::Receiver<Update>) {
        let (commands, receiver) = mpsc::sync_channel(32);
        let (updates, result) = async_channel::bounded(32);
        // Keep native objects on this thread: the PulseAudio backend is not Send.
        // Blocking device queries must not stall GPUI or lyric animation.
        let started = std::thread::Builder::new()
            .name("system-volume".into())
            .spawn(move || {
                let mut previous = None;
                let mut command = None;
                loop {
                    if updates.is_closed() {
                        break;
                    }
                    let result = execute(command).map_err(|error| error.to_string());
                    if command.is_some() || previous.as_ref() != Some(&result) {
                        previous = Some(result.clone());
                        if updates
                            .send_blocking(Update {
                                result,
                                interacted: command.is_some(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    command = match receiver.recv_timeout(Duration::from_millis(500)) {
                        Ok(command) => Some(command),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                }
            });
        if let Err(error) = started {
            // The sender lives in the failed closure; report through a fresh channel.
            let (updates, result) = async_channel::bounded(1);
            let _ = updates.try_send(Update {
                result: Err(format!("无法启动系统音量控制：{error}")),
                interacted: false,
            });
            return (Self { commands }, result);
        }
        (Self { commands }, result)
    }

    pub fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => "系统音量控制繁忙".into(),
                mpsc::TrySendError::Disconnected(_) => "系统音量控制不可用".into(),
            })
    }
}

trait OutputDevice {
    fn state(&self) -> anyhow::Result<State>;
    fn set_volume(&self, percent: u8) -> anyhow::Result<()>;
    fn set_mute(&self, muted: bool) -> anyhow::Result<()>;
}

fn apply(device: &impl OutputDevice, command: Option<Command>) -> anyhow::Result<State> {
    // Always read the current OS state, including changes from media keys/other apps.
    let current = device.state()?;
    match command {
        Some(Command::Change(delta)) => {
            let target = (i32::from(current.percent) + i32::from(delta)).clamp(0, 100) as u8;
            device.set_volume(target)?;
            device.set_mute(false)?;
        }
        Some(Command::ToggleMute) => device.set_mute(!current.muted)?,
        None => return Ok(current),
    }
    device.state()
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
impl OutputDevice for volumecontrol::AudioDevice {
    fn state(&self) -> anyhow::Result<State> {
        Ok(State {
            percent: self.get_vol()?,
            muted: self.is_mute()?,
        })
    }
    fn set_volume(&self, percent: u8) -> anyhow::Result<()> {
        Ok(self.set_vol(percent)?)
    }
    fn set_mute(&self, muted: bool) -> anyhow::Result<()> {
        Ok(volumecontrol::AudioDevice::set_mute(self, muted)?)
    }
}

fn execute(command: Option<Command>) -> anyhow::Result<State> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        // Re-resolve the default output each time, so switching headphones/devices
        // does not leave the buttons controlling a previous endpoint.
        let device = volumecontrol::AudioDevice::from_default()?;
        apply(&device, command)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = command;
        anyhow::bail!("此平台暂不支持系统音量控制")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Device(Cell<State>);
    impl OutputDevice for Device {
        fn state(&self) -> anyhow::Result<State> {
            Ok(self.0.get())
        }
        fn set_volume(&self, percent: u8) -> anyhow::Result<()> {
            self.0.set(State {
                percent,
                ..self.0.get()
            });
            Ok(())
        }
        fn set_mute(&self, muted: bool) -> anyhow::Result<()> {
            self.0.set(State {
                muted,
                ..self.0.get()
            });
            Ok(())
        }
    }

    #[test]
    fn changes_use_live_state_clamp_and_unmute_without_losing_saved_volume() {
        let device = Device(Cell::new(State {
            percent: 98,
            muted: true,
        }));
        assert_eq!(
            apply(&device, Some(Command::Change(5))).unwrap(),
            State {
                percent: 100,
                muted: false
            }
        );
        // Simulate a volume change made outside the app.
        device.0.set(State {
            percent: 3,
            muted: false,
        });
        assert_eq!(
            apply(&device, Some(Command::Change(-5))).unwrap().percent,
            0
        );
        device.0.set(State {
            percent: 42,
            muted: false,
        });
        assert_eq!(
            apply(&device, Some(Command::ToggleMute)).unwrap(),
            State {
                percent: 42,
                muted: true
            }
        );
        assert_eq!(
            apply(&device, None).unwrap(),
            State {
                percent: 42,
                muted: true
            }
        );
        assert_eq!(
            apply(&device, Some(Command::ToggleMute)).unwrap(),
            State {
                percent: 42,
                muted: false
            }
        );
    }

    #[test]
    #[ignore = "Reads the real default output; run explicitly on the target OS"]
    fn native_default_output_is_readable() {
        let state = execute(None).expect("read system output volume");
        assert!(state.percent <= 100);
        println!("System output: {}%, muted={}", state.percent, state.muted);
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[ignore = "Briefly lowers/mutes real system output and restores it; run explicitly"]
    fn native_output_controls_read_back_and_restore() {
        let device = volumecontrol::AudioDevice::from_default().expect("default output");
        let original = device.state().expect("original system volume");
        let probe = (|| -> anyhow::Result<()> {
            let target = original.percent.saturating_sub(1);
            device.set_vol(target)?;
            anyhow::ensure!(device.get_vol()?.abs_diff(target) <= 1, "volume readback");
            device.set_mute(true)?;
            anyhow::ensure!(device.is_mute()?, "mute readback");
            Ok(())
        })();
        // Restore before asserting, including when any native operation failed.
        let restore_volume = device.set_vol(original.percent);
        let restore_mute = device.set_mute(original.muted);
        restore_volume.expect("restore system volume");
        restore_mute.expect("restore system mute");
        probe.expect("native system volume controls");
        println!(
            "System volume/mute readback passed; restored {}%, muted={}",
            original.percent, original.muted
        );
    }
}
