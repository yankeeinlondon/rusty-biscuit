//! Real-cache proof of the worktree-lifecycle outcome in
//! `2026-09-23-ensuring-kache-support` (Verification step 3): a newly created
//! Git worktree restores its dependencies from the shared kache store, with no
//! kache setup step between worktrees and no material store growth.
//!
//! The test drives the real installed `kache` binary, a real Cargo build, and
//! real `git worktree` commands against a scratch store. It builds the
//! committed fixture workspace (`tests/fixtures/kache-worktree`: three chained
//! library crates, no registry dependencies) in worktree A, removes A, creates
//! worktree B, and builds again. It proves that:
//!
//! - B's build reports one local hit per fixture crate and no miss, counted by
//!   `kache stats --json` as the delta over A's totals;
//! - kache's own store view does not grow: `entries` and `disk.store_bytes`
//!   after B equal those after A;
//! - the artifact tree on disk (`<store>/store`, excluding `staging/` and
//!   `*.lock` files) is byte-for-byte the same size and file count after B as
//!   after A, so nothing was stored twice;
//! - the whole store directory, SQLite's write-ahead journal aside, grows by
//!   at most [`CHURN_TOLERANCE_BYTES`], which covers the event log and index
//!   that record B's hits.
//!
//! Isolation: every child runs with a cleared environment. `HOME`,
//! `XDG_CONFIG_HOME`, `CARGO_HOME`, and `TMPDIR` point inside the scratch
//! root; the kache user config there pins `[cache] local_store` with
//! `ignore_env = true`, so an inherited `KACHE_CACHE_DIR` cannot redirect it
//! (none is passed anyway). The test owns a foreground `kache daemon run`
//! whose socket lives in the scratch store — the stand-in for a host's standing
//! daemon — and stops it on drop. Without it, `kache stats` tries to start a
//! background daemon of its own. The host's store, config, daemon, and
//! `$CARGO_HOME/config.toml` are never read or written.
//!
//! Real tier (`real_`): it needs kache installed and a scratch directory on a
//! filesystem that clones (APFS, btrfs, XFS-reflink), the only hosts where the
//! repository enables kache (`docs/kache-strategy.md`). Either missing skips
//! with a `SKIP:` line; `BISCUIT_KACHE_REAL_REQUIRED=1` turns the skip into a
//! failure. The store and both worktrees share one temporary root, so they
//! share a filesystem.

#![cfg(unix)]

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tempfile::TempDir;

/// Set to `1` to fail instead of skip when kache or a cloning filesystem is
/// missing.
const REQUIRED_VAR: &str = "BISCUIT_KACHE_REAL_REQUIRED";

/// One cacheable rlib per crate in the fixture workspace.
const FIXTURE_CRATES: u64 = 3;

/// Upper bound on whole-store growth from B's build, SQLite's write-ahead
/// journal excluded (see [`is_not_sqlite_journal`]). B's hits append event-log
/// lines and build-session records (observed: about 9.7 KiB on APFS with
/// kache 0.26.3, against 30 KiB of fixture artifacts); a re-stored artifact
/// would instead show in the artifact tree, which must not change at all.
const CHURN_TOLERANCE_BYTES: u64 = 64 * 1024;

/// Deadline for the daemon to answer and for `kache stats` to reflect a build;
/// below nextest's 30 s termination so a miss reports its own diagnosis.
const DEADLINE: Duration = Duration::from_secs(10);

/// The daemon binds Unix sockets inside the store, and `sun_path` holds
/// 104 bytes on macOS. macOS's per-user `$TMPDIR` (`/var/folders/…/T/`) leaves
/// too little room, so a long temp dir falls back to `/tmp`.
const MAX_SCRATCH_BASE_LEN: usize = 48;

