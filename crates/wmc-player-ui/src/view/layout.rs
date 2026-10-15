//! Recovered style-1 geometry in window DIPs; typography has its own scale.
pub(super) const LYRIC_FONT_SIZE: f32 = 48.;
pub(super) const LYRIC_LINE_HEIGHT: f32 = 60.;
pub(super) const LYRIC_ROW_GAP: f32 = 40.;
pub(crate) struct Columns {
    pub cover_left: f32,
    pub cover_top: f32,
    pub cover_size: f32,
    pub info_left: f32,
    pub info_top: f32,
    pub info_width: f32,
    pub lyric_left: f32,
    pub lyric_top: f32,
    pub lyric_width: f32,
    pub lyric_height: f32,
    pub wide: bool,
}
pub(super) fn ui_scale(w: f32, h: f32) -> f32 {
    if w / h >= 1.1 && w >= 900. {
        (w / 1376.).min(h / 1030.)
    } else {
        (w / 626.).min(h / 1382.).max(0.75)
    }
}
impl Columns {
    pub fn new(w: f32, h: f32, s: f32) -> Self {
        let wide = w / h >= 1.1 && w >= 900.;
        let (x, y, size, ix, iy, iw, lx, ly, lw) = if wide {
            let size = h * 0.42 * 1.25;
            let x = w / 4. - size / 2.;
            let y = h * 0.4 - size / 2.;
            (
                x,
                y,
                size,
                x,
                y + size + h * 0.04,
                size,
                w / 2. + 18.,
                0.,
                w / 2. - 72. * s,
            )
        } else {
            let m = 40_f32.max(w * 0.08);
            let a = 80_f32.max(w * 0.1);
            let ix = m + a + 20.;
            // Center the whole right-hand group on the compact artwork. Its
            // 92 UI units include metadata, seek hit area and time labels.
            let info_y = 80. + (a - 92. * s) / 2.;
            (
                m,
                80.,
                a,
                ix,
                info_y,
                (w - ix - 40.).max(100.),
                m,
                80. + a,
                w - 2. * m,
            )
        };
        Self {
            cover_left: x / s,
            cover_top: y / s,
            cover_size: size / s,
            info_left: ix / s,
            info_top: iy / s,
            info_width: iw / s,
            lyric_left: lx / s,
            lyric_top: ly / s,
            lyric_width: lw.max(80.) / s,
            // Reserve the user's WMC transport bar below the lyric viewport.
            lyric_height: (h - ly - 52.).max(100.) / s,
            wide,
        }
    }
}
pub(super) fn lyric_anchor(h: f32) -> f32 {
    h * 0.42 - LYRIC_LINE_HEIGHT / 2.
}
pub(super) fn lyric_target(offset: f32, active: f32, anchor: f32, interlude: f32) -> f32 {
    anchor + offset - active
        + if offset >= active {
            (LYRIC_LINE_HEIGHT + LYRIC_ROW_GAP) * interlude
        } else {
            0.
        }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovered_layout_survives_resizing() {
        let s = ui_scale(1376., 1030.);
        assert_eq!(s, 1.);
        let c = Columns::new(1376., 1030., s);
        assert!((c.cover_size - 540.75).abs() < 0.01);
        assert!((c.cover_top - 141.625).abs() < 0.01);
        assert!(c.cover_left + c.cover_size < c.lyric_left);
        assert!(Columns::new(900., 800., ui_scale(900., 800.)).wide);
        assert!(!Columns::new(900., 850., ui_scale(900., 850.)).wide);
        assert!(!Columns::new(899., 600., ui_scale(899., 600.)).wide);
        for (w, h) in [
            (520., 900.),
            (900., 800.),
            (900., 850.),
            (1000., 700.),
            (1776., 1248.),
            (2560., 1440.),
        ] {
            let s = ui_scale(w, h);
            let c = Columns::new(w, h, s);
            assert!(c.cover_left >= 0. && c.lyric_width > 0.);
            assert!((c.lyric_left + c.lyric_width) * s <= w);
            if !c.wide {
                assert!(c.cover_top + c.cover_size <= c.lyric_top);
            }
        }
    }
    #[test]
    fn instrumental_row_stays_clear_of_a_wrapped_previous_stanza() {
        let height = LYRIC_LINE_HEIGHT * 3.;
        let next = height + LYRIC_ROW_GAP;
        let previous = lyric_target(0., next, 260., 1.);
        assert!(previous + height + 8. < 260.);
        assert!(lyric_target(next, next, 260., 1.) - 8. > 308.);
    }
}
