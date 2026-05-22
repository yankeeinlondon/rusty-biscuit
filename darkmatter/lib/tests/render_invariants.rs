//! Ground-truth layout invariants for darkmatter terminal rendering.
//!
//! Unlike `render_comparison.rs` (which checks that the bespoke and render-tree
//! paths *agree*), this suite checks that a single rendered output is *correct*
//! against properties that must hold for **every** block shape under **every**
//! layout. Parity can never catch a fault both renderers share; these can.
//!
//! The invariant predicates themselves live in
//! [`biscuit_test_harness::layout_invariants`] so biscuit-terminal can sweep
//! its own renderable components against the same contract. This suite supplies
//! darkmatter's `shapes() x scenarios()` matrix and the application policy
//! (which invariant applies to which shape).
//!
//! Each invariant generalizes a concrete defect observed in the
//! `2026-05-22-darkmatter-failures` investigation:
//!
//! - I1  Containment        — no line exceeds the physical terminal width.
//! - I2a NoClearToEol        — no `\x1b[K` under an active layout.
//! - I2b BackgroundRectangle — code panel fill spans exactly `[left, width-right]`.
//! - I3  RightBoundaryCoherence — every panel line ends its fill at one column.
//! - I4  LeftOffsetUniformity   — every panel line opens its fill at `left`.
//! - I5  VerticalRhythm      — no run of >=2 consecutive interior blank lines.
//! - I5b MarginBlanks        — leading/trailing blanks equal top/bottom margin.
//! - I6  BlankLineIdempotence — 1 vs N source blank lines render identically.
//! - I7  ColorModeContract   — code resolves its theme against the inverted mode.
//!
//! ## Ledger protocol
//!
//! The sweep records *every* violation across `shapes() x scenarios()` as a
//! `(shape, scenario, invariant)` key and diffs the live set against the
//! committed [`KNOWN_VIOLATIONS`] ledger — exactly like `render_comparison.rs`.
//! A new violation is a regression; a vanished one is a fix to delete from the
//! ledger. Regenerate with:
//! `RECORD_INVARIANTS=1 cargo test -p darkmatter --test render_invariants -- --nocapture`
//! then paste the printed literal between the `&[` and `]` of `KNOWN_VIOLATIONS`.
//! An empty ledger means every shape/scenario satisfies every applicable
//! invariant.

use std::collections::{BTreeMap, BTreeSet};

use biscuit_terminal::terminal::Terminal;
use biscuit_test_harness::layout_invariants::{self as inv, LayoutExpectation};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};

/// One block shape, as a minimal Markdown fixture.
struct Shape {
    name: &'static str,
    md: &'static str,
    /// Whether this shape contains a code block. Code blocks paint a full-bleed
    /// panel background, so the background-rectangle family (I2b/I3/I4) applies;
    /// other shapes paint partial-width or no background, so it does not.
    is_code: bool,
}

