//! Linux GPS backend using the GeoClue2 D-Bus service.
//!
//! Talks to `org.freedesktop.GeoClue2` via `zbus`. Treats any failure to
//! connect to D-Bus, create a GeoClue client, or receive a location update
//! within the timeout as `Ok(None)`. Surfaces only unexpected invariants as
//! hard errors.

use std::time::Duration;

use zbus::{proxy, Connection};

use crate::types::Location;

/// DesktopId identifying this client to the GeoClue service.
///
/// GeoClue requires a desktop-id for authorization. For non-packaged CLI
/// tools there is no `.desktop` file; we use the workspace binary name so
/// sysadmins can allow-list this tool in GeoClue's agent config.
const GEOCLUE_DESKTOP_ID: &str = "biscuit-location";

/// Requested accuracy level (5 = Exact).
const ACCURACY_LEVEL_EXACT: u32 = 5;

#[proxy(
    interface = "org.freedesktop.GeoClue2.Manager",
    default_service = "org.freedesktop.GeoClue2",
    default_path = "/org/freedesktop/GeoClue2/Manager",
    gen_blocking = false
)]
trait Manager {
    #[zbus(object = "Client")]
    fn get_client(&self);
}

#[proxy(
    interface = "org.freedesktop.GeoClue2.Client",
    default_service = "org.freedesktop.GeoClue2",
    gen_blocking = false
)]
trait Client {
    fn start(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_desktop_id(&self, value: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_requested_accuracy_level(&self, value: u32) -> zbus::Result<()>;

    #[zbus(property)]
    fn location(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    #[zbus(signal)]
    fn location_updated(
        &self,
        old: zbus::zvariant::ObjectPath<'_>,
        new: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.GeoClue2.Location",
    default_service = "org.freedesktop.GeoClue2",
    gen_blocking = false
)]
trait GClueLocation {
    #[zbus(property)]
    fn latitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn longitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn accuracy(&self) -> zbus::Result<f64>;
}

pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    let result = tokio::time::timeout(timeout, fetch_fix()).await;
    match result {
        Ok(Ok(location)) => Ok(Some(location)),
        // Any internal failure (service missing, permission denied, etc.)
        // or the outer timeout collapses to "no fix available".
        Ok(Err(_)) | Err(_) => Ok(None),
    }
}

async fn fetch_fix() -> crate::Result<Location> {
    use futures_util::StreamExt;

    let connection = Connection::system()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("dbus connect: {e}")))?;

    let manager = ManagerProxy::new(&connection)
        .await
        .map_err(|e| crate::LocationError::Internal(format!("geoclue manager: {e}")))?;

    let client = manager
        .get_client()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("geoclue get_client: {e}")))?;

    client
        .set_desktop_id(GEOCLUE_DESKTOP_ID)
        .await
        .map_err(|e| crate::LocationError::Internal(format!("set desktop id: {e}")))?;

    client
        .set_requested_accuracy_level(ACCURACY_LEVEL_EXACT)
        .await
        .map_err(|e| crate::LocationError::Internal(format!("set accuracy: {e}")))?;

    // Subscribe to updates BEFORE calling Start() so we don't miss the first
    // signal.
    let mut updates = client
        .receive_location_updated()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("subscribe updates: {e}")))?;

    client
        .start()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("geoclue start: {e}")))?;

    let update = updates
        .next()
        .await
        .ok_or_else(|| crate::LocationError::Internal("no update received".into()))?;
    let args = update
        .args()
        .map_err(|e| crate::LocationError::Internal(format!("update args: {e}")))?;

    let location = GClueLocationProxy::builder(&connection)
        .path(args.new.clone())
        .map_err(|e| crate::LocationError::Internal(format!("location path: {e}")))?
        .build()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("location proxy: {e}")))?;

    let latitude = location
        .latitude()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("latitude: {e}")))?;
    let longitude = location
        .longitude()
        .await
        .map_err(|e| crate::LocationError::Internal(format!("longitude: {e}")))?;
    let accuracy = location.accuracy().await.ok();

    // Best-effort stop; failures here do not invalidate the fix.
    let _ = client.stop().await;

    super::gps_location(latitude, longitude, accuracy)
}
