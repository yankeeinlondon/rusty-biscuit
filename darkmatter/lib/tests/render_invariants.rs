//! Ground-truth layout invariants for darkmatter terminal rendering.
//!
//! Unlike `render_comparison.rs` (which checks that the bespoke and render-tree
//! paths *agree*), this suite checks that a single rendered output is *correct*
//! against properties that must hold for **every** block shape under **every**
//! layout. Parity can never catch a fault both renderers share; these can.
//!
//! Each invariant generalizes a concrete defect observed in the
//! `2026-05-22-darkmatter-failures` investigation:
//!
//! - I1 Containment        — no line exceeds the physical terminal width.
//! - I2 NoClearToEol        — no `\x1b[K` is emitted under an active layout
//!   (clear-to-edge is incompatible with post-hoc margin decoration).
//! - I5 VerticalRhythm      — no run of >=2 consecutive blank lines in the body
//!   (a background-filled padding row is NOT blank).
//! - I5b MarginBlanks       — leading/trailing blank rows equal the configured
//!   top/bottom margin exactly (no constant offset).
//!
//! The suite sweeps `shapes() x scenarios()` and reports *every* violation at
//! once, so latent breakage in shapes we have not hand-inspected is surfaced in
//! a single run rather than block-by-block later.

use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};

/// One block shape, as a minimal Markdown fixture.
struct Shape {
    name: &'static str,
    md: &'static str,
    /// Whether this shape contains a code block (relevant to theme-contrast
    /// invariants; carried for future I7 coverage).
    #[allow(dead_code)]
    is_code: bool,
}

fn shapes() -> Vec<Shape> {
    vec![
        Shape { name: "heading", md: "# A Heading\n", is_code: false },
        Shape { name: "prose", md: "Some prose paragraph text goes here.\n", is_code: false },
        Shape {
            name: "prose_wrap",
            md: "This is a deliberately long paragraph of prose intended to exceed the available content width so that word wrapping is exercised against the page layout under test conditions.\n",
            is_code: false,
        },
        Shape { name: "ulist", md: "- item one\n- item two\n", is_code: false },
        Shape { name: "olist", md: "1. item one\n2. item two\n", is_code: false },
        Shape { name: "blockquote", md: "> a quoted line of text\n", is_code: false },
        Shape {
            name: "table",
            md: "| A | B |\n|---|---|\n| 1 | 2 |\n",
            is_code: false,
        },
        Shape {
            name: "code_rust",
            md: "```rust\npub struct FooBar {\n    foo: String,\n    bar: u32,\n}\n```\n",
            is_code: true,
        },
        Shape {
            name: "code_ts",
            md: "```ts\ntype FooBar = {\n    foo: string;\n    bar: number;\n}\n```\n",
            is_code: true,
        },
        Shape { name: "hr", md: "above\n\n---\n\nbelow\n", is_code: false },
        Shape { name: "image", md: "![alt text|20](nonexistent.png)\n", is_code: false },
        Shape {
            name: "blocks_fixture",
            md: "# Blocks Test\n\nThis Markdown has a series of code blocks.\n\n## Rust\n\n```rust\npub struct FooBar {\n    foo: String,\n    bar: u32,\n}\n```\n\n## Typescript\n\n```ts\ntype FooBar = {\n    foo: string;\n    bar: number;\n}\n```\n",
            is_code: true,
        },
    ]
}

/// One layout configuration applied via [`DarkmatterPage`].
struct Scenario {
    name: &'static str,
    width: u16,
    ml: u16,
    mr: u16,
    mt: u16,
    mb: u16,
}

fn scenarios() -> Vec<Scenario> {
    vec![
        // The literal reproduction from the bug report.
        Scenario { name: "repro_ml4_mr4_mt1_mb1", width: 120, ml: 4, mr: 4, mt: 1, mb: 1 },
        // Full fill (default) with symmetric margins at a narrower width.
        Scenario { name: "margins_2_w80", width: 80, ml: 2, mr: 2, mt: 0, mb: 0 },
        // Left-only and right-only margins.
        Scenario { name: "left_only_w100", width: 100, ml: 6, mr: 0, mt: 0, mb: 0 },
        Scenario { name: "right_only_w100", width: 100, ml: 0, mr: 6, mt: 0, mb: 0 },
        // Vertical margins only.
        Scenario { name: "vert_mt2_mb2_w100", width: 100, ml: 0, mr: 0, mt: 2, mb: 2 },
    ]
}

fn render(shape: &Shape, scenario: &Scenario) -> String {
    let term = Terminal::new_optimistic(scenario.width as u32);
    let page = DarkmatterPage::new(&term)
        .with_margin_left(scenario.ml)
        .with_margin_right(scenario.mr)
        .with_margin_top(scenario.mt)
        .with_margin_bottom(scenario.mb)
        .with_code_theme("dracula");
    let md: Markdown = shape.md.into();
    page.render(&md)
        .unwrap_or_else(|e| format!("<render error: {e}>"))
}

/// Visible width of a raw (ANSI-bearing) line, in characters.
fn visible_width(line: &str) -> usize {
    strip_escape_codes(line).chars().count()
}

/// A line is "blank" when it carries no visible glyphs AND no background fill.
///
/// Whitespace-only counts as blank (a page margin row is `ml + mr` spaces but
/// is semantically empty). A background-filled padding row (e.g. a code-block
/// padding row, or a page padding row) is content, not blank.
fn is_blank(line: &str) -> bool {
    strip_escape_codes(line).trim().is_empty() && !line.contains("\x1b[48")
}

