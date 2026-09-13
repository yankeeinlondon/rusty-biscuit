//! The deterministic L1 spawn contract for the `md` binary.
//!
//! [`CliProcessFixture`] is the supported way an L1 test in this directory
//! obtains an `md` command. [`CliProcessFixture::command`] returns a command
//! that runs against a workspace the test built — never against the checkout
//! the suite was compiled from — so a test is hermetic by construction rather
//! than by a per-test `.env(...)` chain.
//!
//! The default command pins `current_dir` to the fixture `cwd`, points the
//! home/config/cache variables at fixture directories, sweeps the
//! `DARKMATTER_*`/`DM_*`/`MD_*` namespaces plus the Git plumbing and rendering
//! inputs out of the inherited environment, sets `GIT_CONFIG_NOSYSTEM=1` and
//! `NO_COLOR=1`, and composes a `PATH` of the fixture `bin` plus a minimal
//! platform system set.
//!
//! ## Launch inputs made explicit
//!
//! | Family | Policy |
//! |---|---|
//! | CWD | pinned to the fixture `cwd` |
//! | home | `HOME`/`USERPROFILE`/`APPDATA`/`LOCALAPPDATA` → fixture `home`; `HOMEDRIVE`/`HOMEPATH` removed |
//! | config/cache | `XDG_CONFIG_HOME` → fixture `config`, `XDG_CACHE_HOME` → fixture `cache` (the fallback `dirs::cache_dir()` uses for compose artifacts when no `--cache-root` flag is passed) |
//! | temp | `TMPDIR` (Unix) / `TEMP`+`TMP` (Windows) → fixture `tmp` |
//! | darkmatter namespace | inherited `DARKMATTER_*`, `DM_*`, `MD_*`, `THEME`, `CODE_THEME`, `PREFER_ITALICS`, `TERMINAL_IMAGES`, `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`, `BASELINE_SCHEMA`, `AGENT`, `MODEL`, `RUST_LOG` removed by prefix or name |
//! | Git plumbing | `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/`GIT_COMMON_DIR`/`GIT_OBJECT_DIRECTORY` and the `GIT_CONFIG_*` family removed; `GIT_CONFIG_NOSYSTEM=1` set so the host's system gitconfig never applies |
//! | rendering | `COLUMNS`, `LINES`, `TERM`, `COLORTERM`, `COLORFGBG`, `CLICOLOR_FORCE`, `FORCE_COLOR`, inherited `NO_COLOR` removed; `NO_COLOR=1` set. A test whose subject *is* rendering policy declares its own values with [`MdCommandBuilder::rendering_input`] |
//! | PATH | fixture `bin` + [`minimal_system_path`] |
//!
//! The darkmatter cache root has no environment form — persistent remote
//! caching is opt-in through the compose-only `--cache-root` flag, and without
//! that flag compose persists nothing. What the env *can* influence is the
//! `dirs::cache_dir()` fallback, so the builder points it at the fixture;
//! cache tests pass `--cache-root <fixture-owned dir>` explicitly.
//!
//! Removal is per key at build time and scrubs only what was *inherited*, so
//! a value the call site chooses still reaches the child. Which call sites may
//! choose one is the subject of "Declared inputs" below.
//!
//! ## The default `PATH` rule
//!
//! The fixture `bin` directory comes first, then a minimal platform system
//! set (`/usr/bin:/bin` on Unix, `%SystemRoot%\System32` on Windows) with
//! `PATHEXT` left alone so `.cmd` stubs resolve. `md` itself spawns `sh` and
//! `git` by bare name for shell expansion, so a fake-only default would break
//! shell-shaped tests; host tool prefixes (Homebrew, npm, cargo,
//! `~/.local/bin`) stay invisible. Windows executable resolution survives
//! [`MdCommandBuilder::inherit_no_env`] because `build()` restores
//! `SystemRoot`, `COMSPEC`, and `PATHEXT` after the clear.
//!
//! ## The temp-directory precondition
//!
//! The fixture workspace lives under `std::env::temp_dir()`, so a temp
//! directory inside the rusty-biscuit checkout would put every "isolated"
//! workspace *inside* the checkout, where repository discovery walks straight
//! back out to it. [`CliProcessFixture::named`] rejects that — see
//! [`checkout_containment_error`] — comparing canonicalized paths so a
//! symlink-spelled root cannot slip through (on macOS this also normalizes
//! `/tmp` → `/private/tmp`).
//!
//! ## Escapes
//!
//! Each escape requires a comment at the call site naming the tool it needs
//! or the proof it depends on:
//!
//! - [`MdCommandBuilder::fake_only_path`] — `PATH` is the fixture `bin`
//!   alone, for tests whose assertion is that nothing else was found.
//! - [`MdCommandBuilder::host_path`] — the full host `PATH`, for tests
//!   needing a tool outside the minimal set (a real shell, for example).
//! - [`MdCommandBuilder::ambient_context`] — the launch CWD moves to a
//!   directory the test built inside its own workspace, for tests whose
//!   subject is the launch context. It rejects any directory outside the
//!   fixture workspace.
//! - [`MdCommandBuilder::inherit_no_env`] — not an escape but a tightening:
//!   the child starts from an empty block plus the Windows console plumbing
//!   and the fixture defaults.
//!
//! ## Declared inputs
//!
//! `common/protected_env.rs` splits the pinned variables in two, because they
//! are not the same kind of thing. *Containment* — the home/config/cache/temp
//! anchors and the Git plumbing — exists to keep the developer's machine out
//! of the child, so it has no override at any spelling. *Behavior inputs* —
//! the rendering namespace and the darkmatter application namespace — are
//! pinned to deterministic defaults, and a test that wants a different value
//! is making a real claim about behavior, which it states before `build()`:
//!
//! - [`MdCommandBuilder::rendering_input`] /
//!   [`MdCommandBuilder::rendering_input_removed`]
//! - [`MdCommandBuilder::application_input`] /
//!   [`MdCommandBuilder::application_input_removed`]
//! - [`MdCommandBuilder::plain_terminal`] — the whole fixed-size, no-color,
//!   `TERM=dumb` frame in one claim.
//!
//! Declarations are applied after the fixture defaults, so a declared value
//! out-ranks both the inherited one and the default. A key the contract does
//! *not* pin needs no declaration and is set on the command `build()` returns.
//! `spawn_site_guard.rs` is what keeps the two apart: a post-`build()` `.env`
//! or `.env_remove` naming a protected key fails the isolation gate.
//!
//! ## Two command surfaces, one policy
//!
//! [`MdCommandBuilder::build`] returns an `assert_cmd::Command` (run to
//! completion); [`MdCommandBuilder::build_std`] returns a
//! `std::process::Command` for a test that has to hold the child. The policy
//! is computed once as a [`ChildEnvironment`] and applied to both, and
//! `md_process_fixture.rs` asserts the two surfaces hand a child the same
//! environment.

