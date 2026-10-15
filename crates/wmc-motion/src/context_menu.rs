//! Original menu tracks and the actual CIL interpolation formulas.
//! See docs/reference/context-menu.il (ComputeExp/Log/SCurve, RVAs 0x514E0..0x5160C).

#[derive(Clone, Copy, Debug)]
pub struct ContextMenuPose {
    pub scale: [f32; 2],
    pub alpha: f32,
    pub items_alpha: f32,
    pub items_position: f32,
    pub thumbnail_scale: f32,
    pub contextual_alpha: f32,
    pub page_alpha: f32,
    pub page_z: f32,
}

pub fn mcml_exp(t: f32, weight: f32) -> f32 {
    if weight == 0. {
        return t;
    }
    let base = 10. * f64::from(weight);
    // CIL uses double intermediates then conv.r4; retain its order of operations.
    let normalizer = (base - 1.) / 9.;
    (((base.powf(f64::from(t)) - 1.) / 9.) / normalizer) as f32
}

pub fn mcml_log(t: f32, weight: f32) -> f32 {
    if weight == 0. {
        return t;
    }
    let base = 10. * f64::from(weight);
    (1. - ((base.powf(1. - f64::from(t)) - 1.) / 9.) / ((base - 1.) / 9.)) as f32
}

pub fn mcml_s_curve(t: f32, weight: f32) -> f32 {
    if weight == 0. {
        return t;
    }
    let base = 10. * f64::from(weight);
    let normalizer = ((base - 1.) / 9.) * 2.;
    if t < 0.5 {
        (((base.powf(2. * f64::from(t)) - 1.) / 9.) / normalizer) as f32
    } else {
        (1. - ((base.powf(2. * (1. - f64::from(t))) - 1.) / 9.) / normalizer) as f32
    }
}

fn phase(time: f32, start: f32, end: f32) -> f32 {
    ((time - start) / (end - start)).clamp(0., 1.)
}

fn blend(a: f32, b: f32, t: f32) -> f32 {
    ((1. - f64::from(t)) * f64::from(a) + f64::from(t) * f64::from(b)) as f32
}

pub fn context_menu_show(time: f32, reduced: bool) -> ContextMenuPose {
    let t = if reduced { 1. } else { time.max(0.) };
    let x = blend(0.25, 1., mcml_log(phase(t, 0.1, 0.19), 0.25));
    let y = if t < 0.3 {
        blend(0.1, 0.9, mcml_log(phase(t, 0.19, 0.3), 4.))
    } else {
        blend(0.9, 1., mcml_log(phase(t, 0.3, 0.4), 4.))
    };
    ContextMenuPose {
        scale: [x, y],
        alpha: if t < 0.19 {
            blend(0., 0.5, mcml_log(phase(t, 0., 0.19), 0.5))
        } else {
            blend(0.5, 1., phase(t, 0.19, 0.3))
        },
        items_alpha: mcml_exp(phase(t, 0.25, 0.5), 1.),
        items_position: mcml_log(phase(t, 0.3, 0.45), 1.),
        thumbnail_scale: blend(1., 1.3, mcml_log(phase(t, 0., 0.1), 1.)),
        contextual_alpha: mcml_s_curve(phase(t, 0., 0.1), 1.),
        page_alpha: blend(1., 128. / 255., mcml_s_curve(phase(t, 0., 0.3), 1.)),
        page_z: blend(0., 120., mcml_log(phase(t, 0., 0.2), 1.)),
    }
}

