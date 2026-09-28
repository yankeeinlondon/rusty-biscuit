//! Per-user choice to skip provider APIs for chosen repositories: `~/.wt.json`,
//! written by `wt --ignore-api`.
//!
//! A listed repository is checked with `git ls-remote` only, and no provider
//! request of any kind is made for it. The file is separate from
//! `~/.worktree.json` ([`crate::config`]), which holds the base directory.
//!
//! Entries are [`RepoIdentity`] values, never URLs, so no user name, password,
//! or token from `origin` is ever persisted.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sniff::filesystem::git::GitHostingProvider;

use crate::cache::{atomic_write, try_lock_sidecar};
use crate::error::WorktreeError;

pub const PREFERENCE_FORMAT_VERSION: u32 = 1;

/// How long [`add`] waits for another `wt` writing the file.
const LOCK_WAIT: Duration = Duration::from_secs(2);

/// One repository as a provider API sees it.
///
/// `port` is the port the provider's API answers on, not the Git transport's:
/// SSH to a known provider host on the standard SSH port (none spelled, or 22)
/// is 443, so its SSH and HTTPS remotes share one identity. Any other remote,
/// including SSH to a known provider on an explicit nonstandard port, keeps
/// its explicit port or its scheme's default, so two ports on one host stay
/// distinct.
///
/// There is no scheme field: one host and port is one server, so an SSH and an
/// HTTPS remote that spell the same explicit port already name the same one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIdentity {
    /// Lowercase, with no user information.
    pub host: String,
    pub port: u16,
    /// `owner/repo`, case kept, with no `.git` suffix or surrounding slashes.
    pub path: String,
}

impl RepoIdentity {
    /// The identity of `origin`'s fetch URL, or `None` when it names no
    /// host and repository (a local path, for example).
    ///
    /// Only an SSH remote of a known provider on the standard SSH port maps
    /// to 443; an explicit nonstandard SSH port is kept (see [`RepoIdentity`]).
    pub fn from_origin(url: &str) -> Option<Self> {
        let identity = sniff::filesystem::git::remote_identity(url)?;
        let scheme = identity.scheme.to_ascii_lowercase();
        let port = match scheme.as_str() {
            "https" => identity.port.unwrap_or(443),
            "ssh" if matches!(identity.port, None | Some(22)) && is_known_provider(url) => 443,
            "ssh" => identity.port.unwrap_or(22),
            "http" => identity.port.unwrap_or(80),
            "git" => identity.port.unwrap_or(9418),
            _ => return None,
        };
        Some(Self { host: identity.host, port, path: identity.path })
    }
}

fn is_known_provider(url: &str) -> bool {
    matches!(
        GitHostingProvider::from_url(url),
        GitHostingProvider::GitHub
            | GitHostingProvider::GitLab
            | GitHostingProvider::Bitbucket
            | GitHostingProvider::Gitea
            | GitHostingProvider::Forgejo
    )
}

/// The parsed file. A missing file is an empty one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub ignore_api: Vec<RepoIdentity>,
}

#[derive(Serialize, Deserialize)]
struct StoredPreferences {
    format_version: u32,
    #[serde(default)]
    ignore_api: Vec<RepoIdentity>,
}

impl Preferences {
    pub fn is_ignored(&self, identity: &RepoIdentity) -> bool {
        self.ignore_api.contains(identity)
    }

    /// Whether the repository `origin` (a URL) names is ignored; an origin
    /// without an identity never is.
    pub fn ignores_origin(&self, origin: &str) -> bool {
        RepoIdentity::from_origin(origin).is_some_and(|identity| self.is_ignored(&identity))
    }
}

/// `~/.wt.json`, through the same home-directory lookup as `~/.worktree.json`
/// (`%USERPROFILE%` on native Windows).
pub fn preference_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".wt.json"))
}