#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::protected_env::{PATH_VARIABLE, ProtectedClass, protected_class};

/// The `GIT_*` plumbing variables that override cwd-based repository
/// discovery, defeating the pinned `current_dir` entirely.
pub const GIT_PLUMBING_VARS: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

/// The variable a developer edits to move the system temp directory, spelled
/// for the platform this binary was built for; `std::env::temp_dir()` reads
/// `TMPDIR` on Unix and `TMP` then `TEMP` on Windows.
const TEMP_DIR_VARIABLE: &str = if cfg!(windows) {
    "TMP (or TEMP)"
} else {
    "TMPDIR"
};

static WORKSPACE_NONCE: AtomicU64 = AtomicU64::new(0);

/// The rusty-biscuit checkout this test binary was compiled from: the nearest
/// ancestor of the crate directory carrying a `.git` entry (a directory for a
/// clone, a file for a worktree), canonicalized.
///
/// ## Returns
///
/// `None` when no ancestor carries `.git`, as in a relocated
/// `cargo nextest archive` run: there is no checkout there to be captured by.
pub fn checkout_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .and_then(|checkout| checkout.canonicalize().ok())
}

/// Reject a fixture workspace that lives inside the rusty-biscuit checkout.
///
/// Both arguments must already be canonical — [`CliProcessFixture::named`]
/// canonicalizes them exactly as [`MdCommandBuilder::ambient_context`]
/// canonicalizes its own containment check — which is what makes a
/// symlink-spelled workspace inside the checkout compare as inside.
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
         checkout {}. Repository discovery walks out of the workspace and finds that \
         checkout, so every L1 spawn would run against it rather than against the \
         workspace the test built. Point {TEMP_DIR_VARIABLE} at a directory outside \
         the checkout: it is what `std::env::temp_dir()` reads, and the fixture \
         workspace is built there.",
        workspace_root.display(),
        checkout_root.display(),
    ))
}

