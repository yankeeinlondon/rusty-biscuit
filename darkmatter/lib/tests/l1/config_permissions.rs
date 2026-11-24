//! `darkmatter::config::permissions`: the evaluated policy behind
//! `ConfigSnapshot::policy`, through the public API. Scope defaults and
//! combination (user OR (HEAD AND working copy), any `!` excludes), the
//! commit rule, `permissions` reference following in every layer, and
//! file-read path judgment.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, GlobReference, PORTABLE_ENV_VARIABLES, canonicalize_simplified};
use darkmatter::config::load::{
    CONFIG_REFERENCE_CODE, CommittedTreeCapture, ConfigError, ConfigErrorKind, ConfigLoadMode, ConfigRevision,
    ConfigSnapshot, NetworkAccess,
};
use darkmatter::config::permissions::{PermissionPolicy, PermissionVerdict};
use darkmatter::config::schema::CONFIG_FILE_NAME;
use darkmatter::markdown::compose::shell_expansion::policy::{check_user_blacklist, check_whitelist, normalize_command};
use sniff::filesystem::git::{CommittedTreeSnapshot, GitRepo};

const FILE: i32 = 0o100_644;
const LINK: i32 = 0o120_000;

/// A home, a working directory, and an outside folder under one temporary
/// directory; nothing is read from the process.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    env: HashMap<String, String>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        for name in ["home", "work", "outside"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        Self { _dir: dir, root, env: HashMap::new() }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn write(&self, path: impl AsRef<Path>, text: &str) -> PathBuf {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn user(&self, text: &str) {
        self.write(Path::new("home").join(CONFIG_FILE_NAME), text);
    }

    fn worktree(&self, text: &str) {
        self.write(Path::new("work").join(CONFIG_FILE_NAME), text);
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.work(), Some(self.home()), self.env.clone())
    }

    /// The context of a document at `work/<relative>`.
    fn reading(&self, relative: &str) -> FileResolutionContext {
        let document = self.work().join(relative);
        FileResolutionContext::from_snapshot(document.parent().unwrap(), Some(self.home()), self.env.clone())
            .with_repository_root(self.work())
    }

    fn load_outside(&self) -> ConfigSnapshot {
        ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive).unwrap()
    }

    /// `git init`s `work`, commits `entries` (path, mode, content) unless
    /// empty (an unborn `HEAD`), and captures the committed tree.
    fn repository(&self, entries: &[(&str, i32, &str)]) -> CommittedTreeSnapshot {
        let repo = git2::Repository::init(self.work()).unwrap();
        if !entries.is_empty() {
            let tree = repo.find_tree(write_tree(&repo, entries)).unwrap();
            let signature = git2::Signature::now("Fixture", "fixture@example.com").unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "policy", &tree, &[]).unwrap();
        }
        GitRepo::discover(&self.work()).unwrap().expect("a repository").committed_tree().unwrap()
    }

    fn load_repository(&self, tree: &CommittedTreeSnapshot) -> ConfigSnapshot {
        ConfigSnapshot::load(&self.context().with_repository_root(self.work()), Some(tree), ConfigLoadMode::Passive)
            .unwrap()
    }
}

/// Writes `entries` as nested trees; `mode` is [`FILE`] or [`LINK`].
fn write_tree(repo: &git2::Repository, entries: &[(&str, i32, &str)]) -> git2::Oid {
    let mut builder = repo.treebuilder(None).unwrap();
    let mut folders: BTreeMap<&str, Vec<(&str, i32, &str)>> = BTreeMap::new();
    for &(path, mode, content) in entries {
        match path.split_once('/') {
            Some((folder, rest)) => folders.entry(folder).or_default().push((rest, mode, content)),
            None => {
                let blob = repo.blob(content.as_bytes()).unwrap();
                builder.insert(path, blob, mode).unwrap();
            }
        }
    }
    for (folder, nested) in folders {
        let tree = write_tree(repo, &nested);
        builder.insert(folder, tree, 0o040_000).unwrap();
    }
    builder.write().unwrap()
}

fn policy(snapshot: &ConfigSnapshot) -> Arc<PermissionPolicy> {
    snapshot.policy().unwrap_or_else(|error| panic!("a valid policy: {error}")).clone()
}

