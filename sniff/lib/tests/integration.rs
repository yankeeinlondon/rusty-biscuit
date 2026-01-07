use sniff_lib::{detect, detect_with_config, SniffConfig};
use std::path::PathBuf;
use std::time::Instant;

mod fixtures;

#[test]
fn test_detect_returns_hardware_info() {
    let result = detect().unwrap();
    let hardware = result.hardware.expect("hardware should be present");
    assert!(!hardware.os.name.is_empty());
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
    assert!(fs.monorepo.is_some());
    let mono = fs.monorepo.unwrap();
    assert_eq!(mono.packages.len(), 2);
}

#[test]
fn test_detect_completes_in_reasonable_time() {
    // NFR-1: Fast path detection should complete in <300ms
    let start = Instant::now();
    let _ = detect();
    let elapsed = start.elapsed();
    // Allow some slack for CI environments
    assert!(elapsed.as_millis() < 5000, "Detection took too long: {:?}", elapsed);
}

#[test]
fn test_serialization_roundtrip() {
    let result = detect().unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff_lib::SniffResult = serde_json::from_str(&json).unwrap();
    let orig_hw = result.hardware.expect("hardware should be present");
    let parsed_hw = parsed.hardware.expect("parsed hardware should be present");
    assert_eq!(orig_hw.os.name, parsed_hw.os.name);
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
    assert!(fs.monorepo.is_some());
    let mono = fs.monorepo.unwrap();
    assert_eq!(mono.tool, sniff_lib::filesystem::MonorepoTool::PnpmWorkspaces);
}

// === Regression tests for JSON serialization of partial results ===
// Bug: Skipped sections were serialized as empty objects instead of being omitted.

#[test]
fn test_skip_hardware_json_omits_hardware_key() {
    // Regression test: JSON should NOT contain "hardware" key when skipped
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("\"hardware\""), "JSON should not contain hardware key when skipped");
    assert!(json.contains("\"network\""), "JSON should contain network key");
}

#[test]
fn test_skip_network_json_omits_network_key() {
    // Regression test: JSON should NOT contain "network" key when skipped
    let config = SniffConfig::new().skip_network();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("\"network\""), "JSON should not contain network key when skipped");
    assert!(json.contains("\"hardware\""), "JSON should contain hardware key");
}

#[test]
fn test_skip_filesystem_json_omits_filesystem_key() {
    // Regression test: JSON should NOT contain "filesystem" key when skipped
    let config = SniffConfig::new().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("\"filesystem\""), "JSON should not contain filesystem key when skipped");
    assert!(json.contains("\"hardware\""), "JSON should contain hardware key");
}

#[test]
fn test_hardware_only_json_contains_only_hardware() {
    // Regression test: When only hardware is requested, JSON should contain ONLY hardware
    let config = SniffConfig::new()
        .skip_network()
        .skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"hardware\""), "JSON should contain hardware key");
    assert!(!json.contains("\"network\""), "JSON should not contain network key");
    assert!(!json.contains("\"filesystem\""), "JSON should not contain filesystem key");
    assert!(!json.contains("\"interfaces\""), "JSON should not contain interfaces (from network)");
}

#[test]
fn test_partial_result_deserialization_roundtrip() {
    // Regression test: Partial results should deserialize correctly
    let config = SniffConfig::new().skip_hardware();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff_lib::SniffResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.hardware.is_none(), "Deserialized hardware should be None");
    assert!(parsed.network.is_some(), "Deserialized network should be Some");
}
