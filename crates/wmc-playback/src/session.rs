use crate::{AudioData, AudioPlayer, queue::QueueState};
/// Owns audio and queue independently from any window or view.
#[derive(Default)]
pub struct PlaybackSession {
    pub audio: Option<AudioPlayer>,
    pub queue: QueueState,
}
impl PlaybackSession {
    pub fn replace(&mut self, source: AudioData) {
        self.audio = Some(AudioPlayer::new(source));
    }
}
