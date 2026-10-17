use std::ffi::OsString;
use std::path::{Path, PathBuf};

use biscuit_file::file_reference::home_dir;
use serde::{Deserialize, Serialize};
use sniff::os::RuntimeEnvironment;

use crate::error::WorktreeError;

/// Where a base-directory value came from, so errors can name it and give
/// the remedy that applies to that source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseDirSource {
    /// The `WT` environment variable.
    Env,
    /// `base_dir` in the config file at this path (normally `~/.worktree.json`).
    ConfigFile(PathBuf),
    /// A choice made during interactive setup.
    Prompt,
}

impl std::fmt::Display for BaseDirSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Env => f.write_str("the WT environment variable"),
            Self::ConfigFile(path) => write!(f, "the config file {}", path.display()),
            Self::Prompt => f.write_str("the setup prompt"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorktreeConfig {
    pub base_dir: String,
}

/// Returns the path to the user-level config file (`~/.worktree.json`).
///
/// The home directory comes from the environment first (`HOME`, or
/// `USERPROFILE` on Windows) and must be absolute.
pub fn config_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".worktree.json"))
}

/// Returns the candidate directories shown during interactive setup.
///
/// Candidates are `~/worktrees`, `~/wt`, and `~/.claudine/worktrees`, under
/// the same home directory as [`config_path`]. The list is empty if the home
/// directory cannot be determined.
pub fn considered_dirs() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return vec![];
    };
    vec![
        home.join("worktrees"),
        home.join("wt"),
        home.join(".claudine").join("worktrees"),
    ]
}

/// The path syntax a host uses; WSL is [`PathStyle::Posix`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathStyle {
    Posix,
    Windows,
}

impl PathStyle {
    fn host() -> Self {
        if cfg!(windows) { Self::Windows } else { Self::Posix }
    }
}

/// Whether `raw`, already known not to be absolute on a `host`-style system,
/// is spelled in the other operating system's syntax.
///
/// Only meaningful after the native absolute-path check: `//server/share/wt`
/// is absolute on POSIX and must not be called foreign there.
fn is_foreign_syntax(raw: &str, host: PathStyle) -> bool {
    match host {
        PathStyle::Posix => {
            let bytes = raw.as_bytes();
            let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
            drive || raw.starts_with(r"\\")
        }
        PathStyle::Windows => raw.starts_with('/'),
    }
}

/// Checks that `raw` is usable as a base-directory value on this host,
/// without touching the filesystem.
///
/// Returns the path spelled exactly as given: nothing is trimmed, expanded
/// (`~`, `$VAR`), unquoted, joined to the current directory, or
/// canonicalized. Acceptance is decided by the host's
/// [`Path::is_absolute`], so on Windows `C:wt` and `\wt` are rejected.
/// Whether the directory exists is checked separately.
///
/// ## Errors
///
/// - [`WorktreeError::BaseDirectoryEmpty`] — `raw` is empty or only whitespace.
/// - [`WorktreeError::BaseDirectoryForeignPath`] — `raw` uses the other OS's
///   syntax, such as `b:\wt` on POSIX or `/home/me/wt` on Windows.
/// - [`WorktreeError::BaseDirectoryNotAbsolute`] — any other relative value,
///   including `~/wt`.
pub fn validate_base_dir(raw: &str, origin: BaseDirSource) -> Result<PathBuf, WorktreeError> {
    if raw.trim().is_empty() {
        return Err(WorktreeError::BaseDirectoryEmpty { origin });
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return Ok(path);
    }
    let value = raw.to_owned();
    if is_foreign_syntax(raw, PathStyle::host()) {
        Err(WorktreeError::BaseDirectoryForeignPath { value, origin })
    } else {
        Err(WorktreeError::BaseDirectoryNotAbsolute { value, origin })
    }
}

/// A base directory that is valid on this host, for examples in messages.
pub fn example_base_dir() -> &'static str {
    if cfg!(windows) {
        r"C:\Users\me\wt"
    } else if cfg!(target_os = "macos") {
        "/Users/me/wt"
    } else {
        "/home/me/wt"
    }
}

/// A suggestion for a Windows drive path rejected under WSL, such as
/// `B:\wt` → `/mnt/b/wt`.
///
/// Returns `None` unless `runtime` is WSL and `value` is drive-absolute
/// (`B:\wt`, `B:/wt`, `B:\`); drive-relative values (`b:wt`, `b:`), network
/// shares, and `\\?\` namespace paths get no suggestion. The text is
/// conditional on the drive being mounted at WSL's default location, because
/// WSL can mount drives elsewhere and nothing here checks the mount.
pub fn wsl_drive_hint(value: &str, runtime: RuntimeEnvironment) -> Option<String> {
    if !runtime.is_wsl() {
        return None;
    }
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    if !drive_absolute {
        return None;
    }
    let letter = char::from(bytes[0]).to_ascii_lowercase();
    let mut mounted = format!("/mnt/{letter}");
    for part in value[3..].split(['\\', '/']).filter(|part| !part.is_empty()) {
        mounted.push('/');
        mounted.push_str(part);
    }
    Some(format!(
        "Under WSL, Windows drive {}: is usually mounted at /mnt/{letter}. If it is mounted there, use {mounted}.",
        char::from(bytes[0]).to_ascii_uppercase()
    ))
}

