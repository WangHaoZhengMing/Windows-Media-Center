//! One playback sample and one geometry/motion update per rendered frame.
use super::{PlayerView, layout::*, lyrics::layout::Layouts};
use crate::{diagnostics as perf, specs};
use gpui_kit::*;
use std::{sync::Arc, time::Instant};
use wmc_motion::player_entry::PlayerEntry;

pub(super) struct FrameState {
    pub scale: f32,
    pub logical_height: f32,
    pub columns: Columns,
    pub chrome_scale: f32,
    pub dt: f32,
    pub layouts: Arc<Layouts>,
    pub seconds: f64,
    pub duration: f64,
    pub playing: bool,
    pub word_seconds: f64,
    pub selected: Vec<usize>,
    pub active: Option<usize>,
    pub heights: Vec<f32>,
    pub anchor: f32,
    pub reduced: bool,
    pub arrival: PlayerEntry,
    pub interlude: Option<crate::lyrics::interlude::Dots>,
}

impl PlayerView {
    pub(super) fn frame(&mut self, window: &Window, cx: &Context<Self>) -> FrameState {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width).max(1.);
        let height = f32::from(viewport.height).max(1.);
        let scale = ui_scale(width, height);
        let chrome_scale = (width / 1024.).min(height / 720.);
        let columns = Columns::new(width, height, scale);
        let logical_height = columns.lyric_height;
        let available_lyric_width = columns.lyric_width;
        self.translation.prepare_layout(available_lyric_width, scale, window);
        let layouts = self
            .layouts
            .get(&self.lyrics, scale, available_lyric_width, self.settings.disable_syllables, window);
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let playback = self.audio.snapshot();
        let seconds = playback.seconds;
        perf::audio(self.audio.raw_position(), seconds);
        // Scrolling may lead by 100ms, while words still follow the audio clock.
        let mut active = self
            .lyrics
            .active_line(seconds + specs::ANIMATION_HEADSTART);
        let selected = self.lyrics.selected_lines(
            seconds,
            specs::MAX_SELECTED_LINES,
            specs::MAX_END_TIME_OFFSET,
        );
        let word_seconds = self
            .lyric_freeze
            .filter(|(_, started)| started.elapsed().as_secs_f64() < specs::TAP_PROGRESS_FREEZE)
            .map_or(seconds, |(seconds, _)| seconds);
        let duration = playback.duration;
        let playing = playback.playing;
        let reduced = cx.reduce_motion();
        let gap = layouts
            .interludes
            .iter()
            .find(|gap| seconds >= gap.start && seconds < gap.end);
        let interlude = gap.and_then(|gap| gap.sample(seconds, reduced));
        let interlude_space = gap.map_or(0., |gap| {
            let t = ((gap.end - seconds) / 0.65).clamp(0., 1.) as f32;
            t * t * (3. - 2. * t)
        });
        if let Some(gap) = gap {
            active = Some(gap.next_line);
        }
        let elapsed = self.epoch.elapsed().as_secs_f32();
        let arrival = wmc_motion::player_entry::PlayerEntry::sample(elapsed, reduced);
        let heights: Vec<_> = layouts
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                line.height + self.translation.height(index)
                    + if self.lyrics.lines[index].background {
                        15.
                    } else {
                        LYRIC_ROW_GAP
                    }
            })
            .collect();
        let mut offsets = vec![0.; heights.len()];
        let mut offset = 0.;
        for &index in &layouts.order {
            offsets[index] = offset;
            offset += heights[index];
        }
        let active_offset = active.map_or(0., |index| offsets[index]);
        // Center the whole active stanza at the reading anchor, including wraps.
        let stanza_anchor = lyric_anchor(logical_height)
            - active.map_or(0., |index| {
                (layouts.lines[index].height - LYRIC_LINE_HEIGHT) / 2.
            });
        let anchor =
            stanza_anchor + (lyric_anchor(logical_height) - stanza_anchor) * interlude_space;
        let targets: Vec<_> = offsets
            .iter()
            .map(|offset| lyric_target(*offset, active_offset, anchor, interlude_space))
            .collect();
        self.motion.update(&targets, active, &selected, dt, reduced);
        FrameState {
            scale,
            logical_height,
            columns,
            chrome_scale,
            dt,
            layouts,
            seconds,
            duration,
            playing,
            word_seconds,
            selected,
            active,
            heights,
            anchor,
            reduced,
            arrival,
            interlude,
        }
    }
}
