//! git2 golden-value and discovery tri-state parity tests.
//!
//! These lock the observable behavior of sniff's git layer while it is still
//! backed by git2, so the gitoxide port can be validated against an independent
//! git2 ground truth built by the same deterministic fixtures. The fixtures pin
//! signatures and commit times (via the bench `builder`), so object IDs and
//! orderings are reproducible; assertions either compare the public `sniff` API
//! against a freshly-opened git2 handle or against structural invariants that
//! must survive the backend swap.

#[path = "../benches/support/builder.rs"]
mod builder;

use std::fs;
use std::path::{Path, PathBuf};

use git2::Repository;
use tempfile::TempDir;

use sniff::filesystem::detect_git_with_request;
use sniff::filesystem::git::{
    DeltaKind, FileAction, FileStatus, GitHostingProvider, GitRepo, RefKind, get_commit_files,
    merge_conflicts_at,
};
use sniff::request::GitRequest;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Serializes tests that mutate process environment variables. `std::env::set_var`
/// is unsafe in multi-threaded code; acquiring this mutex before mutating env
/// vars prevents races between concurrent tests.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Normalize a path to forward slashes so assertions hold on Windows, where
/// `PathBuf` display uses backslashes even though git stores POSIX paths.
fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Build a linear `main`-branch history with `commits` commits, pinned times.
///
/// Each commit rewrites `file.txt` and adds a per-commit source file so every
/// commit carries a small diff. HEAD is forced to `refs/heads/main` before the
/// first commit so the branch name is deterministic regardless of the host's
/// `init.defaultBranch`.
fn build_linear_main(root: &Path, commits: usize) -> Repository {
    let repo = builder::init_repo(root);
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    let mut seconds = 1_000i64;
    for i in 0..commits {
        builder::write_file(&root.join("file.txt"), &format!("v{i}\n"));
        builder::write_file(
            &root.join(format!("src/m{i:02}.rs")),
            &format!("pub const N: u32 = {i};\n"),
        );
        builder::commit_all_at(&repo, &format!("c{i}: commit {i}"), seconds);
        seconds += 60;
    }
    repo
}

/// Collect HEAD-reachable commit SHAs newest-first using a fresh git2 handle —
/// the independent ground truth the public API must match.
fn git2_head_shas(repo: &Repository) -> Vec<String> {
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.set_sorting(git2::Sort::TIME).expect("set sorting");
    revwalk.push_head().expect("push HEAD");
    revwalk
        .filter_map(|oid| oid.ok().map(|o| o.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Golden values
// ---------------------------------------------------------------------------

#[test]
fn golden_branch_identity_matches_head() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert_eq!(handle.current_branch().as_deref(), Some("main"));

    // Ground truth: HEAD shorthand from an independent git2 handle.
    let head_shorthand = repo.head().unwrap().shorthand().unwrap().to_string();
    assert_eq!(handle.current_branch(), Some(head_shorthand));
}

#[test]
fn golden_commit_ordering_is_newest_first_and_matches_git2() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 5);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let api_shas: Vec<String> = handle
        .recent_commits(5)
        .iter()
        .map(|c| c.sha.clone())
        .collect();

    // Identical SHAs in identical order to the independent git2 walk.
    assert_eq!(api_shas, git2_head_shas(&repo));

    // Timestamps strictly decreasing (newest-first).
    let times: Vec<_> = handle
        .recent_commits(5)
        .iter()
        .map(|c| c.timestamp)
        .collect();
    assert!(
        times.windows(2).all(|w| w[0] >= w[1]),
        "expected newest-first: {times:?}"
    );
}

#[test]
fn golden_status_categories_staged_unstaged_untracked() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);

    // Staged: a brand new file added to the index.
    fs::write(dir.path().join("staged.txt"), "staged\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("staged.txt")).unwrap();
    index.write().unwrap();

    // Unstaged: modify a tracked file without staging.
    fs::write(dir.path().join("file.txt"), "modified\n").unwrap();

    // Untracked: a new file left out of the index.
    fs::write(dir.path().join("untracked.txt"), "untracked\n").unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert_eq!(
        info.status.as_ref().unwrap().staged_count,
        1,
        "one staged add"
    );
    assert_eq!(
        info.status.as_ref().unwrap().unstaged_count,
        1,
        "one unstaged modify"
    );
    assert_eq!(
        info.status.as_ref().unwrap().untracked_count,
        1,
        "one untracked file"
    );
    assert!(info.status.as_ref().unwrap().is_dirty);
}

#[test]
fn golden_commit_file_changes_are_path_ordered() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);

    // A commit touching several files in non-sorted creation order.
    builder::write_file(&dir.path().join("zeta.txt"), "z\n");
    builder::write_file(&dir.path().join("alpha.txt"), "a\n");
    builder::write_file(&dir.path().join("src/m01.rs"), "pub const X: u32 = 1;\n");
    builder::commit_all_at(&repo, "multi-file change", 5_000);

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = gix::open(dir.path()).unwrap();
    let head_oid = gix::ObjectId::from_hex(head_sha.as_bytes()).unwrap();
    let files: Vec<String> = get_commit_files(&gix_repo, head_oid)
        .iter()
        .map(|(p, _)| norm(p))
        .collect();

    assert!(files.contains(&"alpha.txt".to_string()));
    assert!(files.contains(&"zeta.txt".to_string()));
    assert!(files.contains(&"src/m01.rs".to_string()));

    // gix diff_tree_to_tree yields path-ordered results; lock that ordering.
    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted, "commit files should be path-ordered");
}

#[test]
fn golden_branch_ahead_behind_counts() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);

    // Branch `feature` off the second commit (one behind main's tip), then add
    // one commit (one ahead). Relative to main's HEAD it is 1 ahead / 1 behind.
    let second = git2_head_shas(&repo)[1].clone();
    let second_oid = git2::Oid::from_str(&second).unwrap();
    let second_commit = repo.find_commit(second_oid).unwrap();
    repo.branch("feature", &second_commit, false).unwrap();
    repo.set_head("refs/heads/feature").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    builder::write_file(&dir.path().join("feature.txt"), "feature\n");
    builder::commit_all_at(&repo, "feature commit", 4_000);

    // Back to main as the current branch.
    repo.set_head("refs/heads/main").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let branches = handle.branches();
    let feature = branches
        .iter()
        .find(|b| b.name == "feature")
        .expect("feature branch present");
    assert_eq!(feature.ahead, 1, "feature is one commit ahead of main");
    assert_eq!(feature.behind, 1, "feature is one commit behind main");
}

#[test]
fn golden_remote_url_and_provider() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);
    repo.remote("origin", "https://github.com/rust-lang/cargo.git")
        .unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    let origin = info
        .remotes
        .iter()
        .find(|r| r.name == "origin")
        .expect("origin remote present");
    assert_eq!(
        origin.url.as_deref(),
        Some("https://github.com/rust-lang/cargo.git")
    );
    assert_eq!(origin.provider, GitHostingProvider::GitHub);
    assert_eq!(info.org.as_deref(), Some("rust-lang"));
    assert_eq!(info.repo.as_deref(), Some("cargo"));
}

#[test]
fn golden_config_all_twelve_keys() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Golden Tester").unwrap();
        config.set_str("user.email", "golden@sniff.test").unwrap();
        config.set_bool("gpg.use-agent", true).unwrap();
        config.set_str("gpg.program", "gpg2").unwrap();
        config.set_str("credential.helper", "store").unwrap();
        config.set_str("user.signingkey", "ABC123").unwrap();
        config.set_bool("commit.gpgsign", true).unwrap();
        config.set_bool("tag.gpgsign", false).unwrap();
        config.set_str("core.pager", "less").unwrap();
        config.set_str("delta.syntax-theme", "Dracula").unwrap();
        config.set_bool("delta.light", true).unwrap();
        config.set_bool("delta.side-by-side", false).unwrap();
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let config = handle.config();
    assert_eq!(config.user_name.as_deref(), Some("Golden Tester"));
    assert_eq!(config.user_email.as_deref(), Some("golden@sniff.test"));
    assert_eq!(config.gpg_use_agent, Some(true));
    assert_eq!(config.gpg_program.as_deref(), Some("gpg2"));
    assert_eq!(config.credential_helper.as_deref(), Some("store"));
    assert_eq!(config.signing_key.as_deref(), Some("ABC123"));
    assert_eq!(config.commit_sign, Some(true));
    assert_eq!(config.tag_sign, Some(false));
    assert_eq!(config.pager.as_deref(), Some("less"));
    assert_eq!(config.delta_syntax_theme.as_deref(), Some("Dracula"));
    assert_eq!(config.delta_light, Some(true));
    assert_eq!(config.delta_side_by_side, Some(false));
}

