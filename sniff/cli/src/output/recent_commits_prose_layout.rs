//! Terminal-integration regressions for the recent-commits Prose passthrough.
//!
//! Decision 12 of `sniff/features/2026-09-15-recent-commits/spec.md` makes the
//! library author every report byte as Prose and the CLI render it with a
//! single `Prose` call. These render real `RecentCommits::to_prose` output.
//! A deserialized collection has no repository root, so its files are
//! unlinked; `cli.rs` covers `file://` links through a real repository.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::color::ColorDepth;
use biscuit_terminal::prelude::{WordWrap, strip_escape_codes};
use biscuit_terminal::terminal::Terminal;
use chrono::FixedOffset;
use sniff::filesystem::git::{RecentCommits, RecentCommitsOptions, RecentCommitsVerbosity};

const COMMIT_URL: &str = "https://github.com/o/r/commit/abc1234def";

/// Dated well before any test run so the label is an absolute date.
const PAYLOAD: &str = r#"[
  {
    "hash": "abc1234def", "datetime": "2026-09-15T10:42:00+00:00",
    "author": {"name": "Ada", "email": "ada@example.com"},
    "operation": "feat", "scope": "sniff", "heading": "add recent commits redesign",
    "description": "The library now owns selection, filtering, and linking for every output format so the CLI stays a thin passthrough.",
    "bullet_points": ["bullet one with `code`", "bullet two with a long line that should wrap when the terminal is narrow enough to force wrapping"],
    "files": [
      {"kind": "modified", "path": "sniff/lib/src/lib.rs"},
      {"kind": "moved", "path": "sniff/lib/src/new.rs", "original_path": "sniff/lib/src/old.rs"}
    ],
    "file_types": {"source_code": true, "web_assets": false, "images": false, "documentation": false, "configuration": false, "cicd": false},
    "remote": true, "commit_url": "https://github.com/o/r/commit/abc1234def"
  },
  {
    "hash": "def5678abc", "datetime": "2026-09-14T09:00:00+00:00",
    "author": {"name": "Ada", "email": "ada@example.com"},
    "operation": "fix", "scope": null, "heading": "repair OPENCODE_CONFIG_CONTENT handling <red>now</red> and _em_",
    "description": "", "bullet_points": [],
    "files": [{"kind": "added", "path": "docs/a_b_c.md"}],
    "file_types": {"source_code": false, "web_assets": false, "images": false, "documentation": true, "configuration": false, "cicd": false},
    "remote": false
  }
]"#;

/// The visible text a reader should see, line for line.
const VISIBLE: &str = "\
- [abc1234] feat(sniff) at 10:42am 2026-09-15: add recent commits redesign
  The library now owns selection, filtering, and linking for every output format so the CLI stays a thin passthrough.

  Details:

  - bullet one with `code`
  - bullet two with a long line that should wrap when the terminal is narrow enough to force wrapping

  Files Impacted:
  - modified: sniff/lib/src/lib.rs
  - moved: sniff/lib/src/new.rs (from sniff/lib/src/old.rs)

- [def5678] fix at 9:00am 2026-09-14: repair OPENCODE_CONFIG_CONTENT handling <red>now</red> and _em_
  Files Impacted:
  - added: docs/a_b_c.md";

fn report() -> String {
    let commits: RecentCommits = serde_json::from_str(PAYLOAD).unwrap();
    let options = RecentCommitsOptions::new()
        .verbosity(RecentCommitsVerbosity::Verbose)
        .timezone(FixedOffset::east_opt(0).unwrap());
    commits.to_prose(&options)
}

fn wide_terminal() -> Terminal {
    Terminal::new_optimistic(400)
}

