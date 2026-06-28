//! Top-level cause-chain walker that renders darkmatter `BlockError`
//! reports for any error produced by the CLI's subcommands.
//!
//! The walker iterates a [`color_eyre::Report`]'s cause chain from
//! outermost to innermost, calls
//! [`darkmatter::markdown::errors::as_block_error`] on each cause, and
//! renders the deepest typed match via its
//! [`BlockError::report_block_error`] impl.
//!
//! If no cause implements [`BlockError`], the caller falls back to
//! `color_eyre`'s default `Debug` output.

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use claudine::composition::{CompositionError, FrontmatterExcerpt};
use color_eyre::eyre::Report;
use darkmatter::markdown::errors::as_block_error;
use std::error::Error as StdError;

/// Try to render `report` as a darkmatter [`BlockError`] report.
///
/// Walks the report's cause chain and returns the rendered string for the
/// **deepest** cause that implements [`BlockError`]. Deepest is preferred
/// so wrappers (`ClaudineError`, `CompositionError`, `MarkdownError`, …)
/// do not shadow the richer leaf error's metadata.
///
/// Returns `None` when no cause implements [`BlockError`].
pub(crate) fn try_render_block_report(report: &Report, term: &Terminal) -> Option<String> {
    let root: &(dyn StdError + 'static) = report.as_ref();

    // A frontmatter-enriched error wraps its real cause; render from the inner
    // error so the deepest typed block still wins, then append the captured
    // frontmatter excerpt after it.
    let wrapper = find_frontmatter_wrapper(root);
    let start: &(dyn StdError + 'static) = match wrapper {
        Some((inner, _)) => inner,
        None => root,
    };

    let deepest = deepest_block_error(start)?;
    let mut out = deepest.report_block_error(term);

    if let Some((_, excerpt)) = wrapper {
        let appendix = excerpt.render_appendix(term);
        if !appendix.is_empty() {
            out = out.trim_end_matches('\n').to_string();
            out.push_str(&appendix);
            out.push('\n');
        }
    }

    // Ensure plain-mode / NO_COLOR output contains no ANSI escape bytes even
    // when the deepest block error (e.g. MarkdownError) renders with styles.
    if matches!(term.color_depth, ColorDepth::None) {
        out = strip_escape_codes(&out);
    }

    Some(out)
}

/// Find the first [`CompositionError::WithFrontmatter`] in the cause chain,
/// returning its inner error and captured excerpt.
fn find_frontmatter_wrapper<'a>(
    root: &'a (dyn StdError + 'static),
) -> Option<(&'a CompositionError, &'a FrontmatterExcerpt)> {
    let mut current = Some(root);
    while let Some(err) = current {
        if let Some(CompositionError::WithFrontmatter { inner, excerpt }) =
            err.downcast_ref::<CompositionError>()
        {
            return Some((inner.as_ref(), excerpt));
        }
        current = err.source();
    }
    None
}

fn deepest_block_error<'a>(
    err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    let mut deepest = discover_block_error(err);
    let mut current = err.source();
    while let Some(next) = current {
        if let Some(found) = discover_block_error(next) {
            deepest = Some(found);
        }
        current = next.source();
    }
    deepest
}

