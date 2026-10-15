//! Allocation-free PCM tap. Cutoffs are authored here, not recovered from EXE.
//! Native four-frame weighting, peak decay and smootherstep follow the tap.
use rodio::{Source, source::SeekError};
use std::{sync::{Arc, atomic::{AtomicU32, Ordering}}, time::Duration};

#[derive(Default)]
pub(super) struct Spectrum([AtomicU32; 3]);
impl Spectrum {
    pub fn read(&self) -> [f32; 3] {
        self.0.each_ref().map(|v| f32::from_bits(v.load(Ordering::Relaxed)))
    }
    fn write(&self, values: [f32; 3]) {
        for (slot, value) in self.0.iter().zip(values) { slot.store(value.to_bits(), Ordering::Relaxed); }
    }
}
pub(super) struct Tap<S> {
    source: S, output: Arc<Spectrum>, low: f32, upper: f32, mono: f32,
    channel: u16, samples: u32, energy: [f32; 3],
    history: [[f32; 3]; 4], peak: [f32; 3], smooth: [f32; 3],
}
impl<S: Source> Tap<S> {
    pub fn new(source: S, output: Arc<Spectrum>) -> Self {
        output.write([0.; 3]);
        Self { source, output, low:0., upper:0., mono:0., channel:0, samples:0,
            energy:[0.;3], history:[[0.;3];4], peak:[0.;3], smooth:[0.;3] }
    }
    fn sample(&mut self, value: f32) {
        self.mono += value; self.channel += 1;
        let channels = self.source.channels().max(1);
        if self.channel < channels { return; }
        let value = self.mono / f32::from(channels);
        self.channel = 0; self.mono = 0.;
        let rate = self.source.sample_rate().max(1) as f32;
        let coefficient = |hz: f32| (std::f32::consts::TAU * hz / rate).min(1.);
        self.low += coefficient(200.) * (value - self.low);
        self.upper += coefficient(2000.) * (value - self.upper);
        let bands = [self.low, self.upper - self.low, value - self.upper];
        for (sum, value) in self.energy.iter_mut().zip(bands) { *sum += value * value; }
        self.samples += 1;
        if self.samples < (rate / 50.) as u32 { return; }
        self.history.rotate_left(1);
        self.history[3] = self.energy.map(|sum| ((sum / self.samples as f32).sqrt() * 5.).clamp(0., 1.));
        for (i, decay) in [0.98, 0.99, 0.999].into_iter().enumerate() {
            let weighted: f32 = self.history.iter().zip([0.1, 0.2, 0.3, 0.4]).map(|(f,w)| f[i]*w).sum();
            self.peak[i] = weighted.max(self.peak[i] * decay);
            self.smooth[i] += 0.25 * (self.peak[i] - self.smooth[i]);
        }
        self.output.write(self.smooth.map(|x| { let x=x.clamp(0.,1.); x*x*x*(x*(6.*x-15.)+10.) }));
        self.samples = 0; self.energy = [0.; 3];
    }
}
impl<S: Source> Iterator for Tap<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> { let v=self.source.next()?; self.sample(v); Some(v) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.source.size_hint() }
}
impl<S: Source> Source for Tap<S> {
    fn current_span_len(&self) -> Option<usize> { self.source.current_span_len() }
    fn channels(&self) -> u16 { self.source.channels() }
    fn sample_rate(&self) -> u32 { self.source.sample_rate() }
    fn total_duration(&self) -> Option<Duration> { self.source.total_duration() }
    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.source.try_seek(position)?;
        self.low=0.; self.upper=0.; self.mono=0.; self.channel=0; self.samples=0;
        self.energy=[0.;3]; self.history=[[0.;3];4]; self.peak=[0.;3]; self.smooth=[0.;3];
        self.output.write([0.;3]); Ok(())
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn pcm_is_unchanged_and_silence_cannot_generate_a_pulse() {
        let samples:Vec<_>=(0..16000).map(|i| (std::f32::consts::TAU*80.*i as f32/16000.).sin()*0.2).collect();
        let output=Arc::new(Spectrum::default());
        let tapped:Vec<_>=Tap::new(rodio::buffer::SamplesBuffer::new(1,16000,samples.clone()),output.clone()).collect();
        assert_eq!(tapped,samples);
        let b=output.read(); assert!(b[0]>b[1] && b[0]>0.05);
        let _=Tap::new(rodio::buffer::SamplesBuffer::new(1,16000,vec![0.;16000]),output.clone()).count();
        assert_eq!(output.read(),[0.;3]);
    }
}
