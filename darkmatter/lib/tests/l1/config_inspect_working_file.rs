//! `darkmatter::config::load::inspect_working_file`: the passive judgment of
//! one working-copy configuration file that `claudine doctor` relies on.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{ConfigError, ConfigErrorKind, PermissionsSetting, inspect_working_file};
use darkmatter::config::migration::LEGACY_WHITELIST_FILE_NAME;
use darkmatter::config::schema::{CONFIG_FILE_NAME, ConfigScope};

const SCOPES: [ConfigScope; 2] = [ConfigScope::User, ConfigScope::Repository];

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        for name in ["home", "work", "repo"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        Self { _dir: dir, root }
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.root.join("home")), HashMap::new())
            .with_repository_root(self.root.join("repo"))
    }

    fn dir(&self, scope: ConfigScope) -> PathBuf {
        match scope {
            ConfigScope::User => self.root.join("home"),
            ConfigScope::Repository => self.root.join("repo"),
        }
    }

    fn file(&self, scope: ConfigScope) -> PathBuf {
        self.dir(scope).join(CONFIG_FILE_NAME)
    }

    fn write(&self, scope: ConfigScope, text: &str) -> PathBuf {
        let path = self.file(scope);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// Every entry under the fixture (directories included), with the bytes
    /// of each file.
    fn tree(&self) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    out.push((path.clone(), None));
                    walk(&path, out);
                } else {
                    out.push((path.clone(), Some(std::fs::read(&path).unwrap())));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.sort();
        out
    }
}

/// The first problem of a whole-file failure: kind, file, line, column.
fn located(error: &ConfigError) -> (ConfigErrorKind, PathBuf, Option<usize>, Option<usize>) {
    let problem = &error.problems()[0];
    (error.kind(), problem.file.clone(), problem.line, problem.column)
}

#[test]
fn config_inspect_working_file_reports_an_absent_file_as_not_present() {
    let fixture = Fixture::new();
    for scope in SCOPES {
        let file = inspect_working_file(&fixture.context(), scope).unwrap();

        assert!(!file.is_present(), "{scope:?}");
        assert_eq!(file.path(), fixture.file(scope));
        assert!(file.permissions().is_ok(), "{scope:?}: an absent file takes its defaults");
    }
}

#[test]
fn config_inspect_working_file_accepts_a_valid_file_in_each_scope() {
    let fixture = Fixture::new();
    for scope in SCOPES {
        fixture.write(scope, "permissions:\n  network: deny\nregister: [./host.yaml]\n");

        let file = inspect_working_file(&fixture.context(), scope).unwrap();

        assert!(file.is_present(), "{scope:?}");
        assert!(matches!(file.permissions(), Ok(PermissionsSetting::Inline(_))), "{scope:?}");
        assert_eq!(file.registrations().len(), 1, "{scope:?}");
        assert_eq!(file.registrations()[0].reference, "./host.yaml");
        assert!(file.registration_findings().is_empty(), "{scope:?}");
    }
}

#[test]
fn config_inspect_working_file_keeps_an_invalid_permissions_value_inside_the_file() {
    let fixture = Fixture::new();
    for scope in SCOPES {
        let path = fixture.write(scope, "register: []\npermissions: 42\n");

        let file = inspect_working_file(&fixture.context(), scope).unwrap();

        let error = file.permissions().unwrap_err();
        let (kind, problem_file, line, column) = located(error);
        assert_eq!(kind, ConfigErrorKind::InvalidSetting, "{scope:?}: {error}");
        assert_eq!(problem_file, path);
        assert_eq!((line, column), (Some(2), Some(14)), "{scope:?}: {error}");
    }
}

#[test]
fn config_inspect_working_file_keeps_an_invalid_register_entry_as_a_finding() {
    let fixture = Fixture::new();
    for scope in SCOPES {
        let path = fixture.write(scope, "register:\n  - ./host.yaml\n  - 42\n");

        let file = inspect_working_file(&fixture.context(), scope).unwrap();

        assert_eq!(file.registrations().len(), 1, "{scope:?}: the valid entry is kept");
        let findings = file.registration_findings();
        assert_eq!(findings.len(), 1, "{scope:?}: {findings:?}");
        let problem = &findings[0].problems()[0];
        assert_eq!(problem.file, path);
        assert_eq!(problem.pointer, "/register/1");
        assert_eq!(problem.line, Some(3), "{problem}");
    }
}

#[test]
fn config_inspect_working_file_fails_a_whole_file_with_its_location() {
    let fixture = Fixture::new();
    let cases = [
        ("register: [\n", ConfigErrorKind::Syntax),
        ("register: []\nregister: []\n", ConfigErrorKind::Syntax),
        ("register: []\n\nnetwork: allow\n", ConfigErrorKind::UnknownKey),
        ("- register\n", ConfigErrorKind::NotAMapping),
    ];
    for scope in SCOPES {
        for (text, expected) in cases {
            let path = fixture.write(scope, text);

            let error = inspect_working_file(&fixture.context(), scope).unwrap_err();

            let (kind, problem_file, line, column) = located(&error);
            assert_eq!(kind, expected, "{scope:?} {text:?}: {error}");
            assert_eq!(problem_file, path, "{scope:?} {text:?}");
            if expected != ConfigErrorKind::NotAMapping {
                assert!(line.is_some() && column.is_some(), "{scope:?} {text:?}: {error}");
            }
            if expected == ConfigErrorKind::UnknownKey {
                assert_eq!((line, column), (Some(3), Some(10)), "{error}");
            }
        }
    }
}

#[test]
fn config_inspect_working_file_writes_nothing() {
    let fixture = Fixture::new();
    for scope in SCOPES {
        std::fs::write(fixture.dir(scope).join(LEGACY_WHITELIST_FILE_NAME), "echo hello\n").unwrap();
    }
    fixture.write(ConfigScope::Repository, "register: []\n");
    let before = fixture.tree();

    for scope in SCOPES {
        inspect_working_file(&fixture.context(), scope).unwrap();
    }

    assert_eq!(fixture.tree(), before, "no file created, migrated, or deleted");
    assert!(!fixture.file(ConfigScope::User).exists(), "the missing user file is not created");
}

#[test]
fn config_inspect_working_file_does_not_follow_a_permissions_reference() {
    let fixture = Fixture::new();
    let path = fixture.write(ConfigScope::Repository, "permissions: ./missing-policy.yaml\n");

    let file = inspect_working_file(&fixture.context(), ConfigScope::Repository).unwrap();

    match file.permissions() {
        Ok(PermissionsSetting::Reference(reference)) => {
            assert_eq!(reference.reference, "./missing-policy.yaml");
            assert_eq!(reference.declared_in, path);
        }
        other => panic!("expected the reference as written, got {other:?}"),
    }
}

#[test]
fn config_inspect_working_file_refuses_a_context_without_the_scope() {
    let fixture = Fixture::new();
    let bare = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());
    for scope in SCOPES {
        let error = inspect_working_file(&bare, scope).unwrap_err();

        assert_eq!(error.kind(), ConfigErrorKind::Read, "{scope:?}: {error}");
    }
}
