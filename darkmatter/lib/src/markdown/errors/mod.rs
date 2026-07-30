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
}
