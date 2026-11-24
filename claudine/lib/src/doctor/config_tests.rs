//! The `config`, `repo-config`, and `stray-config-keys` rows over real
//! temporary homes and repositories, read through the built-in registry.
//!
//! Each case starts from a valid control and edits one file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};

use super::super::{ContextUnavailable, DoctorRow, Environment, Severity, file_checks, native_path, run_with};
use super::{ClaudineState, DarkmatterState, SemanticProblem, Unusable};
use crate::config::claudine_config::ValidatedSetting;

const CLAUDINE: &str = ".claudine/config.yaml";
const DARKMATTER: &str = ".darkmatter.yaml";
/// The user Darkmatter file: a reference to a policy file of its own, so
/// a case that rewrites Claudine's file leaves the reference's target valid.
const USER_DARKMATTER: &str = "permissions: ~/policy.yaml\n";
const POLICY: &str = "policy.yaml";
const BLOCK: &str = "permissions:\n  network: deny\n";

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
        fixture.user(POLICY, BLOCK);
        fixture.user(DARKMATTER, USER_DARKMATTER);
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
    assert!(row(&rows, "config").detail.contains("Remedy: run `claudine setup`, which converts it"));
}

#[test]
fn a_repository_legacy_file_is_reported_and_left_in_place_by_both_runs() {
    let fixture = Fixture::new();
    fixture.repository(".claudine/config.json", "{ canonical_provider: \"claude\" }");
    // `--fix` repairs the registration under the home; the repository is
    // what must stay as it was.
    let repository = |fixture: &Fixture| {
        fixture.tree().into_iter().filter(|(path, _)| path.starts_with(&fixture.repo)).collect::<Vec<_>>()
    };
    let before = repository(&fixture);
    for fix in [false, true] {
        let rows = run_with(file_checks(), &fixture.environment(), fix);
        assert_eq!(repository(&fixture), before, "fix: {fix}");
        assert_row(&rows, "config", Severity::Warn, "is not present; a legacy `config.json` beside it has not been converted");
        assert_row(&rows, "repo-config", Severity::Info, "a legacy `config.json` awaits conversion");
        assert!(!row(&rows, "repo-config").detail.contains("optional"), "{rows:#?}");
    }
}

#[test]
fn a_legacy_file_at_a_repository_at_the_home_is_reported_once_as_the_users() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::fs::write(fixture.home.join(native_path(".claudine/config.json")), "{}").unwrap();
    let environment = Environment::for_test(Some(fixture.home.clone()), Ok(fixture.context(Some(&fixture.home))));
    let rows = run_with(file_checks(), &environment, false);
    let config = row(&rows, "config");
    assert_eq!(config.severity, Severity::Error, "{config:#?}");
    assert_eq!(config.detail.matches("legacy `config.json`").count(), 1, "{config:#?}");
    assert_row(&rows, "repo-config", Severity::Info, "no repository configuration (it is optional)");
}

// -------------------------------------------------------------- remedies
//
// A remedy names a command only for a problem that command reports in that
// scope, and otherwise says what to correct; the CLI's `doctor.rs` runs the
// commands named here and checks they do.

#[test]
fn every_config_remedy_names_a_command_that_reads_that_file() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    std::fs::write(fixture.home.join(native_path(".claudine/config.json")), "{}").unwrap();
    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert!(config.detail.contains("Remedy: run `claudine setup`, which converts it"), "{config:#?}");

    let fixture = Fixture::new();
    fixture.repository(".claudine/config.json", "{}");
    let config = row(&fixture.rows(), "config").clone();
    assert!(
        config.detail.contains(
            "Remedy: `claudine claude` with a prompt, run inside the repository, converts it before the provider starts"
        ),
        "{config:#?}"
    );
    assert!(!config.detail.contains("claudine setup"), "setup does not convert a repository file: {config:#?}");

    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "logging: [\n");
    fixture.repository(CLAUDINE, "logging: [\n");
    fixture.user(DARKMATTER, "permissions: [\n");
    let config = row(&fixture.rows(), "config").clone();
    assert!(config.detail.contains("Remedy: correct it; `claudine actions` prints the full message"), "{config:#?}");
    assert!(
        config.detail.contains("Remedy: correct it; `claudine claude` run inside the repository stops with the full message"),
        "{config:#?}"
    );
    assert!(
        config.detail.contains("Remedy: correct it; `md compose` of a document that runs a shell command prints the full message"),
        "{config:#?}"
    );
    for vague in ["any other", "any ordinary", "`md` prints"] {
        assert!(!config.detail.contains(vague), "{config:#?}");
    }
}

#[test]
fn a_semantic_remedy_names_a_command_only_where_one_reports_the_rule() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "default_sounds:\n  success: SECRET-SOUND\n");
    let config = row(&fixture.rows(), "config").clone();
    assert!(config.detail.contains("Remedy: correct the setting; `claudine actions` prints the full message"), "{config:#?}");

    // No command reports a rule only the merged pair breaks: the runtime
    // falls back to its defaults.
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, ROUTES);
    fixture.repository(CLAUDINE, "active_messenger: elsewhere\n");
    let config = row(&fixture.rows(), "config").clone();
    assert!(config.detail.contains("Remedy: correct the setting\n"), "{config:#?}");
    assert!(!config.detail.contains("prints the full message"), "{config:#?}");
}

