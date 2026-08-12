//! End-to-end integration tests for shell blocks in the compose pipeline.
//!
//! Validates that `::shell-block` / `::end-block` directives execute correctly
//! through the full `Markdown::compose_with()` pipeline, including interaction
//! with page blocks, transclusion, and other compose stages.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOperation, ComposeOptions,
    shell_expansion::types::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest},
};
use std::sync::Arc;
use tempfile::TempDir;

/// Compose options without the repo-wide capture `ComposeOptions::new()` runs
/// (git, repo, file changes, languages, docs, OS, hardware, GPU via sniff —
/// 1.4s per call on this working tree). No fixture here reads `ctx.*`, and a
/// group an expression does ask for is still captured on demand.
fn context_free_options() -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(
        std::path::Path::new("."),
        "",
    ))
}

fn write_files(dir: &TempDir, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
    }
}

// ── Approval handlers ──────────────────────────────────────────────

struct AllowAllHandler;

impl ShellApprovalHandler for AllowAllHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

struct DenyAllHandler;

impl ShellApprovalHandler for DenyAllHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Basic end-to-end compose tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn compose_single_shell_block() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "# Test\n\n::shell-block\necho hello\n::end-block\n\nDone.\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("hello"),
        "Expected shell output in composed content, got: {output}"
    );
    assert!(
        !output.contains("::shell-block"),
        "Shell block directive should be replaced, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 1);
}

#[test]
fn compose_multiple_shell_blocks() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "::shell-block\necho first\n::end-block\n\n::shell-block\necho second\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("first"),
        "Expected first block output, got: {output}"
    );
    assert!(
        output.contains("second"),
        "Expected second block output, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 2);
}

#[test]
fn compose_shell_block_with_multiple_commands() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("doc.md", "::shell-block\necho a\necho b\n::end-block\n")],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler))
        .disable(ComposeOperation::Cleanup);

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();
    let output = composed.content();

    // Command outputs are concatenated verbatim; the line break between them is
    // each `echo`'s own trailing newline, not an inserted blank line.
    assert!(
        output.contains("a\nb\n"),
        "Expected verbatim-concatenated command outputs, got: {output:?}"
    );
}

#[test]
fn compose_shell_block_with_empty_output_commands() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "::shell-block\necho -n\necho hello\necho -n\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert_eq!(
        output.trim(),
        "hello",
        "Empty outputs should be omitted, got: {output:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Interaction with page blocks
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shell_block_inside_true_page_block_executes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "---\nshow_shell: true\n---\n\n::block when=\"show_shell\"\n::shell-block\necho inside\n::end-block\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("inside"),
        "Shell block inside true page block should execute, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 1);
}

#[test]
fn shell_block_inside_false_page_block_is_removed() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "---\nshow_shell: false\n---\n\n::block when=\"show_shell\"\n::shell-block\necho inside\n::end-block\n::end-block\n\nDone.\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        !output.contains("inside"),
        "Shell block inside false page block should be removed, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 0);
    assert!(
        output.contains("Done."),
        "Remaining content should be preserved"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Interaction with transclusion
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shell_block_in_transcluded_document_executes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("parent.md", "# Parent\n\n::file child.md\n"),
            ("child.md", "::shell-block\necho from-child\n::end-block\n"),
        ],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("parent.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("parent.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("from-child"),
        "Shell block in transcluded document should execute, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 1);
}

#[test]
fn shell_block_with_conditional_transclusion() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                "---\ninclude_child: true\n---\n\n::file child.md when=\"include_child\"\n",
            ),
            ("child.md", "::shell-block\necho conditional\n::end-block\n"),
        ],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("parent.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("parent.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("conditional"),
        "Conditional transclusion should include shell block, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 1);
}

#[test]
fn shell_block_skipped_when_transclusion_condition_false() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                "---\ninclude_child: false\n---\n\n::file child.md when=\"include_child\"\n",
            ),
            ("child.md", "::shell-block\necho hidden\n::end-block\n"),
        ],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("parent.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("parent.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        !output.contains("hidden"),
        "Shell block in skipped transclusion should not execute, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 0);
}

// ═══════════════════════════════════════════════════════════════════
//  Error handling in compose pipeline
// ═══════════════════════════════════════════════════════════════════