/// Hermetic defaults for tests that execute the shipped `md` binary.
///
/// The workspace root is disposable and stays alive through process
/// completion *and* cleanup: it is removed only when the fixture drops, after
/// the test's assertions have read whatever the child wrote. Each test
/// constructs its own fixture; parallel tests never share a root.
pub struct CliProcessFixture {
    root: tempfile::TempDir,
    cwd: PathBuf,
    home: PathBuf,
    bin_dir: PathBuf,
    config_dir: PathBuf,
    cache_dir: PathBuf,
    tmp_dir: PathBuf,
    /// Path of an empty file handed to fixture-side `git` as
    /// `GIT_CONFIG_GLOBAL`, so a host global config (signing, hooksPath,
    /// identity) never reaches a fixture repository.
    git_global_config: PathBuf,
}

impl CliProcessFixture {
    pub fn new() -> Self {
        Self::named("md-fixture")
    }

    pub fn named(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = WORKSPACE_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = tempfile::Builder::new()
            .prefix(&format!("{prefix}-{}-{nonce}-{unique}-", process::id()))
            .tempdir()
            .expect("fixture workspace must be creatable under the temp dir");
        // Fires before anything is written into the workspace, so the
        // developer meets the cause rather than a downstream symptom. The
        // canonicalize is the symlink defense: a root spelled outside the
        // checkout that resolves inside it must compare as inside.
        if let Some(checkout) = checkout_root() {
            let canonical = root
                .path()
                .canonicalize()
                .expect("fixture workspace must exist");
            if let Some(error) = checkout_containment_error(&canonical, &checkout) {
                panic!("{error}");
            }
        }
        let workspace = root.path();
        let cwd = workspace.join("cwd");
        let home = workspace.join("home");
        let bin_dir = workspace.join("bin");
        let config_dir = workspace.join("config");
        let cache_dir = workspace.join("cache");
        let tmp_dir = workspace.join("tmp");
        for dir in [&cwd, &home, &bin_dir, &config_dir, &cache_dir, &tmp_dir] {
            fs::create_dir_all(dir).expect("fixture subdirectories must be creatable");
        }
        let git_global_config = workspace.join("git-config-global");
        fs::write(&git_global_config, b"").expect("empty git global config must be writable");
        Self {
            root,
            cwd,
            home,
            bin_dir,
            config_dir,
            cache_dir,
            tmp_dir,
            git_global_config,
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

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// The fixture-owned cache directory. On Unix this is what the builder
    /// pins as `XDG_CACHE_HOME`; on Windows the equivalent influence is
    /// `LOCALAPPDATA`, which points at the fixture `home`. Compose tests that
    /// exercise persistent caching still pass `--cache-root` explicitly.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn tmp_dir(&self) -> &Path {
        &self.tmp_dir
    }

    /// Root of the fixture's temporary workspace; the containment boundary
    /// [`MdCommandBuilder::ambient_context`] enforces.
    pub fn workspace_path(&self) -> &Path {
        self.root.path()
    }

    /// A `md` command with the hermetic defaults described in the module docs.
    pub fn command(&self) -> assert_cmd::Command {
        self.command_builder().build()
    }

    /// The same command as a `std::process::Command`, for a test that has to
    /// hold the child alive. Identical policy to `command()`.
    pub fn command_std(&self) -> Command {
        self.command_builder().build_std()
    }

    /// The same command, before its defaults are traded for a named escape.
    pub fn command_builder(&self) -> MdCommandBuilder<'_> {
        MdCommandBuilder {
            fixture: self,
            path_policy: PathPolicy::Minimal,
            current_dir: self.cwd.clone(),
            inherit_env: true,
            declared: Vec::new(),
        }
    }

