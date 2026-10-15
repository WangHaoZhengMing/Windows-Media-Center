//! Bounded, deterministic cover quantization; no artwork-sized pixel cache.
use image::{RgbaImage, imageops};

const COLORS: usize = 6;

#[derive(Clone, Copy, Default)]
struct Bin {
    weight: f32,
    sum: [f32; 3],
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum()
}

pub(super) fn extract(cover: &RgbaImage) -> [[f32; 3]; COLORS] {
    if cover.width() == 0 || cover.height() == 0 {
        return [[0.16; 3]; COLORS];
    }
    // Sample actual pixels: interpolating unpremultiplied RGBA would blend
    // invisible RGB from transparent borders into neighboring opaque colors.
    let small = imageops::resize(cover, 64, 64, imageops::FilterType::Nearest);
    let chromatic = small
        .pixels()
        .filter(|p| {
            let max = p.0[..3].iter().copied().max().unwrap_or(0);
            let min = p.0[..3].iter().copied().min().unwrap_or(0);
            p.0[3] > 128 && max > 24 && max - min > 24
        })
        .count()
        > 64;
    let mut bins = vec![Bin::default(); 4096];
    for pixel in small.pixels() {
        let rgb = [pixel.0[0], pixel.0[1], pixel.0[2]].map(|c| f32::from(c) / 255.);
        let max = rgb.into_iter().fold(0., f32::max);
        let min = rgb.into_iter().fold(1., f32::min);
        let saturation = (max - min) / max.max(0.001);
        // Black borders and white lettering must not average away a colored album.
        // Grayscale albums still retain a neutral palette rather than invented hues.
        let neutral = if chromatic && saturation < 0.15 {
            0.025
        } else {
            1.
        };
        let weight = f32::from(pixel.0[3]) / 255. * (0.2 + saturation) * max.max(0.01) * neutral;
        let index = ((pixel.0[0] as usize >> 4) << 8)
            | ((pixel.0[1] as usize >> 4) << 4)
            | (pixel.0[2] as usize >> 4);
        bins[index].weight += weight;
        for (sum, channel) in bins[index].sum.iter_mut().zip(rgb) {
            *sum += channel * weight;
        }
    }
    let mut samples: Vec<_> = bins
        .into_iter()
        .filter(|b| b.weight > 0.)
        .map(|b| (b.sum.map(|c| c / b.weight), b.weight))
        .collect();
    samples.sort_by(|a, b| b.1.total_cmp(&a.1));
    let Some(&(first, _)) = samples.first() else {
        return [[0.16; 3]; COLORS];
    };
    let mut colors = [first; COLORS];
    let count = samples.len().min(COLORS);
    for i in 1..count {
        colors[i] = samples
            .iter()
            .max_by(|a, b| {
                let score = |sample: &([f32; 3], f32)| {
                    colors[..i]
                        .iter()
                        .map(|c| distance(sample.0, *c))
                        .fold(f32::INFINITY, f32::min)
                        * sample.1.sqrt()
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap()
            .0;
    }
    let mut populations = [0.; COLORS];
    for _ in 0..8 {
        let mut sums = [[0.; 3]; COLORS];
        populations.fill(0.);
        for &(rgb, weight) in &samples {
            let nearest = (0..count)
                .min_by(|&a, &b| distance(rgb, colors[a]).total_cmp(&distance(rgb, colors[b])))
                .unwrap();
            populations[nearest] += weight;
            for (sum, channel) in sums[nearest].iter_mut().zip(rgb) {
                *sum += channel * weight;
            }
        }
        for i in 0..count {
            if populations[i] > 0. {
                colors[i] = sums[i].map(|c| c / populations[i]);
            }
        }
    }
    let mut order: Vec<_> = (0..count).collect();
    order.sort_by(|&a, &b| populations[b].total_cmp(&populations[a]));
    // A rare white logo must not receive a whole mesh pool just because it
    // formed its own cluster. Keep colors that carry a meaningful palette share.
    let total: f32 = populations.iter().sum();
    order.retain(|&i| populations[i] >= total * 0.045);
    std::array::from_fn(|i| colors[order[i % order.len()]])
}
