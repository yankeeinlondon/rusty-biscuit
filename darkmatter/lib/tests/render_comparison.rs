//! Render comparison assertions for darkmatter's layout visual-test matrix.
//!
//! For every component/scenario cell this suite renders both the bespoke and
//! render-tree paths, then analyzes the pair across six independent facets
//! (exact bytes, visible text, indentation, blank lines, width, styling).
//! Each detected divergence becomes a `DriftKey`. The live drift set is
//! compared against the committed `KNOWN_DRIFT` ledger: a new divergence is a
//! regression, a vanished one is a fix that must be removed from the ledger.
//!
//! Regenerate the ledger after intentional changes with:
//! `RECORD_DRIFT=1 cargo test -p darkmatter --test render_comparison -- --nocapture`
//! then paste the printed literal between the `&[` and `]` of `KNOWN_DRIFT`.

mod layout_matrix_support;

use std::collections::{BTreeMap, BTreeSet};

use biscuit_terminal::prelude::strip_escape_codes;
use layout_matrix_support::{component_cases, scenarios};

/// One independent dimension along which a bespoke/tree pair can diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Facet {
    /// Byte-for-byte identical output, ANSI retained.
    Exact,
    /// Identical once ANSI escape codes are stripped.
    Text,
    /// Identical leading-space counts on every line.
    Indent,
    /// Identical positions of blank lines.
    BlankLines,
    /// Identical maximum visible line width.
    Width,
    /// Identical SGR sequences at identical visible offsets.
    Styling,
}

/// Which renderer is preferred for a recorded drift entry, and whether the
/// difference is functionally important or cosmetic.
///
/// All current darkmatter entries are `BespokeBehind`; the other variants
/// exist so the ledger can carry hand-assigned classifications after
/// regeneration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(dead_code)]
enum Verdict {
    /// The render-tree output is wrong or incomplete versus the bespoke
    /// renderer — the render-tree has not caught up. The headline metric.
    TreeBehind,
    /// The render-tree output is more correct; the bespoke renderer is the
    /// laggard. Acceptable drift; may never reach zero.
    BespokeBehind,
    /// Cosmetic-only drift where the bespoke renderer is still preferred.
    CosmeticTreeBehind,
    /// Cosmetic-only drift where the render-tree output is preferred.
    CosmeticBespokeBehind,
    /// Cosmetic-only drift with no preferred renderer yet recorded.
    CosmeticNeutral,
}

/// Every facet, in the stable ordering used for ledger output.
const ALL_FACETS: [Facet; 6] = [
    Facet::Exact,
    Facet::Text,
    Facet::Indent,
    Facet::BlankLines,
    Facet::Width,
    Facet::Styling,
];

/// A single recorded divergence: one component, one scenario, one facet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DriftKey {
    component: &'static str,
    scenario: &'static str,
    facet: Facet,
}

