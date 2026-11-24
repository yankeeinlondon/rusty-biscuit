//! The `config`, `repo-config`, and `stray-config-keys` rows over real
//! temporary homes and repositories, read through the built-in registry.
//!
//! Each case starts from a valid control and edits one file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};

use super::super::{ContextUnavailable, DoctorRow, Environment, Severity, file_checks, native_path, run_with};
use super::{ClaudineState, DarkmatterState, SemanticProblem, Unusable};

const CLAUDINE: &str = ".claudine/config.yaml";
const DARKMATTER: &str = ".darkmatter.yaml";

struct Fixture {
    _dir: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
}

impl Fixture {
    /// A home with valid user files and an empty repository.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        let (home, repo) = (root.join("home"), root.join("repo"));
        std::fs::create_dir_all(home.join(".claudine")).unwrap();
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        let fixture = Self { _dir: dir, home, repo };
        fixture.user(CLAUDINE, "# my settings\nlogging: true\n");
        fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
        fixture
    }

    fn user(&self, name: &str, text: &str) -> PathBuf {
        let path = self.home.join(native_path(name));
        std::fs::write(&path, text).unwrap();
        path
    }

    fn repository(&self, name: &str, text: &str) -> PathBuf {
        let path = self.repo.join(native_path(name));
        std::fs::write(&path, text).unwrap();
        path
    }

    fn context(&self, repository: Option<&Path>) -> FileResolutionContext {
        let cwd = repository.unwrap_or(&self.home).to_path_buf();
        let context = FileResolutionContext::from_snapshot(cwd, Some(self.home.clone()), HashMap::new());
        match repository {
            Some(root) => context.with_repository_root(root),
            None => context,
        }
    }

    fn environment(&self) -> Environment {
        Environment::for_test(Some(self.home.clone()), Ok(self.context(Some(&self.repo))))
    }

    fn rows(&self) -> Vec<DoctorRow> {
        run_with(file_checks(), &self.environment(), false)
    }

    /// Every entry under both roots with its bytes.
    fn tree(&self) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    out.push((path.clone(), None));
                    walk(&path, out);
                } else {
                    out.push((path.clone(), std::fs::read(&path).ok()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.home, &mut out);
        walk(&self.repo, &mut out);
        out
    }
}

fn row<'a>(rows: &'a [DoctorRow], id: &str) -> &'a DoctorRow {
    rows.iter().find(|row| row.id == id).unwrap_or_else(|| panic!("no `{id}` row"))
}

#[track_caller]
fn assert_row(rows: &[DoctorRow], id: &str, severity: Severity, needle: &str) {
    let row = row(rows, id);
    assert_eq!(row.severity, severity, "{row:#?}");
    assert!(row.summary.contains(needle) || row.detail.contains(needle), "`{needle}` not in {row:#?}");
}

// ---------------------------------------------------------------- control

#[test]
fn the_valid_control_passes_every_configuration_row() {
    let fixture = Fixture::new();
    let rows = fixture.rows();
    assert_row(&rows, "config", Severity::Ok, "every configuration file is valid");
    let config = row(&rows, "config");
    assert!(config.detail.contains(&format!("{}: valid", fixture.home.join(native_path(CLAUDINE)).display())), "{config:#?}");
    assert!(config.detail.contains(&format!("{}: not present (optional)", fixture.repo.join(DARKMATTER).display())));
    assert_row(&rows, "repo-config", Severity::Info, "no repository configuration (it is optional)");
    assert_row(&rows, "stray-config-keys", Severity::Ok, "no unknown Claudine configuration keys");
    // The registration and permissions rows have repairs of their own; this
    // fixture publishes no registration.
    assert!(["config", "repo-config", "stray-config-keys"].iter().all(|id| !row(&rows, id).fixable));
}

#[test]
fn an_inspection_writes_nothing_and_leaves_a_legacy_file_in_place() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::fs::write(fixture.home.join(native_path(".claudine/config.json")), "{ logging: true }").unwrap();
    fixture.repository(DARKMATTER, "bogus: 1\n");
    let before = fixture.tree();
    let rows = fixture.rows();
    assert_eq!(fixture.tree(), before);
    assert_row(&rows, "config", Severity::Error, "a legacy `config.json` beside it has not been converted");
    assert!(row(&rows, "config").detail.contains("run any other `claudine` command, which converts it"));
}

