//! Checked-out-repos detection and broadcast.
//!
//! Fills this host's `repos/{node_id}` register with a map of canonical
//! repo id → HEAD commit hash for every git checkout found under the
//! configured scan roots. The future scheduler uses it to prefer hosts
//! that already have a job's repository on local storage (and to see
//! how fresh that checkout is).
//!
//! Ratified design notes (host-capability spec D1/D3):
//!
//! - This lives in its own register — NOT the capability register —
//!   because its cadence (every commit) is far hotter than the cold
//!   hardware fields, and isolating the churn keeps the capability
//!   document permanently cold.
//! - Refresh is interval-driven for now; a post-commit event source
//!   can tighten it later without changing the register shape.
//! - Repo identity uses the shared canonicalization in
//!   `rendezvous_core::canonical_repo_id` (spec S4: one form,
//!   everywhere). Checkouts without a canonicalizable remote have no
//!   mesh-wide identity and are skipped.
//! - When several checkouts of the same repo exist (e.g. a base clone
//!   plus worktrees), they share one canonical id and the last one
//!   scanned wins — "the host has this repo" matters more than which
//!   checkout's HEAD is reported.

use std::path::{Path, PathBuf};

use serde_json::{Map as JsonMap, Value as JsonValue, json};
use sniff::filesystem::{detect_git_with_request, preferred_remote_url};
use sniff::request::GitRequest;

use crate::register::{RegisterError, RegisterStore};

/// How deep below each scan root to look for `.git` before giving up.
/// Deep enough for `root/org/repo` and `root/area/worktrees/name`
/// layouts, shallow enough that a scan stays milliseconds-cheap.
const SCAN_MAX_DEPTH: usize = 4;

/// Run one scan pass over `roots` and replace the local repos register
/// with the result. Returns `true` when the register changed. With no
/// roots configured the register is left untouched (the feature is
/// opt-in until a scan location is known).
pub fn refresh_repos(store: &RegisterStore, roots: &[PathBuf]) -> Result<bool, RegisterError> {
    if roots.is_empty() {
        return Ok(false);
    }
    let fields = detect_repos(roots);
    // Replace (not upsert): a checkout deleted from disk must leave
    // the register.
    store.replace_local_fields(&store.local_repos_id(), &fields)
}

/// Scan `roots` for git checkouts and return canonical repo id → HEAD
/// commit hash.
#[must_use]
pub fn detect_repos(roots: &[PathBuf]) -> JsonMap<String, JsonValue> {
    let mut out = JsonMap::new();
    for root in roots {
        walk(root, 0, &mut out);
    }
    out
}

/// Directories under a scan root, hidden entries, and symlinks are
/// skipped; descent stops at the first `.git` (a repo's *contents* are
/// never walked, so junk like `node_modules` inside repos costs
/// nothing).
fn walk(dir: &Path, depth: usize, out: &mut JsonMap<String, JsonValue>) {
    // `.git` is a directory in a normal clone and a FILE in a linked
    // worktree; `exists()` covers both.
    if dir.join(".git").exists() {
        probe_repo(dir, out);
        return;
    }
    if depth >= SCAN_MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        walk(&entry.path(), depth + 1, out);
    }
}