/// Saves `base_dir` to `~/.worktree.json`.
///
/// Only the path's syntax is checked; the caller establishes that it is an
/// existing directory that is not a repository root. No directory is created.
///
/// ## Errors
///
/// An invalid value is rejected before anything is written, leaving an
/// existing config unchanged: [`WorktreeError::BaseDirectoryInvalidEncoding`]
/// when the path is not Unicode, or any [`validate_base_dir`] error. These
/// name the config file as their origin.
pub fn save_config(base_dir: &Path) -> Result<(), WorktreeError> {
    let config_path = config_path().ok_or(WorktreeError::BaseDirectoryNotConfigured)?;
    save_config_to(&config_path, base_dir)
}

fn save_config_to(config_path: &Path, base_dir: &Path) -> Result<(), WorktreeError> {
    let origin = BaseDirSource::ConfigFile(config_path.to_path_buf());
    let Some(text) = base_dir.to_str() else {
        return Err(WorktreeError::BaseDirectoryInvalidEncoding { value: base_dir.to_path_buf(), origin });
    };
    validate_base_dir(text, origin)?;
    let config = WorktreeConfig { base_dir: text.to_owned() };
    let contents = serde_json::to_string_pretty(&config)?;
    std::fs::write(config_path, contents)?;
    Ok(())
}

/// Resolves the base directory for worktree storage.
///
/// ## Resolution Order
///
/// 1. `WT` environment variable, unless it is unset or exactly empty. Any
///    other value is used or rejected; an invalid `WT` never falls back to
///    the config file.
/// 2. `~/.worktree.json` — `{ "base_dir": "/path" }`.
///
/// Either value must be an absolute path on this host (see
/// [`validate_base_dir`]) naming an existing directory (symlinks to
/// directories are followed) without a `.git` file or folder directly
/// inside it. Directories deeper inside a repository are allowed.
///
/// ## Errors
///
/// - [`WorktreeError::BaseDirectoryNotConfigured`] — neither source is set; an
///   interactive caller may run setup and then call [`save_config`].
/// - [`WorktreeError::ConfigInvalidFormat`] — the config file is not UTF-8 JSON
///   with a string `base_dir`.
/// - [`WorktreeError::BaseDirectoryInvalidEncoding`] — `WT` is not Unicode.
/// - any [`validate_base_dir`] error, then
///   [`WorktreeError::BaseDirectoryNotFound`],
///   [`WorktreeError::BaseDirectoryNotDirectory`], or
///   [`WorktreeError::BaseDirectoryUnreadable`] for the path's state.
/// - [`WorktreeError::BaseDirectoryIsGitRepo`] (`WT`) or
///   [`WorktreeError::ConfigBaseDirIsGitRepo`] (config) — a repository root.
///
/// Every base-directory error names the source the value came from.
pub fn resolve_base_dir() -> Result<PathBuf, WorktreeError> {
    resolve_with(std::env::var_os("WT"), config_path().as_deref(), &is_directory)
}

/// The real directory probe: whether the path, following symlinks, is a
/// directory, or `Err` when its state cannot be read.
fn is_directory(path: &Path) -> std::io::Result<bool> {
    std::fs::metadata(path).map(|metadata| metadata.is_dir())
}

fn resolve_with(
    wt: Option<OsString>,
    config_path: Option<&Path>,
    probe: &impl Fn(&Path) -> std::io::Result<bool>,
) -> Result<PathBuf, WorktreeError> {
    if let Some(wt) = wt.filter(|wt| !wt.is_empty()) {
        let wt = wt.into_string().map_err(|value| WorktreeError::BaseDirectoryInvalidEncoding {
            value: PathBuf::from(value),
            origin: BaseDirSource::Env,
        })?;
        return check_base_dir(&wt, BaseDirSource::Env, probe);
    }

    let Some(config_path) = config_path else {
        return Err(WorktreeError::BaseDirectoryNotConfigured);
    };
    let bytes = match std::fs::read(config_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorktreeError::BaseDirectoryNotConfigured);
        }
        Err(e) => return Err(e.into()),
    };
    let invalid_format = |message: String| WorktreeError::ConfigInvalidFormat {
        config_path: config_path.to_path_buf(),
        message,
    };
    let contents =
        String::from_utf8(bytes).map_err(|e| invalid_format(format!("the file is not UTF-8 text ({e})")))?;
    let config: WorktreeConfig = serde_json::from_str(&contents).map_err(|e| invalid_format(e.to_string()))?;

    check_base_dir(&config.base_dir, BaseDirSource::ConfigFile(config_path.to_path_buf()), probe)
}

