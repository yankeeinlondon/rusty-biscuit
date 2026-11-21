//! `darkmatter::config::migration` through the public load: a
//! `ConfigLoadMode::MigrateLegacy` load converts legacy policy files into
//! missing `permissions` properties, saves them through the locked writer,
//! and deletes the sources; a passive load only reports them. Every test
//! uses a temporary home and repository, never the process directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{
    CONFIG_MIGRATION_CODE, CONFIG_MISSING_CODE, ConfigDiagnostic, ConfigErrorKind, ConfigLoadMode, ConfigSnapshot,
    PermissionsSetting, ScopePermissions,
};
use darkmatter::config::migration::{LEGACY_BLACKLIST_FILE_NAME, LEGACY_WHITELIST_FILE_NAME};
use darkmatter::config::migration_receipt::{MigrationSource, RECEIPT_SUFFIX};
use darkmatter::config::schema::CONFIG_FILE_NAME;
use darkmatter::config::write::{ConfigTransaction, TargetChange};
use darkmatter::yaml_edit::YamlEdit;
use simplified_schema::PropertyPath;
use sniff::filesystem::git::{CommittedTreeSnapshot, GitRepo};

/// A home and a working directory under one temporary directory.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn user_file(&self) -> PathBuf {
        self.home().join(CONFIG_FILE_NAME)
    }

    fn whitelist(&self) -> PathBuf {
        self.home().join(LEGACY_WHITELIST_FILE_NAME)
    }

    fn blacklist(&self) -> PathBuf {
        self.home().join(LEGACY_BLACKLIST_FILE_NAME)
    }

    fn grant_store(&self) -> PathBuf {
        self.home().join(".config").join("darkmatter").join("schema-read-policy")
    }

    fn receipt(&self) -> PathBuf {
        self.home().join(format!("{CONFIG_FILE_NAME}{RECEIPT_SUFFIX}"))
    }

    fn write(&self, path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    fn migrate(&self) -> ConfigSnapshot {
        ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::MigrateLegacy).expect("the load succeeds")
    }

    fn user_text(&self) -> Option<String> {
        std::fs::read_to_string(self.user_file()).ok()
    }
}

fn inline(setting: &PermissionsSetting) -> &ScopePermissions {
    match setting {
        PermissionsSetting::Inline(block) => block,
        PermissionsSetting::Reference(reference) => panic!("expected an inline block, got {reference:?}"),
    }
}

fn user_block(snapshot: &ConfigSnapshot) -> ScopePermissions {
    inline(&snapshot.permissions().expect("valid permissions").user).clone()
}

fn shell_entries(block: &ScopePermissions) -> Vec<&str> {
    block.shell_commands.value.iter().map(|entry| entry.authored.as_str()).collect()
}

fn migration_notices(snapshot: &ConfigSnapshot) -> Vec<&ConfigDiagnostic> {
    snapshot.diagnostics().iter().filter(|notice| notice.code == CONFIG_MIGRATION_CODE).collect()
}

fn any_notice(snapshot: &ConfigSnapshot, needle: &str) -> bool {
    migration_notices(snapshot).iter().any(|notice| notice.message.contains(needle))
}

#[test]
fn a_whitelist_and_blacklist_migrate_together_as_one_shell_unit() {
    let fixture = Fixture::new();
    fixture.write(&fixture.user_file(), "# mine\nregister: []\n");
    fixture.write(&fixture.whitelist(), "# approved\nprefix ls\n\nexact ps -e\nprefix ls\n");
    fixture.write(&fixture.blacklist(), "prefix rm\n");

    let snapshot = fixture.migrate();

    let block = user_block(&snapshot);
    assert!(block.shell_commands.authored);
    assert_eq!(shell_entries(&block), ["prefix ls", "exact ps -e", "!prefix rm"]);
    assert!(!block.file_read.authored, "only the shell unit was migrated");
    let text = fixture.user_text().unwrap();
    assert!(text.starts_with("# mine\nregister: []\n"), "comments and other keys are kept: {text}");
    assert!(!fixture.whitelist().exists() && !fixture.blacklist().exists());
    assert!(!fixture.receipt().exists(), "a finished cleanup removes its receipt");
    assert!(any_notice(&snapshot, "now prompts instead of being refused"));
    let rm = migration_notices(&snapshot)
        .into_iter()
        .find(|notice| notice.message.contains("`prefix rm`"))
        .expect("a notice per blacklist rule");
    assert_eq!((rm.file.as_path(), rm.problems[0].line), (fixture.blacklist().as_path(), Some(1)));
    assert_eq!(
        migration_notices(&snapshot).iter().filter(|notice| notice.message.contains("and deleted it")).count(),
        2,
        "one notice per migrated file"
    );
}

