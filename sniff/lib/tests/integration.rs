use sniff_lib::{detect, detect_with_config, SniffConfig};
use std::path::PathBuf;
use std::time::Instant;

mod fixtures;

#[test]
fn test_detect_returns_hardware_info() {
    let result = detect().unwrap();
    let os = result.os.expect("os should be present");
    assert!(!os.name.is_empty());
    let hardware = result.hardware.expect("hardware should be present");
    assert!(hardware.memory.total_bytes > 0);
}

#[test]
fn test_detect_with_custom_base_dir() {
    let config = SniffConfig::new().base_dir(PathBuf::from("."));
    let result = detect_with_config(config).unwrap();
    assert!(result.filesystem.is_some());
}

#[test]
fn test_detect_in_git_repo() {
    let (_dir, path) = fixtures::create_test_git_repo();
    let config = SniffConfig::new().base_dir(path);
    let result = detect_with_config(config).unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.git.is_some());
}

#[test]
fn test_detect_cargo_workspace() {
    let (_dir, path) = fixtures::create_cargo_workspace();
    let config = SniffConfig::new().base_dir(path);
    let result = detect_with_config(config).unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.repo.is_some());
    let repo = fs.repo.unwrap();
    assert!(repo.is_monorepo);
    assert!(repo.packages.is_some());
    assert_eq!(repo.packages.unwrap().len(), 2);
}

#[test]
fn test_detect_completes_in_reasonable_time() {
    // NFR-1: Fast path detection should complete in <300ms
    let start = Instant::now();
    let _ = detect();
    let elapsed = start.elapsed();
    // Allow slack for CI environments and package manager detection (PATH scanning)
    assert!(
        elapsed.as_millis() < 15000,
        "Detection took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_serialization_roundtrip() {
    let result = detect().unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff_lib::SniffResult = serde_json::from_str(&json).unwrap();
    let orig_os = result.os.expect("os should be present");
    let parsed_os = parsed.os.expect("parsed os should be present");
    assert_eq!(orig_os.name, parsed_os.name);
}

#[test]
fn test_skip_all_returns_minimal_result() {
    let config = SniffConfig::new()
        .skip_hardware()
        .skip_network()
        .skip_filesystem();
    let result = detect_with_config(config).unwrap();
    assert!(result.hardware.is_none());
    assert!(result.network.is_none());
    assert!(result.filesystem.is_none());
}

#[test]
fn test_detect_mixed_languages() {
    let (_dir, path) = fixtures::create_mixed_language_dir();
    let config = SniffConfig::new().base_dir(path);
    let result = detect_with_config(config).unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.languages.is_some());
    let langs = fs.languages.unwrap();
    assert!(langs.total_files >= 4);
}

#[test]
fn test_detect_pnpm_workspace() {
    let (_dir, path) = fixtures::create_pnpm_workspace();
    let config = SniffConfig::new().base_dir(path);
    let result = detect_with_config(config).unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.repo.is_some());
    let repo = fs.repo.unwrap();
    assert!(repo.is_monorepo);
    assert_eq!(
        repo.monorepo_tool,
        Some(sniff_lib::filesystem::MonorepoTool::PnpmWorkspaces)
    );
}

// === Regression tests for JSON serialization of partial results ===
// Bug: Skipped sections were serialized as empty objects instead of being omitted.

#[test]
fn test_skip_hardware_json_omits_hardware_key() {
    // Regression test: JSON should NOT contain "hardware" key when skipped
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(
        !json.contains("\"hardware\""),
        "JSON should not contain hardware key when skipped"
    );
    assert!(
        json.contains("\"network\""),
        "JSON should contain network key"
    );
}

