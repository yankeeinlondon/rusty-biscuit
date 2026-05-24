//! Criterion benchmark for prose highlighter initialization.
//!
//! [`ProseHighlighter::new`] precomputes a `syntect` `Highlighter` from the
//! theme; this benchmark guards that one-time construction cost. It replaces a
//! former unit test that asserted a hard `< 1ms` wall-clock bound — an absolute
//! timing assertion that fails unpredictably on loaded CI runners. Criterion
//! measures the cost statistically so a regression surfaces as a `change:`
//! delta instead of a flaky pass/fail.
//!
//! The benchmark is parameterized across several themes so each result line is
//! self-documenting (`prose_highlighter/new/<Theme>`). `Highlighter::new` cost
//! scales with a theme's scope-selector count, so spanning a range of themes
//! keeps the guard honest. `ColdarkDark` is the theme darkmatter resolves for
//! the GitHub-dark pairing in production.
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench prose_highlighter -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench prose_highlighter -- --baseline before
//! ```

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use darkmatter::markdown::highlighting::prose::ProseHighlighter;
use std::hint::black_box;
use two_face::theme::{EmbeddedThemeName, extra};

/// Themes spanning a range of scope-selector counts. `ColdarkDark` is the
/// production GitHub-dark theme; the rest broaden coverage of init cost.
const THEMES: &[(&str, EmbeddedThemeName)] = &[
    ("ColdarkDark", EmbeddedThemeName::ColdarkDark),
    ("Nord", EmbeddedThemeName::Nord),
    ("Dracula", EmbeddedThemeName::Dracula),
    ("MonokaiExtended", EmbeddedThemeName::MonokaiExtended),
    ("SolarizedDark", EmbeddedThemeName::SolarizedDark),
];

fn bench_prose_highlighter_new(c: &mut Criterion) {
    let theme_set = extra();
    let mut group = c.benchmark_group("prose_highlighter/new");

    for (label, name) in THEMES {
        let theme = theme_set.get(*name).clone();
        group.bench_with_input(BenchmarkId::from_parameter(label), &theme, |b, theme| {
            b.iter(|| black_box(ProseHighlighter::new(black_box(theme))));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_prose_highlighter_new);
criterion_main!(benches);