#[test]
fn a_user_mapper_remedy_names_the_setting_and_no_command() {
    // A load checks the rules, so `claudine actions` reports them; it never
    // compiles a mapper, so it lists a broken one and exits 0.
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, "exit_expressions:\n  - pattern: \"(SECRET\"\n    kind: regex\n");
    let config = row(&fixture.rows(), "config").clone();
    assert!(config.detail.contains("Remedy: correct the setting; `claudine actions` prints the full message"), "{config:#?}");

    let fixture = Fixture::new();
    fixture.user(CLAUDINE, BAD_USER_MAPPER);
    let config = row(&fixture.rows(), "config").clone();
    assert!(
        config.detail.contains(
            "Remedy: correct `actions.before_tool[0].call.mapper.pattern` so it is a valid regular expression"
        ),
        "{config:#?}"
    );
    assert!(!config.detail.contains("claudine actions"), "{config:#?}");
    assert!(!config.detail.contains("SECRET"), "{config:#?}");
}

#[test]
fn a_darkmatter_remedy_names_md_compose_only_for_problems_a_compose_prints() {
    const MD: &str = "Remedy: correct it; `md compose` of a document that runs a shell command prints the full message";
    let printed = [
        "permissions: [\n",
        "bogus: 1\n",
        "permissions:\n  network: SECRET\n",
        "permissions: https://example.com/SECRET.yaml\n",
    ];
    for repository in [false, true] {
        let write = |fixture: &Fixture, text: &str| {
            if repository { fixture.repository(DARKMATTER, text) } else { fixture.user(DARKMATTER, text) }
        };
        for text in printed {
            let fixture = Fixture::new();
            write(&fixture, text);
            let config = row(&fixture.rows(), "config").clone();
            assert!(config.detail.contains(MD), "{repository} {text:?}: {config:#?}");
        }
        // `md` reads `register` only to find host registrations: a compose
        // runs without reporting it.
        let fixture = Fixture::new();
        let path = write(&fixture, "register: 3\n");
        let config = row(&fixture.rows(), "config").clone();
        assert!(config.detail.contains(&format!("{}: has an invalid value at `/register` (line 1, column 11)", path.display())), "{config:#?}");
        assert!(
            config.detail.contains(
                "Remedy: correct the `register` setting at that position: it is a list of manifest references; then run `claudine doctor` again"
            ),
            "{repository}: {config:#?}"
        );
        assert!(!config.detail.contains("md compose"), "{repository}: {config:#?}");
    }
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

/// Setup publishes the host registration before it writes a missing user
/// file, and nothing else creates one, so a publisher obstruction leaves the
/// file missing and the remedy says so.
#[cfg(unix)]
#[test]
fn a_missing_user_file_behind_an_obstructed_publisher_is_named_in_the_setup_remedy() {
    use crate::composition::registration::{RegistrationInventory, publish};
    use crate::config::setup::{SetupError, setup_claudine};

    let fixture = Fixture::new();
    publish(&fixture.home, RegistrationInventory::claudine()).unwrap();
    let claudine = fixture.home.join(native_path(CLAUDINE));
    std::fs::remove_file(&claudine).unwrap();
    let (root, moved) = (fixture.home.join(native_path(".claudine/registrations/darkmatter")), fixture.home.join("moved"));
    std::fs::rename(&root, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &root).unwrap();

    let mut wrote = false;
    let setup = setup_claudine(&fixture.context(None), crate::config::schema::ConfigScope::User, |_| {
        wrote = true;
        Ok(crate::config::persist::SaveOutcome::default())
    });
    assert!(matches!(setup, Err(SetupError::Publish { .. })) && !wrote && !claudine.exists(), "{setup:#?}");

    let rows = fixture.rows();
    let config = row(&rows, "config");
    assert!(config.detail.contains("run `claudine setup`, which writes it after publishing the host registration; if publication stops, correct what setup names, then run it again"), "{config:#?}");
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

// ------------------------------------------------- legacy flat entries

/// The `config` and `stray-config-keys` rows of one Claudine file.
fn flat_rows(fixture: &Fixture, scope_file: &dyn Fn(&Fixture, &str) -> PathBuf, text: &str) -> (PathBuf, DoctorRow, DoctorRow) {
    let path = scope_file(fixture, text);
    let rows = fixture.rows();
    (path, row(&rows, "config").clone(), row(&rows, "stray-config-keys").clone())
}

/// Keyed and flat spellings of one entry, each with and without `timout`:
/// the flat form adds its migration finding and keeps the typo's verdict,
/// located at the key the file wrote.
fn flat_matrix(
    scope_file: &dyn Fn(&Fixture, &str) -> PathBuf,
    keyed: &str,
    flat: &str,
    keyed_typo: (&str, &str),
    flat_typo: (&str, &str),
    migration: &str,
) {
    let fixture = Fixture::new();
    let (_, config, stray) = flat_rows(&fixture, scope_file, keyed);
    assert_eq!(config.severity, Severity::Ok, "keyed control: {config:#?}");
    assert_eq!(stray.severity, Severity::Ok, "keyed control: {stray:#?}");

    for (text, within, is_flat) in
        [(format!("{keyed}{}", keyed_typo.0), keyed_typo.1, false), (format!("{flat}{}", flat_typo.0), flat_typo.1, true)]
    {
        let (path, config, stray) = flat_rows(&fixture, scope_file, &text);
        assert_eq!(stray.severity, Severity::Error, "{text}: {stray:#?}");
        assert!(
            stray.detail.ends_with(&format!(
                "{}: unknown key `timout` {within}\nRemedy: remove it, or correct its spelling",
                path.display()
            )),
            "{text}: {stray:#?}"
        );
        assert_eq!(stray.detail.matches("unknown key `").count(), 1, "{text}: {stray:#?}");
        assert_eq!(config.severity, Severity::Error, "{text}: {config:#?}");
        assert!(config.detail.contains("has 1 unknown key"), "{text}: {config:#?}");
        assert_eq!(config.detail.contains(migration), is_flat, "{text}: {config:#?}");
    }

    let (_, config, stray) = flat_rows(&fixture, scope_file, flat);
    assert_eq!(config.severity, Severity::Error, "flat control: {config:#?}");
    assert!(config.detail.contains(migration), "flat control: {config:#?}");
    assert!(!config.detail.contains("unknown key"), "the tag is not a stray key: {config:#?}");
    assert_eq!(stray.severity, Severity::Ok, "flat control: {stray:#?}");
}

const FLAT_ACTION_MIGRATION: &str = "has an invalid value at `/actions/session_start/0` (line 3, column 7)";

#[test]
fn a_flat_user_action_keeps_its_migration_finding_and_locates_a_typo() {
    flat_matrix(
        &|fixture, text| fixture.user(CLAUDINE, text),
        "actions:\n  session_start:\n    - bash:\n        command: echo hello\n",
        "actions:\n  session_start:\n    - type: bash\n      command: echo hello\n",
        ("        timout: 5\n", "in `/actions/session_start/0/bash` (line 5, column 9)"),
        ("      timout: 5\n", "in `/actions/session_start/0` (line 5, column 7)"),
        FLAT_ACTION_MIGRATION,
    );
}

#[test]
fn a_flat_repository_action_keeps_its_migration_finding_and_locates_a_typo() {
    flat_matrix(
        &|fixture, text| fixture.repository(CLAUDINE, text),
        "actions:\n  session_start:\n    - bash:\n        command: echo hello\n",
        "actions:\n  session_start:\n    - type: bash\n      command: echo hello\n",
        ("        timout: 5\n", "in `/actions/session_start/0/bash` (line 5, column 9)"),
        ("      timout: 5\n", "in `/actions/session_start/0` (line 5, column 7)"),
        FLAT_ACTION_MIGRATION,
    );
}

#[test]
fn a_flat_messenger_route_keeps_its_migration_finding_and_locates_a_typo() {
    flat_matrix(
        &|fixture, text| fixture.user(CLAUDINE, text),
        "messenger:\n  configurations:\n    team:\n      slack:\n        channel_id: C1\n",
        "messenger:\n  configurations:\n    team:\n      provider: slack\n      channel_id: C1\n",
        ("        timout: 5\n", "in `/messenger/configurations/*/slack` (line 6, column 9)"),
        ("      timout: 5\n", "in `/messenger/configurations/*` (line 6, column 7)"),
        "has an invalid value at `/messenger/configurations/*` (line 4, column 7)",
    );
}

#[test]
fn a_flat_entry_of_an_unknown_kind_or_with_a_wrong_type_is_still_judged() {
    let fixture = Fixture::new();
    // An unknown tag is no flat entry: the ordinary union verdict, which
    // names the keys no keyed arm declares, and no migration finding.
    let (_, config, stray) =
        flat_rows(&fixture, &|fixture, text| fixture.user(CLAUDINE, text), "actions:\n  session_start:\n    - type: bsh\n      command: x\n");
    assert_eq!(config.summary, "has 2 unknown keys", "{config:#?}");
    assert!(!config.detail.contains("invalid value"), "{config:#?}");
    assert!(stray.detail.contains("unknown key `type` in `/actions/session_start/0` (line 3, column 7)"), "{stray:#?}");
    assert!(stray.detail.contains("unknown key `command` in `/actions/session_start/0` (line 4, column 7)"), "{stray:#?}");

    // A wrong-typed setting of a flat entry is located at the value written.
    let (_, config, stray) = flat_rows(
        &fixture,
        &|fixture, text| fixture.user(CLAUDINE, text),
        "actions:\n  session_start:\n    - type: call\n      command: c\n      timeout_ms: soon\n",
    );
    assert_eq!(config.severity, Severity::Error, "{config:#?}");
    assert!(config.detail.contains(FLAT_ACTION_MIGRATION), "{config:#?}");
    assert!(
        config.detail.contains("has an invalid value at `/actions/session_start/0/timeout_ms` (line 5, column 19)"),
        "{config:#?}"
    );
    assert!(!serde_json::to_string(&config).unwrap().contains("soon"), "{config:#?}");
    assert_eq!(stray.severity, Severity::Ok, "{stray:#?}");
}

// ------------------------- independent document rules beside an entry

/// One action or route as the file writes it, and the pointer of its
/// migration problem when it is flat.
struct Entry {
    text: &'static str,
    migration: Option<&'static str>,
    /// The unknown key the entry misspells, at the key the file wrote.
    typo: Option<&'static str>,
}

const KEYED_ACTION: Entry =
    Entry { text: "actions:\n  session_start:\n    - bash:\n        command: echo hello\n", migration: None, typo: None };
const FLAT_ACTION: Entry = Entry {
    text: "actions:\n  session_start:\n    - type: bash\n      command: echo hello\n",
    migration: Some("/actions/session_start/0"),
    typo: None,
};
const KEYED_ACTION_TYPO: Entry = Entry {
    text: "actions:\n  session_start:\n    - bash:\n        command: echo hello\n        timout: 5\n",
    migration: None,
    typo: Some("/actions/session_start/0/bash/timout"),
};
const FLAT_ACTION_TYPO: Entry = Entry {
    text: "actions:\n  session_start:\n    - type: bash\n      command: echo hello\n      timout: 5\n",
    migration: Some("/actions/session_start/0"),
    typo: Some("/actions/session_start/0/timout"),
};
const KEYED_ROUTE: Entry =
    Entry { text: "messenger:\n  configurations:\n    team:\n      slack:\n        channel_id: C1\n", migration: None, typo: None };
const FLAT_ROUTE: Entry = Entry {
    text: "messenger:\n  configurations:\n    team:\n      provider: slack\n      channel_id: C1\n",
    migration: Some("/messenger/configurations/team"),
    typo: None,
};

/// A document-level defect the schema cannot express, written first in the
/// file, and the pointer, line, and column it is reported at.
type Defect = (&'static str, &'static str, usize, usize);

const BOTH_AGENTS: Defect = ("preferred_agent: claude\nfavorite_agent: claude\n", "/favorite_agent", 2, 17);
const BOTH_PATTERNS: Defect = ("exit_expressions:\n  - pattern: a\n    patterns: [b]\n", "/exit_expressions/0", 2, 5);
const NO_PATTERN: Defect = ("exit_expressions:\n  - kind: regex\n", "/exit_expressions/0", 2, 5);
const RULES_BOTH_PATTERNS: Defect =
    ("exit_expressions:\n  rules:\n    - pattern: a\n      patterns: [b]\n", "/exit_expressions/rules/0", 3, 7);

/// A defect beside an entry is reported, at its authored coordinates,
/// whatever form the entry takes: a flat entry adds its migration problem
/// and a misspelled setting its unknown key, and neither hides the defect.
fn independent_rule_case(scope: crate::config::schema::ConfigScope, entry: &Entry, defect: Defect) {
    use crate::config::load::{ConfigProblemKind, read_document};
    use crate::config::schema::ConfigScope;

    let (defect_text, pointer, line, column) = defect;
    let fixture = Fixture::new();
    let text = format!("{defect_text}{}", entry.text);
    let path = match scope {
        ConfigScope::User => fixture.user(CLAUDINE, &text),
        ConfigScope::Repository => fixture.repository(CLAUDINE, &text),
    };
    let problems = match read_document(&path, scope) {
        Err(crate::error::ClaudineError::ConfigInvalid { problems }) => problems,
        other => panic!("{text}: expected ConfigInvalid, got {other:?}"),
    };
    let found: Vec<(ConfigProblemKind, &str)> = problems.iter().map(|problem| (problem.kind, problem.pointer.as_str())).collect();
    let mut expected = vec![(ConfigProblemKind::InvalidSetting, pointer)];
    expected.extend(entry.migration.map(|at| (ConfigProblemKind::InvalidSetting, at)));
    expected.extend(entry.typo.map(|at| (ConfigProblemKind::UnknownKey, at)));
    let mut sorted = found.clone();
    sorted.sort_by_key(|(_, at)| *at);
    expected.sort_by_key(|(_, at)| *at);
    assert_eq!(sorted, expected, "{text}: {problems:#?}");
    let independent = problems.iter().find(|problem| problem.pointer == pointer).unwrap();
    assert_eq!((independent.line, independent.column), (Some(line), Some(column)), "{text}: {independent:#?}");

    let rows = fixture.rows();
    let reported = row(&rows, "config");
    assert_eq!(reported.severity, Severity::Error, "{text}: {reported:#?}");
    assert!(
        reported.detail.contains(&format!("{}: has an invalid value at `{pointer}` (line {line}, column {column})", path.display())),
        "{text}: {reported:#?}"
    );
}

macro_rules! independent_rule_cases {
    ($($name:ident: $scope:ident, $entry:expr, $defect:expr;)*) => {$(
        #[test]
        fn $name() {
            independent_rule_case(crate::config::schema::ConfigScope::$scope, &$entry, $defect);
        }
    )*};
}

independent_rule_cases! {
    user_keyed_action_beside_both_agent_names: User, KEYED_ACTION, BOTH_AGENTS;
    user_flat_action_beside_both_agent_names: User, FLAT_ACTION, BOTH_AGENTS;
    user_keyed_action_beside_both_patterns: User, KEYED_ACTION, BOTH_PATTERNS;
    user_flat_action_beside_both_patterns: User, FLAT_ACTION, BOTH_PATTERNS;
    user_keyed_action_beside_no_pattern: User, KEYED_ACTION, NO_PATTERN;
    user_flat_action_beside_no_pattern: User, FLAT_ACTION, NO_PATTERN;
    user_keyed_action_beside_both_rule_patterns: User, KEYED_ACTION, RULES_BOTH_PATTERNS;
    user_flat_action_beside_both_rule_patterns: User, FLAT_ACTION, RULES_BOTH_PATTERNS;
    user_misspelled_keyed_action_beside_both_rule_patterns: User, KEYED_ACTION_TYPO, RULES_BOTH_PATTERNS;
    user_misspelled_flat_action_beside_both_agent_names: User, FLAT_ACTION_TYPO, BOTH_AGENTS;
    repository_keyed_action_beside_both_patterns: Repository, KEYED_ACTION, BOTH_PATTERNS;
    repository_flat_action_beside_both_patterns: Repository, FLAT_ACTION, BOTH_PATTERNS;
    repository_keyed_action_beside_no_pattern: Repository, KEYED_ACTION, NO_PATTERN;
    repository_flat_action_beside_no_pattern: Repository, FLAT_ACTION, NO_PATTERN;
    repository_keyed_action_beside_both_rule_patterns: Repository, KEYED_ACTION, RULES_BOTH_PATTERNS;
    repository_flat_action_beside_both_rule_patterns: Repository, FLAT_ACTION, RULES_BOTH_PATTERNS;
    user_keyed_route_beside_both_agent_names: User, KEYED_ROUTE, BOTH_AGENTS;
    user_flat_route_beside_both_agent_names: User, FLAT_ROUTE, BOTH_AGENTS;
    user_keyed_route_beside_both_rule_patterns: User, KEYED_ROUTE, RULES_BOTH_PATTERNS;
    user_flat_route_beside_both_rule_patterns: User, FLAT_ROUTE, RULES_BOTH_PATTERNS;
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
        Err(Unusable::Semantic(SemanticProblem::Rule { setting: ValidatedSetting::DefaultSound("success") }))
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
    assert!(
        config.detail.contains(&format!("{}: the setting `active_messenger` breaks a configuration rule", repo.display())),
        "{config:#?}"
    );
    assert!(config.detail.contains(&format!("must be defined in `messenger.configurations` of {}", user.display())), "{config:#?}");
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

/// The `config` row's semantic findings: `(file, summary)` per finding
/// line, with every line checked for the sentinel `SECRET`.
#[track_caller]
fn semantic_lines(fixture: &Fixture) -> (Severity, String) {
    let rows = fixture.rows();
    assert!(!serde_json::to_string(&rows).unwrap().contains("SECRET"), "{rows:#?}");
    let config = row(&rows, "config");
    (config.severity, config.detail.clone())
}

const BAD_USER_MAPPER: &str =
    "actions:\n  before_tool:\n    - call:\n        command: check\n        mapper: { type: regex, pattern: \"(SECRET\" }\n";
const GOOD_MAPPER: &str = "    - call:\n        command: check\n        mapper: { type: regex, pattern: \"ok\" }\n";
const BAD_MAPPER: &str = "    - call:\n        command: check\n        mapper: { type: regex, pattern: \"(SECRET\" }\n";

#[test]
fn a_failing_repository_selection_names_the_repository_selection_and_the_user_routes() {
    // The user selects a route it does not define; the repository selection
    // decides the effective one.
    let user_text = ROUTES.replace("active_config: team", "active_config: SECRET-GONE");
    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, &user_text);
    fixture.repository(CLAUDINE, "active_messenger: team\n");
    assert_eq!(semantic_lines(&fixture).0, Severity::Ok);

    let repo = fixture.repository(CLAUDINE, "active_messenger: SECRET-ELSEWHERE\n");
    let (severity, detail) = semantic_lines(&fixture);
    assert_eq!(severity, Severity::Error, "{detail}");
    assert!(detail.contains(&format!("{}: the setting `active_messenger` breaks a configuration rule", repo.display())), "{detail}");
    assert!(detail.contains(&format!("must be defined in `messenger.configurations` of {}", user.display())), "{detail}");
    // The user's own selection is replaced, so it is not reported, and no
    // remedy sends the reader to a command that reads the user file alone.
    assert!(!detail.contains(&format!("{}: the", user.display())), "{detail}");
    assert!(!detail.contains("`claudine actions`"), "{detail}");
}

#[test]
fn an_action_mapper_failure_names_the_file_that_authors_the_event() {
    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, BAD_USER_MAPPER);
    // The repository replaces the bad user event with a valid list.
    fixture.repository(CLAUDINE, &format!("actions:\n  before_tool:\n{GOOD_MAPPER}"));
    assert_eq!(semantic_lines(&fixture).0, Severity::Ok);

    let repo = fixture.repository(CLAUDINE, &format!("actions:\n  before_tool:\n{GOOD_MAPPER}  after_tool:\n{BAD_MAPPER}"));
    let (severity, detail) = semantic_lines(&fixture);
    assert_eq!(severity, Severity::Error, "{detail}");
    assert!(
        detail.contains(&format!("{}: the mapper of action 0 of `after_tool` is not a valid regular expression", repo.display())),
        "{detail}"
    );
    assert!(!detail.contains(&format!("{}: the", user.display())), "{detail}");

    // A user event the repository does not list stays the user's.
    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, BAD_USER_MAPPER);
    fixture.repository(CLAUDINE, &format!("actions:\n  after_tool:\n{GOOD_MAPPER}"));
    let (_, detail) = semantic_lines(&fixture);
    assert!(
        detail.contains(&format!("{}: the mapper of action 0 of `before_tool` is not a valid regular expression", user.display())),
        "{detail}"
    );
}

