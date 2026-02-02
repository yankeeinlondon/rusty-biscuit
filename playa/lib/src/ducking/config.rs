//! Configuration for audio ducking behavior.

use crate::ducking::DuckingError;

/// Configuration for audio ducking during playback.
///
/// Controls how system audio is attenuated while Playa plays:
/// - `ramp_ms`: Duration of the fade-down and fade-up transitions
/// - `floor_scalar`: Target volume level during ducking (0.0 = silent, 1.0 = full)
///
/// ## Default Values
///
/// - Ramp: 1000 ms (1 second)
/// - Floor: 0.2 (20% volume)
///
/// ## Example
///
/// ```
/// use playa::ducking::DuckConfig;
///
/// // Use defaults
/// let config = DuckConfig::default();
/// assert_eq!(config.ramp_ms(), 1000);
/// assert!((config.floor_scalar() - 0.2).abs() < f32::EPSILON);
///
/// // Custom configuration
/// let config = DuckConfig::new(500, 0.1).unwrap();
/// assert_eq!(config.ramp_ms(), 500);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuckConfig {
    /// Duration of fade transitions in milliseconds.
    ramp_ms: u32,
    /// Target volume level during ducking (0.0 to 1.0).
    floor_scalar: f32,
}

impl DuckConfig {
    /// Creates a new ducking configuration with validation.
    ///
    /// ## Errors
    ///
    /// Returns [`DuckingError::InvalidRampDuration`] if `ramp_ms` is 0.
    /// Returns [`DuckingError::InvalidFloorScalar`] if `floor_scalar` is outside 0.0..=1.0.
    pub fn new(ramp_ms: u32, floor_scalar: f32) -> Result<Self, DuckingError> {
        if ramp_ms == 0 {
            return Err(DuckingError::InvalidRampDuration(ramp_ms));
        }
        if !(0.0..=1.0).contains(&floor_scalar) {
            return Err(DuckingError::InvalidFloorScalar(floor_scalar));
        }
        Ok(Self {
            ramp_ms,
            floor_scalar,
        })
    }

    /// Returns the ramp duration in milliseconds.
    #[inline]
    pub fn ramp_ms(&self) -> u32 {
        self.ramp_ms
    }

    /// Returns the floor scalar (target volume during ducking).
    #[inline]
    pub fn floor_scalar(&self) -> f32 {
        self.floor_scalar
    }
}

impl Default for DuckConfig {
    fn default() -> Self {
        Self {
            ramp_ms: 1000,
            floor_scalar: 0.2,
        }
    }
}