fn visible_lines(rendered: &str) -> Vec<String> {
    strip_escape_codes(rendered)
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

#[test]
fn single_render_preserves_every_line_and_blank_line_without_inflation() {
    let rendered = Prose::new(report()).render(&wide_terminal());
    let expected: Vec<&str> = VISIBLE.lines().collect();
    assert_eq!(visible_lines(&rendered), expected);
}

#[test]
fn escaped_markup_in_commit_text_renders_literally() {
    let rendered = Prose::new(report()).render(&wide_terminal());
    let visible = strip_escape_codes(&rendered);
    assert!(
        visible.contains("handling <red>now</red> and _em_"),
        "{visible}"
    );
    assert!(!visible.contains('\\'), "escape backslashes must be consumed: {visible}");
    assert!(!rendered.contains("\x1b[31m"), "commit text must not become red: {rendered:?}");
}

#[test]
fn single_render_emits_styles_and_osc8_links_when_supported() {
    let rendered = Prose::new(report()).render(&wide_terminal());
    assert!(rendered.contains("\x1b[1mabc1234"), "{rendered:?}");
    assert!(rendered.contains("\x1b[34mfeat"), "{rendered:?}");
    assert!(rendered.contains("\x1b[3mat"), "{rendered:?}");
    assert!(rendered.contains("\x1b[1mDetails:"), "{rendered:?}");
    assert!(rendered.contains(&format!("\x1b]8;;{COMMIT_URL}\x1b\\")), "{rendered:?}");
    assert!(!rendered.contains("<bold>"), "prose tags must be consumed: {rendered:?}");
}

#[test]
fn links_fall_back_to_markdown_without_osc8_support() {
    let mut terminal = wide_terminal();
    terminal.osc_link_support = false;
    let rendered = Prose::new(report()).render(&terminal);

    assert!(!rendered.contains("\x1b]8;;"), "{rendered:?}");
    let visible = strip_escape_codes(&rendered);
    assert!(
        visible.contains(&format!("- [[abc1234]({COMMIT_URL})] feat(sniff)")),
        "{visible}"
    );
    assert!(visible.contains("- [def5678] fix at"), "unlinked hash: {visible}");
}

#[test]
fn colorless_terminal_keeps_every_visible_line() {
    let mut terminal = wide_terminal();
    terminal.color_depth = ColorDepth::None;
    terminal.osc_link_support = true;
    let rendered = Prose::new(report()).render(&terminal);
    let expected: Vec<&str> = VISIBLE.lines().collect();
    assert_eq!(visible_lines(&rendered), expected);
}

/// `WrapProse(_, Some(n))` is unsuitable: its hanging indent applies to every
/// line after the first line of the whole document, not per source line, so
/// it shifts the second commit header and every indented line.
#[test]
fn word_wrap_layout_keeps_source_lines_and_wraps_within_width() {
    let width = 60;
    let rendered = Prose::new(report())
        .with_word_wrap(WordWrap::WrapProse(None, None))
        .render(&Terminal::new_optimistic(width));
    let lines = visible_lines(&rendered);

    for line in &lines {
        assert!(
            line.chars().count() <= width as usize,
            "line exceeds {width} columns: {line:?}"
        );
    }
    let expected_words: Vec<&str> = VISIBLE.split_whitespace().collect();
    let rendered_text = lines.join("\n");
    let rendered_words: Vec<&str> = rendered_text.split_whitespace().collect();
    assert_eq!(rendered_words, expected_words);

    // Each source line (with its indentation) starts a rendered line, in
    // order; wrapping only appends continuation lines between them.
    let mut rendered_iter = lines.iter();
    for source_line in VISIBLE.lines() {
        let first_word_run: String = source_line.chars().take(12).collect();
        assert!(
            rendered_iter.any(|line| line.starts_with(&first_word_run)),
            "source line lost its own rendered line: {source_line:?}\n{rendered_text}"
        );
    }
    let blank_rendered = lines.iter().filter(|line| line.is_empty()).count();
    let blank_source = VISIBLE.lines().filter(|line| line.is_empty()).count();
    assert_eq!(blank_rendered, blank_source);
    assert!(lines.len() > VISIBLE.lines().count(), "fixture must actually wrap");
}

#[test]
fn colorless_terminal_drops_color_and_emphasis_sequences() {
    let mut terminal = wide_terminal();
    terminal.color_depth = ColorDepth::None;
    let rendered = Prose::new("<blue>feat</blue> <bold>10:42</bold>").render(&terminal);
    assert_eq!(rendered, "feat 10:42");
}
