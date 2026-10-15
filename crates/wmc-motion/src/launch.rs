//! Original launch timing; the UI supplies original image layers.
#[path = "launch_tracks.rs"]
mod tracks;

pub const LAUNCH_DURATION: f32 = 3.1;

pub(super) struct Keyframe {
    time: f32,
    value: [f32; 2],
    curve: u8,
}

pub struct LaunchTrack {
    pub name: &'static str,
    alpha: &'static [Keyframe],
    scale: &'static [Keyframe],
    position: &'static [Keyframe],
}

#[derive(Clone, Copy, Debug)]
pub struct LaunchPose {
    pub opacity: f32,
    pub scale: [f32; 2],
    pub position: [f32; 2],
}

fn sample(frames: &[Keyframe], time: f32, fallback: [f32; 2]) -> [f32; 2] {
    let Some(first) = frames.first() else {
        return fallback;
    };
    if time <= first.time {
        return first.value;
    }
    for pair in frames.windows(2) {
        let [a, b] = pair else { unreachable!() };
        if time <= b.time {
            let t = ((time - a.time) / (b.time - a.time)).clamp(0., 1.);
            // MCML's curve parameters are proprietary. Preserve its timing and
            // approximate SCurve / EaseIn with smoothstep / quadratic easing.
            let t = match a.curve {
                1 => t * t * (3. - 2. * t),
                2 => t * t,
                _ => t,
            };
            return std::array::from_fn(|i| a.value[i] + (b.value[i] - a.value[i]) * t);
        }
    }
    frames.last().map_or(fallback, |frame| frame.value)
}

pub fn launch_pose(name: &str, time: f32) -> LaunchPose {
    let track = tracks::TRACKS
        .iter()
        .find(|track| track.name == name)
        .expect("unknown recovered launch track");
    LaunchPose {
        opacity: sample(track.alpha, time, [1.; 2])[0].clamp(0., 1.),
        scale: sample(track.scale, time, [1.; 2]),
        position: sample(track.position, time, [0.; 2]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_logo_and_text_land_at_one_second_and_disappear_at_2700ms() {
        let base = launch_pose("Launch.Skittle.Base", 1.);
        assert_eq!(base.scale, [1.; 2]);
        assert_eq!(base.opacity, 1.);
        assert_eq!(launch_pose("Launch.Text", 1.).position, [0., 200.]);
        for name in ["Launch.Text", "Launch.Skittle.Base", "Launch.Skittle.Top"] {
            assert_eq!(launch_pose(name, 2.7).opacity, 0.);
        }
    }

    #[test]
    fn recovered_effect_tracks_produce_finite_samples() {
        for track in tracks::TRACKS {
            let pose = launch_pose(track.name, LAUNCH_DURATION);
            assert!(pose.opacity.is_finite());
            assert!(pose.scale.iter().all(|value| value.is_finite()));
        }
    }
}
