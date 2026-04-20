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
use crate::mermaid::MermaidThemeError;
use crate::render::image_ref::ImageRefError;
use crate::render::link::LinkError;
use crate::render::stylesheet::StylesheetError;

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
    if let Some(v) = err.downcast_ref::<PageBlockError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ConditionError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<TocLinkingError>() {
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
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<NormalizationError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<StylesheetError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<LinkError>() {
        return Some(v);
    }
    if let Some(v) = err.downcast_ref::<ImageRefError>() {
        return Some(v);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::markdown::compose::ShellCommandOrigin;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_escape = false;
        for ch in s.chars() {
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else if ch == '\x1b' {
                in_escape = true;
            } else {
                out.push(ch);
            }
        }
        out
    }

    fn render(err: &dyn BlockError) -> String {
        strip_ansi(&err.report_block_error_optimistic(Some(80)))
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
            chain: vec!["a.md".into(), "b.md".into(), "a.md".into()],
        };
        let err = MarkdownError::Transclusion(inner);
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
            chain: vec!["a.md".into(), "b.md".into(), "a.md".into()],
        };
        let out = render(&err);
        assert!(out.contains("a.md"));
        assert!(out.contains("b.md"));
        assert!(out.contains("cycle detected"));
    }

    #[test]
    fn shell_execution_failed_includes_stderr() {
        let err = ShellExpansionError::ExecutionFailed {
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
            command: "gh repo list".into(),
            whitelist_path: "/tmp/wl".into(),
            blacklist_path: "/tmp/bl".into(),
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
        let err = PageBlockError::UnterminatedBlock { line: 14 };
        let out = render(&err);
        assert!(out.contains("unterminated"));
        assert!(out.contains("14"));
        assert!(out.contains("::end-block"));
    }

    #[test]
    fn condition_parse_lists_operators() {
        let err = ConditionError::Parse {
            expr: "a &&& b".into(),
            line: 7,
            message: "unexpected token".into(),
        };
        let out = render(&err);
        assert!(out.contains("ConditionError"));
        assert!(out.contains("parse failed"));
        assert!(out.contains("a &&& b"));
        assert!(out.contains("&&"));
        assert!(out.contains("HasKey"));
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
    }

    #[test]
    fn reference_parse_directive_has_syntax_hint() {
        let err = ReferenceError::ParseDirective {
            line: 2,
            message: "unexpected end".into(),
        };
        let out = render(&err);
        assert!(out.contains("ReferenceError"));
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
