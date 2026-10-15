pub(crate) mod gallery;
pub(crate) mod image_cache;

use crate::shell::AppShell;
use gpui_kit::*;
pub(crate) struct PhotoPageState {
    pub path: Option<std::path::PathBuf>,
    pub images: Entity<image_cache::PhotoImages>,
    pub scroll: UniformListScrollHandle,
    pub visible: std::ops::Range<usize>,
    pub grid_generation: Option<u64>,
    pub zoom: f32,
    _subscription: Subscription,
}
impl PhotoPageState {
    pub fn new(cx: &mut Context<AppShell>) -> Self {
        let images = image_cache::PhotoImages::new(cx);
        let subscription = cx.observe(&images, |_, _, cx| cx.notify());
        Self {
            path: None,
            images,
            scroll: UniformListScrollHandle::new(),
            visible: 0..0,
            grid_generation: None,
            zoom: 1.,
            _subscription: subscription,
        }
    }
}
