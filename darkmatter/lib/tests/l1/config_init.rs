//! `darkmatter::config::init`: what `md init` writes, through the public
//! API. Every case reads back what was written with the ordinary loader.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::init::{
    InitAction, InitError, InitOptions, check_referenced_permissions, default_permissions, init, init_repository,
    init_user, register_manifest, replace_manifest_registration, replace_manifest_registration_existing,
};
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot, PermissionsSetting};
use darkmatter::config::schema::{CONFIG_FILE_NAME, ConfigScope};
use serde_json::{Value, json};
use simplified_schema::yaml_documents::load_yaml_document;

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

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn user_file(&self) -> PathBuf {
        self.home().join(CONFIG_FILE_NAME)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    fn load(&self) -> ConfigSnapshot {
        ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive).unwrap()
    }

    /// Every file under the fixture, with its bytes.
    fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.sort();
        out
    }
}

fn document(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    load_yaml_document(&text, path).unwrap().json
}

#[test]
fn config_init_creates_the_user_file_with_every_default() {
    let fixture = Fixture::new();

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].action, InitAction::Created);
    assert_eq!(report.files[0].path, fixture.user_file());
    assert!(report.notices.is_empty());
    let written = document(&fixture.user_file());
    assert_eq!(written, json!({ "permissions": default_permissions(ConfigScope::User), "register": [] }));
    assert_eq!(written["permissions"]["network"], "deny");
    assert_eq!(written["permissions"]["shell-commands"], json!([]));
    assert_eq!(written["permissions"]["file-read"][0], "./*.txt");

    let snapshot = fixture.load();
    assert!(snapshot.diagnostics().is_empty(), "{:?}", snapshot.diagnostics());
    let blank = Fixture::new();
    std::fs::write(blank.user_file(), "{}\n").unwrap();
    let defaults = blank.load();
    let user = snapshot.policy().unwrap().user().permissions().clone();
    let expected = defaults.policy().unwrap().user().permissions().clone();
    assert_eq!(
        (user.file_read.value, user.shell_commands.value, user.network.value),
        (expected.file_read.value, expected.shell_commands.value, expected.network.value),
        "the written file means exactly the defaults"
    );
}

#[test]
fn config_init_repeated_writes_nothing() {
    let fixture = Fixture::new();
    init_user(&fixture.context(), InitOptions::default()).unwrap();
    let before = fixture.files();

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before, "bytes identical, no lock or temporary file left behind beyond the first run");
}

#[test]
fn config_init_preserves_authored_values_and_comments() {
    let fixture = Fixture::new();
    let authored = "# my settings\npermissions:\n  # mine\n  network: allow   # yes\n  shell-commands:\n    - prefix ls\n";
    fixture.write("home/.darkmatter.yaml", authored);

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Updated);
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.starts_with(authored), "every authored byte stays: {text}");
    assert_eq!(&text[authored.len()..], "register: []\n");
    let again = init_user(&fixture.context(), InitOptions::default()).unwrap();
    assert_eq!(again.files[0].action, InitAction::Unchanged);
}

#[test]
fn config_init_keeps_an_existing_reference() {
    let fixture = Fixture::new();
    fixture.write("home/.claudine/config.yaml", "permissions:\n  network: allow\n");
    let authored = "permissions: ~/.claudine/config.yaml\nregister: []\n";
    fixture.write("home/.darkmatter.yaml", authored);

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Unchanged);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), authored);
}

#[test]
fn config_init_inline_writes_the_referenced_block() {
    let fixture = Fixture::new();
    let target_block = "permissions:\n  network: allow\n  shell-commands:\n    - exact ps -e\n";
    fixture.write("home/.claudine/config.yaml", target_block);
    fixture.write("home/.darkmatter.yaml", "# keep me\npermissions: ~/.claudine/config.yaml\n");

    let report = init_user(&fixture.context(), InitOptions { inline: true }).unwrap();

    assert_eq!(report.files[0].action, InitAction::Updated);
    assert!(report.notices.is_empty(), "{:?}", report.notices);
    let written = document(&fixture.user_file());
    let target = document(&fixture.root.join("home/.claudine/config.yaml"));
    assert_eq!(written["permissions"], target["permissions"]);
    assert_eq!(written["register"], json!([]));
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.starts_with("# keep me\n"), "{text}");
    let snapshot = fixture.load();
    assert!(matches!(snapshot.permissions().unwrap().user, PermissionsSetting::Inline(_)));
    assert!(snapshot.diagnostics().is_empty(), "{:?}", snapshot.diagnostics());
}

