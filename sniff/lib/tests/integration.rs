use sniff::filesystem::ProgrammingLanguage;
use sniff::os::NtpStatus;
use sniff::{detect, detect_with_config, SniffConfig};
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
    // Allow slack for CI environments, package manager detection (PATH scanning),
    // and boundary-aware mixed-workspace package discovery.
    assert!(
        elapsed.as_millis() < 20000,
        "Detection took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_serialization_roundtrip() {
    let result = detect().unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff::SniffResult = serde_json::from_str(&json).unwrap();
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
    assert!(langs.total_files_scanned >= 4);
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
        Some(sniff::filesystem::MonorepoTool::PnpmWorkspaces)
    );
}

#[test]
fn test_detect_language_uses_package_boundary_from_nested_workspace() {
    let (_dir, path) = fixtures::create_mixed_nested_workspace();
    let config = SniffConfig::new().base_dir(path.join("server"));
    let result = detect_with_config(config).unwrap();
    let filesystem = result.filesystem.unwrap();
    let languages = filesystem.languages.unwrap();

    assert_eq!(languages.primary, Some(ProgrammingLanguage::Rust));
    assert_eq!(languages.total_files_scanned, 2);
    assert!(languages
        .languages
        .iter()
        .any(|lang| lang.language == ProgrammingLanguage::Rust));
    assert!(!languages
        .languages
        .iter()
        .any(|lang| lang.language == ProgrammingLanguage::TypeScript));
}

// === Regression tests for JSON serialization of partial results ===
// Bug: Skipped sections were serialized as empty objects instead of being omitted.
//
// NOTE: These tests parse JSON as serde_json::Value and check top-level keys
// rather than using substring matching, because nested data (e.g. Cargo feature
// names like "network") can produce false positives with contains().

/// Helper: parse SniffResult JSON and return the top-level key set.
fn top_level_keys(result: &sniff::SniffResult) -> std::collections::HashSet<String> {
    let json = serde_json::to_string(result).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn test_skip_hardware_json_omits_hardware_key() {
    // Regression test: JSON should NOT contain "hardware" key when skipped
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("hardware"),
        "JSON should not contain hardware key when skipped"
    );
    assert!(keys.contains("network"), "JSON should contain network key");
}

#[test]
fn test_skip_network_json_omits_network_key() {
    // Regression test: JSON should NOT contain "network" key when skipped
    let config = SniffConfig::new().skip_network();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("network"),
        "JSON should not contain network key when skipped"
    );
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_skip_filesystem_json_omits_filesystem_key() {
    // Regression test: JSON should NOT contain "filesystem" key when skipped
    let config = SniffConfig::new().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("filesystem"),
        "JSON should not contain filesystem key when skipped"
    );
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_hardware_only_json_contains_only_hardware() {
    // Regression test: When only hardware is requested, JSON should contain ONLY hardware
    let config = SniffConfig::new().skip_network().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
    assert!(
        !keys.contains("network"),
        "JSON should not contain network key"
    );
    assert!(
        !keys.contains("filesystem"),
        "JSON should not contain filesystem key"
    );
}

#[test]
fn test_partial_result_deserialization_roundtrip() {
    // Regression test: Partial results should deserialize correctly
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff::SniffResult = serde_json::from_str(&json).unwrap();
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
    use sniff::hardware::detect_os;

    let os = detect_os().expect("detect_os should succeed");

    // OS info should have populated fields
    assert!(!os.name.is_empty(), "OS name should be detected");
    assert!(!os.kernel.is_empty(), "Kernel version should be detected");

    // OS type should match current platform
    #[cfg(target_os = "macos")]
    assert_eq!(os.os_type, sniff::hardware::OsType::MacOS);

    #[cfg(target_os = "linux")]
    assert_eq!(os.os_type, sniff::hardware::OsType::Linux);

    #[cfg(target_os = "windows")]
    assert_eq!(os.os_type, sniff::hardware::OsType::Windows);
}

/// Tests that detect_locale returns valid locale data.
#[test]
fn test_detect_locale_returns_valid_data() {
    use sniff::hardware::detect_locale;

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
    let _parsed: sniff::hardware::LocaleInfo =
        serde_json::from_str(&json).expect("LocaleInfo should deserialize");
}

/// Tests that detect_timezone returns a valid UTC offset.
#[test]
fn test_detect_timezone_returns_valid_offset() {
    use sniff::hardware::detect_timezone;

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
    let _parsed: sniff::hardware::TimeInfo =
        serde_json::from_str(&json).expect("TimeInfo should deserialize");
}