    /// Write `content` at `relative` under the workspace, creating parent
    /// directories, and return the absolute path.
    pub fn write_file(&self, relative: impl AsRef<Path>, content: &str) -> PathBuf {
        let path = self.workspace_path().join(relative);
        write(&path, content);
        path
    }

    /// `git init` a disposable repository at `dir`, with repository-local
    /// identity and no host configuration: the host's global and system
    /// gitconfigs are bypassed (`GIT_CONFIG_NOSYSTEM=1` plus a fixture-owned
    /// `GIT_CONFIG_GLOBAL`), so signing keys, `core.hooksPath`, and identity
    /// from the developer's machine cannot reach it.
    ///
    /// ## Returns
    ///
    /// Whether `git init` succeeded; false means `git` is unavailable on this
    /// host and the caller should skip.
    pub fn initialize_repository_at(&self, dir: &Path) -> bool {
        fs::create_dir_all(dir).expect("repository directory must be creatable");
        if !git(&self.git_global_config)
            .arg("init")
            .current_dir(dir)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return false;
        }
        // Repository-local identity only, so commits inside the fixture never
        // consult — or need — the host's global identity or signing config.
        for (key, value) in [
            ("user.name", "md fixture"),
            ("user.email", "md-fixture@example.invalid"),
            ("commit.gpgsign", "false"),
        ] {
            let configured = git(&self.git_global_config)
                .args(["config", "--local", key, value])
                .current_dir(dir)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);
            if !configured {
                return false;
            }
        }
        true
    }

    /// `git init` a disposable repository at the fixture `cwd`; see
    /// [`CliProcessFixture::initialize_repository_at`].
    pub fn initialize_repository(&self) -> bool {
        self.initialize_repository_at(&self.cwd)
    }

    /// Write a document-plus-schema topology whose `$schema` reference stays
    /// relative: `schema_ref` is the schema's path *relative to the document's
    /// directory*, so relocating the pair (or copying shipped content through
    /// [`copy_tree`]) preserves the relative-resolution relationship instead
    /// of rewriting it to an absolute path.
    pub fn nested_schema_layout(
        &self,
        doc_relative: &str,
        schema_relative: &str,
        doc_content: impl FnOnce(&str) -> String,
        schema_content: &str,
    ) -> (PathBuf, PathBuf) {
        let doc = self.workspace_path().join(doc_relative);
        let schema = self.workspace_path().join(schema_relative);
        let schema_ref = relative_reference(&doc, &schema);
        let doc_path = self.write_file(doc_relative, &doc_content(&schema_ref));
        let schema_path = self.write_file(schema_relative, schema_content);
        (doc_path, schema_path)
    }

    /// Write a main document that transcludes `part_name` by relative
    /// reference plus the part itself, both inside `dir_relative`; returns
    /// `(main, part)`.
    pub fn relative_reference_layout(
        &self,
        dir_relative: &str,
        part_name: &str,
        part_content: &str,
    ) -> (PathBuf, PathBuf) {
        let main = self.write_file(
            Path::new(dir_relative).join("doc.md"),
            &format!("# Layout\n\n::file ./{part_name}\n"),
        );
        let part = self.write_file(Path::new(dir_relative).join(part_name), part_content);
        (main, part)
    }
}

