/// Media controls are independent from homepage navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Stop,
    Previous,
    Next,
    Rewind,
    FastForward,
    ToggleMute,
    VolumeDown,
    VolumeUp,
}

/// An empty player has no transport capabilities until a backend supplies media.
/// Volume and mute are persistent player preferences, not system volume changes.
#[derive(Debug)]
pub struct TransportState {
    volume: u8,
    muted: bool,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            volume: 70,
            muted: false,
        }
    }
}

impl TransportState {
    pub fn volume(&self) -> u8 {
        self.volume
    }
    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn available(&self, command: MediaCommand) -> bool {
        match command {
            MediaCommand::ToggleMute => true,
            MediaCommand::VolumeDown => self.volume > 0,
            MediaCommand::VolumeUp => self.volume < 100,
            _ => false,
        }
    }

    /// No synthetic playback state is displayed while the player has no media.
    pub fn dispatch(&mut self, command: MediaCommand) {
        match command {
            MediaCommand::ToggleMute => self.muted = !self.muted,
            MediaCommand::VolumeDown => self.volume = self.volume.saturating_sub(5),
            MediaCommand::VolumeUp => self.volume = self.volume.saturating_add(5).min(100),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_player_disables_transport_but_keeps_audio_preferences() {
        let mut player = TransportState::default();
        assert!(!player.available(MediaCommand::PlayPause));
        assert!(!player.available(MediaCommand::Next));
        player.dispatch(MediaCommand::PlayPause);
        player.dispatch(MediaCommand::ToggleMute);
        assert!(player.muted());
        assert_eq!(player.volume(), 70);
    }

    #[test]
    fn volume_is_bounded_and_mute_preserves_it() {
        let mut player = TransportState::default();
        for _ in 0..30 {
            player.dispatch(MediaCommand::VolumeUp);
        }
        assert_eq!(player.volume(), 100);
        assert!(!player.available(MediaCommand::VolumeUp));
        player.dispatch(MediaCommand::ToggleMute);
        assert_eq!(player.volume(), 100);
        for _ in 0..30 {
            player.dispatch(MediaCommand::VolumeDown);
        }
        assert_eq!(player.volume(), 0);
        assert!(!player.available(MediaCommand::VolumeDown));
    }
}