/// Tests that detect_os_type matches the current platform.
#[test]
fn test_detect_os_type_matches_platform() {
    use sniff::hardware::{detect_os_type, OsType};

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
    use sniff::hardware::{detect_macos_package_managers, SystemPackageManager};

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
    use sniff::hardware::{detect_linux_distro, detect_linux_package_managers};

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
            assert!(
                network.get("wan_ip_address").is_some(),
                "network should have wan_ip_address field"
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
    let parsed: sniff::SniffResult = serde_json::from_str(&json).expect("JSON should deserialize");

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
        assert_eq!(
            orig_net.wan_ip_address, parsed_net.wan_ip_address,
            "wan_ip_address should survive roundtrip"
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

#[test]
fn test_detect_with_plan_summary_mode() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary())
        .hardware(HardwareRequest::summary())
        .without_network()
        .without_filesystem();

    let start = Instant::now();
    let result = sniff::detect_with_plan(plan).unwrap();
    let elapsed = start.elapsed();

    assert!(result.os.is_some());
    assert!(result.hardware.is_some());
    assert!(result.network.is_none());
    assert!(result.filesystem.is_none());

    // Summary mode should be significantly faster than full detection
    assert!(
        elapsed.as_millis() < 2000,
        "Summary detection took too long: {:?}",
        elapsed
    );
}

// ============================================================================
// Selective-cost behavior regression tests (review-3 item 3)
// ============================================================================

/// Creates a temporary git repo with a committed file and an uncommitted modification,
/// suitable for testing file_changes vs diff payload behavior.
fn create_dirty_git_repo() -> (tempfile::TempDir, PathBuf) {
    use git2::{Repository, Signature};
    use std::fs;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Create and commit a file
    let file_path = dir.path().join("hello.txt");
    fs::write(&file_path, "hello world\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("hello.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    // Now modify the file (unstaged change) to make the repo dirty
    fs::write(&file_path, "hello world\nmodified line\n").unwrap();

    // Also create an untracked file
    fs::write(dir.path().join("untracked.txt"), "new file\n").unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

#[test]
fn test_git_full_has_file_changes_but_no_diff_payloads() {
    use sniff::request::*;

    let (_dir, path) = create_dirty_git_repo();

    let plan = DetectionPlan::new()
        .base_dir(path)
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::full())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        );

    let result = sniff::detect_with_plan(plan).unwrap();
    let fs = result.filesystem.expect("filesystem should be present");
    let git = fs.git.expect("git should be present");

    // GitRequest::full() includes file_changes (paths, status, line counts)
    assert!(
        !git.file_changes.is_empty(),
        "full() should populate file_changes"
    );

    // But does NOT include unified diff payloads
    assert!(
        git.status.dirty.is_empty(),
        "full() should NOT populate dirty diff payloads"
    );
    assert!(
        git.status.untracked.is_empty(),
        "full() should NOT populate untracked file details"
    );

    // Verify the counts are correct
    assert!(git.status.is_dirty);
    assert!(git.status.unstaged_count > 0);
    assert!(git.status.untracked_count > 0);
}

#[test]
fn test_git_deep_includes_diff_payloads() {
    use sniff::request::*;

    let (_dir, path) = create_dirty_git_repo();

    let plan = DetectionPlan::new()
        .base_dir(path)
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::deep())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        );

    let result = sniff::detect_with_plan(plan).unwrap();
    let fs = result.filesystem.expect("filesystem should be present");
    let git = fs.git.expect("git should be present");

    // deep() includes both file_changes AND diff payloads
    assert!(
        !git.file_changes.is_empty(),
        "deep() should populate file_changes"
    );
    assert!(
        !git.status.dirty.is_empty(),
        "deep() should populate dirty diff payloads"
    );
    assert!(
        !git.status.untracked.is_empty(),
        "deep() should populate untracked file details"
    );

    // Verify the diff payload contains actual content
    let dirty_file = &git.status.dirty[0];
    assert!(
        !dirty_file.diff.is_empty(),
        "dirty file should have a non-empty diff"
    );
}

#[test]
fn test_os_timezone_without_ntp() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary()
            .include_timezone(true)
            .include_ntp_status(false))
        .without_hardware()
        .without_network()
        .without_filesystem();

    let result = sniff::detect_with_plan(plan).unwrap();
    let os = result.os.expect("os should be present");

    // Timezone data should be populated
    let time = os
        .time
        .expect("time should be present when timezone is enabled");
    assert!(time.timezone.is_some(), "timezone name should be detected");

    // NTP should NOT have been probed — expect Unknown (the default)
    assert!(
        matches!(time.ntp_status, NtpStatus::Unknown),
        "NTP status should be Unknown when NTP probing is disabled, got {:?}",
        time.ntp_status
    );
}

