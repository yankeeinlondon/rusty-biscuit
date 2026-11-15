//! `darkmatter::config::load`: whole-file loading of `.darkmatter.yaml`
//! through the public `ConfigSnapshot` API. The document-loader cells of the
//! configuration input-robustness matrix, the missing user file, dependency
//! records, and the repository's `HEAD` and working-copy projections. Field
//! cells are in `config_fields.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{
    CONFIG_INVALID_CODE, CONFIG_MISSING_CODE, ConfigError, ConfigErrorKind, ConfigLoadMode, ConfigRevision,
    ConfigSnapshot, NetworkAccess, PermissionsSetting, ScopePermissions,
};
use darkmatter::config::migration_receipt::content_hash;
use darkmatter::config::schema::CONFIG_FILE_NAME;
use sniff::filesystem::git::{CommittedTreeSnapshot, GitRepo};

/// A home and a working directory under one temporary directory; nothing is
/// read from the process.
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

    fn write_user(&self, bytes: impl AsRef<[u8]>) {
        std::fs::write(self.user_file(), bytes).unwrap();
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    fn load(&self) -> Result<ConfigSnapshot, ConfigError> {
        ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive)
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

/// The whole-file error a text fails with, and its first problem's
/// position.
fn whole_file_error(text: &str) -> (ConfigErrorKind, Option<usize>, Option<usize>, String) {
    let fixture = Fixture::new();
    fixture.write_user(text);
    let error = fixture.load().expect_err(text);
    let first = &error.problems()[0];
    assert_eq!(first.file, fixture.user_file(), "{text}");
    assert_eq!(error.code(), CONFIG_INVALID_CODE, "{text}");
    (error.kind(), first.line, first.column, first.message.clone())
}

#[test]
fn the_generated_default_configuration_reloads_unchanged_and_fully_authored() {
    // Control: the file `md init` writes holds every setting at its default.
    // Generate it from the schema's defaults, then load it back.
    let fixture = Fixture::new();
    fixture.write_user("{}\n");
    let defaults = fixture.load().unwrap().user().effective().clone();
    assert_eq!(defaults["register"], serde_json::json!([]));
    fixture.write_user(serde_yaml_ng::to_string(&defaults).unwrap());

    let snapshot = fixture.load().unwrap();
    assert_eq!(snapshot.user().effective(), &defaults);
    assert!(snapshot.user().defaults().is_empty(), "every setting is written");
    let block = user_block(&snapshot);
    assert!(block.file_read.authored && block.shell_commands.authored && block.network.authored);
    assert_eq!(block.file_read.value.len(), 10);
    assert!(snapshot.diagnostics().is_empty(), "{:?}", snapshot.diagnostics());
    assert!(snapshot.user_config_missing().is_none());
}

#[test]
fn a_missing_user_file_takes_every_default_and_is_reported_once() {
    let fixture = Fixture::new();
    let snapshot = fixture.load().unwrap();
    assert_eq!(snapshot.user_config_missing(), Some(fixture.user_file().as_path()));
    let missing: Vec<_> = snapshot.diagnostics().iter().filter(|d| d.code == CONFIG_MISSING_CODE).collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].file, fixture.user_file());
    assert!(missing[0].message.contains("md init"), "{}", missing[0].message);

    assert!(!snapshot.user().is_present());
    let block = user_block(&snapshot);
    assert!(!block.file_read.authored && !block.shell_commands.authored && !block.network.authored);
    assert_eq!(block.file_read.value[0].pattern, "./*.txt");
    assert!(block.shell_commands.value.is_empty());
    assert_eq!(block.network.value, NetworkAccess::Deny);
    assert!(snapshot.registrations().entries.is_empty());
    assert!(snapshot.repository().is_none(), "no repository named, none observed");
    assert_eq!(snapshot.dependencies().len(), 1);
    assert_eq!(snapshot.dependencies()[0].content_hash, None);
}

#[test]
fn the_dependency_records_the_bytes_read() {
    let fixture = Fixture::new();
    let text = "permissions:\n  network: allow\n";
    fixture.write_user(text);
    let snapshot = fixture.load().unwrap();
    let dependency = &snapshot.dependencies()[0];
    assert_eq!(
        (dependency.revision, dependency.path.as_path(), dependency.content_hash.as_deref()),
        (ConfigRevision::User, fixture.user_file().as_path(), Some(content_hash(text.as_bytes()).as_str()))
    );
}

#[test]
fn an_empty_mapping_is_valid_and_an_empty_file_is_not_a_mapping() {
    let fixture = Fixture::new();
    fixture.write_user("{}");
    assert!(fixture.load().unwrap().permissions().is_ok());

    for text in ["", "\n", "# only a comment\n", "---\n"] {
        let (kind, line, column, message) = whole_file_error(text);
        assert_eq!((kind, line, column), (ConfigErrorKind::NotAMapping, Some(1), Some(1)), "{text:?}");
        assert!(message.contains("empty"), "{text:?}: {message}");
    }
    for text in ["- a\n", "just text\n", "3\n"] {
        assert_eq!(whole_file_error(text).0, ConfigErrorKind::NotAMapping, "{text:?}");
    }
}

