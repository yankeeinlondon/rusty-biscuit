//! A request's shell children start from the environment it supplies.
//!
//! A host composing several documents in one process (Claudine's sequence
//! steps and group legs) supplies each document's environment through
//! `ComposeOptions::with_shell_child_environment`; nothing here mutates this
//! process's environment.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellChildEnvironment,
    ShellExpansionOptions,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::Markdown;
use tempfile::TempDir;

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn compose(dir: &Path, content: &str, environment: Option<ShellChildEnvironment>) -> String {
    let path: PathBuf = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let mut options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_shell(ShellExpansionOptions {
            policy_root: Some(dir.to_path_buf()),
            ..Default::default()
        })
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(Arc::new(AllowAll));
    if let Some(environment) = environment {
        options = options.with_shell_child_environment(environment);
    }
    let (composed, _) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .unwrap_or_else(|error| panic!("{content:?} failed: {error}"));
    composed.content().trim_end().to_string()
}

#[cfg(unix)]
fn document_b() -> ShellChildEnvironment {
    ShellChildEnvironment::new([("PATH", "/usr/bin:/bin"), ("DM_CHILD_MARK", "from-b")])
}

#[cfg(unix)]
#[test]
fn a_body_directive_and_a_frontmatter_command_read_the_supplied_environment() {
    let dir = TempDir::new().unwrap();
    let body = compose(
        dir.path(),
        "---\nmark: $(printenv DM_CHILD_MARK)\n---\nfm={{ mark }}\n\n::shell printenv DM_CHILD_MARK\n",
        Some(document_b()),
    );
    assert!(body.contains("fm=from-b"), "{body}");
    assert!(body.ends_with("\nfrom-b"), "{body}");
}

#[cfg(unix)]
#[test]
fn the_supplied_environment_replaces_the_inherited_one_and_pwd_names_the_working_directory() {
    let dir = TempDir::new().unwrap();
    let body = compose(dir.path(), "::shell env\n", Some(document_b()));
    // Compose joins the command's output lines; no supplied value has a space.
    let mut entries: Vec<&str> = body.split_whitespace().collect();
    entries.sort_unstable();
    let pwd = format!("PWD={}", dir.path().display());
    // `NO_COLOR` is the default ANSI stripping, laid over whatever is supplied.
    assert_eq!(
        entries,
        ["DM_CHILD_MARK=from-b", "NO_COLOR=1", "PATH=/usr/bin:/bin", pwd.as_str()],
        "{body}"
    );
}

/// This process's environment without `USERNAME`, with `bin` prepended to
/// its `Path` and a document mark added.
///
/// Windows spells `PATH` as `Path`, so finding `probe` on `bin` proves the
/// lookup reads the supplied map case-insensitively; the batch file runs
/// under `cmd.exe`, which needs the `SystemRoot` a captured environment
/// carries.
#[cfg(windows)]
fn document_without_username(bin: &Path) -> ShellChildEnvironment {
    ShellChildEnvironment::new(
        std::env::vars_os()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("USERNAME"))
            .map(|(name, value)| {
                if name.eq_ignore_ascii_case("PATH") {
                    let mut path = bin.as_os_str().to_os_string();
                    path.push(";");
                    path.push(value);
                    (name, path)
                } else {
                    (name, value)
                }
            })
            .chain([("DM_CHILD_MARK".into(), "from-b".into())]),
    )
}

#[cfg(windows)]
#[test]
fn a_batch_child_is_found_on_and_reads_the_supplied_environment() {
    let dir = TempDir::new().unwrap();
    let bin = TempDir::new().unwrap();
    std::fs::write(
        bin.path().join("probe.cmd"),
        "@echo mark=%DM_CHILD_MARK% user=%USERNAME%\r\n",
    )
    .unwrap();
    let body = compose(
        dir.path(),
        "::shell probe\n",
        Some(document_without_username(bin.path())),
    );
    // In a batch file an undefined `%NAME%` expands to nothing, so an empty
    // `user=` shows the inherited `USERNAME` was not passed through.
    assert_eq!(body, "mark=from-b user=");
}