#[test]
fn an_exit_expression_failure_names_the_file_whose_declaration_is_effective() {
    let bad_regex = "exit_expressions:\n  - pattern: \"(SECRET\"\n    kind: regex\n";
    let valid = "exit_expressions:\n  - pattern: stop\n";
    let unknown_scope = "exit_expressions:\n  - pattern: stop\n    scope: SECRET-AGENT\n";

    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, bad_regex);
    fixture.repository(CLAUDINE, valid);
    assert_eq!(semantic_lines(&fixture).0, Severity::Ok);

    let repo = fixture.repository(CLAUDINE, unknown_scope);
    let (severity, detail) = semantic_lines(&fixture);
    assert_eq!(severity, Severity::Error, "{detail}");
    assert!(detail.contains(&format!("{}: the setting `exit_expressions` breaks a configuration rule", repo.display())), "{detail}");
    assert!(!detail.contains(&format!("{}: the", user.display())), "{detail}");

    // With a valid user file the repository rule is still the repository's.
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, valid);
    let repo = fixture.repository(CLAUDINE, unknown_scope);
    let (severity, detail) = semantic_lines(&fixture);
    assert_eq!(severity, Severity::Error, "{detail}");
    assert!(detail.contains(&format!("{}: the setting `exit_expressions` breaks a configuration rule", repo.display())), "{detail}");
}