/// `to` expressed relative to `from`'s parent, with `./`-prefixed siblings —
/// the shape a `$schema` or `::file` reference keeps when content relocates
/// together.
///
/// ## Panics
///
/// When the two paths share no common ancestor (they always do inside one
/// fixture workspace; this builder exists only for fixture-internal pairs).
pub fn relative_reference(from: &Path, to: &Path) -> String {
    let from_dir: Vec<Component> = from
        .parent()
        .unwrap_or(Path::new("."))
        .components()
        .collect();
    let to_parts: Vec<Component> = to.components().collect();
    let mut shared = 0usize;
    while shared < from_dir.len() && shared < to_parts.len() && from_dir[shared] == to_parts[shared]
    {
        shared += 1;
    }
    let depth = from_dir.len() - shared;
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), depth).collect();
    parts.extend(
        to_parts
            .iter()
            .skip(shared)
            .map(|component| component.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        return ".".to_string();
    }
    let joined = parts.join("/");
    if joined.starts_with("..") {
        joined
    } else {
        format!("./{joined}")
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

/// Builder for the one supported L1 spawn of `md`.
///
/// Obtained from [`CliProcessFixture::command_builder`]; see the module docs
/// for the defaults and for what each escape costs.
#[must_use = "a builder does nothing until `build()` is called"]
pub struct MdCommandBuilder<'fixture> {
    fixture: &'fixture CliProcessFixture,
    path_policy: PathPolicy,
    current_dir: PathBuf,
    inherit_env: bool,
    /// Protected-key overrides the call site declared, applied after the
    /// fixture defaults so a declaration out-ranks the value it replaces.
    declared: Vec<EnvironmentOp>,
}

impl MdCommandBuilder<'_> {
    /// Opt out to a `PATH` holding the fixture `bin` and nothing else.
    ///
    /// For tests whose assertion *is* that nothing else was found. The call
    /// site states which proof it depends on.
    pub fn fake_only_path(mut self) -> Self {
        self.path_policy = PathPolicy::FakeOnly;
        self
    }

    /// Opt out to the fixture `bin` followed by the full host `PATH`.
    ///
    /// For tests needing a tool the minimal system set does not carry (a
    /// real shell for expansion coverage, for example). The call site names
    /// that tool; everything installed on the machine becomes visible to the
    /// child.
    pub fn host_path(mut self) -> Self {
        self.path_policy = PathPolicy::Host;
        self
    }

    /// Opt out to a launch CWD other than the fixture `cwd`.
    ///
    /// For tests whose subject is the launch context itself — repository
    /// discovery from a nested directory, schema lookup relative to the
    /// document. `dir` must be a directory the test built inside its own
    /// workspace.
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
             the launch context must be a directory the test built itself",
            dir.display(),
            self.fixture.workspace_path().display()
        );
        // The caller's spelling is pinned, not `canonical_dir`: on macOS the
        // canonical form is `/private/var/...` where the fixture hands out
        // `/var/...`, and a test asserting on paths in md's output would then
        // see a prefix it never chose.
        self.current_dir = dir.to_path_buf();
        self
    }

    /// Give the child nothing but what the fixture and the call site set.
    ///
    /// A tightening rather than an escape: it exists for tests that assert on
    /// what md *found* in its own environment. The Windows console plumbing
    /// (`SystemRoot`, `COMSPEC`, `PATHEXT`) is restored inside `build()`, or
    /// every Windows spawn would silently fail to resolve the fixture's
    /// `.cmd` stubs.
    pub fn inherit_no_env(mut self) -> Self {
        self.inherit_env = false;
        self
    }

    /// Declare the rendering input this test's subject depends on.
    ///
    /// ## Panics
    ///
    /// When `key` is not a rendering input the spawn contract pins.
    pub fn rendering_input(self, key: &str, value: impl AsRef<OsStr>) -> Self {
        let op = EnvironmentOp::Set(key.into(), value.as_ref().to_os_string());
        self.declare(key, ProtectedClass::RenderingInput, op)
    }

    /// Declare that this test's subject requires a pinned rendering input to
    /// be *absent* — `NO_COLOR` for a test whose assertion is colored output,
    /// for example.
    ///
    /// ## Panics
    ///
    /// When `key` is not a rendering input the spawn contract pins.
    pub fn rendering_input_removed(self, key: &str) -> Self {
        let op = EnvironmentOp::Remove(key.into());
        self.declare(key, ProtectedClass::RenderingInput, op)
    }

    /// Declare the darkmatter application input this test's subject depends
    /// on — a `DARKMATTER_*`/`DM_*`/`MD_*` switch, or a name `md` reads
    /// directly such as `HASH_PROPERTY`.
    ///
    /// ## Panics
    ///
    /// When `key` is not an application input the spawn contract pins.
    pub fn application_input(self, key: &str, value: impl AsRef<OsStr>) -> Self {
        let op = EnvironmentOp::Set(key.into(), value.as_ref().to_os_string());
        self.declare(key, ProtectedClass::ApplicationInput, op)
    }

    /// Declare that this test's subject requires a pinned application input to
    /// be absent.
    ///
    /// ## Panics
    ///
    /// When `key` is not an application input the spawn contract pins.
    pub fn application_input_removed(self, key: &str) -> Self {
        let op = EnvironmentOp::Remove(key.into());
        self.declare(key, ProtectedClass::ApplicationInput, op)
    }

    /// Pin a deterministic, non-capable terminal of a fixed size.
    ///
    /// The complete rendering frame rather than only the size: `TERM=dumb` and
    /// color off are restated here so a layout assertion cannot silently
    /// change meaning if the fixture's own color default ever does.
    pub fn plain_terminal(self, columns: u16, lines: u16) -> Self {
        self.rendering_input("COLUMNS", columns.to_string())
            .rendering_input("LINES", lines.to_string())
            .rendering_input("TERM", "dumb")
            .rendering_input_removed("COLORTERM")
            .rendering_input("NO_COLOR", "1")
            .rendering_input_removed("FORCE_COLOR")
    }

    fn declare(mut self, key: &str, expected: ProtectedClass, op: EnvironmentOp) -> Self {
        assert_declarable(key, expected);
        self.declared.push(op);
        self
    }

    /// Materialize the command with the contract described in the module
    /// docs, as an `assert_cmd::Command` run to completion.
    pub fn build(self) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("md").unwrap();
        self.child_environment().apply(&mut command);
        command
    }

    /// The same contract on a `std::process::Command`, for a call site that
    /// has to keep the child. Resolved through `CARGO_BIN_EXE_md` — the same
    /// mechanism `assert_cmd` uses — so relocated runs still find the binary.
    pub fn build_std(self) -> Command {
        let md = std::env::var_os("CARGO_BIN_EXE_md").unwrap_or_else(|| OsString::from("md"));
        let mut command = Command::new(md);
        self.child_environment().apply(&mut command);
        command
    }

    /// The policy both surfaces apply, computed once. Ordered: clear (and the
    /// Windows console restore) first, then the inherited scrub, then the
    /// fixture defaults, then the call site's declarations — so a default
    /// inside a scrubbed namespace survives its own sweep, and a declared
    /// input out-ranks the default it replaces.
    pub fn child_environment(&self) -> ChildEnvironment {
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
        for (key, value) in [
            ("HOME", home.to_os_string()),
            ("USERPROFILE", home.to_os_string()),
            ("APPDATA", home.to_os_string()),
            ("LOCALAPPDATA", home.to_os_string()),
            (
                "XDG_CONFIG_HOME",
                self.fixture.config_dir().as_os_str().to_os_string(),
            ),
            (
                "XDG_CACHE_HOME",
                self.fixture.cache_dir().as_os_str().to_os_string(),
            ),
            ("GIT_CONFIG_NOSYSTEM", "1".into()),
            ("NO_COLOR", "1".into()),
            ("PATH", self.path_value()),
        ] {
            ops.push(EnvironmentOp::Set(key.into(), value));
        }
        let temp = self.fixture.tmp_dir().as_os_str().to_os_string();
        if cfg!(windows) {
            ops.push(EnvironmentOp::Set("TEMP".into(), temp.clone()));
            ops.push(EnvironmentOp::Set("TMP".into(), temp));
        } else {
            ops.push(EnvironmentOp::Set("TMPDIR".into(), temp));
        }
        ops.extend(self.declared.iter().cloned());
        ChildEnvironment {
            clear: !self.inherit_env,
            ops,
            current_dir: self.current_dir.clone(),
        }
    }

    fn path_value(&self) -> OsString {
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
            PathPolicy::Host => {
                let mut paths: Vec<PathBuf> = vec![bin_dir.to_path_buf()];
                paths.extend(std::env::split_paths(
                    &std::env::var_os("PATH").unwrap_or_default(),
                ));
                std::env::join_paths(paths).expect("host PATH must join")
            }
        }
    }
}