#[test]
fn golden_worktree_metadata() {
    let dir = TempDir::new().unwrap();
    builder::build_git_repo_with_worktrees(dir.path(), 2);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let worktrees = handle.worktrees().expect("worktrees should succeed");
    assert_eq!(worktrees.len(), 2, "two linked worktrees");

    for (branch, info) in &worktrees {
        assert!(
            branch == "wt000" || branch == "wt001",
            "unexpected worktree branch {branch}"
        );
        // Each linked worktree carries exactly one divergent commit.
        assert_eq!(
            info.ahead, 1,
            "worktree {branch} should be one commit ahead"
        );
        assert_eq!(info.behind, 0, "worktree {branch} should be zero behind");
        assert!(!info.merged, "worktree {branch} is not merged into base");
    }
}

#[test]
fn worktree_trusted_open_failure_is_propagated() {
    let dir = TempDir::new().unwrap();
    builder::build_git_repo_with_worktrees(dir.path(), 1);

    // Corrupt the linked worktree by removing its `.git` file so
    // `trusted_open` fails (not a repository).
    let wt_path = dir.path().join("_wt").join("wt000");
    let git_file = wt_path.join(".git");
    std::fs::remove_file(&git_file).expect("remove worktree .git file");
    assert!(!git_file.exists(), ".git file should be gone");

    // `get_worktrees` must propagate the open failure rather than
    // silently omitting the worktree or returning empty metadata.
    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let result = handle.worktrees();
    assert!(
        result.is_err(),
        "corrupted linked worktree must surface an error, got {result:?}"
    );
}

#[cfg(unix)]
#[test]
fn worktree_registry_error_is_propagated() {
    let dir = TempDir::new().unwrap();
    builder::build_git_repo_with_worktrees(dir.path(), 1);

    let worktrees_dir = dir.path().join(".git").join("worktrees");
    std::fs::set_permissions(&worktrees_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let result = handle.worktrees();

    std::fs::set_permissions(&worktrees_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        result.is_err(),
        "unreadable worktree registry must surface an error, got {result:?}"
    );
}

#[test]
fn worktree_proxy_base_error_is_propagated() {
    let dir = TempDir::new().unwrap();
    builder::build_git_repo_with_worktrees(dir.path(), 1);

    // Empty the gitdir file so proxy.base() fails (the proxy still
    // exists because the file was present when worktrees() scanned).
    let gitdir_file = dir
        .path()
        .join(".git")
        .join("worktrees")
        .join("wt000")
        .join("gitdir");
    std::fs::write(&gitdir_file, b"").expect("empty worktree gitdir file");

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let result = handle.worktrees();
    assert!(
        result.is_err(),
        "empty worktree gitdir must surface an error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Discovery tri-state (git2)
// ---------------------------------------------------------------------------

#[test]
fn discovery_found_returns_some() {
    let dir = TempDir::new().unwrap();
    build_linear_main(dir.path(), 1);

    let handle = GitRepo::discover(dir.path()).unwrap();
    assert!(handle.is_some(), "a real repo discovers to Ok(Some)");
}

#[test]
fn discovery_genuine_non_repository_returns_ok_none() {
    // A plain temp dir with no `.git` anywhere up to the filesystem root.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("plain.txt"), "not a repo\n").unwrap();

    let result = GitRepo::discover(dir.path()).unwrap();
    assert!(
        result.is_none(),
        "a non-repository directory discovers to Ok(None)"
    );
}

/// Creates a bare repository with one commit on `branch`.
fn init_bare_with_commit(parent: &Path, branch: &str) -> PathBuf {
    let bare_path = parent.join("bare.git");
    let repo = Repository::init_bare(&bare_path).unwrap();
    repo.set_head(&format!("refs/heads/{branch}")).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }
    bare_path
}

#[test]
fn discovery_bare_repository_is_a_valid_repo_with_head_queries() {
    // A bare repository has no working directory, but its refs, HEAD, and
    // objects are all readable — discovery must not reject it.
    let dir = TempDir::new().unwrap();
    let bare_path = init_bare_with_commit(dir.path(), "fixture/bare");

    let handle = GitRepo::discover(&bare_path)
        .expect("bare repo discovery must succeed")
        .expect("bare repo must be discovered");

    assert!(handle.is_bare(), "bare repo must report is_bare");
    assert_eq!(
        handle.current_branch(),
        Some("fixture/bare".to_string()),
        "bare repo with symbolic HEAD reports its attached branch"
    );
    assert_eq!(
        handle.try_current_branch().unwrap(),
        Some("fixture/bare".to_string())
    );
    assert_eq!(
        handle.try_current_worktree_name().unwrap(),
        None,
        "bare repo must not substitute a worktree name"
    );
    assert_eq!(
        handle.merge_conflicts().unwrap(),
        Vec::<PathBuf>::new(),
        "bare repo has no index and therefore no conflicts"
    );
}

#[test]
fn discovery_bare_repository_unborn_head_has_no_branch() {
    let dir = TempDir::new().unwrap();
    let bare_path: PathBuf = dir.path().join("bare.git");
    Repository::init_bare(&bare_path).unwrap();

    let handle = GitRepo::discover(&bare_path)
        .expect("bare repo discovery must succeed")
        .expect("bare repo must be discovered");

    assert!(handle.is_bare());
    assert_eq!(
        handle.current_branch(),
        None,
        "unborn HEAD has no branch even when the ref name is set"
    );
    assert_eq!(handle.merge_conflicts().unwrap(), Vec::<PathBuf>::new());
}

#[test]
fn discovery_bare_repository_detached_head_has_no_branch() {
    let dir = TempDir::new().unwrap();
    let bare_path = init_bare_with_commit(dir.path(), "fixture/bare");

    let repo = Repository::open(&bare_path).unwrap();
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.set_head_detached(head_commit.id()).unwrap();

    let handle = GitRepo::discover(&bare_path)
        .expect("bare repo discovery must succeed")
        .expect("bare repo must be discovered");

    assert!(handle.is_bare());
    assert!(handle.is_detached_head());
    assert_eq!(handle.current_branch(), None);
    assert_eq!(handle.try_current_branch().unwrap(), None);
}

// ---------------------------------------------------------------------------
// Phase 2 — gix-backed discovery and basic identity queries
// ---------------------------------------------------------------------------

#[test]
fn phase2_head_id_matches_git2_oid() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let git2_head = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();

    assert_eq!(
        handle.head_id().as_deref(),
        Some(git2_head.as_str()),
        "gix head_id should equal the git2 HEAD oid"
    );
    assert!(
        !handle.is_detached_head(),
        "HEAD is on a branch, not detached"
    );
}

#[test]
fn phase2_detached_head_is_detected_and_branch_is_none() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let head_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.set_head_detached(head_oid).unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(handle.is_detached_head(), "HEAD should be detached");
    assert_eq!(
        handle.current_branch(),
        None,
        "detached HEAD has no branch name"
    );
    // The id is still resolvable even when detached.
    assert_eq!(
        handle.head_id().as_deref(),
        Some(head_oid.to_string().as_str())
    );
}

#[test]
fn phase2_unborn_head_branch_and_id_are_none() {
    // A freshly initialized repository with no commits has an unborn HEAD.
    // The documented infallible accessors must suppress the underlying error
    // and report `None` rather than propagating or panicking.
    let dir = TempDir::new().unwrap();
    let _repo = builder::init_repo(dir.path());

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert_eq!(handle.current_branch(), None, "unborn HEAD has no branch");
    assert_eq!(handle.head_id(), None, "unborn HEAD has no commit id");
}

#[test]
fn phase2_main_repo_git_dir_equals_common_dir() {
    let dir = TempDir::new().unwrap();
    build_linear_main(dir.path(), 1);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(!handle.in_worktree(), "main repo is not a linked worktree");
    assert_eq!(
        norm(handle.git_dir()),
        norm(handle.common_dir()),
        "main repo git_dir and common_dir coincide"
    );
    assert_eq!(handle.base_repo_root(), None, "main repo has no base root");
}