#[test]
fn a_valid_repository_selection_of_a_defined_user_route_passes() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, ROUTES);
    fixture.repository(CLAUDINE, "active_messenger: team\n");
    assert_row(&fixture.rows(), "config", Severity::Ok, "every configuration file is valid");
}

#[test]
fn user_only_settings_keep_the_user_attribution_beside_any_override() {
    let sound = "default_sounds:\n  success: SECRET-SOUND\n";
    for repo_text in ["active_messenger: team\n", "exit_expressions:\n  - pattern: stop\n", "actions:\n  before_tool: []\n"] {
        let fixture = Fixture::new();
        let user = fixture.user(CLAUDINE, &format!("{ROUTES}{sound}"));
        fixture.repository(CLAUDINE, repo_text);
        let (severity, detail) = semantic_lines(&fixture);
        assert_eq!(severity, Severity::Error, "{repo_text:?}: {detail}");
        assert!(
            detail.contains(&format!("{}: the setting `default_sounds.success` breaks a configuration rule", user.display())),
            "{repo_text:?}: {detail}"
        );
    }

    // The merged check stops at the repository's selection first; the user's
    // own sound is still reported, under the user file.
    let fixture = Fixture::new();
    let user = fixture.user(CLAUDINE, &format!("{ROUTES}{sound}"));
    let repo = fixture.repository(CLAUDINE, "active_messenger: SECRET-ELSEWHERE\n");
    let (_, detail) = semantic_lines(&fixture);
    assert!(detail.contains(&format!("{}: the setting `active_messenger`", repo.display())), "{detail}");
    assert!(detail.contains(&format!("{}: the setting `default_sounds.success`", user.display())), "{detail}");
    let observations = super::observe(&fixture.environment());
    assert!(observations.superseded_user_problem().is_none());
}

