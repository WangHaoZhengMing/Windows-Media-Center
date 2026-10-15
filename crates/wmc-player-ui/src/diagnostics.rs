//! Opt-in frame diagnostics. Sampling never writes files on the UI thread.
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::{Duration, Instant},
};

static BUILD: AtomicU64 = AtomicU64::new(0);
static PAINT: AtomicU64 = AtomicU64::new(0);
static BLUR: AtomicU64 = AtomicU64::new(0);
static SHAPES: AtomicU64 = AtomicU64::new(0);
static RAW_AUDIO: AtomicU64 = AtomicU64::new(0);
static DISPLAY_AUDIO: AtomicU64 = AtomicU64::new(0);

fn recorder() -> Option<&'static SyncSender<[f64; 7]>> {
    static RECORDER: OnceLock<Option<SyncSender<[f64; 7]>>> = OnceLock::new();
    RECORDER
        .get_or_init(|| {
            let path = std::env::var_os("WMC_PLAYER_PROFILE")?;
            let (sender, receiver) = sync_channel::<[f64; 7]>(1024);
            std::thread::spawn(move || {
                let Ok(file) = File::create(path) else { return };
                let mut writer = BufWriter::new(file);
                let _ = writeln!(
                    writer,
                    "interval_ms,build_ms,lyrics_paint_ms,blur_ms,shape_calls,raw_audio_ms,display_audio_ms"
                );
                for sample in receiver {
                    let _ = writeln!(
                        writer,
                        "{:.4},{:.4},{:.4},{:.4},{:.0},{:.4},{:.4}",
                        sample[0], sample[1], sample[2], sample[3], sample[4], sample[5], sample[6]
                    );
                    let _ = writer.flush();
                }
            });
            Some(sender)
        })
        .as_ref()
}

pub(crate) fn enabled() -> bool {
    recorder().is_some()
}

pub(crate) fn shape() {
    if enabled() {
        SHAPES.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn audio(raw: f64, displayed: f64) {
    if enabled() {
        RAW_AUDIO.store((raw * 1000.).to_bits(), Ordering::Relaxed);
        DISPLAY_AUDIO.store((displayed * 1000.).to_bits(), Ordering::Relaxed);
    }
}

pub(crate) struct Timer {
    started: Option<Instant>,
    total: &'static AtomicU64,
}

impl Timer {
    fn new(total: &'static AtomicU64) -> Self {
        Self {
            started: enabled().then(Instant::now),
            total,
        }
    }
    pub fn paint() -> Self {
        Self::new(&PAINT)
    }
    pub fn blur() -> Self {
        Self::new(&BLUR)
    }
    pub fn frame(interval: Duration) -> Self {
        if let Some(sender) = recorder() {
            let ms = |counter: &AtomicU64| counter.swap(0, Ordering::Relaxed) as f64 / 1_000_000.;
            let _ = sender.try_send([
                interval.as_secs_f64() * 1000.,
                ms(&BUILD),
                ms(&PAINT),
                ms(&BLUR),
                SHAPES.swap(0, Ordering::Relaxed) as f64,
                f64::from_bits(RAW_AUDIO.load(Ordering::Relaxed)),
                f64::from_bits(DISPLAY_AUDIO.load(Ordering::Relaxed)),
            ]);
        }
        Self::new(&BUILD)
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            self.total
                .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
    }
}
