//! Launch composition from Microsoft.MediaCenter.Shell / LAUNCH.MCML.
use crate::theme::Layout;
use gpui_kit::{prelude::*, *};
use wmc_assets::LAUNCH_IMAGES;
use wmc_motion::launch_pose;

pub(super) fn launch_overlay(layout: Layout, viewport: Size<Pixels>, time: f32) -> Div {
    let width = f32::from(viewport.width);
    let height = f32::from(viewport.height);
    let mut overlay = div().absolute().size_full().overflow_hidden();
    // Original background layers scale around their lower-right corner.
    for (image, track) in [
        ("COMMON.BACKGROUND.NEWGRAD.PNG", "Launch.NewGrad"),
        ("COMMON.BACKGROUND.LAYER6.PNG", "Launch.Layer6"),
        ("COMMON.BACKGROUND.LAYER5.PNG", "Launch.Layer5"),
        ("COMMON.BACKGROUND.LAYER4.PNG", "Launch.Layer4"),
    ] {
        let pose = launch_pose(track, time);
        let (_, path, _, _) = image_info(image);
        let w = width * pose.scale[0];
        let h = height * pose.scale[1];
        overlay = overlay.child(
            img(*path)
                .absolute()
                .left(px(width - w))
                .top(px(height - h))
                .w(px(w))
                .h(px(h))
                .object_fit(ObjectFit::Fill)
                .opacity(pose.opacity),
        );
    }
    for (image, track) in [
        ("COMMON.BACKGROUND.GOODSHARD.PNG", "Launch.Shard1"),
        ("COMMON.BACKGROUND.LITESHARD.PNG", "Launch.Shard2"),
        ("COMMON.BACKGROUND.SHARDA1.PNG", "Launch.ShardA1.1"),
        ("COMMON.BACKGROUND.SHARDA1.PNG", "Launch.ShardA1.2"),
        ("COMMON.BACKGROUND.SHARDA2.PNG", "Launch.ShardA2.1"),
        ("COMMON.BACKGROUND.SHARDA2.PNG", "Launch.ShardA2.2"),
        ("COMMON.BACKGROUND.SHARDB1.PNG", "Launch.ShardB1.1"),
        ("COMMON.BACKGROUND.SHARDB1.PNG", "Launch.ShardB1.2"),
        ("COMMON.BACKGROUND.SHARDB1.PNG", "Launch.ShardB1.3"),
        ("COMMON.BACKGROUND.SHARDB2.PNG", "Launch.ShardB2.1"),
        ("COMMON.BACKGROUND.SHARDB2.PNG", "Launch.ShardB2.2"),
        ("COMMON.BACKGROUND.SHARDB3.PNG", "Launch.ShardB3"),
    ] {
        let pose = launch_pose(track, time);
        let (_, path, w, h) = image_info(image);
        overlay = overlay.child(
            img(*path)
                .absolute()
                .left(layout.unit(pose.position[0]))
                .top(layout.unit(pose.position[1]))
                .w(layout.unit(w * pose.scale[0]))
                .h(layout.unit(h * pose.scale[1]))
                .object_fit(ObjectFit::Fill)
                .opacity(pose.opacity),
        );
    }
    // Form layout centers the logo, with the original -60 vertical inset.
    let center_y = height / 2. - 60. * layout.scale;
    for (image, track) in [
        ("SKITTLE.BOTTOM.PIECE.PNG", "Launch.Skittle.Base"),
        (
            "SKITTLE.SUBBOTTOM.GLOW.PNG",
            "Launch.Skittle.Base.MotionBlur",
        ),
        ("SKITTLE.FLAG.PIECE.PNG", "Launch.Flag"),
        ("SKITTLE.MIDDLE.PIECE.PNG", "Launch.Skittle.Middle"),
        (
            "SKITTLE.MIDDLE.GLEAM.PIECE.PNG",
            "Launch.Skittle.Middle.Gleam",
        ),
        ("SKITTLE.TOP.PIECE.PNG", "Launch.Skittle.Top"),
        ("LAUNCH.TEXT.MOTIONBLUR.PNG", "Launch.Text.MotionBlur"),
        ("LAUNCH.TEXT.PNG", "Launch.Text"),
    ] {
        let pose = launch_pose(track, time);
        let (_, path, w, h) = image_info(image);
        let w = w * pose.scale[0] * layout.scale;
        let h = h * pose.scale[1] * layout.scale;
        overlay = overlay.child(
            img(*path)
                .absolute()
                .left(px(width / 2. + pose.position[0] * layout.scale - w / 2.))
                .top(px(center_y + pose.position[1] * layout.scale - h / 2.))
                .w(px(w))
                .h(px(h))
                .object_fit(ObjectFit::Fill)
                .opacity(pose.opacity),
        );
    }
    overlay
}

fn image_info(name: &str) -> &'static (&'static str, &'static str, f32, f32) {
    LAUNCH_IMAGES
        .iter()
        .find(|(resource, ..)| *resource == name)
        .expect("missing original launch image")
}
