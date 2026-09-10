//! Deterministic process fixtures and bounded terminal capture for `sniff-cli` tests.
//!
//! The default command runs from a disposable directory outside the checkout,
//! redirects home/config/cache/install roots into the fixture, removes Git
//! plumbing, credentials, Sniff inputs, and rendering controls, and uses only
//! the fixture `bin` plus the platform's minimal system path. Tests that need
//! host tools, a fake-only path, or a repository they built use the named
//! builder escapes instead of overriding the returned command.

#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(feature = "test-fixtures")]
use std::time::{Duration, Instant};

#[cfg(feature = "test-fixtures")]
use biscuit_test_harness::{CapturedFrame, TerminalHarness};

/// Poll a real terminal until the asserted frame is observable or the deadline
/// expires. Returning the last frame keeps assertion failures diagnostic.
#[cfg(feature = "test-fixtures")]
pub fn capture_until(
    harness: &mut impl TerminalHarness,
    timeout: Duration,
    predicate: impl Fn(&CapturedFrame) -> bool,
) -> CapturedFrame {
    const POLL_INTERVAL: Duration = Duration::from_millis(40);

    let deadline = Instant::now() + timeout;
    let mut last = harness.capture().expect("capture failed");
    while !predicate(&last) && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
        if let Ok(frame) = harness.capture() {
            last = frame;
        }
    }
    last
}

/// Git variables that override cwd-based repository discovery.
pub const GIT_PLUMBING_VARS: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

const TEMP_DIR_VARIABLE: &str = if cfg!(windows) {
    "TMP (or TEMP)"
} else {
    "TMPDIR"
};

/// The canonical checkout root, when the test binary still resides in one.
pub fn checkout_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .and_then(|checkout| checkout.canonicalize().ok())
}

/// Return the diagnostic for a fixture root contained by the checkout.
///
/// Both paths must be canonical. Canonical comparison prevents a symlink from
/// disguising an in-checkout fixture root.
pub fn checkout_containment_error(workspace: &Path, checkout: &Path) -> Option<String> {
    workspace.starts_with(checkout).then(|| {
        format!(
            "fixture precondition: the fixture workspace {} is inside the rusty-biscuit \
             checkout {}. Point {TEMP_DIR_VARIABLE} at a directory outside the checkout",
            workspace.display(),
            checkout.display()
        )
    })
}

/// One disposable root and all process inputs derived from it.
pub struct SniffCliFixture {
    root: tempfile::TempDir,
    cwd: PathBuf,
    home: PathBuf,
    bin: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    data: PathBuf,
    tmp: PathBuf,
    program_files: PathBuf,
    git_global_config: PathBuf,
}

impl SniffCliFixture {
    pub fn new() -> Self {
        Self::named("sniff-cli-fixture")
    }

    pub fn named(prefix: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("fixture workspace must be creatable");
        if let Some(checkout) = checkout_root() {
            let canonical = root.path().canonicalize().expect("fixture root must exist");
            if let Some(error) = checkout_containment_error(&canonical, &checkout) {
                panic!("{error}");
            }
        }

        let cwd = root.path().join("cwd");
        let home = root.path().join("home");
        let bin = root.path().join("bin");
        let config = root.path().join("config");
        let cache = root.path().join("cache");
        let data = root.path().join("data");
        let tmp = root.path().join("tmp");
        let program_files = root.path().join("program-files");
        for dir in [
            &cwd,
            &home,
            &bin,
            &config,
            &cache,
            &data,
            &tmp,
            &program_files,
        ] {
            fs::create_dir_all(dir).expect("fixture directory must be creatable");
        }
        let git_global_config = root.path().join("git-config-global");
        fs::write(&git_global_config, b"").expect("fixture Git config must be writable");

        Self {
            root,
            cwd,
            home,
            bin,
            config,
            cache,
            data,
            tmp,
            program_files,
            git_global_config,
        }
    }

    pub fn workspace_path(&self) -> &Path {
        self.root.path()
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    pub fn tmp_dir(&self) -> &Path {
        &self.tmp
    }

    /// The supported `assert_cmd` surface for a Sniff L1 test.
    pub fn command(&self) -> assert_cmd::Command {
        self.command_builder().build()
    }

    /// The identical policy on a raw command for live-child tests.
    pub fn command_std(&self) -> Command {
        self.command_builder().build_std()
    }

    pub fn command_builder(&self) -> SniffCommandBuilder<'_> {
        SniffCommandBuilder {
            fixture: self,
            path_policy: PathPolicy::Minimal,
            current_dir: self.cwd.clone(),
            clear_environment: false,
        }
    }

    /// Build a parent-side Git command isolated from host Git configuration.
    pub fn git(&self) -> Command {
        git_command(&self.git_global_config)
    }
}

/// An assert command that owns the disposable process fixture it uses.
pub struct OwnedSniffCommand {
    command: assert_cmd::Command,
    _fixture: SniffCliFixture,
}