#[test]
fn real_kache_restores_a_new_worktree_from_the_shared_store_without_growth() {
    let Some(kache) = find_on_path("kache") else {
        skip_or_require("`kache` is not on PATH");
        return;
    };
    let scratch = Scratch::new(kache);
    if !scratch.filesystem_clones() {
        skip_or_require(&format!(
            "the scratch filesystem at {} cannot clone files",
            scratch.root().display()
        ));
        return;
    }

    scratch.init_repository();
    let _daemon = ScratchDaemon::start(&scratch);
    let baseline = scratch.stats();

    // Worktree A populates the store.
    let worktree_a = scratch.root().join("wt-a");
    scratch.git_in_repo(&["worktree", "add", "-q", path_str(&worktree_a), "-b", "a"]);
    scratch.cargo_build(&worktree_a);
    let after_a =
        scratch.wait_for_stats(|stats| stats.lookups() >= baseline.lookups() + FIXTURE_CRATES);
    assert_eq!(
        (
            after_a.local_hits - baseline.local_hits,
            after_a.misses - baseline.misses
        ),
        (0, FIXTURE_CRATES),
        "worktree A must compile every fixture crate into an empty store"
    );
    assert_eq!(
        after_a.entries, FIXTURE_CRATES,
        "one store entry per fixture crate"
    );
    let artifacts_a = tree_size(&scratch.store().join("store"), is_artifact);
    let total_a = tree_size(scratch.store(), is_not_sqlite_journal);

    // Remove A before B exists, so B can only restore from the store.
    scratch.git_in_repo(&["worktree", "remove", "--force", path_str(&worktree_a)]);
    assert!(
        !worktree_a.exists(),
        "worktree A must be gone before B is built"
    );

    let worktree_b = scratch.root().join("wt-b");
    scratch.git_in_repo(&["worktree", "add", "-q", path_str(&worktree_b), "-b", "b"]);
    scratch.cargo_build(&worktree_b);
    let after_b =
        scratch.wait_for_stats(|stats| stats.lookups() >= after_a.lookups() + FIXTURE_CRATES);
    let artifacts_b = tree_size(&scratch.store().join("store"), is_artifact);
    let total_b = tree_size(scratch.store(), is_not_sqlite_journal);

    eprintln!(
        "observed: A misses={} entries={} store_bytes={} artifacts={}B/{} files store_dir={}B; \
         B hits={} misses={} entries={} store_bytes={} artifacts={}B/{} files store_dir={}B",
        after_a.misses - baseline.misses,
        after_a.entries,
        after_a.store_bytes,
        artifacts_a.bytes,
        artifacts_a.files,
        total_a.bytes,
        after_b.local_hits - after_a.local_hits,
        after_b.misses - after_a.misses,
        after_b.entries,
        after_b.store_bytes,
        artifacts_b.bytes,
        artifacts_b.files,
        total_b.bytes,
    );

    assert_eq!(
        (
            after_b.local_hits - after_a.local_hits,
            after_b.misses - after_a.misses
        ),
        (FIXTURE_CRATES, 0),
        "worktree B must restore every fixture crate from the store"
    );
    for crate_name in ["alpha", "beta", "gamma"] {
        assert!(
            has_rlib(&worktree_b, crate_name),
            "worktree B must hold the restored rlib for kache_fixture_{crate_name}"
        );
    }
    assert_eq!(
        (after_b.entries, after_b.store_bytes),
        (after_a.entries, after_a.store_bytes),
        "kache's store view (entries, store_bytes) must not grow for worktree B"
    );
    assert_eq!(
        artifacts_b, artifacts_a,
        "the on-disk artifact tree must not grow for worktree B"
    );
    let growth = total_b.bytes.saturating_sub(total_a.bytes);
    assert!(
        growth <= CHURN_TOLERANCE_BYTES,
        "the store directory grew by {growth} bytes for worktree B, above the \
         {CHURN_TOLERANCE_BYTES}-byte event-log/index allowance"
    );
}

fn skip_or_require(reason: &str) {
    if env::var(REQUIRED_VAR).as_deref() == Ok("1") {
        panic!("{REQUIRED_VAR}=1, but {reason}");
    }
    eprintln!("SKIP: {reason}; set {REQUIRED_VAR}=1 to fail instead");
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("scratch paths are UTF-8")
}

/// The temporary root that holds the store, the Git repository, and both
/// worktrees, plus the isolated home the children see.
struct Scratch {
    // Held for its drop, which deletes the tree.
    _root: TempDir,
    canonical_root: PathBuf,
    store: PathBuf,
    kache: PathBuf,
    inherited_path: OsString,
    rustup_home: Option<OsString>,
    rustup_toolchain: Option<OsString>,
}