#[test]
fn a_missing_user_file_is_created_with_only_the_migrated_keys() {
    let fixture = Fixture::new();
    fixture.write(&fixture.whitelist(), "prefix git status\n");

    let snapshot = fixture.migrate();

    assert_eq!(fixture.user_text().unwrap(), "permissions:\n    shell-commands:\n        - prefix git status\n");
    assert!(snapshot.user_config_missing().is_none());
    assert!(snapshot.diagnostics().iter().all(|notice| notice.code != CONFIG_MISSING_CODE));
    assert_eq!(shell_entries(&user_block(&snapshot)), ["prefix git status"]);
}

#[test]
fn an_authored_empty_property_is_never_overwritten_and_its_source_is_kept() {
    let fixture = Fixture::new();
    let authored = "permissions:\n  shell-commands: []\n";
    fixture.write(&fixture.user_file(), authored);
    fixture.write(&fixture.whitelist(), "prefix ls\n");

    let snapshot = fixture.migrate();

    assert_eq!(fixture.user_text().unwrap(), authored);
    assert!(fixture.whitelist().exists());
    assert!(shell_entries(&user_block(&snapshot)).is_empty());
    assert!(any_notice(&snapshot, "is ignored because"));
    assert!(!fixture.receipt().exists());
}

#[test]
fn grant_store_lines_become_home_portable_file_read_entries_after_the_defaults() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let outside = fixture.root.join("outside.yaml");
    let store = [
        "# grants".to_string(),
        format!("allow file {}", home.join("x").join("a[1].yaml").display()),
        format!("allow dir {}", home.join("notes").display()),
        format!("deny file {}", home.join("x").join("secret.yaml").display()),
        format!("deny dir {}", home.join("notes").join("private").display()),
        "allow url https://example.com/types.yaml".to_string(),
        "allow site https://schemas.example.com".to_string(),
        format!("allow file {}", outside.display()),
    ]
    .join("\n");
    fixture.write(&fixture.grant_store(), &store);

    let snapshot = fixture.migrate();

    let block = user_block(&snapshot);
    assert!(block.file_read.authored);
    let entries: Vec<&str> = block.file_read.value.iter().map(|entry| entry.authored.as_str()).collect();
    assert_eq!(entries.len(), 14, "{entries:?}");
    assert_eq!(entries[0], "./*.txt", "the schema default list comes first");
    assert_eq!(&entries[10..], ["~/x/a[[]1[]].yaml", "~/notes/**", "!~/x/secret.yaml", "!~/notes/private/**"]);
    assert!(!fixture.grant_store().exists());
    assert!(fixture.grant_store().parent().unwrap().is_dir(), "only the file is deleted");

    let dropped: Vec<Option<usize>> = migration_notices(&snapshot)
        .iter()
        .filter(|notice| notice.message.contains("is dropped"))
        .map(|notice| notice.problems[0].line)
        .collect();
    assert_eq!(dropped, [Some(6), Some(7), Some(8)], "the URL, site, and outside-home lines");
}

#[test]
fn an_invalid_legacy_line_is_a_located_error_and_nothing_is_written() {
    let fixture = Fixture::new();
    fixture.write(&fixture.whitelist(), "prefix ls\nallow everything\n");
    fixture.write(&fixture.blacklist(), "prefix rm\n");

    let error = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::MigrateLegacy).unwrap_err();

    assert_eq!(error.kind(), ConfigErrorKind::Migration);
    assert_eq!(error.code(), CONFIG_MIGRATION_CODE);
    let problem = &error.problems()[0];
    assert_eq!((problem.file.as_path(), problem.line), (fixture.whitelist().as_path(), Some(2)));
    assert!(fixture.user_text().is_none());
    assert!(fixture.whitelist().exists() && fixture.blacklist().exists());
    assert!(!fixture.receipt().exists());
}