#[test]
fn config_init_inline_with_a_broken_target_writes_defaults_and_says_so() {
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", "permissions: ~/.claudine/config.yaml\n");
    assert!(fixture.load().policy().is_err(), "the reference is broken before the repair");

    let report = init_user(&fixture.context(), InitOptions { inline: true }).unwrap();

    assert_eq!(report.files[0].action, InitAction::Updated);
    assert_eq!(report.notices.len(), 1);
    assert_eq!(report.notices[0].file, fixture.user_file());
    assert!(report.notices[0].message.contains("defaults") && report.notices[0].message.contains("may change"), "{:?}", report.notices);
    assert_eq!(document(&fixture.user_file())["permissions"], default_permissions(ConfigScope::User));
    assert!(fixture.load().policy().is_ok());
}

#[test]
fn config_init_ordinary_run_never_repairs_a_broken_reference() {
    let fixture = Fixture::new();
    let authored = "permissions: ~/.claudine/config.yaml\n";
    fixture.write("home/.darkmatter.yaml", authored);

    let report = init_user(&fixture.context(), InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Updated);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), format!("{authored}register: []\n"));
}

#[test]
fn config_init_refuses_an_invalid_file_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", "permissions:\n  network: sometimes\n");
    let before = fixture.files();

    let error = init_user(&fixture.context(), InitOptions { inline: true }).unwrap_err();

    assert!(matches!(error, InitError::Invalid(_)), "{error}");
    assert_eq!(error.code(), "dm.config.invalid");
    assert_eq!(fixture.files(), before);
}

#[test]
fn config_init_repository_outside_a_repository_writes_nothing() {
    let fixture = Fixture::new();
    let before = fixture.files();

    let error = init(&fixture.context(), &[ConfigScope::User, ConfigScope::Repository], InitOptions::default()).unwrap_err();

    assert!(matches!(error, InitError::NotARepository { .. }), "{error}");
    assert_eq!(error.code(), "dm.config.not_saved");
    assert_eq!(fixture.files(), before, "the user file is not written either");
}

#[test]
fn config_init_repository_writes_the_repository_defaults() {
    let fixture = Fixture::new();
    let root = fixture.root.join("repo");
    let context = fixture.context().with_repository_root(&root);

    let report = init_repository(&context, InitOptions::default()).unwrap();

    assert_eq!(report.files[0].action, InitAction::Created);
    assert_eq!(report.files[0].scope, ConfigScope::Repository);
    let written = document(&root.join(CONFIG_FILE_NAME));
    assert_eq!(written, json!({ "permissions": default_permissions(ConfigScope::Repository), "register": [] }));
    assert_eq!(written["permissions"]["file-read"][0], "&**/*.txt");
    assert!(!fixture.user_file().exists(), "only the scope asked for");
    let again = init_repository(&context, InitOptions::default()).unwrap();
    assert_eq!(again.files[0].action, InitAction::Unchanged);
}

#[test]
fn config_init_without_a_home_writes_nothing() {
    let fixture = Fixture::new();
    let context = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());
    let before = fixture.files();

    let error = init_user(&context, InitOptions::default()).unwrap_err();

    assert!(matches!(error, InitError::NoHome), "{error}");
    assert_eq!(fixture.files(), before);
}

#[test]
fn config_init_without_a_home_loading_warns_once_naming_no_path() {
    let fixture = Fixture::new();
    let context = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());

    let snapshot = ConfigSnapshot::load(&context, None, ConfigLoadMode::Passive).unwrap();

    let missing: Vec<_> = snapshot.diagnostics().iter().filter(|notice| notice.code == "dm.config.missing").collect();
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing[0].message.contains("no home directory"), "{}", missing[0].message);
    assert!(snapshot.policy().is_ok(), "user defaults still apply");
}

