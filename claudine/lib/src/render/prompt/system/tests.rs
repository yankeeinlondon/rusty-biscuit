use super::*;
use crate::system_prompt::PreparedSystemPrompt;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use std::path::PathBuf;
use url::Url;

fn test_terminal() -> Terminal {
    Terminal::new()
}

/// Strip ANSI escape sequences (including full OSC sequences) for test
/// assertions. Delegates to biscuit-terminal's official stripper so
/// OSC8 hyperlinks are correctly removed from visible text.
fn strip_ansi_codes(s: &str) -> String {
    biscuit_terminal::discovery::eval::strip_ansi_codes(s)
}

fn test_prepared(mode: SystemPromptMode, markdown: &str) -> PreparedSystemPrompt {
    PreparedSystemPrompt {
        mode,
        source: SystemPromptSource::BuiltInNonInteractive,
        raw_text: markdown.to_string(),
        composed_markdown: markdown.to_string(),
        non_interactive_appendix: None,
    }
}

// --- Header tests ---

#[test]
fn header_contains_marker_glyph() {
    let term = test_terminal();
    let header = render_system_prompt_header("appended", &term);
    assert!(header.contains("■"));
}

#[test]
fn header_contains_action_appended() {
    let term = test_terminal();
    let header = render_system_prompt_header("appended", &term);
    assert!(header.contains("appended"));
}

#[test]
fn header_contains_action_replaced() {
    let term = test_terminal();
    let header = render_system_prompt_header("replaced", &term);
    assert!(header.contains("replaced"));
}

#[test]
fn header_contains_system_prompt_label() {
    let term = test_terminal();
    let header = render_system_prompt_header("appended", &term);
    assert!(header.contains("System Prompt"));
}

// --- Summary tests ---

#[test]
fn summary_shows_token_count() {
    let term = test_terminal();
    let summary = render_system_prompt_summary(
        &SystemPromptSource::BuiltInNonInteractive,
        SystemPromptMode::Append,
        123,
        None,
        &term,
    );
    assert!(summary.contains("123"));
    assert!(summary.contains("tokens"));
}

#[test]
fn summary_shows_builtin_source() {
    let term = test_terminal();
    let summary = render_system_prompt_summary(
        &SystemPromptSource::BuiltInNonInteractive,
        SystemPromptMode::Append,
        0,
        None,
        &term,
    );
    assert!(summary.contains("built-in"));
}

