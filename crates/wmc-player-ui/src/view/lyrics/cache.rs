//! Song-owned image cache, bounded raster queue and layout generations.
use super::{
    font::SongFont,
    layout::LineLayout,
    raster::{RasterJob, RasterizedLine},
};
use gpui_kit::Window;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
const MAX_IN_FLIGHT: usize = 2;
static GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct Entry {
    ready: Option<Arc<RasterizedLine>>,
    pending: bool,
}

pub(in crate::view) struct LineImageCache {
    font: Option<Arc<SongFont>>,
    scale: f32,
    width: f32,
    generation: u64,
    in_flight: usize,
    lines: Vec<Entry>,
    pending_layout: Option<(f32, f32, Instant)>,
    retired: Vec<Arc<RasterizedLine>>,
}

impl LineImageCache {
    pub fn new(font: Option<Arc<SongFont>>, count: usize) -> Self {
        Self {
            font,
            scale: 0.,
            width: 0.,
            generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            in_flight: 0,
            lines: (0..count).map(|_| Entry::default()).collect(),
            pending_layout: None,
            retired: Vec::new(),
        }
    }
    pub fn set_font(&mut self, font: Option<Arc<SongFont>>) {
        self.font = font;
        self.scale = 0.;
    }
    pub fn prepare_layout(&mut self, scale: f32, width: f32, window: &mut Window) {
        for retired in self.retired.drain(..) {
            retired.drop_images(window);
        }
        if (self.scale - scale).abs() < 0.001 && (self.width - width).abs() < 0.1 {
            self.pending_layout = None;
            return;
        }
        if self.scale > 0. {
            match self.pending_layout {
                Some((pending, pending_width, time))
                    if (pending - scale).abs() < 0.001 && (pending_width - width).abs() < 0.1 =>
                {
                    if time.elapsed() < Duration::from_millis(160) {
                        return;
                    }
                }
                _ => {
                    self.pending_layout = Some((scale, width, Instant::now()));
                    return;
                }
            }
        }
        self.generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        for entry in &mut self.lines {
            if let Some(line) = entry.ready.take() {
                line.drop_images(window);
            }
            entry.pending = false;
        }
        self.scale = scale;
        self.width = width;
        self.pending_layout = None;
    }
    pub fn get(&self, index: usize) -> Option<Arc<RasterizedLine>> {
        self.lines[index].ready.clone()
    }
    pub fn release(&mut self, window: &mut Window) {
        for entry in &mut self.lines {
            if let Some(line) = entry.ready.take() {
                line.drop_images(window);
            }
        }
        for line in self.retired.drain(..) {
            line.drop_images(window);
        }
    }
    pub fn request(
        &mut self,
        index: usize,
        layout: Arc<LineLayout>,
        scale: f32,
    ) -> Option<RasterJob> {
        if self.in_flight >= MAX_IN_FLIGHT
            || self.pending_layout.is_some()
            || (self.scale - scale).abs() >= 0.001
        {
            return None;
        }
        let font = self.font.clone()?;
        let entry = &mut self.lines[index];
        if entry.ready.is_some() || entry.pending {
            return None;
        }
        entry.pending = true;
        self.in_flight += 1;
        Some(RasterJob {
            font,
            layout,
            scale: self.scale,
            generation: self.generation,
        })
    }
    pub fn accept(&mut self, generation: u64, index: usize, line: RasterizedLine) {
        self.in_flight = self.in_flight.saturating_sub(1);
        if generation != self.generation {
            return;
        }
        let entry = &mut self.lines[index];
        if let Some(old) = entry.ready.replace(Arc::new(line)) {
            self.retired.push(old);
        }
        entry.pending = false;
    }
    /// Keep only the visible neighborhood: long songs do not retain every texture.
    pub fn trim(&mut self, first: usize, last: usize, window: &mut Window) {
        for (index, entry) in self.lines.iter_mut().enumerate() {
            if (index < first || index > last)
                && let Some(line) = entry.ready.take()
            {
                line.drop_images(window);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_line() -> RasterizedLine {
        RasterizedLine {
            layers: Vec::new(),
            words: Vec::new(),
            width: 0.,
            height: 0.,
        }
    }

    #[test]
    fn a_resize_discards_stale_background_results_and_releases_the_queue_slot() {
        let mut cache = LineImageCache::new(None, 1);
        cache.generation = 2;
        cache.in_flight = 1;
        cache.accept(1, 0, empty_line());
        assert!(cache.get(0).is_none());
        assert_eq!(cache.in_flight, 0);
        cache.lines[0].pending = true;
        cache.in_flight = 1;
        cache.accept(2, 0, empty_line());
        assert!(cache.get(0).is_some());
        assert!(!cache.lines[0].pending);
        assert_eq!(cache.in_flight, 0);
    }
}