/// The ordering Claudine's handshake relies on: a `published_last` target is
/// replaced after every other one, even when it sorts first in lock order,
/// so an interruption between the two leaves it untouched.
#[cfg(feature = "effects-instrumentation")]
#[test]
fn config_init_a_published_last_target_waits_for_the_others() {
    use darkmatter::config::write::{ConfigTransaction, SourceExpectation, TargetChange, test_hooks};
    use darkmatter::yaml_edit::YamlEdit;
    use simplified_schema::PropertyPath;

    let fixture = Fixture::new();
    let first = fixture.write("work/a.yaml", "value: 1\n");
    let second = fixture.write("work/b.yaml", "value: 1\n");
    let set = || vec![YamlEdit::Set { path: PropertyPath::parse("value").unwrap(), value: json!(2) }];
    let mut transaction = ConfigTransaction::new();
    transaction.add(TargetChange::edit(&first, SourceExpectation::Current, set()).published_last()).unwrap();
    transaction.add(TargetChange::edit(&second, SourceExpectation::Current, set())).unwrap();
    assert_eq!(transaction.lock_order(), [first.as_path(), second.as_path()], "locks stay in path order");

    let error = {
        let _hook = test_hooks::fail_replacement_after(1);
        transaction.commit(&|_| Ok(())).unwrap_err()
    };

    assert!(error.to_string().contains("a.yaml"), "{error}");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "value: 2\n", "the other target went first");
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "value: 1\n", "the last one is untouched");
}

// ── register_manifest ───────────────────────────────────────────────────────

const MANIFEST_REFERENCE: &str = "~/.acme/darkmatter-host.yaml";

fn registered(fixture: &Fixture) -> Vec<String> {
    fixture.load().registrations().entries.iter().map(|entry| entry.reference.clone()).collect()
}

fn with_manifest(fixture: &Fixture) -> PathBuf {
    fixture.write("home/.acme/darkmatter-host.yaml", "kind: darkmatter-host\n")
}

#[test]
fn register_manifest_creates_a_missing_user_file_with_every_default_and_the_entry() {
    let fixture = Fixture::new();
    with_manifest(&fixture);
    let report = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    assert_eq!(report.action, InitAction::Created);
    assert_eq!(report.path, fixture.user_file());
    let written = document(&fixture.user_file());
    assert_eq!(written["register"], json!([MANIFEST_REFERENCE]));
    assert!(written.get("permissions").is_some(), "every default is written: {written}");
    assert_eq!(registered(&fixture), vec![MANIFEST_REFERENCE.to_string()]);
}

#[test]
fn register_manifest_appends_keeping_entries_values_and_comments() {
    let fixture = Fixture::new();
    with_manifest(&fixture);
    fixture.write("home/.other/darkmatter-host.yaml", "kind: darkmatter-host\n");
    let original = "# mine\nregister:\n    - ~/.other/darkmatter-host.yaml # in-house\npermissions:\n    network: deny\n";
    fixture.write("home/.darkmatter.yaml", original);
    let report = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    assert_eq!(report.action, InitAction::Updated);
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.starts_with("# mine\n"), "{text}");
    assert!(text.contains("~/.other/darkmatter-host.yaml # in-house"), "{text}");
    assert_eq!(
        registered(&fixture),
        vec!["~/.other/darkmatter-host.yaml".to_string(), MANIFEST_REFERENCE.to_string()]
    );
    assert_eq!(document(&fixture.user_file())["permissions"], json!({ "network": "deny" }));
}

#[test]
fn register_manifest_adds_a_list_to_a_file_without_one() {
    let fixture = Fixture::new();
    with_manifest(&fixture);
    fixture.write("home/.darkmatter.yaml", "permissions:\n    network: deny\n");
    register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    assert_eq!(registered(&fixture), vec![MANIFEST_REFERENCE.to_string()]);
}

#[test]
fn register_manifest_deduplicates_by_the_manifest_it_names_not_the_spelling() {
    let fixture = Fixture::new();
    let manifest = with_manifest(&fixture);
    let absolute = biscuit_file::to_portable_string(&manifest);
    let original = format!("register:\n    - {absolute}\n");
    fixture.write("home/.darkmatter.yaml", &original);
    let before = fixture.files();
    let report = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    assert_eq!(report.action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before, "nothing is written");
}

#[test]
fn register_manifest_repeated_writes_nothing() {
    let fixture = Fixture::new();
    with_manifest(&fixture);
    register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    let before = fixture.files();
    let report = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap();
    assert_eq!(report.action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before);
}

#[test]
fn register_manifest_refuses_a_missing_manifest_and_writes_nothing() {
    let fixture = Fixture::new();
    let error = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap_err();
    assert!(matches!(error, InitError::Invalid(_)), "{error}");
    assert!(error.to_string().contains(MANIFEST_REFERENCE), "{error}");
    assert!(!fixture.user_file().exists());
}

