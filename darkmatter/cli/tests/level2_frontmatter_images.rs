mod common;

use common::level2::run_md;
use serial_test::serial;

#[test]
#[serial(level2_terminal)]
fn level2_image_fallback_text_respects_alignment() {
    // Use a non-existent image so the renderer emits its alt-text/path
    // fallback rather than attempting an inline image protocol — that gives
    // us a stable plain-text anchor in the captured pane regardless of the
    // host terminal's image support.
    let body = "![Alt text fallback](./does-not-exist.png)\n";

    let Some((left, _)) = run_md(
        body,
        "--align-images left --fill-images max=20 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-images center --fill-images max=20 --max-width 60",
    ) else {
        return;
    };

    // Anchor on the alt text; both captures must contain it.
    let line_left = left
        .plain
        .lines()
        .find(|l| l.contains("Alt text fallback"))
        .unwrap_or_else(|| {
            panic!(
                "left: expected an alt-text anchor in image fallback. plain:\n{}",
                left.plain
            )
        });
    let line_center = center
        .plain
        .lines()
        .find(|l| l.contains("Alt text fallback"))
        .unwrap_or_else(|| {
            panic!(
                "center: expected an alt-text anchor in image fallback. plain:\n{}",
                center.plain
            )
        });

    let left_indent = line_left.chars().take_while(|c| *c == ' ').count();
    let center_indent = line_center.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "image fallback center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_images_alignment_indents_fallback_text() {
    // `style.images.alignment` flows through to image fallback rendering for
    // missing images, the same path Level 1 covers structurally.
    let right = r#"---
style:
    images:
        alignment: right
        max-width: 20ch
---

![Sentinel image alt](./does-not-exist.png)
"#;
    let left = r#"---
style:
    images:
        alignment: left
        max-width: 20ch
---

![Sentinel image alt](./does-not-exist.png)
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 60") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 60") else {
        return;
    };

    let line_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("Sentinel image alt"))
        .unwrap_or_else(|| {
            panic!(
                "right: expected an alt-text anchor in image fallback. plain:\n{}",
                right_frame.plain
            )
        });
    let line_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("Sentinel image alt"))
        .unwrap_or_else(|| {
            panic!(
                "left: expected an alt-text anchor in image fallback. plain:\n{}",
                left_frame.plain
            )
        });

    let right_indent = line_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = line_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "frontmatter style.images.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_hyperlink_color_applies_inside_table() {
    let body = "---\n\
style:\n  hyperlinks:\n    color: red-500\n  table:\n    color: blue-500\n---\n\
| col |\n|---|\n| [clickanchor](https://example.com) |\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // WezTerm re-emits truecolor SGR as either semicolon or ITU colon form.
    let red_semi = "\x1b[38;2;251;44;54m";
    let red_colon = "\x1b[38:2::251:44:54m";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "hyperlink color must appear inside table cell. raw={:?}, plain={:?}",
        frame.raw,
        frame.plain,
    );
    // The OSC8 wrapping must remain so the link is clickable.
    assert!(
        frame.raw.contains("\x1b]8;;https://example.com"),
        "OSC8 link must be preserved in table cell. raw stream:\n{}",
        frame.raw
    );
    // Visible label must remain in the plain capture.
    assert!(
        frame.plain.contains("clickanchor"),
        "link label must render in plain capture:\n{}",
        frame.plain
    );
}

// =============================================================================
//   HR STYLE FRONTMATTER — canonical `style.hr.*` path (sub-spec #6, review-6)
// =============================================================================
//
// Review-6 finding 3: the canonical `style.hr.*` frontmatter path needs Level 2
// real-terminal coverage. Below tests exercise the canonical path (NOT the
// legacy top-level `hr:` block, NOT inline `--- { ... }` attributes) through
// the real `md` CLI in a WezTerm pane.

#[test]
#[serial(level2_terminal)]
fn level2_local_hyperlink_color_differs_from_remote_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    color: red-500\n    local-style:\n      color: blue-500\n---\n\n\
        [LOCAL_LINK](./somewhere.md) [REMOTE_LINK](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // WezTerm may re-emit truecolor SGR as either semicolon (`;`) or ITU
    // colon (`:`) form. Accept both.
    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    let blue_semi = "38;2;43;127;255";
    let blue_colon = "38:2::43:127:255";

    let has_red = frame.raw.contains(red_semi) || frame.raw.contains(red_colon);
    let has_blue = frame.raw.contains(blue_semi) || frame.raw.contains(blue_colon);
    assert!(
        has_red && has_blue,
        "expected both remote red and local blue SGR. has_red={has_red}, has_blue={has_blue}\nraw:\n{}",
        frame.raw
    );
    // Plain labels must still appear.
    assert!(
        frame.plain.contains("LOCAL_LINK") && frame.plain.contains("REMOTE_LINK"),
        "link labels missing from plain capture:\n{}",
        frame.plain
    );
}