fn discover_block_error<'a>(
    err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    if let Some(found) = as_block_error(err) {
        return Some(found);
    }
    if let Some(v) = err.downcast_ref::<CompositionError>() {
        return Some(v);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;
    use color_eyre::eyre::eyre;
    use darkmatter::markdown::MarkdownError;
    use darkmatter::markdown::compose::{ShellCommandOrigin, ShellExpansionError};
    use std::path::PathBuf;

    fn width80() -> Terminal {
        Terminal::new_optimistic(80)
    }

    #[test]
    fn walks_past_wrapper_error_to_typed_inner() {
        let ctx = biscuit_terminal::errors::SourceContext::new(
            PathBuf::from("<test>"),
            PathBuf::from("<test>"),
            "gh repo list".to_string(),
        );
        let shell = ShellExpansionError::ApprovalRequired {
            ctx: Box::new(ctx),
            command: "gh repo list".into(),
            whitelist_path: PathBuf::from("/tmp/wl"),
            blacklist_path: PathBuf::from("/tmp/bl"),
            origin: ShellCommandOrigin::Body { line: 7 },
        };
        let md: MarkdownError = shell.into();
        let cli_err = claudine::error::ClaudineError::SystemPromptComposition(md);
        let report: Report = eyre!(cli_err);

        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("approval required"), "got:\n{plain}");
        assert!(plain.contains("/tmp/wl"), "got:\n{plain}");
    }

    #[test]
    fn returns_none_for_untyped_error() {
        let report: Report = eyre!("bare non-block error");
        assert!(try_render_block_report(&report, &width80()).is_none());
    }

    #[test]
    fn renders_transclusion_cycle_chain() {
        use darkmatter::markdown::compose::TransclusionError;
        let inner = TransclusionError::CycleDetected {
            chain: vec![
                (PathBuf::from("a.md"), 3),
                (PathBuf::from("b.md"), 7),
                (PathBuf::from("a.md"), 3),
            ],
        };
        let md: MarkdownError = MarkdownError::Transclusion(Box::new(inner));
        let report: Report = eyre!(claudine::composition::CompositionError::ComposeFailed(md));

        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("a.md"), "got:\n{plain}");
        assert!(plain.contains("b.md"), "got:\n{plain}");
        assert!(plain.contains("cycle detected"), "got:\n{plain}");
    }

    const LEAK_DOC: &str =
        "---\nreview_file: computed.md\nsuccess:\n    message: \"at {{review-file}}\"\n---\nbody\n";

    fn leak_with_frontmatter(stderr_is_tty: bool) -> CompositionError {
        let excerpt = FrontmatterExcerpt::capture(LEAK_DOC, Some("success.message"), stderr_is_tty)
            .expect("frontmatter block");
        CompositionError::WithFrontmatter {
            inner: Box::new(CompositionError::LifecycleInterpolationLeak {
                source_path: PathBuf::from("review.md"),
                property: "success.message".to_string(),
                expression: "review-file".to_string(),
                reason: String::new(),
            }),
            excerpt,
        }
    }

    #[test]
    fn appends_frontmatter_yaml_block_for_lifecycle_leak_on_tty() {
        let report: Report = eyre!(leak_with_frontmatter(true));
        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        // The primary diagnostic still renders from the inner leak error.
        assert!(plain.contains("interpolation leaked"), "got:\n{plain}");
        // The frontmatter block is appended, showing the offending YAML lines.
        assert!(plain.contains("review_file"), "yaml block missing:\n{plain}");
        assert!(plain.contains("success:"), "yaml block missing:\n{plain}");
    }

    #[test]
    fn withholds_frontmatter_yaml_block_when_not_tty() {
        let report: Report = eyre!(leak_with_frontmatter(false));
        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("interpolation leaked"), "got:\n{plain}");
        // Non-TTY output must not expose the frontmatter body.
        assert!(!plain.contains("review_file"), "yaml leaked to non-tty:\n{plain}");
    }

    #[test]
    fn appends_yaml_block_for_inline_sequence_mismatch_on_tty() {
        let doc = "---\nprompt: Do it\nsequence:\n  - name: Hello\n---\nbody\n";
        let excerpt = FrontmatterExcerpt::capture(doc, None, true).expect("frontmatter block");
        let err = CompositionError::WithFrontmatter {
            inner: Box::new(CompositionError::InlineComposeSequenceMismatch {
                source_path: PathBuf::from("greeting.md"),
            }),
            excerpt,
        };
        let report: Report = eyre!(err);
        let plain =
            strip_escape_codes(try_render_block_report(&report, &width80()).expect("block"));
        assert!(plain.contains("claudine sequence"), "diagnostic: {plain}");
        // The authored frontmatter is shown as a YAML block.
        assert!(plain.contains("prompt: Do it"), "yaml block missing: {plain}");
        assert!(plain.contains("sequence:"), "yaml block missing: {plain}");
    }

    #[test]
    fn renders_frontmatter_parse_block_through_composition_error() {
        let yaml_err: biscuit_file::YamlParseError =
            biscuit_file::serde_yaml_ng::from_str::<biscuit_file::serde_yaml_ng::Value>(
                "prompt: |-\n    four spaces\n   three spaces\n",
            )
            .expect_err("malformed YAML should fail to parse");
        let ctx = biscuit_terminal::errors::SourceContext::new(
            PathBuf::from("metadata.md"),
            PathBuf::from("metadata.md"),
            "prompt: |-\n    four spaces\n   three spaces\n".to_string(),
        );
        let md = MarkdownError::FrontmatterParse {
            ctx,
            source: yaml_err,
        };
        let report: Report = eyre!(claudine::composition::CompositionError::FrontmatterParse(md));

        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("frontmatter parse failed"), "got:\n{plain}");
        assert!(plain.contains("metadata.md"), "got:\n{plain}");
    }

    /// Acceptance criterion #7: the fence-mismatch path still reports the typed
    /// error and actionable hint in non-TTY / no-color output, but withholds
    /// the TTY-only frontmatter appendix and emits no ANSI escape bytes.
    #[test]
    fn fence_mismatch_non_tty_has_no_ansi_and_no_appendix() {
        const FENCE_DOC: &str =
            "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n";
        let excerpt = FrontmatterExcerpt::capture_line(FENCE_DOC, 1, false)
            .expect("near-miss frontmatter block");
        let ctx = biscuit_terminal::errors::SourceContext::new(
            PathBuf::from("metadata.md"),
            PathBuf::from("metadata.md"),
            FENCE_DOC.to_string(),
        );
        let md = MarkdownError::FrontmatterFenceMismatch {
            ctx,
            found: "----".to_string(),
            line: 1,
        };
        let err = CompositionError::WithFrontmatter {
            inner: Box::new(CompositionError::FrontmatterParse(md)),
            excerpt,
        };
        let report: Report = eyre!(err);

        let mut term = Terminal::builder()
            .width(80)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);

        let rendered = try_render_block_report(&report, &term).expect("block error found");
        assert!(
            !rendered.contains('\x1b'),
            "plain render must contain no escape byte; got: {rendered:?}"
        );
        // The diagnostic text and actionable hint still reach the user.
        assert!(
            rendered.contains("frontmatter fence mismatch"),
            "missing diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("Use exactly three dashes"),
            "missing fix hint: {rendered}"
        );
        // The captured excerpt block (rendered as part of the MarkdownError
        // status block, not the TTY-only Claudine appendix) still pinpoints the
        // offending fence line so the user can act on it.
        assert!(rendered.contains("> 1 │ ----"), "missing fence highlight: {rendered}");
    }
}