/// Whether `command` (whitespace-split) runs without asking.
fn shell(policy: &PermissionPolicy, command: &str) -> PermissionVerdict {
    let mut words = command.split_whitespace().map(str::to_string);
    let executable = words.next().unwrap();
    let args: Vec<String> = words.collect();
    let normalized = normalize_command(&executable, &args);
    policy.shell().verdict(
        |allows| check_whitelist(allows, &executable, &normalized),
        |excluded| check_user_blacklist(excluded, &executable, &args, &normalized),
    )
}

fn excluded_by(verdict: &PermissionVerdict) -> Option<(ConfigRevision, String)> {
    match verdict {
        PermissionVerdict::ApprovalRequired { exclusion: Some(exclusion) } => {
            Some((exclusion.revision, exclusion.entry.clone()))
        }
        _ => None,
    }
}

fn reference_error(snapshot: &ConfigSnapshot) -> ConfigError {
    snapshot.permissions().expect("the raw blocks are valid");
    let error = snapshot.policy().expect_err("the reference cannot be followed");
    assert_eq!(error.code(), CONFIG_REFERENCE_CODE, "{error}");
    assert!(snapshot.diagnostics().iter().any(|notice| notice.code == CONFIG_REFERENCE_CODE), "reported once");
    error
}

// ---- defaults and combination --------------------------------------------------

#[test]
fn missing_scope_files_contribute_their_schema_defaults() {
    let fixture = Fixture::new();
    let outside = policy(&fixture.load_outside());
    let beside = fixture.reading("notes/today.md");
    assert!(outside.file_read(&fixture.work().join("notes/data.yaml"), &beside).is_preapproved());
    assert!(!outside.file_read(&fixture.work().join("notes/deeper/data.yaml"), &beside).is_preapproved());
    assert!(!outside.file_read(&fixture.work().join("notes/image.png"), &beside).is_preapproved());
    assert_eq!(outside.network(), NetworkAccess::Deny);
    assert!(!shell(&outside, "ls").is_preapproved());

    let tree = fixture.repository(&[]);
    let inside = policy(&fixture.load_repository(&tree));
    assert!(inside.repository().is_some(), "an unborn HEAD still has a committed layer");
    assert!(inside.file_read(&fixture.work().join("a/b/c.md"), &beside).is_preapproved(), "repository default");
    assert!(!inside.file_read(&fixture.work().join("a/b/c.png"), &beside).is_preapproved());
}

#[test]
fn an_empty_list_preapproves_nothing_at_its_scope_and_keeps_the_other_scope() {
    let fixture = Fixture::new();
    fixture.user("permissions:\n  file-read: []\n");
    let reading = fixture.reading("notes/today.md");
    let outside = policy(&fixture.load_outside());
    assert!(!outside.file_read(&fixture.work().join("notes/data.yaml"), &reading).is_preapproved());

    let tree = fixture.repository(&[]);
    let inside = policy(&fixture.load_repository(&tree));
    assert!(inside.file_read(&fixture.work().join("notes/data.yaml"), &reading).is_preapproved(), "the repository default still allows");
}

#[test]
fn head_and_working_copy_allows_are_judged_per_operation_not_per_entry() {
    let fixture = Fixture::new();
    let committed = "permissions:\n  shell-commands: [prefix ls]\n  file-read: [\"&docs/**/*.md\"]\n";
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, committed)]);
    fixture.worktree("permissions:\n  shell-commands: [exact ls -la]\n  file-read: [\"&**/*.md\"]\n");
    let policy = policy(&fixture.load_repository(&tree));

    assert!(shell(&policy, "ls -la").is_preapproved(), "prefix at HEAD and exact in the working copy both allow");
    assert!(!shell(&policy, "ls -R").is_preapproved(), "only HEAD allows");
    let reading = fixture.reading("README.md");
    assert!(policy.file_read(&fixture.work().join("docs/guide/a.md"), &reading).is_preapproved());
    assert!(!policy.file_read(&fixture.work().join("src/a.md"), &reading).is_preapproved(), "only the working copy allows");
}

