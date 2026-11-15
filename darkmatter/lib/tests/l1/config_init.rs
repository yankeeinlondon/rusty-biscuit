//! `darkmatter::config::init`: what `md init` writes, through the public
//! API. Every case reads back what was written with the ordinary loader.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::init::{
    InitAction, InitError, InitOptions, default_permissions, init, init_repository, init_user, register_manifest,
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
