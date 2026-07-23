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

/// A `HarnessError` was invisible to the walker before Phase 5: it implemented
/// `Diagnostic`, but the walker's own downcast list only knew
/// `CompositionError`, so it could never render a block. This is spec §"Failure
/// Pattern 2" and the class of the motivating incident.
#[test]
fn renders_harness_error_block() {
    let err = claudine::harness::HarnessError::PathResolutionFailed {
        raw: "no/such/target.md".to_string(),
        failure: claudine::harness::PathResolutionFailure::TargetMissing,
        source_path: Some(PathBuf::from("route.md")),
        resolved: Some(PathBuf::from("/repo/no/such/target.md")),
        resolution: None,
    };
    let report: Report = eyre!(err);

    let rendered = try_render_block_report(&report, &width80()).expect("block error found");
    let plain = strip_escape_codes(&rendered);
    assert!(plain.contains("composition.invalid_file_reference"), "got:\n{plain}");
    assert!(plain.contains("no/such/target.md"), "got:\n{plain}");
}

/// Selection is role-based, not deepest-wins: a `Semantic` Claudine wrapper
/// stops the walk even though a renderable lower-layer cause sits below it.
///
/// The wrapper is what knows *which authored value* asked and *where*, so the
/// pre-Phase-5 deepest-wins rule would have discarded exactly the context the
/// operator needs.
#[test]
fn semantic_wrapper_wins_over_deeper_lower_layer_cause() {
    let report: Report = eyre!(invalid_file_reference());

    let rendered = try_render_block_report(&report, &width80()).expect("block error found");
    let plain = strip_escape_codes(&rendered);
    // The wrapper's authoring context, which the inner `HarnessError` lacks.
    assert!(plain.contains("route.md"), "source path missing:\n{plain}");
    assert!(plain.contains("initialize"), "event missing:\n{plain}");
    // The typed source's own detail still reaches the block through it.
    assert!(plain.contains("no/such/target.md"), "reference missing:\n{plain}");
}

/// Rendering, `err.*`, and the serialized snapshot must resolve through the
/// *same* selection — a route that classifies one cause while rendering another
/// is the defect D2/D5 exist to prevent.
#[test]
fn rendering_lifecycle_err_and_snapshot_agree_on_one_diagnostic() {
    use claudine::composition::LifecycleErrorInfo;
    use claudine::diagnostics::{DiagnosticSnapshot, select_effective_diagnostic};

    let err = invalid_file_reference();
    let selected = select_effective_diagnostic(&err)
        .and_then(|s| s.diagnostic())
        .expect("a diagnostic is selected");
    let snapshot = DiagnosticSnapshot::from_diagnostic(selected);
    let info = LifecycleErrorInfo::from_composition_error(&err);
    let embedded = info.snapshot.as_ref().expect("typed error projects a snapshot");

    // One code across all three surfaces.
    assert_eq!(embedded.code, selected.code());
    assert_eq!(snapshot.code, selected.code());
    // `err.*` embeds exactly the snapshot the selection produced — one
    // projection, not a parallel rebuild that could drift.
    assert_eq!(**embedded, snapshot);
    // One message across `err.msg` and the snapshot.
    assert_eq!(info.msg, snapshot.message);
    // And the terminal rendering is that same diagnostic's own block — not a
    // wrapper's flattened `Display`, and not a deeper cause's block.
    let term = width80();
    let report: Report = eyre!(invalid_file_reference());
    assert_eq!(
        try_render_block_report(&report, &term).expect("block"),
        selected.report_block_error(&term),
    );
}

/// The motivating incident's error shape: a composition semantic wrapper over a
/// typed `HarnessError` resolution failure.
fn invalid_file_reference() -> CompositionError {
    CompositionError::InvalidFileReference {
        context: Box::new(claudine::composition::FileReferenceContext {
            source_path: PathBuf::from("route.md"),
            event: Some("initialize".to_string()),
            property: "initialize.stack[0].proxy".to_string(),
            reference: "no/such/target.md".to_string(),
            hint: "A `proxy` target must name an existing Markdown document."
                .to_string(),
        }),
        source: claudine::harness::HarnessError::PathResolutionFailed {
            raw: "no/such/target.md".to_string(),
            failure: claudine::harness::PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("route.md")),
            resolved: Some(PathBuf::from("/repo/no/such/target.md")),
            resolution: None,
        },
    }
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

const SCHEMA_DOC: &str =
    "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nbody\n";

fn schema_parse_with_frontmatter(stderr_is_tty: bool) -> CompositionError {
    let excerpt = FrontmatterExcerpt::capture_schema_span(
        SCHEMA_DOC,
        Some("$schema.spec"),
        14,
        stderr_is_tty,
    )
    .expect("frontmatter block");
    CompositionError::WithFrontmatter {
        inner: Box::new(CompositionError::SchemaParse {
            source_path: PathBuf::from("review.md"),
            property: Some("spec".to_string()),
            message: "expected `;` between constraints".to_string(),
            span: Some(14..15),
        }),
        excerpt,
    }
}

