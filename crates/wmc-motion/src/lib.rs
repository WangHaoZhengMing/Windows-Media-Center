//! Recovered WMC timing profiles and deterministic, GUI-independent motion.
mod category;
pub mod context_menu;
mod home_arrival;
mod launch;
pub mod player_entry;
pub mod player_return;
pub mod profile;

pub use category::{CategoryMotion, CategoryPose, MenuSample};
pub use home_arrival::HomeArrival;
pub use launch::{LAUNCH_DURATION, LaunchPose, launch_pose};

#[cfg(test)]
mod tests;