#[test]
fn a_user_problem_the_overrides_replace_is_superseded_and_left_to_its_readers() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, &ROUTES.replace("active_config: team", "active_config: gone"));
    fixture.repository(CLAUDINE, "active_messenger: team\n");
    let observations = super::observe(&fixture.environment());
    assert_eq!(
        observations.superseded_user_problem(),
        Some(&SemanticProblem::Rule { setting: ValidatedSetting::MessengerSelection })
    );

    // Outside the repository `config` reports it, so it is not superseded.
    let environment = Environment::for_test(Some(fixture.home.clone()), Ok(fixture.context(None)));
    assert!(super::observe(&environment).superseded_user_problem().is_none());
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
        fixture.user(DARKMATTER, USER_DARKMATTER);
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

// ------------------------------------------------- Darkmatter reference targets
//
// A `.darkmatter.yaml` whose `permissions` is a valid reference has its
// target judged by Darkmatter's working-copy resolver, whatever state the
// scope's Claudine file is in: the `darkmatter-permissions` handoff may skip
// an incomplete pair, but `config` still reports the broken target.

/// The state of the scope's Claudine file beside the Darkmatter file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Counterpart {
    Absent,
    /// The repository file is a link to the user file (repository only).
    #[cfg(unix)]
    Aliased,
    Malformed,
    /// A user `config.json` with no YAML file beside it (user only).
    Legacy,
    Valid,
}

