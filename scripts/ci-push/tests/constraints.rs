//! The constraint store, matching, and expiry, at the module's public API.
//!
//! `parity.json` is shared with `scripts/ci/test_constraints.py`: both
//! readers run every case and must reach the same outcome.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde_json::Value;
use tempfile::TempDir;

use crate::ci_push::ConstraintCell;
use crate::ci_push::constraints::{
    self, Constraint, ENVIRONMENT_VARIABLE, default_directory, load, read_plan, repository_identity,
    resolve_directory, select, store_root, store_segments, unsatisfied,
};

const MINIMAL: &str = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("test date")
}

fn minimal_with(field: &str, value: Value) -> String {
    let mut record: Value = serde_json::from_str(MINIMAL).expect("control is JSON");
    record[field] = value;
    record.to_string()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parent");
    fs::write(path, text).expect("write fixture file");
}

/// `path` below `root`, `/`-separated, as the parity fixture names it.
fn relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).expect("path is inside the store");
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| entry.as_str().expect("string").to_owned())
        .collect()
}

// --- Shared parity cases ---

#[test]
fn every_identity_case_in_the_parity_fixture_agrees() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../ci/fixtures/pre-push/constraints/parity.json")).expect("fixture");
    for case in fixture["identities"].as_array().expect("identities") {
        let remote = case["remote"].as_str().expect("remote");
        let identity = repository_identity(remote);
        assert_eq!(identity, case["identity"].as_str().expect("identity"), "remote {remote:?}");
        assert_eq!(store_segments(&identity), strings(&case["segments"]), "remote {remote:?}");
    }
}

#[test]
fn every_store_case_in_the_parity_fixture_agrees() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../ci/fixtures/pre-push/constraints/parity.json")).expect("fixture");
    for case in fixture["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let temporary = TempDir::new().expect("temporary directory");
        let store = temporary.path().join("store");
        if let Some(files) = case["store"].as_object() {
            fs::create_dir(&store).expect("store directory");
            for (path, content) in files {
                let text = match content {
                    Value::String(raw) => raw.clone(),
                    other => other.to_string(),
                };
                write(&store.join(path), &text);
            }
        }
        let branches = strings(&case["branches"]);
        let today = date(case["today"].as_str().expect("today"));
        let selection = select(
            load(&store).unwrap_or_else(|error| panic!("{name}: {error}")),
            case["repository"].as_str().expect("repository"),
            &branches,
            today,
        );
        let expected = &case["expected"];
        let paths = |constraints: &[Constraint]| -> Vec<String> {
            constraints.iter().map(|constraint| relative(&store, &constraint.path)).collect()
        };
        assert_eq!(paths(&selection.active), strings(&expected["active"]), "{name}: active");
        assert_eq!(paths(&selection.expired), strings(&expected["expired"]), "{name}: expired");
        let malformed: Vec<(String, String)> = selection
            .malformed
            .iter()
            .map(|constraint| {
                let error = constraint.record.as_ref().expect_err("malformed");
                (relative(&store, &constraint.path), error.location.clone())
            })
            .collect();
        let expected_malformed: Vec<(String, String)> = expected["malformed"]
            .as_array()
            .expect("malformed")
            .iter()
            .map(|entry| {
                let text = |field: &str| entry[field].as_str().expect("string").to_owned();
                (text("path"), text("location"))
            })
            .collect();
        assert_eq!(malformed, expected_malformed, "{name}: malformed");

        let cells = match case.get("plan").and_then(Value::as_str) {
            None => None,
            Some(text) => {
                let plan = temporary.path().join("plan.json");
                write(&plan, text);
                match (read_plan(&plan), expected.get("plan_error")) {
                    (Err(error), Some(location)) => {
                        assert_eq!(error.location, location.as_str().expect("location"), "{name}: {error}");
                        continue;
                    }
                    (Err(error), None) => panic!("{name}: unexpected plan error {error}"),
                    (Ok(_), Some(_)) => panic!("{name}: the plan was expected to be refused"),
                    (Ok(cells), None) => Some(cells),
                }
            }
        };
        assert_eq!(
            paths(&unsatisfied(&selection.active, cells.as_deref()).into_iter().cloned().collect::<Vec<_>>()),
            strings(&expected["unsatisfied"]),
            "{name}: unsatisfied"
        );
    }
}

// --- Expiry ---

#[test]
fn a_record_expires_only_after_its_expiry_day() {
    let today = date("2026-11-10");
    for (expiry, expired) in [("2026-11-09", true), ("2026-11-10", false), ("2026-11-11", false)] {
        let text = minimal_with("expiry", Value::from(expiry));
        let constraint = Constraint {
            path: PathBuf::from("record.json"),
            record: crate::ci_push::parse_constraint_record(text.as_bytes(), "record.json"),
        };
        assert_eq!(constraint.expired(today), expired, "expiry {expiry}");
    }
}

#[test]
fn a_malformed_record_never_expires_and_applies_everywhere() {
    let text = minimal_with("branch", Value::Null);
    let constraint = Constraint {
        path: PathBuf::from("record.json"),
        record: crate::ci_push::parse_constraint_record(text.as_bytes(), "record.json"),
    };
    assert!(constraint.is_malformed());
    assert!(!constraint.expired(date("2200-01-01")));
    assert!(constraint.applies_to("github.com/someone-else/repo", &["another-branch"]));
}

// --- The store walk ---

#[test]
fn a_missing_store_holds_no_records() {
    let temporary = TempDir::new().expect("temporary directory");
    assert_eq!(load(&temporary.path().join("absent")).expect("missing store").len(), 0);
    assert_eq!(load(Path::new("")).expect("empty path").len(), 0);
}

