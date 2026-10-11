//! Git fixture helpers shared by the library's unit tests.
//!
//! Each `git` spawn costs far more than the file write it replaces (most on
//! Windows CI), so fixtures write repository settings straight into the
//! config file instead of running `git config` once per key.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Appends `entries` (`section[.subsection].name`, value) to the config file
/// of the repository whose working tree is `repo`, with the same effect as
/// one `git config <key> <value>` per entry.
///
/// ## Panics
///
/// When `repo/.git` is not a directory (a linked worktree's `.git` is a file;
/// its settings live in the main repository's config).
pub(crate) fn configure(repo: &Path, entries: &[(&str, &str)]) {
    append_config(&repo.join(".git").join("config"), entries);
}

/// [`configure`] for a config file named directly, such as a bare
/// repository's `config`.
pub(crate) fn append_config(config: &Path, entries: &[(&str, &str)]) {
    assert!(config.is_file(), "no git config at {config:?}");
    let mut text = String::new();
    let mut current_header = String::new();
    for (key, value) in entries {
        let (header, name) = header_and_name(key);
        if header != current_header {
            text.push_str(&header);
            text.push('\n');
            current_header = header;
        }
        text.push_str(&format!("\t{name} = {}\n", quoted(value)));
    }
    let mut file = OpenOptions::new().append(true).open(config).expect("open git config");
    file.write_all(text.as_bytes()).expect("append git config");
}

/// `[section]` or `[section "subsection"]`, and the variable name.
fn header_and_name(key: &str) -> (String, &str) {
    let (section, rest) = key.split_once('.').unwrap_or_else(|| panic!("config key {key:?} has no section"));
    match rest.rsplit_once('.') {
        Some((subsection, name)) => (format!("[{section} {}]", quoted(subsection)), name),
        None => (format!("[{section}]"), rest),
    }
}

/// A double-quoted config value: Windows paths carry backslashes, which a
/// config file reads as escapes.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn get(repo: &Path, key: &str) -> String {
        let output = Command::new("git").current_dir(repo).args(["config", "--get", key]).output().expect("git runs");
        assert!(output.status.success(), "{key}: {output:?}");
        String::from_utf8_lossy(&output.stdout).trim_end_matches(['\r', '\n']).to_string()
    }

    #[test]
    fn written_settings_read_back_as_git_config_would_set_them() {
        let dir = tempfile::tempdir().unwrap();
        let status = Command::new("git").current_dir(dir.path()).args(["init", "-q"]).status().unwrap();
        assert!(status.success());
        let path = r#"C:\Users\a "b"\origin.git"#;

        configure(
            dir.path(),
            &[("user.name", "Test User"), ("user.email", "test@example.com"), ("core.commitGraph", "false"), ("remote.origin.url", path)],
        );

        assert_eq!(get(dir.path(), "user.name"), "Test User");
        assert_eq!(get(dir.path(), "user.email"), "test@example.com");
        assert_eq!(get(dir.path(), "core.commitgraph"), "false");
        assert_eq!(get(dir.path(), "core.bare"), "false", "init's own settings are kept");
        assert_eq!(get(dir.path(), "remote.origin.url"), path);
    }
}