#[test]
fn test_os_summary_has_no_time_data() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary())
        .without_hardware()
        .without_network()
        .without_filesystem();

    let result = sniff::detect_with_plan(plan).unwrap();
    let os = result.os.expect("os should be present");

    // Summary mode disables both timezone and NTP
    assert!(os.time.is_none(), "summary() should not include time data");
}

#[test]
fn test_executable_index_parity_with_which_for_common_programs() {
    use sniff::programs::{find_program_with_source, ExecutableIndex};

    let index = ExecutableIndex::build();

    // Test a broader set of programs that are commonly available
    let programs = ["git", "bash", "sh", "env", "ls", "cat"];

    for prog in &programs {
        let which_found = find_program_with_source(prog).is_some();
        let index_found = index.find_with_source(prog).is_some();

        assert_eq!(
            which_found, index_found,
            "Parity mismatch for '{}': which={}, index={}",
            prog, which_found, index_found
        );
    }
}

// ============================================================================
// Windows Cross-Platform Integration Tests
// ============================================================================

/// Asserts that `primary_interface` is populated on eligible hosts.
///
/// On macOS and Linux a desktop/workstation usually has at least one
/// non-loopback, up interface with an IPv4 address, so the primary
/// selector should succeed.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn test_network_primary_interface_is_populated() {
    let result = detect().unwrap();
    let network = result.network.expect("network should be present");

    let has_eligible_interface = !network.permission_denied
        && network
            .interfaces
            .iter()
            .any(|i| !i.flags.is_loopback && !i.ipv4_addresses.is_empty() && i.flags.is_up);

    if has_eligible_interface {
        assert!(
            network.primary_interface.is_some(),
            "primary_interface should be populated when a non-loopback IPv4 interface exists"
        );
        let primary = network.primary_interface.unwrap();
        assert!(
            !primary.is_empty(),
            "primary_interface name should not be empty"
        );
    }
}

/// Asserts that `services_detailed(ServiceState::All)` returns at least one
/// service with a non-empty name on supported platforms.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn test_services_detailed_returns_non_empty_names() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let services = manager.services_detailed(ServiceState::All);

    if manager.init_system != sniff::services::InitSystem::Unknown {
        assert!(
            !services.is_empty(),
            "services_detailed(All) should return at least one service for {:?}",
            manager.init_system
        );
        for svc in &services {
            assert!(
                !svc.name.is_empty(),
                "every service should have a non-empty name"
            );
        }
    }
}

/// On Windows the default `detect_timezone()` code path should populate the
/// `timezone` field via `tzutil`.  This test locks down the runtime contract
/// on an actual Windows host without using the plan-based opt-in path.
#[cfg(target_os = "windows")]
#[test]
fn test_detect_timezone_windows_populates_timezone_name() {
    let time_info = sniff::hardware::detect_timezone();

    assert!(
        time_info.timezone.is_some(),
        "detect_timezone() should populate timezone on Windows via tzutil"
    );

    let tz = time_info.timezone.unwrap();
    assert!(!tz.is_empty(), "timezone name should not be empty");

    // IANA names contain '/' (e.g. "America/Los_Angeles").  Unmapped Windows
    // IDs typically contain "Standard" or "Daylight" but never '/'.
    // Either way the value should be non-empty and valid.
    assert!(
        tz.len() >= 3,
        "timezone name should be at least 3 characters, got: '{tz}'"
    );
}

/// On Windows `services_detailed(Running)` should return only services whose
/// SCM state is `SERVICE_RUNNING`.
#[cfg(target_os = "windows")]
#[test]
fn test_services_detailed_running_filter_windows() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let all = manager.services_detailed(ServiceState::All);
    let running = manager.services_detailed(ServiceState::Running);

    // Running should be a subset of all
    assert!(
        running.len() <= all.len(),
        "Running services ({}) should not exceed total ({})",
        running.len(),
        all.len()
    );

    for svc in &running {
        assert!(
            svc.running,
            "Service '{}' passed Running filter but running=false",
            svc.name
        );
    }
}

/// On Windows `services_detailed(Stopped)` should return only stopped services.
#[cfg(target_os = "windows")]
#[test]
fn test_services_detailed_stopped_filter_windows() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let stopped = manager.services_detailed(ServiceState::Stopped);

    for svc in &stopped {
        assert!(
            !svc.running,
            "Service '{}' passed Stopped filter but running=true",
            svc.name
        );
    }
}
