//! A request's configuration snapshot: loaded once when the request is
//! prepared, from the tree its own repository discovery pinned, shared by
//! every derivation, and required from the caller of `with_context` when the
//! context names a repository.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use darkmatter::config::load::{
    ConfigError, ConfigLoadMode, ConfigRevision, ConfigSnapshot, ConfigSnapshotSource, NetworkAccess, PermissionsSetting,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, ComposeRequest, ContextBuildError, RequestSnapshot, capture_resolution_context,
};

/// Runs git with a fixture-local identity, so the caller's hooks and signing
/// configuration never reach the fixture.
fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed");
}

/// A repository under `temp` whose committed `.darkmatter.yaml` allows the
/// network while the working copy denies it, and a home with no user file.
fn repository_with_edited_config(temp: &Path, name: &str) -> (PathBuf, PathBuf) {
    let root = biscuit_file::canonicalize_simplified(temp).unwrap().join(name);
    let home = root.parent().unwrap().join(format!("{name}-home"));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    git(&root, &["init", "-q"]);
    std::fs::write(root.join(".darkmatter.yaml"), "permissions:\n  network: allow\n").unwrap();
    git(&root, &["add", ".darkmatter.yaml"]);
    git(&root, &["commit", "-q", "-m", "policy"]);
    std::fs::write(root.join(".darkmatter.yaml"), "permissions:\n  network: deny\n").unwrap();
    (root, home)
}

fn network(setting: &PermissionsSetting) -> NetworkAccess {
    match setting {
        PermissionsSetting::Inline(permissions) => permissions.network.value,
        other => panic!("expected an inline block, got {other:?}"),
    }
}

/// Preparing a request loads its configuration passively from the tree the
/// request's repository discovery pinned: the committed revision is `HEAD`'s,
/// not the working copy's.
#[test]
fn prepare_loads_head_and_worktree_policy_from_its_own_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let (root, home) = repository_with_edited_config(temp.path(), "repo");
    let snapshot = RequestSnapshot::new(&root).with_home(Some(home.clone()));

    let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap();

    let config = request.config_snapshot();
    assert_eq!(config.mode(), ConfigLoadMode::Passive);
    assert_eq!(config.user_config_missing(), Some(home.join(".darkmatter.yaml").as_path()));
    let repository = config.repository().expect("the request names a repository");
    assert_eq!(repository.head.revision(), ConfigRevision::RepositoryHead);
    let permissions = config.permissions().unwrap();
    let projections = permissions.repository.as_ref().unwrap();
    assert_eq!(network(&projections.head), NetworkAccess::Allow, "HEAD is the committed text");
    assert_eq!(network(&projections.worktree), NetworkAccess::Deny, "the working copy is the edited text");
}

/// The capturing builder hands back the tree its discovery pinned, so a host
/// loads configuration without discovering the repository again.
#[test]
fn the_capturing_builder_returns_the_pinned_head_tree() {
    let temp = tempfile::tempdir().unwrap();
    let (root, home) = repository_with_edited_config(temp.path(), "repo");

    let (context, committed) = capture_resolution_context(&RequestSnapshot::new(&root).with_home(Some(home))).unwrap();

    let tree = committed.expect("a repository was found").expect("HEAD is readable");
    assert!(tree.commit_id().is_some(), "HEAD names the fixture's commit");
    let config = ConfigSnapshot::load_captured(&context, Some(&Ok(Arc::clone(&tree))), ConfigLoadMode::Passive).unwrap();
    assert!(config.repository().unwrap().head.is_present());
}

/// An unborn `HEAD` is an empty committed view, not an error.
#[test]
fn an_unborn_head_is_an_empty_committed_view() {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    git(&root, &["init", "-q"]);
    std::fs::write(root.join(".darkmatter.yaml"), "{}\n").unwrap();

    let request = ComposeRequest::prepare(ComposeOptions::new(), &RequestSnapshot::new(&root)).unwrap();

    let repository = request.config_snapshot().repository().unwrap();
    assert!(!repository.head.is_present(), "nothing is committed yet");
    assert!(repository.worktree.is_present());
}

/// `with_context` never rediscovers a repository's committed policy: a
/// context naming a repository without a supplied snapshot is a typed error.
#[test]
fn with_context_requires_a_snapshot_for_a_repository_context() {
    let temp = tempfile::tempdir().unwrap();
    let (root, home) = repository_with_edited_config(temp.path(), "repo");
    let (context, _) = capture_resolution_context(&RequestSnapshot::new(&root).with_home(Some(home))).unwrap();

    let error = ComposeRequest::with_context(ComposeOptions::new(), context.clone()).unwrap_err();

    assert!(
        matches!(&error, ContextBuildError::ConfigSnapshotRequired { repository, .. } if repository == &root),
        "{error:?}"
    );
    let load = ConfigSnapshot::load_captured(&context, None, ConfigLoadMode::Passive).unwrap_err();
    assert!(load.to_string().contains("no committed tree"), "{load}");
}

