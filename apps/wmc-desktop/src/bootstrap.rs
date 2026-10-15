use gpui_kit::*;
use wmc_design::MediaAssets;
use wmc_shell::{AppShell, bind_keys};

pub fn run() {
    let (application, hide_cursor) = crate::desktop_host::application();
    application.with_assets(MediaAssets).run(move |cx| {
        #[cfg(target_os = "windows")]
        cx.set_app_identity(
            wmc_platform::media_session::APP_ID,
            wmc_platform::media_session::APP_NAME,
        );
        gpui_kit::init(cx);
        if let Err(error) = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(
            include_bytes!("../../../crates/wmc-assets/assets/wmc/fonts/segmcsb.ttf").as_slice(),
        )]) {
            eprintln!("Could not load original WMC menu font: {error:#}");
        }
        bind_keys(cx);
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
                cx.new(|cx| {
                    AppShell::new(
                        wmc_application::Application::new(),
                        hide_cursor.clone(),
                        window,
                        cx,
                    )
                })
            },
        );
        if let Err(error) = result {
            eprintln!("Could not open Media Center: {error:#}");
            cx.quit();
        }
    });
}