#[test]
fn schema_parse_appends_highlighted_frontmatter_and_links_file_on_tty() {
    let report: Report = eyre!(schema_parse_with_frontmatter(true));
    let term = width80();
    let rendered = try_render_block_report(&report, &term).expect("block error found");
    let plain = strip_escape_codes(&rendered);

    // The primary diagnostic still renders from the inner SchemaParse error.
    assert!(plain.contains("invalid schema"), "header missing:\n{plain}");
    assert!(plain.contains("spec"), "property name missing:\n{plain}");
    // The appended excerpt shows the offending `$schema.spec` line.
    assert!(
        plain.contains("spec: file(required, match(**/*spec*.md))"),
        "offending frontmatter line missing:\n{plain}"
    );
    // The OSC8 link to the prompt file survives at optimistic color depth.
    assert!(
        rendered.contains("\u{1b}]8;;"),
        "expected an OSC8 link to the prompt file; raw:\n{rendered}"
    );
}

#[test]
fn schema_parse_withholds_frontmatter_block_when_not_tty() {
    let report: Report = eyre!(schema_parse_with_frontmatter(false));
    let rendered = try_render_block_report(&report, &width80()).expect("block error found");
    let plain = strip_escape_codes(&rendered);
    assert!(plain.contains("invalid schema"), "header missing:\n{plain}");
    // Non-TTY output must not expose the frontmatter body.
    assert!(
        !plain.contains("file(required, match"),
        "frontmatter excerpt leaked to non-tty:\n{plain}"
    );
}

#[test]
fn renders_lifecycle_evaluation_error_for_success_when() {
    // A `success.when` guard that *raised* (vs. cleanly evaluating to
    // `false`) renders the styled lifecycle-evaluation block: the event
    // name, the offending surface, the raised reason, and — critically —
    // text that distinguishes a crashed guard from a clean false guard.
    let err = CompositionError::LifecycleEvaluationError {
        source_path: PathBuf::from("review.md"),
        event: "success".to_string(),
        surface: "when".to_string(),
        message: "frontmatter(review_file,'ready') raised: path did not resolve".to_string(),
    };
    let report: Report = eyre!(err);
    let rendered = try_render_block_report(&report, &width80()).expect("block error found");
    let plain = strip_escape_codes(&rendered);

    assert!(plain.contains("lifecycle evaluation error"), "header missing:\n{plain}");
    assert!(plain.contains("success"), "event name missing:\n{plain}");
    assert!(plain.contains("when:"), "surface label missing:\n{plain}");
    assert!(plain.contains("did not resolve"), "reason missing:\n{plain}");
    // Distinguish a crashed guard from a clean false guard.
    assert!(
        plain.contains("crashed expression") && plain.contains("false"),
        "crashed-vs-false distinction missing:\n{plain}"
    );
}

#[test]
fn renders_lifecycle_evaluation_error_for_finalize() {
    // An evaluation error raised *inside* `finalize` itself is still
    // surfaced (visible, not swallowed). Non-recursion is enforced in the
    // orchestrator (see `loop_control` L1 tests); here we prove the
    // user-facing render is produced.
    let err = CompositionError::LifecycleEvaluationError {
        source_path: PathBuf::from("review.md"),
        event: "finalize".to_string(),
        surface: "interpolation".to_string(),
        message: "unknown root `missing_root`".to_string(),
    };
    let report: Report = eyre!(err);
    let rendered = try_render_block_report(&report, &width80()).expect("block error found");
    let plain = strip_escape_codes(&rendered);

    assert!(plain.contains("lifecycle evaluation error"), "header missing:\n{plain}");
    assert!(plain.contains("finalize"), "event name missing:\n{plain}");
    assert!(plain.contains("interpolated string"), "surface label missing:\n{plain}");
    assert!(plain.contains("missing_root"), "reason missing:\n{plain}");
}

#[test]
fn lifecycle_evaluation_error_is_plain_without_color() {
    // Non-TTY / NO_COLOR follows the existing CLI error rendering
    // convention: `report_block_error` strips escapes at `ColorDepth::None`.
    let err = CompositionError::LifecycleEvaluationError {
        source_path: PathBuf::from("review.md"),
        event: "success".to_string(),
        surface: "when".to_string(),
        message: "boom".to_string(),
    };
    let report: Report = eyre!(err);
    let mut term = Terminal::new_optimistic(80);
    term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
    let rendered = try_render_block_report(&report, &term).expect("block error found");
    assert!(
        !rendered.contains('\u{1b}'),
        "ANSI escapes leaked into NO_COLOR output:\n{rendered}"
    );
    assert!(rendered.contains("lifecycle evaluation error"), "got:\n{rendered}");
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