#[test]
fn duplicate_keys_fail_the_whole_file_at_every_depth() {
    // The shared loader locates a duplicate at its mapping.
    for text in [
        "register: []\nregister: []\n",
        "permissions:\n  network: deny\n  network: allow\n",
        "register:\n  - {a: 1, a: 2}\n",
    ] {
        let (kind, line, column, message) = whole_file_error(text);
        assert_eq!(kind, ConfigErrorKind::Syntax, "{text}: {message}");
        assert!(line.is_some() && column.is_some() && message.contains("duplicate"), "{text}: {message}");
    }
}

#[test]
fn malformed_yaml_fails_the_whole_file_with_its_position() {
    for (text, line) in [
        ("permissions: [unclosed\n", 2),
        ("permissions: {}\n]]] trailing\n", 2),
        ("register: []\n---\nregister: []\n", 2),
        ("1: x\n", 1),
        ("register: [9007199254740993.0]\n", 1),
        ("register: []\n\tbad: indent\n", 2),
    ] {
        let (kind, at, column, message) = whole_file_error(text);
        assert_eq!(kind, ConfigErrorKind::Syntax, "{text}: {message}");
        assert_eq!(at, Some(line), "{text}: {message}");
        assert!(column.is_some(), "{text}");
    }
}

#[test]
fn invalid_utf8_is_a_located_syntax_error() {
    let (kind, line, column, _) = whole_file_error_bytes(b"register: []\nperm\xffissions: {}\n");
    assert_eq!((kind, line, column), (ConfigErrorKind::Syntax, Some(2), Some(5)));
}

fn whole_file_error_bytes(bytes: &[u8]) -> (ConfigErrorKind, Option<usize>, Option<usize>, String) {
    let fixture = Fixture::new();
    fixture.write_user(bytes);
    let error = fixture.load().expect_err("invalid bytes");
    let first = &error.problems()[0];
    (error.kind(), first.line, first.column, first.message.clone())
}

#[test]
fn an_unknown_root_key_fails_the_whole_file_at_the_key() {
    let (kind, line, column, message) = whole_file_error("register: []\n\nnetwork: allow\n");
    assert_eq!((kind, line, column), (ConfigErrorKind::UnknownKey, Some(3), Some(10)));
    assert!(message.contains("`network` is not a configuration setting"), "{message}");
}

#[test]
fn byte_order_marks_crlf_comments_and_document_markers_are_admitted() {
    let fixture = Fixture::new();
    fixture.write_user("\u{feff}---\r\n# settings\r\npermissions:\r\n  network: allow # open\r\nregister: []\r\n...\r\n");
    let snapshot = fixture.load().unwrap();
    let block = user_block(&snapshot);
    assert_eq!(block.network.value, NetworkAccess::Allow);
    assert!(block.network.authored && !block.file_read.authored);

    // Positions after a mark and CRLF lines still name the authored line.
    fixture.write_user("\u{feff}permissions:\r\n  network: maybe\r\n");
    let error = fixture.load().unwrap().permissions().expect_err("invalid network");
    assert_eq!((error.problems()[0].line, error.problems()[0].column), (Some(2), Some(12)));
}

#[test]
fn a_key_with_no_value_takes_the_default() {
    let fixture = Fixture::new();
    fixture.write_user("permissions:\nregister: []\n");
    let snapshot = fixture.load().unwrap();
    let permissions = snapshot.permissions().expect("a null block reads as absent");
    match &permissions.user {
        PermissionsSetting::Inline(block) => assert_eq!(block.network.value, NetworkAccess::Deny),
        other => panic!("{other:?}"),
    }
}

#[test]
fn migrate_legacy_without_legacy_files_loads_like_passive() {
    let fixture = Fixture::new();
    fixture.write_user("permissions:\n  network: allow\n");
    let passive = fixture.load().unwrap();
    let migrating = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::MigrateLegacy).unwrap();
    assert_eq!(migrating.mode(), ConfigLoadMode::MigrateLegacy);
    assert_eq!(migrating.permissions().unwrap(), passive.permissions().unwrap());
    assert_eq!(migrating.dependencies(), passive.dependencies());
}

/// A repository under the fixture: `.git` from `git init`, and a captured
/// committed tree after optionally committing `.darkmatter.yaml`.
fn repository(fixture: &Fixture, committed: Option<&str>) -> (PathBuf, CommittedTreeSnapshot) {
    let root = fixture.root.join("work");
    let repo = git2::Repository::init(&root).unwrap();
    if let Some(text) = committed {
        let blob = repo.blob(text.as_bytes()).unwrap();
        let mut builder = repo.treebuilder(None).unwrap();
        builder.insert(CONFIG_FILE_NAME, blob, 0o100_644).unwrap();
        let tree = repo.find_tree(builder.write().unwrap()).unwrap();
        let signature = git2::Signature::now("Fixture", "fixture@example.com").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "config", &tree, &[]).unwrap();
    }
    let snapshot = GitRepo::discover(&root).unwrap().expect("a repository").committed_tree().unwrap();
    (root, snapshot)
}

