//! Criterion benchmarks for the 2026-07-15 performance follow-up Phase 6
//! (Findings 11–14): frontmatter/body interpolation and text replacement.
//!
//! Inputs are the committed, hashed manifest fixtures (Architecture Decision A)
//! so the measured bytes are frozen and reproducible. Each stage is isolated
//! via [`ComposeOptions::only`] so a per-finding delta points at exactly the
//! path it changed:
//!
//! - `frontmatter_interpolation_heavy` — the wide (30-key) + deep (15-link)
//!   frontmatter dependency graph (F11 incremental fixpoint, F12 borrowed
//!   context).
//! - `body_interpolation_heavy` — body `{{ }}` including nested interpolation
//!   over the same fixture (F14).
//! - `body_interpolation_no_expressions` — the 1000-heading TOC fixture, which
//!   contains **no** `{{ }}`; exercises F14's scan fast-path (the MarkdownAware
//!   pulldown-cmark code-region parse is skipped entirely).
//! - `text_replacement_heavy` — the 43-rule / ~1600-occurrence `replace:`
//!   fixture (F13 leftmost-longest automaton vs the per-character scan).
//!
//! Compare before/after with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench phase6_interpolation -- --save-baseline before
//! # ...change...
//! cargo bench -p darkmatter --bench phase6_interpolation -- --baseline before
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::interpolation::ExpressionFinder;
use darkmatter::markdown::compose::replacement::apply_replacements;
use darkmatter::markdown::compose::{
    ComposeOperation, ComposeOptions, EffectiveState, EffectiveStateBuilder,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hint::black_box;
use std::path::PathBuf;

/// Loads a committed manifest fixture by stem.
fn fixture(stem: &str) -> Markdown {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()));
    Markdown::from(text.as_str())
}

/// Composes `md` with only `op` enabled, discarding the result under a black box.
fn compose_stage(md: &Markdown, op: ComposeOperation) {
    let options = ComposeOptions::new().only(&[op]);
    let (composed, report) = md
        .compose_with(options)
        .expect("compose must not fault");
    black_box((composed, report));
}

/// Reads a fixture file's raw text.
fn fixture_text(stem: &str) -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture readable: {e}"))
}

/// Builds an [`EffectiveState`] carrying the `replace_heavy` rule set (43 rules:
/// 40 same-length + `TOKEN`, `TOKEN_01`, `TOKEN_01_EXTRA` overlaps).
fn replace_heavy_map() -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    for i in 1..=40 {
        map.insert(format!("TOKEN_{i:02}"), json!(format!("value-{i:02}")));
    }
    map.insert("TOKEN".to_string(), json!("short"));
    map.insert("TOKEN_01_EXTRA".to_string(), json!("longer"));
    map
}

fn replace_heavy_state() -> EffectiveState {
    let map = replace_heavy_map();
    let mut fm = HashMap::new();
    fm.insert("replace".to_string(), Value::Object(map));
    EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .expect("state builds")
}

/// The `replace_heavy` body (everything after the closing frontmatter fence) —
/// ~800 lines, ~1600 replacement occurrences.
fn replace_heavy_body() -> String {
    let text = fixture_text("replace_heavy");
    // Skip the leading `---` … `---` frontmatter block.
    let mut parts = text.splitn(3, "---\n");
    let _ = parts.next();
    let _ = parts.next();
    parts.next().expect("body after frontmatter").to_string()
}

fn bench_phase6(c: &mut Criterion) {
    let interp = fixture("compose_interpolation_heavy");
    let toc = fixture("toc_large");
    let replace = fixture("replace_heavy");

    let mut group = c.benchmark_group("phase6_interpolation");
    group.sample_size(50);

    group.bench_function("frontmatter_interpolation_heavy", |b| {
        b.iter(|| compose_stage(black_box(&interp), ComposeOperation::FrontmatterInterpolation));
    });
    group.bench_function("body_interpolation_heavy", |b| {
        b.iter(|| compose_stage(black_box(&interp), ComposeOperation::Interpolation));
    });
    group.bench_function("body_interpolation_no_expressions", |b| {
        b.iter(|| compose_stage(black_box(&toc), ComposeOperation::Interpolation));
    });
    group.bench_function("text_replacement_heavy", |b| {
        b.iter(|| compose_stage(black_box(&replace), ComposeOperation::TextReplacement));
    });

    // Direct `apply_replacements` measurement (Finding 13): builds the state
    // once so the number isolates the matcher (leftmost-longest automaton vs the
    // old per-character `starts_with` scan) from compose-context capture, which
    // otherwise dominates the whole-pipeline timings above.
    let replace_state = replace_heavy_state();
    let replace_body = replace_heavy_body();
    group.bench_function("apply_replacements_direct", |b| {
        b.iter(|| {
            let (out, count) =
                apply_replacements(black_box(&replace_body), black_box(&replace_state));
            black_box((out, count));
        });
    });

    // F14 isolation: for an interpolation-free body, the baseline body-scan runs
    // `ExpressionFinder::new(&body).find_all()` — a full MarkdownAware
    // pulldown-cmark code-region parse — whereas the candidate short-circuits on
    // `body.contains("{{")`. These two functions quantify exactly the per-compose
    // work F14 removes on a no-`{{` body (the 1000-heading TOC fixture).
    let toc_body = fixture_text("toc_large");
    group.bench_function("f14_baseline_markdown_scan", |b| {
        b.iter(|| black_box(ExpressionFinder::new(black_box(&toc_body)).find_all()));
    });
    group.bench_function("f14_candidate_contains_guard", |b| {
        b.iter(|| black_box(black_box(&toc_body).contains("{{")));
    });

    group.finish();

    bench_f13_scan_and_replace(c, &replace_body);
}