/// The preferences at `path` for reading: a missing, unreadable, or corrupt
/// file (including another format version) is empty.
pub fn load(path: &Path) -> Preferences {
    read(path).unwrap_or_default()
}

/// Records `identity` at `path`, keeping every existing entry.
///
/// ## Errors
///
/// [`WorktreeError::PreferenceUnwritable`] when the file cannot be read or
/// parsed (it is never replaced then, since that would discard the entries
/// it may hold), when another writer keeps the lock beyond two seconds, or
/// when the write fails.
pub fn add(path: &Path, identity: &RepoIdentity) -> Result<(), WorktreeError> {
    let unwritable = |reason: String| WorktreeError::PreferenceUnwritable { path: path.to_path_buf(), reason };
    let _lock = lock(&lock_path(path)).map_err(unwritable)?;
    let mut preferences = read(path).map_err(unwritable)?;
    if preferences.is_ignored(identity) {
        return Ok(());
    }
    preferences.ignore_api.push(identity.clone());
    let stored = StoredPreferences {
        format_version: PREFERENCE_FORMAT_VERSION,
        ignore_api: preferences.ignore_api,
    };
    let mut bytes = serde_json::to_vec_pretty(&stored)?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).map_err(|error| unwritable(error.to_string()))
}

/// `Ok(empty)` for a missing file; `Err` for any file that is present but
/// not a valid format-1 document.
fn read(path: &Path) -> Result<Preferences, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Preferences::default()),
        Err(error) => return Err(format!("cannot read it: {error}")),
    };
    let stored: StoredPreferences =
        serde_json::from_slice(&bytes).map_err(|error| format!("it is not valid: {error}"))?;
    if stored.format_version != PREFERENCE_FORMAT_VERSION {
        return Err(format!("it has format version {}, not {PREFERENCE_FORMAT_VERSION}", stored.format_version));
    }
    Ok(Preferences { ignore_api: stored.ignore_api })
}

/// The persistent sidecar serializing writers; never deleted (see
/// [`try_lock_sidecar`]).
fn lock_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    path.with_file_name(name)
}

