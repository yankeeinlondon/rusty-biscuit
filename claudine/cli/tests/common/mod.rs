//! Shared helpers for the `claudine-cli` Level 1 test binaries.
//!
//! ## The L1 spawn contract
//!
//! [`CliProcessFixture`] is the supported way an L1 test in this directory
//! obtains a `claudine` command. [`CliProcessFixture::command`] returns a
//! command that runs against a workspace the test built — never against the
//! checkout the suite was compiled from — so a test is hermetic by
//! construction rather than by a per-test `.env(...)` chain.
//! `spawn_site_guard.rs` keeps the alternatives — a raw
//! `assert_cmd::Command::cargo_bin("claudine")` and a [`claudine_bin`]
//! shell-out — out of the other L1 binaries. Its allow-list is empty: every
//! L1 binary now goes through this builder.
//!
//! The contract does not end at the spawn. [`CliProcessFixture::command`]
//! returns a bare `assert_cmd::Command`, so a call site could undo the pinned
//! `current_dir` or the composed `PATH` on the way to `.assert()` and leave the
//! spawn form untouched. `spawn_site_guard.rs` carries a second gate for that:
//! in the migrated L1 files it flags `.current_dir(…)`, `.env("PATH", …)`,
//! `.env_remove("PATH")`, `.env_clear()`, and any direct reach for
//! [`augmented_path`]. The escapes below are how those needs are spelled.
//!
//! The default command pins `current_dir` to the fixture `cwd`, points
//! `HOME`/`USERPROFILE`/`APPDATA`/`LOCALAPPDATA` at the fixture `home`,
//! removes `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME`, and sets
//! `CLAUDINE_RENDEZVOUS_REPORT=false`, `NO_COLOR=1`, `PLAYA_DRY_RUN=1`, and a
//! fixture-local `PLAYA_SPOOL_DIR`.
//!
//! ### Why audio is a spawn-contract concern
//!
//! A lifecycle audio effect does not play in-process: claudine re-execs
//! *itself* as playa's detached spool worker, which outlives the command that
//! enqueued the job — by design, so a doorbell survives the CLI exiting. An L1
//! test that reaches one therefore leaves two orphaned `claudine` processes on
//! the developer's machine and plays a sound through their speakers, neither of
//! which any assertion asked for. `PLAYA_DRY_RUN=1` is what keeps the effect a
//! decision the test can observe instead of a process it has to own, and the
//! fixture-local spool keeps a job that *is* under test out of the shared
//! per-user root. `detached_audio.rs`, whose subject is the worker itself, is
//! the one file that opts back in.
//!
//! ### The inheritance contract
//!
//! A child inherits the parent's whole environment by default, so the builder
//! also *removes* three families the developer's shell routinely exports:
//!
//! - the whole `CLAUDINE_*` namespace, by prefix — an exported
//!   `CLAUDINE_STEP_TIMEOUT` otherwise re-parameterizes the timeout tests;
//! - the `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/`GIT_COMMON_DIR`/
//!   `GIT_OBJECT_DIRECTORY` plumbing family, which overrides cwd-based
//!   repository discovery and so defeats the pinned `current_dir`;
//! - the rendering inputs `TERM_WIDTH`, `COLUMNS`, and `FORCE_COLOR`, so
//!   claudine's documented 80-column fallback applies and `FORCE_COLOR` cannot
//!   out-vote the `NO_COLOR=1` above.
//!
//! Removal is per key at build time and scrubs only what was *inherited*: a
//! call site that sets any of these on the returned command still wins, and a
//! test that needs a specific render width pins it there rather than relying on
//! whatever the terminal exported.
//!
//! ### The default `PATH` rule
//!
//! `PATH` is the fixture `bin` directory followed by a minimal platform system
//! set (`/usr/bin:/bin` on Unix, `%SystemRoot%\System32` on Windows), with
//! `PATHEXT` left alone so `.cmd` stubs resolve. A fake-only `PATH` would be
//! stricter but claudine itself spawns `sh` and `cmd` by bare name — for
//! lifecycle shell actions, sequence shell tasks, and darkmatter's `$SHELL`
//! alias expansion — so fake-only breaks shell-shaped tests inside claudine
//! rather than in the fixture, and the opt-out would become the norm. Host
//! providers install under Homebrew, npm, cargo, and `~/.local/bin` prefixes,
//! none of which the minimal set contains, so provider discovery still cannot
//! see the machine.
//!
//! ### The temp-directory precondition
//!
//! The fixture workspace is built under `std::env::temp_dir()`, so a temp
//! directory that itself sits inside the rusty-biscuit checkout puts every
//! "isolated" workspace *inside* the checkout, where claudine's repository
//! discovery walks straight back out to it. [`CliProcessFixture::named`]
//! rejects that at construction — see [`checkout_containment_error`] — because
//! the alternative is a suite that gets slower and reports the symptom
//! somewhere else.
//!
//! ### Opt-outs
//!
//! Three escapes exist on [`ClaudineCommandBuilder`]. Each requires a comment
//! at the call site naming the tool it needs or the proof it depends on:
//!
//! - [`ClaudineCommandBuilder::fake_only_path`] — `PATH` is the fixture `bin`
//!   alone, for tests whose assertion is that nothing else was found.
//! - [`ClaudineCommandBuilder::host_path`] — the full host `PATH`, for tests
//!   needing a tool outside the minimal set.
//! - [`ClaudineCommandBuilder::ambient_context`] — the launch CWD moves to a
//!   directory the test built inside its own workspace, for tests whose
//!   subject *is* the launch context. It rejects any directory outside the
//!   fixture workspace, so the rusty-biscuit checkout can never be inherited.
//!
//! [`ClaudineCommandBuilder::inherit_no_env`] is not an escape — it tightens
//! the default rather than relaxing it — but it is spelled on the builder for
//! the same reason: the child's inherited environment has to be decided in one
//! place. On Windows the builder puts `PATHEXT`, `COMSPEC`, and `SystemRoot`
//! back after clearing: a console host without them cannot resolve or launch
//! the `.cmd` stubs the fixture writes, so the knob would be unusable rather
//! than merely strict. Nothing else returns; the rest of what a cleared run
//! needs is the call site's to re-add.
//!
//! ### Two command surfaces, one policy
//!
//! [`ClaudineCommandBuilder::build`] returns an `assert_cmd::Command`, which
//! has no `spawn`: it can only run a child to completion. A test whose subject
//! *is* the running child — signal delivery, a deadline expiring, streaming
//! stdout, `CREATE_NEW_PROCESS_GROUP`, an `expectrl` session — needs a
//! `std::process::Command`, and until it had one it had to build that command
//! by hand and re-derive the isolation above at the call site.
//! [`ClaudineCommandBuilder::build_std`] (and the
//! [`CliProcessFixture::command_std`] shorthand) is that command, carrying the
//! same policy and the same escapes.
//!
//! The two surfaces share no trait and neither hands back the other, so the
//! policy is neither a receiver method nor a copy: it is computed once as a
//! [`ChildEnvironment`] — clear flag, ordered removes, ordered sets,
//! `current_dir` — and applied by two [`ConfigurableCommand`] impls of four
//! one-line methods. `cli_process_fixture.rs` asserts the two surfaces produce
//! the same effective environment against a recording stub, so a policy change
//! that reaches only one of them fails there rather than in a Windows-only
//! test six months later.