/// Reject a declaration the spawn contract cannot honor.
///
/// The containment arm is the one that matters: a home, cache, temp, or Git
/// plumbing value handed back to the child is exactly the contamination the
/// fixture exists to prevent, so it has no declared form at any spelling.
///
/// ## Panics
///
/// When `key` is not a behavior input of class `expected`.
fn assert_declarable(key: &str, expected: ProtectedClass) {
    let expected_method = expected
        .declaring_method()
        .expect("only declarable classes reach this validator");
    match protected_class(key) {
        Some(class) if class == expected => {}
        Some(class) if class.is_containment() => panic!(
            "`{key}` is a {} the spawn contract owns, not a {}. Undoing it hands the child \
             host state the fixture removed, so there is no declared override; if the launch \
             context itself is the subject, use `ambient_context`.",
            class.description(),
            expected.description(),
        ),
        Some(class) => panic!(
            "`{key}` is a {}, not a {}: declare it with `{}`.",
            class.description(),
            expected.description(),
            class
                .declaring_method()
                .expect("a non-containment class declares"),
        ),
        None if key == PATH_VARIABLE => panic!(
            "`PATH` is composed by the builder: use `host_path()` or `fake_only_path()` \
             rather than `{expected_method}`."
        ),
        None => panic!(
            "`{key}` is not a {} the spawn contract pins, so it needs no declaration: set it \
             on the command `build()` returns.",
            expected.description(),
        ),
    }
}