impl Counterpart {
    fn for_scope(repository: bool) -> Vec<Self> {
        let mut all = vec![Self::Absent, Self::Malformed, Self::Valid];
        if repository {
            #[cfg(unix)]
            all.push(Self::Aliased);
        } else {
            all.push(Self::Legacy);
        }
        all
    }
}

/// What the reference names; written as `policy.yaml` at the scope root.
#[derive(Debug, Clone, Copy)]
enum Target {
    Missing,
    Text(&'static str),
    /// A link to a valid policy file outside both the home and the
    /// repository.
    #[cfg(unix)]
    OutsideLink,
}

/// One scope's files for a target case, and the paths the report names.
struct TargetCase {
    fixture: Fixture,
    darkmatter: PathBuf,
    claudine: PathBuf,
}

impl TargetCase {
    fn new(repository: bool, counterpart: Counterpart, declaration: &str, target: Target) -> Self {
        let fixture = Fixture::new();
        let root = if repository { fixture.repo.clone() } else { fixture.home.clone() };
        let claudine = root.join(native_path(CLAUDINE));
        let policy = root.join(POLICY);
        let _ = std::fs::remove_file(&policy);
        match target {
            Target::Missing => {}
            Target::Text(text) => std::fs::write(&policy, text).unwrap(),
            #[cfg(unix)]
            Target::OutsideLink => {
                let outside = root.parent().unwrap().join("outside");
                std::fs::create_dir_all(&outside).unwrap();
                std::fs::write(outside.join(POLICY), BLOCK).unwrap();
                std::os::unix::fs::symlink(outside.join(POLICY), &policy).unwrap();
            }
        }
        let darkmatter = root.join(DARKMATTER);
        std::fs::write(&darkmatter, declaration).unwrap();
        match counterpart {
            Counterpart::Absent => {
                let _ = std::fs::remove_file(&claudine);
            }
            #[cfg(unix)]
            Counterpart::Aliased => std::os::unix::fs::symlink(fixture.home.join(native_path(CLAUDINE)), &claudine).unwrap(),
            Counterpart::Malformed => std::fs::write(&claudine, "logging: [\n").unwrap(),
            Counterpart::Legacy => {
                std::fs::remove_file(&claudine).unwrap();
                std::fs::write(claudine.with_file_name("config.json"), "{ logging: true }").unwrap();
            }
            Counterpart::Valid if repository => std::fs::write(&claudine, "# repo\nactive_messenger: null\n").unwrap(),
            Counterpart::Valid => {}
        }
        Self { fixture, darkmatter, claudine }
    }
}

/// The reference as each scope's tests write it; its value starts at
/// column 14.
fn declaration(repository: bool) -> &'static str {
    if repository { "permissions: ./policy.yaml\n" } else { USER_DARKMATTER }
}

/// What `config` says of the Claudine counterpart, beside the target.
fn counterpart_needle(counterpart: Counterpart) -> Option<&'static str> {
    match counterpart {
        Counterpart::Malformed => Some("is not one valid YAML mapping"),
        Counterpart::Legacy => Some("is missing; a legacy `config.json` beside it has not been converted"),
        Counterpart::Absent | Counterpart::Valid => None,
        #[cfg(unix)]
        Counterpart::Aliased => None,
    }
}

