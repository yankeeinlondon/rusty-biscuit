//! Phase 1 baseline benchmarks for the `md clean` hot paths.
//!
//! Captures the pre-change cost of the two common cases the
//! invalid-frontmatter feature must not regress (see
//! `darkmatter/features/2026-07-14-invalid-frontmatter/`):
//!
//! * `no_frontmatter` — a representative document with no frontmatter block.
//!   The v1 contract is "no frontmatter → zero YAML/schema/trigger work".
//! * `clean_frontmatter` — a representative document whose frontmatter is
//!   already standards-based. The v1 contract is "already-clean frontmatter
//!   parses once and is not reparsed".
//!
//! Each case times the full library pipeline the non-save CLI path runs
//! (`try_from_content` → `cleanup` → `as_string`) plus the parse stage alone,
//! so a Phase 7 regression can be attributed to parsing versus cleanup.
//!
//! ```text
//! cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline phase1-before
//! # ...implement phases 2-6...
//! cargo bench -p darkmatter --bench clean_hot_paths -- --baseline phase1-before
//! ```
//!
//! The second group, `clean_list_budgets`, serves the fixed-width-lists fix
//! (`darkmatter/fixes/2026-07-13-fixed-width-lists/`). It covers the four
//! fixture classes that specification's performance section names — top-level
//! prose, flat lists, deeply nested lists, and blockquoted task lists — in both
//! default and fixed-width cleanup, so its three budgets can be checked from one
//! baseline/candidate pair.
//!
//! Timings from this group are only admissible on a quiet host; a loaded machine
//! has a noise floor wider than the 10% budget. Use `-- --test` to check the
//! harness runs without claiming any measurement.
//!
//! Fixtures are generated deterministically from constants: no clock, no RNG, no
//! filesystem, so baseline and candidate see byte-identical input.

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::{cleanup_content, reflow_to_width};

/// Representative no-frontmatter document: headings, prose, a nested list,
/// and a fenced code block — the shape an agent emits for a plain note.
const NO_FRONTMATTER: &str = "\
# Meeting Notes

Notes from the weekly sync. Action items are listed below.

## Attendees

- Alice
- Bob
  - Carol (notes)
- Dave

## Discussion

The rollout plan covers three regions. Each region gets a staged deploy:

1. staging validation
2. canary at five percent
3. full rollout

```sh
cargo build --release
cargo nextest run
```

Follow up next week with the metrics review.
";

/// Representative already-clean frontmatter document: a realistic Darkmatter
/// property set over the same body shape.
const CLEAN_FRONTMATTER: &str = "\
---
title: Weekly Sync Notes
tags:
- meetings
- ops
hash: 9f86d081884c7d65-8a2b8f1f3d9a3b1c
style:
  page:
    margin: 2
---

# Meeting Notes

Notes from the weekly sync. Action items are listed below.

## Attendees

- Alice
- Bob
  - Carol (notes)
- Dave

## Discussion

The rollout plan covers three regions. Each region gets a staged deploy:

1. staging validation
2. canary at five percent
3. full rollout

```sh
cargo build --release
cargo nextest run
```

Follow up next week with the metrics review.
";

/// The non-save `md clean` library pipeline: parse, cleanup, serialize.
fn clean_pipeline(source: &str) -> String {
    let mut md = Markdown::try_from_content(source).expect("baseline fixtures must parse");
    md.cleanup();
    md.as_string()
}

fn bench_clean_hot_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("clean_hot_paths");

    group.bench_function("no_frontmatter/full_pipeline", |b| {
        b.iter(|| black_box(clean_pipeline(black_box(NO_FRONTMATTER))))
    });
    group.bench_function("no_frontmatter/parse_only", |b| {
        b.iter(|| {
            black_box(Markdown::try_from_content(black_box(NO_FRONTMATTER)).expect("parse"))
        })
    });
    group.bench_function("clean_frontmatter/full_pipeline", |b| {
        b.iter(|| black_box(clean_pipeline(black_box(CLEAN_FRONTMATTER))))
    });
    group.bench_function("clean_frontmatter/parse_only", |b| {
        b.iter(|| {
            black_box(Markdown::try_from_content(black_box(CLEAN_FRONTMATTER)).expect("parse"))
        })
    });

    group.finish();
}

/// Number of repeated units per fixed-width fixture.
///
/// Sized so each fixture is tens of kilobytes: large enough that per-call
/// constant costs do not dominate the median, small enough that a full
/// Criterion run of the group stays in minutes rather than hours.
const FIXTURE_UNITS: usize = 60;

