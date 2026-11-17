//! The automatic-help setting, its precedence, and the opportunity budget.
//!
//! The configuration matrix reads real config files through the loaders the
//! wrapper uses, one edit per cell from a control file that opts out.

use std::ffi::OsStr;
use std::path::Path;

use super::*;
use crate::config::persist;
use crate::config::schema::ConfigScope;
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

/// `(cell, file text, expected in a user file, expected in a repo file)`.
///
/// A user file's unset `enabled` (absent or `null`, at any level) takes the
/// schema default `true`; a repo file's stays unset so the user's value
/// decides. A repo's `steering` and `automatic` overrides are `not-empty`.
fn cells() -> Vec<(&'static str, &'static str, Expected, Expected)> {
    let both = |expected: Expected| (expected, expected);
    let rows: Vec<(&'static str, &'static str, (Expected, Expected))> = vec![
        ("control", CONTROL, both(Ok(Some(false)))),
        ("control true", r#"{ "steering": { "automatic": { "enabled": true } } }"#, both(Ok(Some(true)))),
        ("empty automatic", r#"{ "steering": { "automatic": {} } }"#, (Ok(Some(true)), Err(()))),
        ("absent automatic", r#"{ "steering": {} }"#, (Ok(Some(true)), Err(()))),
        ("absent steering", r#"{}"#, (Ok(Some(true)), Ok(None))),
        ("null value", r#"{ "steering": { "automatic": { "enabled": null } } }"#, (Ok(Some(true)), Ok(None))),
        ("null automatic", r#"{ "steering": { "automatic": null } }"#, (Ok(Some(true)), Ok(None))),
        ("null steering", r#"{ "steering": null }"#, (Ok(Some(true)), Ok(None))),
        ("string value", r#"{ "steering": { "automatic": { "enabled": "false" } } }"#, both(Err(()))),
        ("number value", r#"{ "steering": { "automatic": { "enabled": 0 } } }"#, both(Err(()))),
        ("array value", r#"{ "steering": { "automatic": { "enabled": [false] } } }"#, both(Err(()))),
        ("automatic not an object", r#"{ "steering": { "automatic": false } }"#, both(Err(()))),
        ("steering not an object", r#"{ "steering": false }"#, both(Err(()))),
        ("unknown key beside", r#"{ "steering": { "automatic": { "enabled": false, "enable": true } } }"#, both(Err(()))),
        ("unknown steering key", r#"{ "steering": { "automatic": { "enabled": false }, "auto": {} } }"#, both(Err(()))),
        ("trailing content", r#"{ "steering": { "automatic": { "enabled": false } } } false"#, both(Err(()))),
        ("duplicate key", r#"{ "steering": { "automatic": { "enabled": false, "enabled": true } } }"#, both(Err(()))),
    ];
    rows.into_iter().map(|(cell, text, (user, repo))| (cell, text, user, repo)).collect()
}

fn write(dir: &Path, text: &str) -> std::path::PathBuf {
    let path = dir.join("config.yaml");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn repo_file_matrix() {
    for (cell, text, _, expected) in cells() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = loader::load_repo_override_config(&write(tmp.path(), text));
        let actual = loaded.map(|config| config.expect("file exists").steering.automatic.enabled).map_err(|_| ());
        assert_eq!(actual, expected, "repo cell `{cell}`");
    }
}

#[test]
fn user_file_matrix() {
    for (cell, text, expected, _) in cells() {
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

    std::fs::write(repo_root.join(".claudine/config.yaml"), r#"{ "canonical_provider": "claude" }"#).unwrap();
    let merged = loader::load_claudine_config(Some(&user_path), Some(&repo_root)).unwrap();
    assert_eq!(merged.steering.automatic.enabled, Some(false), "an absent repo value keeps the user opt-out");

    std::fs::write(repo_root.join(".claudine/config.yaml"), r#"{ "steering": { "automatic": { "enabled": true } } }"#).unwrap();
    let merged = loader::load_claudine_config(Some(&user_path), Some(&repo_root)).unwrap();
    assert_eq!(merged.steering.automatic.enabled, Some(true));
}

#[test]
fn a_saved_config_round_trips_the_setting_and_starts_at_its_default() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.yaml");
    let defaults = crate::config::claudine_config::ClaudineConfig::default();
    persist::initialize_config(&path, ConfigScope::User, &defaults, &defaults.clone()).unwrap();
    let created: serde_json::Value =
        biscuit_file::serde_yaml_ng::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(created.pointer("/steering/automatic/enabled"), Some(&serde_json::json!(true)), "{created}");

    for _ in 0..2 {
        let opened = loader::load_claudine_config(Some(&path), None).unwrap();
        let mut edited = opened.clone();
        edited.steering.automatic.enabled = Some(false);
        persist::save_changes(&path, ConfigScope::User, &opened, &edited).unwrap();
        let config = loader::load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(config.steering.automatic.enabled, Some(false));
    }
}