fn shapes() -> Vec<Shape> {
    vec![
        Shape {
            name: "heading",
            md: "# A Heading\n",
            is_code: false,
        },
        Shape {
            name: "prose",
            md: "Some prose paragraph text goes here.\n",
            is_code: false,
        },
        Shape {
            name: "prose_wrap",
            md: "This is a deliberately long paragraph of prose intended to exceed the available content width so that word wrapping is exercised against the page layout under test conditions.\n",
            is_code: false,
        },
        Shape {
            name: "ulist",
            md: "- item one\n- item two\n",
            is_code: false,
        },
        Shape {
            name: "olist",
            md: "1. item one\n2. item two\n",
            is_code: false,
        },
        Shape {
            name: "blockquote",
            md: "> a quoted line of text\n",
            is_code: false,
        },
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
        Shape {
            name: "hr",
            md: "above\n\n---\n\nbelow\n",
            is_code: false,
        },
        Shape {
            name: "image",
            md: "![alt text|20](nonexistent.png)\n",
            is_code: false,
        },
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

impl Scenario {
    /// The resolved [`LayoutExpectation`]. With default `Full` fill and no
    /// alignment, the resolved offsets equal the configured margins.
    fn expectation(&self) -> LayoutExpectation {
        LayoutExpectation {
            width: self.width as usize,
            left: self.ml as usize,
            right: self.mr as usize,
            top: self.mt as usize,
            bottom: self.mb as usize,
        }
    }
}

fn scenarios() -> Vec<Scenario> {
    vec![
        // The literal reproduction from the bug report.
        Scenario {
            name: "repro_ml4_mr4_mt1_mb1",
            width: 120,
            ml: 4,
            mr: 4,
            mt: 1,
            mb: 1,
        },
        // Full fill (default) with symmetric margins at a narrower width.
        Scenario {
            name: "margins_2_w80",
            width: 80,
            ml: 2,
            mr: 2,
            mt: 0,
            mb: 0,
        },
        // Left-only and right-only margins.
        Scenario {
            name: "left_only_w100",
            width: 100,
            ml: 6,
            mr: 0,
            mt: 0,
            mb: 0,
        },
        Scenario {
            name: "right_only_w100",
            width: 100,
            ml: 0,
            mr: 6,
            mt: 0,
            mb: 0,
        },
        // Vertical margins only.
        Scenario {
            name: "vert_mt2_mb2_w100",
            width: 100,
            ml: 0,
            mr: 0,
            mt: 2,
            mb: 2,
        },
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

/// One applicable invariant: stable id, whether it only applies to full-bleed
/// code panels, and the predicate from the shared harness.
type Check = (
    &'static str,
    bool,
    fn(&str, &LayoutExpectation) -> Result<(), String>,
);

fn checks() -> Vec<Check> {
    vec![
        ("I1", false, inv::containment),
        ("I2a", false, inv::no_clear_to_eol),
        ("I2b", true, inv::background_rectangle),
        ("I3", true, inv::right_boundary_coherence),
        ("I4", true, inv::left_offset_uniformity),
        ("I5", false, inv::vertical_rhythm),
        ("I5b", false, inv::margin_blanks),
    ]
}

/// A single recorded breach: one shape, one scenario, one invariant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ViolationKey {
    shape: &'static str,
    scenario: &'static str,
    invariant: &'static str,
}

/// The committed violation ledger. Each tuple is `(shape, scenario, invariant)`.
/// An empty ledger means every shape/scenario satisfies every applicable
/// invariant. Regenerate with `RECORD_INVARIANTS=1`.
#[rustfmt::skip]
const KNOWN_VIOLATIONS: &[(&str, &str, &str)] = &[];

fn record_mode() -> bool {
    matches!(
        std::env::var("RECORD_INVARIANTS")
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

#[test]
fn layout_invariants_hold_across_matrix() {
    let known: BTreeSet<ViolationKey> = KNOWN_VIOLATIONS
        .iter()
        .map(|(shape, scenario, invariant)| ViolationKey {
            shape,
            scenario,
            invariant,
        })
        .collect();
    assert_eq!(
        known.len(),
        KNOWN_VIOLATIONS.len(),
        "duplicate KNOWN_VIOLATIONS entry"
    );

    let mut live = BTreeSet::<ViolationKey>::new();
    let mut messages = BTreeMap::<ViolationKey, String>::new();

    for shape in shapes() {
        for scenario in scenarios() {
            let rendered = render(&shape, &scenario);
            let exp = scenario.expectation();
            for (id, code_only, check) in checks() {
                if code_only && !shape.is_code {
                    continue;
                }
                if let Err(msg) = check(&rendered, &exp) {
                    let key = ViolationKey {
                        shape: shape.name,
                        scenario: scenario.name,
                        invariant: id,
                    };
                    messages.insert(key.clone(), msg);
                    live.insert(key);
                }
            }
        }
    }

    if record_mode() {
        println!("=== BEGIN KNOWN_VIOLATIONS (paste between `&[` and `]`) ===");
        for key in &live {
            println!(
                "    ({:?}, {:?}, {:?}),",
                key.shape, key.scenario, key.invariant
            );
        }
        println!("=== END KNOWN_VIOLATIONS ({} entries) ===", live.len());
        return;
    }

    let unrecorded: Vec<&ViolationKey> = live.difference(&known).collect();
    let fixed: Vec<&ViolationKey> = known.difference(&live).collect();

    if unrecorded.is_empty() && fixed.is_empty() {
        return;
    }

    let mut msg = format!(
        "live violations: {}, known: {}, regressions: {}, fixed: {}\n",
        live.len(),
        known.len(),
        unrecorded.len(),
        fixed.len(),
    );

    if !unrecorded.is_empty() {
        msg.push_str("\nREGRESSION / unrecorded violations:\n");
        for key in &unrecorded {
            msg.push_str(&format!(
                "  [{} / {}] {}\n",
                key.shape,
                key.scenario,
                messages.get(*key).map(String::as_str).unwrap_or("?"),
            ));
        }
    }

    if !fixed.is_empty() {
        msg.push_str("\nFIXED — remove from KNOWN_VIOLATIONS:\n");
        for key in &fixed {
            msg.push_str(&format!(
                "    ({:?}, {:?}, {:?}),\n",
                key.shape, key.scenario, key.invariant
            ));
        }
    }

    panic!("{msg}");
}

/// I6 BlankLineIdempotence — two source documents differing only in the number
/// of blank lines between blocks must render identically. Swept across every
/// shape and scenario.
#[test]
fn blank_line_count_is_idempotent() {
    // Each shape's fixture, joined to a trailing block with 1 vs 5 vs 99 blanks.
    let tail = "## Tail\n\nClosing paragraph.\n";
    let mut violations = Vec::new();

    for shape in shapes() {
        for scenario in scenarios() {
            let one = Shape {
                name: shape.name,
                md: Box::leak(format!("{}\n{tail}", shape.md).into_boxed_str()),
                is_code: shape.is_code,
            };
            let five = Shape {
                name: shape.name,
                md: Box::leak(format!("{}\n\n\n\n\n{tail}", shape.md).into_boxed_str()),
                is_code: shape.is_code,
            };
            let many = Shape {
                name: shape.name,
                md: Box::leak(format!("{}{}{tail}", shape.md, "\n".repeat(99)).into_boxed_str()),
                is_code: shape.is_code,
            };

            let r1 = render(&one, &scenario);
            let r5 = render(&five, &scenario);
            let r99 = render(&many, &scenario);

            if let Err(e) = inv::blank_line_idempotent(&r1, &r5) {
                violations.push(format!("[{} / {}] 1 vs 5: {e}", shape.name, scenario.name));
            }
            if let Err(e) = inv::blank_line_idempotent(&r1, &r99) {
                violations.push(format!("[{} / {}] 1 vs 99: {e}", shape.name, scenario.name));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} blank-line-idempotence violation(s):\n{}",
        violations.len(),
        violations.join("\n"),
    );
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