/// Sentence fragments cycled through the generated fixtures.
///
/// Varying length is what makes wrapping decisions non-uniform; a single
/// repeated sentence would let every reflow block take the same branch.
const PHRASES: [&str; 6] = [
    "the rollout plan covers three regions and each region gets a staged deploy",
    "canary traffic is held at five percent until error budgets recover",
    "follow up with the metrics review before widening the blast radius",
    "the runbook lists rollback steps for every stage of the deployment",
    "owners are paged only when the sustained error rate exceeds the budget",
    "annotate the dashboard so the next on-call can reconstruct the timeline",
];

fn phrase(index: usize) -> &'static str {
    PHRASES[index % PHRASES.len()]
}

/// Top-level prose: wrapped paragraphs with no list containers.
fn prose_fixture() -> String {
    let mut fixture = String::from("# Rollout Notes\n\n");
    for unit in 0..FIXTURE_UNITS {
        let _ = writeln!(
            fixture,
            "Paragraph {unit}: {}\n{}\n{}\n",
            phrase(unit),
            phrase(unit + 1),
            phrase(unit + 2),
        );
    }
    fixture
}

/// Flat unordered list, every item's prose wrapped across source lines.
fn flat_list_fixture() -> String {
    let mut fixture = String::from("# Action Items\n\n");
    for unit in 0..FIXTURE_UNITS {
        let _ = writeln!(
            fixture,
            "- item {unit}: {}\n  {}",
            phrase(unit),
            phrase(unit + 1),
        );
    }
    fixture
}

/// Four-level nested list. Continuation indent grows with depth, which is what
/// exercises per-item hanging-prefix reconstruction rather than a fixed width.
fn nested_list_fixture() -> String {
    let mut fixture = String::from("# Nested Plan\n\n");
    for unit in 0..FIXTURE_UNITS {
        for depth in 0..4usize {
            let indent = " ".repeat(depth * 2);
            let _ = writeln!(
                fixture,
                "{indent}- depth {depth} item {unit}: {}\n{indent}  {}",
                phrase(unit + depth),
                phrase(unit + depth + 1),
            );
        }
    }
    fixture
}

/// Blockquoted task list: composite `>` plus list plus task-box prefixes, the
/// deepest container stack cleanup has to rebuild.
fn blockquoted_tasks_fixture() -> String {
    let mut fixture = String::from("# Review Queue\n\n");
    for unit in 0..FIXTURE_UNITS {
        let box_state = if unit % 2 == 0 { ' ' } else { 'x' };
        let _ = writeln!(
            fixture,
            "> - [{box_state}] task {unit}: {}\n>   {}\n>   - [ ] subtask {unit}: {}\n>     {}",
            phrase(unit),
            phrase(unit + 1),
            phrase(unit + 2),
            phrase(unit + 3),
        );
    }
    fixture
}

/// Width used for every fixed-width case.
///
/// Narrower than the fixtures' natural line length so wrapping actually fires
/// on all four classes, including the deepest nesting level.
const FIXED_WIDTH: usize = 72;

/// The sequence `md clean --fixed-width` runs (`cli/src/commands/clean.rs`,
/// `apply_cleanup`): full cleanup, then reflow of the cleaned text.
fn fixed_width_pipeline(source: &str) -> String {
    reflow_to_width(&cleanup_content(source), FIXED_WIDTH)
}

/// The three specification budgets this group measures:
///
/// * default cleanup within 10% of its pre-fix median, on every fixture;
/// * fixed-width cleanup within 15% of its pre-fix median on the list-heavy
///   fixtures; and
/// * fixed-width cleanup below 2x default cleanup on the same fixture.
///
/// Cases time `cleanup_content` and `reflow_to_width` directly rather than the
/// `Markdown` wrapper used by `clean_hot_paths`, because frontmatter parsing and
/// re-serialization are constant costs shared by both modes and would compress
/// the 2x ratio toward 1.
fn bench_clean_list_budgets(c: &mut Criterion) {
    let fixtures = [
        ("prose", prose_fixture()),
        ("flat_list", flat_list_fixture()),
        ("nested_list", nested_list_fixture()),
        ("blockquoted_tasks", blockquoted_tasks_fixture()),
    ];

    let mut group = c.benchmark_group("clean_list_budgets");
    for (label, fixture) in &fixtures {
        group.throughput(criterion::Throughput::Bytes(fixture.len() as u64));
        group.bench_function(format!("{label}/default_cleanup"), |b| {
            b.iter(|| black_box(cleanup_content(black_box(fixture))))
        });
        group.bench_function(format!("{label}/fixed_width_cleanup"), |b| {
            b.iter(|| black_box(fixed_width_pipeline(black_box(fixture))))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_clean_hot_paths, bench_clean_list_budgets);
criterion_main!(benches);