impl Scratch {
    fn new(kache: PathBuf) -> Self {
        let temp_dir = env::temp_dir();
        let base = match temp_dir.canonicalize() {
            Ok(base) if base.as_os_str().len() <= MAX_SCRATCH_BASE_LEN => base,
            _ => PathBuf::from("/tmp"),
        };
        let root = tempfile::Builder::new()
            .prefix("real-kache-wt-")
            .tempdir_in(base)
            .expect("create scratch root");
        // kache records build roots canonically; macOS's temp dir is behind
        // the `/var` -> `/private/var` symlink.
        let canonical_root = root
            .path()
            .canonicalize()
            .expect("canonicalize scratch root");
        for dir in ["home/.config/kache", "cargo-home", "store", "tmp", "repo"] {
            fs::create_dir_all(canonical_root.join(dir)).expect("create scratch dir");
        }
        let store = canonical_root.join("store");
        let store_str = path_str(&store);
        assert!(
            !store_str.contains('\''),
            "store path must fit a TOML literal string"
        );
        fs::write(
            canonical_root.join("home/.config/kache/config.toml"),
            format!("[cache]\nlocal_store = '{store_str}'\nignore_env = true\n"),
        )
        .expect("write scratch kache config");

        // The children get a scratch HOME, so rustup must be told where the
        // real toolchains live.
        let rustup_home = env::var_os("RUSTUP_HOME")
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup").into()));
        Self {
            _root: root,
            canonical_root,
            store,
            kache,
            inherited_path: env::var_os("PATH").unwrap_or_default(),
            rustup_home,
            rustup_toolchain: env::var_os("RUSTUP_TOOLCHAIN"),
        }
    }

    fn root(&self) -> &Path {
        &self.canonical_root
    }

    fn store(&self) -> &Path {
        &self.store
    }

    fn command(&self, program: &Path, cwd: &Path) -> Command {
        let home = self.root().join("home");
        let mut command = Command::new(program);
        command
            .env_clear()
            .current_dir(cwd)
            .env("PATH", &self.inherited_path)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("CARGO_HOME", self.root().join("cargo-home"))
            .env("TMPDIR", self.root().join("tmp"))
            .env("GIT_CONFIG_NOSYSTEM", "1");
        if let Some(rustup_home) = &self.rustup_home {
            command.env("RUSTUP_HOME", rustup_home);
        }
        if let Some(toolchain) = &self.rustup_toolchain {
            command.env("RUSTUP_TOOLCHAIN", toolchain);
        }
        command
    }

    fn kache_command(&self) -> Command {
        self.command(&self.kache, self.root())
    }

    fn filesystem_clones(&self) -> bool {
        let source = self.root().join("tmp/clone-probe-source");
        let target = self.root().join("tmp/clone-probe-target");
        fs::write(&source, b"clone probe").expect("write clone probe");
        let flag = if cfg!(target_os = "macos") {
            "-c"
        } else if cfg!(target_os = "linux") {
            "--reflink=always"
        } else {
            return false;
        };
        let cloned = Command::new("cp")
            .arg(flag)
            .arg(&source)
            .arg(&target)
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let _ = fs::remove_file(&target);
        cloned
    }

    fn init_repository(&self) {
        let fixture = biscuit_test_harness::manifest_dir!().join("tests/fixtures/kache-worktree");
        let repo = self.root().join("repo");
        copy_tree(&fixture, &repo);
        fs::write(repo.join(".gitignore"), "/target\n").expect("write .gitignore");
        self.git_in_repo(&["init", "-q", "-b", "main"]);
        self.git_in_repo(&["add", "-A"]);
        self.git_in_repo(&["commit", "-q", "-m", "kache worktree fixture"]);
    }

