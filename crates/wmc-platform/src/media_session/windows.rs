//! Manual Windows SMTC. A source-less MediaPlayer owns the session; rodio owns audio.
use super::{Command, Metadata, Playback, Status};
use async_channel::Sender;
use image::{ImageFormat, imageops};
use std::io::Cursor;
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, Playback::MediaPlayer,
        PlaybackPositionChangeRequestedEventArgs, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
    core::HSTRING,
};

pub const APP_ID: &str = "Wmc.MediaCenter.Player";
pub const APP_NAME: &str = "Windows Media Center";

fn register_identity() -> anyhow::Result<()> {
    let key = windows_registry::CURRENT_USER
        .create(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))?;
    key.set_string("DisplayName", APP_NAME)?;
    Ok(())
}

pub(super) struct WindowsSession {
    player: MediaPlayer,
    controls: SystemMediaTransportControls,
    button_token: Option<i64>,
    position_token: Option<i64>,
}

fn timespan(seconds: f64) -> TimeSpan {
    TimeSpan {
        Duration: (seconds.max(0.) * 10_000_000.).min(i64::MAX as f64) as i64,
    }
}

impl WindowsSession {
    pub fn new(metadata: Metadata<'_>, sender: Sender<Command>) -> anyhow::Result<Self> {
        register_identity()?;
        let player = MediaPlayer::new()?;
        // Automatic transport commands would act on this source-less player.
        // All real playback remains in rodio and is controlled by our command queue.
        player.CommandManager()?.SetIsEnabled(false)?;
        let controls = player.SystemMediaTransportControls()?;
        let mut session = Self {
            player,
            controls,
            button_token: None,
            position_token: None,
        };
        session.controls.SetIsPlayEnabled(true)?;
        session.controls.SetIsPauseEnabled(true)?;
        session.controls.SetIsStopEnabled(true)?;
        session.controls.SetIsPreviousEnabled(true)?;
        session.controls.SetIsNextEnabled(false)?;
        session.controls.SetIsRewindEnabled(true)?;
        session.controls.SetIsFastForwardEnabled(true)?;

        let position_sender = sender.clone();
        let buttons = TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let command = match args.Button()? {
                    SystemMediaTransportControlsButton::Play => Some(Command::Play),
                    SystemMediaTransportControlsButton::Pause => Some(Command::Pause),
                    SystemMediaTransportControlsButton::Stop => Some(Command::Stop),
                    SystemMediaTransportControlsButton::Previous => Some(Command::Restart),
                    SystemMediaTransportControlsButton::Rewind => Some(Command::SeekBy(-5.)),
                    SystemMediaTransportControlsButton::FastForward => Some(Command::SeekBy(5.)),
                    _ => None,
                };
                if let Some(command) = command {
                    let _ = sender.try_send(command);
                }
            }
            Ok(())
        });
        session.button_token = Some(session.controls.ButtonPressed(&buttons)?);
        session.set_position_handler(position_sender)?;
        session.set_metadata(metadata)?;
        session.controls.SetIsEnabled(true)?;
        Ok(session)
    }

    pub fn set_position_handler(&mut self, sender: Sender<Command>) -> anyhow::Result<()> {
        let positions = TypedEventHandler::<
            SystemMediaTransportControls,
            PlaybackPositionChangeRequestedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let seconds =
                    args.RequestedPlaybackPosition()?.Duration.max(0) as f64 / 10_000_000.;
                let _ = sender.try_send(Command::SeekTo(seconds));
            }
            Ok(())
        });
        self.position_token = Some(self.controls.PlaybackPositionChangeRequested(&positions)?);
        Ok(())
    }

    pub(super) fn set_metadata(&self, metadata: Metadata<'_>) -> anyhow::Result<()> {
        let updater = self.controls.DisplayUpdater()?;
        updater.SetAppMediaId(&HSTRING::from(APP_ID))?;
        updater.SetType(MediaPlaybackType::Music)?;
        let music = updater.MusicProperties()?;
        music.SetTitle(&HSTRING::from(metadata.title))?;
        music.SetArtist(&HSTRING::from(metadata.artist))?;
        music.SetAlbumTitle(&HSTRING::from(metadata.album))?;
        let cover = imageops::thumbnail(metadata.cover, 256, 256);
        if cover.width() > 0 && cover.height() > 0 {
            let mut png = Cursor::new(Vec::new());
            cover.write_to(&mut png, ImageFormat::Png)?;
            let writer = DataWriter::new()?;
            writer.WriteBytes(png.get_ref())?;
            let stream = InMemoryRandomAccessStream::new()?;
            stream.WriteAsync(&writer.DetachBuffer()?)?.join()?;
            stream.Seek(0)?;
            updater.SetThumbnail(&RandomAccessStreamReference::CreateFromStream(&stream)?)?;
        }
        updater.Update()?;
        Ok(())
    }

    pub fn publish(&self, playback: Playback) -> anyhow::Result<()> {
        self.controls.SetPlaybackStatus(match playback.status {
            Status::Playing => MediaPlaybackStatus::Playing,
            Status::Paused => MediaPlaybackStatus::Paused,
            Status::Stopped => MediaPlaybackStatus::Stopped,
        })?;
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(timespan(0.))?;
        timeline.SetMinSeekTime(timespan(0.))?;
        timeline.SetEndTime(timespan(playback.duration))?;
        timeline.SetMaxSeekTime(timespan(playback.duration))?;
        timeline.SetPosition(timespan(playback.seconds.min(playback.duration)))?;
        self.controls.UpdateTimelineProperties(&timeline)?;
        Ok(())
    }
}

