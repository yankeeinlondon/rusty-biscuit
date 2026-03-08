//! Device discovery trait and bounded scan utilities.
//!
//! Each integration implements [`DeviceDiscovery`] to probe and validate
//! candidate addresses. The helper provides [`bounded_scan`] for concurrency-
//! and timeout-bounded scanning across a candidate list.

use std::collections::HashSet;
use std::time::Duration;

use crate::registry::{DeviceMetadata, KnownDevice};

/// Integration-specific device discovery and validation.
#[allow(async_fn_in_trait)]
pub trait DeviceDiscovery: Send + Sync {
    /// Probe a single host and validate device identity.
    ///
    /// Returns `None` if the host is not a valid device of this type.
    async fn validate_host(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Option<DeviceMetadata>;
}

/// Build a deduplicated candidate list from multiple sources, ordered by priority.
///
/// Priority: persisted known devices first, then CLI hints, then explicit addresses.
#[must_use]
pub fn build_candidate_list(
    persisted_known: &[KnownDevice],
    cli_hints: &[(String, u16)],
    explicit: &[(String, u16)],
) -> Vec<(String, u16)> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    // 1. Previously validated persisted devices (fastest — just re-validate)
    for device in persisted_known {
        let key = (device.host.clone(), device.port);
        if seen.insert(key.clone()) {
            candidates.push(key);
        }
    }

    // 2. CLI hint addresses
    for hint in cli_hints {
        if seen.insert(hint.clone()) {
            candidates.push(hint.clone());
        }
    }

    // 3. Explicit addresses
    for addr in explicit {
        if seen.insert(addr.clone()) {
            candidates.push(addr.clone());
        }
    }

    candidates
}

/// Scan candidates with concurrency limit and total timeout.
///
/// Uses `tokio::time::timeout` for total bound and scans candidates
/// sequentially (simple and predictable). Returns validated devices found,
/// including partial results if the total timeout fires.
pub async fn bounded_scan<D: DeviceDiscovery>(
    discovery: &D,
    candidates: Vec<(String, u16)>,
    per_host_timeout: Duration,
    total_timeout: Duration,
) -> Vec<KnownDevice> {
    let mut results = Vec::new();

    let scan = async {
        for (host, port) in &candidates {
            if let Some(metadata) = discovery.validate_host(host, *port, per_host_timeout).await {
                let device_id = metadata
                    .mac_address
                    .clone()
                    .unwrap_or_else(|| format!("{host}:{port}"));

                results.push(KnownDevice {
                    device_id,
                    source: crate::registry::DiscoverySource::NetworkScan,
                    host: host.clone(),
                    port: *port,
                    metadata,
                    last_validated: Some(chrono::Utc::now()),
                });
            }
        }
    };

    let _ = tokio::time::timeout(total_timeout, scan).await;
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NeverResponds;

    impl DeviceDiscovery for NeverResponds {
        async fn validate_host(
            &self,
            _host: &str,
            _port: u16,
            timeout: Duration,
        ) -> Option<DeviceMetadata> {
            tokio::time::sleep(timeout + Duration::from_secs(1)).await;
            None
        }
    }

    struct AlwaysFinds {
        model: String,
    }

    impl DeviceDiscovery for AlwaysFinds {
        async fn validate_host(
            &self,
            _host: &str,
            _port: u16,
            _timeout: Duration,
        ) -> Option<DeviceMetadata> {
            Some(DeviceMetadata {
                model: Some(self.model.clone()),
                mac_address: Some("AA:BB:CC:DD:EE:FF".to_string()),
                ..Default::default()
            })
        }
    }

    #[test]
    fn candidate_list_deduplicates() {
        let persisted = vec![KnownDevice {
            device_id: "dev1".into(),
            source: crate::registry::DiscoverySource::Persisted,
            host: "192.168.1.10".into(),
            port: 9529,
            metadata: Default::default(),
            last_validated: None,
        }];

        let cli = vec![
            ("192.168.1.10".to_string(), 9529), // duplicate
            ("192.168.1.20".to_string(), 9529),
        ];

        let explicit = vec![("192.168.1.30".to_string(), 9529)];

        let result = build_candidate_list(&persisted, &cli, &explicit);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "192.168.1.10");
        assert_eq!(result[1].0, "192.168.1.20");
        assert_eq!(result[2].0, "192.168.1.30");
    }

    #[tokio::test]
    async fn scan_empty_candidates() {
        let discovery = NeverResponds;
        let results = bounded_scan(
            &discovery,
            vec![],
            Duration::from_secs(1),
            Duration::from_secs(5),
        )
        .await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn total_timeout_cancels_in_progress() {
        let discovery = NeverResponds;
        let candidates = vec![
            ("10.0.0.1".to_string(), 9529),
            ("10.0.0.2".to_string(), 9529),
        ];

        let start = tokio::time::Instant::now();
        let results = bounded_scan(
            &discovery,
            candidates,
            Duration::from_secs(10),
            Duration::from_millis(100),
        )
        .await;

        assert!(results.is_empty());
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn successful_scan_returns_device() {
        let discovery = AlwaysFinds {
            model: "DMP-A8".to_string(),
        };
        let candidates = vec![("192.168.1.50".to_string(), 9529)];

        let results = bounded_scan(
            &discovery,
            candidates,
            Duration::from_secs(5),
            Duration::from_secs(10),
        )
        .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.model.as_deref(), Some("DMP-A8"));
        assert_eq!(results[0].device_id, "AA:BB:CC:DD:EE:FF");
    }
}
