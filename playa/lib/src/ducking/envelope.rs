//! Fade envelope calculations for smooth volume transitions.
//!
//! This module provides the math for computing fade steps that transition
//! volume levels smoothly without audible "zipper noise" (stepping artifacts).

use crate::ducking::DuckConfig;

/// Minimum number of fade steps to avoid audible stepping artifacts.
const MIN_STEPS: u32 = 3;

/// Target step interval in milliseconds (10-20ms is generally inaudible).
const TARGET_STEP_INTERVAL_MS: u32 = 15;

/// A single step in a volume fade transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadeStep {
    /// The target volume level for this step (0.0 to 1.0).
    pub volume: f32,
    /// Time to wait after applying this step, in milliseconds.
    pub delay_ms: u32,
}

/// Computes the fade steps needed to transition from one volume to another.
///
/// The algorithm:
/// 1. Calculate ideal step count based on target interval (15ms)
/// 2. Clamp to minimum 3 steps to avoid jarring transitions
/// 3. Distribute steps evenly across the ramp duration
/// 4. Use linear interpolation for volume values
///
/// ## Arguments
///
/// * `from_volume` - Starting volume (0.0 to 1.0)
/// * `to_volume` - Target volume (0.0 to 1.0)
/// * `config` - Ducking configuration with ramp duration
///
/// ## Returns
///
/// A vector of [`FadeStep`]s that, when applied in sequence, will smoothly
/// transition the volume. The final step always has the exact target volume.
///
/// ## Example
///
/// ```
/// use playa::ducking::{DuckConfig, compute_fade_steps};
///
/// let config = DuckConfig::default(); // 1000ms ramp
/// let steps = compute_fade_steps(1.0, 0.2, &config);
///
/// // First step starts the transition
/// assert!((steps[0].volume - 1.0).abs() < 0.1);
/// // Last step reaches the target
/// assert!((steps.last().unwrap().volume - 0.2).abs() < f32::EPSILON);
/// ```
pub fn compute_fade_steps(from_volume: f32, to_volume: f32, config: &DuckConfig) -> Vec<FadeStep> {
    let ramp_ms = config.ramp_ms();

    // Calculate step count: prefer ~15ms intervals, minimum 3 steps
    let ideal_steps = ramp_ms / TARGET_STEP_INTERVAL_MS;
    let step_count = ideal_steps.max(MIN_STEPS);

    // Calculate delay between steps
    let delay_per_step = ramp_ms / step_count;

    // Generate steps with linear interpolation
    (0..step_count)
        .map(|i| {
            let t = (i + 1) as f32 / step_count as f32;
            let volume = interpolate_volume(from_volume, to_volume, t);
            FadeStep {
                volume,
                delay_ms: delay_per_step,
            }
        })
        .collect()
}

/// Linearly interpolates between two volume levels.
///
/// ## Arguments
///
/// * `from` - Starting volume
/// * `to` - Target volume
/// * `t` - Interpolation factor (0.0 = from, 1.0 = to)
///
/// ## Returns
///
/// The interpolated volume value, clamped to 0.0..=1.0.
///
/// ## Example
///
/// ```
/// use playa::ducking::interpolate_volume;
///
/// assert!((interpolate_volume(1.0, 0.0, 0.5) - 0.5).abs() < f32::EPSILON);
/// assert!((interpolate_volume(0.0, 1.0, 0.25) - 0.25).abs() < f32::EPSILON);
/// ```
#[inline]
pub fn interpolate_volume(from: f32, to: f32, t: f32) -> f32 {
    let result = from + (to - from) * t;
    result.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_volume_at_zero_returns_from() {
        assert!((interpolate_volume(0.8, 0.2, 0.0) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn interpolate_volume_at_one_returns_to() {
        assert!((interpolate_volume(0.8, 0.2, 1.0) - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn interpolate_volume_at_half_returns_midpoint() {
        assert!((interpolate_volume(1.0, 0.0, 0.5) - 0.5).abs() < f32::EPSILON);
        assert!((interpolate_volume(0.0, 1.0, 0.5) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn interpolate_volume_clamps_result() {
        // Even with extreme inputs, result stays in range
        assert!(interpolate_volume(2.0, -1.0, 0.5) >= 0.0);
        assert!(interpolate_volume(2.0, -1.0, 0.5) <= 1.0);
    }

    #[test]
    fn compute_fade_steps_has_minimum_three_steps() {
        // Very short ramp should still have at least 3 steps
        let config = DuckConfig::new(10, 0.2).unwrap();
        let steps = compute_fade_steps(1.0, 0.2, &config);
        assert!(steps.len() >= 3);
    }

    #[test]
    fn compute_fade_steps_final_volume_is_target() {
        let config = DuckConfig::default();
        let steps = compute_fade_steps(1.0, 0.2, &config);
        let final_step = steps.last().unwrap();
        assert!((final_step.volume - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn compute_fade_steps_total_duration_matches_ramp() {
        let config = DuckConfig::new(1000, 0.2).unwrap();
        let steps = compute_fade_steps(1.0, 0.2, &config);
        let total_delay: u32 = steps.iter().map(|s| s.delay_ms).sum();
        // Should be close to ramp_ms (may have rounding differences)
        assert!((900..=1100).contains(&total_delay));
    }

    #[test]
    fn compute_fade_steps_are_monotonic_decreasing() {
        let config = DuckConfig::default();
        let steps = compute_fade_steps(1.0, 0.2, &config);

        for window in steps.windows(2) {
            assert!(
                window[0].volume >= window[1].volume,
                "Volume should decrease: {} -> {}",
                window[0].volume,
                window[1].volume
            );
        }
    }

    #[test]
    fn compute_fade_steps_are_monotonic_increasing() {
        let config = DuckConfig::default();
        let steps = compute_fade_steps(0.2, 1.0, &config);

        for window in steps.windows(2) {
            assert!(
                window[0].volume <= window[1].volume,
                "Volume should increase: {} -> {}",
                window[0].volume,
                window[1].volume
            );
        }
    }
}