/// What `darkmatter-permissions` says of the scope.
fn handoff_needle(repository: bool, counterpart: Counterpart) -> String {
    let label = if repository { "repository" } else { "user" };
    match counterpart {
        Counterpart::Absent | Counterpart::Legacy => {
            format!("{label} scope skipped: this Claudine configuration does not exist")
        }
        #[cfg(unix)]
        Counterpart::Aliased => "repository scope skipped: this file is the user file".into(),
        Counterpart::Malformed => format!("{label} scope skipped: this file has problems, reported under `config`"),
        Counterpart::Valid => format!("the {label} `permissions` refers to a file, which does not supply a valid"),
    }
}

/// Every counterpart of the scope with a broken `target`: `config` is an
/// error naming the Darkmatter file and the reference's position, once,
/// beside any Claudine error of its own; the handoff keeps its verdict and
/// refers to `config` instead of repeating the diagnostic.
#[track_caller]
fn broken_target_case(repository: bool, target: Target, needle: &str) {
    for counterpart in Counterpart::for_scope(repository) {
        let case = TargetCase::new(repository, counterpart, declaration(repository), target);
        let rows = case.fixture.rows();
        let config = row(&rows, "config");
        let what = format!("{repository} {counterpart:?} {target:?}");
        assert_eq!(config.severity, Severity::Error, "{what}: {config:#?}");
        let located = format!("{}: has a `permissions` reference {needle} (line 1, column 14)", case.darkmatter.display());
        assert_eq!(config.detail.matches(&located).count(), 1, "{what}: `{located}` not once in {config:#?}");
        assert_eq!(config.detail.matches("has a `permissions` reference").count(), 1, "{what}: {config:#?}");
        if let Some(own) = counterpart_needle(counterpart) {
            assert!(config.detail.contains(&format!("{}: {own}", case.claudine.display())), "{what}: {config:#?}");
        }
        let handoff = row(&rows, "darkmatter-permissions");
        assert!(handoff.detail.contains(&handoff_needle(repository, counterpart)), "{what}: {handoff:#?}");
        if counterpart == Counterpart::Valid {
            assert_eq!(handoff.severity, Severity::Error, "{what}: {handoff:#?}");
            assert!(handoff.detail.contains("the reference is reported under `config`"), "{what}: {handoff:#?}");
            assert!(!handoff.detail.contains("line 1, column 14"), "{what}: {handoff:#?}");
        }
        assert!(!handoff.fixable, "{what}: a broken reference is never repaired");
        let report = serde_json::to_string(&rows).unwrap();
        assert!(!report.contains("policy.yaml") && !report.contains("SECRET"), "{what}: {report}");
    }
}