/// Validates `raw`'s syntax, then that it names an existing directory that
/// is not a repository root.
fn check_base_dir(
    raw: &str,
    origin: BaseDirSource,
    probe: &impl Fn(&Path) -> std::io::Result<bool>,
) -> Result<PathBuf, WorktreeError> {
    let path = validate_base_dir(raw, origin.clone())?;
    let value = raw.to_owned();
    match probe(&path) {
        Ok(true) => {}
        Ok(false) => return Err(WorktreeError::BaseDirectoryNotDirectory { value, origin }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorktreeError::BaseDirectoryNotFound { value, origin });
        }
        Err(cause) => return Err(WorktreeError::BaseDirectoryUnreadable { value, origin, cause }),
    }
    if path.join(".git").exists() {
        return Err(match origin {
            BaseDirSource::ConfigFile(config_path) => WorktreeError::ConfigBaseDirIsGitRepo { config_path, dir: value },
            origin => WorktreeError::BaseDirectoryIsGitRepo { value, origin },
        });
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use test_toolkit::EnvGuard;

    /// How [`validate_base_dir`] treats a value on one host style.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Accept,
        Foreign,
        NotAbsolute,
        Empty,
    }
    use Outcome::*;

    /// The spec's input table: `(input, on POSIX, on Windows)`.
    const TABLE: &[(&str, Outcome, Outcome)] = &[
        ("/home/me/wt", Accept, Foreign),
        ("/mnt/b/wt", Accept, Foreign),
        (r"b:\wt", Foreign, Accept),
        ("B:/wt", Foreign, Accept),
        (r"\\server\share\wt", Foreign, Accept),
        ("//server/share/wt", Accept, Accept),
        (r"\\?\C:\wt", Foreign, Accept),
        (r"\\?\UNC\server\share\wt", Foreign, Accept),
        ("b:wt", Foreign, NotAbsolute),
        ("b:", Foreign, NotAbsolute),
        (r"\wt", NotAbsolute, NotAbsolute),
        ("wt", NotAbsolute, NotAbsolute),
        ("./wt", NotAbsolute, NotAbsolute),
        ("../wt", NotAbsolute, NotAbsolute),
        ("~/wt", NotAbsolute, NotAbsolute),
        ("~", NotAbsolute, NotAbsolute),
        ("", Empty, Empty),
        ("   ", Empty, Empty),
        ("\t\n", Empty, Empty),
        // A colon or backslash inside a POSIX filename is not foreign syntax.
        (r"/home/me/b:\wt", Accept, Foreign),
    ];

    fn outcome(result: &Result<PathBuf, WorktreeError>) -> Outcome {
        result.as_ref().map_or_else(error_outcome, |_| Accept)
    }

    fn error_outcome(error: &WorktreeError) -> Outcome {
        match error {
            WorktreeError::BaseDirectoryForeignPath { .. } => Foreign,
            WorktreeError::BaseDirectoryNotAbsolute { .. } => NotAbsolute,
            WorktreeError::BaseDirectoryEmpty { .. } => Empty,
            other => panic!("unexpected error: {other:?}"),
        }
    }

    fn origins() -> [BaseDirSource; 3] {
        [
            BaseDirSource::Env,
            BaseDirSource::ConfigFile(PathBuf::from("/h/.worktree.json")),
            BaseDirSource::Prompt,
        ]
    }

    fn error_origin(error: &WorktreeError) -> Option<&BaseDirSource> {
        match error {
            WorktreeError::BaseDirectoryEmpty { origin }
            | WorktreeError::BaseDirectoryForeignPath { origin, .. }
            | WorktreeError::BaseDirectoryNotAbsolute { origin, .. }
            | WorktreeError::BaseDirectoryNotDirectory { origin, .. }
            | WorktreeError::BaseDirectoryInvalidEncoding { origin, .. }
            | WorktreeError::BaseDirectoryNotFound { origin, .. }
            | WorktreeError::BaseDirectoryUnreadable { origin, .. }
            | WorktreeError::BaseDirectoryIsGitRepo { origin, .. } => Some(origin),
            _ => None,
        }
    }

    /// An absolute directory spelling for this host that need not exist.
    fn host_absolute(name: &str) -> String {
        if cfg!(windows) { format!(r"C:\Users\me\{name}") } else { format!("/home/me/{name}") }
    }

    fn foreign_absolute() -> &'static str {
        if cfg!(windows) { "/home/me/wt" } else { r"b:\wt" }
    }

    #[test]
    fn foreign_syntax_helper_classifies_both_styles_on_every_host() {
        for &(input, posix, windows) in TABLE {
            for (style, expected) in [(PathStyle::Posix, posix), (PathStyle::Windows, windows)] {
                // The helper only sees values the native check did not accept.
                if expected == Accept || expected == Empty {
                    continue;
                }
                assert_eq!(
                    is_foreign_syntax(input, style),
                    expected == Foreign,
                    "{input:?} on {style:?}"
                );
            }
        }
    }

    #[test]
    fn foreign_syntax_helper_never_flags_host_absolute_lookalikes_alone() {
        // `//server/share/wt` resembles a Windows share but is absolute on
        // POSIX, which is why the helper runs only after `is_absolute`.
        assert!(!is_foreign_syntax("//server/share/wt", PathStyle::Posix));
        assert!(!is_foreign_syntax(r"\wt", PathStyle::Posix));
        assert!(!is_foreign_syntax("1:/wt", PathStyle::Posix));
        assert!(!is_foreign_syntax(r"\wt", PathStyle::Windows));
        assert!(!is_foreign_syntax("b:wt", PathStyle::Windows));
    }

    #[test]
    fn validator_matches_the_spec_table_on_this_host() {
        for &(input, posix, windows) in TABLE {
            let expected = if cfg!(windows) { windows } else { posix };
            let result = validate_base_dir(input, BaseDirSource::Env);
            assert_eq!(outcome(&result), expected, "{input:?}: {result:?}");
            if expected == Accept {
                assert_eq!(result.unwrap(), PathBuf::from(input), "{input:?} spelling preserved");
            }
        }
    }

    #[test]
    fn validator_errors_carry_the_value_and_every_origin() {
        let cases = [("", Empty), ("  ", Empty), ("wt", NotAbsolute), (foreign_absolute(), Foreign)];
        for origin in origins() {
            for (input, expected) in cases {
                let error = validate_base_dir(input, origin.clone()).unwrap_err();
                assert_eq!(error_outcome(&error), expected, "{input:?}");
                assert_eq!(error_origin(&error), Some(&origin), "{input:?}");
                match &error {
                    WorktreeError::BaseDirectoryForeignPath { value, .. }
                    | WorktreeError::BaseDirectoryNotAbsolute { value, .. } => assert_eq!(value, input),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn validator_preserves_spaces_and_never_expands_text() {
        let trailing = host_absolute("wt ");
        assert_eq!(validate_base_dir(&trailing, BaseDirSource::Env).unwrap(), PathBuf::from(&trailing));
        let leading_space = format!(" {}", host_absolute("wt"));
        assert!(matches!(
            validate_base_dir(&leading_space, BaseDirSource::Env),
            Err(WorktreeError::BaseDirectoryNotAbsolute { value, .. }) if value == leading_space
        ));
        let literal_var = host_absolute("$USER");
        assert_eq!(validate_base_dir(&literal_var, BaseDirSource::Env).unwrap(), PathBuf::from(&literal_var));
        for unexpanded in ["~/wt", "$HOME/wt", "%USERPROFILE%\\wt"] {
            assert!(
                matches!(
                    validate_base_dir(unexpanded, BaseDirSource::Prompt),
                    Err(WorktreeError::BaseDirectoryNotAbsolute { ref value, .. }) if value == unexpanded
                ),
                "{unexpanded:?}"
            );
        }
        let quoted = format!("\"{}\"", host_absolute("wt"));
        assert!(matches!(
            validate_base_dir(&quoted, BaseDirSource::Env),
            Err(WorktreeError::BaseDirectoryNotAbsolute { value, .. }) if value == quoted
        ));
    }

    // ---- resolution through the private config-path helper ----

    struct Fixture {
        tmp: TempDir,
        config: PathBuf,
        base: PathBuf,
    }

    impl Fixture {
        /// A temp home holding an existing base directory and a config
        /// written for it by the real writer.
        fn new() -> Self {
            let tmp = TempDir::new().unwrap();
            let base = tmp.path().join("wt");
            fs::create_dir(&base).unwrap();
            let config = tmp.path().join(".worktree.json");
            save_config_to(&config, &base).unwrap();
            Self { tmp, config, base }
        }

        fn config_text(&self) -> String {
            fs::read_to_string(&self.config).unwrap()
        }

        /// The config's `base_dir` value as it appears in the JSON text.
        fn json_value(&self) -> String {
            serde_json::to_string(self.base.to_str().unwrap()).unwrap()
        }

        fn resolve(&self, wt: Option<&str>) -> Result<PathBuf, WorktreeError> {
            resolve_with(wt.map(OsString::from), Some(&self.config), &is_directory)
        }

        fn origin(&self) -> BaseDirSource {
            BaseDirSource::ConfigFile(self.config.clone())
        }
    }

    #[test]
    fn config_value_resolves_to_its_directory() {
        let fixture = Fixture::new();
        assert_eq!(fixture.resolve(None).unwrap(), fixture.base);
    }

    #[test]
    fn missing_config_file_means_not_configured() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join(".worktree.json");
        assert!(matches!(
            resolve_with(None, Some(&missing), &is_directory),
            Err(WorktreeError::BaseDirectoryNotConfigured)
        ));
        assert!(matches!(resolve_with(None, None, &is_directory), Err(WorktreeError::BaseDirectoryNotConfigured)));
    }

    #[test]
    fn config_value_that_does_not_exist_names_the_config_file() {
        let fixture = Fixture::new();
        let gone = fixture.tmp.path().join("gone");
        save_config_to(&fixture.config, &gone).unwrap();
        let error = fixture.resolve(None).unwrap_err();
        assert!(matches!(
            &error,
            WorktreeError::BaseDirectoryNotFound { value, origin }
                if value == gone.to_str().unwrap() && *origin == fixture.origin()
        ), "{error:?}");
        assert!(error.to_string().contains(&fixture.config.display().to_string()));
    }

    #[test]
    fn config_value_naming_a_file_is_not_a_directory() {
        let fixture = Fixture::new();
        let file = fixture.tmp.path().join("plain-file");
        fs::write(&file, "x").unwrap();
        save_config_to(&fixture.config, &file).unwrap();
        assert!(matches!(
            fixture.resolve(None),
            Err(WorktreeError::BaseDirectoryNotDirectory { value, origin })
                if value == file.to_str().unwrap() && origin == fixture.origin()
        ));
    }

    #[test]
    fn repository_roots_are_rejected_with_a_git_folder_or_file() {
        for git_is_file in [false, true] {
            let fixture = Fixture::new();
            let git = fixture.base.join(".git");
            if git_is_file {
                fs::write(&git, "gitdir: /elsewhere\n").unwrap();
            } else {
                fs::create_dir(&git).unwrap();
            }
            assert!(matches!(
                fixture.resolve(None),
                Err(WorktreeError::ConfigBaseDirIsGitRepo { config_path, dir })
                    if config_path == fixture.config && dir == fixture.base.to_str().unwrap()
            ), "config, .git file: {git_is_file}");
            assert!(matches!(
                fixture.resolve(fixture.base.to_str()),
                Err(WorktreeError::BaseDirectoryIsGitRepo { value, origin: BaseDirSource::Env })
                    if value == fixture.base.to_str().unwrap()
            ), "WT, .git file: {git_is_file}");
        }
    }

    #[test]
    fn a_directory_inside_a_repository_is_allowed() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.base.join(".git")).unwrap();
        let nested = fixture.base.join("nested");
        fs::create_dir(&nested).unwrap();
        assert_eq!(fixture.resolve(nested.to_str()).unwrap(), nested);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_to_a_directory_is_allowed() {
        let fixture = Fixture::new();
        let link = fixture.tmp.path().join("link");
        std::os::unix::fs::symlink(&fixture.base, &link).unwrap();
        assert_eq!(fixture.resolve(link.to_str()).unwrap(), link);
    }

    #[test]
    fn unreadable_state_keeps_its_cause_and_is_not_called_missing() {
        let fixture = Fixture::new();
        let denied = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        for (wt, expected_origin) in [(None, fixture.origin()), (fixture.base.to_str(), BaseDirSource::Env)] {
            let error = resolve_with(wt.map(OsString::from), Some(&fixture.config), &denied).unwrap_err();
            match &error {
                WorktreeError::BaseDirectoryUnreadable { value, origin, cause } => {
                    assert_eq!(value, fixture.base.to_str().unwrap());
                    assert_eq!(*origin, expected_origin);
                    assert_eq!(cause.kind(), std::io::ErrorKind::PermissionDenied);
                }
                other => panic!("expected Unreadable, got {other:?}"),
            }
            assert!(std::error::Error::source(&error).is_some());
        }
    }

    #[test]
    fn probe_results_map_to_not_found_and_not_directory() {
        let fixture = Fixture::new();
        let missing = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::NotFound));
        let file = |_: &Path| Ok(false);
        assert!(matches!(
            resolve_with(None, Some(&fixture.config), &missing),
            Err(WorktreeError::BaseDirectoryNotFound { .. })
        ));
        assert!(matches!(
            resolve_with(None, Some(&fixture.config), &file),
            Err(WorktreeError::BaseDirectoryNotDirectory { .. })
        ));
    }

    #[test]
    fn syntax_is_checked_before_any_filesystem_probe() {
        let fixture = Fixture::new();
        fs::write(&fixture.config, format!("{{\"base_dir\": {}}}", serde_json::to_string(foreign_absolute()).unwrap()))
            .unwrap();
        let probed = std::cell::Cell::new(false);
        let probe = |_: &Path| {
            probed.set(true);
            Ok(true)
        };
        assert!(matches!(
            resolve_with(None, Some(&fixture.config), &probe),
            Err(WorktreeError::BaseDirectoryForeignPath { origin, .. }) if origin == fixture.origin()
        ));
        assert!(!probed.get());
    }

    #[test]
    fn the_incident_value_from_the_config_names_value_file_and_host() {
        let fixture = Fixture::new();
        let incident = if cfg!(windows) { "/home/me/wt" } else { r"b:\wt" };
        fs::write(&fixture.config, format!("{{\n  \"base_dir\": {}\n}}", serde_json::to_string(incident).unwrap()))
            .unwrap();
        let error = fixture.resolve(None).unwrap_err();
        assert!(matches!(
            &error,
            WorktreeError::BaseDirectoryForeignPath { value, origin } if value == incident && *origin == fixture.origin()
        ), "{error:?}");
        let message = error.to_string();
        assert!(message.contains(incident), "{message}");
        assert!(message.contains(&fixture.config.display().to_string()), "{message}");
        assert!(message.contains("base_dir"), "{message}");
    }

    // ---- precedence through the public resolver and real home lookup ----

    /// Points the home lookup at `home` on every OS, and sets or removes `WT`.
    fn guard_env(home: &Path, wt: Option<&str>) -> [EnvGuard; 3] {
        // SAFETY: callers are in the `wt_env` serial group, and nextest
        // runs each test in its own process.
        unsafe {
            [
                EnvGuard::set("HOME", home),
                EnvGuard::set("USERPROFILE", home),
                match wt {
                    Some(wt) => EnvGuard::set("WT", wt),
                    None => EnvGuard::remove("WT"),
                },
            ]
        }
    }

    #[test]
    #[serial_test::serial(wt_env)]
    fn config_lookups_use_the_environment_home() {
        let fixture = Fixture::new();
        let _env = guard_env(fixture.tmp.path(), None);
        assert_eq!(config_path().unwrap(), fixture.config);
        assert_eq!(
            considered_dirs(),
            vec![
                fixture.tmp.path().join("worktrees"),
                fixture.tmp.path().join("wt"),
                fixture.tmp.path().join(".claudine").join("worktrees"),
            ]
        );
    }

    #[test]
    #[serial_test::serial(wt_env)]
    fn unset_or_empty_wt_falls_back_to_the_config() {
        let fixture = Fixture::new();
        for wt in [None, Some("")] {
            let _env = guard_env(fixture.tmp.path(), wt);
            assert_eq!(resolve_base_dir().unwrap(), fixture.base, "WT = {wt:?}");
        }
    }

    #[test]
    #[serial_test::serial(wt_env)]
    fn invalid_nonempty_wt_blocks_a_valid_config() {
        let fixture = Fixture::new();
        let missing = fixture.tmp.path().join("missing");
        for wt in ["  ", "wt", "~/wt", foreign_absolute(), missing.to_str().unwrap()] {
            let _env = guard_env(fixture.tmp.path(), Some(wt));
            let error = resolve_base_dir().unwrap_err();
            assert_eq!(error_origin(&error), Some(&BaseDirSource::Env), "WT = {wt:?}: {error:?}");
        }
    }

    #[test]
    #[serial_test::serial(wt_env)]
    fn valid_wt_wins_over_a_malformed_config() {
        let fixture = Fixture::new();
        fs::write(&fixture.config, "{ not json").unwrap();
        let other = fixture.tmp.path().join("other");
        fs::create_dir(&other).unwrap();
        let _env = guard_env(fixture.tmp.path(), other.to_str());
        assert_eq!(resolve_base_dir().unwrap(), other);
    }

    #[cfg(unix)]
    #[test]
    #[serial_test::serial(wt_env)]
    fn non_unicode_wt_is_rejected_without_fallback() {
        use std::os::unix::ffi::OsStrExt;
        let fixture = Fixture::new();
        let raw = std::ffi::OsStr::from_bytes(b"/tmp/wt-\xff");
        let _env = guard_env(fixture.tmp.path(), None);
        // SAFETY: as in `guard_env`.
        let _wt = unsafe { EnvGuard::set("WT", raw) };
        assert!(matches!(
            resolve_base_dir(),
            Err(WorktreeError::BaseDirectoryInvalidEncoding { value, origin: BaseDirSource::Env })
                if value.as_os_str() == raw
        ));
    }

    #[cfg(windows)]
    #[test]
    #[serial_test::serial(wt_env)]
    fn non_unicode_wt_is_rejected_without_fallback() {
        use std::os::windows::ffi::OsStringExt;
        let fixture = Fixture::new();
        let raw = OsString::from_wide(&[b'C' as u16, b':' as u16, b'\\' as u16, 0xD800]);
        let _env = guard_env(fixture.tmp.path(), None);
        // SAFETY: as in `guard_env`.
        let _wt = unsafe { EnvGuard::set("WT", &raw) };
        assert!(matches!(
            resolve_base_dir(),
            Err(WorktreeError::BaseDirectoryInvalidEncoding { value, origin: BaseDirSource::Env })
                if value.as_os_str() == raw
        ));
    }

    // ---- the writer ----

    #[test]
    fn invalid_save_leaves_a_missing_config_absent() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join(".worktree.json");
        for invalid in ["", "  ", "wt", "~/wt", foreign_absolute()] {
            let error = save_config_to(&config, Path::new(invalid)).unwrap_err();
            assert_eq!(error_origin(&error), Some(&BaseDirSource::ConfigFile(config.clone())), "{invalid:?}");
            assert!(!config.exists(), "{invalid:?} created a config");
        }
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0, "nothing created");
    }

    #[test]
    fn invalid_save_leaves_an_existing_config_byte_for_byte_unchanged() {
        let fixture = Fixture::new();
        let before = fs::read(&fixture.config).unwrap();
        for invalid in ["", "wt", foreign_absolute()] {
            assert!(save_config_to(&fixture.config, Path::new(invalid)).is_err(), "{invalid:?}");
            assert_eq!(fs::read(&fixture.config).unwrap(), before, "{invalid:?}");
        }
    }

    #[test]
    fn save_does_not_create_the_directory() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join(".worktree.json");
        let absent = tmp.path().join("absent");
        save_config_to(&config, &absent).unwrap();
        assert!(!absent.exists());
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_save_is_rejected_without_writing() {
        use std::os::unix::ffi::OsStrExt;
        let fixture = Fixture::new();
        let before = fs::read(&fixture.config).unwrap();
        let raw = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/wt-\xff"));
        assert!(matches!(
            save_config_to(&fixture.config, raw),
            Err(WorktreeError::BaseDirectoryInvalidEncoding { value, origin })
                if value == raw && origin == fixture.origin()
        ));
        assert_eq!(fs::read(&fixture.config).unwrap(), before);
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_save_is_rejected_without_writing() {
        use std::os::windows::ffi::OsStringExt;
        let fixture = Fixture::new();
        let before = fs::read(&fixture.config).unwrap();
        let raw = PathBuf::from(OsString::from_wide(&[b'C' as u16, b':' as u16, b'\\' as u16, 0xD800]));
        assert!(matches!(
            save_config_to(&fixture.config, &raw),
            Err(WorktreeError::BaseDirectoryInvalidEncoding { value, origin })
                if value == raw && origin == fixture.origin()
        ));
        assert_eq!(fs::read(&fixture.config).unwrap(), before);
    }

    #[test]
    fn save_and_reload_round_trips_repeatedly() {
        let fixture = Fixture::new();
        let first = fs::read(&fixture.config).unwrap();
        assert_eq!(fixture.resolve(None).unwrap(), fixture.base);
        save_config_to(&fixture.config, &fixture.resolve(None).unwrap()).unwrap();
        assert_eq!(fs::read(&fixture.config).unwrap(), first, "rewrite is stable");
        assert_eq!(fixture.resolve(None).unwrap(), fixture.base);
    }

    // ---- input robustness matrix ----

    #[test]
    fn config_base_dir_robustness_matrix() {
        let fixture = Fixture::new();
        let control = fixture.config_text();
        let value = fixture.json_value();
        let key = "\"base_dir\"";
        assert!(control.contains(key) && control.contains(&value), "fixture shape: {control}");

        type Check = fn(&Result<PathBuf, WorktreeError>, &Fixture) -> bool;
        let invalid_format: Check = |r, f| {
            matches!(r, Err(WorktreeError::ConfigInvalidFormat { config_path, .. }) if *config_path == f.config)
        };
        let empty: Check = |r, f| matches!(r, Err(WorktreeError::BaseDirectoryEmpty { origin }) if *origin == f.origin());
        let not_absolute: Check =
            |r, f| matches!(r, Err(WorktreeError::BaseDirectoryNotAbsolute { origin, .. }) if *origin == f.origin());
        let foreign: Check =
            |r, f| matches!(r, Err(WorktreeError::BaseDirectoryForeignPath { origin, .. }) if *origin == f.origin());
        let json = |text: &str| serde_json::to_string(text).unwrap();
        let with_value = |replacement: &str| control.replace(&value, replacement);

        let duplicate = control.replacen(key, &format!("{key}: {value},\n  {key}"), 1);
        let cells: Vec<(&str, Vec<u8>, Check)> = vec![
            ("absent", control.replace(key, "\"other\"").into_bytes(), invalid_format),
            ("explicit null", with_value("null").into_bytes(), invalid_format),
            ("wrong type: number", with_value("123").into_bytes(), invalid_format),
            ("wrong type: object", with_value("{}").into_bytes(), invalid_format),
            ("wrong type: array", with_value("[]").into_bytes(), invalid_format),
            ("empty", with_value("\"\"").into_bytes(), empty),
            ("whitespace-only", with_value("\"  \"").into_bytes(), empty),
            // serde's derived struct rejects a repeated field, so a duplicate
            // never silently picks one of two values.
            ("duplicate key", duplicate.into_bytes(), invalid_format),
            ("trailing content", format!("{control} garbage").into_bytes(), invalid_format),
            ("non-UTF-8 bytes", {
                let mut bytes = with_value("\"/wt-X\"").into_bytes();
                let at = bytes.iter().position(|&b| b == b'X').unwrap();
                bytes[at] = 0xff;
                bytes
            }, invalid_format),
            ("relative", with_value(&json("wt")).into_bytes(), not_absolute),
            ("dot-relative", with_value(&json("./wt")).into_bytes(), not_absolute),
            ("home-relative", with_value(&json("~/wt")).into_bytes(), not_absolute),
            ("foreign", with_value(&json(foreign_absolute())).into_bytes(), foreign),
        ];

        assert_eq!(fixture.resolve(None).unwrap(), fixture.base, "control");
        for (name, bytes, check) in cells {
            fs::write(&fixture.config, &bytes).unwrap();
            let result = fixture.resolve(None);
            assert!(check(&result, &fixture), "{name}: {result:?}");
        }
        fs::write(&fixture.config, &control).unwrap();
        assert_eq!(fixture.resolve(None).unwrap(), fixture.base, "control restored");
    }

    #[test]
    fn wt_robustness_matrix() {
        let fixture = Fixture::new();
        let override_dir = fixture.tmp.path().join("override");
        fs::create_dir(&override_dir).unwrap();
        let env_error = |result: Result<PathBuf, WorktreeError>| {
            let error = result.unwrap_err();
            assert_eq!(error_origin(&error), Some(&BaseDirSource::Env), "{error:?}");
            error
        };

        assert_eq!(fixture.resolve(override_dir.to_str()).unwrap(), override_dir, "control");
        assert_eq!(fixture.resolve(None).unwrap(), fixture.base, "absent falls back");
        assert_eq!(fixture.resolve(Some("")).unwrap(), fixture.base, "empty falls back");
        assert!(matches!(env_error(fixture.resolve(Some("  "))), WorktreeError::BaseDirectoryEmpty { .. }));
        for relative in ["wt", "./wt", "~/wt"] {
            assert!(
                matches!(env_error(fixture.resolve(Some(relative))), WorktreeError::BaseDirectoryNotAbsolute { .. }),
                "{relative:?}"
            );
        }
        assert!(matches!(
            env_error(fixture.resolve(Some(foreign_absolute()))),
            WorktreeError::BaseDirectoryForeignPath { .. }
        ));
        #[cfg(unix)]
        let non_unicode = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(b"/tmp/wt-\xff".to_vec())
        };
        #[cfg(windows)]
        let non_unicode = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[b'C' as u16, b':' as u16, b'\\' as u16, 0xD800])
        };
        assert!(matches!(
            env_error(resolve_with(Some(non_unicode), Some(&fixture.config), &is_directory)),
            WorktreeError::BaseDirectoryInvalidEncoding { .. }
        ));
    }

    #[test]
    fn wsl_drive_hint_suggests_the_default_mount_for_drive_absolute_values() {
        for runtime in [RuntimeEnvironment::Wsl1, RuntimeEnvironment::Wsl2] {
            for (value, mounted) in [
                (r"b:\wt", "/mnt/b/wt"),
                (r"B:\wt", "/mnt/b/wt"),
                ("B:/wt", "/mnt/b/wt"),
                (r"C:\Users\me\wt\", "/mnt/c/Users/me/wt"),
                (r"D:\", "/mnt/d"),
            ] {
                let hint = wsl_drive_hint(value, runtime)
                    .unwrap_or_else(|| panic!("{value} under {runtime} got no hint"));
                assert!(hint.contains(&format!("use {mounted}.")), "{value}: {hint}");
                assert!(hint.contains("If it is mounted there"), "{value}: {hint}");
            }
        }
    }

    #[test]
    fn wsl_drive_hint_is_never_offered_natively_or_for_other_spellings() {
        for value in [r"b:\wt", "B:/wt"] {
            assert_eq!(wsl_drive_hint(value, RuntimeEnvironment::Native), None, "{value}");
        }
        for value in [
            "b:wt",
            "b:",
            r"\\server\share\wt",
            "//server/share/wt",
            r"\\?\C:\wt",
            r"\\?\UNC\server\share\wt",
            "/home/me/wt",
            "wt",
            "",
        ] {
            assert_eq!(wsl_drive_hint(value, RuntimeEnvironment::Wsl2), None, "{value}");
        }
    }
}