#[test]
fn an_exclusion_in_any_layer_removes_preapproval_everywhere() {
    for (layer, revision) in [
        ("user", ConfigRevision::User),
        ("head", ConfigRevision::RepositoryHead),
        ("worktree", ConfigRevision::RepositoryWorktree),
    ] {
        let fixture = Fixture::new();
        let allow = "permissions:\n  shell-commands: [prefix git]\n";
        let exclude =
            "permissions:\n  shell-commands: [prefix git, \"!exact git push\"]\n  file-read: [\"&**/*.md\", \"!&secrets/**\"]\n";
        fixture.user(if layer == "user" { exclude } else { allow });
        let tree =
            fixture.repository(&[(CONFIG_FILE_NAME, FILE, if layer == "head" { exclude } else { allow })]);
        fixture.worktree(if layer == "worktree" { exclude } else { allow });
        let policy = policy(&fixture.load_repository(&tree));

        assert!(shell(&policy, "git status").is_preapproved(), "{layer}");
        let pushed = shell(&policy, "git push");
        assert_eq!(excluded_by(&pushed), Some((revision, "!exact git push".to_string())), "{layer}");
        let secret = policy.file_read(&fixture.work().join("secrets/key.md"), &fixture.reading("README.md"));
        assert_eq!(excluded_by(&secret), Some((revision, "!&secrets/**".to_string())), "{layer}");
    }
}

#[test]
fn an_uncommitted_addition_waits_for_a_commit_and_a_removal_applies_at_once() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions:\n  shell-commands: [prefix ls, prefix cat]\n")]);
    let reading = fixture.reading("README.md");

    fixture.worktree("permissions:\n  shell-commands: [prefix ls, prefix cat, prefix rm]\n");
    let added = policy(&fixture.load_repository(&tree));
    assert!(!shell(&added, "rm x").is_preapproved(), "an uncommitted addition has no effect");

    fixture.worktree("permissions:\n  shell-commands: [prefix ls]\n");
    let removed = policy(&fixture.load_repository(&tree));
    assert!(shell(&removed, "ls").is_preapproved());
    assert!(!shell(&removed, "cat x").is_preapproved(), "an uncommitted removal applies at once");

    fixture.worktree("permissions:\n  shell-commands: [prefix ls, prefix cat]\n  file-read: [\"&**/*.md\", \"!&docs/**\"]\n");
    let excluded = policy(&fixture.load_repository(&tree));
    assert!(excluded.file_read(&fixture.work().join("docs/a.md"), &reading).is_excluded(), "an added `!` applies at once");
}

#[test]
fn network_is_allowed_only_when_every_applicable_layer_allows() {
    let allow = "permissions:\n  network: allow\n";
    let fixture = Fixture::new();
    fixture.user(allow);
    assert_eq!(policy(&fixture.load_outside()).network(), NetworkAccess::Allow, "outside a repository only the user counts");

    for (user, head, worktree, expected) in [
        (allow, allow, allow, NetworkAccess::Allow),
        ("{}\n", allow, allow, NetworkAccess::Deny),
        (allow, "{}\n", allow, NetworkAccess::Deny),
        (allow, allow, "{}\n", NetworkAccess::Deny),
    ] {
        let fixture = Fixture::new();
        fixture.user(user);
        let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, head)]);
        fixture.worktree(worktree);
        assert_eq!(policy(&fixture.load_repository(&tree)).network(), expected, "{user:?} {head:?} {worktree:?}");
    }
}

#[test]
fn a_failed_head_capture_is_an_error_never_a_working_copy_only_policy() {
    let fixture = Fixture::new();
    git2::Repository::init(fixture.work()).unwrap();
    fixture.worktree("permissions:\n  network: allow\n");
    let failed: CommittedTreeCapture =
        Err(Arc::new(sniff::SniffError::InvalidRepositoryPath { path: "x".into(), reason: "corrupt fixture" }));
    let error = ConfigSnapshot::load_captured(
        &fixture.context().with_repository_root(fixture.work()),
        Some(&failed),
        ConfigLoadMode::Passive,
    )
    .expect_err("no committed tree, no policy");
    assert_eq!((error.kind(), error.problems()[0].revision), (ConfigErrorKind::Read, ConfigRevision::RepositoryHead));
}