/// A context outside every repository loads the user file passively.
#[test]
fn with_context_outside_a_repository_loads_the_user_file() {
    let temp = tempfile::tempdir().unwrap();
    let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    std::fs::write(dir.join(".darkmatter.yaml"), "register: []\n").unwrap();
    let context = biscuit_file::FileResolutionContext::from_snapshot(&dir, Some(dir.clone()), Default::default());

    let request = ComposeRequest::with_context(ComposeOptions::new(), context).unwrap();

    assert!(request.config_snapshot().user().is_present());
    assert!(request.config_snapshot().repository().is_none());
}

/// Every view of one request shares the one snapshot.
#[test]
fn clones_and_option_changes_share_the_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    let request = ComposeRequest::prepare(ComposeOptions::new(), &RequestSnapshot::new(&dir)).unwrap();

    let mapped = request.clone().map_options(|options| options.with_fail_fast(true));
    let replaced = request.clone().map_options(|_| ComposeOptions::new());

    assert!(Arc::ptr_eq(request.config_snapshot(), mapped.config_snapshot()));
    assert!(Arc::ptr_eq(request.config_snapshot(), replaced.config_snapshot()));
    assert!(Arc::ptr_eq(request.config_snapshot(), request.options().config_snapshot().unwrap()));
}

/// A source in another repository gets that repository's projection while
/// reading the user file from the bytes the first load already holds.
#[test]
fn a_source_in_another_repository_gets_its_own_projection_and_the_same_user_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let (first, home) = repository_with_edited_config(temp.path(), "first");
    let (second, _) = repository_with_edited_config(temp.path(), "second");
    std::fs::write(second.join(".darkmatter.yaml"), "permissions:\n  network: allow\n").unwrap();
    std::fs::write(home.join(".darkmatter.yaml"), "register: []\n").unwrap();
    let snapshot = RequestSnapshot::new(&first).with_home(Some(home.clone()));
    let (first_context, first_tree) = capture_resolution_context(&snapshot).unwrap();
    let base = ConfigSnapshot::load_captured(&first_context, first_tree.as_ref(), ConfigLoadMode::MigrateLegacy).unwrap();
    // Gone from disk: a projection that re-read the file would find no user
    // configuration.
    std::fs::remove_file(home.join(".darkmatter.yaml")).unwrap();
    let (second_context, second_tree) = capture_resolution_context(&snapshot.at_request_dir(&second)).unwrap();

    let projected = base.with_repository_projection(&second_context, second_tree.as_ref()).unwrap();

    assert_eq!(projected.mode(), ConfigLoadMode::MigrateLegacy);
    assert_eq!(projected.repository().unwrap().root, second);
    assert!(projected.user().is_present(), "the user file comes from the shared bytes");
    assert_eq!(projected.dependencies()[0], base.dependencies()[0]);
    let worktree = &projected.permissions().unwrap().repository.as_ref().unwrap().worktree;
    assert_eq!(network(worktree), NetworkAccess::Allow, "the second repository's own working copy");
}

/// A host's capture that holds one snapshot for one repository root.
#[derive(Debug)]
struct OneRepository {
    root: PathBuf,
    snapshot: Arc<ConfigSnapshot>,
}

impl ConfigSnapshotSource for OneRepository {
    fn config_snapshot_for(
        &self,
        context: &biscuit_file::FileResolutionContext,
    ) -> Option<Result<Arc<ConfigSnapshot>, ConfigError>> {
        (context.repository_root() == Some(self.root.as_path())).then(|| Ok(Arc::clone(&self.snapshot)))
    }
}

/// A host supplies its snapshot through the captured `ctx` it composes
/// under; a repository its capture does not cover is still a typed error.
#[test]
fn a_hosts_captured_context_supplies_its_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let (first, home) = repository_with_edited_config(temp.path(), "first");
    let (second, _) = repository_with_edited_config(temp.path(), "second");
    let snapshot = RequestSnapshot::new(&first).with_home(Some(home));
    let (first_context, tree) = capture_resolution_context(&snapshot).unwrap();
    let (second_context, _) = capture_resolution_context(&snapshot.at_request_dir(&second)).unwrap();
    let captured =
        Arc::new(ConfigSnapshot::load_captured(&first_context, tree.as_ref(), ConfigLoadMode::Passive).unwrap());
    let ctx = ComposeContext::capture_minimal()
        .with_config_source(Arc::new(OneRepository { root: first.clone(), snapshot: Arc::clone(&captured) }));

    let request = ComposeRequest::with_context(ComposeOptions::new_with_context(ctx.clone()), first_context).unwrap();
    let elsewhere = ComposeRequest::with_context(ComposeOptions::new_with_context(ctx), second_context).unwrap_err();

    assert!(Arc::ptr_eq(request.config_snapshot(), &captured));
    assert!(matches!(elsewhere, ContextBuildError::ConfigSnapshotRequired { .. }), "{elsewhere:?}");
}