/// The committed drift ledger.
///
/// Each tuple is `(component, scenario, facet, verdict)`. The first three
/// fields identify the divergence; the `Verdict` records which renderer is
/// preferred (see [`Verdict`]). Regenerate with `RECORD_DRIFT=1`: regeneration
/// preserves the verdict of every still-present entry, and any genuinely new
/// entry is emitted as `Verdict::TreeBehind` and must be reviewed and
/// reclassified by hand. An empty ledger means the bespoke and tree paths
/// agree across every facet of every matrix cell.
///
/// The render-tree path now syntax-highlights `YamlBlock` through darkmatter's
/// `TerminalCodeRenderer` `CodeRenderer` hook, so the `baseline`, `width_40`,
/// and `width_120` scenarios — which exercise no node layout — agree across
/// every facet and carry no ledger entries.
///
/// Every remaining entry is `BespokeBehind`: the divergences are confined to
/// the layout scenarios (margins, alignment, word-wrap), and in each one the
/// render-tree output is *more* correct. The bespoke renderer applies layout
/// by calling `Layout::apply_layout` on an already-built, full-width
/// `{header}\n{body}` string, which (a) builds the header at the full terminal
/// width and then prefixes the margin, overflowing the line; (b) ragged-centers
/// individual code lines instead of shifting the block as a unit; and (c) loses
/// the code block's intrinsic top/bottom padding rows and the vertical-margin
/// blank lines. The render tree instead renders the code body within the
/// margin-reduced width and applies block-level alignment and vertical margins
/// around it. No entry is `TreeBehind`.
#[rustfmt::skip]
const KNOWN_DRIFT: &[(&str, &str, Facet, Verdict)] = &[
    ("YamlBlock", "align_center", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "align_center", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "align_center", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "align_center", Facet::BlankLines, Verdict::BespokeBehind),
    ("YamlBlock", "align_center", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "align_right", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "align_right", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "align_right", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "align_right", Facet::BlankLines, Verdict::BespokeBehind),
    ("YamlBlock", "align_right", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "bottom_margin_2", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "bottom_margin_2", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "bottom_margin_2", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "bottom_margin_2", Facet::BlankLines, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_4", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_4", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_4", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_4", Facet::Width, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_4", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_pct_10", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_pct_10", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_pct_10", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_pct_10", Facet::Width, Verdict::BespokeBehind),
    ("YamlBlock", "left_margin_pct_10", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "right_margin_4", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "right_margin_4", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "right_margin_4", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "right_margin_4", Facet::Width, Verdict::BespokeBehind),
    ("YamlBlock", "right_margin_4", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "top_margin_2", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "top_margin_2", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "top_margin_2", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "top_margin_2", Facet::BlankLines, Verdict::BespokeBehind),
    ("YamlBlock", "top_margin_2", Facet::Styling, Verdict::BespokeBehind),
    ("YamlBlock", "word_wrap_prose", Facet::Exact, Verdict::BespokeBehind),
    ("YamlBlock", "word_wrap_prose", Facet::Text, Verdict::BespokeBehind),
    ("YamlBlock", "word_wrap_prose", Facet::Indent, Verdict::BespokeBehind),
    ("YamlBlock", "word_wrap_prose", Facet::BlankLines, Verdict::BespokeBehind),
    ("YamlBlock", "word_wrap_prose", Facet::Styling, Verdict::BespokeBehind),
];

/// Returns `true` when `a` and `b` are byte-for-byte identical.
fn extract_exact(a: &str, b: &str) -> bool {
    a == b
}

/// Returns `true` when `a` and `b` match once ANSI codes are stripped.
fn extract_text(a: &str, b: &str) -> bool {
    strip_escape_codes(a) == strip_escape_codes(b)
}

/// Counts leading `U+0020` spaces on each line, after ANSI stripping.
fn indent_profile(s: &str) -> Vec<usize> {
    strip_escape_codes(s)
        .split('\n')
        .map(|line| line.chars().take_while(|c| *c == ' ').count())
        .collect()
}

/// Returns `true` when both strings share the same per-line indentation.
fn extract_indent(a: &str, b: &str) -> bool {
    indent_profile(a) == indent_profile(b)
}

/// Collects 0-based indices of blank lines, after ANSI stripping.
fn blank_line_profile(s: &str) -> Vec<usize> {
    strip_escape_codes(s)
        .split('\n')
        .enumerate()
        .filter(|(_, line)| line.is_empty())
        .map(|(i, _)| i)
        .collect()
}

/// Returns `true` when both strings have blank lines at the same positions.
fn extract_blank_lines(a: &str, b: &str) -> bool {
    blank_line_profile(a) == blank_line_profile(b)
}

/// Maximum visible line width, in characters, after ANSI stripping.
fn max_width(s: &str) -> usize {
    strip_escape_codes(s)
        .split('\n')
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

/// Returns `true` when both strings have the same maximum visible width.
fn extract_width(a: &str, b: &str) -> bool {
    max_width(a) == max_width(b)
}

/// Collects each SGR sequence paired with the visible offset it appears at.
fn styling_profile(s: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut visible = 0usize;
    let mut chars = s.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '\x1b' && matches!(chars.peek(), Some((_, '['))) {
            // Consume the '['.
            chars.next();
            let seq_start = idx;
            let mut seq_end = idx + 2;
            for (i, c) in chars.by_ref() {
                seq_end = i + c.len_utf8();
                if c == 'm' {
                    break;
                }
                if c.is_ascii_alphabetic() {
                    // A non-SGR CSI terminator; stop without recording.
                    break;
                }
            }
            let seq = &s[seq_start..seq_end];
            if seq.ends_with('m') {
                out.push((visible, seq.to_string()));
            }
        } else {
            // Assumes well-formed CSI-only escape input: a bare or malformed
            // `\x1b` reaches here and is miscounted as a visible column.
            visible += 1;
        }
    }
    out
}

/// Returns `true` when both strings carry identical styling at identical
/// visible offsets.
fn extract_styling(a: &str, b: &str) -> bool {
    styling_profile(a) == styling_profile(b)
}

/// Returns `true` when the `RECORD_DRIFT` environment variable, trimmed and
/// lowercased, is `1`, `true`, or `yes`.
fn record_mode() -> bool {
    match std::env::var("RECORD_DRIFT") {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        ),
        Err(_) => false,
    }
}