// ---- references -------------------------------------------------------------------

const CLAUDINE: &str = "# Claudine's own settings\nmodel: opus\npermissions:\n  shell-commands:\n    - prefix ls\n  network: allow\nhooks: {}\n";

#[test]
fn a_user_reference_is_followed_to_its_target_block_with_defaults() {
    let fixture = Fixture::new();
    fixture.user("permissions: ~/.claudine/config.yaml\n");
    let target = fixture.write("home/.claudine/config.yaml", CLAUDINE);
    let snapshot = fixture.load_outside();
    let policy = policy(&snapshot);

    let user = policy.user();
    assert_eq!((user.block_file(), user.is_referenced()), (target.as_path(), true));
    assert!(user.permissions().network.authored && !user.permissions().file_read.authored);
    assert_eq!(user.permissions().file_read.value.len(), 10, "the user scope's defaults");
    assert!(shell(&policy, "ls -la").is_preapproved());
    assert_eq!(policy.network(), NetworkAccess::Allow);
    let dependency = snapshot.dependencies().iter().find(|dependency| dependency.path == target).expect("the target is a dependency");
    assert_eq!(dependency.referenced_from.as_deref(), Some(fixture.home().join(CONFIG_FILE_NAME).as_path()));
    assert!(dependency.content_hash.is_some());
}

#[test]
fn an_unusable_reference_target_is_an_error_naming_both_files() {
    for (target, needle) in [
        (None, "does not exist"),
        (Some("model: opus\n"), "no `permissions` mapping"),
        (Some("permissions: ./other.yaml\n"), "do not chain"),
        (Some("permissions: [\n"), "not valid YAML"),
        (Some("permissions: 3\n"), "not a mapping"),
        (Some("permissions:\n  network: sometimes\n"), "whose block is invalid"),
        (Some("permissions:\n  shell-commands: [\"bogus ls\"]\n"), "whose block is invalid"),
    ] {
        let fixture = Fixture::new();
        fixture.user("# mine\npermissions: ./claudine.yaml\n");
        let target_path = fixture.home().join("claudine.yaml");
        if let Some(text) = target {
            fixture.write("home/claudine.yaml", text);
        }
        let error = reference_error(&fixture.load_outside());
        let text = error.to_string();
        assert!(text.contains(needle), "{target:?}: {text}");
        assert!(text.contains(&target_path.display().to_string()), "{target:?}: names the target: {text}");
        let source = error.problems().last().unwrap();
        assert_eq!(
            (source.file.as_path(), source.pointer.as_str(), source.line),
            (fixture.home().join(CONFIG_FILE_NAME).as_path(), "/permissions", Some(2)),
            "{target:?}: located at the reference"
        );
        if target.is_none() || needle == "no `permissions` mapping" {
            assert!(text.contains("md init --inline"), "{text}");
        }
    }
}

/// Where a broken-reference fixture declares `permissions: <reference>`.
#[derive(Clone, Copy, Debug)]
enum Declared {
    User,
    Worktree,
    Head,
}

/// The reference error when the file of `declared` holds `permissions:
/// <reference>` and `policy.yaml` beside it holds `target` (absent for
/// `None`); every other layer is valid.
fn broken_reference(declared: Declared, reference: &str, target: Option<&str>) -> (Fixture, ConfigError) {
    let fixture = Fixture::new();
    let declaration = format!("permissions: {reference}\n");
    let error = match declared {
        Declared::User => {
            fixture.user(&declaration);
            if let Some(text) = target {
                fixture.write("home/policy.yaml", text);
            }
            reference_error(&fixture.load_outside())
        }
        Declared::Worktree => {
            fixture.user("permissions: {}\n");
            let tree = fixture.repository(&[]);
            fixture.worktree(&declaration);
            if let Some(text) = target {
                fixture.write("work/policy.yaml", text);
            }
            reference_error(&fixture.load_repository(&tree))
        }
        Declared::Head => {
            fixture.user("permissions: {}\n");
            let mut entries = vec![(CONFIG_FILE_NAME, FILE, declaration.as_str())];
            entries.extend(target.map(|text| ("policy.yaml", FILE, text)));
            let tree = fixture.repository(&entries);
            fixture.worktree("permissions: {}\n");
            reference_error(&fixture.load_repository(&tree))
        }
    };
    (fixture, error)
}

