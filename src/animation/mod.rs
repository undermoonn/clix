pub(crate) mod easing;
mod transition;

#[allow(unused_imports)]
pub use transition::{
    linear_progress, scale_seconds, scaled_elapsed_seconds, ExponentialAnimation,
    SPEED_MULTIPLIER,
};