fn lock(path: &Path) -> Result<fs::File, String> {
    let started = Instant::now();
    loop {
        match try_lock_sidecar(path) {
            Ok(Some(file)) => return Ok(file),
            Ok(None) if started.elapsed() < LOCK_WAIT => std::thread::sleep(Duration::from_millis(25)),
            Ok(None) => return Err("another wt kept it locked".to_string()),
            Err(error) => return Err(format!("cannot lock it: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(host: &str, port: u16, path: &str) -> RepoIdentity {
        RepoIdentity { host: host.to_string(), port, path: path.to_string() }
    }

    fn store() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(".wt.json");
        (dir, path)
    }

    #[test]
    fn https_and_ssh_remotes_of_one_provider_repository_share_an_identity() {
        let expected = Some(identity("github.com", 443, "Owner/Repo"));
        for url in [
            "https://github.com/Owner/Repo.git",
            "https://github.com/Owner/Repo",
            "https://GitHub.com/Owner/Repo/",
            "git@github.com:Owner/Repo.git",
            "ssh://git@github.com/Owner/Repo.git",
            "ssh://git@github.com:22/Owner/Repo.git",
        ] {
            assert_eq!(RepoIdentity::from_origin(url), expected, "{url}");
        }
    }

    #[test]
    fn user_information_never_reaches_the_identity_or_the_file() {
        let (_dir, path) = store();
        let url = "https://alice:s3cr3t-token@github.com/owner/repo.git";
        let identity = RepoIdentity::from_origin(url).expect("identifiable");
        assert_eq!(identity, self::identity("github.com", 443, "owner/repo"));
        add(&path, &identity).expect("add");
        let written = fs::read_to_string(&path).expect("written");
        assert!(!written.contains("alice") && !written.contains("s3cr3t"), "{written}");
        assert!(!written.contains("https://"), "no URL is stored: {written}");
    }

    #[test]
    fn distinct_ports_and_paths_stay_distinct() {
        let default_port = RepoIdentity::from_origin("https://git.example.invalid/o/r.git").unwrap();
        let other_port = RepoIdentity::from_origin("https://git.example.invalid:8443/o/r.git").unwrap();
        let other_path = RepoIdentity::from_origin("https://git.example.invalid/o/other.git").unwrap();
        let plain_http = RepoIdentity::from_origin("http://git.example.invalid/o/r.git").unwrap();
        assert_eq!(default_port.port, 443);
        assert_eq!(other_port.port, 8443);
        assert_eq!(plain_http.port, 80);
        assert_ne!(default_port, other_port);
        assert_ne!(default_port, other_path);
        assert_ne!(default_port, plain_http);

        let (_dir, path) = store();
        add(&path, &default_port).unwrap();
        let preferences = load(&path);
        assert!(preferences.is_ignored(&default_port));
        assert!(!preferences.is_ignored(&other_port));
        assert!(!preferences.is_ignored(&other_path));
    }

    #[test]
    fn a_known_provider_on_a_nonstandard_ssh_port_keeps_that_port() {
        let https = RepoIdentity::from_origin("https://github.com/Owner/Repo.git").unwrap();
        let port_2222 = RepoIdentity::from_origin("ssh://git@github.com:2222/Owner/Repo.git").unwrap();
        let port_2200 = RepoIdentity::from_origin("ssh://git@github.com:2200/Owner/Repo.git").unwrap();
        assert_eq!(port_2222, identity("github.com", 2222, "Owner/Repo"));
        assert_eq!(port_2200.port, 2200);
        assert_ne!(port_2222, https);
        assert_ne!(port_2200, https);
        assert_ne!(port_2222, port_2200);
    }

    #[test]
    fn an_ignored_nonstandard_ssh_port_survives_a_round_trip_without_ignoring_other_ports() {
        let (_dir, path) = store();
        let recorded = "ssh://git@github.com:2222/Owner/Repo.git";
        add(&path, &RepoIdentity::from_origin(recorded).unwrap()).unwrap();
        let preferences = load(&path);

        assert!(preferences.ignores_origin(recorded));
        for origin in [
            "ssh://git@github.com:2200/Owner/Repo.git",
            "ssh://git@github.com:22/Owner/Repo.git",
            "git@github.com:Owner/Repo.git",
            "https://github.com/Owner/Repo.git",
        ] {
            assert!(!preferences.ignores_origin(origin), "{origin}");
        }
    }

    #[test]
    fn ssh_to_an_unknown_host_keeps_its_own_port() {
        let ssh = RepoIdentity::from_origin("ssh://git@git.example.invalid:2222/o/r.git").unwrap();
        let https = RepoIdentity::from_origin("https://git.example.invalid/o/r.git").unwrap();
        assert_eq!(ssh.port, 2222);
        assert_ne!(ssh, https);
        assert_eq!(RepoIdentity::from_origin("git@git.example.invalid:o/r.git").unwrap().port, 22);
    }

    #[test]
    fn a_local_path_has_no_identity() {
        for url in ["/srv/git/r.git", "file:///srv/git/r.git", "../r.git", ""] {
            assert_eq!(RepoIdentity::from_origin(url), None, "{url}");
        }
    }

    #[test]
    fn a_missing_file_ignores_nothing() {
        let (_dir, path) = store();
        assert_eq!(load(&path), Preferences::default());
    }

    #[test]
    fn add_round_trips_and_is_idempotent() {
        let (_dir, path) = store();
        let first = identity("github.com", 443, "o/a");
        let second = identity("gitlab.com", 443, "g/b");
        add(&path, &first).unwrap();
        add(&path, &second).unwrap();
        let after_two = fs::read(&path).unwrap();
        add(&path, &first).unwrap();
        assert_eq!(fs::read(&path).unwrap(), after_two, "a repeated entry is not written again");
        assert_eq!(load(&path).ignore_api, vec![first, second]);

        let parsed: serde_json::Value = serde_json::from_slice(&after_two).unwrap();
        assert_eq!(parsed["format_version"], 1);
    }

    #[test]
    fn an_added_repository_is_ignored_by_any_of_its_origin_spellings() {
        let (_dir, path) = store();
        let https = "https://github.com/Owner/Repo.git";
        add(&path, &RepoIdentity::from_origin(https).unwrap()).unwrap();
        let preferences = load(&path);

        for origin in [https, "git@github.com:Owner/Repo.git", "https://token@github.com/Owner/Repo"] {
            assert!(preferences.ignores_origin(origin), "{origin}");
        }
        for origin in ["https://github.com/Owner/Other.git", "https://github.com:8443/Owner/Repo.git", "/srv/git/repo.git"] {
            assert!(!preferences.ignores_origin(origin), "{origin}");
        }
    }

    #[test]
    fn a_corrupt_or_other_format_file_reads_as_empty_and_add_refuses_to_replace_it() {
        let wanted = identity("github.com", 443, "o/r");
        for contents in [
            "{ not json",
            r#"{"format_version": 2, "ignore_api": []}"#,
            r#"{"ignore_api": [{"host": "github.com", "port": 443, "path": "o/r"}]}"#,
            r#"{"format_version": 1, "ignore_api": [{"host": "github.com"}]}"#,
        ] {
            let (_dir, path) = store();
            fs::write(&path, contents).unwrap();
            assert_eq!(load(&path), Preferences::default(), "{contents}");
            let error = add(&path, &wanted).expect_err("refuses a file it cannot read");
            assert!(matches!(error, WorktreeError::PreferenceUnwritable { .. }), "{error:?}");
            assert_eq!(fs::read_to_string(&path).unwrap(), contents, "left untouched");
        }
    }

    #[test]
    fn an_unreadable_file_reads_as_empty_and_add_refuses_it() {
        // A directory in the file's place cannot be read on every OS.
        let (_dir, path) = store();
        fs::create_dir(&path).unwrap();
        assert_eq!(load(&path), Preferences::default());
        let error = add(&path, &identity("github.com", 443, "o/r")).expect_err("refuses");
        assert!(matches!(error, WorktreeError::PreferenceUnwritable { .. }), "{error:?}");
        assert!(path.is_dir(), "left untouched");
    }

    #[test]
    fn a_held_lock_times_out_as_a_write_error() {
        let (_dir, path) = store();
        let _held = try_lock_sidecar(&lock_path(&path)).unwrap().expect("free");
        let started = Instant::now();
        let error = add(&path, &identity("github.com", 443, "o/r")).expect_err("times out");
        assert!(matches!(error, WorktreeError::PreferenceUnwritable { .. }), "{error:?}");
        assert!(started.elapsed() >= LOCK_WAIT);
        assert!(!path.exists());
    }

    #[test]
    fn concurrent_adds_keep_every_entry() {
        let (_dir, path) = store();
        let identities: Vec<_> = (0..8).map(|n| identity("github.com", 443, &format!("o/r{n}"))).collect();
        std::thread::scope(|scope| {
            for identity in &identities {
                let path = &path;
                scope.spawn(move || add(path, identity).expect("add"));
            }
        });
        let stored = load(&path);
        for identity in &identities {
            assert!(stored.is_ignored(identity), "{identity:?} was lost");
        }
        assert_eq!(stored.ignore_api.len(), identities.len());
    }

    #[test]
    fn the_file_sits_in_the_home_directory() {
        let home = dirs::home_dir().expect("home");
        assert_eq!(preference_path(), Some(home.join(".wt.json")));
    }

    #[cfg(windows)]
    #[test]
    fn the_file_resolves_under_userprofile_on_windows() {
        let profile = PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE is set"));
        assert_eq!(preference_path(), Some(profile.join(".wt.json")));
    }
}
