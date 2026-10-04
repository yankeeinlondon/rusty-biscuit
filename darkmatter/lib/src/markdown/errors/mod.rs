//! Block-style error rendering for darkmatter's error enums.
//!
//! This module wires each public error enum in the library up to the
//! [`BlockError`] contract from `biscuit-terminal` and provides a
//! [`as_block_error`] helper that callers (e.g. the `md` CLI top-level
//! handler) can use to discover a [`BlockError`] impl on an arbitrary
//! [`std::error::Error`] trait object.
//!
//! [`BlockError`]: biscuit_terminal::errors::BlockError

pub(crate) mod blocks;

use std::error::Error as StdError;

use biscuit_terminal::errors::BlockError;

use crate::editor::EditorError;
use crate::markdown::MarkdownError;
use crate::markdown::compose::FileLinksError;
use crate::markdown::compose::ShellBlockError;
use crate::markdown::compose::ShellExpansionError;
use crate::markdown::compose::TocLinkingError;
use crate::markdown::compose::TransclusionError;
use crate::markdown::compose::conditions::ConditionError;
use crate::markdown::compose::context::merge::CtxMergeError;
use crate::markdown::compose::page_blocks::PageBlockError;
use crate::markdown::compose::transclusion::DeferredSetError;
use crate::markdown::normalize::NormalizationError;
use crate::markdown::reference::ReferenceError;
use crate::markdown::reference::file_tree::FileTreeError;
use crate::markdown::schemas::SchemaError;
use crate::mermaid::MermaidThemeError;
use crate::render::image_ref::ImageRefError;
use crate::render::link::LinkError;
use crate::render::stylesheet::StylesheetBlockError;

/// Try to view `err` as a reference to one of darkmatter's known
/// [`BlockError`] implementations.
///
/// Stable Rust cannot upcast `&dyn StdError` to `&dyn BlockError`
/// automatically, so this helper performs runtime downcasting against the
/// set of concrete darkmatter error types. Callers that already hold a typed
/// reference should call the trait methods directly instead.
///
/// ## Examples
///
/// ```
/// use std::error::Error;
/// use darkmatter::markdown::errors::as_block_error;
/// use darkmatter::markdown::MarkdownError;
///
/// let err: MarkdownError =
///     MarkdownError::Transform("example".to_string());
/// let dyn_err: &(dyn Error + 'static) = &err;
/// assert!(as_block_error(dyn_err).is_some());
/// ```
pub fn as_block_error<'a>(
    err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    if let Some(v) = err.downcast_ref::<MarkdownError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<TransclusionError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ShellExpansionError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ShellBlockError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<PageBlockError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ConditionError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<TocLinkingError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<FileLinksError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ReferenceError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<EditorError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<FileTreeError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<MermaidThemeError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<CtxMergeError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<DeferredSetError>() {
        // Kept for library consumers that call the parser directly. The CLI path
        // cannot reach this arm because `TransclusionError::InvalidFrontmatterAssignment`
        // (promoted from `DeferredSetError` at `compose/transclusion/types.rs:324-336`)
        // is what gets returned from the compose pipeline — `DeferredSetError` itself is
        // never exposed at the top level.
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<NormalizationError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<StylesheetBlockError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<LinkError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ImageRefError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<SchemaError>() {
        return Some(v);
    }
    None
}

/// The stable kebab-case name of a file-reference failure class, as the
/// `failure` detail row of a rendered error spells it.
///
/// The names are a contract: tests and tools read the row instead of the
/// message text, so a name never changes once published.
///
/// ## Examples
///
/// ```
/// use biscuit_file::ResolutionFailure;
/// use darkmatter::markdown::errors::resolution_failure_name;
///
/// assert_eq!(resolution_failure_name(ResolutionFailure::NoMatch), "no-match");
/// ```
pub fn resolution_failure_name(failure: biscuit_file::ResolutionFailure) -> &'static str {
    use biscuit_file::ResolutionFailure;
    match failure {
        ResolutionFailure::InvalidReference => "invalid-reference",
        ResolutionFailure::MissingContext => "missing-context",
        ResolutionFailure::NoMatch => "no-match",
        ResolutionFailure::Io => "io",
        ResolutionFailure::UnsupportedRemote => "unsupported-remote",
    }
}