#![allow(dead_code)]

pub(crate) mod completion;
pub(crate) mod host_tools;
pub(crate) mod incomplete_subagents;
#[cfg(unix)]
pub(crate) mod pty;
pub(crate) mod source_scan;
pub(crate) mod wrap;

// Re-exported so a call site keeps saying `common::helper_command`; the
// definitions live in their own file for the binaries that include it alone.
pub use host_tools::{GIT_PLUMBING_VARS, helper_command};

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

/// Initialize the workspace tracing subscriber for this test binary so
/// `trace_phase!` spans become visible without setting `RUST_LOG` manually.
///
/// Idempotent (`test_toolkit::init_test_tracing` uses an internal `Once`),
/// so all helpers below can call it cheaply on every invocation.
fn ensure_test_tracing_initialized() {
    test_toolkit::init_test_tracing();
}

/// Public re-export so individual tests that bypass the helpers below can
/// still opt-in explicitly.
#[allow(unused_imports)]
pub use test_toolkit::init_test_tracing;

/// Absolute path to the `claudine` binary built for this test binary.
///
/// Resolved at run time rather than with `env!`, so a suite executed from a
/// relocated `cargo nextest archive` (the `wsl2-ubuntu` leg) still finds the
/// binary — see [`biscuit_test_harness::bin_exe`]. Returned as `&str` because
/// most call sites interpolate it into a shell command line.
///
/// This is a path, not a command: it carries none of the L1 spawn contract's
/// environment. A binary that stands up a real terminal-emulator session needs
/// exactly that — the pane's login shell owns the child's environment — which is
/// why `spawn_site_guard.rs` exempts those files and governs every other caller.
pub fn claudine_bin() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        biscuit_test_harness::bin_exe!("claudine")
            .to_string_lossy()
            .into_owned()
    })
}

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        Self::named("claudine-test")
    }

    pub fn named(prefix: &str) -> Self {
        ensure_test_tracing_initialized();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{nonce}-{unique}", process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Hermetic defaults for tests that execute the shipped `claudine` binary.
///
/// Repository topology, prompts, provider memory, and user configuration are
/// absent until a test writes them into the fixture explicitly.
pub struct CliProcessFixture {
    workspace: TestWorkspace,
    cwd: PathBuf,
    home: PathBuf,
    bin_dir: PathBuf,
}

/// The variable a developer edits to move the system temp directory, spelled
/// for the platform this binary was built for.
///
/// `std::env::temp_dir()` reads `TMPDIR` on Unix and `TMP` then `TEMP` on
/// Windows, so [`checkout_containment_error`] names whichever one its reader
/// can actually act on.
const TEMP_DIR_VARIABLE: &str = if cfg!(windows) { "TMP (or TEMP)" } else { "TMPDIR" };

/// The rusty-biscuit checkout this test binary was compiled from.
///
/// `CARGO_MANIFEST_DIR` alone is the *crate* directory (`claudine/cli`), which
/// is too narrow a boundary to be useful: the temp directory that provoked this
/// guard was `<checkout>/target/tmpdir-probe`, outside the crate and still
/// inside the checkout. The nearest ancestor carrying a `.git` entry — a
/// directory for a clone, a file for a worktree — is the checkout itself, and
/// so is exactly the root claudine's own repository discovery walks out to.
///
/// ## Returns
///
/// `None` when no ancestor carries `.git`, as in a relocated
/// `cargo nextest archive` run: there is no checkout at that path to be
/// captured by.
fn checkout_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .and_then(|checkout| checkout.canonicalize().ok())
}

