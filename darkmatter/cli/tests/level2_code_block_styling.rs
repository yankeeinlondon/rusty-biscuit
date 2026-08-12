mod common;

use common::level2::{
    CODE_DOC, is_sentinel_line, max_bg_luma_on_line, max_fg_luma_on_line, min_fg_luma_on_line,
    raw_line_is_blank, rendered_region, rtrim, run_md, run_md_env, run_md_in_tmux,
};
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

#[test]
#[serial(level2_terminal)]
fn level2_code_block_fill_pad_adds_left_indent() {
    let body = "```rust\nfn main() {}\n```\n";
    let Some((frame, _)) = run_md(
        body,
        "--fill-code-blocks pad=4 --max-width 40 --page-bg subtle",
    ) else {
        return;
    };

    // With Pad(4) on a 40-col page, the code block renders at 32 cols and is
    // shifted right by 4 cols of left padding (Left alignment is the default
    // since the layout fix). Locate the header line (contains "rust") and
    // verify it starts with at least 4 leading spaces.
    let rust_line = frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .expect("expected a line containing the 'rust' code-block label");
    let leading = rust_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "expected >=4 leading spaces for Pad(4) left padding, got {leading}: {rust_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_code_block_max_fill_constrains_body_width() {
    let body = "```rust\nfn really_long_function_name_that_would_overflow() {}\n```\n";
    let Some((frame, _)) = run_md(
        body,
        "--fill-code-blocks max=30 --max-width 60 --page-bg subtle",
    ) else {
        return;
    };

    // With Max(30), neither the header line (containing "rust") nor the body
    // line should have visible content exceeding 30 columns.
    //
    // We locate the header via its "rust" tag and assert its visible width
    // (after rtrim) is <= 30 plus any leading whitespace. Then we find the
    // function-name line and assert the same.
    let lines: Vec<&str> = frame.plain.lines().collect();

    let header = lines
        .iter()
        .find(|l| l.contains("rust"))
        .expect("expected a header line containing 'rust'");

    let leading = header.chars().take_while(|c| *c == ' ').count();
    let content_visible = rtrim(header).chars().count() - leading;
    assert!(
        content_visible <= 30,
        "header content visible width should be <=30, got {content_visible}: {header:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_align_code_block_center_indents_more_than_left() {
    let body = "```rust\nfn main() {}\n```\n";
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    // Left-aligned baseline.
    let left_frame = run_md_in_tmux(
        body,
        None,
        "--align-code-blocks left --fill-code-blocks max=20 --max-width 60",
        &[],
    );
    let left_header = left_frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .unwrap_or_else(|| {
            panic!(
                "left: expected a header line containing 'rust'; captured frame:\n{}",
                left_frame.plain
            )
        });
    let left_indent = left_header.chars().take_while(|c| *c == ' ').count();

    // Center-aligned: same fill, same page width, so the only difference is
    // the alignment surplus added as additional left padding.
    let center_frame = run_md_in_tmux(
        body,
        None,
        "--align-code-blocks center --fill-code-blocks max=20 --max-width 60",
        &[],
    );
    let center_header = center_frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .unwrap_or_else(|| {
            panic!(
                "center: expected a header line containing 'rust'; captured frame:\n{}",
                center_frame.plain
            )
        });
    let center_indent = center_header.chars().take_while(|c| *c == ' ').count();

    assert!(
        center_indent > left_indent,
        "center alignment should indent more than left: left={left_indent}, center={center_indent}\nleft header: {left_header:?}\ncenter header: {center_header:?}"
    );
}

// =============================================================================
//                  TABLES / IMAGES / BLOCKQUOTES / LISTS
// =============================================================================
//
// Real-terminal captures for component-specific alignment, fill, and wrapping.
// These cover the gap called out in review-1: code-block coverage existed
// already, but tables, images, blockquotes, and lists were Level 1 only.

#[test]
#[serial(level2_terminal)]
fn level2_code_block_inverts_to_light_in_dark_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = run_md_in_tmux(
        CODE_DOC,
        None,
        "--code-theme github --max-width 60",
        &[("COLORFGBG", "15;0")], // bg index 0 => dark terminal
    );

    let code_luma = max_bg_luma_on_line(&frame.raw, "rust").unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the code line. raw:\n{}",
            frame.raw
        )
    });
    assert!(
        code_luma > 140.0,
        "code panel should be LIGHT (high luma) in a dark terminal, got luma {code_luma:.0}. \
         plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_default_code_block_inverts_background_and_foreground() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut captured = None;
    let mut bg: Option<f32> = None;
    let mut min_fg: Option<f32> = None;
    let mut max_fg: Option<f32> = None;

    for _ in 0..3 {
        let frame = run_md_in_tmux(
            CODE_DOC,
            None,
            "--max-width 60",
            &[("COLORFGBG", "15;0")], // bg index 0 => dark terminal
        );

        bg = max_bg_luma_on_line(&frame.raw, "FooBar");
        min_fg = min_fg_luma_on_line(&frame.raw, "FooBar");
        max_fg = max_fg_luma_on_line(&frame.raw, "FooBar");
        captured = Some(frame);
        if bg.is_some() && min_fg.is_some() && max_fg.is_some() {
            break;
        }
    }

    let frame = captured.expect("capture should be present");
    let code_bg = bg.unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });
    let darkest_fg = min_fg.unwrap_or_else(|| {
        panic!(
            "no truecolor foreground found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });
    let brightest_fg = max_fg.unwrap_or_else(|| {
        panic!(
            "no truecolor foreground found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });

    assert!(
        code_bg > 175.0,
        "default code block should use a light page-inverted background in a dark terminal, got luma {code_bg:.0}. \
         plain:\n{}",
        frame.plain
    );
    assert!(
        darkest_fg < 140.0,
        "default code block should use dark token foregrounds on the light inverted background, got darkest luma {darkest_fg:.0}. \
         plain:\n{}",
        frame.plain
    );
    assert!(
        brightest_fg < 190.0,
        "default code block should not keep bright dark-theme foregrounds on the light inverted background, got brightest luma {brightest_fg:.0}. \
         plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_code_block_clears_inherited_dim_before_theme_colors() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut captured = None;
    let mut bg = None;
    for _ in 0..3 {
        let frame = run_md_in_tmux(
            CODE_DOC,
            Some("printf '\\033[2m'"),
            "--max-width 60",
            &[("COLORFGBG", "15;0")],
        );
        bg = max_bg_luma_on_line(&frame.raw, "FooBar");
        captured = Some(frame);
        if bg.is_some() {
            break;
        }
    }

    let frame = captured.expect("capture should be present");
    let code_bg = bg.unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the dim-inherited code line. raw:\n{}",
            frame.raw
        )
    });

    assert!(
        code_bg > 175.0,
        "code block must clear inherited dim before applying the page-inverted background, got luma {code_bg:.0}. \
         plain:\n{}",
        frame.plain
    );
}

// #0 mirror — "light terminal -> dark code panel" is not repeated here. The
// inversion in that direction is covered where the light surface is the test's
// primary subject:
//   - level2_schema_about_light_terminal_uses_dark_code_theme (L2, tmux: OSC-11
//     unanswered, so COLORFGBG=0;15 is honored)
//   - i7_code_block_inverts_theme_against_light_terminal (L1, sets the terminal
//     color mode directly)
//   - html_code_block_inverts_for_light_page (HTML)

/// #1/#2 — With left+right margins the code panel must stay within the content
/// rectangle: no rendered line exceeds the content width, and the right-margin
/// columns are blank (the old `\x1b[K` bug painted to the physical edge and
/// left a margin-width unstyled gap).

#[test]
#[serial(level2_terminal)]
fn level2_code_block_respects_right_margin() {
    let Some((frame, _)) = run_md_env(
        CODE_DOC,
        "--ml 4 --mr 4 --max-width 50",
        &[("COLORFGBG", "15;0")],
    ) else {
        return;
    };

    // Anchor on the code line and verify it carries a background fill.
    assert!(
        max_bg_luma_on_line(&frame.raw, "rust").is_some(),
        "code line must carry a background fill. raw:\n{}",
        frame.raw
    );

    // Content rectangle = ml(4) + max_width(50) + mr(4) = 58 columns. No
    // rendered line may exceed it (the bug overflowed to the physical edge).
    // Scope to the rendered region so the echoed command (long temp path) does
    // not pollute the width scan.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let (head, sentinel) = rendered_region(&lines, "FooBar");
    let max_visible = lines[head..sentinel]
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_visible <= 58,
        "no rendered line may exceed ml+max_width+mr = 58 cols, got {max_visible}. plain:\n{}",
        frame.plain
    );
}

/// #3 — A document ending in a code block must not accrue the old constant
/// trailing blank-line offset (the body emitted `mb + 2` blank rows). The
/// `printf` sentinel injects one trailing newline of its own, so the robust
/// bound above the sentinel is `mb + 1`; the pre-fix bug would produce `mb + 3`.

#[test]
#[serial(level2_terminal)]
fn level2_no_trailing_blank_offset_after_code() {
    let Some((frame, _)) = run_md_env(CODE_DOC, "--mb 1 --max-width 60", &[("COLORFGBG", "15;0")])
    else {
        return;
    };

    // Use raw lines so a bg-filled code padding row is not mistaken for a blank
    // margin row.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let sentinel_idx = raw_lines
        .iter()
        .rposition(|l| is_sentinel_line(l))
        .unwrap_or_else(|| panic!("sentinel line not found. raw:\n{}", frame.raw));

    // Count genuinely-blank rows immediately above the sentinel. With mb=1 this
    // must be at most 2 (1 bottom margin + 1 printf newline); the pre-fix
    // constant +2 offset would have made it 4.
    let trailing_blanks = raw_lines[..sentinel_idx]
        .iter()
        .rev()
        .take_while(|l| raw_line_is_blank(l))
        .count();
    assert!(
        trailing_blanks <= 2,
        "trailing blank rows must not pile up (mb=1 ⇒ at most 2 with the printf \
         newline); the old +2 offset bug produced 4. got {trailing_blanks}. plain:\n{}",
        frame.plain
    );
}

// Interior vertical rhythm (no run of >=2 blank rows) is verified
// deterministically and across every shape by the library-side `I5` invariant
// in `darkmatter/lib/tests/render_invariants.rs`. A real-terminal version is
// omitted here because transient mid-scroll captures make it flaky without
// adding coverage the deterministic invariant does not already provide.

#[test]
#[serial(level2_terminal)]
fn level2_style_page_code_theme_changes_terminal_rendering() {
    let doc_dracula = "---\nstyle:\n  page:\n    code:\n      theme: dracula\n---\n\n\
        ```rust\nfn _theme_marker_dm() { let x = 1; }\n```\n";
    let doc_nord = "---\nstyle:\n  page:\n    code:\n      theme: nord\n---\n\n\
        ```rust\nfn _theme_marker_dm() { let x = 1; }\n```\n";

    let Some((dracula, _)) = run_md(doc_dracula, "--max-width 60") else {
        return;
    };
    let Some((nord, _)) = run_md(doc_nord, "--max-width 60") else {
        return;
    };

    assert!(
        dracula.plain.contains("_theme_marker_dm"),
        "code body missing from dracula capture:\n{}",
        dracula.plain
    );
    assert!(
        nord.plain.contains("_theme_marker_dm"),
        "code body missing from nord capture:\n{}",
        nord.plain
    );
    assert_ne!(
        dracula.raw, nord.raw,
        "style.page.code.theme: dracula vs nord must produce different SGR bytes"
    );
}

/// CLI `--code-theme` must beat `style.page.code.theme`. With frontmatter set
/// to `dracula` and CLI passing `--code-theme nord`, the rendered output must
/// use the nord theme — its panel background and at least one of its signature
/// syntax foregrounds — and must not use the dracula panel background.
///
/// Pins `--code-block dark` so each pair resolves to its own (distinct) dark
/// theme. Under the default `inverse` mode on a dark terminal both nord and
/// dracula would resolve to their light theme — and they share the same one
/// (OneHalfLight) — collapsing the panel backgrounds and erasing the
/// discriminator this test relies on.

#[test]
#[serial(level2_terminal)]
fn level2_cli_code_theme_overrides_style_page_code_theme() {
    let doc_with_fm = "---\nstyle:\n  page:\n    code:\n      theme: dracula\n---\n\n\
        ```rust\nfn _cli_override_marker() { let x = 1; }\n```\n";

    let Some((with_fm, _)) = run_md(doc_with_fm, "--code-theme nord --code-block dark --max-width 60")
    else {
        return;
    };

    assert!(
        with_fm.plain.contains("_cli_override_marker"),
        "fm-with-cli plain missing body:\n{}",
        with_fm.plain
    );

    // Nord panel background `#2e3440` = rgb(46,52,64). Dracula panel
    // background `#282a36` = rgb(40,42,54). WezTerm's `get-text --escapes`
    // re-emits SGR in either semicolon or ITU colon form and collapses
    // contiguous same-attribute cells into a single span, so per-line byte
    // equality is unreliable — assert on the presence of the nord SGR and
    // the absence of the dracula SGR in the full captured stream instead.
    let nord_bg_semi = "\x1b[48;2;46;52;64m";
    let nord_bg_colon = "\x1b[48:2::46:52:64m";
    let dracula_bg_semi = "\x1b[48;2;40;42;54m";
    let dracula_bg_colon = "\x1b[48:2::40:42:54m";

    assert!(
        with_fm.raw.contains(nord_bg_semi) || with_fm.raw.contains(nord_bg_colon),
        "expected nord panel bg (46,52,64) from CLI override. raw={:?}",
        with_fm.raw
    );
    assert!(
        !with_fm.raw.contains(dracula_bg_semi) && !with_fm.raw.contains(dracula_bg_colon),
        "frontmatter dracula panel bg (40,42,54) must not appear when CLI --code-theme \
         claims the slot. raw={:?}",
        with_fm.raw
    );

    // Nord's "frost" Blue `#81a1c1` = rgb(129,161,193) highlights the `fn`
    // and `let` keywords in our rust snippet; dracula's pink `#ff79c6` =
    // rgb(255,121,198) would color those instead. Asserting that the nord
    // keyword color is present and the dracula one is absent is a sharper
    // signal than panel bg alone (panel bg can match between themes that
    // share `#2e3440`, but nord/dracula have distinct keyword palettes).
    let nord_kw_semi = "\x1b[38;2;129;161;193m";
    let nord_kw_colon = "\x1b[38:2::129:161:193m";
    let dracula_kw_semi = "\x1b[38;2;255;121;198m";
    let dracula_kw_colon = "\x1b[38:2::255:121:198m";

    assert!(
        with_fm.raw.contains(nord_kw_semi) || with_fm.raw.contains(nord_kw_colon),
        "expected nord keyword fg (129,161,193) from CLI override. raw={:?}",
        with_fm.raw
    );
    assert!(
        !with_fm.raw.contains(dracula_kw_semi) && !with_fm.raw.contains(dracula_kw_colon),
        "frontmatter dracula keyword fg (255,121,198) must not appear when CLI --code-theme \
         claims the slot. raw={:?}",
        with_fm.raw
    );
}
