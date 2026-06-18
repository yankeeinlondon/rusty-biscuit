mod common;

use common::level2::run_md_built;
use serial_test::serial;

fn raw_sgr_has_attr(raw: &str, param: u32) -> bool {
    for chunk in raw.split("\x1b[").skip(1) {
        let Some(mend) = chunk.find('m') else {
            continue;
        };
        if chunk[..mend]
            .split([';', ':'])
            .filter_map(|s| s.parse::<u32>().ok())
            .any(|n| n == param)
        {
            return true;
        }
    }
    false
}

#[test]
#[serial(level2_terminal)]
fn level2_disclosure_body_renders_as_dim_italic_block_quote() {
    let body = "::disclosure\nLicense_sentinel Agreement\n::details\nKeep_sentinel your hands off.\n::end-disclosure\n";
    let Some((frame, _)) = run_md_built(body, "--max-width 60") else {
        return;
    };

    // Summary is rendered normally (no quote glyph on its line); body text is
    // present and carried inside a block quote (`│` prefix).
    assert!(
        frame.plain.contains("License_sentinel"),
        "expected disclosure summary text in capture. plain:\n{}",
        frame.plain
    );
    let body_line = frame
        .plain
        .lines()
        .find(|l| l.contains("Keep_sentinel"))
        .unwrap_or_else(|| panic!("disclosure body line missing. plain:\n{}", frame.plain));
    assert!(
        body_line.contains('│'),
        "disclosure body must render as a block quote (│ prefix), got: {body_line:?}"
    );

    // The disclosed body is the only styled content in this minimal document, so
    // a dim (SGR 2) and italic (SGR 3) attribute in the raw capture proves the
    // body styling reached the real terminal.
    assert!(
        raw_sgr_has_attr(&frame.raw, 3),
        "expected italic (SGR 3) on the disclosed body. raw:\n{}",
        frame.raw
    );
    assert!(
        raw_sgr_has_attr(&frame.raw, 2),
        "expected dim (SGR 2) on the disclosed body. raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_disclosure_honors_inline_opener_color_and_width() {
    // Inline opener tokens (`color`, `max-width`) on the `::disclosure` line
    // must reach the real terminal: the summary carries the truecolor red and
    // the narrow `max-width` wraps the body into multiple quoted lines.
    let body = "::disclosure color=red-500 max-width=24ch Inline_sentinel Title\n::details\nThis disclosed body is comfortably longer than twenty-four columns wide here.\n::end-disclosure\n";
    let Some((frame, _)) = run_md_built(body, "--max-width 70") else {
        return;
    };

    assert!(
        frame.plain.contains("Inline_sentinel"),
        "expected disclosure summary text in capture. plain:\n{}",
        frame.plain
    );

    // `max-width=24ch` forces the body to wrap, so more than one block-quoted
    // (`│`) line must appear in the visible capture.
    let quoted_lines = frame.plain.lines().filter(|l| l.contains('│')).count();
    assert!(
        quoted_lines >= 2,
        "expected max-width to wrap the body into multiple quoted lines. plain:\n{}",
        frame.plain
    );

    // Tailwind `red-500` lowers to the truecolor triple `251;44;54`. WezTerm
    // preserves the operands but may re-emit them in ITU colon form
    // (`38:2::251:44:54`), so accept either separator between the RGB values.
    assert!(
        frame.raw.contains("251;44;54") || frame.raw.contains("251:44:54"),
        "expected red-500 truecolor on the disclosure. raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_disclosure_inline_width_overrides_frontmatter_max_width() {
    // Cross-property precedence: an instance `width=60ch` must override the
    // lower-priority frontmatter `max-width: 24ch` (review-6 finding). The two
    // properties are a mutually exclusive layout choice across precedence
    // layers, so the stale 24-column cap must not survive to clamp the body.
    //
    // The body is wider than 24 columns but well under 60: with the bug the
    // stale cap wraps it to multiple quoted lines; with the fix it renders on a
    // single quoted line at the 60-column instance width.
    let body = r#"---
style:
    disclosure:
        max-width: 24ch
---

::disclosure width=60ch Inline_sentinel Title
::details
This disclosed body stays on one line at sixty wide.
::end-disclosure
"#;
    let Some((frame, _)) = run_md_built(body, "--max-width 80") else {
        return;
    };

    assert!(
        frame.plain.contains("Inline_sentinel"),
        "expected disclosure summary text in capture. plain:\n{}",
        frame.plain
    );

    // Only one block-quoted (`│`) line: the inline `width=60ch` wins, so the
    // ~52-column body is not clamped to the frontmatter 24-column cap.
    let quoted_lines = frame.plain.lines().filter(|l| l.contains('│')).count();
    assert_eq!(
        quoted_lines, 1,
        "inline width=60ch must override frontmatter max-width=24ch; body must \
         stay on a single quoted line. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_disclosure_honors_frontmatter_style_color_width_alignment() {
    // `style.disclosure.*` frontmatter must reach the real terminal through the
    // built CLI (which wires `apply_disclosure_style` + `apply_color_style`): the
    // summary carries the truecolor red, the narrow `max-width` wraps the body
    // into multiple quoted lines, and `alignment: center` indents the centered
    // block (review-5 finding #2).
    //
    // `bg-color` terminal-cell painting for disclosures is not asserted here: the
    // disclosure terminal target renders its body as a dim/italic block quote and
    // does not fill background cells for the component bucket. The browser tier
    // (`darkmatter/lib/tests/browser_render.rs`) covers component `bg-color`.
    let body = r#"---
style:
    disclosure:
        color: red-500
        max-width: 24ch
        alignment: center
---

::disclosure
Frontmatter_sentinel Title
::details
This disclosed body is comfortably longer than twenty-four columns wide here.
::end-disclosure
"#;
    let Some((frame, _)) = run_md_built(body, "--max-width 70") else {
        return;
    };

    // The summary sentinel only appears in rendered output, never in the echoed
    // command (which carries the temp path, not the file contents).
    let summary_line = frame
        .plain
        .lines()
        .find(|l| l.contains("Frontmatter_sentinel"))
        .unwrap_or_else(|| panic!("disclosure summary line missing. plain:\n{}", frame.plain));

    // `alignment: center` centers the narrow block, so the summary line carries
    // a left indent.
    let leading = summary_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 2,
        "centered disclosure summary must be indented, got {leading} leading spaces: {summary_line:?}"
    );

    // `max-width=24ch` forces the body to wrap into more than one block-quoted
    // (`│`) line.
    let quoted_lines = frame.plain.lines().filter(|l| l.contains('│')).count();
    assert!(
        quoted_lines >= 2,
        "frontmatter max-width must wrap the body into multiple quoted lines. plain:\n{}",
        frame.plain
    );

    // red-500 lowers to truecolor `251;44;54`; WezTerm may re-emit it in ITU
    // colon form.
    assert!(
        frame.raw.contains("251;44;54") || frame.raw.contains("251:44:54"),
        "expected red-500 truecolor from frontmatter on the disclosure. raw:\n{}",
        frame.raw
    );
}