#[test]
fn phase2_linked_worktree_paths_and_flags() {
    let dir = TempDir::new().unwrap();
    builder::build_git_repo_with_worktrees(dir.path(), 1);

    let wt_path = dir.path().join("_wt").join("wt000");
    let handle = GitRepo::discover(&wt_path)
        .unwrap()
        .expect("worktree found");

    assert!(
        handle.in_worktree(),
        "discovered handle is a linked worktree"
    );
    assert_ne!(
        norm(handle.git_dir()),
        norm(handle.common_dir()),
        "linked worktree git_dir differs from the shared common_dir"
    );
    let base = handle.base_repo_root().expect("worktree has a base root");
    // The base root is the main repository's working directory (parent of the
    // canonicalized common .git directory).
    let expected_base = std::fs::canonicalize(dir.path()).expect("canonicalize temp dir");
    assert_eq!(
        norm(&base),
        norm(&expected_base),
        "base_repo_root is the main repository's working directory"
    );
}

#[test]
fn phase2_discover_walks_up_from_subdirectory() {
    let dir = TempDir::new().unwrap();
    build_linear_main(dir.path(), 1);
    let nested = dir.path().join("src");

    let handle = GitRepo::discover(&nested).unwrap().expect("repo found");
    assert_eq!(
        norm(handle.repo_root()),
        norm(dir.path()),
        "discovery walks parents to the working-tree root"
    );
}

#[test]
fn phase2_sha256_repository_does_not_panic() {
    // sniff enables only the gix `sha1` feature, preserving the git2 SHA-1-only
    // contract. Where the host `git` can create a SHA-256 repository, discovery
    // and the basic accessors must produce a well-defined result (Ok/Err)
    // rather than panicking or misparsing a fixed-width object id.
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path().join("sha256");
    fs::create_dir_all(&repo_path).unwrap();

    let init = std::process::Command::new("git")
        .args(["init", "--object-format=sha256"])
        .current_dir(&repo_path)
        .output();
    let Ok(out) = init else {
        eprintln!("skipping: `git` not available");
        return;
    };
    if !out.status.success() {
        eprintln!("skipping: this git lacks --object-format=sha256");
        return;
    }

    // sniff enables only gix's `sha1` feature, so a SHA-256 repository is
    // unsupported. The contract is an explicit error (gix cannot load its
    // config) — distinct from both success and "not a repository" (`Ok(None)`),
    // so a SHA-256 repo is never silently mis-handled or reported as absent.
    let result = GitRepo::discover(&repo_path);
    assert!(
        result.is_err(),
        "SHA-256 repo must surface an error, not success or absence"
    );
}

/// Initialize an unsupported (SHA-256) repository, returning its path, or `None`
/// when the host `git` cannot create one.
fn sha256_repo(dir: &TempDir) -> Option<PathBuf> {
    let repo_path = dir.path().join("sha256");
    fs::create_dir_all(&repo_path).ok()?;
    let out = std::process::Command::new("git")
        .args(["init", "--object-format=sha256"])
        .current_dir(&repo_path)
        .output()
        .ok()?;
    out.status.success().then_some(repo_path)
}

#[test]
fn cli_facing_apis_surface_open_failure_instead_of_absence() {
    // An unsupported repository is a trust/permission/IO/corruption-class
    // failure: the CLI-facing helpers must propagate it (Err), never erase it to
    // "missing repo" / "no remote" / "no conflicts". This is the contract the
    // `.ok()`/`.ok().flatten()`/raw-`gix::open` swallowing used to violate.
    let dir = TempDir::new().unwrap();
    let Some(repo_path) = sha256_repo(&dir) else {
        eprintln!("skipping: host git cannot create a SHA-256 repository");
        return;
    };
    let p = repo_path.as_path();

    assert!(sniff::filesystem::repo_root(p).is_err(), "repo_root");
    assert!(
        sniff::filesystem::merge_conflicts_at(p).is_err(),
        "merge_conflicts_at"
    );
    assert!(
        sniff::filesystem::commit_by_sha_at(p, "deadbeef").is_err(),
        "commit_by_sha_at"
    );
    assert!(
        sniff::filesystem::commit_files_at(p, "deadbeef").is_err(),
        "commit_files_at"
    );
    assert!(
        sniff::filesystem::commits_for_path_at(p, "", 5).is_err(),
        "commits_for_path_at"
    );
    assert!(
        sniff::filesystem::commits_for_branch_at(p, "main", 5).is_err(),
        "commits_for_branch_at"
    );
}

/// Overwrite the loose object file for `sha` with garbage so any read/decode of
/// it fails. Git creates objects read-only, so make the file writable first.
fn corrupt_loose_object(repo_path: &Path, sha: &str) {
    let obj_path = repo_path
        .join(".git")
        .join("objects")
        .join(&sha[..2])
        .join(&sha[2..]);
    let mut perms = fs::metadata(&obj_path).unwrap().permissions();
    #[cfg(unix)]
    perms.set_mode(0o644);
    #[cfg(not(unix))]
    perms.set_readonly(false);
    fs::set_permissions(&obj_path, perms).unwrap();
    fs::write(&obj_path, b"garbage").unwrap();
}

/// Flip the trailing checksum byte of the index so a read detects the mismatch.
fn corrupt_index(repo_path: &Path) {
    let index_path = repo_path.join(".git").join("index");
    let mut bytes = fs::read(&index_path).unwrap();
    let len = bytes.len();
    assert!(len >= 20, "index must have a trailing checksum to corrupt");
    bytes[len - 1] = bytes[len - 1].wrapping_add(1);
    fs::write(&index_path, bytes).unwrap();
}

/// The SHA of the `main` tip via a fresh git2 handle.
fn head_sha(repo: &Repository) -> String {
    repo.head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string()
}

#[test]
fn commit_by_sha_at_surfaces_corrupt_commit_object() {
    // A corrupt commit object the SHA resolves to must surface as an error, not
    // be reported as "commit not found".
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    assert!(
        sniff::filesystem::commit_by_sha_at(dir.path(), &sha).is_err(),
        "corrupt commit object must surface through commit_by_sha_at"
    );
}

#[test]
fn commit_files_at_surfaces_corrupt_commit_object() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    assert!(
        sniff::filesystem::commit_files_at(dir.path(), &sha).is_err(),
        "corrupt commit object must surface through commit_files_at"
    );
}

#[test]
fn commit_files_at_surfaces_corrupt_parent_commit() {
    // The tip is intact but its parent is corrupt: computing the tip's file
    // changes diffs against the parent tree, so the corruption must surface.
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let tip = repo.head().unwrap().peel_to_commit().unwrap();
    let parent = tip.parent(0).unwrap().id().to_string();
    let tip_sha = tip.id().to_string();

    corrupt_loose_object(dir.path(), &parent);

    assert!(
        sniff::filesystem::commit_files_at(dir.path(), &tip_sha).is_err(),
        "corrupt parent commit must surface through commit_files_at"
    );
}

#[test]
fn commits_for_branch_at_surfaces_corrupt_ancestor() {
    // The tip resolves fine, but the revwalk reaches a corrupt ancestor: the
    // failure must surface rather than truncating history.
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);
    let root = git2_head_shas(&repo).last().unwrap().clone();

    corrupt_loose_object(dir.path(), &root);

    assert!(
        sniff::filesystem::commits_for_branch_at(dir.path(), "main", 10).is_err(),
        "corrupt ancestor must surface through commits_for_branch_at"
    );
}

#[test]
fn commits_for_path_at_surfaces_corrupt_ancestor() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);
    let root = git2_head_shas(&repo).last().unwrap().clone();

    corrupt_loose_object(dir.path(), &root);

    assert!(
        sniff::filesystem::commits_for_path_at(dir.path(), "", 10).is_err(),
        "corrupt ancestor must surface through commits_for_path_at"
    );
}

#[test]
fn merge_conflicts_at_surfaces_corrupt_index() {
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 1);

    corrupt_index(dir.path());

    assert!(
        sniff::filesystem::merge_conflicts_at(dir.path()).is_err(),
        "corrupt index must surface through merge_conflicts_at"
    );
}

