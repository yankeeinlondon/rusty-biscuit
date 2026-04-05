//! Windows GPS backend using `Windows.Devices.Geolocation.Geolocator`.
//!
//! Treats denied/unavailable location services and timeouts as `Ok(None)`.
//! The underlying Windows API blocks on the returned `IAsyncOperation`, so
//! the blocking call is executed on a dedicated thread and driven to
//! completion via a oneshot channel awaited by the async caller.

use std::time::Duration;

use tokio::sync::oneshot;
use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator};

use crate::types::Location;

pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    let (tx, rx) = oneshot::channel();

    std::thread::spawn(move || {
        let result = fetch_fix_blocking();
        let _ = tx.send(result);
    });

    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(Ok(fix))) => Ok(Some(fix)),
        // fetch_fix_blocking returned None or an error: all map to "no fix".
        Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => Ok(None),
    }
}

/// Fetches a one-shot geoposition synchronously using the Windows API.
///
/// Returns `Err(())` when the operation fails for any reason (denied,
/// unavailable, transient failure). The caller treats all failures as
/// "no fix available".
fn fetch_fix_blocking() -> std::result::Result<Location, ()> {
    // Ask the user for access to location services. If this resolves to
    // anything other than `Allowed`, we cannot obtain a fix.
    let access = Geolocator::RequestAccessAsync()
        .map_err(|_| ())?
        .join()
        .map_err(|_| ())?;
    if access != GeolocationAccessStatus::Allowed {
        return Err(());
    }

    let locator = Geolocator::new().map_err(|_| ())?;
    let op = locator.GetGeopositionAsync().map_err(|_| ())?;
    let position = op.join().map_err(|_| ())?;
    let coord = position.Coordinate().map_err(|_| ())?;
    let point = coord.Point().map_err(|_| ())?;
    let position = point.Position().map_err(|_| ())?;

    // `Accuracy` on `Geocoordinate` is reported in meters.
    let accuracy = coord.Accuracy().ok();

    super::gps_location(position.Latitude, position.Longitude, accuracy).map_err(|_| ())
}
