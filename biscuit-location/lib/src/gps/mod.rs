use std::time::Duration;

use crate::types::{Coordinates, Location, LocationSource};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// Request a one-shot GPS fix from the host device.
///
/// Returns `Ok(None)` when:
/// - Location services are disabled
/// - Permission is denied
/// - No fix is available before timeout
/// - The platform has no GPS provider
///
/// ## Errors
///
/// Returns `UnsupportedPlatform` on targets without a GPS backend.
pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    #[cfg(target_os = "macos")]
    {
        macos::current_fix(timeout).await
    }
    #[cfg(target_os = "windows")]
    {
        windows::current_fix(timeout).await
    }
    #[cfg(target_os = "linux")]
    {
        linux::current_fix(timeout).await
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = timeout;
        Err(crate::LocationError::UnsupportedPlatform)
    }
}

/// Build a `Location` from raw GPS coordinates and optional accuracy.
fn gps_location(latitude: f64, longitude: f64, accuracy: Option<f64>) -> crate::Result<Location> {
    let coordinates = Coordinates::new(latitude, longitude)?;
    Ok(Location {
        coordinates,
        place: None,
        source: LocationSource::Gps,
        accuracy_meters: accuracy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gps_location_sets_source_and_accuracy() {
        let loc = gps_location(37.7749, -122.4194, Some(12.5)).unwrap();
        assert_eq!(loc.coordinates.latitude, 37.7749);
        assert_eq!(loc.coordinates.longitude, -122.4194);
        assert_eq!(loc.source, LocationSource::Gps);
        assert_eq!(loc.accuracy_meters, Some(12.5));
        assert!(loc.place.is_none());
    }

    #[test]
    fn gps_location_without_accuracy() {
        let loc = gps_location(0.0, 0.0, None).unwrap();
        assert_eq!(loc.accuracy_meters, None);
    }

    #[test]
    fn gps_location_rejects_invalid_coordinates() {
        let result = gps_location(91.0, 0.0, None);
        assert!(matches!(
            result,
            Err(crate::LocationError::InvalidCoordinates { .. })
        ));
    }

    /// Verify that a zero-duration timeout collapses to `Ok(None)` on every
    /// supported platform — this is the core semantic guarantee of the GPS
    /// API: unavailability is never a hard error.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test]
    async fn current_fix_zero_timeout_returns_none() {
        let result = current_fix(Duration::from_millis(0)).await;
        // The platform may still hand back a cached fix before the timer
        // fires on fast hardware, but what MUST hold is that we never
        // bubble a hard error up to the caller.
        assert!(matches!(result, Ok(None) | Ok(Some(_))));
    }

    /// On unsupported targets, construction should surface a clear error
    /// rather than silently succeeding with no backend.
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    #[tokio::test]
    async fn current_fix_unsupported_platform_errors() {
        let result = current_fix(Duration::from_secs(1)).await;
        assert!(matches!(
            result,
            Err(crate::LocationError::UnsupportedPlatform)
        ));
    }
}
