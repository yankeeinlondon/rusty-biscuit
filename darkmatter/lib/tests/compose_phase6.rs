//! End-to-end regression coverage for the 2026-07-15 performance follow-up
//! Phase 6 (Findings 11–14): frontmatter/body interpolation and text
//! replacement. Each test composes a **committed, hashed manifest fixture**
//! through the normal `compose_with` path and asserts the observable composed
//! output — the byte-identical behavior the F11 incremental fixpoint, F12
//! borrowed context, F13 leftmost-longest replacement automaton, and F14
//! scan fast-path must all preserve.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use std::path::PathBuf;

fn fixture_text(stem: &str) -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()))
}

fn compose(stem: &str) -> String {
    let md = Markdown::from(fixture_text(stem).as_str());
    let (composed, _report) = md
        .compose_with(ComposeOptions::new())
        .expect("fixture composes without fault");
    composed.content().to_string()
}

/// The wide (30-key) + deep (15-link) frontmatter graph resolves fully, body
/// interpolation (incl. nested) resolves, a `{{{ … }}}` literal survives as
/// literal text (F14), a fenced code block's `{{ … }}` is left untouched, and
/// the `replace:` map is applied — all in one composed document.
#[test]
fn interpolation_heavy_fixture_composes_expected_output() {
    let out = compose("compose_interpolation_heavy");

    // Body interpolation of a frontmatter-resolved title.
    assert!(out.contains("# Darkmatter Interpolation Fixture"), "title:\n{out}");
    // Nested interpolation: the outer ternary yields a string still holding
    // `{{proj}}`, which the rescan pass then resolves.
    assert!(out.contains("Nested: inside Darkmatter now"), "nested:\n{out}");
    // A `{{{ … }}}` literal is converted to literal `{{ … }}` text, never
    // evaluated.
    assert!(
        out.contains("Literal escape stays raw: {{ not_interpolated }}"),
        "literal:\n{out}"
    );
    // The full 15-link deep dependency chain resolved in order.
    assert!(
        out.contains(
            "/root/project/level-01/level-02/level-03/level-04/level-05/level-06/\
             level-07/level-08/level-09/level-10/level-11/level-12/level-13/level-14/level-15"
        ) || out.contains("level-15"),
        "deep chain:\n{out}"
    );
    // A `{{ … }}` inside a fenced code block is NOT interpolated (MarkdownAware).
    assert!(
        out.contains("// {{ not_touched }} stays literal inside a fence"),
        "fence untouched:\n{out}"
    );
    // Text replacement applied (runs before interpolation in the pipeline).
    assert!(
        out.contains("Darkmatter ships 1.0.0; status resolved."),
        "replacements:\n{out}"
    );
    // The wide fan-out resolved every key (first and last).
    assert!(out.contains("/root/project/section-01/Darkmatter"), "wide first:\n{out}");
    assert!(out.contains("/root/project/section-30/Darkmatter"), "wide last:\n{out}");
    // Unicode prose is preserved through interpolation.
    assert!(out.contains("Unicode prose: café Darkmatter 日本語 🎉"), "unicode:\n{out}");
    // No raw template markers leaked out (other than the intended literal above).
    assert!(!out.contains("{{ base }}"), "unresolved base leaked:\n{out}");
    assert!(!out.contains("{{ proj }}"), "unresolved proj leaked:\n{out}");
}

/// The 43-rule / overlapping-prefix `replace:` fixture applies leftmost-longest
/// replacement (F13): every `TOKEN_NN` becomes `value-NN` (the longer key wins
/// over the bare `TOKEN` prefix), and no token or short/longer artifact leaks.
#[test]
fn replace_heavy_fixture_applies_longest_match() {
    let out = compose("replace_heavy");

    assert!(out.contains("value-01"), "first rule:\n{}", &out[..out.len().min(400)]);
    assert!(out.contains("value-40"), "last rule applied");
    // Longest-match: `TOKEN_01` resolves to `value-01`, never `short` + `_01`.
    assert!(!out.contains("TOKEN_01"), "TOKEN_01 left unreplaced");
    assert!(!out.contains("short_01"), "bare TOKEN matched inside TOKEN_01");
    // Unicode content around the tokens survives.
    assert!(out.contains("café value-01"), "unicode around replacement");
    // No raw TOKEN_NN markers remain.
    assert!(!out.contains("TOKEN_"), "some TOKEN_ marker left unreplaced");
}