/// The `failure` detail row a failed file reference's error block carries,
/// rendering as `failure: <name>` (see [`resolution_failure_name`]).
pub fn resolution_failure_row(
    failure: biscuit_file::ResolutionFailure,
) -> biscuit_terminal::components::prose::Prose {
    biscuit_terminal::components::prose::Prose::new(format!(
        "<dim>failure:</dim> {}",
        resolution_failure_name(failure)
    ))
}

/// `message` followed by biscuit-file's literal-glob hint
/// ([`ResolutionFailure::glob_hint`](biscuit_file::ResolutionFailure::glob_hint))
/// on its own `hint:` line, when `failure` is a no-match of a `reference`
/// whose text looks like a glob; otherwise `message` unchanged.
///
/// Every Darkmatter message that reports a single-file failure passes through
/// this, so a literal-glob miss is explained in the same words on every
/// surface and no message tests for wildcards itself.
///
/// ## Examples
///
/// ```
/// use biscuit_file::ResolutionFailure;
/// use darkmatter::markdown::errors::with_glob_hint;
///
/// let glob = with_glob_hint("File not found: docs/*.md", ResolutionFailure::NoMatch, "docs/*.md");
/// assert!(glob.contains("::file-links"));
/// let plain = with_glob_hint("File not found: a.md", ResolutionFailure::NoMatch, "a.md");
/// assert_eq!(plain, "File not found: a.md");
/// ```
pub fn with_glob_hint(
    message: impl Into<String>,
    failure: biscuit_file::ResolutionFailure,
    reference: &str,
) -> String {
    with_hint_line(message, failure.glob_hint(reference))
}

/// `message` followed by `hint` on its own `hint:` line, when there is one:
/// the line [`with_glob_hint`] writes, for a caller that computed the hint
/// when the failure was raised.
pub fn with_hint_line(message: impl Into<String>, hint: Option<&str>) -> String {
    let mut message = message.into();
    if let Some(hint) = hint {
        message.push_str("\nhint: ");
        message.push_str(hint);
    }
    message
}