#[test]
fn register_manifest_refuses_a_register_value_that_is_not_a_list_and_writes_nothing() {
    let fixture = Fixture::new();
    with_manifest(&fixture);
    let original = "register: ~/.acme/darkmatter-host.yaml\n";
    fixture.write("home/.darkmatter.yaml", original);
    let error = register_manifest(&fixture.context(), ConfigScope::User, MANIFEST_REFERENCE).unwrap_err();
    assert!(matches!(error, InitError::Invalid(_)), "{error}");
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), original);
}

#[test]
fn register_manifest_without_a_home_writes_nothing() {
    let fixture = Fixture::new();
    let context = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());
    let error = register_manifest(&context, ConfigScope::User, MANIFEST_REFERENCE).unwrap_err();
    assert!(matches!(error, InitError::NoHome), "{error}");
}

// ── replace_manifest_registration ───────────────────────────────────────────

const NEW_REFERENCE: &str = "~/.acme/registration/manifest.yaml";

/// The retired manifest's path; the file itself is never created, as after
/// a host removed it.
fn old_manifest(fixture: &Fixture) -> PathBuf {
    fixture.home().join(".acme/darkmatter-host.yaml")
}

fn with_new_manifest(fixture: &Fixture) -> PathBuf {
    fixture.write("home/.acme/registration/manifest.yaml", "kind: darkmatter-host\n")
}

fn replace(fixture: &Fixture) -> Result<darkmatter::config::init::RegisterReport, InitError> {
    replace_manifest_registration(&fixture.context(), ConfigScope::User, NEW_REFERENCE, &old_manifest(fixture))
}

#[test]
fn replace_manifest_registration_creates_a_missing_user_file_with_the_entry() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    let report = replace(&fixture).unwrap();
    assert_eq!(report.action, InitAction::Created);
    assert_eq!(document(&fixture.user_file())["register"], json!([NEW_REFERENCE]));
    assert!(report.replaced.is_empty());
}

#[test]
fn replace_manifest_registration_adds_a_list_to_a_file_without_one() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.darkmatter.yaml", "# mine\npermissions:\n    network: deny\n");
    assert_eq!(replace(&fixture).unwrap().action, InitAction::Updated);
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.starts_with("# mine\npermissions:\n    network: deny\n"), "{text}");
    assert_eq!(registered(&fixture), vec![NEW_REFERENCE.to_string()]);
}

#[test]
fn replace_manifest_registration_appends_to_an_empty_list() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.darkmatter.yaml", "register: []\n");
    replace(&fixture).unwrap();
    assert_eq!(registered(&fixture), vec![NEW_REFERENCE.to_string()]);
}

#[test]
fn replace_manifest_registration_replaces_the_old_entry_in_place_keeping_every_other_byte() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.other/darkmatter-host.yaml", "kind: darkmatter-host\n");
    fixture.write("home/.third/manifest.yaml", "kind: darkmatter-host\n");
    let original = "# mine\nregister:\n    # in-house first\n    - ~/.other/darkmatter-host.yaml # kept\n    - ~/.acme/darkmatter-host.yaml\n    # the third\n    - ~/.third/manifest.yaml\npermissions:\n    network: deny\n";
    fixture.write("home/.darkmatter.yaml", original);

    let report = replace(&fixture).unwrap();

    assert_eq!(report.action, InitAction::Updated);
    assert_eq!(report.replaced, vec!["~/.acme/darkmatter-host.yaml".to_string()]);
    let expected = original.replace("~/.acme/darkmatter-host.yaml", NEW_REFERENCE);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), expected);
}

#[test]
fn replace_manifest_registration_matches_a_missing_old_manifest_in_every_spelling() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    let absolute = biscuit_file::to_portable_string(&old_manifest(&fixture));
    for spelling in [absolute.as_str(), "~/.acme/darkmatter-host.yaml", "./.acme/darkmatter-host.yaml"] {
        fixture.write("home/.darkmatter.yaml", &format!("register:\n    - {spelling}\n"));
        let report = replace(&fixture).unwrap();
        assert_eq!(report.replaced, vec![spelling.to_string()], "{spelling}");
        assert_eq!(registered(&fixture), vec![NEW_REFERENCE.to_string()], "{spelling}");
    }
}