impl Drop for WindowsSession {
    fn drop(&mut self) {
        if let Some(token) = self.button_token.take() {
            let _ = self.controls.RemoveButtonPressed(token);
        }
        if let Some(token) = self.position_token.take() {
            let _ = self.controls.RemovePlaybackPositionChangeRequested(token);
        }
        let _ = self
            .controls
            .SetPlaybackStatus(MediaPlaybackStatus::Stopped);
        let _ = self.controls.SetIsEnabled(false);
        let _ = self.player.Close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::{thread, time::Duration};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    };

    fn find(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
        title: &str,
    ) -> Option<GlobalSystemMediaTransportControlsSession> {
        manager.GetSessions().ok()?.into_iter().find(|session| {
            session
                .TryGetMediaPropertiesAsync()
                .and_then(|op| op.join())
                .and_then(|metadata| metadata.Title())
                .is_ok_and(|value| value == title)
        })
    }

    #[test]
    #[ignore = "Registers a temporary Windows SMTC session; run explicitly on a desktop"]
    fn native_session_publishes_artwork_receives_commands_and_detaches() -> anyhow::Result<()> {
        let title = format!("WMC SMTC probe {}", std::process::id());
        let cover = RgbaImage::from_pixel(16, 16, Rgba([110, 35, 230, 255]));
        let (sender, receiver) = crate::media_session::channel();
        let owner = WindowsSession::new(
            Metadata {
                title: &title,
                artist: "WMC probe artist",
                album: "WMC probe album",
                cover: &cover,
            },
            sender,
        )?;
        owner.publish(Playback {
            status: Status::Playing,
            seconds: 7.,
            duration: 42.,
        })?;
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
        let mut published = None;
        for _ in 0..40 {
            published = find(&manager, &title);
            if published.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let published = published.expect("SMTC session was not visible to Windows");
        let key = windows_registry::CURRENT_USER
            .open(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))?;
        assert_eq!(key.get_string("DisplayName")?, APP_NAME);
        let metadata = published.TryGetMediaPropertiesAsync()?.join()?;
        assert_eq!(metadata.Title()?, title);
        assert_eq!(metadata.Artist()?, "WMC probe artist");
        assert_eq!(metadata.AlbumTitle()?, "WMC probe album");
        assert!(metadata.Thumbnail()?.OpenReadAsync()?.join()?.Size()? > 0);
        assert!(!published.GetPlaybackInfo()?.Controls()?.IsNextEnabled()?);
        assert!(published.TryPauseAsync()?.join()?);
        let mut command = None;
        for _ in 0..40 {
            command = receiver.try_recv().ok();
            if command.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(command, Some(Command::Pause));
        owner.publish(Playback {
            status: Status::Paused,
            seconds: 7.,
            duration: 42.,
        })?;
        assert!(
            published
                .TryChangePlaybackPositionAsync(120_000_000)?
                .join()?
        );
        let mut command = None;
        for _ in 0..40 {
            command = receiver.try_recv().ok();
            if command.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(command, Some(Command::SeekTo(12.)));
        drop(owner);
        for _ in 0..40 {
            if find(&manager, &title).is_none() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }
        anyhow::bail!("SMTC session remained after detach")
    }

    #[test]
    #[ignore = "Controls only an explicitly named external player-demo probe"]
    fn hidden_player_receives_system_commands_without_render_frames() -> anyhow::Result<()> {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as State;
        let title = std::env::var("WMC_SMTC_PROBE_TITLE")?;
        anyhow::ensure!(
            title.starts_with("WMC SMTC hidden probe "),
            "only a dedicated probe may be controlled"
        );
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
        let mut session = None;
        for _ in 0..100 {
            session = find(&manager, &title);
            if session.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let session = session.expect("player did not register its media session");
        assert_eq!(session.SourceAppUserModelId()?, APP_ID);
        let key = windows_registry::CURRENT_USER
            .open(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))?;
        assert_eq!(key.get_string("DisplayName")?, APP_NAME);
        let wait_state = |expected| -> anyhow::Result<()> {
            for _ in 0..60 {
                if session.GetPlaybackInfo()?.PlaybackStatus()? == expected {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(50));
            }
            anyhow::bail!("player did not acknowledge requested status")
        };
        wait_state(State::Playing)?;
        assert!(session.TryPauseAsync()?.join()?);
        wait_state(State::Paused)?;
        assert!(
            session
                .TryChangePlaybackPositionAsync(200_000_000)?
                .join()?
        );
        let mut sought = false;
        for _ in 0..60 {
            if session.GetTimelineProperties()?.Position()?.Duration == 200_000_000 {
                sought = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(sought, "paused player did not seek to 20s");
        assert!(session.TryPlayAsync()?.join()?);
        wait_state(State::Playing)?;
        let mut advancing = false;
        for _ in 0..60 {
            if session.GetTimelineProperties()?.Position()?.Duration > 200_000_000 {
                advancing = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(advancing, "hidden player's timeline did not advance");
        assert!(session.TryStopAsync()?.join()?);
        wait_state(State::Stopped)?;
        assert!(session.TryPlayAsync()?.join()?);
        wait_state(State::Playing)?;
        assert!(
            session.GetTimelineProperties()?.Position()?.Duration < 30_000_000,
            "stopped player did not restart from zero"
        );
        Ok(())
    }
}
