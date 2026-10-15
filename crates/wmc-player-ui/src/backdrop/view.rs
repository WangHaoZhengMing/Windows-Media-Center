//! Continuous artwork rendering with one CPU job and two GPU frames at most.
use super::{Backdrop, Parameters};
use crate::view::{PlayerView, frame::FrameState};
use gpui_kit::{prelude::*, *};
use std::{sync::Arc, time::Instant};
use wmc_design::images as resources;

pub(crate) struct BackgroundState {
    backdrop: Backdrop,
    old_art: Option<(Backdrop, Instant)>,
    images: Vec<Arc<RenderImage>>,
    retired: Vec<Arc<RenderImage>>,
    task: Option<Task<()>>,
    pending: bool,
    generation: u64,
    phase: f32,
    blur: f32,
    spectrum: [f32; 3],
    started: Instant,
    ready: Instant,
    viewport: (u32, u32),
}
impl BackgroundState {
    pub fn new(backdrop: Backdrop, image: Arc<RenderImage>) -> Self {
        Self {
            backdrop,
            old_art: None,
            images: vec![image],
            retired: vec![],
            task: None,
            pending: false,
            generation: 0,
            phase: 0.,
            blur: 80.,
            spectrum: [0.; 3],
            started: Instant::now(),
            ready: Instant::now(),
            viewport: (0, 0),
        }
    }
    pub fn replace_backdrop(&mut self, backdrop: Backdrop) {
        self.old_art = Some((self.backdrop.clone(), Instant::now()));
        self.backdrop = backdrop;
        self.generation += 1;
        self.task = None;
        self.pending = false;
        self.viewport = (0, 0);
    }
    pub fn release(&mut self, window: &mut Window) {
        self.generation += 1;
        self.task = None;
        for image in self.images.drain(..).chain(self.retired.drain(..)) {
            let _ = window.drop_image(image);
        }
    }
    pub fn retire(&mut self, image: Arc<RenderImage>) {
        self.retired.push(image);
    }
}

impl PlayerView {
    pub(crate) fn update_background(
        &mut self,
        frame: &FrameState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let b = &mut self.background;
        for image in b.retired.drain(..) {
            let _ = window.drop_image(image);
        }
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        let size = (width as u32, height as u32);
        let moving = self.animate_background && !frame.reduced;
        let dt = frame.dt.min(0.1);
        if moving {
            b.phase += dt * 1000. * if frame.columns.wide { 0.13 } else { 0.13 * 0.7 };
        }
        let target = if frame.playing && moving {
            self.app.spectrum()
        } else {
            [0.; 3]
        };
        let alpha = 1. - (-dt * 12.).exp();
        for (value, target) in b.spectrum.iter_mut().zip(target) {
            *value += (target - *value) * alpha;
        }
        let has_lyrics = !self.lyrics.lines.is_empty() || self.lyrics.plain_text.is_some();
        let blur_target = if !has_lyrics {
            80.
        } else if frame.columns.wide {
            60.
        } else {
            47.5
        };
        b.blur += (blur_target - b.blur).clamp(-dt * 30., dt * 30.);
        let resize = b.viewport != size;
        if b.pending || (!resize && b.started.elapsed().as_secs_f32() < 1. / 24.) {
            return;
        }
        if !moving
            && !resize
            && b.old_art.is_none()
            && (b.blur - blur_target).abs() < 0.01
            && b.spectrum.iter().all(|x| *x < 0.001)
        {
            return;
        }
        b.started = Instant::now();
        b.pending = true;
        b.viewport = size;
        let backdrop = b.backdrop.clone();
        let previous = b.old_art.as_ref().map(|(art, start)| {
            (
                art.clone(),
                1. - (-start.elapsed().as_secs_f32() / 0.3).exp(),
            )
        });
        if previous.as_ref().is_some_and(|(_, mix)| *mix > 0.995) {
            b.old_art = None;
        }
        let generation = b.generation;
        let parameters = Parameters {
            time: b.phase / 60.,
            width,
            height,
            spectrum: b.spectrum,
            has_lyrics,
            blur: b.blur,
        };
        let (w, h) = Backdrop::surface_size(width, height, frame.columns.wide);
        b.task = Some(cx.spawn(async move |this, cx| {
            let image = cx
                .background_executor()
                .spawn(async move {
                    let image = backdrop.render_transition(
                        parameters,
                        w,
                        h,
                        previous.as_ref().map(|(old, mix)| (old, *mix)),
                    );
                    resources::background_image(image)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let b = &mut this.background;
                if b.generation != generation {
                    return;
                }
                b.pending = false;
                if b.images.len() == 2 {
                    b.retired.push(b.images.remove(0));
                }
                b.images.push(image);
                b.ready = Instant::now();
                cx.notify();
            });
        }));
    }
    pub(crate) fn background_layer(&self, frame: &FrameState) -> Div {
        let arrival = frame.arrival;
        let b = &self.background;
        let mix = (b.ready.elapsed().as_secs_f32() * 24.).clamp(0., 1.);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                background_tile(b.images[0].clone())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .opacity(arrival.background_alpha),
            )
            .child(
                background_tile(b.images.last().unwrap().clone())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .opacity(mix * arrival.background_alpha),
            )
    }
}

fn background_tile(image: Arc<RenderImage>) -> Div {
    resources::guarded_image_element(image)
}