// ---------------------------------------------------------------------------
// Fallible revision-resolution corruption parity (review-7 finding 1)
//
// The fallible entry points must only collapse genuine not-found / unborn /
// detached cases into None/empty. Malformed refs, refs targeting a missing
// object, and ambiguous lookups must surface as errors rather than masquerade
// as absence.
// ---------------------------------------------------------------------------

/// Full hex object IDs of every loose object in the repository.
fn loose_object_ids(repo_path: &Path) -> Vec<String> {
    let objects = repo_path.join(".git").join("objects");
    let mut ids = Vec::new();
    for entry in fs::read_dir(&objects).unwrap() {
        let entry = entry.unwrap();
        let dir_name = entry.file_name().into_string().unwrap();
        if dir_name.len() != 2 || !entry.path().is_dir() {
            continue; // skip `info/`, `pack/`, etc.
        }
        for file in fs::read_dir(entry.path()).unwrap() {
            let rest = file.unwrap().file_name().into_string().unwrap();
            ids.push(format!("{dir_name}{rest}"));
        }
    }
    ids
}

/// A SHA prefix shared by at least two distinct objects, so it resolves
/// ambiguously. With >16 loose objects a 1-char collision is guaranteed by the
/// pigeonhole principle; the shared prefix of the first colliding sorted pair is
/// returned.
fn ambiguous_prefix(repo_path: &Path) -> String {
    let mut ids = loose_object_ids(repo_path);
    ids.sort();
    for pair in ids.windows(2) {
        let shared = pair[0]
            .chars()
            .zip(pair[1].chars())
            .take_while(|(a, b)| a == b)
            .count();
        if shared >= 1 {
            return pair[0][..shared].to_string();
        }
    }
    panic!(
        "no ambiguous object prefix found among {} objects",
        ids.len()
    );
}

#[test]
fn commit_by_sha_at_surfaces_ambiguous_prefix() {
    // An ambiguous prefix is not "no commit matches" — it must surface rather
    // than collapse into Ok(None).
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 20);

    let prefix = ambiguous_prefix(dir.path());
    assert!(
        sniff::filesystem::commit_by_sha_at(dir.path(), &prefix).is_err(),
        "ambiguous prefix {prefix:?} must surface through commit_by_sha_at"
    );
}

#[test]
fn commit_by_sha_at_returns_none_for_unmatched_prefix() {
    // A well-formed hex prefix that matches no object is genuine absence.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let result = sniff::filesystem::commit_by_sha_at(dir.path(), "deadbeef").unwrap();
    assert!(
        result.is_none(),
        "an unmatched SHA prefix must be Ok(None), got {result:?}"
    );
}

#[test]
fn commits_for_path_at_surfaces_malformed_head() {
    // A malformed HEAD is corruption, not an unborn branch.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    assert!(
        sniff::filesystem::commits_for_path_at(dir.path(), "", 10).is_err(),
        "malformed HEAD must surface through commits_for_path_at"
    );
}

#[test]
fn commits_for_branch_at_surfaces_malformed_branch_ref() {
    // A malformed requested branch ref must surface rather than fall through to
    // an empty history.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    fs::write(
        dir.path()
            .join(".git")
            .join("refs")
            .join("heads")
            .join("broken"),
        b"not a valid ref target\n",
    )
    .unwrap();

    assert!(
        sniff::filesystem::commits_for_branch_at(dir.path(), "broken", 10).is_err(),
        "malformed branch ref must surface through commits_for_branch_at"
    );
}

#[test]
fn commits_for_branch_at_surfaces_branch_ref_to_missing_object() {
    // A branch ref that peels to a missing object is corruption, not absence.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    fs::write(
        dir.path()
            .join(".git")
            .join("refs")
            .join("heads")
            .join("broken"),
        b"0000000000000000000000000000000000000000\n",
    )
    .unwrap();

    assert!(
        sniff::filesystem::commits_for_branch_at(dir.path(), "broken", 10).is_err(),
        "branch ref to a missing object must surface through commits_for_branch_at"
    );
}

#[test]
fn commits_for_branch_at_surfaces_malformed_remote_tracking_ref() {
    // A malformed `refs/remotes/origin/main` must surface rather than fall
    // through to an empty history. `origin/main` is non-hex, so without a
    // structured ref lookup the SHA-prefix probe would report it as absent.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let remote_ref = dir
        .path()
        .join(".git")
        .join("refs")
        .join("remotes")
        .join("origin")
        .join("main");
    fs::create_dir_all(remote_ref.parent().unwrap()).unwrap();
    fs::write(&remote_ref, b"not a valid ref target\n").unwrap();

    assert!(
        sniff::filesystem::commits_for_branch_at(dir.path(), "origin/main", 10).is_err(),
        "malformed remote-tracking ref must surface through commits_for_branch_at"
    );
}

#[test]
fn commits_for_branch_at_surfaces_remote_tracking_ref_to_missing_object() {
    // A remote-tracking ref that peels to a missing object is corruption.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let remote_ref = dir
        .path()
        .join(".git")
        .join("refs")
        .join("remotes")
        .join("origin")
        .join("main");
    fs::create_dir_all(remote_ref.parent().unwrap()).unwrap();
    fs::write(&remote_ref, b"0000000000000000000000000000000000000000\n").unwrap();

    assert!(
        sniff::filesystem::commits_for_branch_at(dir.path(), "origin/main", 10).is_err(),
        "remote-tracking ref to a missing object must surface through commits_for_branch_at"
    );
}

// ---------------------------------------------------------------------------
// Unresolved-branch absence is Ok(empty), never an error (review-9 finding 2)
//
// An absent branch must not be inferred as a corrupt SHA from its characters.
// A name that is non-hex, too short, or too long for an object-ID prefix is
// genuine branch absence, not an operational failure.
// ---------------------------------------------------------------------------

#[test]
fn commits_for_branch_at_absent_short_hex_is_empty() {
    // `add` is hex but 3 chars — below gix's 4-char minimum object-ID prefix.
    // It must be treated as an absent branch, not a malformed-SHA error.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let commits = sniff::filesystem::commits_for_branch_at(dir.path(), "add", 10)
        .expect("absent short-hex branch name must be Ok(empty)");
    assert!(
        commits.is_empty(),
        "absent branch must produce empty history"
    );
}

#[test]
fn commits_for_branch_at_absent_valid_length_hex_is_empty() {
    // A validly-shaped hex prefix that matches no object resolves to empty
    // history rather than an error.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let commits = sniff::filesystem::commits_for_branch_at(dir.path(), "abcdef12", 10)
        .expect("absent valid-length-hex branch name must be Ok(empty)");
    assert!(
        commits.is_empty(),
        "absent branch must produce empty history"
    );
}

#[test]
fn commits_for_branch_at_absent_ordinary_branch_is_empty() {
    // A non-hex absent branch name returns empty history.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let commits = sniff::filesystem::commits_for_branch_at(dir.path(), "nonexistent", 10)
        .expect("absent ordinary branch name must be Ok(empty)");
    assert!(
        commits.is_empty(),
        "absent branch must produce empty history"
    );
}

// ---------------------------------------------------------------------------
// Packed checked-out branch parity (review-8 finding 1)
//
// After `git pack-refs --all --prune` the checked-out branch may exist only in
// `packed-refs`. Branch name, current-branch flag, tracking status, and full
// detection must be identical to the loose-ref shape.
// ---------------------------------------------------------------------------

/// Pack the loose ref for `branch` into `packed-refs` and delete the loose
/// file, mirroring `git pack-refs --all --prune` for the checked-out branch.
fn pack_and_prune_branch(repo_path: &Path, branch: &str) {
    let git = repo_path.join(".git");
    let loose = git.join("refs").join("heads").join(branch);
    let oid = fs::read_to_string(&loose).unwrap().trim().to_string();
    fs::write(
        git.join("packed-refs"),
        format!("# pack-refs with: peeled fully-peeled\n{oid} refs/heads/{branch}\n"),
    )
    .unwrap();
    fs::remove_file(&loose).unwrap();
}