    fn git_in_repo(&self, args: &[&str]) {
        let output = self
            .command(Path::new("git"), &self.root().join("repo"))
            .args([
                "-c",
                "user.name=kache fixture",
                "-c",
                "user.email=kache-fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("run git");
        assert_success(&output, &format!("git {}", args.join(" ")));
    }

    fn cargo_build(&self, worktree: &Path) {
        let output = self
            .command(Path::new("cargo"), worktree)
            .env("RUSTC_WRAPPER", &self.kache)
            .args(["build", "--offline", "--locked"])
            .output()
            .expect("run cargo build");
        assert_success(&output, &format!("cargo build in {}", worktree.display()));
    }

    fn stats(&self) -> Stats {
        let output = self
            .kache_command()
            .args(["stats", "--json", "--since", "1h"])
            .output()
            .expect("run kache stats");
        assert_success(&output, "kache stats --json");
        let document: Value =
            serde_json::from_slice(&output.stdout).expect("kache stats --json is JSON");
        let field = |pointer: &str| {
            document
                .pointer(pointer)
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("kache stats --json lacks {pointer}: {document}"))
        };
        assert_eq!(
            document.pointer("/daemon_connected"),
            Some(&Value::Bool(true)),
            "kache stats must reach the scratch daemon"
        );
        Stats {
            local_hits: field("/local_hits"),
            misses: field("/misses"),
            entries: field("/entries"),
            store_bytes: field("/disk/store_bytes"),
        }
    }

    /// Polls `kache stats` until `done` holds; the wrapper's events can land
    /// after Cargo exits.
    fn wait_for_stats(&self, done: impl Fn(&Stats) -> bool) -> Stats {
        let started = Instant::now();
        loop {
            let stats = self.stats();
            if done(&stats) {
                return stats;
            }
            assert!(
                started.elapsed() < DEADLINE,
                "kache stats did not reach the expected totals within {DEADLINE:?}: {stats:?}"
            );
            thread::sleep(Duration::from_millis(200));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Stats {
    local_hits: u64,
    misses: u64,
    entries: u64,
    store_bytes: u64,
}

impl Stats {
    /// Every recorded cache decision for a crate, hit or miss.
    fn lookups(&self) -> u64 {
        self.local_hits + self.misses
    }
}

/// A foreground `kache daemon run` bound to the scratch store, stopped on drop.
struct ScratchDaemon<'a> {
    scratch: &'a Scratch,
    child: Child,
}

impl<'a> ScratchDaemon<'a> {
    fn start(scratch: &'a Scratch) -> Self {
        let log = fs::File::create(scratch.root().join("daemon.log")).expect("create daemon log");
        let child = scratch
            .kache_command()
            .args(["daemon", "run"])
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("clone daemon log handle"))
            .stderr(log)
            .spawn()
            .expect("spawn kache daemon run");
        let mut daemon = Self { scratch, child };
        daemon.wait_until_ready();
        daemon
    }

    fn wait_until_ready(&mut self) {
        let started = Instant::now();
        loop {
            if let Ok(Some(exit)) = self.child.try_wait() {
                panic!(
                    "the scratch kache daemon exited with {exit}; log: {}",
                    fs::read_to_string(self.scratch.root().join("daemon.log")).unwrap_or_default()
                );
            }
            let output = self
                .scratch
                .kache_command()
                .args(["daemon", "status", "--json"])
                .output()
                .expect("run kache daemon status");
            if let Ok(status) = serde_json::from_slice::<Value>(&output.stdout)
                && status.pointer("/daemon_running") == Some(&Value::Bool(true))
            {
                // Guards against answering for any daemon but this one.
                let socket = status
                    .pointer("/socket")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                assert!(
                    Path::new(socket).starts_with(self.scratch.store()),
                    "the daemon socket {socket} is outside the scratch store"
                );
                return;
            }
            assert!(
                started.elapsed() < DEADLINE,
                "the scratch kache daemon did not come up within {DEADLINE:?}; log: {}",
                fs::read_to_string(self.scratch.root().join("daemon.log")).unwrap_or_default()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for ScratchDaemon<'_> {
    fn drop(&mut self) {
        let _ = self
            .scratch
            .kache_command()
            .args(["daemon", "stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
struct TreeSize {
    bytes: u64,
    files: u64,
}

/// Sums regular-file sizes under `dir` for the files `include` accepts,
/// without following symlinks.
fn tree_size(dir: &Path, include: impl Fn(&Path) -> bool + Copy) -> TreeSize {
    let mut size = TreeSize::default();
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
    {
        let entry = entry.expect("read directory entry");
        let path = entry.path();
        let file_type = entry.file_type().expect("read file type");
        if file_type.is_dir() {
            if include(&path) {
                let inner = tree_size(&path, include);
                size.bytes += inner.bytes;
                size.files += inner.files;
            }
        } else if file_type.is_file() && include(&path) {
            size.bytes += entry.metadata().expect("read file metadata").len();
            size.files += 1;
        }
    }
    size
}

/// Excludes SQLite's write-ahead journal (`index.db-wal`, `index.db-shm`):
/// it grows with every index write until SQLite checkpoints it back into
/// `index.db`, so its size tracks checkpoint timing, not stored bytes.
fn is_not_sqlite_journal(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| !name.ends_with("-wal") && !name.ends_with("-shm"))
}

/// Artifact-tree membership: everything but in-flight staging and lock files.
fn is_artifact(path: &Path) -> bool {
    path.file_name().is_none_or(|name| name != "staging")
        && path.extension().is_none_or(|extension| extension != "lock")
}

fn has_rlib(worktree: &Path, crate_name: &str) -> bool {
    let prefix = format!("libkache_fixture_{crate_name}-");
    fs::read_dir(worktree.join("target/debug/deps"))
        .expect("read worktree deps directory")
        .filter_map(Result::ok)
        .any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&prefix) && name.ends_with(".rlib")
        })
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture copy dir");
    for entry in
        fs::read_dir(source).unwrap_or_else(|error| panic!("read {}: {error}", source.display()))
    {
        let entry = entry.expect("read fixture entry");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

fn assert_success(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