macro_rules! broken_target_cases {
    ($($(#[$meta:meta])* $name:ident: $target:expr => $needle:expr;)*) => {$(
        $(#[$meta])*
        #[test]
        fn $name() {
            broken_target_case(true, $target, $needle);
            broken_target_case(false, $target, $needle);
        }
    )*};
}

const UNUSABLE: &str = "to a file that does not supply a valid `permissions` mapping";

broken_target_cases! {
    a_missing_target_is_a_config_error_in_both_scopes: Target::Missing => "to a file that does not exist";
    an_invalid_yaml_target_is_a_config_error_in_both_scopes: Target::Text("permissions: [\n") => UNUSABLE;
    a_chained_target_is_a_config_error_in_both_scopes: Target::Text("permissions: ./SECRET-next.yaml\n") => UNUSABLE;
    a_null_block_target_is_a_config_error_in_both_scopes: Target::Text("permissions: null\n") => UNUSABLE;
    a_scalar_block_target_is_a_config_error_in_both_scopes: Target::Text("permissions: 3\n") => UNUSABLE;
    a_target_without_a_block_is_a_config_error_in_both_scopes: Target::Text("other: SECRET\n") => UNUSABLE;
    a_nonmapping_target_is_a_config_error_in_both_scopes: Target::Text("- SECRET\n- 2\n") => UNUSABLE;
}

/// Darkmatter's repository policy refuses a target leading out of the
/// repository; its user policy follows such a link.
#[cfg(unix)]
#[test]
fn a_link_out_of_the_repository_is_a_config_error_and_allowed_for_the_user() {
    broken_target_case(true, Target::OutsideLink, "that does not lead to a file inside the repository");
    for counterpart in Counterpart::for_scope(false) {
        let case = TargetCase::new(false, counterpart, USER_DARKMATTER, Target::OutsideLink);
        let config = row(&case.fixture.rows(), "config").clone();
        assert!(config.detail.contains(&format!("{}: valid", case.darkmatter.display())), "{counterpart:?}: {config:#?}");
        assert!(!config.detail.contains("`permissions` reference"), "{counterpart:?}: {config:#?}");
    }
}

#[test]
fn a_valid_target_passes_in_both_spellings_whatever_the_counterpart() {
    for repository in [true, false] {
        let spellings: &[&str] = if repository {
            &["permissions: ./policy.yaml\n", "permissions: ././policy.yaml\n"]
        } else {
            &[USER_DARKMATTER, "permissions: ~/././policy.yaml\n"]
        };
        for spelling in spellings {
            for counterpart in Counterpart::for_scope(repository) {
                let case = TargetCase::new(repository, counterpart, spelling, Target::Text(BLOCK));
                let config = row(&case.fixture.rows(), "config").clone();
                let what = format!("{repository} {spelling:?} {counterpart:?}");
                assert!(config.detail.contains(&format!("{}: valid", case.darkmatter.display())), "{what}: {config:#?}");
                assert!(!config.detail.contains("`permissions` reference"), "{what}: {config:#?}");
                let expected = if counterpart_needle(counterpart).is_some() || (!repository && counterpart == Counterpart::Absent) {
                    Severity::Error
                } else {
                    Severity::Ok
                };
                assert_eq!(config.severity, expected, "{what}: {config:#?}");
            }
        }
    }
}

/// The `permissions` field's own shape is judged before any target: absent
/// and `null` are no reference, a blank or non-string value is one
/// declaration error, and only a valid string is followed.
#[test]
fn the_permissions_declaration_is_judged_once_before_any_target() {
    for repository in [true, false] {
        for counterpart in Counterpart::for_scope(repository) {
            for (declaration, finding) in [
                ("register: []\n", None),
                ("permissions: null\n", None),
                ("permissions: \"\"\n", Some("has an invalid `permissions` reference")),
                ("permissions: \"   \"\n", Some("has an invalid `permissions` reference")),
                ("permissions: 42\n", Some("has an invalid value at `/permissions`")),
                ("permissions: true\n", Some("has an invalid value at `/permissions`")),
            ] {
                let case = TargetCase::new(repository, counterpart, declaration, Target::Missing);
                let config = row(&case.fixture.rows(), "config").clone();
                let what = format!("{repository} {counterpart:?} {declaration:?}");
                assert!(!config.detail.contains("has a `permissions` reference"), "{what}: {config:#?}");
                let shown = case.darkmatter.display().to_string();
                match finding {
                    None => assert!(config.detail.contains(&format!("{shown}: valid")), "{what}: {config:#?}"),
                    Some(finding) => {
                        assert_eq!(config.severity, Severity::Error, "{what}: {config:#?}");
                        assert_eq!(config.detail.matches(&format!("{shown}: {finding}")).count(), 1, "{what}: {config:#?}");
                        assert_eq!(config.detail.matches(&format!("{shown}: ")).count(), 1, "{what}: {config:#?}");
                    }
                }
            }
        }
    }
}

/// `--fix` repairs no reference target, and a repository `.darkmatter.yaml`
/// that is the user file is judged once, as the user file.
#[test]
fn a_broken_target_is_left_by_fix_and_an_aliased_darkmatter_file_is_judged_once() {
    for repository in [true, false] {
        for counterpart in Counterpart::for_scope(repository) {
            let case = TargetCase::new(repository, counterpart, declaration(repository), Target::Missing);
            let policy = case.darkmatter.with_file_name(POLICY);
            // The registration repair may add its `register` entry to the
            // user file; the reference, its target, and the Claudine side
            // are what must stay.
            let files = || {
                let darkmatter = std::fs::read_to_string(&case.darkmatter).unwrap();
                let reference = darkmatter.lines().find(|line| line.starts_with("permissions:")).map(str::to_owned);
                let claudine = [case.claudine.clone(), case.claudine.with_file_name("config.json"), policy.clone()]
                    .map(|path| std::fs::read(path).ok());
                (reference, claudine)
            };
            let before = files();
            let rows = run_with(file_checks(), &case.fixture.environment(), true);
            assert_eq!(files(), before, "{repository} {counterpart:?}");
            assert_row(&rows, "config", Severity::Error, "has a `permissions` reference to a file that does not exist");
        }
    }

    #[cfg(unix)]
    {
        let case = TargetCase::new(false, Counterpart::Valid, USER_DARKMATTER, Target::Missing);
        let repository_darkmatter = case.fixture.repo.join(DARKMATTER);
        std::os::unix::fs::symlink(&case.darkmatter, &repository_darkmatter).unwrap();
        let config = row(&case.fixture.rows(), "config").clone();
        assert_eq!(config.detail.matches("has a `permissions` reference").count(), 1, "{config:#?}");
        assert!(config.detail.contains(&format!("{}: is the user configuration", repository_darkmatter.display())), "{config:#?}");
    }
}

/// A reference naming the scope's own Claudine file says so, beside that
/// file's own finding.
#[test]
fn a_reference_to_an_unavailable_claudine_file_names_claudines_configuration() {
    for (claudine, own) in [(None, "is missing"), (Some("logging: [\n"), "is not one valid YAML mapping")] {
        let fixture = Fixture::new();
        let path = fixture.home.join(native_path(CLAUDINE));
        match claudine {
            None => std::fs::remove_file(&path).unwrap(),
            Some(text) => std::fs::write(&path, text).unwrap(),
        }
        let darkmatter = fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
        let config = row(&fixture.rows(), "config").clone();
        let what = if claudine.is_none() { "does not exist" } else { "does not supply a valid `permissions` mapping" };
        assert!(config.detail.contains(&format!("{}: {own}", path.display())), "{config:#?}");
        assert!(
            config.detail.contains(&format!(
                "{}: has a `permissions` reference to Claudine's configuration, which {what} (line 1, column 14)",
                darkmatter.display()
            )),
            "{config:#?}"
        );
    }
}

/// An invalid `register` setting and a broken target are independent
/// findings of the same file; neither hides the other.
#[test]
fn a_broken_target_and_an_invalid_register_are_both_reported() {
    for repository in [true, false] {
        let case = TargetCase::new(repository, Counterpart::Valid, &format!("{}register: 3\n", declaration(repository)), Target::Missing);
        let config = row(&case.fixture.rows(), "config").clone();
        let shown = case.darkmatter.display();
        assert!(config.detail.contains(&format!("{shown}: has a `permissions` reference to a file that does not exist")), "{config:#?}");
        assert!(config.detail.contains(&format!("{shown}: has an invalid value at `/register` (line 2, column 11)")), "{config:#?}");
    }
}
