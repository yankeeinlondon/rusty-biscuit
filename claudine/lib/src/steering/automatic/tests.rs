//! The automatic-help setting, its precedence, and the opportunity budget.
//!
//! The configuration matrix reads real config files through the loaders the
//! wrapper uses, one edit per cell from a control file that opts out.

use std::ffi::OsStr;
use std::path::Path;

use super::*;
use crate::dispatch::loader;

fn user(enabled: Option<bool>) -> SteeringConfig {
    SteeringConfig { automatic: AutomaticSteeringConfig { enabled } }
}

#[test]
fn every_documented_env_spelling_parses_trimmed_and_case_insensitive() {
    for on in ["true", "1", "yes", "on", " TRUE ", "Yes", "\tOn\n"] {
        assert!(parse_auto_steer(OsStr::new(on)).unwrap(), "{on:?}");
    }
    for off in ["false", "0", "no", "off", " FALSE ", "No", " oFF "] {
        assert!(!parse_auto_steer(OsStr::new(off)).unwrap(), "{off:?}");
    }
}

#[test]
fn empty_and_malformed_env_values_are_configuration_errors() {
    for bad in ["", "   ", "maybe", "2", "enabled", "t", "y", "true false"] {
        let error = parse_auto_steer(OsStr::new(bad)).expect_err(bad);
        let text = error.to_string();
        assert!(matches!(error, ClaudineError::ConfigValidation(_)), "{bad:?}: {text}");
        assert!(text.contains(AUTO_STEER_ENV), "{bad:?}: {text}");
    }
    assert!(parse_auto_steer(OsStr::new("")).unwrap_err().to_string().contains("is empty"));
    assert!(parse_auto_steer(OsStr::new(" maybe ")).unwrap_err().to_string().contains("is `maybe`"));
}

#[cfg(unix)]
#[test]
fn a_non_unicode_env_value_is_a_configuration_error() {
    use std::os::unix::ffi::OsStrExt;
    let error = parse_auto_steer(OsStr::from_bytes(&[0x74, 0xff])).unwrap_err();
    assert!(error.to_string().contains("not valid Unicode"));
}

#[test]
fn precedence_is_env_then_repo_then_user_then_on() {
    let off = OsStr::new("off");
    let on = OsStr::new("on");
    assert!(resolve_enabled(None, None, &user(None)).unwrap(), "built-in default is on");
    assert!(!resolve_enabled(None, None, &user(Some(false))).unwrap());
    assert!(resolve_enabled(None, Some(&user(Some(true))), &user(Some(false))).unwrap(), "repo overrides user");
    assert!(!resolve_enabled(None, Some(&user(Some(false))), &user(Some(true))).unwrap());
    assert!(!resolve_enabled(None, Some(&user(None)), &user(Some(false))).unwrap(), "absent repo value keeps a user opt-out");
    assert!(resolve_enabled(Some(on), Some(&user(Some(false))), &user(Some(false))).unwrap(), "env overrides both");
    assert!(!resolve_enabled(Some(off), Some(&user(Some(true))), &user(Some(true))).unwrap());
    assert!(resolve_enabled(Some(OsStr::new("")), None, &user(None)).is_err(), "a present but empty env value never inherits");
}

#[test]
fn three_opportunities_per_execution_each_distinct() {
    let mut budget = OpportunityBudget::default();
    let ids: Vec<_> = (0..3).map(|_| budget.claim().expect("within the cap")).collect();
    assert_eq!(budget.used(), 3);
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    assert!(budget.claim().is_none());
    assert!(budget.claim().is_none(), "exhaustion is permanent for the execution");
    assert_eq!(budget.used(), 3);
    assert!(OpportunityBudget::default().claim().is_some(), "a new execution has its own allowance");
}

#[test]
fn the_helper_message_is_the_specified_suspicion_and_fits_a_steering_message() {
    assert!(HELPER_MESSAGE.starts_with("Claudine has detected repeated output that may indicate a loop."));
    assert!(HELPER_MESSAGE.contains("change your approach or stop and explain what is preventing progress"));
    assert!(HELPER_MESSAGE.ends_with("Claudine's existing runaway limits still apply."));
    assert!(crate::steering::contract::SteeringMessage::new(HELPER_MESSAGE).is_ok());
}

// ---------------------------------------------------------------------------
// Input robustness matrix: `steering.automatic.enabled`, user and repo files
// ---------------------------------------------------------------------------

const CONTROL: &str = r#"{ "steering": { "automatic": { "enabled": false } } }"#;