#[test]
fn packed_checkout_branch_reports_name_tracking_and_detection() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 3);

    // A remote + remote-tracking ref pointing at HEAD so tracking status has
    // something to report (0 ahead / 0 behind — tips are equal).
    repo.remote("origin", "https://github.com/o/r.git").unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.reference("refs/remotes/origin/main", head.id(), true, "remote tip")
        .unwrap();

    // refs/heads/main now lives only in packed-refs.
    pack_and_prune_branch(dir.path(), "main");

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");

    assert_eq!(
        handle.current_branch().as_deref(),
        Some("main"),
        "a packed checked-out branch must still report its name"
    );

    let tracking = handle.try_tracking_status().unwrap();
    let origin = tracking
        .iter()
        .find(|t| t.remote == "origin")
        .expect("origin tracking present for packed branch");
    assert_eq!((origin.ahead, origin.behind), (0, 0));

    let info = handle.detect_with_request(&GitRequest::full()).unwrap();
    assert_eq!(
        info.current_branch.as_deref(),
        Some("main"),
        "full detection must carry the packed branch name"
    );
    // The checked-out branch is marked current: ahead/behind are zeroed for it.
    let main = info
        .branches
        .iter()
        .find(|b| b.name == "main")
        .expect("main branch present in detection");
    assert_eq!((main.ahead, main.behind), (0, 0));
}

// ---------------------------------------------------------------------------
// Fallible HEAD error policy (review-8 finding 3)
//
// `try_branches`, `try_tracking_status`, and a metadata-producing detection
// request must surface a missing/malformed HEAD instead of producing
// successful-looking branch metadata with zeroed ahead/behind values.
// ---------------------------------------------------------------------------

#[test]
fn malformed_head_surfaces_through_try_branches() {
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);
    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.try_branches().is_err(),
        "malformed HEAD must surface through try_branches"
    );
}

#[test]
fn malformed_head_surfaces_through_try_tracking_status() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    repo.remote("origin", "https://example.com/r.git").unwrap();
    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.try_tracking_status().is_err(),
        "malformed HEAD must surface through try_tracking_status"
    );
}

#[test]
fn malformed_head_surfaces_through_metadata_detection() {
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);
    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.detect_with_request(&GitRequest::full()).is_err(),
        "malformed HEAD must surface through a metadata-producing detection request"
    );
}

#[test]
fn malformed_head_surfaces_through_minimal_detection() {
    // `minimal()` skips branch/tracking collection, so the fallible HEAD query
    // at the top of `detect_with_request` is the only place its corruption can
    // surface — it must not return `Ok` with `current_branch: None`.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);
    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.detect_with_request(&GitRequest::minimal()).is_err(),
        "malformed HEAD must surface through a minimal detection request"
    );
}

#[test]
fn malformed_head_surfaces_through_summary_detection() {
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);
    fs::write(dir.path().join(".git").join("HEAD"), b"not a valid head\n").unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.detect_with_request(&GitRequest::summary()).is_err(),
        "malformed HEAD must surface through a summary detection request"
    );
}

#[test]
fn missing_head_surfaces_through_minimal_detection() {
    // Delete HEAD *after* discovery so the detection call itself — not the
    // discovery step — exercises the failure path.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    fs::remove_file(dir.path().join(".git").join("HEAD")).unwrap();

    assert!(
        handle.detect_with_request(&GitRequest::minimal()).is_err(),
        "a HEAD missing at detection time must surface through a minimal request"
    );
}

#[test]
fn detached_head_to_missing_object_surfaces_through_try_branches() {
    // HEAD detaches to a missing object, so the branch name resolves to `None`
    // (legitimately detached) but the HEAD-identity (`head_id`) lookup fails.
    // Local branches still peel cleanly, so only the HEAD-identity query can
    // surface the corruption — guarding the `head_id_opt` propagation.
    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 2);
    fs::write(
        dir.path().join(".git").join("HEAD"),
        b"0000000000000000000000000000000000000000\n",
    )
    .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    assert!(
        handle.try_branches().is_err(),
        "a detached HEAD naming a missing object must surface through try_branches"
    );
}

// ---------------------------------------------------------------------------
// Full detection + recent-commit corruption parity (review-7 finding 2)
//
// The primary detection API and every recent-commit query return `Result`, so
// repository corruption must surface as an error instead of a successful but
// empty/partial history.
// ---------------------------------------------------------------------------

#[test]
fn detect_git_with_request_surfaces_corrupt_history() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    let request = GitRequest::full().commit_count(10);
    assert!(
        detect_git_with_request(dir.path(), &request).is_err(),
        "corrupt history must surface through detect_git_with_request"
    );
}

#[test]
fn recent_commits_by_count_surfaces_corrupt_history() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    assert!(
        sniff::filesystem::get_recent_commits_by_count(dir.path(), 10).is_err(),
        "corrupt history must surface through get_recent_commits_by_count"
    );
}

#[test]
fn recent_commits_by_duration_surfaces_corrupt_history() {
    use chrono::Duration;
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    assert!(
        sniff::filesystem::get_recent_commits_by_duration(dir.path(), Duration::weeks(520), "5y")
            .is_err(),
        "corrupt history must surface through get_recent_commits_by_duration"
    );
}

#[test]
fn recent_commits_in_range_surfaces_corrupt_history() {
    use chrono::{TimeZone, Utc};
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    let since = Utc.timestamp_opt(0, 0).unwrap();
    let until = Utc::now();
    assert!(
        sniff::filesystem::get_recent_commits_in_range(dir.path(), since, until, "all").is_err(),
        "corrupt history must surface through get_recent_commits_in_range"
    );
}

#[test]
fn recent_commits_by_date_surfaces_corrupt_history() {
    use chrono::NaiveDate;
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    let date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    assert!(
        sniff::filesystem::get_recent_commits_by_date(dir.path(), date).is_err(),
        "corrupt history must surface through get_recent_commits_by_date"
    );
}

#[test]
fn recent_commits_by_hash_surfaces_corrupt_history() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);
    let sha = head_sha(&repo);

    corrupt_loose_object(dir.path(), &sha);

    // Targeting HEAD itself skips the merge-base reachability probe, so the
    // corruption surfaces through the history collector rather than the probe.
    assert!(
        sniff::filesystem::get_recent_commits_by_hash(dir.path(), &sha).is_err(),
        "corrupt history must surface through get_recent_commits_by_hash"
    );
}

#[test]
fn ref_decorations_match_git2_for_branches_and_tags() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);

    // A second local branch and a lightweight tag, both at HEAD.
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &head, false).unwrap();
    repo.tag_lightweight("v1.0", head.as_object(), false)
        .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let commits = handle.recent_commits(2);
    let head_sha = head.id().to_string();
    let head_commit = commits
        .iter()
        .find(|c| c.sha == head_sha)
        .expect("HEAD commit present");

    let names: Vec<&str> = head_commit.refs.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"main"), "got: {names:?}");
    assert!(names.contains(&"feature"), "got: {names:?}");
    assert!(names.contains(&"v1.0"), "got: {names:?}");

    // The active branch is `main`, and it sorts first (HEAD decoration).
    assert_eq!(head_commit.refs[0].name, "main");
    let main = head_commit.refs.iter().find(|r| r.name == "main").unwrap();
    assert!(main.is_head, "main should be the HEAD branch");
    assert_eq!(main.kind, RefKind::LocalBranch);

    let feature = head_commit
        .refs
        .iter()
        .find(|r| r.name == "feature")
        .unwrap();
    assert!(!feature.is_head);
    assert_eq!(feature.kind, RefKind::LocalBranch);

    let tag = head_commit.refs.iter().find(|r| r.name == "v1.0").unwrap();
    assert_eq!(tag.kind, RefKind::Tag);
    assert!(!tag.is_head);
}

#[test]
fn ref_decorations_include_remote_tracking_refs() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    // Create a fake remote-tracking ref pointing at HEAD.
    repo.reference("refs/remotes/origin/main", head.id(), true, "fake remote")
        .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let commits = handle.recent_commits(2);
    let head_sha = head.id().to_string();
    let head_commit = commits
        .iter()
        .find(|c| c.sha == head_sha)
        .expect("HEAD commit present");

    let remote = head_commit
        .refs
        .iter()
        .find(|r| r.name == "origin/main")
        .expect("origin/main decoration present");
    assert_eq!(remote.kind, RefKind::RemoteBranch);
    assert!(!remote.is_head);
}