/// The inherited environment families the spawn contract removes.
///
/// The darkmatter namespaces go by prefix (`DARKMATTER_*` grows without this
/// list being told; `MD_DRY_RUN` is the `md` binary's own switch and an
/// inherited value silently flips every write test into dry-run). Only names
/// the *parent* actually carries are listed — a prefix rule can only be
/// expressed as a key list by enumerating the parent.
pub fn inherited_scrub_keys() -> Vec<OsString> {
    const NAMES: &[&str] = &[
        // Git plumbing: overrides cwd-based repository discovery.
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        // Windows home-splitting variables; the whole home family must come
        // from the fixture or none of it does.
        "HOMEDRIVE",
        "HOMEPATH",
        // Terminal capability and color inputs read before any fallback.
        "COLUMNS",
        "LINES",
        "TERM",
        "COLORTERM",
        "COLORFGBG",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        // darkmatter application inputs read by name.
        "THEME",
        "CODE_THEME",
        "PREFER_ITALICS",
        "TERMINAL_IMAGES",
        "HASH_PROPERTY",
        "HASH_IGNORE_PROPERTIES",
        "BASELINE_SCHEMA",
    ];
    let mut keys: Vec<OsString> = std::env::vars_os()
        .map(|(key, _)| key)
        .filter(|key| {
            let key = key.to_string_lossy();
            key.starts_with("DARKMATTER_") || key.starts_with("DM_") || key.starts_with("MD_")
        })
        .collect();
    keys.extend(GIT_PLUMBING_VARS.iter().map(OsString::from));
    keys.extend(
        std::env::vars_os()
            .map(|(key, _)| key)
            .filter(|key| key.to_string_lossy().starts_with("GIT_CONFIG_")),
    );
    keys.extend(NAMES.iter().map(|name| OsString::from(*name)));
    // Context-expression inputs (`env.AGENT`, `env.MODEL`) and log filtering
    // change compose output and stderr shape when a developer's shell exports
    // them; both are darkmatter inputs read by name.
    keys.extend(["AGENT", "MODEL", "RUST_LOG"].map(OsString::from));
    keys
}

/// One ordered operation on the child's environment block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentOp {
    Remove(OsString),
    Set(OsString, OsString),
}

/// The spawn contract as data, ready to apply to either command surface.
pub struct ChildEnvironment {
    /// Whether the child starts from an empty block.
    clear: bool,
    /// Removes and sets, in application order.
    ops: Vec<EnvironmentOp>,
    /// The pinned launch directory.
    current_dir: PathBuf,
}