#[test]
fn replace_manifest_registration_leaves_one_canonical_entry_at_the_first_affected_index() {
    let fixture = Fixture::new();
    let new = with_new_manifest(&fixture);
    fixture.write("home/.other/darkmatter-host.yaml", "kind: darkmatter-host\n");
    let new_absolute = biscuit_file::to_portable_string(&new);
    let original = format!(
        "register:\n    - ~/.other/darkmatter-host.yaml\n    - {new_absolute}\n    - ~/.acme/darkmatter-host.yaml\n    - {NEW_REFERENCE}\n    - ~/.acme/darkmatter-host.yaml\n"
    );
    fixture.write("home/.darkmatter.yaml", &original);

    let report = replace(&fixture).unwrap();

    assert_eq!(
        registered(&fixture),
        vec!["~/.other/darkmatter-host.yaml".to_string(), NEW_REFERENCE.to_string()]
    );
    assert_eq!(report.replaced.len(), 4, "{:?}", report.replaced);
    // Running it again finds nothing to do.
    let before = fixture.files();
    assert_eq!(replace(&fixture).unwrap().action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before);
}

#[test]
fn replace_manifest_registration_writes_nothing_when_the_new_entry_is_listed() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.darkmatter.yaml", &format!("register:\n    - {NEW_REFERENCE}\n"));
    let before = fixture.files();
    let report = replace(&fixture).unwrap();
    assert_eq!(report.action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before);
}

#[test]
fn replace_manifest_registration_refuses_a_malformed_file_with_its_location_and_writes_nothing() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    let original = "register:\n    - ~/.acme/darkmatter-host.yaml\npermissions: [unclosed\n";
    fixture.write("home/.darkmatter.yaml", original);
    let error = replace(&fixture).unwrap_err();
    assert!(matches!(error, InitError::Invalid(_)), "{error}");
    let message = error.to_string();
    assert!(message.contains(":3:") || message.contains("line 3"), "the error names its line: {message}");
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), original);
}

#[test]
fn replace_manifest_registration_refuses_a_file_changed_since_it_was_read() {
    use darkmatter::config::init::{commit_checked, observe, replace_manifest_change};
    use darkmatter::config::write::{ConfigTransaction, ConfigWriteError};

    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.darkmatter.yaml", "register:\n    - ~/.acme/darkmatter-host.yaml\n");
    let context = fixture.context();
    let observation = observe(&context, ConfigScope::User).unwrap();
    let (change, _) = replace_manifest_change(&observation, &context, NEW_REFERENCE, &old_manifest(&fixture)).unwrap();
    let concurrent = "register:\n    - ~/.acme/darkmatter-host.yaml\n# edited meanwhile\n";
    fixture.write("home/.darkmatter.yaml", concurrent);

    let mut transaction = ConfigTransaction::new();
    transaction.add(change.expect("the old entry is replaced")).unwrap();
    let user_file = fixture.user_file();
    let error = commit_checked(transaction, &context, &[(user_file.as_path(), ConfigScope::User)], &|_| None).unwrap_err();

    assert!(matches!(error, InitError::Write(ConfigWriteError::Changed { .. })), "{error}");
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), concurrent);
}

// ── replace_manifest_registration_existing ──────────────────────────────────

fn replace_existing(fixture: &Fixture) -> Result<darkmatter::config::init::RegisterReport, InitError> {
    replace_manifest_registration_existing(&fixture.context(), ConfigScope::User, NEW_REFERENCE, &old_manifest(fixture))
}

#[test]
fn replace_manifest_registration_existing_never_creates_the_file() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    let error = replace_existing(&fixture).unwrap_err();
    assert!(
        matches!(&error, InitError::Read { path, source } if *path == fixture.user_file() && source.kind() == std::io::ErrorKind::NotFound),
        "{error:?}"
    );
    assert!(!fixture.user_file().exists());
}

#[test]
fn replace_manifest_registration_existing_edits_in_place_and_converges() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    fixture.write("home/.other/darkmatter-host.yaml", "kind: darkmatter-host\n");
    let original = "# mine\nregister:\n    - ~/.other/darkmatter-host.yaml # kept\n    - ~/.acme/darkmatter-host.yaml\n";
    fixture.write("home/.darkmatter.yaml", original);

    let report = replace_existing(&fixture).unwrap();
    assert_eq!(report.action, InitAction::Updated);
    assert_eq!(report.replaced, vec!["~/.acme/darkmatter-host.yaml".to_string()]);
    let expected = original.replace("~/.acme/darkmatter-host.yaml", NEW_REFERENCE);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), expected);

    let again = replace_existing(&fixture).unwrap();
    assert_eq!(again.action, InitAction::Unchanged);
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), expected);
}