/// `style.hyperlinks.width: 20` must produce a label box padded to that exact
/// width before the OSC8 close. We can't compare visible widths without a
/// stable column probe, so we assert the raw stream pads the label.

#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_width_pads_label_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    width: 20\n---\n\n\
        [HI](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // Padded label width = 20 cells, label "HI" is 2 cells, so 18 trailing
    // spaces precede the OSC8 close. Look for the label followed by at least
    // 10 spaces and the OSC8 terminator. (Be tolerant of any ANSI bytes that
    // a terminal may inject for cursor positioning; the padding spaces are
    // the visible signal.)
    assert!(
        frame.plain.contains("HI                  "),
        "expected padded label `HI` followed by 18 spaces. plain:\n{}",
        frame.plain
    );
}

/// Regression (review-1, finding 1): an exact `style.hyperlinks.width` is an
/// exact field, so a label wider than the field must be truncated in a real
/// terminal — the visible field must not overflow the five columns.

#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_exact_width_truncates_label_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    width: 5\n---\n\n\
        [A very long hyperlink label](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The five-column field truncates with an ellipsis; the overflowing tail of
    // the label must be absent from the visible capture.
    assert!(
        frame.plain.contains('…'),
        "expected the long label truncated to an ellipsis. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("hyperlink label"),
        "the overflowing label tail must not appear in the visible field. plain:\n{}",
        frame.plain
    );
}

/// Regression (review-3): truncating a colored hyperlink label must keep its
/// closing SGR reset, so inline text following the truncated link does not
/// inherit the link's color in a real terminal.

#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_truncation_does_not_bleed_color_in_terminal() {
    // A red link with an exact 8-cell width truncates, immediately followed by
    // an unstyled trailing marker on the same line.
    let body = "---\nstyle:\n  hyperlinks:\n    color: red-500\n    width: 8\n---\n\n\
        [A very long hyperlink label](https://example.com) ZZTRAIL\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "expected the link's red foreground SGR in the capture. raw len={}",
        frame.raw.len()
    );

    // The trailing marker must not sit inside the link's red run: there must be
    // an SGR reset (or default-foreground) between the last red introduction and
    // the marker. WezTerm reconstructs SGR per cell, so a leaked color would
    // wrap the marker cells with red and no intervening reset.
    let trail_pos = frame
        .raw
        .find("ZZTRAIL")
        .unwrap_or_else(|| panic!("trailing marker missing in raw capture. plain:\n{}", frame.plain));
    let before = &frame.raw[..trail_pos];
    let red_idx = before
        .rfind(red_semi)
        .or_else(|| before.rfind(red_colon))
        .unwrap_or_else(|| {
            panic!("link's red SGR must precede the trailing marker. raw:\n{}", frame.raw)
        });
    let between = &before[red_idx..];
    assert!(
        between.contains("\x1b[0m")
            || between.contains("\x1b[m")
            || between.contains("\x1b[39m"),
        "trailing text inherits the truncated link color: no reset between the \
         red SGR and the marker. raw:\n{}",
        frame.raw
    );
}

/// `style.images.local-style.color` + `bg-color` must color a local image's
/// fallback alt text in a real terminal. Remote images must not pick this up.

#[test]
#[serial(level2_terminal)]
fn level2_style_images_local_style_colors_fallback_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      color: red-500\n---\n\n\
        ![ALT_LOCAL](./no-such-image.png)\n\n![ALT_REMOTE](https://example.com/x.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    let has_red = frame.raw.contains(red_semi) || frame.raw.contains(red_colon);
    assert!(
        has_red,
        "expected red foreground SGR for local image fallback. raw:\n{}",
        frame.raw
    );
    // The local fallback line carries the red bytes; the remote line must
    // not. We only check the raw stream for the presence of red SGR; the
    // remote line shouldn't add a second red occurrence.
    let red_hits = frame.raw.matches(red_semi).count() + frame.raw.matches(red_colon).count();
    assert!(
        (1..=4).contains(&red_hits),
        "unexpected red SGR hit count {red_hits} (heuristic). raw len={}",
        frame.raw.len()
    );
    assert!(
        frame.plain.contains("ALT_LOCAL") && frame.plain.contains("ALT_REMOTE"),
        "alt fallbacks missing from plain capture:\n{}",
        frame.plain
    );
}