fn repository_context(fixture: &Fixture, root: &Path) -> FileResolutionContext {
    fixture.context().with_repository_root(root)
}

#[test]
fn a_context_naming_a_repository_requires_its_committed_tree() {
    let fixture = Fixture::new();
    let root = fixture.root.join("work");
    let error = ConfigSnapshot::load(&repository_context(&fixture, &root), None, ConfigLoadMode::Passive)
        .expect_err("no snapshot");
    assert_eq!(error.kind(), ConfigErrorKind::RepositorySnapshotRequired);
    assert!(error.to_string().contains(&root.display().to_string()), "{error}");
}

#[test]
fn an_unborn_head_and_a_missing_working_file_both_take_the_repository_defaults() {
    let fixture = Fixture::new();
    let (root, tree) = repository(&fixture, None);
    let snapshot = ConfigSnapshot::load(&repository_context(&fixture, &root), Some(&tree), ConfigLoadMode::Passive).unwrap();
    let repository = snapshot.permissions().unwrap().repository.clone().expect("a repository");
    for block in [&repository.head, &repository.worktree] {
        let block = inline(block);
        assert_eq!(block.file_read.value[0].pattern, "&**/*.txt");
        assert!(!block.file_read.authored);
    }
    assert!(snapshot.user_config_missing().is_some(), "only the user file is reported missing");
    assert_eq!(snapshot.diagnostics().len(), 1);
    let revisions: Vec<_> = snapshot.dependencies().iter().map(|d| (d.revision, d.content_hash.clone())).collect();
    assert_eq!(
        revisions,
        [(ConfigRevision::User, None), (ConfigRevision::RepositoryHead, None), (ConfigRevision::RepositoryWorktree, None)]
    );
}

#[test]
fn head_and_working_copy_are_independent_projections() {
    let fixture = Fixture::new();
    let committed = "permissions:\n  network: allow\n";
    let (root, tree) = repository(&fixture, Some(committed));
    let working = "permissions:\n  shell-commands: [prefix ls]\n";
    std::fs::write(root.join(CONFIG_FILE_NAME), working).unwrap();

    let snapshot = ConfigSnapshot::load(&repository_context(&fixture, &root), Some(&tree), ConfigLoadMode::Passive).unwrap();
    let repository = snapshot.permissions().unwrap().repository.clone().unwrap();
    let (head, worktree) = (inline(&repository.head), inline(&repository.worktree));
    assert_eq!((head.network.value, head.network.authored), (NetworkAccess::Allow, true));
    assert!(head.shell_commands.value.is_empty() && !head.shell_commands.authored);
    assert_eq!((worktree.network.value, worktree.network.authored), (NetworkAccess::Deny, false));
    assert_eq!(worktree.shell_commands.value[0].authored, "prefix ls");

    let hashes: Vec<_> = snapshot.dependencies()[1..].iter().map(|d| d.content_hash.clone()).collect();
    assert_eq!(hashes, [Some(content_hash(committed.as_bytes())), Some(content_hash(working.as_bytes()))]);
    let config = snapshot.repository().unwrap();
    assert_eq!((config.head.revision(), config.worktree.revision()), (ConfigRevision::RepositoryHead, ConfigRevision::RepositoryWorktree));
}

#[test]
fn an_invalid_committed_policy_blocks_permissions_and_names_head() {
    let fixture = Fixture::new();
    let (root, tree) = repository(&fixture, Some("permissions:\n  network: sometimes\n"));
    std::fs::write(root.join(CONFIG_FILE_NAME), "permissions: {}\n").unwrap();
    let snapshot = ConfigSnapshot::load(&repository_context(&fixture, &root), Some(&tree), ConfigLoadMode::Passive).unwrap();
    let error = snapshot.permissions().expect_err("HEAD is invalid");
    let problem = &error.problems()[0];
    assert_eq!((problem.revision, problem.line, problem.pointer.as_str()), (ConfigRevision::RepositoryHead, Some(2), "/permissions/network"));
    assert!(error.to_string().contains("(HEAD)"), "{error}");
    assert!(snapshot.repository().unwrap().worktree.permissions().is_ok());

    // A committed syntax error stops the whole load, as a working one does.
    let fixture = Fixture::new();
    let (root, tree) = repository(&fixture, Some("register: [\n"));
    let error = ConfigSnapshot::load(&repository_context(&fixture, &root), Some(&tree), ConfigLoadMode::Passive)
        .expect_err("committed syntax error");
    assert_eq!((error.kind(), error.problems()[0].revision), (ConfigErrorKind::Syntax, ConfigRevision::RepositoryHead));
}