#[test]
fn ref_decorations_resolve_symbolic_remote_head() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    // Add a real remote so remote_names() includes "origin".
    repo.remote("origin", "https://example.com/repo.git")
        .unwrap();
    // Create refs/remotes/origin/HEAD as a symbolic ref to origin/main.
    repo.reference(
        "refs/remotes/origin/main",
        head.id(),
        true,
        "fake remote branch",
    )
    .unwrap();
    repo.reference_symbolic(
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
        true,
        "fake remote HEAD",
    )
    .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let remotes = handle.remotes(true);
    let origin = remotes
        .iter()
        .find(|r| r.name == "origin")
        .expect("origin present");
    assert_eq!(
        origin.default_branch.as_deref(),
        Some("main"),
        "symbolic refs/remotes/origin/HEAD should resolve to 'main'"
    );
}

#[test]
fn ref_decorations_peel_annotated_tags() {
    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 2);

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    // Create an annotated tag (tag object, not direct ref).
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.tag(
        "v2.0",
        head.as_object(),
        &sig,
        "annotated tag message",
        false,
    )
    .unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let commits = handle.recent_commits(2);
    let head_sha = head.id().to_string();
    let head_commit = commits
        .iter()
        .find(|c| c.sha == head_sha)
        .expect("HEAD commit present");

    let tag = head_commit
        .refs
        .iter()
        .find(|r| r.name == "v2.0")
        .expect("v2.0 annotation present");
    assert_eq!(tag.kind, RefKind::Tag);
    assert!(!tag.is_head);
}

// ---------------------------------------------------------------------------
// Phase 3 — gix-backed working-tree status parity
// ---------------------------------------------------------------------------

/// Build a deterministic repo with one committed file and return the git2 handle.
fn repo_with_one_file(root: &Path, filename: &str, content: &str) -> Repository {
    let repo = builder::init_repo(root);
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");
    builder::write_file(&root.join(filename), content);
    builder::commit_all_at(&repo, "initial", 1_000);
    repo
}

#[test]
fn phase3_clean_repo_reports_zero_counts_and_not_dirty() {
    let dir = TempDir::new().unwrap();
    let _repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(!info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().untracked_count, 0);
    assert!(info.file_changes.is_empty());
}

#[test]
fn phase3_staged_add_is_created_with_staged_status() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    fs::write(dir.path().join("new.rs"), "pub const X: u32 = 1;\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("new.rs")).unwrap();
    index.write().unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 1);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 0);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "new.rs")
        .expect("new.rs change present");
    assert_eq!(change.status, FileStatus::Staged);
    assert_eq!(change.action, FileAction::Created);
    assert_eq!(change.lines_added, 1);
    assert_eq!(change.lines_removed, 0);
}

#[test]
fn phase3_staged_delete_is_deleted_with_staged_status() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "old.rs", "pub const X: u32 = 1;\n");

    fs::remove_file(dir.path().join("old.rs")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.rs")).unwrap();
    index.write().unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 1);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 0);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "old.rs")
        .expect("old.rs change present");
    assert_eq!(change.status, FileStatus::Staged);
    assert_eq!(change.action, FileAction::Deleted);
    assert_eq!(change.lines_added, 0);
    assert_eq!(change.lines_removed, 1);
}

#[test]
fn phase3_unstaged_modification_is_modified_with_modified_status() {
    let dir = TempDir::new().unwrap();
    let _repo = repo_with_one_file(dir.path(), "file.txt", "line one\nline two\n");

    fs::write(dir.path().join("file.txt"), "line one\nmodified two\n").unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 1);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "file.txt")
        .expect("file.txt change present");
    assert_eq!(change.status, FileStatus::Modified);
    assert_eq!(change.action, FileAction::Modified);
    assert_eq!(change.lines_added, 1);
    assert_eq!(change.lines_removed, 1);
}

#[test]
fn phase3_unstaged_delete_is_deleted_with_modified_status() {
    let dir = TempDir::new().unwrap();
    let _repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    fs::remove_file(dir.path().join("file.txt")).unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 1);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "file.txt")
        .expect("file.txt change present");
    assert_eq!(change.status, FileStatus::Modified);
    assert_eq!(change.action, FileAction::Deleted);
    assert_eq!(change.lines_added, 0);
    assert_eq!(change.lines_removed, 1);
}

#[test]
fn phase3_untracked_file_is_created_with_untracked_status() {
    let dir = TempDir::new().unwrap();
    let _repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    fs::write(dir.path().join("scratch.txt"), "scratch\n").unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().untracked_count, 1);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "scratch.txt")
        .expect("scratch.txt change present");
    assert_eq!(change.status, FileStatus::Untracked);
    assert_eq!(change.action, FileAction::Created);
}

#[test]
fn phase3_both_staged_and_unstaged_reports_both_status() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "a\nb\nc\n");

    // Stage a modification.
    fs::write(dir.path().join("file.txt"), "a\nB\nc\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();

    // Then modify again without staging.
    fs::write(dir.path().join("file.txt"), "a\nB\nC\n").unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 1);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 1);

    let change = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "file.txt")
        .expect("file.txt change present");
    assert_eq!(change.status, FileStatus::Both);
    assert_eq!(change.action, FileAction::Modified);
    // Stats aggregate staged (1 add / 1 rem) and unstaged (1 add / 1 rem).
    assert_eq!(change.lines_added, 2);
    assert_eq!(change.lines_removed, 2);
}

#[test]
fn phase3_rename_surfaces_as_delete_and_create() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "old.rs", "pub const X: u32 = 1;\n");

    fs::rename(dir.path().join("old.rs"), dir.path().join("new.rs")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.rs")).unwrap();
    index.add_path(Path::new("new.rs")).unwrap();
    index.write().unwrap();

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 2);

    let paths: Vec<String> = info.file_changes.iter().map(|c| norm(&c.path)).collect();
    assert!(paths.contains(&"old.rs".to_string()));
    assert!(paths.contains(&"new.rs".to_string()));
}

#[test]
fn phase3_merge_conflict_detected_and_reported() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "base\n");

    // Create a branch with a conflicting change.
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("other", &head, false).unwrap();
    repo.set_head("refs/heads/other").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    fs::write(dir.path().join("file.txt"), "theirs\n").unwrap();
    builder::commit_all_at(&repo, "theirs", 2_000);

    // Switch back to main and make an incompatible change.
    repo.set_head("refs/heads/main").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    fs::write(dir.path().join("file.txt"), "ours\n").unwrap();
    builder::commit_all_at(&repo, "ours", 3_000);

    // Merge other into main. This will produce conflicts.
    let other_oid = repo
        .find_reference("refs/heads/other")
        .unwrap()
        .target()
        .unwrap();
    let other_annotated = repo.find_annotated_commit(other_oid).unwrap();
    repo.merge(&[&other_annotated], None, None).ok();

    let conflicts = merge_conflicts_at(dir.path()).unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(norm(&conflicts[0]), "file.txt");

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    let conflict = info
        .file_changes
        .iter()
        .find(|c| norm(&c.path) == "file.txt")
        .expect("conflict present");
    assert_eq!(conflict.status, FileStatus::Conflicted);
}

#[test]
fn phase3_unborn_head_is_not_dirty_and_has_no_changes() {
    let dir = TempDir::new().unwrap();
    let _repo = builder::init_repo(dir.path());

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(!info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().unstaged_count, 0);
    assert_eq!(info.status.as_ref().unwrap().untracked_count, 0);
    assert!(info.file_changes.is_empty());
}

/// Inject a staged blob whose index path is the raw `bytes` (not necessarily
/// valid UTF-8). Bypasses the filesystem — macOS APFS rejects invalid-UTF-8
/// filenames — to exercise the byte-native status path directly.
fn stage_raw_path(repo: &git2::Repository, bytes: &[u8], content: &[u8]) {
    let mut index = repo.index().unwrap();
    index
        .add_frombuffer(
            &git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: content.len() as u32,
                id: repo.blob(content).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: bytes.to_vec(),
            },
            content,
        )
        .unwrap();
    index.write().unwrap();
}