#[test]
fn a_missing_user_file_is_an_error_with_the_setup_remedy() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::fs::remove_file(fixture.home.join(DARKMATTER)).unwrap();
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert_eq!(config.summary, "is missing (+1 more)");
    assert_eq!(config.detail.matches("Remedy: run `claudine setup`").count(), 2, "{config:#?}");
    // Nothing to inspect for keys, and that is not a stray-key problem.
    assert_row(&rows, "stray-config-keys", Severity::Ok, "no unknown");
}

#[test]
fn outside_a_repository_the_repository_rows_are_informational() {
    let fixture = Fixture::new();
    let environment = Environment::for_test(Some(fixture.home.clone()), Ok(fixture.context(None)));
    let rows = run_with(file_checks(), &environment, false);
    assert_row(&rows, "repo-config", Severity::Info, "not inside a repository");
    assert_row(&rows, "config", Severity::Ok, "every configuration file is valid");
}

#[test]
fn without_a_home_or_a_context_the_config_row_says_why_and_other_files_are_still_judged() {
    let fixture = Fixture::new();
    let rows = run_with(file_checks(), &Environment::for_test(None, Err(ContextUnavailable)), false);
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert!(config.detail.contains("the home directory cannot be located"), "{config:#?}");
    assert!(config.detail.contains("file-resolution context could not be built"));
    assert_row(&rows, "repo-config", Severity::Info, "the repository could not be determined");

    // With the home but no context, the Claudine user file is still read.
    fixture.user(CLAUDINE, "bogus: 1\n");
    let rows = run_with(file_checks(), &Environment::for_test(Some(fixture.home.clone()), Err(ContextUnavailable)), false);
    assert_row(&rows, "stray-config-keys", Severity::Error, "unknown key `bogus`");
}

// ---------------------------------------------------------- unknown keys

#[test]
fn a_top_level_unknown_key_is_located_at_the_key_and_named_only_in_the_stray_row() {
    let fixture = Fixture::new();
    let path = fixture.user(CLAUDINE, "logging: true\nbogus: 1\n");
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert_eq!(config.summary, "has 1 unknown key");
    assert!(config.detail.contains("listed under `stray-config-keys`"));
    assert!(!config.summary.contains("bogus") && !config.detail.contains("bogus"), "{config:#?}");
    let stray = row(&rows, "stray-config-keys");
    assert_eq!(stray.severity, Severity::Error);
    assert_eq!(
        stray.detail,
        format!(
            "{}: unknown key `bogus` at the top level (line 2, column 1)\nRemedy: remove it, or correct its spelling",
            path.display()
        )
    );
    assert_eq!(stray.summary, "unknown key `bogus` at the top level (line 2, column 1)");
}

#[test]
fn nested_unknown_keys_are_found_in_the_tts_mapping_the_imported_permissions_and_a_keyed_action() {
    let fixture = Fixture::new();
    let path = fixture.user(
        CLAUDINE,
        "tts:\n  provider: say\n  pitch: 3\npermissions:\n  network: deny\n  sockets: []\nactions:\n  session_start:\n    - bash:\n        command: echo\n      extra: 1\n",
    );
    let rows = fixture.rows();
    let stray = row(&rows, "stray-config-keys");
    assert_eq!(stray.severity, Severity::Error, "{stray:#?}");
    let shown = |line: &str| format!("{}: {line}", path.display());
    // Listed in source order, not by pointer (`/actions…` sorts first).
    let mut at = 0;
    for expected in [
        "unknown key `pitch` in `/tts` (line 3, column 3)",
        "unknown key `sockets` in `/permissions` (line 6, column 3)",
        "unknown key `extra` in `/actions/session_start/0` (line 11, column 7)",
    ] {
        let found = stray.detail[at..].find(&shown(expected)).unwrap_or_else(|| panic!("`{expected}` not after byte {at} in {stray:#?}"));
        at += found + expected.len();
    }
    assert_eq!(stray.summary, "unknown key `pitch` in `/tts` (line 3, column 3) (+2 more)");
}

#[test]
fn a_known_key_of_another_arm_is_not_reported_as_stray() {
    let fixture = Fixture::new();
    // `voice` belongs to `tts-settings`; `true` is the other arm.
    fixture.user(CLAUDINE, "tts:\n  voice: Samantha\n  gender: female\n");
    let rows = fixture.rows();
    assert_row(&rows, "stray-config-keys", Severity::Ok, "no unknown");
    assert_row(&rows, "config", Severity::Ok, "every configuration file is valid");
}

