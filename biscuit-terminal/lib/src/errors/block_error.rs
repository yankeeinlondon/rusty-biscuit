//! The [`BlockError`] trait and supporting rendering helpers.
//!
//! `BlockError` is the terminal-rendering contract for errors that want to
//! produce a consistent **Block Style Error** report — a `Status` title line
//! over a `StatusBlock` red-bordered body plus an optional hint.
//!
//! Implementors pair this trait with their existing `std::error::Error`
//! (typically via `thiserror`); `BlockError` adds rendering without
//! replacing anything. See `biscuit-terminal/README.md` for the full
//! rendering contract.

use std::error::Error as StdError;

use crate::components::prose::Prose;
use crate::components::renderable::Renderable;
use crate::components::status::StatusState;
use crate::components::status_block::StatusBlock;
use crate::terminal::Terminal;

/// Maximum cause-chain depth rendered by [`render_with_causes`].
const DEFAULT_CAUSE_DEPTH: usize = 3;

/// An error that can render itself as a Block Style Error for terminal output.
///
/// Implementors provide a [`status_block`](Self::status_block) method that
/// returns a configured (but unrendered) [`StatusBlock`]. All other methods
/// have defaults that compose the block into a string using the supplied
/// [`Terminal`].
///
/// `BlockError` extends [`StdError`] so every `thiserror`-derived enum already
/// satisfies the supertrait bound.
///
/// ## Examples
///
/// ```
/// use std::error::Error;
/// use std::fmt;
/// use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
/// use biscuit_terminal::prelude::*;
///
/// #[derive(Debug)]
/// struct CycleDetected { chain: Vec<String> }
///
/// impl fmt::Display for CycleDetected {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "cycle detected: {}", self.chain.join(" -> "))
///     }
/// }
///
/// impl Error for CycleDetected {}
///
/// impl BlockError for CycleDetected {
///     fn status_block(&self, _term: &Terminal) -> StatusBlock {
///         StatusBlock::new(self.severity())
///             .error_header(ErrorHeader::new("CycleDetected", "transclusion cycle"))
///             .body(format!("chain: {}", self.chain.join(" -> ")))
///             .hint("Break the cycle by removing one of the edges.")
///     }
/// }
///
/// let err = CycleDetected { chain: vec!["a.md".into(), "b.md".into(), "a.md".into()] };
/// let out = err.report_block_error_optimistic(Some(80));
/// assert!(out.contains("CycleDetected"));
/// assert!(out.contains("transclusion cycle"));
/// ```
pub trait BlockError: StdError {
    /// Build the [`StatusBlock`] for this error.
    ///
    /// Implementors own the header/body/hint text and the choice of severity.
    /// The `term` argument lets implementors inspect terminal capabilities
    /// (width, color depth, nerd font, etc.) while assembling block content.
    fn status_block(&self, term: &Terminal) -> StatusBlock;

    /// Preferred severity for this error.
    ///
    /// Defaults to [`StatusState::Error`]. Override for warnings or other
    /// non-fatal conditions that still want block-style reporting.
    fn severity(&self) -> StatusState {
        StatusState::Error
    }

    /// Render this error to a terminal-ready string.
    ///
    /// The default implementation calls
    /// `status_block(term).render(term)`. Implementors that need to prepend
    /// a summary or append a cause chain should override this method; see
    /// [`render_with_causes`] for the canonical cause-chain pattern.
    fn report_block_error(&self, term: &Terminal) -> String {
        self.status_block(term).render(term)
    }

    /// Terminal-agnostic render for logs, piped output, or tests.
    ///
    /// Uses [`Terminal::new_optimistic`] with either the provided `width` or
    /// 80 columns as a fallback. The default implementation delegates to
    /// [`report_block_error`](Self::report_block_error) so any override there
    /// is honoured automatically.
    fn report_block_error_optimistic(&self, width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(width.unwrap_or(80));
        self.report_block_error(&term)
    }

    /// If this error wraps another [`BlockError`], return a reference to it.
    ///
    /// Used by wrapper errors (e.g. `MarkdownError`) to surface the inner
    /// block under a `Caused by:` caption. Defaults to `None`; wrapper enums
    /// override this to match on their variants.
    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        None
    }
}

/// Canonical header builder for Block Style Errors.
///
/// Produces the `<b>ErrorName:</b> <b>Title</b>` prose pattern shared by every
/// [`BlockError`]. Use with [`StatusBlockExt::error_header`] to avoid hand-
/// rolling the prose tokens at each call site.
#[derive(Debug, Clone, Copy)]
pub struct ErrorHeader<'a> {
    /// The error type name (e.g. `"TransclusionError"`).
    pub error_name: &'a str,
    /// The human-readable summary (e.g. `"cycle detected"`).
    pub title: &'a str,
}