/// Prose markup for a [`StatusBlock::hint`] whose text spans rows, such as a
/// message carrying the `hint:` line [`with_hint_line`] writes: each newline
/// becomes Prose's backslash hard break, since a hint is otherwise one
/// soft-wrapped paragraph.
///
/// [`StatusBlock::hint`]: biscuit_terminal::components::status_block::StatusBlock::hint
pub fn hint_rows(markup: &str) -> String {
    markup.lines().collect::<Vec<_>>().join("\\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::errors::SourceContext;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;
    use std::path::PathBuf;

    use crate::markdown::compose::ShellCommandOrigin;

    fn render(err: &dyn BlockError) -> String {
        strip_escape_codes(err.report_block_error_optimistic(Some(80)))
    }

    /// Rendered rows without the `┃` border, trimmed, blank rows dropped.
    fn rows(err: &dyn BlockError) -> Vec<String> {
        strip_escape_codes(err.report_block_error_optimistic(Some(200)))
            .lines()
            .map(|line| line.trim_start().trim_start_matches('┃').trim().to_string())
            .filter(|row| !row.is_empty())
            .collect()
    }

    #[test]
    fn a_not_found_transclusion_keeps_its_hint_row() {
        let err = crate::markdown::compose::transclusion::TransclusionError::target_not_found("plans/*.md");
        let rows = rows(&err);
        let at = rows
            .iter()
            .position(|row| row == "File not found: plans/*.md")
            .unwrap_or_else(|| panic!("{rows:#?}"));
        assert!(rows[at + 1].starts_with("hint: *, ?, and [ are literal"), "{rows:#?}");
    }

    #[test]
    fn hint_rows_keep_a_multi_line_hint_on_separate_rows() {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;

        let block = StatusBlock::new(StatusState::Error)
            .body("body")
            .hint(hint_rows("Error: no file matched\nhint: quote it"));
        let rendered = strip_escape_codes(biscuit_terminal::components::renderable::TerminalRenderable::render(
            &block,
            &biscuit_terminal::terminal::Terminal::new_optimistic(200),
        ));
        let rows: Vec<&str> = rendered
            .lines()
            .map(|line| line.trim_start().trim_start_matches('┃').trim())
            .filter(|row| !row.is_empty())
            .collect();
        let at = rows.iter().position(|row| *row == "Error: no file matched").unwrap_or_else(|| panic!("{rows:#?}"));
        assert_eq!(rows[at + 1], "hint: quote it", "{rows:#?}");
    }

    fn test_ctx() -> SourceContext {
        SourceContext::new(PathBuf::from("/test"), PathBuf::from("test"), String::new())
    }

    #[test]
    fn markdown_error_transform_renders_leaf_block() {
        let err = MarkdownError::Transform("pipeline stalled".to_string());
        let out = render(&err);
        assert!(out.contains("MarkdownError"));
        assert!(out.contains("transform failed"));
        assert!(out.contains("pipeline stalled"));
    }

    #[test]
    fn markdown_error_delegates_transclusion_block_without_caused_by() {
        let inner = TransclusionError::CycleDetected {
            chain: vec![
                (PathBuf::from("a.md"), 3),
                (PathBuf::from("b.md"), 7),
                (PathBuf::from("a.md"), 3),
            ],
        };
        let err = MarkdownError::Transclusion(Box::new(inner));
        let out = render(&err);
        assert!(out.contains("TransclusionError"));
        assert!(out.contains("cycle detected"));
        assert!(
            !out.contains("Caused by:"),
            "delegating variant should not duplicate the inner block under a Caused-by: caption: {out}",
        );
    }

    #[test]
    fn transclusion_cycle_detected_lists_chain() {
        let err = TransclusionError::CycleDetected {
            chain: vec![
                (PathBuf::from("a.md"), 3),
                (PathBuf::from("b.md"), 7),
                (PathBuf::from("a.md"), 3),
            ],
        };
        let out = render(&err);
        assert!(out.contains("a.md"));
        assert!(out.contains("b.md"));
        assert!(out.contains("cycle detected"));
        assert!(out.contains(":line 3"));
        assert!(out.contains(":line 7"));
    }

    #[test]
    fn shell_execution_failed_includes_stderr() {
        let err = ShellExpansionError::ExecutionFailed {
            ctx: Box::new(test_ctx()),
            command: "ls --bogus".into(),
            code: 2,
            stdout: String::new(),
            stderr: "ls: unrecognized option".into(),
            origin: ShellCommandOrigin::Body { line: 12 },
        };
        let out = render(&err);
        assert!(out.contains("execution failed"));
        assert!(out.contains("Exit code:"));
        assert!(out.contains("stderr"));
        assert!(out.contains("ls: unrecognized option"));
    }

    #[test]
    fn shell_approval_required_names_whitelist_paths() {
        let err = ShellExpansionError::ApprovalRequired {
            ctx: Box::new(test_ctx()),
            command: "gh repo list".into(),
            whitelist_path: Box::new("/tmp/wl".into()),
            blacklist_path: Box::new("/tmp/bl".into()),
            origin: ShellCommandOrigin::Body { line: 3 },
        };
        let out = render(&err);
        assert!(out.contains("approval required"));
        assert!(out.contains("/tmp/wl"));
        assert!(out.contains("/tmp/bl"));
        assert!(out.contains("--approve-shell"));
    }

    #[test]
    fn page_block_unterminated_has_opening_line() {
        // Build content where line 14 contains the opening directive so the
        // excerpt actually surfaces the line number.
        let mut content = String::new();
        for n in 1..14 {
            content.push_str(&format!("line {n}\n"));
        }
        content.push_str("::block when=\"x\"\nbody\n");

        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("/test.md"),
            std::path::PathBuf::from("test.md"),
            content,
        );
        let err = PageBlockError::UnterminatedBlock {
            ctx: Box::new(ctx),
            opening_line: 14,
            opening_text: "::block when=\"x\"".to_string(),
        };
        let out = render(&err);
        assert!(out.contains("unterminated"));
        assert!(out.contains("14"));
        assert!(out.contains("::block when=\"x\""));
        assert!(out.contains("::end-block"));
    }

    #[test]
    fn condition_parse_lists_operators() {
        let err = ConditionError::Parse {
            ctx: Box::new(test_ctx()),
            expr: "a &&& b".into(),
            line: 7,
            message: "unexpected token".into(),
            span: 3..4,
        };
        let out = render(&err);
        assert!(out.contains("ConditionError"));
        assert!(out.contains("parse failed"));
        assert!(out.contains("a &&& b"));
        assert!(out.contains("^"));
        assert!(out.contains("&&"));
        assert!(out.contains("has_key"));
    }

    #[test]
    fn toc_linking_invalid_cleanup_service_enumerates_valid_names() {
        let err = TocLinkingError::InvalidCleanupService {
            service: "bogus".into(),
            line: 4,
        };
        let out = render(&err);
        assert!(out.contains("invalid cleanup service"));
        assert!(out.contains("bogus"));
        // All cleanup service names should appear in the valid-values list.
        for name in [
            "emoji_leader",
            "emoji_trailing",
            "emoji",
            "number",
            "capitalize",
        ] {
            assert!(
                out.contains(name),
                "expected valid-values list to include {name}; output was:\n{out}"
            );
        }
        assert!(out.contains("Strip leading emoji"));
    }

    #[test]
    fn reference_parse_directive_has_syntax_hint() {
        let err = ReferenceError::ParseDirective {
            ctx: Box::new(SourceContext::new(
                PathBuf::from("/tmp/test/docs/root.md"),
                PathBuf::from("docs/root.md"),
                "::file ./broken.md when=\n".to_string(),
            )),
            line: 2,
            message: "unexpected end".into(),
            directive_text: "::file ./broken.md when=".to_string(),
            caret_col: Some(27),
        };
        let out = render(&err);
        assert!(out.contains("ReferenceError"));
        assert!(out.contains("docs/root.md"));
        assert!(out.contains("::file ./broken.md when="));
        assert!(out.contains("Column 27"));
        assert!(out.contains("::file"));
    }

    #[test]
    fn editor_no_editor_found_lists_probed_binaries() {
        let err = EditorError::NoEditorFound;
        let out = render(&err);
        assert!(out.contains("no editor found"));
        assert!(out.contains("$EDITOR"));
        // Probe list should mention at least one well-known editor binary name.
        assert!(
            out.contains("nvim") || out.contains("vim") || out.contains("code"),
            "probe list missing expected editor binary: {out}"
        );
    }

    #[test]
    fn file_tree_path_not_found_contains_provided_path() {
        let err = FileTreeError::PathNotFound("./definitely/missing.md".into());
        let out = render(&err);
        assert!(out.contains("path not found"));
        assert!(out.contains("./definitely/missing.md"));
    }

    #[test]
    fn mermaid_invalid_color_hints_accepted_formats() {
        let err = MermaidThemeError::InvalidColor {
            field: "background".into(),
            value: "notacolor".into(),
        };
        let out = render(&err);
        assert!(out.contains("invalid color"));
        assert!(out.contains("background"));
        assert!(out.contains("notacolor"));
        assert!(out.contains("#rrggbb") || out.contains("#rgb"));
    }

    // ── as_block_error downcast registry ──────────────────────────────

    #[test]
    fn as_block_error_discovers_markdown_error() {
        let err = MarkdownError::Transform("x".to_string());
        let dyn_err: &(dyn std::error::Error + 'static) = &err;
        assert!(as_block_error(dyn_err).is_some());
    }

    #[test]
    fn as_block_error_discovers_reference_error() {
        let err = ReferenceError::Validation("bad".to_string());
        let dyn_err: &(dyn std::error::Error + 'static) = &err;
        assert!(as_block_error(dyn_err).is_some());
    }

    #[test]
    fn as_block_error_returns_none_for_unknown_error() {
        let err: std::io::Error = std::io::Error::other("bare");
        let dyn_err: &(dyn std::error::Error + 'static) = &err;
        assert!(as_block_error(dyn_err).is_none());
    }

    #[test]
    fn escaped_message_renders_its_code_span_without_backslashes() {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable;

        let rendered = strip_escape_codes(
            Prose::new(Prose::escape_text_outside_code_spans("glob `docs/*.md` and *not* <this>")).render_optimistic(Some(80)),
        );
        assert!(rendered.contains("docs/*.md"), "{rendered}");
        assert!(rendered.contains("*not* <this>"), "{rendered}");
        assert!(!rendered.contains('\\'), "{rendered}");
    }
}