/// Reject a fixture workspace that lives inside the rusty-biscuit checkout.
///
/// Both arguments are already canonical — [`CliProcessFixture::named`]
/// canonicalizes them exactly as [`ClaudineCommandBuilder::ambient_context`]
/// canonicalizes its own containment check — which leaves this a pure path
/// comparison, callable from a test on every platform without touching the
/// filesystem or the process environment.
///
/// ## Returns
///
/// The panic message when `workspace_root` is contained, `None` otherwise.
pub fn checkout_containment_error(workspace_root: &Path, checkout_root: &Path) -> Option<String> {
    if !workspace_root.starts_with(checkout_root) {
        return None;
    }
    Some(format!(
        "fixture precondition: the fixture workspace {} is inside the rusty-biscuit \
         checkout {}. Claudine's repository discovery walks out of the workspace and \
         finds that checkout, so every L1 spawn would run against it rather than \
         against the workspace the test built — slowly, and against topology no test \
         wrote. Point {TEMP_DIR_VARIABLE} at a directory outside the checkout: it is \
         what `std::env::temp_dir()` reads, and the fixture workspace is built there.",
        workspace_root.display(),
        checkout_root.display(),
    ))
}

impl CliProcessFixture {
    pub fn named(prefix: &str) -> Self {
        let workspace = TestWorkspace::named(prefix);
        // Fires before anything is written into the workspace, so the developer
        // meets the cause rather than a downstream assertion's symptom.
        if let Some(checkout) = checkout_root() {
            let root = workspace
                .path()
                .canonicalize()
                .expect("fixture workspace must exist");
            if let Some(error) = checkout_containment_error(&root, &checkout) {
                panic!("{error}");
            }
        }
        let cwd = workspace.path().join("cwd");
        let home = workspace.path().join("home");
        let bin_dir = workspace.path().join("bin");
        for path in [&cwd, &home, &bin_dir] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            workspace,
            cwd,
            home,
            bin_dir,
        }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    pub fn initialize_repository(&self) {
        assert!(
            init_git_repo(&self.cwd),
            "failed to initialize fixture repository at {}",
            self.cwd.display()
        );
    }

