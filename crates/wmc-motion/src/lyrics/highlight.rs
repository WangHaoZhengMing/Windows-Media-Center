//! A shared feather travels along cumulative text advances, including wrapped rows.
/// Word timestamps determine distance; gaps hold it and seeking is stateless.
pub fn line_cursor(
    seconds: f64,
    words: impl IntoIterator<Item = (f64, f64, f32)>,
    feather: f32,
) -> f32 {
    let mut distance = 0.;
    let mut total = 0.;
    for (start, end, width) in words {
        let progress = if seconds < start {
            0.
        } else if seconds >= end {
            1.
        } else {
            ((seconds - start) / (end - start).max(0.001)) as f32
        };
        distance += width * progress;
        total += width;
    }
    // Distribute the feather's extra travel over the whole line, rather than
    // restarting it at every word. The first/last samples remain dark/full.
    if total > 0. {
        distance * (1. + feather.max(0.001) / total)
    } else {
        0.
    }
}

pub fn coverage_at(cursor: f32, x: f32, feather: f32) -> f32 {
    ((cursor - x) / feather.max(0.001)).clamp(0., 1.)
}

/// Coverage for a single continuous span.
pub fn coverage(progress: f32, x: f32, width: f32, feather: f32) -> f32 {
    let feather = feather.max(0.001);
    coverage_at(progress.clamp(0., 1.) * (width + feather), x, feather)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feather_crosses_unequal_word_boundaries_without_restarting() {
        let words = [(1., 1.2, 44.), (1.2, 2., 88.), (2., 2.5, 44.)];
        for boundary in [1.2, 2.] {
            let before = line_cursor(boundary - 0.00001, words, 40.);
            let after = line_cursor(boundary + 0.00001, words, 40.);
            for x in 0..176 {
                assert!(
                    (coverage_at(after, x as f32, 40.) - coverage_at(before, x as f32, 40.)).abs()
                        < 0.001
                );
            }
        }
        // At the first boundary both sides of it are partially lit.
        let cursor = line_cursor(1.2, words, 40.);
        assert!(coverage_at(cursor, 43., 40.) > 0.);
        assert!(coverage_at(cursor, 43., 40.) < 1.);
        assert!(coverage_at(cursor, 45., 40.) > 0.);
        assert_eq!(line_cursor(0., words, 40.), 0.);
        assert_eq!(coverage_at(line_cursor(3., words, 40.), 176., 40.), 1.);
    }

    #[test]
    fn gaps_hold_the_cursor_and_backward_seeks_restore_it() {
        let words = [(1., 2., 44.), (3., 4., 44.)];
        assert_eq!(line_cursor(2., words, 40.), line_cursor(2.9, words, 40.));
        assert!(line_cursor(3.5, words, 40.) > line_cursor(1.5, words, 40.));
        assert_eq!(line_cursor(1.5, words, 40.), 32.);
        assert_eq!(line_cursor(5., [], 40.), 0.);
    }
    #[test]
    fn a_completed_word_has_no_brightness_jump() {
        for x in 0..52 {
            let x = x as f32 + 0.5;
            assert_eq!(coverage(0., x, 52., 40.), 0.);
            assert_eq!(coverage(1., x, 52., 40.), 1.);
            assert!((coverage(1., x, 52., 40.) - coverage(0.9999, x, 52., 40.)).abs() < 0.001);
        }
    }
    #[test]
    fn fixed_pixel_samples_change_continuously_and_monotonically() {
        for x in 0..52 {
            let x = x as f32 + 0.5;
            let mut previous = 0.;
            for frame in 0..120 {
                let alpha = coverage(frame as f32 / 119., x, 52., 40.);
                assert!(alpha >= previous);
                assert!(alpha - previous < 0.02);
                previous = alpha;
            }
        }
    }
}