/// `Ok(value)` is the field the loader reads, `Err(())` a load error.
type Expected = std::result::Result<Option<bool>, ()>;

/// `(cell, file text, expected)`.
fn cells() -> Vec<(&'static str, &'static str, Expected)> {
    vec![
        ("control", CONTROL, Ok(Some(false))),
        ("control true", r#"{ "steering": { "automatic": { "enabled": true } } }"#, Ok(Some(true))),
        ("absent key", r#"{ "steering": { "automatic": {} } }"#, Ok(None)),
        ("absent automatic", r#"{ "steering": {} }"#, Ok(None)),
        ("absent steering", r#"{}"#, Ok(None)),
        ("null value", r#"{ "steering": { "automatic": { "enabled": null } } }"#, Err(())),
        ("null automatic", r#"{ "steering": { "automatic": null } }"#, Err(())),
        ("null steering", r#"{ "steering": null }"#, Err(())),
        ("string value", r#"{ "steering": { "automatic": { "enabled": "false" } } }"#, Err(())),
        ("number value", r#"{ "steering": { "automatic": { "enabled": 0 } } }"#, Err(())),
        ("array value", r#"{ "steering": { "automatic": { "enabled": [false] } } }"#, Err(())),
        ("automatic not an object", r#"{ "steering": { "automatic": false } }"#, Err(())),
        ("steering not an object", r#"{ "steering": false }"#, Err(())),
        ("unknown key beside", r#"{ "steering": { "automatic": { "enabled": false, "enable": true } } }"#, Err(())),
        ("unknown steering key", r#"{ "steering": { "automatic": { "enabled": false }, "auto": {} } }"#, Err(())),
        ("trailing content", r#"{ "steering": { "automatic": { "enabled": false } } } false"#, Err(())),
        // Known gap, loader-wide: config files are parsed as JSON5 into a
        // `serde_json::Value` first, which keeps the last duplicate, so no
        // Claudine config key can reject one. Pinned here so a stricter
        // parser shows up as a deliberate change.
        ("duplicate key", r#"{ "steering": { "automatic": { "enabled": false, "enabled": true } } }"#, Ok(Some(true))),
    ]
}

fn write(dir: &Path, text: &str) -> std::path::PathBuf {
    let path = dir.join("config.json");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn repo_file_matrix() {
    for (cell, text, expected) in cells() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = loader::load_repo_override_config(&write(tmp.path(), text));
        let actual = loaded.map(|config| config.expect("file exists").steering.automatic.enabled).map_err(|_| ());
        assert_eq!(actual, expected, "repo cell `{cell}`");
    }
}

#[test]
fn user_file_matrix() {
    for (cell, text, expected) in cells() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = loader::load_claudine_config(Some(&write(tmp.path(), text)), None);
        let actual = loaded.map(|config| config.steering.automatic.enabled).map_err(|_| ());
        assert_eq!(actual, expected, "user cell `{cell}`");
    }
}

#[test]
fn a_repo_file_merged_by_the_user_loader_overrides_only_what_it_sets() {
    let tmp = tempfile::tempdir().unwrap();
    let user_path = write(tmp.path(), CONTROL);
    let repo_root = tmp.path().join("repo");
    std::fs::create_dir_all(repo_root.join(".claudine")).unwrap();

    std::fs::write(repo_root.join(".claudine/config.json"), r#"{ "canonical_provider": "claude" }"#).unwrap();
    let merged = loader::load_claudine_config(Some(&user_path), Some(&repo_root)).unwrap();
    assert_eq!(merged.steering.automatic.enabled, Some(false), "an absent repo value keeps the user opt-out");

    std::fs::write(repo_root.join(".claudine/config.json"), r#"{ "steering": { "automatic": { "enabled": true } } }"#).unwrap();
    let merged = loader::load_claudine_config(Some(&user_path), Some(&repo_root)).unwrap();
    assert_eq!(merged.steering.automatic.enabled, Some(true));
}

#[test]
fn a_saved_config_round_trips_the_setting_and_omits_it_when_unset() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    let mut config = crate::config::claudine_config::ClaudineConfig::default();
    loader::save_claudine_config(&config, &path).unwrap();
    assert!(!std::fs::read_to_string(&path).unwrap().contains("steering"));

    config.steering.automatic.enabled = Some(false);
    for _ in 0..2 {
        loader::save_claudine_config(&config, &path).unwrap();
        config = loader::load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(config.steering.automatic.enabled, Some(false));
    }
}
