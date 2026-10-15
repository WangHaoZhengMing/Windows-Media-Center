//! Compare with memory-probe: no window/audio allocation, same staged sampling.
#[path = "../src/view/lyrics/font.rs"]
mod glyph_cache;

use std::{
    io::{self, Write},
    time::Instant,
};

fn stage(name: &str) {
    println!("stage={name} pid={}", std::process::id());
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut String::new()).unwrap();
}

fn main() {
    let text = std::fs::read_to_string(std::env::args_os().nth(1).expect("lyrics path")).unwrap();
    stage("baseline");
    let start = Instant::now();
    let font = glyph_cache::SongFont::load(&text).expect("system font");
    println!("parse_ms={:.2}", start.elapsed().as_secs_f64() * 1000.);
    stage("font_parsed");
    let start = Instant::now();
    let mut bytes = 0;
    for c in text.chars().collect::<std::collections::HashSet<_>>() {
        bytes += font.rasterize(c, 44.).unwrap().bitmap.len();
    }
    println!(
        "mask_bytes={bytes} raster_ms={:.2}",
        start.elapsed().as_secs_f64() * 1000.
    );
    stage("song_glyphs");
    drop(font);
    stage("dropped");
}
