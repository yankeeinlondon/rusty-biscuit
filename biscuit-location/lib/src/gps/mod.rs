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