    pub fn seed_user_config(&self) {
        wrap::seed_minimal_config(&self.home);
    }

    pub fn write_root_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.cwd.join("system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_user_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.home.join(".claudine/system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_repo_appendix(&self, content: &str) -> PathBuf {
        let path = self.cwd.join(".claudine/non-interactive.md");
        write(&path, content);
        path
    }

    pub fn write_provider_memory(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.cwd.join(relative);
        write(&path, content);
        path
    }

    /// Root of the fixture's temporary workspace.
    ///
    /// The `cwd`, `home`, and `bin` directories are children of it, and it is
    /// the containment boundary [`ClaudineCommandBuilder::ambient_context`]
    /// enforces.
    pub fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    /// A `claudine` command with the hermetic defaults described in the module
    /// docs.
    pub fn command(&self) -> assert_cmd::Command {
        self.command_builder().build()
    }

    /// The same command as a `std::process::Command`, for a test that has to
    /// hold the child alive.
    ///
    /// Identical policy to [`CliProcessFixture::command`]; see the module docs'
    /// "Two command surfaces" section for when to reach for it.
    pub fn command_std(&self) -> Command {
        self.command_builder().build_std()
    }

    /// The same command, before its defaults are traded for one of the named
    /// escapes.
    pub fn command_builder(&self) -> ClaudineCommandBuilder<'_> {
        ClaudineCommandBuilder {
            fixture: self,
            path_policy: PathPolicy::Minimal,
            current_dir: self.cwd.clone(),
            inherit_env: true,
        }
    }
}

/// How the child's `PATH` is composed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathPolicy {
    /// Fixture `bin` followed by [`minimal_system_path`] — the default.
    Minimal,
    /// Fixture `bin` alone.
    FakeOnly,
    /// Fixture `bin` followed by the host's own `PATH`.
    Host,
}

/// Builder for the one supported L1 spawn of `claudine`.
///
/// Obtained from [`CliProcessFixture::command_builder`]; see the module docs
/// for the defaults and for what each escape costs.
#[must_use = "a builder does nothing until `build()` is called"]
pub struct ClaudineCommandBuilder<'fixture> {
    fixture: &'fixture CliProcessFixture,
    path_policy: PathPolicy,
    current_dir: PathBuf,
    inherit_env: bool,
}

impl<'fixture> ClaudineCommandBuilder<'fixture> {
    /// Opt out to a `PATH` holding the fixture `bin` and nothing else.
    ///
    /// For tests whose assertion *is* that nothing else was found — a dry run
    /// that must never resolve a provider, for example. The call site states
    /// which proof it depends on.
    pub fn fake_only_path(mut self) -> Self {
        self.path_policy = PathPolicy::FakeOnly;
        self
    }

    /// Opt out to the fixture `bin` followed by the full host `PATH`.
    ///
    /// For tests needing a tool the minimal system set does not carry. The
    /// call site names that tool; everything the host has installed becomes
    /// visible to the child, including real provider binaries.
    pub fn host_path(mut self) -> Self {
        self.path_policy = PathPolicy::Host;
        self
    }

    /// Opt out to a launch CWD other than the fixture `cwd`.
    ///
    /// For tests whose subject is the launch context itself — repository
    /// discovery from a nested directory, package-scoped prompt resolution.
    /// `dir` must be a directory the test built inside its own workspace
    /// (with [`CliProcessFixture::initialize_repository`] or
    /// `wrap::create_claudine_monorepo`).
    ///
    /// ## Panics
    ///
    /// When `dir` does not exist, or resolves outside the fixture workspace.
    /// Inheriting the rusty-biscuit checkout as the launch context is the
    /// failure mode this fixture exists to prevent.
    pub fn ambient_context(mut self, dir: &Path) -> Self {
        let canonical_dir = dir.canonicalize().unwrap_or_else(|error| {
            panic!(
                "ambient-context directory {} must exist before it is pinned: {error}",
                dir.display()
            )
        });
        let workspace = self
            .fixture
            .workspace_path()
            .canonicalize()
            .expect("fixture workspace must exist");
        assert!(
            canonical_dir.starts_with(&workspace),
            "ambient-context escape: {} is not inside the fixture workspace {}; \
             the launch context must be a repository the test built itself",
            dir.display(),
            self.fixture.workspace_path().display()
        );
        // The caller's spelling is pinned, not `canonical_dir`: on macOS the
        // canonical form is `/private/var/...` where the fixture hands out
        // `/var/...`, and a test that asserts on paths in claudine's output
        // would then see a prefix it never chose.
        self.current_dir = dir.to_path_buf();
        self
    }