/// The remedy of a broken reference names a command only when that command
/// writes the declaring file: `md init --inline` for the user file, a
/// direct correction for a repository file, and the commit for `HEAD`.
fn assert_scoped_remedy(declared: Declared, target: Option<&str>, diagnosis: &str, fix: &str) {
    let (fixture, error) = broken_reference(declared, "./policy.yaml", target);
    let source = error.problems().last().unwrap();
    let message = source.message.as_str();
    assert!(message.contains(diagnosis) && message.contains(fix), "{declared:?} {target:?}: {message}");
    let (revision, declared_in) = match declared {
        Declared::User => (ConfigRevision::User, fixture.home().join(CONFIG_FILE_NAME)),
        Declared::Worktree => (ConfigRevision::RepositoryWorktree, fixture.work().join(CONFIG_FILE_NAME)),
        Declared::Head => (ConfigRevision::RepositoryHead, fixture.work().join(CONFIG_FILE_NAME)),
    };
    assert_eq!((source.revision, source.file.as_path()), (revision, declared_in.as_path()), "{message}");
    let commit = "commit the correction too, because this reference is declared in the commit at HEAD";
    match declared {
        Declared::User => {
            assert!(message.contains("or run `md init --inline` to write the block back inline"), "{message}");
            assert!(!message.contains("commit"), "{message}");
        }
        Declared::Worktree | Declared::Head => {
            assert!(!message.contains("md init"), "a user-scoped command cannot repair it: {message}");
            let inline = format!("point `permissions` in `{}` at another file or write the block there inline", declared_in.display());
            assert!(message.contains(&inline), "{message}");
            assert_eq!(message.contains(commit), matches!(declared, Declared::Head), "{message}");
        }
    }
}

macro_rules! scoped_remedy_tests {
    ($($name:ident: $declared:ident, $target:expr, $diagnosis:expr, $fix:expr;)+) => {$(
        #[test]
        fn $name() {
            assert_scoped_remedy(Declared::$declared, $target, $diagnosis, $fix);
        }
    )+};
}

scoped_remedy_tests! {
    missing_target_remedy_user: User, None, "which does not exist", "create it";
    missing_target_remedy_worktree: Worktree, None, "which does not exist", "create it";
    missing_target_remedy_head: Head, None, "which does not exist", "create it";
    invalid_yaml_target_remedy_user: User, Some("permissions: [\n"), "which is not valid YAML", "fix it";
    invalid_yaml_target_remedy_worktree: Worktree, Some("permissions: [\n"), "which is not valid YAML", "fix it";
    invalid_yaml_target_remedy_head: Head, Some("permissions: [\n"), "which is not valid YAML", "fix it";
    chained_target_remedy_user: User, Some("permissions: ./other.yaml\n"), "references do not chain", "write the block in `";
    chained_target_remedy_worktree: Worktree, Some("permissions: ./other.yaml\n"), "references do not chain", "write the block in `";
    chained_target_remedy_head: Head, Some("permissions: ./other.yaml\n"), "references do not chain", "write the block in `";
    nonmapping_target_remedy_user: User, Some("[]\n"), "which is not a YAML mapping", "write a `permissions` mapping in it";
    nonmapping_target_remedy_worktree: Worktree, Some("[]\n"), "which is not a YAML mapping", "write a `permissions` mapping in it";
    nonmapping_target_remedy_head: Head, Some("[]\n"), "which is not a YAML mapping", "write a `permissions` mapping in it";
    wrong_block_type_remedy_user: User, Some("permissions: 3\n"), "whose `permissions` is not a mapping", "write a `permissions` mapping in it";
    wrong_block_type_remedy_worktree: Worktree, Some("permissions: 3\n"), "whose `permissions` is not a mapping", "write a `permissions` mapping in it";
    wrong_block_type_remedy_head: Head, Some("permissions: 3\n"), "whose `permissions` is not a mapping", "write a `permissions` mapping in it";
    missing_block_remedy_user: User, Some("{}\n"), "which has no `permissions` mapping", "write a `permissions` mapping in it";
    missing_block_remedy_worktree: Worktree, Some("{}\n"), "which has no `permissions` mapping", "write a `permissions` mapping in it";
    missing_block_remedy_head: Head, Some("{}\n"), "which has no `permissions` mapping", "write a `permissions` mapping in it";
}