#[test]
fn summary_shows_file_source() {
    let term = test_terminal();
    let source = SystemPromptSource::StandardDiscovered {
        path: PathBuf::from("/tmp/prompt.md"),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let summary =
        render_system_prompt_summary(&source, SystemPromptMode::Append, 50, None, &term);
    assert!(summary.contains("prompt.md"));
}

#[test]
fn summary_uses_spec_prose_format_append() {
    let term = test_terminal();
    let summary = render_system_prompt_summary(
        &SystemPromptSource::BuiltInNonInteractive,
        SystemPromptMode::Append,
        321,
        None,
        &term,
    );
    let plain = strip_ansi_codes(&summary);
    assert!(plain.contains("The system prompt was"));
    assert!(plain.contains("appended to"));
    assert!(plain.contains("was _composed_ from") || plain.contains("was composed from"));
    assert!(plain.contains("The composed system prompt is roughly 321 tokens."));
}

#[test]
fn summary_uses_spec_prose_format_replace() {
    let term = test_terminal();
    let summary = render_system_prompt_summary(
        &SystemPromptSource::BuiltInNonInteractive,
        SystemPromptMode::Replace,
        77,
        None,
        &term,
    );
    let plain = strip_ansi_codes(&summary);
    assert!(plain.contains("replaced"));
    assert!(plain.contains("The replacement system prompt is roughly 77 tokens."));
}

#[test]
fn summary_relative_path_when_base_provided() {
    // Force is_nerd_font = false so we hit the `./relpath` branch
    // rather than the Nerd Font glyph branch.
    let term = plain_terminal();
    // Build a path inside the temp dir so canonicalize succeeds.
    let tmp = tempfile::tempdir().unwrap();
    let sp = tmp.path().join("system-prompt.md");
    std::fs::write(&sp, "x").unwrap();
    let source = SystemPromptSource::StandardDiscovered {
        path: sp.clone(),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let base = sp.parent().unwrap().canonicalize().unwrap();
    let summary =
        render_system_prompt_summary(&source, SystemPromptMode::Append, 10, Some(&base), &term);
    let plain = strip_ansi_codes(&summary);
    assert!(
        plain.contains("./system-prompt.md"),
        "expected `./system-prompt.md` in {plain:?}"
    );
    // The base directory must NOT appear in the visible text (only in
    // the OSC8 href, which is stripped by `strip_ansi_codes`).
    assert!(!plain.contains(base.display().to_string().as_str()));
}

// --- resolve_display_label tests ---

fn nerd_font_terminal() -> Terminal {
    let mut t = Terminal::builder().osc_link_support(true).build();
    t.is_nerd_font = Some(true);
    t
}

fn plain_terminal() -> Terminal {
    let mut t = Terminal::builder().osc_link_support(true).build();
    t.is_nerd_font = Some(false);
    t
}

#[test]
fn display_label_nerd_font_in_base_uses_glyph_with_path() {
    let term = nerd_font_terminal();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let path = base.join(".claude").join("system-prompt.md");
    let label = resolve_display_label(&path, Some(&base), &term);
    // The glyph stands in for the repo root, followed by the relative
    // path inside the repo.
    assert_eq!(
        label,
        format!("{NERD_FONT_REPO_GLYPH}/.claude/system-prompt.md")
    );
}

#[test]
fn display_label_no_nerd_font_in_base_uses_relative() {
    let term = plain_terminal();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    // Use a nested path to confirm subdir support.
    let path = base.join(".claude").join("system-prompt.md");
    let label = resolve_display_label(&path, Some(&base), &term);
    assert_eq!(label, "./.claude/system-prompt.md");
}

#[test]
fn display_label_outside_base_uses_absolute() {
    let term = plain_terminal();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    // Path that does not share the base prefix.
    let outside = std::path::PathBuf::from("/etc/hosts");
    let label = resolve_display_label(&outside, Some(&base), &term);
    assert_eq!(label, "/etc/hosts");
}

#[test]
fn display_label_no_base_uses_absolute() {
    let term = plain_terminal();
    let path = std::path::PathBuf::from("/some/abs/path.md");
    let label = resolve_display_label(&path, None, &term);
    assert_eq!(label, "/some/abs/path.md");
}

#[test]
fn display_label_nerd_font_no_base_uses_absolute() {
    // Nerd Font support alone should not trigger the glyph — the path
    // must also resolve inside `base`.
    let term = nerd_font_terminal();
    let path = std::path::PathBuf::from("/some/abs/path.md");
    let label = resolve_display_label(&path, None, &term);
    assert_eq!(label, "/some/abs/path.md");
}

#[test]
fn display_label_drive_path_uses_portable_separators() {
    let term = plain_terminal();
    let path = PathBuf::from(r"C:\repo\prompts\system-prompt.md");
    let label = resolve_display_label(&path, None, &term);
    assert_eq!(label, "C:/repo/prompts/system-prompt.md");
}

#[cfg(windows)]
#[test]
fn display_label_declined_portable_spelling_keeps_native_text() {
    let term = plain_terminal();
    let path = PathBuf::from(r"\\server\share\system-prompt.md");
    let label = resolve_display_label(&path, None, &term);
    assert_eq!(label, path.to_string_lossy());
}

#[test]
fn summary_visible_label_is_blue() {
    // The visible label (post-OSC-strip) should carry an ANSI sequence
    // for Tailwind Blue 400 (RGB 96, 165, 250) in 24-bit color mode.
    let term = Terminal::builder()
        .is_tty(true)
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(true)
        .build();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let sp = base.join("system-prompt.md");
    std::fs::write(&sp, "x").unwrap();
    let source = SystemPromptSource::StandardDiscovered {
        path: sp.clone(),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let summary =
        render_system_prompt_summary(&source, SystemPromptMode::Append, 10, Some(&base), &term);
    // Tailwind Blue 400 in 24-bit color emits ESC[38;2;81;162;255m.
    assert!(
        summary.contains("38;2;81;162;255"),
        "expected Tailwind Blue 400 RGB foreground sequence in {summary:?}"
    );
}

#[test]
fn summary_emits_osc8_for_file_link() {
    let term = Terminal::builder().osc_link_support(true).build();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let sp = base.join("system-prompt.md");
    std::fs::write(&sp, "x").unwrap();
    let source = SystemPromptSource::StandardDiscovered {
        path: sp.clone(),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let summary =
        render_system_prompt_summary(&source, SystemPromptMode::Append, 10, Some(&base), &term);
    // OSC8 opener: ESC ]8;;
    assert!(summary.contains("\x1b]8;;file://"));
    let abs = sp.canonicalize().unwrap();
    let href = Url::from_file_path(&abs).unwrap();
    assert!(summary.contains(href.as_str()));
}

#[test]
fn summary_file_uri_percent_encodes_reserved_characters() {
    let term = Terminal::builder().osc_link_support(true).build();
    let tmp = tempfile::tempdir().unwrap();
    let sp = tmp.path().join("prompt #100%.md");
    std::fs::write(&sp, "x").unwrap();
    let source = SystemPromptSource::StandardDiscovered {
        path: sp.clone(),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let summary = render_system_prompt_summary(
        &source,
        SystemPromptMode::Append,
        10,
        Some(tmp.path()),
        &term,
    );
    let href = Url::from_file_path(sp.canonicalize().unwrap()).unwrap();

    assert!(summary.contains(href.as_str()));
    assert!(href.as_str().contains("prompt%20%23100%25.md"));
}

#[test]
fn summary_declined_file_uri_falls_back_to_plain_text() {
    let term = Terminal::builder().osc_link_support(true).build();
    let path = PathBuf::from("missing prompt #100%.md");
    let source = SystemPromptSource::StandardDiscovered {
        path: path.clone(),
        scope: crate::system_prompt::StandardPromptScope::Repo,
    };
    let summary = render_system_prompt_summary(&source, SystemPromptMode::Append, 10, None, &term);

    assert!(!summary.contains("\x1b]8;;"));
    assert!(strip_ansi_codes(&summary).contains(&to_portable_string(&path)));
}

// --- Body tests ---

#[test]
fn summary_format_returns_empty_body() {
    let term = test_terminal();
    let body = render_system_prompt_body(
        "some content",
        ReportMode::Summary,
        &term,
    );
    assert!(body.is_empty());
}

#[test]
fn full_format_renders_content() {
    let term = test_terminal();
    let body = render_system_prompt_body(
        "Hello world",
        ReportMode::Full,
        &term,
    );
    let plain = strip_ansi_codes(&body);
    assert!(plain.contains("Hello world"));
}

#[test]
fn partial_format_truncates_long_text() {
    let text: String = (1..=50)
        .map(|i| format!("- Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let term = test_terminal();
    let body = render_system_prompt_body(
        &text,
        ReportMode::Partial {
            truncation: TruncationMode::FrontBack,
        },
        &term,
    );
    let plain = strip_ansi_codes(&body);
    assert!(plain.contains("Line 1"));
    // Word-wrap may split "Line 50" so that "Line " ends one row and
    // "50" starts the next. Tolerate the wrap by scanning lines for "50"
    // as a standalone token at the start of a row or after a space.
    let contains_last_line_number = plain
        .lines()
        .any(|l| l.trim_start().starts_with("50") || l.contains(" 50"));
    assert!(
        contains_last_line_number,
        "should contain the last line number: {plain:?}"
    );
    // Verify truncation happened by checking that not all lines are present.
    // "Line 25" may be wrapped as "Line \n25"; check both forms.
    let contains_line_25 =
        plain.contains("Line 25") || plain.lines().any(|l| l.trim_start().starts_with("25"));
    assert!(
        !contains_line_25,
        "middle lines should be truncated: {plain:?}"
    );
}

#[test]
fn partial_format_short_text_unchanged() {
    let text = "Line 1\nLine 2\nLine 3";
    let term = test_terminal();
    let body = render_system_prompt_body(
        text,
        ReportMode::Partial {
            truncation: TruncationMode::FrontBack,
        },
        &term,
    );
    let plain = strip_ansi_codes(&body);
    assert!(plain.contains("Line 1"));
    assert!(plain.contains("Line 2"));
    assert!(plain.contains("Line 3"));
    // Should NOT contain truncation marker for short text
    assert!(!plain.contains("---"));
}

// --- SystemPrompt direct tests ---

#[test]
fn report_silent_returns_none() {
    let prepared = test_prepared(SystemPromptMode::Append, "Test prompt");
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    assert!(SystemPrompt::from_mode(&resolved, ReportMode::Silent, None).is_none());
}

#[test]
fn report_summary_renders_header_and_summary() {
    let term = test_terminal();
    let prepared = test_prepared(SystemPromptMode::Append, "Test prompt content here.");
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Summary, None)
        .expect("should produce output");
    let output = report.render(&term);
    assert!(output.contains("■"));
    assert!(output.contains("System Prompt"));
    assert!(output.contains("appended"));
    assert!(output.contains("tokens"));
}

#[test]
fn report_full_renders_body() {
    let term = test_terminal();
    let prepared = test_prepared(SystemPromptMode::Replace, "Full prompt body.");
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Full, None)
        .expect("should produce output");
    let plain = strip_ansi_codes(&report.render(&term));
    assert!(plain.contains("■"));
    assert!(plain.contains("replaced"));
    assert!(plain.contains("Full prompt body"));
}

#[test]
fn report_partial_renders_truncated_body() {
    let text: String = (1..=50)
        .map(|i| format!("- Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let term = test_terminal();
    let prepared = test_prepared(SystemPromptMode::Append, &text);
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    let report = SystemPrompt::from_mode(
        &resolved,
        ReportMode::Partial {
            truncation: TruncationMode::FrontBack,
        },
        None,
    )
    .expect("should produce output");
    let plain = strip_ansi_codes(&report.render(&term));
    assert!(plain.contains("■"));
    assert!(plain.contains("Line 1"));
    assert!(plain.contains(" 50"), "should contain the last line number");
    assert!(!plain.contains("Line 25"), "middle lines should be truncated");
}

#[test]
fn report_none_in_summary_returns_none() {
    let resolved = ResolvedSystemPrompt::None;
    assert!(SystemPrompt::from_mode(&resolved, ReportMode::Summary, None).is_none());
}

#[test]
fn report_none_in_full_renders() {
    let term = test_terminal();
    let resolved = ResolvedSystemPrompt::None;
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Full, None)
        .expect("should produce output");
    let plain = strip_ansi_codes(&report.render(&term));
    assert!(plain.contains("none"));
    assert!(plain.contains("not been modified"));
}

#[test]
fn report_disabled_in_summary_returns_none() {
    let source = SystemPromptSource::BuiltInNonInteractive;
    let resolved = ResolvedSystemPrompt::Disabled { source };
    assert!(SystemPrompt::from_mode(&resolved, ReportMode::Summary, None).is_none());
}

#[test]
fn report_disabled_in_full_renders() {
    let term = test_terminal();
    let source = SystemPromptSource::BuiltInNonInteractive;
    let resolved = ResolvedSystemPrompt::Disabled { source };
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Full, None)
        .expect("should produce output");
    let plain = strip_ansi_codes(&report.render(&term));
    assert!(plain.contains("disabled"));
    assert!(plain.contains("been disabled"));
}

#[test]
fn summary_line_lives_inside_blockquote() {
    // Spec 6.1: the Summary sentence must sit inside the orange
    // BlockQuote (not as a bare Prose line). After the header line,
    // every subsequent non-empty line should start with the
    // BlockQuote's prefix (one-space left margin + `│ `).
    let term = test_terminal();
    let prepared = test_prepared(SystemPromptMode::Append, "Body content");
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Summary, None)
        .expect("should produce output");
    let plain = strip_ansi_codes(&report.render(&term));
    let mut lines = plain.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().expect("header line");
    assert!(header.contains("■"), "header line should contain ■");
    // At least one subsequent non-empty line must begin with the
    // BlockQuote prefix.
    let mut saw_quote = false;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            line.starts_with("┃ "),
            "expected BlockQuote prefix on body line, got {line:?}"
        );
        saw_quote = true;
    }
    assert!(saw_quote, "expected at least one BlockQuote-wrapped line");
}

#[test]
fn replace_mode_shows_replaced_action() {
    let term = test_terminal();
    let prepared = test_prepared(SystemPromptMode::Replace, "Replacement prompt.");
    let resolved = ResolvedSystemPrompt::Ready(prepared);
    let report = SystemPrompt::from_mode(&resolved, ReportMode::Summary, None)
        .expect("should produce output");
    assert!(report.render(&term).contains("replaced"));
}