#[test]
fn phase3_non_utf8_path_resolves_exact_index_entry_for_stats() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    // 0xC0 is an invalid UTF-8 lead byte. A lossy round-trip would re-encode it
    // as the replacement character and fail the index lookup (zero stats); the
    // byte-native path must resolve the real entry and count its lines.
    stage_raw_path(&repo, &[0xC0, b'r', b's', b't'], b"one\ntwo\nthree\n");

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert!(info.status.as_ref().unwrap().is_dirty);
    assert_eq!(info.status.as_ref().unwrap().staged_count, 1);
    // The index entry has no worktree file, so it surfaces as a staged add
    // (3 lines added) plus a worktree delete (3 lines removed). Both stats are
    // non-zero only because the exact byte path resolved its index blob; a
    // lossy round-trip would have failed the lookup and reported zero.
    let change = info
        .file_changes
        .iter()
        .find(|c| c.lines_added > 0)
        .expect("staged non-UTF-8 file with stats");
    assert_eq!(change.lines_added, 3, "exact byte path must find the blob");
    assert_eq!(change.lines_removed, 3);
}

#[test]
fn phase3_distinct_non_utf8_paths_do_not_collide() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    // Two distinct invalid-UTF-8 paths that both lossily render to a single
    // replacement character. A `PathBuf`-keyed map would collapse them into one
    // entry; byte-native keys must keep them distinct.
    stage_raw_path(&repo, &[0xC0], b"a\n");
    stage_raw_path(&repo, &[0xC1], b"b\n");

    let info = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    assert_eq!(
        info.status.as_ref().unwrap().staged_count,
        2,
        "distinct byte paths must not collapse to one"
    );
    assert_eq!(
        info.file_changes
            .iter()
            .filter(|c| c.lines_added > 0)
            .count(),
        2,
        "both non-UTF-8 files must carry independent stats"
    );
}

#[test]
fn phase3_counts_only_matches_full_request_totals() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    fs::write(dir.path().join("staged.rs"), "staged\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("staged.rs")).unwrap();
    index.write().unwrap();

    fs::write(dir.path().join("unstaged.rs"), "unstaged\n").unwrap();
    fs::write(dir.path().join("untracked.rs"), "untracked\n").unwrap();

    let full = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    // A request with no file changes but non-minimal metadata triggers the
    // detailed-count code path. Its totals must agree with the full walk.
    let counts = detect_git_with_request(
        dir.path(),
        &GitRequest {
            commit_count: 0,
            include_file_changes: false,
            include_file_diffs: false,
            include_worktrees: true, // keeps is_minimal() false
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
            full_worktree_details: false,
            identity_only: false,
        },
    )
    .unwrap()
    .expect("repo found");

    assert_eq!(
        counts.status.as_ref().unwrap().is_dirty,
        full.status.as_ref().unwrap().is_dirty
    );
    assert_eq!(
        counts.status.as_ref().unwrap().staged_count,
        full.status.as_ref().unwrap().staged_count
    );
    assert_eq!(
        counts.status.as_ref().unwrap().unstaged_count,
        full.status.as_ref().unwrap().unstaged_count
    );
    assert_eq!(
        counts.status.as_ref().unwrap().untracked_count,
        full.status.as_ref().unwrap().untracked_count
    );
}

#[test]
fn phase3_detailed_counts_match_full_request_totals() {
    let dir = TempDir::new().unwrap();
    let repo = repo_with_one_file(dir.path(), "file.txt", "hello\n");

    fs::write(dir.path().join("staged.rs"), "staged\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("staged.rs")).unwrap();
    index.write().unwrap();

    fs::write(dir.path().join("unstaged.rs"), "unstaged\n").unwrap();
    fs::write(dir.path().join("untracked.rs"), "untracked\n").unwrap();

    let full = detect_git_with_request(dir.path(), &GitRequest::full())
        .unwrap()
        .expect("repo found");

    // GitRequest::deep() triggers the detailed path listing but omits diffs.
    let detailed = detect_git_with_request(dir.path(), &GitRequest::deep())
        .unwrap()
        .expect("repo found");

    assert_eq!(
        detailed.status.as_ref().unwrap().is_dirty,
        full.status.as_ref().unwrap().is_dirty
    );
    assert_eq!(
        detailed.status.as_ref().unwrap().staged_count,
        full.status.as_ref().unwrap().staged_count
    );
    assert_eq!(
        detailed.status.as_ref().unwrap().unstaged_count,
        full.status.as_ref().unwrap().unstaged_count
    );
    assert_eq!(
        detailed.status.as_ref().unwrap().untracked_count,
        full.status.as_ref().unwrap().untracked_count
    );

    // Detailed includes path lists; the count of dirty + untracked entries
    // should equal the file_changes length (excluding conflicts, which this
    // fixture does not produce).
    assert_eq!(
        detailed.status.as_ref().unwrap().dirty.len()
            + detailed.status.as_ref().unwrap().untracked.len(),
        full.file_changes.len()
    );
}

// ---------------------------------------------------------------------------
// Phase 4 — gix-backed commit tree-to-tree diff parity
// ---------------------------------------------------------------------------

/// Helper: open a gix handle from a path for phase-4 tests.
fn open_gix(root: &Path) -> gix::Repository {
    gix::open(root).expect("open with gix")
}

/// Helper: resolve a full git2 SHA to a gix ObjectId.
fn sha_to_gix_oid(sha: &str) -> gix::ObjectId {
    gix::ObjectId::from_hex(sha.as_bytes()).expect("valid full sha")
}

#[test]
fn phase4_root_commit_shows_all_files_as_added() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("alpha.txt"), "alpha\n");
    builder::write_file(&dir.path().join("src/lib.rs"), "pub fn lib() {}\n");
    builder::commit_all_at(&repo, "initial", 1_000);

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert_eq!(files.len(), 2, "root commit should list all added files");
    let paths: Vec<String> = files.iter().map(|(p, _)| norm(p)).collect();
    assert!(paths.contains(&"alpha.txt".to_string()));
    assert!(paths.contains(&"src/lib.rs".to_string()));

    for (_, kind) in &files {
        assert_eq!(*kind, DeltaKind::Added, "root commit files should be Added");
    }
}

#[test]
fn phase4_first_parent_commit_with_modifications() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("file.txt"), "v0\n");
    builder::commit_all_at(&repo, "initial", 1_000);

    builder::write_file(&dir.path().join("file.txt"), "v1\n");
    builder::commit_all_at(&repo, "modify", 2_000);

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert_eq!(files.len(), 1);
    assert_eq!(norm(&files[0].0), "file.txt");
    assert_eq!(files[0].1, DeltaKind::Modified);
}

#[test]
fn phase4_commit_with_deletions() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("keep.txt"), "keep\n");
    builder::write_file(&dir.path().join("remove.txt"), "remove\n");
    builder::commit_all_at(&repo, "initial", 1_000);

    std::fs::remove_file(dir.path().join("remove.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("remove.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "delete", &tree, &[&parent])
        .unwrap();

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert_eq!(files.len(), 1);
    assert_eq!(norm(&files[0].0), "remove.txt");
    assert_eq!(files[0].1, DeltaKind::Deleted);
}

#[test]
fn phase4_rename_surfaces_as_delete_and_add() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("old.rs"), "pub const X: u32 = 1;\n");
    builder::commit_all_at(&repo, "initial", 1_000);

    std::fs::rename(dir.path().join("old.rs"), dir.path().join("new.rs")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.rs")).unwrap();
    index.add_path(Path::new("new.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "rename", &tree, &[&parent])
        .unwrap();

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert_eq!(files.len(), 2, "rename tracking disabled → delete + add");
    let kinds: Vec<DeltaKind> = files.iter().map(|(_, k)| *k).collect();
    assert!(kinds.contains(&DeltaKind::Deleted));
    assert!(kinds.contains(&DeltaKind::Added));
}

#[test]
fn phase4_binary_file_in_commit() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("data.bin"), "\x00\x01\x02\x03\n");
    builder::commit_all_at(&repo, "add binary", 1_000);

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert_eq!(files.len(), 1);
    assert_eq!(norm(&files[0].0), "data.bin");
    assert_eq!(files[0].1, DeltaKind::Added);
}

