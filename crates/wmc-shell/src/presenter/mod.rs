mod playback;
mod player_transition;

use gpui_kit::*;
use std::{sync::Arc, time::Instant};
#[derive(Default)]
pub(crate) struct PlayerPageState {
    pub view: Option<Entity<wmc_player_ui::PlayerView>>,
    pub visible: bool,
    pub cover: Option<Arc<RenderImage>>,
    pub subscription: Option<Subscription>,
    pub entering: Option<Instant>,
    pub exiting: Option<Instant>,
    pub library_return: Option<Instant>,
}
