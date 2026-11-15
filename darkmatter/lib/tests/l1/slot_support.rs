//! Shared fixtures for schema-slot tests: the configuration whose
//! `permissions.file-read` decides which reads a slot may make without
//! asking, written into a temp home (never the developer's), and `git` with
//! a fixture identity.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};

/// A `permissions` block pre-approving no read.
pub(crate) const NO_READS: &str = "permissions:\n    file-read: []\n";

/// The `.darkmatter.yaml` text whose `permissions.file-read` lists `entries`.
pub(crate) fn file_read_config(entries: &[&str]) -> String {
    if entries.is_empty() {
        return NO_READS.to_string();
    }
    let list: String = entries.iter().map(|entry| format!("        - \"{entry}\"\n")).collect();
    format!("permissions:\n    file-read:\n{list}")
}

/// Writes `home/.darkmatter.yaml` pre-approving exactly `entries`.
pub(crate) fn write_user_permissions(home: &Path, entries: &[&str]) {
    std::fs::create_dir_all(home).expect("home");
    std::fs::write(home.join(".darkmatter.yaml"), file_read_config(entries)).expect("user configuration");
}

/// The user configuration of `home` (no repository), captured after
/// writing `entries` as its `permissions.file-read`.
pub(crate) fn user_permissions(home: &Path, entries: &[&str]) -> Arc<ConfigSnapshot> {
    write_user_permissions(home, entries);
    let context = biscuit_file::FileResolutionContext::from_snapshot(home, Some(home.to_path_buf()), HashMap::new());
    Arc::new(ConfigSnapshot::load(&context, None, ConfigLoadMode::Passive).expect("the configuration loads"))
}

/// Runs Git in `root` with a fixture identity and no host configuration.
pub(crate) fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed in {}", root.display());
}
