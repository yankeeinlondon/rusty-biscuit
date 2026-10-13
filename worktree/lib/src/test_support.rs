//! Git fixture helpers shared by the library's unit tests.
//!
//! Each `git` spawn costs far more than the file write it replaces (most on
//! Windows CI), so fixtures write repository settings straight into the
//! config file instead of running `git config` once per key.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Runs one `git fast-import` over `stream` in the repository at `dir` (a
/// bare repository or a working tree) and returns what it printed, which is
/// the answer to each `get-mark`.
pub(crate) fn fast_import(dir: &Path, stream: &str) -> String {
    let mut child = crate::git::command()
        .current_dir(dir)
        .args(["fast-import", "--quiet", "--done"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git should be installed");
    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(stream.as_bytes()).expect("write to git fast-import");
    stdin.write_all(b"done\n").expect("finish git fast-import");
    drop(stdin);
    let output = child.wait_with_output().expect("git fast-import");
    assert!(output.status.success(), "git fast-import in {dir:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("utf-8")
}

/// Commits a new file named `file` (holding `file` and a newline, message
/// `add <file>`) onto `branch` in the bare repository `origin`, as another
/// person's push would, and returns the commit. A branch `origin` does not
/// have starts from its `main`.
///
/// One `git fast-import`, where a clone, a commit, and a push start about a
/// dozen git processes.
pub(crate) fn commit_to_origin(origin: &Path, branch: &str, file: &str) -> String {
    let base = if has_branch(origin, branch) { branch } else { "main" };
    let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after 1970").as_secs();
    let message = format!("add {file}");
    let contents = format!("{file}\n");
    // `^0`: fast-import does not let a branch start from itself.
    let stream = format!(
        "commit refs/heads/{branch}\nmark :1\ncommitter Test User <test@example.com> {time} +0000\n\
         data {}\n{message}\nfrom refs/heads/{base}^0\nM 100644 inline {file}\ndata {}\n{contents}\nget-mark :1\n",
        message.len(),
        contents.len()
    );
    let sha = fast_import(origin, &stream).trim().to_string();
    assert_eq!(sha.len(), 40, "git fast-import gave {sha:?}");
    sha
}

/// Whether the bare repository `origin` has `branch`, loose or packed.
fn has_branch(origin: &Path, branch: &str) -> bool {
    let refname = format!("refs/heads/{branch}");
    origin.join(&refname).is_file()
        || std::fs::read_to_string(origin.join("packed-refs"))
            .is_ok_and(|packed| packed.lines().any(|line| line.split_once(' ').is_some_and(|(_, name)| name == refname)))
}

/// This package's directory, read at run time so an archived test run on
/// another host resolves its own copy rather than the producer's path. Only
/// the FSEvents and ReadDirectoryChangesW tests call it.
#[cfg(any(target_os = "macos", windows))]
pub(crate) fn package_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(repo: &Path, key: &str) -> String {
        let output = crate::git::command().current_dir(repo).args(["config", "--get", key]).output().expect("git runs");
        assert!(output.status.success(), "{key}: {output:?}");
        String::from_utf8_lossy(&output.stdout).trim_end_matches(['\r', '\n']).to_string()
    }

    #[test]
    fn written_settings_read_back_as_git_config_would_set_them() {
        let dir = tempfile::tempdir().unwrap();
        let status = crate::git::command().current_dir(dir.path()).args(["init", "-q"]).status().unwrap();
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

    #[test]
    fn a_commit_to_origin_extends_the_branch_or_starts_one_from_main() {
        let repo = crate::remove::test_support::TestRepo::with_origin();
        let origin = repo.origin_path();
        let main_before = repo.sha("main");

        let on_main = commit_to_origin(&origin, "main", "upstream.txt");
        let on_feature = commit_to_origin(&origin, "feature", "feature.txt");

        let git = |args: &[&str]| repo.git_in(&origin, args);
        assert_eq!(git(&["rev-parse", "main", "main^", "feature", "feature^"]), format!("{on_main}\n{main_before}\n{on_feature}\n{on_main}"));
        assert_eq!(git(&["show", "main:upstream.txt"]), "upstream.txt");
        assert_eq!(git(&["log", "-1", "--format=%s %an <%ae>", "feature"]), "add feature.txt Test User <test@example.com>");
    }
}