/// A rule for [`baseline_scan_and_replace`], pinned to the pre-Finding-13 shape
/// (it carried `key_char_len` because the retired scanner advanced by chars).
struct BaselineRule {
    key: String,
    key_char_len: usize,
    value: String,
}

/// Rebuilds the rule set exactly as `build_replacement_rules` does — same
/// filtering, same scalar coercion, same *key length descending, then key
/// ascending* ordering, which the retired scanner depended on for
/// longest-match-wins.
fn baseline_rules() -> Vec<BaselineRule> {
    let mut rules: Vec<BaselineRule> = replace_heavy_map()
        .iter()
        .filter(|(key, _)| !key.is_empty())
        .filter_map(|(key, value)| {
            let value = match value {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Array(_) | Value::Object(_) => return None,
            };
            Some(BaselineRule {
                key: key.clone(),
                key_char_len: key.chars().count(),
                value,
            })
        })
        .collect();
    rules.sort_by(|a, b| match b.key.len().cmp(&a.key.len()) {
        std::cmp::Ordering::Equal => a.key.cmp(&b.key),
        other => other,
    });
    rules
}

/// The pre-Finding-13 `scan_and_replace`, copied verbatim from
/// `markdown/compose/replacement.rs` at commit `b425fb466`: a per-character
/// cursor that retried every rule via `starts_with` at every offset
/// (`O(content × rules × key_len)`).
///
/// It is pinned here rather than reached by checking out `b425fb466` because
/// this host is shared and heavily loaded: cross-run Criterion baselines drift
/// by tens of percent, so baseline and candidate must be sampled interleaved in
/// one process for the ratio to mean anything. Kept honest by the equivalence
/// assertion in [`bench_f13_scan_and_replace`].
fn baseline_scan_and_replace(content: &str, rules: &[BaselineRule]) -> (String, usize) {
    let mut result = String::with_capacity(content.len());
    let mut replacement_count = 0;

    let mut char_starts: Vec<usize> = content.char_indices().map(|(idx, _)| idx).collect();
    char_starts.push(content.len());
    let content_len = char_starts.len().saturating_sub(1);
    let mut pos = 0;

    while pos < content_len {
        let byte_pos = char_starts[pos];
        let remaining = &content[byte_pos..];

        let matched = rules.iter().find(|rule| remaining.starts_with(&rule.key));

        if let Some(rule) = matched {
            result.push_str(&rule.value);
            replacement_count += 1;
            pos += rule.key_char_len;
        } else {
            let next_byte_pos = char_starts[pos + 1];
            result.push_str(&content[byte_pos..next_byte_pos]);
            pos += 1;
        }
    }

    (result, replacement_count)
}

/// Finding 13's baseline/candidate pair, sampled **interleaved in one process**
/// so both see identical thermal and scheduling conditions.
///
/// The `apply_replacements_direct` bench above measures only the candidate, so
/// it cannot on its own support a speed-up claim without a second, separately
/// compiled run — which this host's drift makes unsound.
fn bench_f13_scan_and_replace(c: &mut Criterion, body: &str) {
    let rules = baseline_rules();
    let state = replace_heavy_state();

    // Equivalence gate: refuse to report a ratio between two algorithms that do
    // not agree on this body. Guards against the pinned copy silently drifting
    // from the algorithm it claims to represent.
    let (baseline_out, baseline_count) = baseline_scan_and_replace(body, &rules);
    let (candidate_out, candidate_count) = apply_replacements(body, &state);
    assert_eq!(
        baseline_out, candidate_out,
        "F13 baseline and candidate must produce byte-identical output"
    );
    assert_eq!(
        baseline_count, candidate_count,
        "F13 baseline and candidate must report the same replacement count"
    );

    let mut group = c.benchmark_group("f13_scan_and_replace");
    group.sample_size(50);
    group.bench_function("baseline", |b| {
        b.iter(|| {
            black_box(baseline_scan_and_replace(
                black_box(body),
                black_box(&rules),
            ))
        });
    });
    group.bench_function("candidate", |b| {
        b.iter(|| black_box(apply_replacements(black_box(body), black_box(&state))));
    });
    group.finish();
}

criterion_group!(benches, bench_phase6);
criterion_main!(benches);
