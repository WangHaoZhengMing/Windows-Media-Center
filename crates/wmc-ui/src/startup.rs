use crate::{
    assets::MediaAssets,
    home::{HomeView, bind_keys},
};
use gpui_kit::*;

pub fn run() {
    wmc_player::application()
        .with_assets(MediaAssets)
        .run(|cx| {
            wmc_player::initialize_app_identity(cx);
            gpui_kit::init(cx);
            if let Err(error) = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(
                include_bytes!("../../wmc-assets/assets/wmc/fonts/segmcsb.ttf").as_slice(),
            )]) {
                eprintln!("Could not load original WMC menu font: {error:#}");
            }
            bind_keys(cx);
            wmc_player::bind_keys(cx);
            cx.activate(true);
            let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
            let result = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("WMC · Media Center".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    window_min_size: Some(size(px(900.), px(540.))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window.on_window_should_close(cx, |_, cx| {
                        cx.quit();
                        true
                    });
                    cx.new(|cx| HomeView::new(window, cx))
                },
            );
            if let Err(error) = result {
                eprintln!("Could not open Media Center: {error:#}");
                cx.quit();
            }
        });
}