#[test]
fn replace_manifest_registration_existing_refuses_an_invalid_register_and_writes_nothing() {
    let fixture = Fixture::new();
    with_new_manifest(&fixture);
    let original = "register: 7\n";
    fixture.write("home/.darkmatter.yaml", original);
    assert!(matches!(replace_existing(&fixture), Err(InitError::Invalid(_))));
    assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), original);
}

// ── replace_manifest_refusal ────────────────────────────────────────────────

/// One `~/.darkmatter.yaml` layout and whether the registration edit it
/// needs meets a layout obstacle.
struct LayoutCase {
    name: &'static str,
    text: &'static str,
    /// `None`: the edit applies, or none is needed. `Some(None)`: refused at
    /// the root. `Some(Some(path))`: refused at the edit of `path`.
    refused: Option<Option<&'static str>>,
}

const fn case(name: &'static str, text: &'static str, refused: Option<Option<&'static str>>) -> LayoutCase {
    LayoutCase { name, text, refused }
}

/// The instance and control rows of review finding R4-F2 for registration.
const LAYOUT_CASES: &[LayoutCase] = &[
    // R4-F2.1: insertion, replacement, and duplicate removal in an inline list,
    // and the same edits in block lists.
    case("inline list, other host: append", "register: [~/.other/darkmatter-host.yaml]\n", Some(Some("register.1"))),
    case(
        "inline list, retired entry: replace",
        "register: [~/.other/darkmatter-host.yaml, ~/.acme/darkmatter-host.yaml]\n",
        Some(Some("register.1")),
    ),
    case(
        "inline list, current and retired: remove",
        "register: [~/.acme/registration/manifest.yaml, ~/.acme/darkmatter-host.yaml]\n",
        Some(Some("register.1")),
    ),
    case("block list, other host: append", "register:\n    - ~/.other/darkmatter-host.yaml\n", None),
    case(
        "block list, retired entry: replace",
        "register:\n    - ~/.other/darkmatter-host.yaml\n    - ~/.acme/darkmatter-host.yaml\n",
        None,
    ),
    case(
        "block list, current and retired: remove",
        "register:\n    - ~/.acme/registration/manifest.yaml\n    - ~/.acme/darkmatter-host.yaml\n",
        None,
    ),
    // R4-F2.2: an inline root mapping refuses even an empty list's edit.
    case("inline root, empty list", "{register: [], permissions: {network: deny}}\n", Some(None)),
    case("block root, empty list", "register: []\npermissions: {network: deny}\n", None),
    // R4-F2.c1: forms the edit replaces whole, and an inline list that needs no edit.
    case("omitted register", "permissions:\n    network: deny\n", None),
    case("null register", "register:\n", None),
    case(
        "inline list already current",
        "register: [~/.other/darkmatter-host.yaml, ~/.acme/registration/manifest.yaml]\n",
        None,
    ),
    // R4-F2.x1: an anchor on the list is the same layout class.
    case("anchored block list", "register: &hosts\n    - ~/.other/darkmatter-host.yaml\n", Some(Some("register.1"))),
];

