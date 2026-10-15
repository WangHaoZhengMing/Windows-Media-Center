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
pub(super) const MAX_CACHED_LINES: usize = 5;
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
    wanted: Vec<usize>,
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
            wanted: Vec::with_capacity(MAX_CACHED_LINES),
        }
    }
    pub fn set_font(&mut self, font: Option<Arc<SongFont>>) {
        self.font = font;
        self.scale = 0.;
    }
    pub fn reset(&mut self, count: usize) {
        for entry in &mut self.lines {
            if let Some(line) = entry.ready.take() {
                self.retired.push(line);
            }
        }
        self.lines = (0..count).map(|_| Entry::default()).collect();
        self.generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        self.scale = 0.;
        self.pending_layout = None;
        self.wanted.clear();
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
        self.lines.get(index).and_then(|entry| entry.ready.clone())
    }
    pub fn release(&mut self, window: &mut Window) {
        self.generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        self.wanted.clear();
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
            || !self.wanted.contains(&index)
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
        if let Some(entry) = self.lines.get_mut(index) {
            entry.pending = false;
        }
        if !self.wanted.contains(&index) {
            return;
        }
        let entry = &mut self.lines[index];
        if let Some(old) = entry.ready.replace(Arc::new(line)) {
            self.retired.push(old);
        }
        entry.pending = false;
    }
    pub fn select_lines(&mut self, indices: &[usize], window: &mut Window) {
        self.retain_lines(indices, |line| line.drop_images(window));
    }

    fn retain_lines(&mut self, indices: &[usize], mut release: impl FnMut(Arc<RasterizedLine>)) {
        self.wanted.clear();
        for &index in indices {
            if self.wanted.len() == MAX_CACHED_LINES {
                break;
            }
            if index < self.lines.len() && !self.wanted.contains(&index) {
                self.wanted.push(index);
            }
        }
        for (index, entry) in self.lines.iter_mut().enumerate() {
            if !self.wanted.contains(&index)
                && let Some(line) = entry.ready.take()
            {
                release(line);
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
    fn a_whole_song_never_retains_more_than_five_lines_and_eviction_releases_them() {
        let mut cache = LineImageCache::new(None, 300);
        let mut weak = Vec::new();
        let mut released = 0;
        for current in 0usize..300 {
            let indices: Vec<_> = (current.saturating_sub(2)..(current + 3).min(300)).collect();
            cache.retain_lines(&indices, |_| released += 1);
            for index in indices {
                if cache.get(index).is_none() {
                    cache.accept(cache.generation, index, empty_line());
                    weak.push(Arc::downgrade(&cache.get(index).unwrap()));
                }
            }
            assert!(weak.iter().filter(|line| line.strong_count() > 0).count() <= MAX_CACHED_LINES);
        }
        cache.retain_lines(&[], |_| released += 1);
        assert_eq!(released, weak.len());
        assert!(weak.iter().all(|line| line.upgrade().is_none()));
    }

    #[test]
    fn late_background_results_cannot_restore_evicted_lines_or_exceed_the_limit() {
        let mut cache = LineImageCache::new(None, 20);
        cache.retain_lines(&[0], |_| {});
        cache.lines[0].pending = true;
        cache.in_flight = 1;
        cache.retain_lines(&(1..20).collect::<Vec<_>>(), |_| {});
        assert_eq!(cache.wanted.len(), MAX_CACHED_LINES);
        cache.accept(cache.generation, 0, empty_line());
        assert!(cache.get(0).is_none());
        assert!(!cache.lines[0].pending);
        assert_eq!(cache.in_flight, 0);
    }

    #[test]
    fn replacing_lyrics_rejects_old_results_even_when_the_new_song_has_fewer_lines() {
        let mut cache = LineImageCache::new(None, 3);
        let generation = cache.generation;
        cache.in_flight = 1;
        cache.reset(1);
        cache.accept(generation, 2, empty_line());
        assert_eq!(cache.in_flight, 0);
        assert!(cache.get(0).is_none());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn a_resize_discards_stale_background_results_and_releases_the_queue_slot() {
        let mut cache = LineImageCache::new(None, 1);
        cache.retain_lines(&[0], |_| {});
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