    /// Give the child nothing but what the fixture and the call site set.
    ///
    /// The defaults still apply on top, so this is a tightening rather than an
    /// escape: it exists for tests that assert on what claudine *found* in its
    /// own environment (the removed-sensitive-variable report, for one), where
    /// an inherited `GITHUB_TOKEN` on the developer's machine would change the
    /// output the assertion pins.
    ///
    /// A cleared environment is not a realistic one — `TERM` and the temp-dir
    /// variables disappear — so the call site re-adds whatever the run needs.
    /// The exception is the Windows console plumbing, which
    /// [`restore_windows_console_variables`] puts back inside `build()`:
    /// leaving it to the call site would make every Windows caller re-derive
    /// the same three values before it could launch a `.cmd` stub at all.
    pub fn inherit_no_env(mut self) -> Self {
        self.inherit_env = false;
        self
    }

    /// Materialize the command with the contract described in the module docs.
    ///
    /// The scrub runs before the defaults are applied, so the two fixture keys
    /// that live in the scrubbed namespaces — `CLAUDINE_RENDEZVOUS_REPORT` here,
    /// `CLAUDINE_PROBE_CAPTURE` at a call site — survive.
    pub fn build(self) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("claudine").unwrap();
        self.child_environment().apply(&mut command);
        command
    }

    /// The same contract on a `std::process::Command`, for a call site that
    /// has to keep the child.
    ///
    /// The program is resolved through [`claudine_bin`] rather than
    /// `assert_cmd`'s `cargo_bin`, so a relocated `cargo nextest archive` run
    /// finds it; both name the same file whenever the runner built the binary.
    pub fn build_std(self) -> Command {
        let mut command = Command::new(claudine_bin());
        self.child_environment().apply(&mut command);
        command
    }

    /// Apply the same policy to a command the fixture did not build.
    ///
    /// For the one shape [`ClaudineCommandBuilder::build_std`] cannot express:
    /// a test whose subject is a *shell* redirect around the claudine launch,
    /// so the child the PTY owns is `/bin/sh` and claudine is its grandchild.
    /// The claudine path still comes from `build_std().get_program()` rather
    /// than a hand-rolled `cargo_bin`, and the environment the shell hands down
    /// is this one — which is the whole reason the policy is data.
    pub fn apply_policy_to(&self, command: &mut Command) {
        self.child_environment().apply(command);
    }

    /// The policy both surfaces apply, computed once.
    ///
    /// Ordered exactly as the two `build` methods used to spell it inline:
    /// clear (and the Windows console restore) first, then the inherited
    /// scrub, then the fixture defaults — so a default living inside a
    /// scrubbed namespace survives its own sweep, and a per-key `.env` after
    /// `build()` still wins.
    fn child_environment(&self) -> ChildEnvironment {
        let mut ops = Vec::new();
        if !self.inherit_env {
            for (key, value) in windows_console_variables() {
                ops.push(EnvironmentOp::Set(key, value));
            }
        }
        for key in inherited_scrub_keys() {
            ops.push(EnvironmentOp::Remove(key));
        }
        let home = self.fixture.home().as_os_str();
        for (key, value) in [("HOME", home), ("USERPROFILE", home)] {
            ops.push(EnvironmentOp::Set(key.into(), value.to_os_string()));
        }
        for key in ["HOMEDRIVE", "HOMEPATH", "XDG_CONFIG_HOME"] {
            ops.push(EnvironmentOp::Remove(key.into()));
        }
        for (key, value) in [
            ("APPDATA", home.to_os_string()),
            ("LOCALAPPDATA", home.to_os_string()),
            ("PATH", self.path_value()),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false".into()),
            ("NO_COLOR", "1".into()),
            ("PLAYA_DRY_RUN", "1".into()),
            (
                "PLAYA_SPOOL_DIR",
                self.fixture.workspace_path().join("playa-spool").into(),
            ),
        ] {
            ops.push(EnvironmentOp::Set(key.into(), value));
        }
        ChildEnvironment {
            clear: !self.inherit_env,
            ops,
            current_dir: self.current_dir.clone(),
        }
    }

    fn path_value(&self) -> std::ffi::OsString {
        let bin_dir = self.fixture.bin_dir();
        match self.path_policy {
            PathPolicy::Minimal => {
                let mut entries = vec![bin_dir.to_path_buf()];
                entries.extend(minimal_system_path());
                std::env::join_paths(entries).expect("minimal PATH entries must join")
            }
            PathPolicy::FakeOnly => {
                std::env::join_paths([bin_dir]).expect("fake-only PATH must join")
            }
            PathPolicy::Host => augmented_path(bin_dir),
        }
    }
}