#[test]
fn phase4_empty_commit_has_no_files() {
    let dir = TempDir::new().unwrap();
    let repo = builder::init_repo(dir.path());
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");

    builder::write_file(&dir.path().join("file.txt"), "hello\n");
    builder::commit_all_at(&repo, "initial", 1_000);

    // Empty commit: same tree as parent
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = parent.tree().unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "empty", &tree, &[&parent])
        .unwrap();

    let head_sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    let gix_repo = open_gix(dir.path());
    let files = get_commit_files(&gix_repo, sha_to_gix_oid(&head_sha));

    assert!(
        files.is_empty(),
        "empty commit should have no changed files"
    );
}

// ---------------------------------------------------------------------------
// Phase 5 — gix-backed refs, branches, and config parity
// ---------------------------------------------------------------------------

// Linux-only: macOS APFS/HFS+ reject invalid-UTF-8 filenames, so the
// fixture cannot be created there. The non-UTF-8 path policy is covered
// by the path-based tests above on all Unix platforms.
#[cfg(target_os = "linux")]
#[test]
fn phase5_non_utf8_ref_is_not_silently_dropped() {
    use std::os::unix::ffi::OsStrExt;

    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);

    // Create a branch with a non-UTF-8 name by writing the ref file directly.
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let refs_dir = repo.path().join("refs").join("heads");
    let bad_name = std::ffi::OsStr::from_bytes(b"bad\xC0ref");
    let ref_path = refs_dir.join(bad_name);
    std::fs::write(&ref_path, format!("{}\n", head.id())).unwrap();

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let branches = handle.branches();
    let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();

    // The ref must not be silently dropped; it appears with the Unicode
    // replacement character (U+FFFD) where the invalid UTF-8 sequence was.
    assert!(
        names.iter().any(|n| n.contains('\u{FFFD}')),
        "non-UTF-8 ref must appear with replacement character, got: {names:?}"
    );
}

#[test]
fn config_local_overrides_global() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);

    // Set up a fake HOME with a global gitconfig.
    let fake_home = TempDir::new().unwrap();
    let global_config = fake_home.path().join(".gitconfig");
    std::fs::write(
        &global_config,
        "[user]\n\tname = Global Tester\n\temail = global@sniff.test\n",
    )
    .unwrap();

    // Set local config values that should override global.
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Local Tester").unwrap();
        config.set_str("user.email", "local@sniff.test").unwrap();
    }

    // Point GIT_CONFIG_GLOBAL at the fake file so gix reads it.
    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let cfg = handle.config();

    // Restore before assertions so panics don't leak the change.
    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    assert_eq!(
        cfg.user_name.as_deref(),
        Some("Local Tester"),
        "local config must override global"
    );
    assert_eq!(
        cfg.user_email.as_deref(),
        Some("local@sniff.test"),
        "local config must override global"
    );
}

#[test]
fn config_global_fallback_when_no_local_value() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 1);

    let fake_home = TempDir::new().unwrap();
    let global_config = fake_home.path().join(".gitconfig");
    std::fs::write(
        &global_config,
        "[user]\n\tname = Global Tester\n\temail = global@sniff.test\n",
    )
    .unwrap();

    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let cfg = handle.config();

    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    assert_eq!(
        cfg.user_name.as_deref(),
        Some("Global Tester"),
        "global config must be read when local is absent"
    );
    assert_eq!(
        cfg.user_email.as_deref(),
        Some("global@sniff.test"),
        "global config must be read when local is absent"
    );
}

#[test]
fn config_system_global_local_precedence() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let repo = build_linear_main(dir.path(), 1);

    // Create fake system and global configs.
    let system_config = dir.path().join("system.gitconfig");
    let global_config = dir.path().join("global.gitconfig");
    std::fs::write(
        &system_config,
        "[user]\n\tname = System Tester\n\temail = system@sniff.test\n",
    )
    .unwrap();
    std::fs::write(
        &global_config,
        "[user]\n\tname = Global Tester\n\temail = global@sniff.test\n",
    )
    .unwrap();

    // Set local values that should override both.
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Local Tester").unwrap();
        config.set_str("user.email", "local@sniff.test").unwrap();
    }

    let original_system = std::env::var_os("GIT_CONFIG_SYSTEM");
    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_SYSTEM", &system_config);
    }
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let cfg = handle.config();

    match original_system {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_SYSTEM", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_SYSTEM") },
    }
    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    assert_eq!(
        cfg.user_name.as_deref(),
        Some("Local Tester"),
        "local must override global and system"
    );
    assert_eq!(
        cfg.user_email.as_deref(),
        Some("local@sniff.test"),
        "local must override global and system"
    );
}

#[test]
fn config_system_fallback_when_no_local_or_global_value() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 1);

    let system_config = dir.path().join("system.gitconfig");
    std::fs::write(
        &system_config,
        "[user]\n\tname = System Tester\n\temail = system@sniff.test\n",
    )
    .unwrap();

    // Create an empty global config to suppress the real ~/.gitconfig.
    let global_config = dir.path().join("global.gitconfig");
    std::fs::write(&global_config, "").unwrap();

    let original_system = std::env::var_os("GIT_CONFIG_SYSTEM");
    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_SYSTEM", &system_config);
    }
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let cfg = handle.config();

    match original_system {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_SYSTEM", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_SYSTEM") },
    }
    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    assert_eq!(
        cfg.user_name.as_deref(),
        Some("System Tester"),
        "system config must be read when local and global are absent"
    );
    assert_eq!(
        cfg.user_email.as_deref(),
        Some("system@sniff.test"),
        "system config must be read when local and global are absent"
    );
}

#[test]
fn config_all_twelve_keys_from_global_source() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 1);

    let fake_home = TempDir::new().unwrap();
    let global_config = fake_home.path().join(".gitconfig");
    std::fs::write(
        &global_config,
        r#"[user]
	name = Global Tester
	email = global@sniff.test
	signingkey = GLOBAL123
[gpg]
	use-agent = true
	program = gpg2
[credential]
	helper = store
[commit]
	gpgsign = true
[tag]
	gpgsign = false
[core]
	pager = less
[delta]
	syntax-theme = Dracula
	light = true
	side-by-side = false
"#,
    )
    .unwrap();
    let _ = &fake_home; // keep temp dir alive

    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let config = handle.config();

    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    assert_eq!(config.user_name.as_deref(), Some("Global Tester"));
    assert_eq!(config.user_email.as_deref(), Some("global@sniff.test"));
    assert_eq!(config.gpg_use_agent, Some(true));
    assert_eq!(config.gpg_program.as_deref(), Some("gpg2"));
    assert_eq!(config.credential_helper.as_deref(), Some("store"));
    assert_eq!(config.signing_key.as_deref(), Some("GLOBAL123"));
    assert_eq!(config.commit_sign, Some(true));
    assert_eq!(config.tag_sign, Some(false));
    assert_eq!(config.pager.as_deref(), Some("less"));
    assert_eq!(config.delta_syntax_theme.as_deref(), Some("Dracula"));
    assert_eq!(config.delta_light, Some(true));
    assert_eq!(config.delta_side_by_side, Some(false));
}

#[cfg(target_os = "macos")]
#[test]
fn config_extra_system_file_is_lowest_precedence() {
    let _guard = ENV_LOCK.lock().unwrap();

    let dir = TempDir::new().unwrap();
    let _repo = build_linear_main(dir.path(), 1);

    // Create a fake global config.
    let fake_home = TempDir::new().unwrap();
    let global_config = fake_home.path().join(".gitconfig");
    std::fs::write(
        &global_config,
        "[user]\n\tname = Global Tester\n\temail = global@sniff.test\n",
    )
    .unwrap();

    let original_global = std::env::var_os("GIT_CONFIG_GLOBAL");
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global_config);
    }

    let handle = GitRepo::discover(dir.path()).unwrap().expect("repo found");
    let cfg = handle.config();

    match original_global {
        Some(h) => unsafe { std::env::set_var("GIT_CONFIG_GLOBAL", h) },
        None => unsafe { std::env::remove_var("GIT_CONFIG_GLOBAL") },
    }

    // Global must win over the (possibly absent) CLT extra config. The real
    // value of this test is that it exercises the code path on macOS; when the
    // CLT file is absent the fallback is a no-op, and when present it is
    // lowest-precedence, so global always dominates.
    assert_eq!(
        cfg.user_name.as_deref(),
        Some("Global Tester"),
        "global config must take precedence over extra system fallback"
    );
}