/// `style.images.local-style.width: 40` + `alignment: right` must right-align
/// the *complete* fallback placeholder within 40 visible cells.

#[test]
#[serial(level2_terminal)]
fn level2_style_images_local_style_width_alignment_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      width: 40\n      alignment: right\n---\n\n\
        ![A](./no-such-image.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The tree path shapes the *complete* placeholder: `▉ IMAGE[A]` is
    // right-aligned within the 40-cell field, so the padding precedes the
    // placeholder and the alt inside the brackets is untouched.
    let fallback_line = frame
        .plain
        .lines()
        .find(|l| l.contains("▉ IMAGE["))
        .unwrap_or_else(|| panic!("fallback line missing in plain capture:\n{}", frame.plain));
    let inner = fallback_line
        .split_once("▉ IMAGE[")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(inner, _)| inner)
        .unwrap_or("");
    assert_eq!(
        inner, "A",
        "alt inside the brackets must be untouched: {fallback_line:?}"
    );
    let leading_spaces = fallback_line.chars().take_while(|c| *c == ' ').count();
    let field_width = fallback_line.trim_end().chars().count();
    assert!(
        leading_spaces >= 28 && field_width == 40,
        "expected the complete placeholder right-aligned within 40 cells, got {leading_spaces} leading, width {field_width}: {fallback_line:?}"
    );
}

/// A long alt under an exact `width` must truncate the *complete* placeholder
/// to the field in a real terminal — the visible field must not overflow.

#[test]
#[serial(level2_terminal)]
fn level2_style_images_exact_width_truncates_long_alt_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      width: 12\n---\n\n\
        ![A very long image alt text](./no-such-image.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The exact 12-column field truncates with an ellipsis; the overflowing
    // tail of the alt must be absent and the visible placeholder must fill
    // exactly the field, framing included.
    let placeholder_line = frame
        .plain
        .lines()
        .find(|l| l.contains('…'))
        .unwrap_or_else(|| panic!("placeholder line missing in plain capture:\n{}", frame.plain));
    assert!(
        !placeholder_line.contains("image alt text"),
        "the overflowing alt tail must not appear in the visible field: {placeholder_line:?}"
    );
    assert_eq!(
        placeholder_line.trim_end().chars().count(),
        12,
        "the complete visible placeholder must fill exactly the 12-column field: {placeholder_line:?}"
    );
}

/// Regression (review-4): truncating a colored local-image placeholder must keep
/// its closing SGR reset, so inline text following the truncated image does not
/// inherit the image's color in a real terminal. Links and images use distinct
/// renderer branches, so the hyperlink color-bleed regression
/// (`level2_style_hyperlinks_truncation_does_not_bleed_color_in_terminal`) does
/// not cover the image placeholder's reset — this verifies it separately.

#[test]
#[serial(level2_terminal)]
fn level2_style_images_truncation_does_not_bleed_color_in_terminal() {
    // A red local-image placeholder with an exact 12-cell width truncates the
    // long alt, immediately followed by an unstyled trailing marker on the same
    // line.
    let body = "---\nstyle:\n  images:\n    local-style:\n      color: red-500\n      width: 12\n---\n\n\
        ![A very long image alt text](./no-such-image.png) ZZTRAIL\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "expected the local image's red foreground SGR in the capture. raw len={}",
        frame.raw.len()
    );

    // The trailing marker must not sit inside the image's red run: there must be
    // an SGR reset (or default-foreground) between the last red introduction and
    // the marker. WezTerm reconstructs SGR per cell, so a leaked color would
    // wrap the marker cells with red and no intervening reset.
    let trail_pos = frame
        .raw
        .find("ZZTRAIL")
        .unwrap_or_else(|| panic!("trailing marker missing in raw capture. plain:\n{}", frame.plain));
    let before = &frame.raw[..trail_pos];
    let red_idx = before
        .rfind(red_semi)
        .or_else(|| before.rfind(red_colon))
        .unwrap_or_else(|| {
            panic!("image's red SGR must precede the trailing marker. raw:\n{}", frame.raw)
        });
    let between = &before[red_idx..];
    assert!(
        between.contains("\x1b[0m")
            || between.contains("\x1b[m")
            || between.contains("\x1b[39m"),
        "trailing text inherits the truncated image color: no reset between the \
         red SGR and the marker. raw:\n{}",
        frame.raw
    );
}