/// The inherited environment families the L1 spawn contract removes, in the
/// order the child command receives them.
///
/// `CLAUDINE_*` and `PLAYA_*` go by prefix rather than by name: the first
/// namespace is ~48 names across `lib/src` and `cli/src` and grows without this
/// helper being told, and the second carries the two keys — `PLAYA_DRY_RUN`,
/// `PLAYA_SPOOL_DIR` — whose fixture defaults decide whether a lifecycle audio
/// effect spawns a detached worker on the developer's machine.
/// Only names the *parent* actually carries are listed — removing a key a
/// command never had is a no-op, and enumerating the parent is the only way a
/// prefix rule can be expressed as a key list at all.
/// `TERM_WIDTH`/`COLUMNS`/`FORCE_COLOR` go because `cli/src/log.rs` reads them
/// before falling back to 80 columns, and because `FORCE_COLOR` would otherwise
/// out-vote the `NO_COLOR=1` the builder sets.
fn inherited_scrub_keys() -> Vec<OsString> {
    let mut keys: Vec<OsString> = std::env::vars_os()
        .map(|(key, _)| key)
        .filter(|key| {
            let key = key.to_string_lossy();
            key.starts_with("CLAUDINE_") || key.starts_with("PLAYA_")
        })
        .collect();
    keys.extend(GIT_PLUMBING_VARS.iter().map(OsString::from));
    keys.extend(["TERM_WIDTH", "COLUMNS", "FORCE_COLOR"].map(OsString::from));
    keys
}

/// One ordered operation on the child's environment block.
///
/// A `Vec` of these rather than a map, because two of the contract's rules are
/// ordering rules: `CLAUDINE_RENDEZVOUS_REPORT` is set *after* the
/// `CLAUDINE_*` sweep that would otherwise remove it, and the Windows console
/// restore runs after the clear that took those three away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentOp {
    Remove(OsString),
    Set(OsString, OsString),
}

/// The L1 spawn contract as data, ready to apply to either command surface.
///
/// `assert_cmd::Command` and `std::process::Command` share no trait, and
/// neither hands back the other, so "one shared implementation" cannot be a
/// method on a receiver. It is this description plus [`ChildEnvironment::apply`],
/// which is the only code that decides what a `claudine` child inherits.
pub struct ChildEnvironment {
    /// Whether the child starts from an empty block ([`ClaudineCommandBuilder::inherit_no_env`]).
    clear: bool,
    /// Removes and sets, in application order.
    ops: Vec<EnvironmentOp>,
    /// The pinned launch directory.
    current_dir: PathBuf,
}

impl ChildEnvironment {
    /// Apply the policy to `command`.
    pub fn apply<C: ConfigurableCommand>(&self, command: &mut C) {
        if self.clear {
            command.clear_environment();
        }
        for op in &self.ops {
            match op {
                EnvironmentOp::Remove(key) => command.remove_variable(key),
                EnvironmentOp::Set(key, value) => command.set_variable(key, value),
            }
        }
        command.pin_current_dir(&self.current_dir);
    }
}