#[test]
fn an_invalid_grant_store_line_is_a_located_error_and_nothing_is_written() {
    let fixture = Fixture::new();
    fixture.write(&fixture.user_file(), "{}\n");
    fixture.write(&fixture.grant_store(), "allow file relative/path.yaml\n");

    let error = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::MigrateLegacy).unwrap_err();

    assert_eq!(error.kind(), ConfigErrorKind::Migration);
    assert_eq!(error.problems()[0].line, Some(1));
    assert_eq!(fixture.user_text().unwrap(), "{}\n");
    assert!(fixture.grant_store().exists());
}

#[test]
fn a_passive_load_writes_nothing_and_reports_the_legacy_file() {
    let fixture = Fixture::new();
    fixture.write(&fixture.whitelist(), "prefix ls\n");

    let snapshot = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::Passive).unwrap();

    assert!(fixture.user_text().is_none());
    assert!(fixture.whitelist().exists());
    let notices = migration_notices(&snapshot);
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].file, fixture.whitelist());
    assert!(notices[0].message.contains("not in use"), "{}", notices[0].message);
}

#[test]
fn a_repeated_migration_is_a_no_op_and_never_duplicates_entries() {
    let fixture = Fixture::new();
    fixture.write(&fixture.whitelist(), "prefix ls\n");
    fixture.migrate();
    let migrated = fixture.user_text().unwrap();

    let repeat = fixture.migrate();
    assert!(migration_notices(&repeat).is_empty(), "{:?}", repeat.diagnostics());
    assert_eq!(fixture.user_text().unwrap(), migrated);

    // A legacy file written again after the migration is not merged in.
    fixture.write(&fixture.whitelist(), "prefix ls\nprefix cat\n");
    let again = fixture.migrate();
    assert_eq!(fixture.user_text().unwrap(), migrated);
    assert_eq!(shell_entries(&user_block(&again)), ["prefix ls"]);
    assert!(fixture.whitelist().exists());
}

/// Publishes the migrated whitelist through the writer with its receipt, as
/// a migration interrupted before cleanup leaves it.
fn publish_without_cleanup(fixture: &Fixture, whitelist: &str) {
    fixture.write(&fixture.whitelist(), whitelist);
    let set = YamlEdit::Set {
        path: PropertyPath::parse("permissions.shell-commands").unwrap(),
        value: serde_json::json!(["prefix ls"]),
    };
    let mut transaction = ConfigTransaction::new();
    transaction
        .add(
            TargetChange::create(fixture.user_file(), "{}\n", vec![set])
                .with_migration(vec![MigrationSource::delete(fixture.whitelist(), whitelist.as_bytes())]),
        )
        .unwrap();
    transaction.commit(&|_| Ok(())).unwrap();
}

#[test]
fn a_crash_between_publication_and_cleanup_is_finished_by_the_next_load() {
    let fixture = Fixture::new();
    publish_without_cleanup(&fixture, "prefix ls\n");
    let published = fixture.user_text().unwrap();
    assert!(fixture.receipt().exists() && fixture.whitelist().exists());

    let snapshot = fixture.migrate();

    assert!(!fixture.whitelist().exists());
    assert!(!fixture.receipt().exists());
    assert_eq!(fixture.user_text().unwrap(), published, "the published file is not migrated into again");
    assert_eq!(shell_entries(&user_block(&snapshot)), ["prefix ls"]);
    assert!(any_notice(&snapshot, "finished an interrupted migration"));
}

#[test]
fn a_source_changed_after_its_receipt_is_kept_with_a_notice() {
    let fixture = Fixture::new();
    publish_without_cleanup(&fixture, "prefix ls\n");
    fixture.write(&fixture.whitelist(), "prefix ls\nprefix cat\n");

    let snapshot = fixture.migrate();

    assert_eq!(std::fs::read_to_string(fixture.whitelist()).unwrap(), "prefix ls\nprefix cat\n");
    assert!(fixture.receipt().exists(), "the cleanup is still pending");
    let notice = migration_notices(&snapshot)
        .into_iter()
        .find(|notice| notice.message.contains("not cleaned up"))
        .expect("a notice naming the pending cleanup");
    assert!(notice.message.contains("changed after it was migrated"), "{}", notice.message);
}