#[test]
fn a_user_setting_in_the_repository_file_is_stray_there_with_a_move_remedy() {
    let fixture = Fixture::new();
    let path = fixture.repository(CLAUDINE, "tts: true\n");
    let rows = fixture.rows();
    let stray = row(&rows, "stray-config-keys");
    assert_eq!(stray.severity, Severity::Error);
    assert!(
        stray.detail.ends_with(&format!(
            "{}: unknown key `tts` at the top level (line 1, column 1)\nRemedy: it is a user setting; move it to `~/.claudine/config.yaml` or remove it",
            path.display()
        )),
        "{stray:#?}"
    );
    assert_row(&rows, "repo-config", Severity::Ok, "its problems are reported under `config`");
    // The imported `permissions` setting is valid in either scope.
    fixture.repository(CLAUDINE, "permissions:\n  network: deny\n");
    assert_row(&fixture.rows(), "stray-config-keys", Severity::Ok, "no unknown");
}

// ---------------------------------------------------------- YAML shapes

macro_rules! syntax_cases {
    ($($name:ident: $text:expr,)*) => {$(
        #[test]
        fn $name() {
            let fixture = Fixture::new();
            let path = fixture.user(CLAUDINE, $text);
            let rows = fixture.rows();
            let config = row(&rows, "config");
            assert_eq!(config.severity, Severity::Error, "{config:#?}");
            assert!(config.summary.starts_with("is not one valid YAML mapping"), "{config:#?}");
            assert_row(&rows, "stray-config-keys", Severity::Info, "could not be inspected for unknown keys");
            assert!(row(&rows, "stray-config-keys").detail.starts_with(&path.display().to_string()));
            assert!(!config.detail.contains("SECRET"), "{config:#?}");
        }
    )*};
}

syntax_cases! {
    a_duplicate_key_is_a_located_syntax_error: "logging: true\nlogging: SECRET\n",
    trailing_nonsense_is_a_syntax_error: "logging: true\n]SECRET[\n",
    malformed_yaml_is_a_syntax_error: "logging: [SECRET\n",
    a_second_document_is_a_syntax_error: "logging: true\n---\nlogging: SECRET\n",
    a_non_string_key_is_a_syntax_error: "1: SECRET\n",
    an_empty_document_is_not_the_empty_mapping: "",
}

#[test]
fn a_duplicate_key_names_the_repeated_key_line() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "logging: true\nlogging: false\n");
    assert_eq!(row(&fixture.rows(), "config").summary, "is not one valid YAML mapping (line 2, column 1)");
}

#[test]
fn a_bom_comments_explicit_null_and_the_empty_mapping_are_accepted() {
    let fixture = Fixture::new();
    for text in ["\u{feff}# only a comment\n{}\n", "{}", "tts:\nmessenger:\n", "# comment\nlogging: false # trailing\n"] {
        fixture.user(CLAUDINE, text);
        let rows = fixture.rows();
        assert_row(&rows, "config", Severity::Ok, "every configuration file is valid");
    }
}

// ---------------------------------------------------- values and redaction

#[test]
fn an_invalid_value_is_located_without_its_text() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "logging: true\ntts: SECRET-VALUE\n");
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert_eq!(config.summary, "has an invalid value at `/tts` (line 2, column 6)");
    let json = serde_json::to_string(&rows).unwrap();
    assert!(!json.contains("SECRET"), "{json}");
}

#[test]
fn a_route_name_the_user_chose_is_not_shown_in_a_pointer() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "messenger:\n  configurations:\n    SECRET-ROUTE:\n      slack:\n        channel_id: 42\n");
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error, "{config:#?}");
    assert!(config.summary.contains("`/messenger/configurations/*"), "{config:#?}");
    assert!(!serde_json::to_string(&rows).unwrap().contains("SECRET"));
}

#[test]
fn tts_shapes_are_judged_by_the_user_schema() {
    let fixture = Fixture::new();
    for (text, valid) in [
        ("tts: true\n", true),
        ("tts: false\n", true),
        ("tts:\n", true),
        ("tts: {}\n", true),
        ("tts:\n  provider:\n", true),
        ("tts:\n  provider: say\n", true),
        ("tts:\n  provider: not-a-real-engine\n", true),
        ("tts:\n  provider: \"\"\n", false),
        ("tts:\n  provider: 3\n", false),
        ("tts: 3\n", false),
        ("tts: [true]\n", false),
    ] {
        fixture.user(CLAUDINE, text);
        let severity = row(&fixture.rows(), "config").severity;
        assert_eq!(severity == Severity::Ok, valid, "{text:?} gave {severity:?}");
    }
}

