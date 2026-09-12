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
//! shell-out — out of the other L1 binaries, with a reasoned allow-list for
//! the files this contract has not reached yet.
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
//! removes `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME`, sets
//! `CLAUDINE_RENDEZVOUS_REPORT=false` and `NO_COLOR=1`, and silences lifecycle
//! audio (below).
//!
//! ### The audio contract
//!
//! Shipped prompts carry live `say:`/`effect:` lifecycle actions, and a test
//! that executes one through a fake provider still reaches the real speech
//! and playback boundary — `feature-review.md` cheered aloud from
//! `shipped_prompt_contract.rs` on 2026-09-10 that way. The default command
//! therefore sets child-local `PLAYA_DRY_RUN=1` and points `PLAYA_SPOOL_DIR` at
//! [`CliProcessFixture::audio_spool`], a directory inside the fixture workspace
//! that dry-run never creates; a test that executes a shipped document proves
//! its silence by asserting that directory is still absent afterwards. The
//! one L1 file whose subject *is* durable audio publication
//! (`detached_audio.rs`) opts out per key on the built command and holds the
//! worker lock on its own private spool instead.
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

#![allow(dead_code)]

pub(crate) mod completion;
#[cfg(unix)]
pub(crate) mod pty;
pub(crate) mod review_router;
pub(crate) mod source_scan;
pub(crate) mod wrap;

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
/// environment. L2/L3 binaries drive claudine through a real terminal and need
/// exactly that, which is why `spawn_site_guard.rs` governs L1 callers only.
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

    /// Where the default command sends detached audio jobs.
    ///
    /// The default `PLAYA_DRY_RUN=1` publishes nothing, so this directory is
    /// created only if a child escaped the dry run; its absence after a run is
    /// the proof that no lifecycle audio was published.
    pub fn audio_spool(&self) -> PathBuf {
        self.workspace.path().join("audio-spool")
    }

    /// A `claudine` command with the hermetic defaults described in the module
    /// docs.
    pub fn command(&self) -> assert_cmd::Command {
        self.command_builder().build()
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
        if !self.inherit_env {
            command.env_clear();
            restore_windows_console_variables(&mut command);
        }
        scrub_inherited_environment(&mut command);
        command
            .current_dir(&self.current_dir)
            .env("HOME", self.fixture.home())
            .env("USERPROFILE", self.fixture.home())
            .env_remove("HOMEDRIVE")
            .env_remove("HOMEPATH")
            .env_remove("XDG_CONFIG_HOME")
            // Model resolution consults the generic `MODEL` environment
            // variable ahead of frontmatter (`composition::select` precedence
            // step 3), and a Claudine-wrapped agent session exports one — so an
            // unscrubbed host silently replaces every fixture's model.
            .env_remove("MODEL")
            .env("APPDATA", self.fixture.home())
            .env("LOCALAPPDATA", self.fixture.home())
            .env("PATH", self.path_value())
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("NO_COLOR", "1")
            .env("PLAYA_DRY_RUN", "1")
            .env("PLAYA_SPOOL_DIR", self.fixture.audio_spool());
        command
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

/// The `GIT_*` plumbing variables that override cwd-based repository discovery.
///
/// An inherited pair defeats `current_dir` entirely, in the fixture as well as
/// in the child: on 2026-08-31 a pre-push hook run of this suite inherited
/// `GIT_DIR` and drove fixture `git` commands into the real repository,
/// committing fixture files onto a feature branch.
const GIT_PLUMBING_VARS: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

/// Remove the inherited environment families the L1 spawn contract owns.
///
/// `CLAUDINE_*` goes by prefix rather than by name: the namespace is ~48 names
/// across `lib/src` and `cli/src` and grows without this helper being told.
/// `TERM_WIDTH`/`COLUMNS`/`FORCE_COLOR` go because `cli/src/log.rs` reads them
/// before falling back to 80 columns, and because `FORCE_COLOR` would otherwise
/// out-vote the `NO_COLOR=1` the builder sets.
fn scrub_inherited_environment(command: &mut assert_cmd::Command) {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CLAUDINE_") {
            command.env_remove(&key);
        }
    }
    for key in GIT_PLUMBING_VARS {
        command.env_remove(key);
    }
    for key in ["TERM_WIDTH", "COLUMNS", "FORCE_COLOR"] {
        command.env_remove(key);
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

/// Put the Windows console plumbing back after
/// [`ClaudineCommandBuilder::inherit_no_env`] has cleared the child.
///
/// `env_clear()` on Windows also removes `PATHEXT` — without which the fixture
/// `bin` resolves none of its `.cmd` stubs — `COMSPEC`, and the `SystemRoot`
/// that [`minimal_system_path`] reads. These three are what the console host
/// itself needs; restoring only them keeps the knob a tightening rather than a
/// trap, and a Unix run is untouched.
fn restore_windows_console_variables(command: &mut assert_cmd::Command) {
    #[cfg(windows)]
    {
        let system_root = windows_system_root();
        let comspec = std::env::var_os("COMSPEC")
            .unwrap_or_else(|| system_root.join("System32").join("cmd.exe").into_os_string());
        let pathext = std::env::var_os("PATHEXT")
            .unwrap_or_else(|| std::ffi::OsString::from(WINDOWS_PATHEXT_FALLBACK));
        command
            .env("SystemRoot", &system_root)
            .env("COMSPEC", comspec)
            .env("PATHEXT", pathext);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
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
/// environment — so the `GIT_*` plumbing family is scrubbed here as well as on
/// the child command; see [`GIT_PLUMBING_VARS`] for the incident.
pub fn init_git_repo(path: &Path) -> bool {
    ensure_test_tracing_initialized();
    let mut command = Command::new("git");
    for key in GIT_PLUMBING_VARS {
        command.env_remove(key);
    }
    command
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
/// call-site comment naming the tool; it stays public for the L2/L3 binaries,
/// where a realistic host `PATH` is the point.
///
/// Visibility cannot express that split. Every integration test binary compiles
/// its own copy of this module as a private `mod common`, so `pub` here already
/// means "this binary only" and `pub(crate)` would reach exactly as far — there
/// is no marker that admits `level2_*` and refuses `wrap_basics`. The isolation
/// gate in `spawn_site_guard.rs` is what keeps the migrated L1 files off it.
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

/// Poll the pane until `expected` is drawn, returning that frame.
pub fn wait_for_pane_text(
    harness: &mut impl biscuit_test_harness::TerminalHarness,
    expected: &str,
    timeout: std::time::Duration,
) -> biscuit_test_harness::CapturedFrame {
    let deadline = std::time::Instant::now() + timeout;
    let mut frame = harness.capture().expect("initial capture");
    while std::time::Instant::now() < deadline {
        if frame.plain.contains(expected) {
            return frame;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        frame = harness.capture().expect("poll terminal content");
    }
    panic!("expected terminal content {expected:?} never rendered; plain:\n{}", frame.plain);
}

/// Poll the pane until a row starting with `<marker>:` is drawn, returning
/// that frame and whatever followed the colon (the exit status the command's
/// trailer echoed).
pub fn wait_for_exit_marker(
    harness: &mut impl biscuit_test_harness::TerminalHarness,
    marker: &str,
    timeout: std::time::Duration,
) -> (biscuit_test_harness::CapturedFrame, String) {
    let prefix = format!("{marker}:");
    let deadline = std::time::Instant::now() + timeout;
    let mut frame = harness.capture().expect("initial capture");
    while std::time::Instant::now() < deadline {
        if let Some(status) = frame
            .plain
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix))
        {
            let status = status.trim().to_string();
            return (frame, status);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        frame = harness.capture().expect("poll for exit marker");
    }
    panic!("command exit marker {marker:?} never rendered; plain:\n{}", frame.plain);
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
/// `expectrl::Session::spawn` call in this crate's L2 PTY tests requires.
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

/// Wrap `value` in a POSIX single-quoted word, escaping embedded quotes.
#[cfg(unix)]
pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// POSIX-shell fragment that replaces `$CLAUDINE_DOC`'s body with
/// `$CLAUDINE_BODY`, inserting `$CLAUDINE_ADD` before the closing delimiter.
///
/// Builtins only. Inline fixtures routinely set `PATH` to the stub directory
/// alone, so a fragment that reached for `cat`, `awk`, or `mv` would silently
/// do nothing and look like an agent that refused the task.
#[cfg(unix)]
pub const INLINE_BODY_REWRITE: &str = concat!(
    "CLAUDINE_OUT=''\n",
    "CLAUDINE_N=0\n",
    "while IFS= read -r CLAUDINE_LINE || [ -n \"$CLAUDINE_LINE\" ]; do\n",
    "  if [ \"$CLAUDINE_LINE\" = '---' ] && [ \"$CLAUDINE_N\" -lt 2 ]; then\n",
    "    CLAUDINE_N=$((CLAUDINE_N + 1))\n",
    "    if [ \"$CLAUDINE_N\" -eq 2 ]; then CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_ADD\"; fi\n",
    "    CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_LINE\n\"\n",
    "    continue\n",
    "  fi\n",
    "  if [ \"$CLAUDINE_N\" -lt 2 ]; then CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_LINE\n\"; fi\n",
    "done < \"$CLAUDINE_DOC\"\n",
    "printf '%s%s' \"$CLAUDINE_OUT\" \"$CLAUDINE_BODY\" > \"$CLAUDINE_DOC\"\n",
);

/// POSIX-shell fragment that recovers the active document from the delivered
/// prompt header into `$CLAUDINE_DOC`, the same way a real agent learns it.
///
/// For a fixture whose active document is not known when the stub is written —
/// a sequence task, or a proxied target. Scans argv first, then stdin, because
/// prompt delivery is per provider and per session mode.
#[cfg(unix)]
pub const INLINE_DOC_FROM_PROMPT: &str = concat!(
    "CLAUDINE_DOC=''\n",
    "for CLAUDINE_ARG in \"$@\"; do\n",
    "  case \"$CLAUDINE_ARG\" in\n",
    "    *'**Document:** `'*)\n",
    "      CLAUDINE_DOC=\"${CLAUDINE_ARG#*'**Document:** `'}\"\n",
    "      CLAUDINE_DOC=\"${CLAUDINE_DOC%%\\`*}\"\n",
    "      ;;\n",
    "  esac\n",
    "done\n",
    "if [ -z \"$CLAUDINE_DOC\" ]; then\n",
    "  while IFS= read -r CLAUDINE_ARG; do\n",
    "    case \"$CLAUDINE_ARG\" in\n",
    "      *'**Document:** `'*)\n",
    "        CLAUDINE_DOC=\"${CLAUDINE_ARG#*'**Document:** `'}\"\n",
    "        CLAUDINE_DOC=\"${CLAUDINE_DOC%%\\`*}\"\n",
    "        break\n",
    "        ;;\n",
    "    esac\n",
    "  done\n",
    "fi\n",
);

/// A provider stub that behaves like a file-aware inline agent.
///
/// Under the 2026-09-05 inline contract the agent *is* the writer: it edits the
/// active document and returns a short summary, and Claudine reads the file
/// back. A stub that only prints to stdout is a stub that did no work, so every
/// inline fixture builds its script here — the body is rewritten in place, the
/// authored frontmatter is preserved byte-for-byte, and only the declared
/// additions are inserted.
///
/// Every field is embedded as a POSIX single-quoted word, so any content is
/// safe. `prelude` is emitted verbatim and must end with a newline when set.
#[cfg(unix)]
pub struct InlineAgentStub<'a> {
    document: &'a Path,
    prelude: &'a str,
    frontmatter_additions: &'a str,
    body: &'a str,
    body_is_literal: bool,
    summary: &'a str,
    exit_code: i32,
}

#[cfg(unix)]
impl<'a> InlineAgentStub<'a> {
    /// An agent that replaces `document`'s body and reports one line.
    pub fn new(document: &'a Path) -> Self {
        Self {
            document,
            prelude: "",
            frontmatter_additions: "",
            body: "Agent body\n",
            body_is_literal: true,
            summary: "Wrote the requested content.",
            exit_code: 0,
        }
    }

    /// Shell run before the edit — argv capture, run counters, sentinels.
    pub fn prelude(mut self, prelude: &'a str) -> Self {
        self.prelude = prelude;
        self
    }

    /// Frontmatter lines inserted immediately before the closing delimiter.
    pub fn frontmatter_additions(mut self, additions: &'a str) -> Self {
        self.frontmatter_additions = additions;
        self
    }

    /// The body the agent writes.
    pub fn body(mut self, body: &'a str) -> Self {
        self.body = body;
        self.body_is_literal = true;
        self
    }

    /// The body as a shell word the caller quotes itself, so a fixture whose
    /// body must vary per run can interpolate a variable set in the prelude.
    pub fn body_expression(mut self, expression: &'a str) -> Self {
        self.body = expression;
        self.body_is_literal = false;
        self
    }

    /// The final response the agent returns to the caller.
    pub fn summary(mut self, summary: &'a str) -> Self {
        self.summary = summary;
        self
    }

    /// Exit with `code` after the edit.
    pub fn exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    /// Render the `/bin/sh` script.
    pub fn script(&self) -> String {
        let document = sh_quote(&self.document.display().to_string());
        let additions = sh_quote(self.frontmatter_additions);
        let body = if self.body_is_literal {
            sh_quote(self.body)
        } else {
            self.body.to_string()
        };
        let summary = sh_quote(self.summary);
        let prelude = &self.prelude;
        let exit_code = self.exit_code;
        format!(
            "#!/bin/sh\n\
             {prelude}\
             CLAUDINE_DOC={document}\n\
             CLAUDINE_ADD={additions}\n\
             CLAUDINE_BODY={body}\n\
             {INLINE_BODY_REWRITE}\
             printf '%s\\n' {summary}\n\
             exit {exit_code}\n"
        )
    }

    /// Write the script to `bin_dir/binary` as an executable.
    pub fn install(&self, bin_dir: &Path, binary: &str) {
        write_executable(&bin_dir.join(binary), &self.script());
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