/// The dry run is judged before the manifest is published and agrees with
/// the writer once it is: a refused layout fails the edit and keeps every
/// byte; an accepted one is edited (or, needing no edit, left as it was).
#[test]
fn replace_manifest_refusal_predicts_the_registration_edit_before_publication() {
    use darkmatter::config::init::{observe, replace_manifest_refusal};
    use darkmatter::config::write::ConfigWriteError;

    for case in LAYOUT_CASES {
        let fixture = Fixture::new();
        fixture.write("home/.other/darkmatter-host.yaml", "kind: darkmatter-host\n");
        fixture.write("home/.darkmatter.yaml", case.text);
        let context = fixture.context();
        let observation = observe(&context, ConfigScope::User).unwrap();

        let refusal = replace_manifest_refusal(&observation, &context, NEW_REFERENCE, &old_manifest(&fixture)).unwrap();
        let attributed = refusal.map(|refusal| refusal.path.map(|path| path.to_string()));
        assert_eq!(attributed, case.refused.map(|path| path.map(str::to_string)), "{}", case.name);

        with_new_manifest(&fixture);
        let written = replace_existing(&fixture);
        if case.refused.is_some() {
            assert!(
                matches!(written, Err(InitError::Write(ConfigWriteError::UnsupportedEdit { .. }))),
                "{}: {written:?}",
                case.name
            );
            assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), case.text, "{}", case.name);
        } else {
            let report = written.unwrap_or_else(|error| panic!("{}: {error}", case.name));
            let entries = registered(&fixture);
            assert!(entries.iter().any(|entry| entry == NEW_REFERENCE), "{}: {entries:?}", case.name);
            assert_eq!(
                entries.iter().any(|entry| entry == "~/.other/darkmatter-host.yaml"),
                case.text.contains("~/.other/"),
                "{}: other hosts are kept",
                case.name
            );
            if report.action == InitAction::Unchanged {
                assert_eq!(std::fs::read_to_string(fixture.user_file()).unwrap(), case.text, "{}", case.name);
            }
        }
    }
}

// ── an explicitly scoped write keeps its scope ──────────────────────────────

/// How the repository the context names relates to the user file.
#[derive(Debug, Clone, Copy)]
enum UserAlias {
    /// `{repo}/.darkmatter.yaml` is a symbolic link to `~/.darkmatter.yaml`.
    #[cfg(unix)]
    LinkedRepositoryFile,
    /// The home is the repository root.
    HomeIsRepository,
}

fn user_aliases() -> Vec<UserAlias> {
    let mut aliases = vec![UserAlias::HomeIsRepository];
    #[cfg(unix)]
    aliases.push(UserAlias::LinkedRepositoryFile);
    aliases
}

/// A user-only grant (a home reference a repository file may not write)
/// and a policy both scopes accept.
const USER_ONLY_POLICY: &str = "permissions:\n    file-read:\n        - ~/notes/*.md\n";
const SHARED_POLICY: &str = "permissions:\n    network: deny\n";

/// Writes the user file with another host's entry, a comment, and `policy`,
/// lays out `alias`, and returns a context launched in the repository and
/// the user file's original text.
fn aliased_user_file(fixture: &Fixture, alias: UserAlias, policy: &str) -> (FileResolutionContext, String) {
    fixture.write("home/.other/manifest.yaml", "kind: darkmatter-host\n");
    let original = format!("register:\n    # another host\n    - ~/.other/manifest.yaml\n{policy}");
    fixture.write("home/.darkmatter.yaml", &original);
    let root = match alias {
        #[cfg(unix)]
        UserAlias::LinkedRepositoryFile => {
            let root = fixture.root.join("repo");
            std::os::unix::fs::symlink(fixture.user_file(), root.join(CONFIG_FILE_NAME)).unwrap();
            root
        }
        UserAlias::HomeIsRepository => fixture.home(),
    };
    // Launched inside the repository, as a user working there would be.
    let context = FileResolutionContext::from_snapshot(root.clone(), Some(fixture.home()), HashMap::new())
        .with_repository_root(&root);
    (context, original)
}

fn assert_user_entry_added(fixture: &Fixture, original: &str, reference: &str, case: &str) {
    let text = std::fs::read_to_string(fixture.user_file()).unwrap();
    assert!(text.contains("    # another host\n    - ~/.other/manifest.yaml\n"), "{case}: comment and other host kept: {text}");
    assert!(text.contains(original.split("register:").nth(1).unwrap().split("permissions:").nth(1).unwrap()), "{case}: policy kept: {text}");
    assert_eq!(registered(fixture), vec!["~/.other/manifest.yaml".to_string(), reference.to_string()], "{case}");
}

