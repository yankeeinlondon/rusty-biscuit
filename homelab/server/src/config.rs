//! Configuration for homelab server.
//!
//! Re-exports shared config types from `homelab::config` and provides
//! server-specific functionality (env migration with petname generation).

use petname::Generator;

// Re-export shared config types
pub use homelab::config::{
    ArcamAmpService, ConfigError, HomeyConfig, SonyReceiverService, is_valid_device_name,
    parse_host_port,
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