impl<'a> ErrorHeader<'a> {
    /// Create a new [`ErrorHeader`].
    pub fn new(error_name: &'a str, title: &'a str) -> Self {
        Self { error_name, title }
    }

    /// Render the header as a Prose-ready string with two bold segments.
    pub fn into_prose(self) -> String {
        format!("<b>{}:</b> <b>{}</b>", self.error_name, self.title)
    }
}

/// Extension on [`StatusBlock`] that accepts an [`ErrorHeader`] directly.
///
/// Keeps the canonical header shape at one call per error variant, instead
/// of every variant re-typing the same Prose tokens.
pub trait StatusBlockExt {
    /// Set the header to the prose form of `header`.
    fn error_header(self, header: ErrorHeader<'_>) -> Self;
}

impl StatusBlockExt for StatusBlock {
    fn error_header(self, header: ErrorHeader<'_>) -> Self {
        self.header(header.into_prose())
    }
}

/// Render a [`BlockError`] together with up to three nested causes.
///
/// Walks [`BlockError::block_source`] and stacks each cause's block under a
/// dim `Caused by:` caption, joined with newlines. Use this from wrapper
/// errors whose inner variants also implement `BlockError` (see the
/// `MarkdownError` delegation strategy in `darkmatter/docs/error-rendering.md`).
pub fn render_with_causes<E: BlockError + ?Sized>(err: &E, term: &Terminal) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(err.status_block(term).render(term));

    let mut current = err.block_source();
    let mut depth = 0;
    while let Some(cause) = current {
        if depth >= DEFAULT_CAUSE_DEPTH {
            break;
        }
        parts.push(Prose::new("<dim>Caused by:</dim>").render(term));
        parts.push(cause.status_block(term).render(term));
        current = cause.block_source();
        depth += 1;
    }

    parts.join("\n")
}