impl Deref for OwnedSniffCommand {
    type Target = assert_cmd::Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl DerefMut for OwnedSniffCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

impl OwnedSniffCommand {
    /// Run from a disposable context built by the test.
    pub fn ambient_context(&mut self, dir: &Path) -> &mut Self {
        assert_disposable_context(dir);
        self.command.current_dir(dir);
        self
    }
}

/// Named CWD escape for a command already configured through a fluent chain.
pub trait DisposableAmbientContext {
    fn ambient_context(&mut self, dir: &Path) -> &mut Self;
}

impl DisposableAmbientContext for assert_cmd::Command {
    fn ambient_context(&mut self, dir: &Path) -> &mut Self {
        assert_disposable_context(dir);
        self.current_dir(dir)
    }
}

/// The fluent escape's ownership contract: the launch directory must be one the
/// test built and can throw away. Rejecting everything outside the system
/// temporary root keeps a developer-owned repository or home subdirectory —
/// whose Git state and contents vary per machine — from becoming a test input.
fn assert_disposable_context(dir: &Path) {
    let canonical_dir = dir.canonicalize().unwrap_or_else(|error| {
        panic!(
            "ambient-context directory {} must exist before use: {error}",
            dir.display()
        )
    });
    if let Some(checkout) = checkout_root() {
        assert!(
            !canonical_dir.starts_with(&checkout),
            "ambient-context escape: {} is inside the rusty-biscuit checkout {}. \
             The fixture must own its launch directory",
            dir.display(),
            checkout.display()
        );
    }
    let temporary_root = std::env::temp_dir()
        .canonicalize()
        .expect("system temporary root must exist");
    assert!(
        canonical_dir.starts_with(&temporary_root),
        "ambient-context escape: {} is not disposable — it is outside the system \
         temporary root {}. The fixture must own its launch directory; build it \
         with `tempfile` or use `SniffCommandBuilder::ambient_context`",
        dir.display(),
        temporary_root.display()
    );
}

/// Build the default isolated command while retaining its disposable root.
pub fn owned_sniff_command() -> OwnedSniffCommand {
    let fixture = SniffCliFixture::new();
    let command = fixture.command();
    OwnedSniffCommand {
        command,
        _fixture: fixture,
    }
}

#[derive(Clone, Copy)]
enum PathPolicy {
    Minimal,
    FakeOnly,
    Host,
}

/// Builder for the one supported L1 spawn of `sniff`.
#[must_use = "a command builder does nothing until built"]
pub struct SniffCommandBuilder<'fixture> {
    fixture: &'fixture SniffCliFixture,
    path_policy: PathPolicy,
    current_dir: PathBuf,
    clear_environment: bool,
}

impl SniffCommandBuilder<'_> {
    /// Use only fixture stubs. The call site must name the absence being proved.
    pub fn fake_only_path(mut self) -> Self {
        self.path_policy = PathPolicy::FakeOnly;
        self
    }

    /// Add the host PATH. The call site must name the real tool under test.
    pub fn host_path(mut self) -> Self {
        self.path_policy = PathPolicy::Host;
        self
    }

    /// Pin launch context to an existing directory inside this fixture.
    pub fn ambient_context(mut self, dir: &Path) -> Self {
        let canonical_dir = dir.canonicalize().unwrap_or_else(|error| {
            panic!(
                "ambient-context directory {} must exist before use: {error}",
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
            "ambient-context escape: {} is outside fixture workspace {}",
            dir.display(),
            self.fixture.workspace_path().display()
        );
        self.current_dir = dir.to_path_buf();
        self
    }

    /// Start from an empty environment before applying fixture defaults.
    pub fn inherit_no_env(mut self) -> Self {
        self.clear_environment = true;
        self
    }

    pub fn build(self) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("sniff").unwrap();
        self.environment().apply(&mut command);
        command
    }

    pub fn build_std(self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin("sniff"));
        self.environment().apply(&mut command);
        command
    }

    /// Apply this builder's policy to a recording or wrapper command.
    pub fn apply_policy_to<C: ConfigurableCommand>(&self, command: &mut C) {
        self.environment().apply(command);
    }

    fn environment(&self) -> ChildEnvironment {
        let mut operations = Vec::new();
        if self.clear_environment {
            operations.extend(
                windows_console_variables()
                    .into_iter()
                    .map(|(key, value)| EnvironmentOperation::Set(key, value)),
            );
        }
        operations.extend(
            inherited_scrub_keys()
                .into_iter()
                .map(EnvironmentOperation::Remove),
        );

        let fixture = self.fixture;
        let home = fixture.home.as_os_str().to_os_string();
        let sets = [
            ("HOME", home.clone()),
            ("USERPROFILE", home.clone()),
            ("APPDATA", home.clone()),
            ("LOCALAPPDATA", home.clone()),
            ("XDG_CONFIG_HOME", fixture.config.clone().into_os_string()),
            ("XDG_CACHE_HOME", fixture.cache.clone().into_os_string()),
            ("XDG_DATA_HOME", fixture.data.clone().into_os_string()),
            ("TMPDIR", fixture.tmp.clone().into_os_string()),
            ("TMP", fixture.tmp.clone().into_os_string()),
            ("TEMP", fixture.tmp.clone().into_os_string()),
            (
                "ProgramFiles",
                fixture.program_files.clone().into_os_string(),
            ),
            (
                "ProgramFiles(x86)",
                fixture.program_files.clone().into_os_string(),
            ),
            ("GIT_CONFIG_NOSYSTEM", OsString::from("1")),
            (
                "GIT_CONFIG_GLOBAL",
                fixture.git_global_config.clone().into_os_string(),
            ),
            ("NO_COLOR", OsString::from("1")),
            ("PATH", self.path_value()),
        ];
        operations.extend(
            sets.into_iter()
                .map(|(key, value)| EnvironmentOperation::Set(OsString::from(key), value)),
        );

        ChildEnvironment {
            clear: self.clear_environment,
            operations,
            current_dir: self.current_dir.clone(),
        }
    }

    fn path_value(&self) -> OsString {
        let bin = self.fixture.bin_dir();
        let entries: Vec<PathBuf> = match self.path_policy {
            PathPolicy::Minimal => std::iter::once(bin.to_path_buf())
                .chain(minimal_system_path())
                .collect(),
            PathPolicy::FakeOnly => vec![bin.to_path_buf()],
            PathPolicy::Host => {
                let mut entries = vec![bin.to_path_buf()];
                if let Some(path) = std::env::var_os("PATH") {
                    entries.extend(std::env::split_paths(&path));
                }
                entries
            }
        };
        std::env::join_paths(entries).expect("fixture PATH entries must join")
    }
}