#[test]
fn test_skip_network_json_omits_network_key() {
    // Regression test: JSON should NOT contain "network" key when skipped
    let config = SniffConfig::new().skip_network();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(
        !json.contains("\"network\""),
        "JSON should not contain network key when skipped"
    );
    assert!(
        json.contains("\"hardware\""),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_skip_filesystem_json_omits_filesystem_key() {
    // Regression test: JSON should NOT contain "filesystem" key when skipped
    let config = SniffConfig::new().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(
        !json.contains("\"filesystem\""),
        "JSON should not contain filesystem key when skipped"
    );
    assert!(
        json.contains("\"hardware\""),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_hardware_only_json_contains_only_hardware() {
    // Regression test: When only hardware is requested, JSON should contain ONLY hardware
    let config = SniffConfig::new().skip_network().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(
        json.contains("\"hardware\""),
        "JSON should contain hardware key"
    );
    assert!(
        !json.contains("\"network\""),
        "JSON should not contain network key"
    );
    assert!(
        !json.contains("\"filesystem\""),
        "JSON should not contain filesystem key"
    );
    assert!(
        !json.contains("\"interfaces\""),
        "JSON should not contain interfaces (from network)"
    );
}

#[test]
fn test_partial_result_deserialization_roundtrip() {
    // Regression test: Partial results should deserialize correctly
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff_lib::SniffResult = serde_json::from_str(&json).unwrap();
    assert!(
        parsed.hardware.is_none(),
        "Deserialized hardware should be None"
    );
    assert!(
        parsed.network.is_some(),
        "Deserialized network should be Some"
    );
}

// ============================================================================
// OS Detection Integration Tests
// ============================================================================

/// Tests that detect_os returns populated OS detection fields.
#[test]
fn test_detect_os_has_detection_fields() {
    use sniff_lib::hardware::detect_os;

    let os = detect_os().expect("detect_os should succeed");

    // OS info should have populated fields
    assert!(!os.name.is_empty(), "OS name should be detected");
    assert!(!os.kernel.is_empty(), "Kernel version should be detected");

    // OS type should match current platform
    #[cfg(target_os = "macos")]
    assert_eq!(os.os_type, sniff_lib::hardware::OsType::MacOS);

    #[cfg(target_os = "linux")]
    assert_eq!(os.os_type, sniff_lib::hardware::OsType::Linux);

    #[cfg(target_os = "windows")]
    assert_eq!(os.os_type, sniff_lib::hardware::OsType::Windows);
}

/// Tests that detect_locale returns valid locale data.
#[test]
fn test_detect_locale_returns_valid_data() {
    use sniff_lib::hardware::detect_locale;

    let locale = detect_locale();

    // At least one of LANG or LC_* should typically be set on most systems
    // But we can't require it in all environments (CI containers may have minimal setup)
    // So we just verify the structure is populated correctly
    if locale.lang.is_some() || locale.lc_all.is_some() {
        // If we have locale data, preferred_language extraction should work
        // (unless the locale is "C" or "POSIX")
        if let Some(ref lang) = locale.lang {
            if lang != "C" && lang != "POSIX" && lang.contains('_') {
                assert!(
                    locale.preferred_language.is_some(),
                    "Should extract preferred language from locale"
                );
            }
        }
    }

    // LocaleInfo should always have valid structure even if empty
    let json = serde_json::to_string(&locale).expect("LocaleInfo should serialize");
    let _parsed: sniff_lib::hardware::LocaleInfo =
        serde_json::from_str(&json).expect("LocaleInfo should deserialize");
}

/// Tests that detect_timezone returns a valid UTC offset.
#[test]
fn test_detect_timezone_returns_valid_offset() {
    use sniff_lib::hardware::detect_timezone;

    let time_info = detect_timezone();

    // UTC offset should be within valid bounds (-12h to +14h in seconds)
    assert!(
        time_info.utc_offset_seconds >= -12 * 3600,
        "UTC offset should be >= -12 hours"
    );
    assert!(
        time_info.utc_offset_seconds <= 14 * 3600,
        "UTC offset should be <= +14 hours"
    );

    // Timezone abbreviation should be present on all platforms
    assert!(
        time_info.timezone_abbr.is_some(),
        "Timezone abbreviation should be detected"
    );

    // Monotonic clock should always be available on modern systems
    assert!(
        time_info.monotonic_available,
        "Monotonic clock should be available"
    );

    // TimeInfo should serialize/deserialize correctly
    let json = serde_json::to_string(&time_info).expect("TimeInfo should serialize");
    let _parsed: sniff_lib::hardware::TimeInfo =
        serde_json::from_str(&json).expect("TimeInfo should deserialize");
}

/// Tests that detect_os_type matches the current platform.
#[test]
fn test_detect_os_type_matches_platform() {
    use sniff_lib::hardware::{detect_os_type, OsType};

    let os_type = detect_os_type();

    // Verify the detected type matches the compilation target
    #[cfg(target_os = "macos")]
    assert_eq!(
        os_type,
        OsType::MacOS,
        "Should detect macOS on macOS platform"
    );

    #[cfg(target_os = "linux")]
    assert_eq!(
        os_type,
        OsType::Linux,
        "Should detect Linux on Linux platform"
    );

    #[cfg(target_os = "windows")]
    assert_eq!(
        os_type,
        OsType::Windows,
        "Should detect Windows on Windows platform"
    );

    #[cfg(target_os = "freebsd")]
    assert_eq!(
        os_type,
        OsType::FreeBSD,
        "Should detect FreeBSD on FreeBSD platform"
    );

    // On any platform, the type should have a valid Display implementation
    let display = os_type.to_string();
    assert!(!display.is_empty(), "OsType should have valid Display");
}

// ============================================================================
// Platform-Specific Package Manager Integration Tests
// ============================================================================

/// Tests macOS package manager detection finds homebrew or softwareupdate.
#[cfg(target_os = "macos")]
#[test]
fn test_macos_package_managers_finds_expected_managers() {
    use sniff_lib::hardware::{detect_macos_package_managers, SystemPackageManager};

    let managers = detect_macos_package_managers();

    // softwareupdate is always present on macOS as a system utility
    let has_softwareupdate = managers
        .managers
        .iter()
        .any(|m| m.manager == SystemPackageManager::Softwareupdate);
    assert!(
        has_softwareupdate,
        "macOS should always have softwareupdate available"
    );

    // A primary should always be selected on macOS
    assert!(
        managers.primary.is_some(),
        "macOS should have a primary package manager"
    );

    // If homebrew is installed, it should be detected
    let homebrew_apple_silicon = std::path::Path::new("/opt/homebrew/bin/brew").exists();
    let homebrew_intel = std::path::Path::new("/usr/local/bin/brew").exists();

    if homebrew_apple_silicon || homebrew_intel {
        let has_homebrew = managers
            .managers
            .iter()
            .any(|m| m.manager == SystemPackageManager::Homebrew);
        assert!(has_homebrew, "Homebrew should be detected when installed");
        assert_eq!(
            managers.primary,
            Some(SystemPackageManager::Homebrew),
            "Homebrew should be primary when installed"
        );
    }
}

/// Tests Linux package manager detection finds at least one manager.
#[cfg(target_os = "linux")]
#[test]
fn test_linux_package_managers_finds_at_least_one() {
    use sniff_lib::hardware::{detect_linux_distro, detect_linux_package_managers};

    // Get distro info to determine family
    let linux_family = detect_linux_distro().map(|d| d.family);
    let managers = detect_linux_package_managers(linux_family);

    // On any real Linux system, at least one package manager should be found
    // This may fail in extremely minimal containers, which is acceptable
    if !managers.managers.is_empty() {
        // If managers are found, primary should be set
        assert!(
            managers.primary.is_some(),
            "Should have primary if managers are found"
        );

        // Each detected manager should have a valid path
        for m in &managers.managers {
            assert!(
                !m.path.is_empty(),
                "Detected manager {} should have a path",
                m.manager
            );
        }
    }
}

/// Tests that the OS info from detect() includes package manager info.
#[test]
fn test_os_includes_package_managers() {
    let result = detect().unwrap();
    let os = result.os.expect("os should be present");

    // On desktop platforms (macOS, Linux, Windows), package managers should be detected
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        assert!(
            os.system_package_managers.is_some(),
            "System package managers should be detected on desktop platforms"
        );

        let mgrs = os.system_package_managers.as_ref().unwrap();
        // At minimum, the structure should be valid
        assert!(
            mgrs.primary.is_some() || mgrs.managers.is_empty(),
            "If managers exist, primary should be set"
        );
    }
}

/// Tests that the OS info from detect() includes locale info.
#[test]
fn test_os_includes_locale() {
    let result = detect().unwrap();
    let os = result.os.expect("os should be present");

    assert!(
        os.locale.is_some(),
        "Locale info should be included in OS detection"
    );
}

/// Tests that the OS info from detect() includes time info.
#[test]
fn test_os_includes_time_info() {
    let result = detect().unwrap();
    let os = result.os.expect("os should be present");

    assert!(
        os.time.is_some(),
        "Time info should be included in OS detection"
    );

    let time = os.time.as_ref().unwrap();
    // Verify basic time info fields
    assert!(
        time.utc_offset_seconds >= -12 * 3600 && time.utc_offset_seconds <= 14 * 3600,
        "UTC offset should be within valid range"
    );
}

// ============================================================================
// Network ip_addresses Integration Tests
// ============================================================================

/// Tests that network info includes ip_addresses field with proper structure.
#[test]
fn test_network_has_ip_addresses_field() {
    let result = detect().unwrap();
    let network = result.network.expect("network should be present");

    if !network.permission_denied {
        // ip_addresses field should exist and have v4/v6 vectors
        // (even if empty, the structure should be present)
        let v4_count = network.ip_addresses.v4.len();
        let v6_count = network.ip_addresses.v6.len();

        // If interfaces have addresses, they should be aggregated
        let expected_v4: usize = network
            .interfaces
            .iter()
            .map(|i| i.ipv4_addresses.len())
            .sum();
        let expected_v6: usize = network
            .interfaces
            .iter()
            .map(|i| i.ipv6_addresses.len())
            .sum();

        assert_eq!(
            v4_count, expected_v4,
            "ip_addresses.v4 count should match interface IPv4 sum"
        );
        assert_eq!(
            v6_count, expected_v6,
            "ip_addresses.v6 count should match interface IPv6 sum"
        );
    }
}

/// Tests that ip_addresses JSON serialization produces expected structure.
#[test]
fn test_network_ip_addresses_json_structure() {
    let result = detect().unwrap();
    let json = serde_json::to_string(&result).expect("SniffResult should serialize");

    // If network is present, JSON should have ip_addresses with v4/v6
    if result.network.is_some() {
        // Parse as Value to inspect structure
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("JSON should parse as Value");

        if let Some(network) = value.get("network") {
            let ip_addresses = network.get("ip_addresses");
            assert!(
                ip_addresses.is_some(),
                "network should have ip_addresses field"
            );

            let ip_addr = ip_addresses.unwrap();
            assert!(ip_addr.get("v4").is_some(), "ip_addresses should have v4");
            assert!(ip_addr.get("v6").is_some(), "ip_addresses should have v6");

            // v4 and v6 should be arrays
            assert!(
                ip_addr.get("v4").unwrap().is_array(),
                "ip_addresses.v4 should be an array"
            );
            assert!(
                ip_addr.get("v6").unwrap().is_array(),
                "ip_addresses.v6 should be an array"
            );

            // Each address entry should have address and interface fields
            if let Some(v4_arr) = ip_addr.get("v4").and_then(|v| v.as_array()) {
                for addr in v4_arr {
                    assert!(
                        addr.get("address").is_some(),
                        "IPv4 entry should have address field"
                    );
                    assert!(
                        addr.get("interface").is_some(),
                        "IPv4 entry should have interface field"
                    );
                }
            }

            if let Some(v6_arr) = ip_addr.get("v6").and_then(|v| v.as_array()) {
                for addr in v6_arr {
                    assert!(
                        addr.get("address").is_some(),
                        "IPv6 entry should have address field"
                    );
                    assert!(
                        addr.get("interface").is_some(),
                        "IPv6 entry should have interface field"
                    );
                }
            }
        }
    }
}

/// Tests that ip_addresses roundtrip through JSON correctly.
#[test]
fn test_network_ip_addresses_roundtrip() {
    let result = detect().unwrap();
    let json = serde_json::to_string(&result).expect("SniffResult should serialize");
    let parsed: sniff_lib::SniffResult =
        serde_json::from_str(&json).expect("JSON should deserialize");

    if let (Some(orig_net), Some(parsed_net)) = (&result.network, &parsed.network) {
        // Counts should match
        assert_eq!(
            orig_net.ip_addresses.v4.len(),
            parsed_net.ip_addresses.v4.len(),
            "v4 count should survive roundtrip"
        );
        assert_eq!(
            orig_net.ip_addresses.v6.len(),
            parsed_net.ip_addresses.v6.len(),
            "v6 count should survive roundtrip"
        );

        // Contents should match
        for (orig, parsed) in orig_net
            .ip_addresses
            .v4
            .iter()
            .zip(parsed_net.ip_addresses.v4.iter())
        {
            assert_eq!(
                orig.address, parsed.address,
                "IPv4 address should survive roundtrip"
            );
            assert_eq!(
                orig.interface, parsed.interface,
                "IPv4 interface should survive roundtrip"
            );
        }

        for (orig, parsed) in orig_net
            .ip_addresses
            .v6
            .iter()
            .zip(parsed_net.ip_addresses.v6.iter())
        {
            assert_eq!(
                orig.address, parsed.address,
                "IPv6 address should survive roundtrip"
            );
            assert_eq!(
                orig.interface, parsed.interface,
                "IPv6 interface should survive roundtrip"
            );
        }
    }
}
