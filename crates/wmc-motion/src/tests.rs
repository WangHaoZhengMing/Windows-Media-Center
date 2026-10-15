use crate::{CategoryMotion, profile::*};
use std::time::Duration;

fn ms(time: u64) -> Duration {
    Duration::from_millis(time)
}

#[test]
fn focus_gain_waits_while_the_old_focus_finishes() {
    let mut motion = CategoryMotion::<4>::new(0);
    motion.retarget(1, ms(0));
    let early = motion.sample(ms(100), false);
    assert_eq!(early.rows[0].highlight, 0.);
    assert_eq!(early.rows[1].highlight, 0.);
    assert!(early.rows[1].row_y > 0. && early.rows[1].row_y < EXPANDED_STEP);
    assert_eq!(motion.sample(ms(133), false).rows[1].highlight, 0.);
    assert!(motion.sample(ms(200), false).rows[1].highlight > 0.);
    let finished = motion.sample(ms(267), false);
    assert!(!finished.moving);
    assert_eq!(finished.rows[1].row_y, 0.);
    assert_eq!(finished.rows[1].title_y, TITLE_TOP_INSET);
    assert_eq!(finished.rows[1].highlight, 1.);
}

#[test]
fn reversing_mid_scroll_preserves_every_current_channel() {
    let mut motion = CategoryMotion::<4>::new(0);
    motion.retarget(1, ms(0));
    let before = motion.sample(ms(180), false);
    motion.retarget(0, ms(180));
    let after = motion.sample(ms(180), false);
    assert_eq!(before.rows, after.rows);
    assert_eq!(motion.sample(ms(447), false).rows[0].row_y, 0.);
}

#[test]
fn passed_titles_remain_ordered_above_the_anchor() {
    let mut motion = CategoryMotion::<4>::new(0);
    motion.retarget(3, ms(0));
    let rows = motion.sample(ms(267), false).rows;
    assert!(rows[0].row_y < rows[1].row_y && rows[1].row_y < rows[2].row_y);
    assert!(rows[2].row_y < rows[3].row_y);
    assert_eq!(rows[3].row_y, 0.);
}

#[test]
fn initialization_and_reduced_motion_do_not_fly_in_from_zero() {
    let mut motion = CategoryMotion::<4>::new(2);
    assert!(!motion.sample(ms(0), false).moving);
    assert_eq!(motion.sample(ms(0), false).rows[2].highlight, 1.);
    motion.retarget(3, ms(1));
    let sample = motion.sample(ms(1), true);
    assert!(!sample.moving);
    assert_eq!(sample.rows[3].row_y, 0.);
    assert_eq!(sample.rows[3].highlight, 1.);
}