#[test]
fn scoped_registration_writes_judge_an_aliased_user_file_as_the_user_file() {
    for alias in user_aliases() {
        for policy in [USER_ONLY_POLICY, SHARED_POLICY] {
            let case = format!("{alias:?}, {policy:?}");

            let fixture = Fixture::new();
            with_manifest(&fixture);
            let (context, original) = aliased_user_file(&fixture, alias, policy);
            let report = register_manifest(&context, ConfigScope::User, MANIFEST_REFERENCE)
                .unwrap_or_else(|error| panic!("register_manifest, {case}: {error}"));
            assert_eq!(report.action, InitAction::Updated, "{case}");
            assert_user_entry_added(&fixture, &original, MANIFEST_REFERENCE, &case);

            let fixture = Fixture::new();
            with_new_manifest(&fixture);
            let (context, original) = aliased_user_file(&fixture, alias, policy);
            replace_manifest_registration(&context, ConfigScope::User, NEW_REFERENCE, &old_manifest(&fixture))
                .unwrap_or_else(|error| panic!("replace_manifest_registration, {case}: {error}"));
            assert_user_entry_added(&fixture, &original, NEW_REFERENCE, &case);

            let fixture = Fixture::new();
            with_new_manifest(&fixture);
            let (context, original) = aliased_user_file(&fixture, alias, policy);
            replace_manifest_registration_existing(&context, ConfigScope::User, NEW_REFERENCE, &old_manifest(&fixture))
                .unwrap_or_else(|error| panic!("replace_manifest_registration_existing, {case}: {error}"));
            assert_user_entry_added(&fixture, &original, NEW_REFERENCE, &case);
        }
    }
}

#[test]
fn init_user_completes_an_aliased_user_file_as_the_user_file() {
    for alias in user_aliases() {
        let fixture = Fixture::new();
        let (context, _) = aliased_user_file(&fixture, alias, USER_ONLY_POLICY);
        // Without `register`, so `init` has a missing setting to add.
        let original = USER_ONLY_POLICY;
        fixture.write("home/.darkmatter.yaml", original);
        let report = init(&context, &[ConfigScope::User], InitOptions::default())
            .unwrap_or_else(|error| panic!("{alias:?}: {error}"));
        assert_eq!(report.files[0].action, InitAction::Updated, "{alias:?}");
        let text = std::fs::read_to_string(fixture.user_file()).unwrap();
        assert!(text.starts_with(original), "{alias:?}: only missing settings are added: {text}");
    }
}

#[test]
fn commit_checked_judges_each_target_by_the_scope_its_caller_names() {
    use darkmatter::config::init::commit_checked;
    use darkmatter::config::write::{ConfigTransaction, ConfigWriteError, SourceExpectation, TargetChange};
    use darkmatter::yaml_edit::YamlEdit;
    use simplified_schema::PropertyPath;

    // One file at the repository root, edited to hold a user-only grant: the
    // caller's scope decides, never the file's location.
    let fixture = Fixture::new();
    let original = "register: []\n";
    let file = fixture.write("repo/.darkmatter.yaml", original);
    let root = fixture.root.join("repo");
    let context = FileResolutionContext::from_snapshot(root.clone(), Some(fixture.home()), HashMap::new())
        .with_repository_root(&root);
    let commit = |scope: ConfigScope| {
        let edit = YamlEdit::Set {
            path: PropertyPath::from_segments(["permissions"]).unwrap(),
            value: json!({ "file-read": ["~/notes/*.md"] }),
        };
        let mut transaction = ConfigTransaction::new();
        transaction
            .add(TargetChange::edit(&file, SourceExpectation::Bytes(original.as_bytes().to_vec()), vec![edit]))
            .unwrap();
        commit_checked(transaction, &context, &[(file.as_path(), scope)], &|_| None)
    };

    let error = commit(ConfigScope::Repository).unwrap_err();
    assert!(matches!(error, InitError::Write(ConfigWriteError::ValidationFailed { .. })), "{error}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original, "a refused write changes nothing");

    commit(ConfigScope::User).unwrap();
    assert_eq!(document(&file)["permissions"]["file-read"], json!(["~/notes/*.md"]));
}

// ── check_referenced_permissions ────────────────────────────────────────────

#[test]
fn check_referenced_permissions_needs_a_valid_mapping_of_the_scope() {
    let fixture = Fixture::new();
    let path = fixture.home().join(".acme/config.yaml");
    let context = fixture.context();
    let check = |text: &str| check_referenced_permissions(text, &path, ConfigScope::User, &context);
    assert_eq!(check("other: 1\npermissions: {}\n"), Ok(()));
    assert_eq!(check("permissions:\n    network: deny\n"), Ok(()));
    for invalid in ["other: 1\n", "permissions: null\n", "permissions: ~/.elsewhere.yaml\n", "permissions:\n    network: 7\n"] {
        assert!(check(invalid).is_err(), "{invalid:?}");
    }
}