#[test]
fn an_equivalent_reference_spelling_keeps_the_remedy() {
    for declared in [Declared::User, Declared::Worktree, Declared::Head] {
        let remedy = |reference: &str| {
            let (fixture, error) = broken_reference(declared, reference, Some("permissions: 3\n"));
            let message = error.problems().last().unwrap().message.clone();
            message.replacen(&format!("`{reference}`"), "`<reference>`", 1).replace(&fixture.root.display().to_string(), "<root>")
        };
        assert_eq!(remedy("./policy.yaml"), remedy("./sub/../policy.yaml"), "{declared:?}");
    }
}

#[test]
fn a_valid_referenced_mapping_reports_no_initializer_in_either_scope() {
    let fixture = Fixture::new();
    fixture.user("permissions: ./policy.yaml\n");
    fixture.write("home/policy.yaml", "permissions: {}\n");
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions: ./policy.yaml\n"), ("policy.yaml", FILE, "permissions: {}\n")]);
    fixture.worktree("permissions: ./policy.yaml\n");
    fixture.write("work/policy.yaml", "permissions: {}\n");
    for snapshot in [fixture.load_outside(), fixture.load_repository(&tree)] {
        let policy = policy(&snapshot);
        assert!(policy.user().is_referenced());
        assert!(snapshot.diagnostics().iter().all(|notice| !notice.message.contains("md init")), "{:?}", snapshot.diagnostics());
    }
}

#[test]
fn a_repository_reference_must_stay_inside_the_repository() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[]);
    fixture.worktree("permissions: ../outside/policy.yaml\n");
    let snapshot = fixture.load_repository(&tree);
    assert_eq!(snapshot.permissions().expect_err("lexical escape").kind(), ConfigErrorKind::InvalidReference);

    #[cfg(unix)]
    {
        let fixture = Fixture::new();
        let tree = fixture.repository(&[]);
        let outside = fixture.write("outside/policy.yaml", "permissions:\n  shell-commands: [prefix rm]\n");
        std::os::unix::fs::symlink(&outside, fixture.work().join("policy.yaml")).unwrap();
        fixture.worktree("permissions: ./policy.yaml\n");
        let error = reference_error(&fixture.load_repository(&tree));
        assert!(error.to_string().contains("outside the repository"), "{error}");
    }
}

#[test]
fn inline_to_reference_conversion_is_subject_to_the_commit_rule() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions:\n  shell-commands: [prefix ls]\n")]);
    fixture.worktree("permissions: ./policy.yaml\n");
    fixture.write("work/policy.yaml", "permissions:\n  shell-commands: [prefix ls, prefix cat]\n");
    let policy = policy(&fixture.load_repository(&tree));
    assert!(policy.repository().unwrap().worktree.is_referenced());
    assert!(shell(&policy, "ls").is_preapproved());
    assert!(!shell(&policy, "cat x").is_preapproved(), "the new target widens nothing until committed");
}

