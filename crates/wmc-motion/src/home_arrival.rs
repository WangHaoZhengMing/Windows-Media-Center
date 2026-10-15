//! Home's separate Full-intensity arrival tracks, following the launch logo.
use crate::{LaunchPose, launch_pose};

pub const HOME_MENU_DELAY: f32 = 2.35;
pub const HOME_CATEGORY_STAGGER: f32 = 0.067;
pub const HOME_LINK_END: f32 = 3.6;

pub struct HomeArrival {
    time: f32,
    immediate: bool,
    pub selector: LaunchPose,
    pub links: LaunchPose,
    pub moving: bool,
}

impl HomeArrival {
    pub fn sample(time: f32, selected: usize, count: usize, immediate: bool) -> Self {
        let furthest = selected.max(count.saturating_sub(selected + 1));
        let end =
            HOME_LINK_END.max(HOME_MENU_DELAY + furthest as f32 * HOME_CATEGORY_STAGGER + 0.8);
        let mut result = Self {
            time,
            immediate,
            selector: settled(),
            links: settled(),
            moving: !immediate && time < end,
        };
        if !immediate {
            result.selector = launch_pose("Start.Selector.Launch.Full", time - HOME_MENU_DELAY);
            // This provider's keyframes already include the startup delay.
            result.links = launch_pose("Start.LinkStrip.Launch.Full", time);
        }
        result
    }

    pub fn category(&self, index: usize, selected: usize) -> LaunchPose {
        if self.immediate {
            return settled();
        }
        let delay = HOME_MENU_DELAY + index.abs_diff(selected) as f32 * HOME_CATEGORY_STAGGER;
        launch_pose("Start.Category.Launch.Full", self.time - delay)
    }
}

fn settled() -> LaunchPose {
    LaunchPose {
        opacity: 1.,
        scale: [1.; 2],
        position: [0.; 2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_selector_and_links_follow_distinct_original_tracks() {
        let start = HomeArrival::sample(2.35, 1, 4, false);
        assert_eq!(start.category(1, 1).opacity, 0.);
        assert_eq!(start.category(1, 1).scale, [2.; 2]);
        assert_eq!(start.selector.scale, [0.2; 2]);
        assert_eq!(start.links.opacity, 0.);
        let middle = HomeArrival::sample(2.71, 1, 4, false);
        assert!(middle.category(1, 1).opacity > middle.category(0, 1).opacity);
        assert_eq!(middle.category(0, 1).opacity, middle.category(2, 1).opacity);
        assert_eq!(middle.links.opacity, 0.);
        let end = HomeArrival::sample(3.61, 1, 4, false);
        assert!(!end.moving);
        assert_eq!(end.selector.scale, [1.; 2]);
        assert_eq!(end.links.opacity, 1.);
        assert_eq!(end.category(0, 1).opacity, 1.);
    }

    #[test]
    fn skip_or_reduced_motion_settles_every_layer_immediately() {
        let pose = HomeArrival::sample(0., 0, 4, true);
        assert!(!pose.moving);
        assert_eq!(pose.links.opacity, 1.);
        assert_eq!(pose.selector.scale, [1.; 2]);
        assert_eq!(pose.category(3, 0).scale, [1.; 2]);
    }
}