/// Attempt to view `err` as a [`BlockError`] reference.
///
/// Generic upcasting from `&dyn StdError` to `&dyn BlockError` is not possible
/// on stable Rust, so this helper currently returns [`None`]. It exists as the
/// single detection seam callers (e.g. the `md` CLI top-level handler) can use
/// today; a later phase of the rollout will extend it — via either a proc-macro
/// registry or stable trait upcasting — without touching call sites.
///
/// Callers that already hold a typed reference to a concrete `BlockError`
/// should call its methods directly instead of routing through this function.
pub fn as_block_error<'a>(
    _err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::renderable::Renderable;
    use crate::discovery::detection::ColorDepth;
    use crate::utils::color::{Color, Tailwind};
    use std::fmt;

    // ── Test doubles ──────────────────────────────────────────────────

    #[derive(Debug)]
    struct SampleError {
        name: &'static str,
        title: &'static str,
        body: &'static str,
        hint: Option<&'static str>,
        severity: StatusState,
    }

    impl fmt::Display for SampleError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {}", self.name, self.title)
        }
    }

    impl StdError for SampleError {}

    impl BlockError for SampleError {
        fn status_block(&self, _term: &Terminal) -> StatusBlock {
            let mut block = StatusBlock::new(self.severity())
                .error_header(ErrorHeader::new(self.name, self.title))
                .body(self.body);
            if let Some(hint) = self.hint {
                block = block.hint(hint);
            }
            block
        }

        fn severity(&self) -> StatusState {
            self.severity.clone()
        }
    }

    #[derive(Debug)]
    struct WrapperError {
        outer_body: &'static str,
        inner: SampleError,
    }

    impl fmt::Display for WrapperError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "wrapper: {}", self.inner)
        }
    }

    impl StdError for WrapperError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            Some(&self.inner)
        }
    }

    impl BlockError for WrapperError {
        fn status_block(&self, _term: &Terminal) -> StatusBlock {
            StatusBlock::new(self.severity())
                .error_header(ErrorHeader::new("WrapperError", "outer failure"))
                .body(self.outer_body)
        }

        fn report_block_error(&self, term: &Terminal) -> String {
            render_with_causes(self, term)
        }

        fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
            Some(&self.inner)
        }
    }

    fn sample(name: &'static str, title: &'static str, body: &'static str) -> SampleError {
        SampleError {
            name,
            title,
            body,
            hint: None,
            severity: StatusState::Error,
        }
    }

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

    fn no_color_term(width: u32) -> Terminal {
        let mut term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    // ── ErrorHeader ───────────────────────────────────────────────────

    #[test]
    fn error_header_into_prose_wraps_both_segments() {
        let header = ErrorHeader::new("TransclusionError", "cycle detected");
        assert_eq!(
            header.into_prose(),
            "<b>TransclusionError:</b> <b>cycle detected</b>"
        );
    }

    #[test]
    fn status_block_ext_applies_error_header() {
        let term = no_color_term(80);
        let block = StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("MyError", "a problem"))
            .body("details here");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("MyError"));
        assert!(rendered.contains("a problem"));
        assert!(rendered.contains("┃ details here"));
    }

    // ── Severity defaults ─────────────────────────────────────────────

    #[test]
    fn default_severity_is_error() {
        #[derive(Debug)]
        struct Minimal;
        impl fmt::Display for Minimal {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("Minimal")
            }
        }
        impl StdError for Minimal {}
        impl BlockError for Minimal {
            fn status_block(&self, _term: &Terminal) -> StatusBlock {
                StatusBlock::new(self.severity()).body("msg")
            }
        }
        let m = Minimal;
        assert_eq!(m.severity(), StatusState::Error);
    }

    #[test]
    fn severity_override_is_respected() {
        let err = SampleError {
            name: "MildError",
            title: "heads up",
            body: "just a warning",
            hint: None,
            severity: StatusState::Warning,
        };
        assert_eq!(err.severity(), StatusState::Warning);
        let block = err.status_block(&no_color_term(80));
        // The border colour comes from severity; warnings are orange.
        let output = strip_ansi(&block.render(&no_color_term(80)));
        assert!(output.contains("heads up"));
        // Sanity-check the Warning default colour is not Error's red.
        assert_ne!(
            StatusState::Warning.default_color(),
            Color::Tailwind(Tailwind::Red500)
        );
    }

    // ── report_block_error / optimistic ───────────────────────────────

    #[test]
    fn report_block_error_contains_header_body_hint() {
        let err = SampleError {
            name: "FileLoad",
            title: "file not found",
            body: "checked ./docs/root.md",
            hint: Some("Create the file or fix the reference."),
            severity: StatusState::Error,
        };
        let term = no_color_term(80);
        let rendered = strip_ansi(&err.report_block_error(&term));
        assert!(rendered.contains("FileLoad"));
        assert!(rendered.contains("file not found"));
        assert!(rendered.contains("┃ checked ./docs/root.md"));
        assert!(rendered.contains("Create the file or fix the reference."));
    }

    #[test]
    fn report_block_error_optimistic_matches_render_at_same_width() {
        let err = sample("OptimisticError", "something went wrong", "body-text");
        let width = 80;
        let term = Terminal::new_optimistic(width);
        assert_eq!(
            err.report_block_error_optimistic(Some(width)),
            err.report_block_error(&term)
        );
    }

    #[test]
    fn report_block_error_optimistic_defaults_to_eighty_columns() {
        let err = sample("OptimisticError", "default width", "body");
        let out = err.report_block_error_optimistic(None);
        // The optimistic render uses TrueColor, so ANSI sequences appear.
        // Strip them and confirm the human content survives.
        let plain = strip_ansi(&out);
        assert!(plain.contains("OptimisticError"));
        assert!(plain.contains("default width"));
        assert!(plain.contains("body"));
    }

    #[test]
    fn block_source_defaults_to_none() {
        let err = sample("Leaf", "no source", "body");
        assert!(err.block_source().is_none());
    }

    // ── render_with_causes ────────────────────────────────────────────

    #[test]
    fn render_with_causes_includes_caused_by_caption() {
        let wrapper = WrapperError {
            outer_body: "top-level wrapper body",
            inner: sample("InnerError", "root cause", "inner body"),
        };
        let term = no_color_term(80);
        let rendered = strip_ansi(&wrapper.report_block_error(&term));
        assert!(rendered.contains("WrapperError"));
        assert!(rendered.contains("InnerError"));
        assert!(rendered.contains("Caused by:"));
        // Outer block comes before the Caused by: caption, and caption comes
        // before the inner block.
        let outer_idx = rendered
            .find("top-level wrapper body")
            .expect("outer body present");
        let caused_idx = rendered.find("Caused by:").expect("caused-by present");
        let inner_idx = rendered.find("inner body").expect("inner body present");
        assert!(
            outer_idx < caused_idx && caused_idx < inner_idx,
            "ordering outer < caused-by < inner should hold in the output: {rendered:?}",
        );
    }

    #[test]
    fn render_with_causes_stops_at_none() {
        let err = sample("Leaf", "lonely", "body only");
        let term = no_color_term(80);
        let rendered = strip_ansi(&render_with_causes(&err, &term));
        assert!(rendered.contains("Leaf"));
        assert!(!rendered.contains("Caused by:"));
    }

    // ── as_block_error stub ───────────────────────────────────────────

    #[test]
    fn as_block_error_returns_none_for_arbitrary_std_errors() {
        let err = sample("Opaque", "title", "body");
        let std_err: &(dyn StdError + 'static) = &err;
        assert!(as_block_error(std_err).is_none());
    }
}