#[cfg(feature = "effects-instrumentation")]
#[test]
fn a_failed_save_keeps_every_source_and_the_next_load_retries() {
    let fixture = Fixture::new();
    fixture.write(&fixture.user_file(), "{}\n");
    fixture.write(&fixture.whitelist(), "prefix ls\n");
    {
        let _failure = darkmatter::config::write::test_hooks::fail_next_replacement();
        let snapshot = fixture.migrate();
        assert!(any_notice(&snapshot, "could not be migrated"));
        assert_eq!(fixture.user_text().unwrap(), "{}\n");
        assert!(fixture.whitelist().exists());
        assert!(!user_block(&snapshot).shell_commands.authored);
    }

    let retried = fixture.migrate();
    assert_eq!(shell_entries(&user_block(&retried)), ["prefix ls"]);
    assert!(!fixture.whitelist().exists());
    assert!(!fixture.receipt().exists());
}

#[test]
fn a_document_directory_policy_file_is_never_migrated() {
    let fixture = Fixture::new();
    let docs = fixture.root.join("work").join("docs");
    let local = docs.join(LEGACY_WHITELIST_FILE_NAME);
    fixture.write(&local, "prefix ls\n");
    let context = FileResolutionContext::from_snapshot(&docs, Some(fixture.home()), HashMap::new());

    let snapshot = ConfigSnapshot::load(&context, None, ConfigLoadMode::MigrateLegacy).unwrap();

    assert!(local.exists());
    assert!(fixture.user_text().is_none());
    assert!(migration_notices(&snapshot).is_empty());
}

#[test]
fn a_permissions_reference_is_not_migrated_into() {
    let fixture = Fixture::new();
    let authored = "permissions: ~/.claudine/config.yaml\n";
    fixture.write(&fixture.user_file(), authored);
    fixture.write(&fixture.whitelist(), "prefix ls\n");

    let snapshot = fixture.migrate();

    assert_eq!(fixture.user_text().unwrap(), authored);
    assert!(fixture.whitelist().exists());
    assert!(any_notice(&snapshot, "refers to `~/.claudine/config.yaml`"));
}

#[test]
fn without_a_home_the_user_scope_is_skipped() {
    let fixture = Fixture::new();
    fixture.write(&fixture.whitelist(), "prefix ls\n");
    let context = FileResolutionContext::from_snapshot(fixture.root.join("work"), None, HashMap::new());

    let snapshot = ConfigSnapshot::load(&context, None, ConfigLoadMode::MigrateLegacy).unwrap();

    assert!(migration_notices(&snapshot).is_empty());
    assert!(fixture.whitelist().exists());
}

/// `work` as a repository with no commit, and its captured committed tree.
fn repository(fixture: &Fixture) -> (PathBuf, CommittedTreeSnapshot) {
    let root = fixture.root.join("work");
    git2::Repository::init(&root).unwrap();
    let snapshot = GitRepo::discover(&root).unwrap().expect("a repository").committed_tree().unwrap();
    (root, snapshot)
}

#[test]
fn a_repository_whitelist_migrates_into_the_working_copy_file() {
    let fixture = Fixture::new();
    fixture.write(&fixture.user_file(), "{}\n");
    let (root, tree) = repository(&fixture);
    let whitelist = root.join(LEGACY_WHITELIST_FILE_NAME);
    fixture.write(&whitelist, "exact make test\n");
    let context = fixture.context().with_repository_root(&root);

    let snapshot = ConfigSnapshot::load(&context, Some(&tree), ConfigLoadMode::MigrateLegacy).unwrap();

    assert!(!whitelist.exists());
    assert_eq!(
        std::fs::read_to_string(root.join(CONFIG_FILE_NAME)).unwrap(),
        "permissions:\n    shell-commands:\n        - exact make test\n"
    );
    assert_eq!(fixture.user_text().unwrap(), "{}\n", "the user file is untouched");
    let repository = snapshot.permissions().unwrap().repository.as_ref().expect("a repository");
    assert_eq!(shell_entries(inline(&repository.worktree)), ["exact make test"]);
    assert!(shell_entries(inline(&repository.head)).is_empty(), "HEAD has no file");
    assert!(any_notice(&snapshot, "once the file is committed"));
}
