mod common;

use biscuit_test_harness::CapturedFrame;
use common::level2::run_md_env;
use serial_test::serial;

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_kind_waves_renders_in_real_terminal() {
    // `style.hr.kind: waves` must reach the HR renderer through the canonical
    // path. Unicode-capable terminals print `≋`; ASCII fallback prints `~`.
    let body = r#"---
style:
    hr:
        kind: waves
---

hr_waves_lead_anchor

---

hr_waves_tail_anchor
"#;

    // Force the text tier: in a graphics-capable terminal the styled HR
    // rasterizes to an image and no glyph reaches a text row (review-1
    // finding 3). The assertion is also anchored between sentinels so a stray
    // `~` from the shell prompt cannot satisfy it.
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((plain, _)) =
        locate_hr_between_sentinels(&frame, "hr_waves_lead_anchor", "hr_waves_tail_anchor")
    else {
        panic!(
            "expected a waves HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    assert!(
        plain.contains('\u{224B}') || plain.contains('~'),
        "style.hr.kind: waves must produce the waves glyph (`≋` or `~`) on the rule row; got: {plain:?}",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_weight_thick_differs_from_thin_in_real_terminal() {
    // `style.hr.weight: thick` vs `thin` must produce visibly different bytes
    // in the captured pane (verified separately via terminal_text_options in
    // Level 1; this proves the difference survives a real terminal).
    let body_thick = r#"---
style:
    hr:
        kind: dashes
        weight: thick
---

hr_weight_lead_anchor

---

hr_weight_tail_anchor
"#;
    let body_thin = r#"---
style:
    hr:
        kind: dashes
        weight: thin
---

hr_weight_lead_anchor

---

hr_weight_tail_anchor
"#;

    // Force the text tier so the weight difference appears as glyphs rather than
    // pixels, and isolate the rule row between sentinels so the comparison
    // cannot accidentally match the (per-invocation distinct) command echo
    // (review-1 finding 3).
    let Some((frame_thick, _)) =
        run_md_env(body_thick, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((frame_thin, _)) =
        run_md_env(body_thin, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };

    let Some((thick_rule_line, _)) =
        locate_hr_between_sentinels(&frame_thick, "hr_weight_lead_anchor", "hr_weight_tail_anchor")
    else {
        panic!(
            "expected a thick HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_thick.plain, frame_thick.raw
        );
    };
    let Some((thin_rule_line, _)) =
        locate_hr_between_sentinels(&frame_thin, "hr_weight_lead_anchor", "hr_weight_tail_anchor")
    else {
        panic!(
            "expected a thin HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_thin.plain, frame_thin.raw
        );
    };

    assert_ne!(
        thick_rule_line.trim(),
        thin_rule_line.trim(),
        "thick and thin HR weights must render visibly different glyphs",
    );
}

/// Find the captured rule line between the unique sentinels `LEAD_SENTINEL`
/// and `TAIL_SENTINEL`. Returns the matching `(plain_line, raw_line)` pair
/// or `None` when the rule is missing or scrolled out of the capture.
fn locate_hr_between_sentinels<'a>(
    frame: &'a CapturedFrame,
    lead: &str,
    tail: &str,
) -> Option<(&'a str, &'a str)> {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let lead_idx = plain_lines.iter().position(|l| l.contains(lead))?;
    let tail_idx = plain_lines.iter().position(|l| l.contains(tail))?;
    if lead_idx >= tail_idx {
        return None;
    }
    // The rule glyph lives on some line strictly between the sentinels.
    for i in (lead_idx + 1)..tail_idx {
        let line = plain_lines.get(i)?;
        // Skip blank rows; the rule itself carries visible glyphs.
        if line.trim().is_empty() {
            continue;
        }
        let raw_line = raw_lines.get(i).copied().unwrap_or("");
        return Some((line, raw_line));
    }
    None
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_color_emits_sgr_in_real_terminal() {
    // `style.hr.color: red-500` must emit a red SGR escape on the rule row.
    // We use unique sentinels around the rule to isolate the captured row
    // and accept both WezTerm SGR re-emission forms (semicolon, colon).
    let body = r#"---
style:
    hr:
        color: red-500
---

hr_color_lead_anchor

---

hr_color_tail_anchor
"#;

    // Force the text tier: in a graphics-capable terminal (WezTerm supports the
    // Kitty graphics protocol) a styled HR rasterizes to an image, so the text
    // rule row — and the foreground SGR this test asserts — never appears. The
    // color is a text-rule property, so it must be exercised on the text tier
    // (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((_plain, raw)) =
        locate_hr_between_sentinels(&frame, "hr_color_lead_anchor", "hr_color_tail_anchor")
    else {
        // The harness was available and `md` completed (we have a frame), so a
        // missing rule row is a real failure of a terminal-visible requirement,
        // not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    let red_semi = "\x1b[38;2;251;44;54m";
    let red_colon = "\x1b[38:2::251:44:54m";
    assert!(
        raw.contains(red_semi) || raw.contains(red_colon),
        "style.hr.color must reach the rule row as a foreground SGR. \
         raw row:\n{raw}\nfull raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_bg_color_emits_background_sgr_in_real_terminal() {
    // `style.hr.bg-color: blue-500` must paint a background SGR on the rule
    // row. WezTerm re-emits truecolor backgrounds as `48;2;…` or `48:2:…`.
    let body = r#"---
style:
    hr:
        bg-color: blue-500
---

hr_bg_lead_anchor

---

hr_bg_tail_anchor
"#;

    // Force the text tier so the rule paints as a real row (see the color test):
    // a graphics-capable terminal would rasterize the styled HR to an image and
    // the background SGR would never reach a text row (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((_plain, raw)) =
        locate_hr_between_sentinels(&frame, "hr_bg_lead_anchor", "hr_bg_tail_anchor")
    else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    let bg_present = raw.contains("\x1b[48;2;") || raw.contains("\x1b[48:2:");
    assert!(
        bg_present,
        "style.hr.bg-color must paint a background SGR on the rule row. \
         raw row:\n{raw}\nfull raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_alignment_center_offsets_rule_from_left_in_real_terminal() {
    // `style.hr.alignment: center` plus a narrow `width: 20` must offset
    // the rule from the left edge. We compare the leading-space count of
    // the rule row in a centered render against a left-aligned render.
    let centered = r#"---
style:
    hr:
        kind: dashes
        alignment: center
        width: 20
---

hr_align_lead_anchor

---

hr_align_tail_anchor
"#;
    let left = r#"---
style:
    hr:
        kind: dashes
        alignment: left
        width: 20
---

hr_align_lead_anchor

---

hr_align_tail_anchor
"#;

    // Force the text tier: a rasterized HR encodes alignment in pixels, not in
    // leading whitespace, so the indent comparison this test makes is only
    // meaningful on the text rule (review-1 finding 3).
    let Some((frame_center, _)) =
        run_md_env(centered, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((frame_left, _)) = run_md_env(left, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((plain_center, _)) = locate_hr_between_sentinels(
        &frame_center,
        "hr_align_lead_anchor",
        "hr_align_tail_anchor",
    ) else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected a centered HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_center.plain, frame_center.raw
        );
    };
    let Some((plain_left, _)) =
        locate_hr_between_sentinels(&frame_left, "hr_align_lead_anchor", "hr_align_tail_anchor")
    else {
        panic!(
            "expected a left-aligned HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_left.plain, frame_left.raw
        );
    };

    let center_indent = plain_center.chars().take_while(|c| *c == ' ').count();
    let left_indent = plain_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "centered HR must have more leading whitespace than left-aligned; \
         center={center_indent}, left={left_indent}\ncentered row: {plain_center:?}\nleft row: {plain_left:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_width_caps_visible_columns_in_real_terminal() {
    // `style.hr.width: 20` must produce a rule whose visible glyphs span no
    // more than 20 columns, regardless of the surrounding page width.
    let body = r#"---
style:
    hr:
        kind: dashes
        width: 20
---

hr_width_lead_anchor

---

hr_width_tail_anchor
"#;

    // Force the text tier: a rasterized HR encodes its width in pixels, so the
    // visible-glyph-count cap this test asserts only applies to the text rule
    // (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };
    let Some((plain, _)) =
        locate_hr_between_sentinels(&frame, "hr_width_lead_anchor", "hr_width_tail_anchor")
    else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    // Count only the visible rule glyphs (skip the left padding). Dashes
    // render as `╌` (Unicode) or `-` (ASCII fallback). The rule glyphs are
    // contiguous; non-rule characters are spaces.
    let rule_glyph_count = plain
        .chars()
        .filter(|c| !c.is_whitespace())
        .count();
    assert!(
        rule_glyph_count > 0,
        "expected visible rule glyphs in row:\n{plain}\nfull plain:\n{}",
        frame.plain
    );
    assert!(
        rule_glyph_count <= 20,
        "style.hr.width: 20 must cap the visible rule to <=20 glyphs; got {rule_glyph_count} \
         in row:\n{plain}"
    );
}