// ---- Invariants: each returns Some(violation message) on failure ----

fn i1_containment(rendered: &str, sc: &Scenario) -> Option<String> {
    for (i, line) in rendered.lines().enumerate() {
        let w = visible_width(line);
        if w > sc.width as usize {
            return Some(format!(
                "I1 Containment: line {i} visible width {w} exceeds terminal width {}",
                sc.width
            ));
        }
    }
    None
}

fn i2_no_clear_to_eol(rendered: &str, _sc: &Scenario) -> Option<String> {
    if rendered.contains("\x1b[K") {
        let count = rendered.matches("\x1b[K").count();
        return Some(format!(
            "I2 NoClearToEol: found {count} `\\x1b[K` (clear-to-edge) under active layout"
        ));
    }
    None
}

fn i5_vertical_rhythm(rendered: &str, _sc: &Scenario) -> Option<String> {
    // Only the *interior* body is checked — leading and trailing page margins
    // legitimately stack and are validated separately by I5b. The interior is
    // the span between the first and last non-blank line.
    let lines: Vec<&str> = rendered.lines().collect();
    let first = lines.iter().position(|l| !is_blank(l));
    let last = lines.iter().rposition(|l| !is_blank(l));
    let (Some(first), Some(last)) = (first, last) else {
        return None;
    };
    let mut run = 0usize;
    for (offset, line) in lines[first..=last].iter().enumerate() {
        if is_blank(line) {
            run += 1;
            if run >= 2 {
                let i = first + offset;
                return Some(format!(
                    "I5 VerticalRhythm: >=2 consecutive interior blank lines ending at line {i}"
                ));
            }
        } else {
            run = 0;
        }
    }
    None
}

fn i5b_margin_blanks(rendered: &str, sc: &Scenario) -> Option<String> {
    let lines: Vec<&str> = rendered.lines().collect();
    let leading = lines.iter().take_while(|l| is_blank(l)).count();
    let trailing = lines.iter().rev().take_while(|l| is_blank(l)).count();
    if leading != sc.mt as usize {
        return Some(format!(
            "I5b MarginBlanks: leading blank rows = {leading}, expected mt = {}",
            sc.mt
        ));
    }
    if trailing != sc.mb as usize {
        return Some(format!(
            "I5b MarginBlanks: trailing blank rows = {trailing}, expected mb = {}",
            sc.mb
        ));
    }
    None
}

type Invariant = (&'static str, fn(&str, &Scenario) -> Option<String>);

fn invariants() -> Vec<Invariant> {
    vec![
        ("I1", i1_containment),
        ("I2", i2_no_clear_to_eol),
        ("I5", i5_vertical_rhythm),
        ("I5b", i5b_margin_blanks),
    ]
}

#[test]
fn layout_invariants_hold_across_matrix() {
    let mut violations: Vec<String> = Vec::new();

    for shape in shapes() {
        for scenario in scenarios() {
            let rendered = render(&shape, &scenario);
            for (_, check) in invariants() {
                if let Some(msg) = check(&rendered, &scenario) {
                    violations.push(format!("[{} / {}] {}", shape.name, scenario.name, msg));
                }
            }
        }
    }

    if !violations.is_empty() {
        let report = violations.join("\n");
        panic!(
            "{} layout-invariant violation(s):\n{report}",
            violations.len()
        );
    }
}

/// Background SGR string for a syntect theme's resolved background color.
fn theme_bg_ansi(pair: ThemePair, mode: ColorMode) -> String {
    let bg = CodeHighlighter::new(pair, mode)
        .theme()
        .settings
        .background
        .expect("theme has a background color");
    format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b)
}

/// I7 ColorModeContract — a code block resolves its theme against the
/// **inverted** terminal color mode, so it contrasts against the page. With a
/// paired theme (`github`) in a dark terminal, the code body must paint the
/// *light* variant's background and never the dark variant's.
#[test]
fn i7_code_block_inverts_theme_against_dark_terminal() {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term)
        .with_color_mode(ColorMode::Dark)
        .with_code_theme("github")
        .with_margin_left(2)
        .with_margin_right(2);
    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let out = page.render(&md).unwrap();

    let light_bg = theme_bg_ansi(ThemePair::Github, ColorMode::Light);
    let dark_bg = theme_bg_ansi(ThemePair::Github, ColorMode::Dark);

    assert!(
        out.contains(&light_bg),
        "dark terminal should paint the code block with the LIGHT github background {light_bg:?}"
    );
    assert!(
        !out.contains(&dark_bg),
        "dark terminal must NOT paint the code block with the dark github background {dark_bg:?}"
    );
}

/// The mirror of I7: a light terminal must use the dark variant for code.
#[test]
fn i7_code_block_inverts_theme_against_light_terminal() {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term)
        .with_color_mode(ColorMode::Light)
        .with_code_theme("github")
        .with_margin_left(2)
        .with_margin_right(2);
    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let out = page.render(&md).unwrap();

    let dark_bg = theme_bg_ansi(ThemePair::Github, ColorMode::Dark);
    assert!(
        out.contains(&dark_bg),
        "light terminal should paint the code block with the DARK github background {dark_bg:?}"
    );
}

/// Single-variant themes (e.g. `dracula`) resolve identically under both modes
/// by design; inversion is a deliberate no-op. Pin it so a future change can't
/// silently break the documented abstraction.
#[test]
fn single_variant_theme_ignores_mode() {
    assert_eq!(
        theme_bg_ansi(ThemePair::Dracula, ColorMode::Dark),
        theme_bg_ansi(ThemePair::Dracula, ColorMode::Light),
    );
}