pub fn context_menu_hide(time: f32, from: ContextMenuPose, reduced: bool) -> ContextMenuPose {
    let t = if reduced { 1. } else { time.max(0.) };
    let shrink = mcml_exp(phase(t, 0., 0.25), 1.);
    ContextMenuPose {
        scale: [
            blend(from.scale[0], 0.65, shrink),
            blend(from.scale[1], 0.65, shrink),
        ],
        alpha: blend(from.alpha, 0., shrink),
        items_alpha: blend(from.items_alpha, 0., mcml_s_curve(phase(t, 0., 0.25), 1.)),
        items_position: from.items_position,
        thumbnail_scale: blend(from.thumbnail_scale, 1., mcml_exp(phase(t, 0., 0.15), 1.)),
        contextual_alpha: blend(
            from.contextual_alpha,
            0.,
            mcml_s_curve(phase(t, 0.5, 0.6), 1.),
        ),
        page_alpha: blend(from.page_alpha, 1., mcml_s_curve(phase(t, 0., 0.35), 1.)),
        page_z: blend(from.page_z, 0., mcml_s_curve(phase(t, 0.1, 0.25), 1.)),
    }
}

pub fn context_menu_focus_glow(time: f32) -> f32 {
    let t = time.rem_euclid(3.);
    if t < 1.5 {
        0.63 * mcml_s_curve(t / 1.5, 1.)
    } else {
        0.63 * (1. - mcml_s_curve((t - 1.5) / 1.5, 1.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_il_curves_match_numeric_checkpoints() {
        assert!((mcml_exp(0.5, 1.) - 0.24025308).abs() < 1e-7);
        assert!((mcml_log(0.5, 4.) - 0.86347294).abs() < 1e-7);
        assert!((mcml_s_curve(0.25, 1.) - 0.12012654).abs() < 1e-7);
        for weight in [0., 0.25, 0.5, 1., 4.] {
            assert_eq!(mcml_exp(0., weight), 0.);
            assert_eq!(mcml_exp(1., weight), 1.);
            assert_eq!(mcml_s_curve(0.5, weight), 0.5);
        }
    }

    #[test]
    fn original_open_keyframes_and_delayed_labels_are_preserved() {
        assert_eq!(context_menu_show(0., false).scale, [0.25, 0.1]);
        assert_eq!(context_menu_show(0.1, false).scale, [0.25, 0.1]);
        assert_eq!(context_menu_show(0.19, false).scale, [1., 0.1]);
        assert_eq!(context_menu_show(0.3, false).scale, [1., 0.9]);
        assert_eq!(context_menu_show(0.4, false).scale, [1., 1.]);
        assert_eq!(context_menu_show(0.25, false).items_alpha, 0.);
        assert_eq!(context_menu_show(0.5, false).items_alpha, 1.);
        assert_eq!(context_menu_show(0.3, false).items_position, 0.);
        assert_eq!(context_menu_show(0.45, false).items_position, 1.);
        assert_eq!(context_menu_show(0.1, false).thumbnail_scale, 1.3);
        assert_eq!(context_menu_show(0.3, false).page_alpha, 128. / 255.);
        assert_eq!(context_menu_show(0.2, false).page_z, 120.);
    }

    #[test]
    fn mid_open_close_starts_at_current_pose_and_finishes_at_original_deadlines() {
        let from = context_menu_show(0.22, false);
        let initial = context_menu_hide(0., from, false);
        assert_eq!(initial.scale, from.scale);
        assert_eq!(initial.alpha, from.alpha);
        let end = context_menu_hide(0.25, from, false);
        assert_eq!(end.scale, [0.65; 2]);
        assert_eq!(end.alpha, 0.);
        assert_eq!(end.items_alpha, 0.);
        assert_eq!(context_menu_hide(0.15, from, false).thumbnail_scale, 1.);
        assert_eq!(
            context_menu_hide(0.5, from, false).contextual_alpha,
            from.contextual_alpha
        );
        assert_eq!(context_menu_hide(0.6, from, false).contextual_alpha, 0.);
        assert_eq!(context_menu_hide(0.35, from, false).page_alpha, 1.);
        assert_eq!(context_menu_hide(0.1, from, false).page_z, from.page_z);
        assert_eq!(context_menu_show(0., true).items_alpha, 1.);
    }
}