#[test]
fn override_null_inherits_and_empty_mappings_and_lists_follow_the_schema() {
    let fixture = Fixture::new();
    for (scope_is_user, text, valid) in [
        (false, "actions:\nmatchers: null\nsteering:\n", true),
        (false, "actions: {}\n", false),
        (false, "steering: {}\n", false),
        (true, "actions:\n  session_start:\n", true),
        (false, "actions:\n  session_start:\n", true),
        (true, "actions:\n  session_start: []\n", false),
        (true, "actions:\n  session_start:\n    - bash: { command: echo }\n    - 3\n", false),
        (true, "actions:\n  session_start:\n    - 3\n    - true\n", false),
    ] {
        fixture.user(CLAUDINE, "logging: true\n");
        let _ = std::fs::remove_file(fixture.repo.join(native_path(CLAUDINE)));
        if scope_is_user {
            fixture.user(CLAUDINE, text);
        } else {
            fixture.repository(CLAUDINE, text);
        }
        let severity = row(&fixture.rows(), "config").severity;
        assert_eq!(severity == Severity::Ok, valid, "{text:?} (user: {scope_is_user}) gave {severity:?}");
    }
}

// ------------------------------------------------------- semantic checks

const ROUTES: &str = "messenger:\n  active_config: team\n  configurations:\n    team:\n      slack:\n        channel_id: C1\n";

#[test]
fn a_semantic_rule_names_the_setting_without_its_value() {
    let fixture = Fixture::new();
    let path = fixture.user(CLAUDINE, "default_sounds:\n  success: SECRET-SOUND\n");
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert_eq!(config.summary, "the setting `default_sounds.success` breaks a configuration rule");
    assert!(config.detail.contains(&format!("{}: the setting", path.display())));
    assert!(!serde_json::to_string(&rows).unwrap().contains("SECRET"));
    let observations = super::observe(&fixture.environment());
    assert!(matches!(
        observations.user_config,
        Err(Unusable::Semantic(SemanticProblem::Rule { setting: Some(_) }))
    ));
}

#[test]
fn a_mapper_the_runtime_compiler_refuses_is_a_config_error() {
    let fixture = Fixture::new();
    fixture.user(
        CLAUDINE,
        "actions:\n  before_tool:\n    - call:\n        command: check\n        mapper: { type: regex, pattern: \"(SECRET\" }\n",
    );
    let rows = fixture.rows();
    assert_row(&rows, "config", Severity::Error, "the mapper of action 0 of `before_tool` is not a valid regular expression");
    assert!(!serde_json::to_string(&rows).unwrap().contains("SECRET"));
}

#[test]
fn a_repository_selection_with_no_user_route_names_both_files() {
    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, ROUTES);
    let repo = fixture.repository(CLAUDINE, "active_messenger: elsewhere\n");
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error, "{config:#?}");
    assert!(config.detail.contains(&format!("{}: a setting breaks a configuration rule", repo.display())), "{config:#?}");
    assert!(config.detail.contains(&format!("merged onto {}", user.display())), "{config:#?}");
    let observations = super::observe(&fixture.environment());
    assert!(observations.user_config.is_ok());
    assert!(observations.effective_config.is_err());
}

#[test]
fn a_valid_merged_pair_supersedes_a_user_only_selection_problem() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, &ROUTES.replace("active_config: team", "active_config: gone"));
    fixture.repository(CLAUDINE, "active_messenger: team\n");
    let observations = super::observe(&fixture.environment());
    // The user file alone breaks the rule, which speech and the handoff see...
    assert!(observations.user_config.is_err());
    // ...but the effective configuration is valid, so `config` reports none.
    assert!(observations.effective_config.is_ok());
    assert_row(&fixture.rows(), "config", Severity::Ok, "every configuration file is valid");
}

// ------------------------------------------------------ aliases and files