/// A command environment represented as ordered operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentOperation {
    Remove(OsString),
    Set(OsString, OsString),
}

/// The policy shared by assert and raw command surfaces.
pub struct ChildEnvironment {
    clear: bool,
    operations: Vec<EnvironmentOperation>,
    current_dir: PathBuf,
}

impl ChildEnvironment {
    pub fn apply<C: ConfigurableCommand>(&self, command: &mut C) {
        if self.clear {
            command.clear_environment();
        }
        for operation in &self.operations {
            match operation {
                EnvironmentOperation::Remove(key) => command.remove_environment(key),
                EnvironmentOperation::Set(key, value) => command.set_environment(key, value),
            }
        }
        command.set_current_dir(&self.current_dir);
    }
}

pub trait ConfigurableCommand {
    fn clear_environment(&mut self);
    fn remove_environment(&mut self, key: &OsStr);
    fn set_environment(&mut self, key: &OsStr, value: &OsStr);
    fn set_current_dir(&mut self, dir: &Path);
}

impl ConfigurableCommand for assert_cmd::Command {
    fn clear_environment(&mut self) {
        self.env_clear();
    }

    fn remove_environment(&mut self, key: &OsStr) {
        self.env_remove(key);
    }

    fn set_environment(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }

    fn set_current_dir(&mut self, dir: &Path) {
        self.current_dir(dir);
    }
}

impl ConfigurableCommand for Command {
    fn clear_environment(&mut self) {
        self.env_clear();
    }

    fn remove_environment(&mut self, key: &OsStr) {
        self.env_remove(key);
    }

    fn set_environment(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }

    fn set_current_dir(&mut self, dir: &Path) {
        self.current_dir(dir);
    }
}

fn inherited_scrub_keys() -> Vec<OsString> {
    const EXACT: &[&str] = &[
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "HOMEDRIVE",
        "HOMEPATH",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "COLUMNS",
        "LINES",
        "TERM_WIDTH",
        "COLORTERM",
        "COLORFGBG",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "RUST_LOG",
        "VIRTUAL_ENV",
        "SNIFF_WAN_IP_ENDPOINTS",
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GITLAB_TOKEN",
        "GITLAB_PRIVATE_TOKEN",
        "GITEA_TOKEN",
        "FORGEJO_TOKEN",
        "CODEBERG_TOKEN",
        "BITBUCKET_TOKEN",
        "AZURE_DEVOPS_TOKEN",
        "AWS_CODECOMMIT_GIT_USERNAME",
        "AWS_CODECOMMIT_GIT_PASSWORD",
    ];
    let mut keys: Vec<OsString> = std::env::vars_os()
        .map(|(key, _)| key)
        .filter(|key| {
            let key = key.to_string_lossy();
            key.starts_with("SNIFF_") || key.starts_with("GIT_CONFIG_")
        })
        .collect();
    keys.extend(EXACT.iter().map(OsString::from));
    keys
}

pub fn minimal_system_path() -> Vec<PathBuf> {
    #[cfg(unix)]
    {
        vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")]
    }
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot")
            .or_else(|| std::env::var_os("WINDIR"))
            .unwrap_or_else(|| OsString::from(r"C:\Windows"));
        vec![PathBuf::from(root).join("System32")]
    }
}

fn windows_console_variables() -> Vec<(OsString, OsString)> {
    #[cfg(windows)]
    {
        ["SystemRoot", "COMSPEC", "PATHEXT"]
            .into_iter()
            .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Build a parent-side Git command with all inherited plumbing removed.
pub fn git_command(global_config: &Path) -> Command {
    let mut command = Command::new("git");
    for key in inherited_scrub_keys() {
        command.env_remove(key);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", global_config)
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}