#[test]
fn compose_fails_when_shell_block_command_denied() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "# Test\n\n::shell-block\necho hello\n::end-block\n\nDone.\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(DenyAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let result = md.compose_with(options);

    assert!(
        result.is_err(),
        "Expected compose to fail when shell command is denied"
    );
}

#[test]
fn compose_fails_on_unterminated_shell_block() {
    let dir = TempDir::new().unwrap();
    write_files(&dir, &[("doc.md", "# Test\n\n::shell-block\necho hello\n")]);

    let options = context_free_options().with_source_file(dir.path().join("doc.md"));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let result = md.compose_with(options);

    assert!(
        result.is_err(),
        "Expected compose to fail on unterminated shell block"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Unterminated") || err.contains("parse error"),
        "Expected unterminated error, got: {err}"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Shell blocks with interpolation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shell_block_after_interpolation() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "---\nmessage: hello\n---\n\n::shell-block\necho {{message}}\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("hello"),
        "Shell block should see interpolated frontmatter value, got: {output}"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Bulk: many blocks in a single document
// ═══════════════════════════════════════════════════════════════════

/// Composes `block_count` trivial shell blocks and returns the composed output
/// plus the applied-block count.
fn compose_shell_blocks(block_count: usize) -> (String, usize) {
    let dir = TempDir::new().unwrap();

    let mut content = String::new();
    for i in 0..block_count {
        content.push_str(&format!("::shell-block\necho block-{i}\n::end-block\n\n"));
    }
    write_files(&dir, &[("doc.md", &content)]);

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();

    (composed.content().to_string(), report.shell_blocks_applied)
}

/// Every block in a many-block document is applied, and spans stay aligned
/// across the whole splice.
///
/// This deliberately asserts no wall-clock bound. Cost here is one real process
/// spawn per block — measured at ~4ms/block and flat from 12 to 96 blocks — so
/// scan and splice work is under 1% of the elapsed time and a timing assertion
/// grades the host's process-spawn throughput instead of this crate's block
/// handling. Spawn latency is heavy-tailed enough that a large-over-small ratio
/// did not tame it either: on an idle Linux host the 48-over-12 ratio ranged
/// 1.9×–5.7× around a nominal 4×, and it reached 10.3× under full-suite load.
/// Windows and macOS spawn more slowly still. Compose scaling belongs in the
/// `compose_pipeline` criterion bench, where a run is repeated and compared
/// against a stored baseline.
#[test]
fn many_blocks_in_one_document() {
    let (output, applied) = compose_shell_blocks(48);

    assert_eq!(applied, 48, "Expected all 48 shell blocks to be applied");

    // First and last confirm the reverse-order splice did not drift: a span
    // mismatch corrupts the ends before anything in the middle.
    assert!(output.contains("block-0"), "Expected block-0 in output");
    assert!(output.contains("block-47"), "Expected block-47 in output");
}

// ═══════════════════════════════════════════════════════════════════
//  Shell block with error handling options
// ═══════════════════════════════════════════════════════════════════

#[test]
fn compose_shell_block_with_when_error() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "::shell-block when_error=\"fallback\"\necho hello\nfalse\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("hello"),
        "Expected 'hello' output, got: {output}"
    );
    assert!(
        output.contains("fallback"),
        "Expected 'fallback' for failed command, got: {output}"
    );
}

#[test]
fn compose_shell_block_with_timeout() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("doc.md", "::shell-block timeout=1\nsleep 5\n::end-block\n")],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let result = md.compose_with(options);

    assert!(result.is_err(), "Expected timeout error");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("timed out") || err.contains("timeout"),
        "Expected timeout error, got: {err}"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Mixed shell directives and shell blocks
// ═══════════════════════════════════════════════════════════════════

#[test]
fn compose_mixed_shell_directive_and_shell_block() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "::shell echo standalone\n\n::shell-block\necho block-a\necho block-b\n::end-block\n",
        )],
    );

    let options = context_free_options()
        .with_source_file(dir.path().join("doc.md"))
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(std::env::current_dir().unwrap())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));

    let md = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let (composed, report) = md.compose_with(options).unwrap();
    let output = composed.content();

    assert!(
        output.contains("standalone"),
        "Expected standalone shell output, got: {output}"
    );
    assert!(
        output.contains("block-a"),
        "Expected shell block output a, got: {output}"
    );
    assert!(
        output.contains("block-b"),
        "Expected shell block output b, got: {output}"
    );
    assert_eq!(report.shell_blocks_applied, 1);
}