/// Resolve one checkout to (canonical id, HEAD sha) and record it.
/// Failures degrade to skipping the checkout — a repo the daemon
/// cannot identify simply is not advertised.
fn probe_repo(path: &Path, out: &mut JsonMap<String, JsonValue>) {
    let url = match preferred_remote_url(path) {
        Ok(Some(url)) => url,
        Ok(None) => return, // remote-less checkout: no mesh identity
        Err(err) => {
            tracing::debug!(
                target: "rendezvous_daemon::repos",
                path = %path.display(),
                %err,
                "skipping checkout: remote lookup failed",
            );
            return;
        }
    };
    let Some(repo_id) = rendezvous_core::canonical_repo_id(&url) else {
        return;
    };
    match detect_git_with_request(path, &GitRequest::identity()) {
        Ok(Some(info)) => {
            if let Some(head) = info.head_id {
                out.insert(repo_id, json!(head));
            }
        }
        Ok(None) => {}
        Err(err) => {
            tracing::debug!(
                target: "rendezvous_daemon::repos",
                path = %path.display(),
                %err,
                "skipping checkout: git identity probe failed",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use rendezvous_core::NodeIdentity;
    use std::process::Command;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// `true` when a usable `git` binary is on PATH; the fixture tests
    /// build real repositories with it and skip (loudly) when absent.
    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    }

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .expect("run git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }

    /// Create a real repo at `root/<name>` with one commit and an
    /// origin remote, returning its HEAD sha.
    fn make_repo(root: &Path, name: &str, remote: &str) -> String {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).expect("mkdir");
        git(&dir, &["init", "--quiet", "--initial-branch=main"]);
        git(&dir, &["remote", "add", "origin", remote]);
        std::fs::write(dir.join("README.md"), name).expect("write");
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        let out = Command::new("git")
            .current_dir(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("rev-parse");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn scan_finds_repos_and_skips_remoteless() {
        if !git_available() {
            eprintln!("skipping: git binary not available");
            return;
        }
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("coding");
        std::fs::create_dir_all(root.join("not-a-repo/deeper")).expect("mkdir");

        let widget_head = make_repo(&root, "widget", "git@github.com:acme/widget.git");
        let nested_head = make_repo(
            &root.join("group"),
            "gadget",
            "https://gitlab.com/acme/gadget.git",
        );
        // Remote-less checkout: valid repo, no mesh identity.
        let orphan = root.join("orphan");
        std::fs::create_dir_all(&orphan).expect("mkdir");
        git(&orphan, &["init", "--quiet", "--initial-branch=main"]);

        let repos = detect_repos(&[root]);
        assert_eq!(
            repos
                .get("github.com/acme/widget")
                .and_then(JsonValue::as_str),
            Some(widget_head.as_str()),
        );
        assert_eq!(
            repos
                .get("gitlab.com/acme/gadget")
                .and_then(JsonValue::as_str),
            Some(nested_head.as_str()),
        );
        assert_eq!(repos.len(), 2, "orphan must be skipped: {repos:?}");
    }

    #[test]
    fn refresh_replaces_and_removes_deleted_checkouts() {
        if !git_available() {
            eprintln!("skipping: git binary not available");
            return;
        }
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("coding");
        let storage = Storage::open(tmp.path().join("repos.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([6u8; 32]));
        let store = RegisterStore::new(storage, identity).expect("store");

        make_repo(&root, "widget", "git@github.com:acme/widget.git");
        make_repo(&root, "gadget", "git@github.com:acme/gadget.git");
        let roots = vec![root.clone()];
        assert!(refresh_repos(&store, &roots).expect("first refresh"));

        let value = store
            .deep_value(&store.local_repos_id())
            .expect("read")
            .expect("present");
        assert!(value.get("github.com/acme/widget").is_some());
        assert!(value.get("github.com/acme/gadget").is_some());

        // Unchanged disk state: no write.
        assert!(!refresh_repos(&store, &roots).expect("steady state"));

        // Deleting a checkout removes it from the register.
        std::fs::remove_dir_all(root.join("gadget")).expect("rm");
        assert!(refresh_repos(&store, &roots).expect("after delete"));
        let value = store
            .deep_value(&store.local_repos_id())
            .expect("read")
            .expect("present");
        assert!(value.get("github.com/acme/widget").is_some());
        assert!(
            value.get("github.com/acme/gadget").is_none(),
            "deleted checkout must leave the register: {value}",
        );
    }

    #[test]
    fn empty_roots_leave_register_untouched() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("repos.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([8u8; 32]));
        let store = RegisterStore::new(storage, identity).expect("store");
        assert!(!refresh_repos(&store, &[]).expect("noop"));
        assert!(
            store
                .deep_value(&store.local_repos_id())
                .expect("read")
                .is_none()
        );
    }
}