#[test]
fn a_store_path_that_is_a_file_holds_no_records() {
    let temporary = TempDir::new().expect("temporary directory");
    let file = temporary.path().join("store.json");
    write(&file, MINIMAL);
    assert_eq!(load(&file).expect("file store").len(), 0);
    assert_eq!(load(&file.join("below")).expect("below a file").len(), 0);
}

#[cfg(unix)]
fn without_permissions(path: &Path) -> impl Drop {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(PathBuf, u32);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(self.1));
        }
    }
    let mode = fs::metadata(path).expect("metadata").permissions().mode();
    fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    Restore(path.to_path_buf(), mode)
}

/// Permission bits do not stop a privileged user; such a host cannot produce
/// an unreadable entry this way.
#[cfg(unix)]
fn readable_anyway(path: &Path) -> bool {
    fs::read_dir(path).is_ok() || fs::read(path).is_ok()
}

#[cfg(unix)]
#[test]
fn an_unreadable_record_blocks_rather_than_being_skipped() {
    let temporary = TempDir::new().expect("temporary directory");
    let record = temporary.path().join("current.json");
    write(&record, MINIMAL);
    let _restore = without_permissions(&record);
    if readable_anyway(&record) {
        return;
    }
    let error = load(temporary.path()).expect_err("an unreadable record blocks");
    assert_eq!(error.path, record);
    assert!(error.to_string().contains("current.json"), "{error}");
}

#[cfg(unix)]
#[test]
fn an_unreadable_directory_in_the_store_blocks() {
    let temporary = TempDir::new().expect("temporary directory");
    let nested = temporary.path().join("github.com");
    write(&nested.join("current.json"), MINIMAL);
    let _restore = without_permissions(&nested);
    if readable_anyway(&nested) {
        return;
    }
    let error = load(temporary.path()).expect_err("a traversal failure blocks");
    assert_eq!(error.path, nested);
}

#[test]
fn a_malformed_record_names_its_path_and_field() {
    let temporary = TempDir::new().expect("temporary directory");
    let record = temporary.path().join("repo").join("current.json");
    write(&record, &minimal_with("gate", Value::Null));
    let constraints = load(temporary.path()).expect("store");
    let described = constraints[0].describe();
    assert!(described.contains(&record.display().to_string()), "{described}");
    assert!(described.contains("`gate`"), "{described}");
}

#[test]
fn a_valid_record_describes_its_refusal() {
    let scoped = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    let constraint = Constraint {
        path: PathBuf::from("scoped.json"),
        record: crate::ci_push::parse_constraint_record(scoped.as_bytes(), "scoped.json"),
    };
    assert_eq!(
        constraint.describe(),
        "wsl2-ubuntu L1: WSL L1 already ran for this branch on build-wsl (set by ken, expires 2099-01-01)"
    );
}

// --- Plans ---

#[test]
fn a_missing_plan_file_is_an_error_naming_it() {
    let temporary = TempDir::new().expect("temporary directory");
    let plan = temporary.path().join("absent.json");
    let error = read_plan(&plan).expect_err("missing plan");
    assert_eq!(error.document, plan.display().to_string());
    assert!(error.problem.contains("cannot be read"), "{error}");
}

#[test]
fn without_a_plan_and_with_a_malformed_record_nothing_is_satisfied() {
    let valid = Constraint {
        path: PathBuf::from("valid.json"),
        record: crate::ci_push::parse_constraint_record(MINIMAL.as_bytes(), "valid.json"),
    };
    let malformed = Constraint {
        path: PathBuf::from("malformed.json"),
        record: crate::ci_push::parse_constraint_record(b"{}", "malformed.json"),
    };
    let both = [valid, malformed];
    assert_eq!(unsatisfied(&both, None).len(), 2);
    let reused = [ConstraintCell {
        environment: "wsl2-ubuntu".into(),
        gate: "L1".into(),
        execution: "reuse".into(),
        state: None,
    }];
    let left = unsatisfied(&both, Some(&reused));
    assert_eq!(left.len(), 1);
    assert!(left[0].is_malformed());
}

// --- Location ---

#[test]
fn the_default_is_the_repository_directory_under_the_given_home() {
    let home = Path::new("home");
    let root = home.join(".rusty-biscuit").join("ci-constraints");
    assert_eq!(store_root(home), root);
    assert_eq!(
        default_directory(home, "git@github.com:yankeeinlondon/rusty-biscuit.git"),
        root.join("github.com").join("yankeeinlondon").join("rusty-biscuit")
    );
    assert_eq!(default_directory(home, ""), root);
    assert_eq!(
        default_directory(home, "C:\\Users\\ken\\origin.git"),
        root.join("C").join("Users").join("ken").join("origin")
    );
}

#[test]
fn an_explicit_store_wins_over_the_variable_which_wins_over_the_default() {
    let home = Path::new("home");
    let remote = "git@github.com:yankeeinlondon/rusty-biscuit.git";
    let variable = std::ffi::OsStr::new("elsewhere");
    let explicit = Path::new("explicit");
    assert_eq!(ENVIRONMENT_VARIABLE, "BISCUIT_CI_CONSTRAINTS_DIR");
    assert_eq!(resolve_directory(home, remote, Some(explicit), Some(variable)), explicit);
    assert_eq!(resolve_directory(home, remote, None, Some(variable)), Path::new("elsewhere"));
    assert_eq!(
        resolve_directory(home, remote, Some(Path::new("")), Some(std::ffi::OsStr::new(""))),
        constraints::default_directory(home, remote)
    );
}