/// The four calls [`ChildEnvironment::apply`] needs from a command surface.
///
/// Deliberately not `std::process::Command`'s own method names: an adapter
/// that merely forwarded them could be replaced by the inherent method and the
/// policy would drift back into the call site unnoticed.
pub trait ConfigurableCommand {
    fn clear_environment(&mut self);
    fn remove_variable(&mut self, key: &OsStr);
    fn set_variable(&mut self, key: &OsStr, value: &OsStr);
    fn pin_current_dir(&mut self, dir: &Path);
}

impl ConfigurableCommand for assert_cmd::Command {
    fn clear_environment(&mut self) {
        self.env_clear();
    }

    fn remove_variable(&mut self, key: &OsStr) {
        self.env_remove(key);
    }

    fn set_variable(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }

    fn pin_current_dir(&mut self, dir: &Path) {
        self.current_dir(dir);
    }
}

impl ConfigurableCommand for Command {
    fn clear_environment(&mut self) {
        self.env_clear();
    }

    fn remove_variable(&mut self, key: &OsStr) {
        self.env_remove(key);
    }

    fn set_variable(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }

    fn pin_current_dir(&mut self, dir: &Path) {
        self.current_dir(dir);
    }
}

/// `%SystemRoot%` for a parent process that has none.
///
/// Both the default `PATH` and the cleared-environment restore below depend on
/// this value, and they have to agree, so the fallback is decided once here.
const WINDOWS_SYSTEM_ROOT_FALLBACK: &str = r"C:\Windows";

/// `PATHEXT` for a parent process that has none — enough for the `.cmd` stubs
/// the fixture writes, and deliberately shorter than the Windows default.
const WINDOWS_PATHEXT_FALLBACK: &str = ".COM;.EXE;.BAT;.CMD";

/// The `%SystemRoot%` this fixture resolves against, the parent's value first.
///
/// Kept out of any `cfg` block so a typo is a compile error on every leg rather
/// than only on `windows-latest`; the callers are Windows-only.
fn windows_system_root() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(WINDOWS_SYSTEM_ROOT_FALLBACK))
}

/// The Windows console plumbing to put back after
/// [`ClaudineCommandBuilder::inherit_no_env`] has cleared the child; empty
/// everywhere else.
///
/// `env_clear()` on Windows also removes `PATHEXT` — without which the fixture
/// `bin` resolves none of its `.cmd` stubs — `COMSPEC`, and the `SystemRoot`
/// that [`minimal_system_path`] reads. These three are what the console host
/// itself needs; restoring only them keeps the knob a tightening rather than a
/// trap, and a Unix run is untouched.
///
/// The platform test is `cfg!` rather than `#[cfg]` so both arms compile on
/// every leg: a Unix run proves the restore stays Windows-only, which is the
/// half of the contract a macOS or Linux host can verify at all.
fn windows_console_variables() -> Vec<(OsString, OsString)> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let system_root = windows_system_root();
    let comspec = std::env::var_os("COMSPEC")
        .unwrap_or_else(|| system_root.join("System32").join("cmd.exe").into_os_string());
    let pathext = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| OsString::from(WINDOWS_PATHEXT_FALLBACK));
    vec![
        (OsString::from("SystemRoot"), system_root.into_os_string()),
        (OsString::from("COMSPEC"), comspec),
        (OsString::from("PATHEXT"), pathext),
    ]
}

/// The system directories the default `PATH` carries behind the fixture `bin`.
///
/// The roster differs per platform and the difference matters: `/usr/bin:/bin`
/// resolves `sh`, `cat`, `sleep`, and `git`, while `%SystemRoot%\System32`
/// resolves `cmd.exe`, `where.exe`, and PowerShell and **none** of those four —
/// Git for Windows lives under `Program Files`, which this set excludes along
/// with every other prefix an agentic CLI installs into. A fixture that needs a
/// POSIX utility is Unix-gated, spelled with an absolute path, or given a stub
/// in the fixture `bin`.
pub fn minimal_system_path() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        vec![windows_system_root().join("System32")]
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")]
    }
}

pub fn write(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write(path, &serde_json::to_string_pretty(value).unwrap());
}

