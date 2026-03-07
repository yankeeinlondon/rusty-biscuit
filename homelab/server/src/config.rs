//! Configuration for homelab server.
//!
//! Re-exports shared config types from `homelab::config` and provides
//! server-specific functionality (env migration with petname generation).

use petname::Generator;

// Re-export shared config types
pub use homelab::config::{
    ArcamAmpService, ConfigError, EversoloService, HomeyConfig, SamsungTvService,
    SonyReceiverService, is_valid_device_name, parse_host_port,
};

/// Migrates environment variables to config if config is empty.
///
/// Checks for `SONY_RECEIVER` and `ARCAM_AMP` environment variables.
/// If found and config is empty for that device type, adds the device
/// with a random petname (e.g., "brave-cardinal").
///
/// ## Returns
///
/// Returns `true` if any migration occurred and config was modified.
pub fn migrate_from_env(config: &mut HomeyConfig) -> bool {
    let mut modified = false;

    // Migrate SONY_RECEIVER if present and no receivers configured
    if config.sony_receivers.is_empty()
        && let Ok(host) = std::env::var("SONY_RECEIVER")
        && !host.is_empty()
    {
        let name = generate_petname();
        let (host, port) = parse_host_port(&host, 10000);
        config
            .sony_receivers
            .insert(name, SonyReceiverService { host, port });
        modified = true;
    }

    // Migrate ARCAM_AMP if present and no amps configured
    if config.arcam_amps.is_empty()
        && let Ok(host) = std::env::var("ARCAM_AMP")
        && !host.is_empty()
    {
        let name = generate_petname();
        let (host, port) = parse_host_port(&host, 50000);
        config
            .arcam_amps
            .insert(name, ArcamAmpService { host, port });
        modified = true;
    }

    // Migrate EVERSOLO if present and no devices configured
    if config.eversolo_devices.is_empty()
        && let Ok(host) = std::env::var("EVERSOLO")
        && !host.is_empty()
    {
        let name = generate_petname();
        let (host, port) = parse_host_port(&host, 9529);
        config.eversolo_devices.insert(
            name,
            EversoloService {
                host,
                port,
                mac_address: None,
            },
        );
        modified = true;
    }

    // Migrate SAMSUNG_TV if present and no TVs configured
    if config.samsung_tvs.is_empty()
        && let Ok(host) = std::env::var("SAMSUNG_TV")
        && !host.is_empty()
    {
        let name = generate_petname();
        let (host, rest_port) = parse_host_port(&host, 8001);
        config.samsung_tvs.insert(
            name,
            SamsungTvService {
                host,
                rest_port,
                ws_port: 8002,
                use_https: false,
                mac_address: None,
            },
        );
        modified = true;
    }

    modified
}

/// Generates a random two-word petname like "brave-cardinal".
fn generate_petname() -> String {
    petname::Petnames::default()
        .generate_one(2, "-")
        .unwrap_or_else(|| "device".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_petname_format() {
        let name = generate_petname();
        assert!(
            name.contains('-'),
            "petname should contain hyphen: {}",
            name
        );
        assert!(
            is_valid_device_name(&name),
            "petname should be valid: {}",
            name
        );
    }

    #[test]
    fn test_generate_petname_uniqueness() {
        let names: Vec<_> = (0..5).map(|_| generate_petname()).collect();
        let unique_count = names.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(
            unique_count >= 3,
            "expected at least 3 unique names, got {}",
            unique_count
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_sony() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "192.168.1.100:10000");
        }

        let mut config = HomeyConfig::new();
        let modified = migrate_from_env(&mut config);

        assert!(modified);
        assert_eq!(config.sony_receivers.len(), 1);
        assert!(config.arcam_amps.is_empty());

        let (name, service) = config.sony_receivers.iter().next().unwrap();
        assert!(is_valid_device_name(name));
        assert_eq!(service.host, "192.168.1.100");
        assert_eq!(service.port, 10000);

        unsafe {
            std::env::remove_var("SONY_RECEIVER");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_arcam() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("ARCAM_AMP", "192.168.1.101");
        }

        let mut config = HomeyConfig::new();
        let modified = migrate_from_env(&mut config);

        assert!(modified);
        assert!(config.sony_receivers.is_empty());
        assert_eq!(config.arcam_amps.len(), 1);

        let (name, service) = config.arcam_amps.iter().next().unwrap();
        assert!(is_valid_device_name(name));
        assert_eq!(service.host, "192.168.1.101");
        assert_eq!(service.port, 50000);

        unsafe {
            std::env::remove_var("ARCAM_AMP");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_skips_if_already_configured() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "192.168.1.100");
        }

        let mut config = HomeyConfig::new();
        config.sony_receivers.insert(
            "existing".to_string(),
            SonyReceiverService {
                host: "10.0.0.1".to_string(),
                port: 10000,
            },
        );

        let modified = migrate_from_env(&mut config);

        assert!(!modified);
        assert_eq!(config.sony_receivers.len(), 1);
        assert!(config.sony_receivers.contains_key("existing"));

        unsafe {
            std::env::remove_var("SONY_RECEIVER");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_eversolo() {
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::remove_var("EVERSOLO");
            std::env::set_var("EVERSOLO", "192.168.1.50");
        }

        let mut config = HomeyConfig::new();
        let modified = migrate_from_env(&mut config);

        assert!(modified);
        assert_eq!(config.eversolo_devices.len(), 1);

        let (name, service) = config.eversolo_devices.iter().next().unwrap();
        assert!(is_valid_device_name(name));
        assert_eq!(service.host, "192.168.1.50");
        assert_eq!(service.port, 9529);

        unsafe {
            std::env::remove_var("EVERSOLO");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_from_env_eversolo_with_port() {
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::remove_var("EVERSOLO");
            std::env::set_var("EVERSOLO", "192.168.1.50:9530");
        }

        let mut config = HomeyConfig::new();
        let modified = migrate_from_env(&mut config);

        assert!(modified);
        assert_eq!(config.eversolo_devices.len(), 1);

        let (_, service) = config.eversolo_devices.iter().next().unwrap();
        assert_eq!(service.host, "192.168.1.50");
        assert_eq!(service.port, 9530);

        unsafe {
            std::env::remove_var("EVERSOLO");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_skips_if_eversolo_configured() {
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::remove_var("EVERSOLO");
            std::env::set_var("EVERSOLO", "192.168.1.50");
        }

        let mut config = HomeyConfig::new();
        config.eversolo_devices.insert(
            "existing".to_string(),
            EversoloService {
                host: "10.0.0.1".to_string(),
                port: 9529,
                mac_address: None,
            },
        );

        let modified = migrate_from_env(&mut config);

        assert!(!modified);
        assert_eq!(config.eversolo_devices.len(), 1);
        assert!(config.eversolo_devices.contains_key("existing"));

        unsafe {
            std::env::remove_var("EVERSOLO");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_migrate_skips_empty_env_var() {
        // SAFETY: Tests run serially via serial_test
        unsafe {
            std::env::remove_var("SONY_RECEIVER");
            std::env::remove_var("ARCAM_AMP");
            std::env::set_var("SONY_RECEIVER", "");
        }

        let mut config = HomeyConfig::new();
        let modified = migrate_from_env(&mut config);

        assert!(!modified);
        assert!(config.sony_receivers.is_empty());

        unsafe {
            std::env::remove_var("SONY_RECEIVER");
        }
    }
}