/// Formats a `DriftKey` plus its `Verdict` as a pasteable `KNOWN_DRIFT`
/// tuple literal.
fn ledger_line(key: &DriftKey, verdict: Verdict) -> String {
    format!(
        "    ({:?}, {:?}, Facet::{:?}, Verdict::{:?}),",
        key.component, key.scenario, key.facet, verdict
    )
}

/// Truncates `s` to roughly `max` characters for panic-message previews.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

#[test]
fn render_matches_bespoke() {
    let known: BTreeSet<DriftKey> = KNOWN_DRIFT
        .iter()
        .map(|(c, s, f, _)| DriftKey {
            component: c,
            scenario: s,
            facet: *f,
        })
        .collect();
    assert_eq!(
        known.len(),
        KNOWN_DRIFT.len(),
        "duplicate KNOWN_DRIFT entry"
    );

    // Verdict is metadata: it never affects regression/fixed detection, it is
    // only carried through to `RECORD_DRIFT` output so hand-assigned
    // classifications survive regeneration.
    let verdicts: BTreeMap<DriftKey, Verdict> = KNOWN_DRIFT
        .iter()
        .map(|(c, s, f, v)| {
            (
                DriftKey {
                    component: c,
                    scenario: s,
                    facet: *f,
                },
                *v,
            )
        })
        .collect();

    let mut live = BTreeSet::<DriftKey>::new();
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            for facet in ALL_FACETS {
                let drifts = match facet {
                    Facet::Exact => !extract_exact(&bespoke, &tree),
                    Facet::Text => !extract_text(&bespoke, &tree),
                    Facet::Indent => !extract_indent(&bespoke, &tree),
                    Facet::BlankLines => !extract_blank_lines(&bespoke, &tree),
                    Facet::Width => !extract_width(&bespoke, &tree),
                    Facet::Styling => !extract_styling(&bespoke, &tree),
                };
                if drifts {
                    live.insert(DriftKey {
                        component: case.name,
                        scenario: scenario.name,
                        facet,
                    });
                }
            }
        }
    }

    if record_mode() {
        println!("=== BEGIN KNOWN_DRIFT (paste between `&[` and `]`) ===");
        for key in &live {
            let verdict = verdicts.get(key).copied().unwrap_or(Verdict::TreeBehind);
            println!("{}", ledger_line(key, verdict));
        }
        println!("=== END KNOWN_DRIFT ({} entries) ===", live.len());
        return;
    }

    let unrecorded: Vec<&DriftKey> = live.difference(&known).collect();
    let fixed: Vec<&DriftKey> = known.difference(&live).collect();

    if unrecorded.is_empty() && fixed.is_empty() {
        return;
    }

    let mut msg = format!(
        "live drift: {}, known: {}, unrecorded: {}, fixed: {}\n",
        live.len(),
        known.len(),
        unrecorded.len(),
        fixed.len(),
    );

    if !unrecorded.is_empty() {
        msg.push_str("\nREGRESSION / unrecorded drift:\n");
        for key in unrecorded.iter().take(5) {
            msg.push_str(&format!(
                "  {:?} / {:?} / {:?}\n",
                key.component, key.scenario, key.facet
            ));
            if matches!(key.facet, Facet::Exact | Facet::Text) {
                let (bespoke, tree) = rendered_pair(key);
                msg.push_str(&format!("    bespoke: {:?}\n", truncate(&bespoke, 200)));
                msg.push_str(&format!("    tree:    {:?}\n", truncate(&tree, 200)));
            }
        }
        if unrecorded.len() > 5 {
            msg.push_str(&format!("  … and {} more\n", unrecorded.len() - 5));
        }
    }

    if !fixed.is_empty() {
        msg.push_str("\nFIXED — remove from KNOWN_DRIFT:\n");
        for key in &fixed {
            let verdict = verdicts.get(*key).copied().unwrap_or(Verdict::TreeBehind);
            msg.push_str(&format!("{}\n", ledger_line(key, verdict)));
        }
    }

    panic!("{msg}");
}

/// Re-renders the component/scenario pair behind a `DriftKey` for previews.
fn rendered_pair(key: &DriftKey) -> (String, String) {
    for case in component_cases() {
        if case.name != key.component {
            continue;
        }
        for scenario in scenarios() {
            if scenario.name == key.scenario {
                return (case.render)(&scenario);
            }
        }
    }
    panic!(
        "rendered_pair: no matrix cell for component {:?} / scenario {:?} \
         — KNOWN_DRIFT is out of sync with the live matrix",
        key.component, key.scenario
    );
}
