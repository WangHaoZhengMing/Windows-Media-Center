//! OS media integration. Native callbacks send commands to the player's UI loop.
use async_channel::{Receiver, Sender};
use image::RgbaImage;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{APP_ID, APP_NAME};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Play,
    Pause,
    Stop,
    Restart,
    SeekTo(f64),
    SeekBy(f64),
}

pub struct Metadata<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub cover: &'a RgbaImage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug)]
pub struct Playback {
    pub status: Status,
    pub seconds: f64,
    pub duration: f64,
}

#[derive(Default)]
pub struct MediaSession {
    #[cfg(target_os = "windows")]
    backend: Option<windows::WindowsSession>,
    last: Option<Playback>,
    last_error: Option<String>,
}

impl MediaSession {
    pub fn attach(metadata: Metadata<'_>, sender: Sender<Command>) -> anyhow::Result<Self> {
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                backend: Some(windows::WindowsSession::new(metadata, sender)?),
                ..Self::default()
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (
                metadata.title,
                metadata.artist,
                metadata.album,
                metadata.cover,
                sender,
            );
            Ok(Self::default())
        }
    }

    pub fn publish(&mut self, playback: Playback) -> anyhow::Result<()> {
        if self.last.is_some_and(|last| {
            last.status == playback.status
                && last.seconds == playback.seconds
                && last.duration == playback.duration
        }) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        if let Some(backend) = &mut self.backend
            && let Err(error) = backend.publish(playback)
        {
            let message = error.to_string();
            if self.last_error.as_ref() != Some(&message) {
                eprintln!("系统媒体状态同步失败：{message}");
                self.last_error = Some(message);
            }
            return Err(error);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (playback.status, playback.seconds, playback.duration);
        self.last = Some(playback);
        self.last_error = None;
        Ok(())
    }

    pub fn update_metadata(&self, metadata: Metadata<'_>) -> anyhow::Result<()> {
        #[cfg(target_os = "windows")]
        if let Some(backend) = &self.backend {
            backend.set_metadata(metadata)?;
        }
        #[cfg(not(target_os = "windows"))]
        let _ = metadata;
        Ok(())
    }

    pub fn detach(&mut self) {
        #[cfg(target_os = "windows")]
        {
            self.backend.take();
        }
    }

    pub fn is_attached(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.backend.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}

pub fn channel() -> (Sender<Command>, Receiver<Command>) {
    async_channel::bounded(32)
}