#[test]
fn a_repository_at_the_home_is_judged_once_as_the_user_file() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "bogus: 1\n");
    let context = fixture.context(Some(&fixture.home));
    let environment = Environment::for_test(Some(fixture.home.clone()), Ok(context));
    let observations = super::observe(&environment);
    assert!(matches!(observations.claudine_repo.as_ref().unwrap().state, ClaudineState::AliasOfUser));
    assert!(matches!(observations.darkmatter_repo.as_ref().unwrap().state, DarkmatterState::AliasOfUser));
    let rows = run_with(file_checks(), &environment, false);
    assert_row(&rows, "repo-config", Severity::Ok, "is the user configuration");
    let stray = row(&rows, "stray-config-keys");
    assert_eq!(stray.detail.matches("unknown key `bogus`").count(), 1, "{stray:#?}");
    assert_eq!(row(&rows, "config").summary, "has 1 unknown key");
}

#[cfg(unix)]
#[test]
fn a_dangling_link_and_a_directory_are_unreadable_files() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::os::unix::fs::symlink(fixture.home.join("nowhere.yaml"), fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::fs::create_dir(fixture.repo.join(DARKMATTER)).unwrap();
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert!(config.detail.contains(&format!("{}: is a symbolic link to nothing", fixture.home.join(native_path(CLAUDINE)).display())));
    assert!(config.detail.contains(&format!("{}: is not a regular file", fixture.repo.join(DARKMATTER).display())));
}

#[test]
fn an_invalid_repository_file_is_present_and_its_problems_are_in_config() {
    let fixture = Fixture::new();
    let path = fixture.repository(CLAUDINE, "actions: 3\n");
    let rows = fixture.rows();
    assert_row(&rows, "repo-config", Severity::Ok, "present; its problems are reported under `config`");
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error);
    assert!(config.detail.contains(&format!("{}: has an invalid value at `/actions`", path.display())), "{config:#?}");
}

// ------------------------------------------------------------ Darkmatter

#[test]
fn darkmatter_files_are_judged_by_their_scope_schema() {
    let fixture = Fixture::new();
    for (user, text, needle) in [
        (true, "permissions: 42\n", "has an invalid value at `/permissions` (line 1, column 14)"),
        (true, "permissions: \"\"\n", "invalid"),
        (true, "register: 3\n", "has an invalid value at `/register`"),
        (true, "register: [~/.x/manifest.yaml, 3]\n", "has an invalid value at `/register/1`"),
        (true, "register: [3, true]\n", "has an invalid value at `/register/0`"),
        // Darkmatter locates an unknown key at its value.
        (true, "bogus: SECRET\n", "has an unknown key `bogus` (line 1, column 8)"),
        (false, "permissions:\n  shell-commands: [ls, 3]\n", "has an invalid value at `/permissions"),
        (false, "permissions:\n  file-read: [\"~/notes/*.md\"]\n", "invalid"),
        (true, "permissions:\n  file-read: [\"/etc/*.md\"]\n", "invalid"),
        (true, "permissions:\n  network: 3\n", "has an invalid value at `/permissions"),
        (true, "permissions:\n  network: SECRET\n", "has an invalid value at `/permissions"),
        (true, "a: 1\na: 2\n", "is not one valid YAML mapping (line 2, column 1)"),
    ] {
        fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
        let _ = std::fs::remove_file(fixture.repo.join(DARKMATTER));
        let path = if user { fixture.user(DARKMATTER, text) } else { fixture.repository(DARKMATTER, text) };
        let rows = fixture.rows();
        let config = row(&rows, "config");
        assert_eq!(config.severity, Severity::Error, "{text:?}: {config:#?}");
        assert!(config.detail.contains(&format!("{}: ", path.display())), "{text:?}: {config:#?}");
        assert!(config.detail.contains(needle), "{text:?}: `{needle}` not in {config:#?}");
        assert!(!serde_json::to_string(&rows).unwrap().contains("SECRET"), "{text:?}");
    }
}

#[test]
fn darkmatter_defaults_and_empty_values_are_valid() {
    let fixture = Fixture::new();
    for text in [
        "{}\n",
        "permissions:\n",
        "permissions: {}\n",
        "permissions:\n  shell-commands: []\n",
        // The scope projections declare `file-read` without `not-empty`.
        "permissions:\n  file-read: []\n",
        "register:\n",
        "register: []\n",
        "register: [~/.other-host/manifest.yaml]\n",
    ] {
        fixture.user(DARKMATTER, text);
        assert_row(&fixture.rows(), "config", Severity::Ok, "every configuration file is valid");
    }
}
