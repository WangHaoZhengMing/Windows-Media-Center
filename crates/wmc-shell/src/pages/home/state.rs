use crate::navigation::{CATEGORIES, HomeState};
use std::time::Instant;
use wmc_motion::CategoryMotion;
pub(crate) struct HomePageState {
    pub navigation: HomeState,
    pub motion: CategoryMotion<{ CATEGORIES.len() }>,
    pub epoch: Instant,
    pub launch_epoch: Option<Instant>,
    pub launch_dismissed: bool,
}
impl HomePageState {
    pub fn new() -> Self {
        Self {
            navigation: HomeState::default(),
            motion: CategoryMotion::new(0),
            epoch: Instant::now(),
            launch_epoch: None,
            launch_dismissed: false,
        }
    }
}