#[test]
fn a_redirected_reference_and_an_edited_target_are_read_per_revision() {
    let a = "permissions:\n  shell-commands: [prefix ls]\n";
    let b = "permissions:\n  shell-commands: [prefix ls, prefix cat]\n";
    let fixture = Fixture::new();
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions: ./a.yaml\n"), ("a.yaml", FILE, a), ("b.yaml", FILE, b)]);
    fixture.write("work/a.yaml", a);
    fixture.write("work/b.yaml", b);

    fixture.worktree("permissions: ./b.yaml\n");
    let redirected = policy(&fixture.load_repository(&tree));
    assert!(!shell(&redirected, "cat x").is_preapproved(), "redirection cannot widen");

    fixture.worktree("permissions: ./a.yaml\n");
    fixture.write("work/a.yaml", b);
    let edited = policy(&fixture.load_repository(&tree));
    assert!(!shell(&edited, "cat x").is_preapproved(), "HEAD reads the committed target bytes");
    assert_eq!(edited.repository().unwrap().head.block_file(), fixture.work().join("a.yaml"));

    fixture.write("work/a.yaml", "permissions:\n  shell-commands: []\n");
    let narrowed = policy(&fixture.load_repository(&tree));
    assert!(!shell(&narrowed, "ls").is_preapproved(), "a narrowing target edit applies at once");

    std::fs::remove_file(fixture.work().join("a.yaml")).unwrap();
    let error = reference_error(&fixture.load_repository(&tree));
    assert_eq!(error.problems().last().unwrap().revision, ConfigRevision::RepositoryWorktree, "{error}");
}

#[test]
fn a_committed_reference_reads_only_the_committed_tree() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions: ./a.yaml\n")]);
    fixture.write("work/a.yaml", "permissions:\n  shell-commands: [prefix ls]\n");
    fixture.worktree("permissions: ./a.yaml\n");
    let error = reference_error(&fixture.load_repository(&tree));
    let source = error.problems().last().unwrap();
    assert_eq!(source.revision, ConfigRevision::RepositoryHead, "the uncommitted target is never read for HEAD: {error}");
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn an_unborn_head_takes_the_default_and_the_working_reference_is_followed() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[]);
    fixture.worktree("permissions: ./a.yaml\n");
    fixture.write("work/a.yaml", "permissions:\n  shell-commands: [prefix ls]\n");
    let policy = policy(&fixture.load_repository(&tree));
    let repository = policy.repository().unwrap();
    assert!(!repository.head.is_referenced() && repository.worktree.is_referenced());
    assert!(!shell(&policy, "ls").is_preapproved(), "the committed default allows no command");
}

#[test]
fn committed_symbolic_links_are_followed_inside_the_tree() {
    let block = "permissions:\n  shell-commands: [prefix ls]\n";
    let fixture = Fixture::new();
    let tree = fixture.repository(&[
        (CONFIG_FILE_NAME, FILE, "permissions: ./link.yaml\n"),
        ("link.yaml", LINK, "config/hop.yaml"),
        ("config/hop.yaml", LINK, "../policies/real.yaml"),
        ("policies/real.yaml", FILE, block),
    ]);
    fixture.worktree("permissions:\n  shell-commands: [prefix ls]\n");
    let policy = policy(&fixture.load_repository(&tree));
    assert_eq!(policy.repository().unwrap().head.block_file(), fixture.work().join("policies/real.yaml"));
    assert!(shell(&policy, "ls").is_preapproved());

    for (links, needle) in [
        (vec![("link.yaml", "other.yaml"), ("other.yaml", "link.yaml")], "cycle"),
        (vec![("link.yaml", "../outside/policy.yaml")], "outside the repository"),
        (vec![("link.yaml", "/etc/policy.yaml")], "outside the repository"),
    ] {
        let fixture = Fixture::new();
        let mut entries = vec![(CONFIG_FILE_NAME, FILE, "permissions: ./link.yaml\n")];
        entries.extend(links.iter().map(|&(path, target)| (path, LINK, target)));
        let tree = fixture.repository(&entries);
        let error = reference_error(&fixture.load_repository(&tree));
        assert!(error.to_string().contains(needle), "{links:?}: {error}");
        assert_eq!(error.problems()[0].revision, ConfigRevision::RepositoryHead);
    }
}

#[test]
fn a_long_committed_link_chain_is_refused() {
    let fixture = Fixture::new();
    let names: Vec<String> = (0..=41).map(|index| format!("l{index}.yaml")).collect();
    let mut entries = vec![(CONFIG_FILE_NAME, FILE, "permissions: ./l0.yaml\n")];
    for index in 0..41 {
        entries.push((names[index].as_str(), LINK, names[index + 1].as_str()));
    }
    entries.push((names[41].as_str(), FILE, "permissions: {}\n"));
    let tree = fixture.repository(&entries);
    let error = reference_error(&fixture.load_repository(&tree));
    assert!(error.to_string().contains("more than 40"), "{error}");
}