impl ChildEnvironment {
    /// Every key the policy *sets* on the child, in application order.
    pub fn set_keys(&self) -> Vec<OsString> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                EnvironmentOp::Set(key, _) => Some(key.clone()),
                EnvironmentOp::Remove(_) => None,
            })
            .collect()
    }

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
/// that merely forwarded them could be replaced by the inherent method and
/// the policy would drift back into the call site unnoticed.
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
const WINDOWS_SYSTEM_ROOT_FALLBACK: &str = r"C:\Windows";

/// `PATHEXT` for a parent process that has none — enough for the `.cmd`
/// stubs the fixture writes, and deliberately shorter than the Windows
/// default.
const WINDOWS_PATHEXT_FALLBACK: &str = ".COM;.EXE;.BAT;.CMD";

fn windows_system_root() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(WINDOWS_SYSTEM_ROOT_FALLBACK))
}

/// The Windows console plumbing to put back after
/// [`MdCommandBuilder::inherit_no_env`] has cleared the child; empty
/// everywhere else.
///
/// `env_clear()` on Windows also removes `PATHEXT` — without which the
/// fixture `bin` resolves none of its `.cmd` stubs — `COMSPEC`, and the
/// `SystemRoot` that [`minimal_system_path`] reads. The platform test is
/// `cfg!` rather than `#[cfg]` so both arms compile on every leg: a Unix run
/// proves the restore stays Windows-only.
fn windows_console_variables() -> Vec<(OsString, OsString)> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let system_root = windows_system_root();
    let comspec = std::env::var_os("COMSPEC").unwrap_or_else(|| {
        system_root
            .join("System32")
            .join("cmd.exe")
            .into_os_string()
    });
    let pathext =
        std::env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(WINDOWS_PATHEXT_FALLBACK));
    vec![
        ("SystemRoot", system_root.into_os_string()),
        ("COMSPEC", comspec),
        ("PATHEXT", pathext),
    ]
    .into_iter()
    .map(|(key, value)| (OsString::from(key), value))
    .collect()
}

/// The system directories the default `PATH` carries behind the fixture
/// `bin`: `/usr/bin:/bin` on Unix (resolves `sh` and `git` for shell
/// expansion), `%SystemRoot%\System32` on Windows (which resolves none of
/// the Unix four — a fixture needing a POSIX utility is Unix-gated, spelled
/// absolutely, or stubbed in the fixture `bin`).
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

/// Write `content` to `path`, creating parent directories.
pub fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent directories must be creatable");
    }
    fs::write(path, content).expect("fixture file must be writable");
}

/// Write an executable stub; on Windows a `.cmd` sibling is the resolvable
/// form and callers append the extension themselves.
pub fn write_executable(path: &Path, content: &str) {
    write(path, content);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .expect("stub must exist to chmod")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("stub chmod");
    }
}

/// A parent-side helper tool with [`GIT_PLUMBING_VARS`] removed.
///
/// Helper tools build the fixture's inputs; they are not the subject of any
/// assertion and need the host `PATH` and `HOME` to run — the opposite of
/// what the `md` child needs.
pub fn helper_command(program: &str) -> Command {
    let mut command = Command::new(program);
    for key in GIT_PLUMBING_VARS {
        command.env_remove(key);
    }
    command
}

/// A parent-side `git` that also bypasses the host's system and global
/// configuration: repository-local state only. `global_config` is an empty
/// fixture-owned file (see [`CliProcessFixture`]).
pub fn git(global_config: &Path) -> Command {
    let mut command = helper_command("git");
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", global_config);
    command
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_SYSTEM");
    command
}

/// Copy a directory tree byte-for-byte, preserving relative structure — the
/// relocation rule for shipped content: references between copied files keep
/// resolving because their relative relationships survive the move.
pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_symlink() {
            let destination = fs::read_link(entry.path())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&destination, &target)?;
            #[cfg(windows)]
            {
                if destination.is_absolute() {
                    std::os::windows::fs::symlink_file(&destination, &target)?;
                } else {
                    std::os::windows::fs::symlink_file(
                        &target.parent().unwrap().join(&destination).canonicalize()?,
                        &target,
                    )?;
                }
            }
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