/// Create a git repository at `path`, returning whether `git init` succeeded.
///
/// This runs from the *parent* process, which has a fully inherited
/// environment, so it goes through [`helper_command`] rather than
/// `Command::new("git")`; see [`GIT_PLUMBING_VARS`] for the incident.
pub fn init_git_repo(path: &Path) -> bool {
    helper_command("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The fixture `bin` followed by the host's own `PATH`.
///
/// Everything installed on the machine becomes visible to the child, including
/// real provider binaries that claudine's `which`-based discovery will find, so
/// an assertion downstream of it pins whichever host ran the suite. L1 tests
/// reach it through [`ClaudineCommandBuilder::host_path`], which requires a
/// call-site comment naming the tool; it stays public for the binaries that
/// drive a real terminal-emulator session, where a realistic host `PATH` is the
/// point.
///
/// Visibility cannot express that split. Every integration test binary compiles
/// its own copy of this module as a private `mod common`, so `pub` here already
/// means "this binary only" and `pub(crate)` would reach exactly as far — there
/// is no marker that admits an emulator-session file and refuses `wrap_basics`.
/// The isolation gate in `spawn_site_guard.rs` is what keeps the migrated L1
/// files off it.
pub fn augmented_path(fake_bin: &Path) -> std::ffi::OsString {
    ensure_test_tracing_initialized();
    let system_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = vec![fake_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&system_path));
    std::env::join_paths(paths).expect("join_paths")
}

/// Clear an inherited `NO_COLOR` from a pane's interactive shell before a
/// color fixture runs in it.
///
/// `send_command_with_env` cannot express this: it renders env as `VAR='' cmd`,
/// and claudine treats `NO_COLOR` as *present* rather than as truthy
/// (`cli/src/log.rs` — `colors_disabled()` short-circuits `force_color_enabled()`),
/// so an empty value still disables color and `FORCE_COLOR=1` cannot out-vote
/// it. A host that exports `NO_COLOR=1` — as review environments legitimately
/// do — would otherwise turn every L2 color assertion into a false failure, or
/// worse, a vacuous pass.
///
/// The unset lands in the shell the caller then drives, so it survives any
/// `clear`/`cd` sent afterwards.
pub fn clear_no_color<H: biscuit_test_harness::TerminalHarness>(harness: &mut H) {
    harness
        .send_text(b"unset NO_COLOR\n")
        .expect("unset NO_COLOR");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
}

/// Assert that the captured pane row carrying `needle` is itself styled.
///
/// The weaker `frame.raw.contains('\u{1b}')` this replaces is satisfied by *any*
/// escape anywhere in the pane — a colored shell prompt, or tmux's own
/// re-emission — so it can hold on a run that rendered the diagnostic with no
/// styling at all. Anchoring on the row that carries the diagnostic text makes
/// the assertion fail when the thing under test loses its color.
pub fn assert_row_is_styled(raw: &str, needle: &str, what: &str) {
    let row = raw
        .lines()
        .find(|line| strip_ansi(line).contains(needle))
        .unwrap_or_else(|| panic!("no captured row contains {needle:?}.\nraw:\n{raw}"));
    assert!(
        row.contains('\u{1b}'),
        "{what}: the row carrying {needle:?} reached the pane unstyled.\nrow: {row:?}"
    );
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if code == '\u{7}' {
                            break;
                        }
                        if code == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(ch);
    }

    out
}

/// Probe for a usable pseudo-terminal on the host.
///
/// On Unix, attempts to open `/dev/ptmx` (the master multiplexer). Returns
/// `true` when allocation succeeds, which is the precondition every
/// `expectrl::Session::spawn` call in this crate's Level 1 PTY tests requires.
/// On non-Unix targets returns `false` (the PTY test files are themselves
/// `#[cfg(unix)]`, so this branch is unreachable from those tests).
#[allow(dead_code)]
pub fn pty_available() -> bool {
    #[cfg(unix)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn write_executable(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        write(path, content);
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(not(unix))]
    {
        write(path, content);
    }
}

pub fn write_dry_run_provider_stub(bin_dir: &Path, binary: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        write_executable(
            &bin_dir.join(binary),
            r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
        );
    }
    #[cfg(windows)]
    {
        write(
            &bin_dir.join(format!("{binary}.cmd")),
            "@echo off\r\necho SHOULD NOT RUN\r\nexit /b 1\r\n",
        );
    }
}