// ---- file reads ----------------------------------------------------------------------

#[test]
fn user_patterns_cover_home_portable_variables_and_escaped_exact_files() {
    let mut fixture = Fixture::new();
    let notes = fixture.root.join("notes");
    fixture.env = HashMap::from([
        (PORTABLE_ENV_VARIABLES.to_string(), "NOTES".to_string()),
        ("NOTES".to_string(), notes.display().to_string()),
    ]);
    let exact = format!("~/data/{}", GlobReference::escape("[id].yaml"));
    fixture.user(&format!("permissions:\n  file-read: [\"~/docs/*.md\", \"{{{{NOTES}}}}/*.txt\", \"{exact}\"]\n"));
    let policy = policy(&fixture.load_outside());
    let reading = fixture.reading("README.md");
    for (target, expected) in [
        (fixture.home().join("docs/a.md"), true),
        (fixture.home().join("docs/sub/a.md"), false),
        (notes.join("today.txt"), true),
        (fixture.home().join("data/[id].yaml"), true),
        (fixture.home().join("data/i.yaml"), false),
    ] {
        assert_eq!(policy.file_read(&target, &reading).is_preapproved(), expected, "{}", target.display());
    }
}

#[test]
fn traversal_is_judged_lexically_and_missing_files_by_their_location() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[(CONFIG_FILE_NAME, FILE, "permissions:\n  file-read: [\"&**/*.md\", \"!&secrets/**\"]\n")]);
    fixture.worktree("permissions:\n  file-read: [\"&**/*.md\", \"!&secrets/**\"]\n");
    fixture.user("permissions:\n  file-read: []\n");
    let policy = policy(&fixture.load_repository(&tree));
    let reading = fixture.reading("docs/guide.md");

    assert!(policy.file_read(Path::new("../secrets/key.md"), &reading).is_excluded(), "relative to the reading document");
    assert!(policy.file_read(&fixture.work().join("docs/../secrets/key.md"), &reading).is_excluded());
    assert!(!policy.file_read(&fixture.work().join("../outside/a.md"), &reading).is_preapproved());
    assert!(policy.file_read(&fixture.work().join("docs/not/yet/written.md"), &reading).is_preapproved(), "a missing file probe");
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_repository_is_never_a_repository_allow() {
    let fixture = Fixture::new();
    let tree = fixture.repository(&[]);
    fixture.user("permissions:\n  file-read: []\n");
    let outside = fixture.write("outside/notes.md", "secret\n");
    std::fs::create_dir_all(fixture.work().join("docs")).unwrap();
    std::os::unix::fs::symlink(&outside, fixture.work().join("docs/notes.md")).unwrap();
    std::os::unix::fs::symlink(fixture.root.join("outside"), fixture.work().join("linked")).unwrap();
    let linked = policy(&fixture.load_repository(&tree));
    let reading = fixture.reading("README.md");

    assert_eq!(linked.file_read(&fixture.work().join("docs/notes.md"), &reading), PermissionVerdict::ApprovalRequired { exclusion: None });
    assert!(!linked.file_read(&fixture.work().join("linked/missing.md"), &reading).is_preapproved(), "a missing file below an escaping link");
    std::fs::write(fixture.work().join("docs/real.md"), "x").unwrap();
    assert!(linked.file_read(&fixture.work().join("docs/real.md"), &reading).is_preapproved(), "control");

    // An exclusion matches where the link leads, too.
    let fixture2 = Fixture::new();
    let tree = fixture2.repository(&[]);
    fixture2.worktree("permissions:\n  file-read: [\"&**/*.md\", \"!&secrets/**\"]\n");
    std::fs::create_dir_all(fixture2.work().join("secrets")).unwrap();
    std::fs::write(fixture2.work().join("secrets/key.md"), "x").unwrap();
    std::os::unix::fs::symlink(fixture2.work().join("secrets/key.md"), fixture2.work().join("alias.md")).unwrap();
    let aliased = policy(&fixture2.load_repository(&tree));
    assert!(aliased.file_read(&fixture2.work().join("alias.md"), &fixture2.reading("README.md")).is_excluded());
}
