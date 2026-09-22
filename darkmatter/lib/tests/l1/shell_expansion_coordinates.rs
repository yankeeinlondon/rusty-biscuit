//! File-relative line coordinates for shell-expansion errors.
//!
//! Phase 3 of the composition shell-error diagnostics feature: body
//! `::shell`, `::shell-block`, and frontmatter `$(...)` origins must be
//! reported in the file's 1-based coordinate space, not the body-relative
//! space.

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    ComposeOptions,
    shell_expansion::types::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest},
};
use std::sync::Arc;
use tempfile::TempDir;

struct AllowAllHandler;

impl ShellApprovalHandler for AllowAllHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn compose_options(dir: &TempDir) -> ComposeOptions {
    ComposeOptions::new()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler))
}

/// A portable failing command that prints to stderr and exits non-zero on
/// macOS, Windows, and Linux.
fn failing_command() -> String {
    let executable = std::env::current_exe().expect("test executable path should be available");
    let executable = biscuit_file::to_portable_string(&executable);
    format!("\"{executable}\" --definitely-invalid-libtest-option")
}

#[test]
fn body_shell_origin_is_file_relative_with_frontmatter() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\ntitle: Test\ncmd: placeholder\n---\n# Heading\n\n::shell {}\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing ::shell should error");

    let msg = err.to_string();
    assert!(
        msg.contains("line 7") || msg.contains("ShellExpansionError"),
        "expected file-relative line in error; got: {msg}"
    );

    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));
    assert!(
        rendered.contains("> 7 │"),
        "excerpt should highlight file line 7; got: {rendered}"
    );
}

#[test]
fn shell_block_origin_is_file_relative_with_frontmatter() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\ntitle: Test\n---\n# Heading\n\n::shell-block\n{}\n::end-block\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing ::shell-block should error");

    // Wrapper's `command_line` field surfaces in its `Display` ("Shell block
    // command failed at line 7"). The inner `ShellExpansionError` does not
    // produce this string, so the assertion uniquely pins the wrapper.
    let msg = err.to_string();
    assert!(
        msg.contains("Shell block command failed at line 7"),
        "expected wrapper Display to report file-relative command line 7; got: {msg}"
    );

    // The wrapper's `status_block` emits its own labels — `Command at line`
    // and `Block opened at line` — which the inner error never produces, so
    // their presence proves the wrapper (not just the inner error) is
    // file-relative. Fixture: block opens at file line 6, command at line 7.
    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));
    assert!(
        rendered.contains("Command at line: 7"),
        "wrapper status_block should report file-relative command line 7; got: {rendered}"
    );
    assert!(
        rendered.contains("Block opened at line: 6"),
        "wrapper status_block should report file-relative block open line 6; got: {rendered}"
    );
}

#[test]
fn shell_block_origin_counts_lines_not_bytes_with_crlf() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\r\ntitle: Test\r\n---\r\n# Heading\r\n\r\n::shell-block\r\n{}\r\n::end-block\r\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing ::shell-block should error");

    let msg = err.to_string();
    assert!(
        msg.contains("Shell block command failed at line 7"),
        "CRLF fixture should report file-relative command line 7 in wrapper Display; got: {msg}"
    );

    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));
    assert!(
        rendered.contains("Block opened at line: 6"),
        "CRLF fixture should report file-relative block open line 6; got: {rendered}"
    );
    assert!(
        rendered.contains("Command at line: 7"),
        "CRLF fixture should report file-relative command line 7; got: {rendered}"
    );
    assert!(
        rendered.contains("> 7 │"),
        "CRLF fixture should highlight file line 7; got: {rendered}"
    );
}

#[test]
fn shell_block_execution_failed_renders_inner_diagnostic() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\ntitle: Test\n---\n# Heading\n\n::shell-block\n{}\n::end-block\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing ::shell-block should error");

    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));

    assert!(
        rendered.contains("doc.md"),
        "rendered diagnostic should contain linked source path; got: {rendered}"
    );
    assert!(
        rendered.contains("> 7 │"),
        "rendered diagnostic should contain source excerpt centered on file line 7; got: {rendered}"
    );
    assert!(
        rendered.contains("```yaml"),
        "rendered diagnostic should contain composed frontmatter block; got: {rendered}"
    );
    assert!(
        rendered.contains("title: Test"),
        "rendered diagnostic should contain frontmatter content; got: {rendered}"
    );
    assert!(
        rendered.contains("stderr:"),
        "rendered diagnostic should label captured stderr; got: {rendered}"
    );
    let after_stderr = rendered
        .split("stderr:")
        .nth(1)
        .expect("stderr section should be present");
    assert!(
        after_stderr.contains("error:"),
        "captured stderr text should appear after the stderr label; got: {rendered}"
    );
}

#[test]
fn frontmatter_shell_origin_is_file_relative() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\ntitle: Test\ncmd: \"$({})\"\n---\n# Heading\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing frontmatter shell should error");

    let msg = err.to_string();
    assert!(
        msg.contains("frontmatter.cmd"),
        "expected frontmatter origin in error; got: {msg}"
    );

    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));
    assert!(
        rendered.contains("> 3 │"),
        "excerpt should highlight frontmatter key line 3; got: {rendered}"
    );
}

#[test]
fn frontmatter_line_count_counts_source_lines_with_crlf() {
    let content = format!(
        "---\r\ntitle: Test\r\n---\r\n# Body\r\n::shell {}\r\n",
        failing_command()
    );
    let md: Markdown = content.into();
    // 3 frontmatter source lines (---, title, ---); body starts at line 4.
    assert_eq!(md.frontmatter_line_count(), 3);
}

#[test]
fn body_shell_origin_counts_lines_not_bytes_with_crlf() {
    let dir = TempDir::new().unwrap();
    let content = format!(
        "---\r\ntitle: Test\r\ncmd: placeholder\r\n---\r\n# Heading\r\n\r\n::shell {}\r\n",
        failing_command()
    );
    std::fs::write(dir.path().join("doc.md"), content).unwrap();

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let err = md
        .compose_with(compose_options(&dir))
        .expect_err("failing ::shell should error");

    let term = Terminal::builder().width(80).build();
    let rendered = strip_escape_codes(err.status_block(&term).render(&term));
    assert!(
        rendered.contains("> 7 │"),
        "CRLF fixture should report file line 7 (source lines, not bytes); got: {rendered}"
    );
}
