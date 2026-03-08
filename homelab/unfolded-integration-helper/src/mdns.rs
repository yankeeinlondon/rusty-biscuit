//! mDNS/DNS-SD advertisement for UC external integration drivers.
//!
//! Registers the integration as a `_uc-integration._tcp.local.` service so
//! the Unfolded Circle Remote can auto-discover it on the local network.

use mdns_sd::{ServiceDaemon, ServiceInfo};
use tracing::{error, info};

/// Service type used by the Unfolded Circle Remote for integration discovery.
const UC_SERVICE_TYPE: &str = "_uc-integration._tcp.local.";

/// Handle to a running mDNS advertisement. Dropping it shuts down the daemon.
pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
}

impl MdnsAdvertiser {
    /// Advertise an integration driver on the local network via mDNS/DNS-SD.
    ///
    /// ## Errors
    ///
    /// Returns an error if the mDNS daemon cannot be created or the service
    /// cannot be registered.
    pub fn new(
        driver_id: &str,
        published_name: &str,
        developer_name: &str,
        driver_version: &str,
        api_version: &str,
        port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let daemon = ServiceDaemon::new()?;

        let properties = [
            ("name", published_name),
            ("ver", driver_version),
            ("developer", developer_name),
            ("api_version", api_version),
        ];

        let hostname = format!("{}.local.", hostname_slug(driver_id));
        let service = ServiceInfo::new(
            UC_SERVICE_TYPE,
            driver_id,
            &hostname,
            "",
            port,
            &properties[..],
        )?
        .enable_addr_auto();

        daemon.register(service)?;

        info!(
            driver = driver_id,
            published_name,
            developer_name,
            port,
            service_type = UC_SERVICE_TYPE,
            "mDNS advertisement started"
        );

        Ok(Self { daemon })
    }

    /// Stop advertising and shut down the mDNS daemon.
    pub fn shutdown(self) {
        if let Err(e) = self.daemon.shutdown() {
            error!(error = %e, "failed to shut down mDNS daemon");
        }
    }
}

/// Convert a driver name into a valid mDNS hostname slug.
fn hostname_slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_slug_replaces_non_alphanumeric() {
        assert_eq!(hostname_slug("sony-receiver"), "sony-receiver");
        assert_eq!(hostname_slug("my driver_v2"), "my-driver-v2");
    }
}
