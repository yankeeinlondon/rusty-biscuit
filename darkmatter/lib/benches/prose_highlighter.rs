//! Criterion benchmark for prose highlighter initialization.
//!
//! [`ProseHighlighter::new`] precomputes a `syntect` `Highlighter` from the
//! theme; this benchmark guards that one-time construction cost. It replaces a
//! former unit test that asserted a hard `< 1ms` wall-clock bound — an absolute
//! timing assertion that fails unpredictably on loaded CI runners. Criterion
//! measures the cost statistically so a regression surfaces as a `change:`
//! delta instead of a flaky pass/fail.
//!
//! Compare before/after a refactor with explicit baselines:
//!
//! ```text
//! cargo bench -p darkmatter --bench prose_highlighter -- --save-baseline before
//! # ...refactor...
//! cargo bench -p darkmatter --bench prose_highlighter -- --baseline before
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::highlighting::prose::ProseHighlighter;
use std::hint::black_box;
use two_face::theme::{EmbeddedThemeName, extra};

fn bench_prose_highlighter_new(c: &mut Criterion) {
    // Mirrors `themes::load_theme(ThemePair::Github, ColorMode::Dark)`: the
    // GitHub-dark pairing resolves to two-face's ColdarkDark embedded theme.
    let theme = extra().get(EmbeddedThemeName::ColdarkDark).clone();

    c.bench_function("prose_highlighter/new", |b| {
        b.iter(|| black_box(ProseHighlighter::new(black_box(&theme))));
    });
}

criterion_group!(benches, bench_prose_highlighter_new);
criterion_main!(benches);
