//! Playback controls and metadata shared with the containing application.
use super::PlayerView;
use gpui_kit::*;

impl PlayerView {
    pub fn release(&mut self, window: &mut Window) {
        // A frame may still retain this entity after the shell stops showing it.
        // Invalidate callbacks before they can update the shared current track.
        self.lyrics_search.generation += 1;
        self.lyrics_search.task = None;
        self.cover_search.generation += 1;
        self.cover_search.task = None;
        self._asset_task = None;
        self.translation.stop();
        self.lyric_images.release(window);
        for mut cache in self.retired_lyrics.drain(..) {
            cache.release(window);
        }
        self.background.release(window);
        let _ = window.drop_image(self.cover.clone());
        for (_, image) in self.cover_search.candidates.drain(..) {
            if let Some(image) = image {
                let _ = window.drop_image(image);
            }
        }
    }

    pub fn leave_page(&mut self) {
        self.dragging = false;
    }
}
