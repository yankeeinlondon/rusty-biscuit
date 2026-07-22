//! Criterion benchmarks for the 2026-07-15 performance follow-up Phase 10
//! (the remaining Finding-35 residual sub-items).
//!
//! Inputs are the committed, hashed manifest fixtures (Architecture Decision A)
//! so the measured bytes are frozen and reproducible. Each sub-item declares its
//! own target and control groups; the plan forbids one aggregate number that
//! could conceal a per-item no-win or regression.
//!
//! ## Why baseline and candidate are measured in the same run
//!
//! `--save-baseline` / `--baseline` compares two *separate* Criterion runs, which
//! is only sound on a quiet host. The measurement host for this phase is shared
//! and heavily loaded (load average ~30 with a concurrent Spotlight index and
//! parallel `rustc` jobs), and identical code re-measured across runs drifted by
//! +50%. Cross-run comparison cannot resolve a control's few-percent delta under
//! that drift.
//!
//! So each sub-item ships a `*_baseline` function holding a faithful copy of the
//! pre-optimization algorithm next to the `*_candidate` function calling the
//! shipped code. Both are sampled in the same process under the same thermal and
//! scheduling conditions, so the baseline/candidate *ratio* stays meaningful even
//! when absolute numbers move. The baseline copies are pinned to the
//! pre-change implementation by a differential equivalence test
//! (`engine::tests::finding_35_2`), so they cannot silently drift from the
//! algorithm they claim to represent.
//!
//! ## 35.2 — `relevel_with_overflow`
//!
//! - `f35_2_relevel_prefix_toc_large` (target) — `toc_large` (1 H1 + 1000 H2 over
//!   ~80 KB) releveled to H2. Every heading takes the prefix-rewrite branch, so
//!   the baseline copies the whole ~80 KB document once per heading and rescans
//!   `content[..start]` once per heading for its line number.
//! - `f35_2_relevel_overflow_toc_large` (target) — the same fixture releveled to
//!   H6, pushing all 1000 H2s past H6 into the bold-text overflow branch. Covers
//!   the warning-emitting path and its span-replacing output construction.
//! - `f35_2_relevel_extract_only` (target) — `toc_large` releveled to its own root
//!   level (H1). This returns on the `adjustment == 0` fast path, but only *after*
//!   `extract_headings` has run, so it isolates the per-heading line-number rescan
//!   from the output-construction change. It is a target, not a control.
//! - `f35_2_relevel_no_headings` (control) — a heading-free body of the same byte
//!   scale. Nothing on this path changed (no line is ever requested, so the
//!   deferred offset table is never built); baseline and candidate must stay at
//!   parity.
//!
//! 35.6 (`normalize_body_rhythm`) is measured by an in-crate harness instead:
//! the function is private, and exposing it for a bench would be exactly the
//! public API addition the standing contract bars. See
//! `benchmarks/raw/f35-residuals/run-20260716T160000/f35_6-rhythm-profile.txt`.
//!
//! ```text
//! cargo bench -p darkmatter --bench phase10_residuals
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::compose::transclusion::relevel_with_overflow;
use darkmatter::markdown::normalize::HeadingLevel;
use pulldown_cmark::{Event, HeadingLevel as PulldownHeadingLevel, Parser, Tag, TagEnd};
use std::hint::black_box;
use std::path::PathBuf;

/// Reads a committed manifest fixture by stem.
fn fixture_text(stem: &str) -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// 35.2 baseline: the pre-optimization `relevel_with_overflow`.
//
// Kept byte-faithful to the algorithm this phase replaced: per-heading line
// numbers via `content[..start].lines().count() + 1`, replacements sorted
// descending, and one whole-document rebuild per replacement.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
struct BaselineLevel(u8);

impl BaselineLevel {
    fn hash_count(self) -> usize {
        self.0 as usize
    }
}

struct BaselineHeading {
    level: BaselineLevel,
    title: String,
    line: usize,
    start: usize,
    end: usize,
}

fn baseline_level(level: PulldownHeadingLevel) -> BaselineLevel {
    BaselineLevel(match level {
        PulldownHeadingLevel::H1 => 1,
        PulldownHeadingLevel::H2 => 2,
        PulldownHeadingLevel::H3 => 3,
        PulldownHeadingLevel::H4 => 4,
        PulldownHeadingLevel::H5 => 5,
        PulldownHeadingLevel::H6 => 6,
    })
}

fn baseline_extract_headings(content: &str) -> Vec<BaselineHeading> {
    let mut headings = Vec::new();
    let mut current: Option<(BaselineLevel, String, usize)> = None;

    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((baseline_level(level), String::new(), range.start));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, title, _)) = current.as_mut() {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title, start)) = current.take() {
                    let line = content[..start].lines().count() + 1;
                    headings.push(BaselineHeading {
                        level,
                        title,
                        line,
                        start,
                        end: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

enum BaselineReplacement {
    Prefix {
        start: usize,
        old_level: BaselineLevel,
        new_level: BaselineLevel,
    },
    Overflow {
        start: usize,
        end: usize,
        title: String,
        line: usize,
        new_level_raw: u8,
    },
}

fn baseline_relevel(content: &str, target: BaselineLevel) -> (String, Vec<String>) {
    let headings = baseline_extract_headings(content);
    if headings.is_empty() {
        return (content.to_string(), Vec::new());
    }

    let root = headings[0].level;
    let adjustment = target.0 as i8 - root.0 as i8;
    if adjustment == 0 {
        return (content.to_string(), Vec::new());
    }

    let mut replacements = Vec::new();
    let mut warnings = Vec::new();

    for heading in &headings {
        let new_level_raw = heading.level.0 as i8 + adjustment;
        if (1..=6).contains(&new_level_raw) {
            replacements.push(BaselineReplacement::Prefix {
                start: heading.start,
                old_level: heading.level,
                new_level: BaselineLevel(new_level_raw as u8),
            });
        } else {
            replacements.push(BaselineReplacement::Overflow {
                start: heading.start,
                end: heading.end,
                title: heading.title.clone(),
                line: heading.line,
                new_level_raw: new_level_raw.max(7) as u8,
            });
        }
    }

    replacements.sort_by(|left, right| {
        let left_start = match left {
            BaselineReplacement::Prefix { start, .. }
            | BaselineReplacement::Overflow { start, .. } => *start,
        };
        let right_start = match right {
            BaselineReplacement::Prefix { start, .. }
            | BaselineReplacement::Overflow { start, .. } => *start,
        };
        right_start.cmp(&left_start)
    });

    let mut result = content.to_string();

    for replacement in replacements {
        match replacement {
            BaselineReplacement::Prefix {
                start,
                old_level,
                new_level,
            } => {
                let prefix_end = start + old_level.hash_count();
                let replacement = "#".repeat(new_level.hash_count());
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    replacement,
                    &result[prefix_end..]
                );
            }
            BaselineReplacement::Overflow {
                start,
                end,
                title,
                line,
                new_level_raw,
            } => {
                let bold_block = format!("\n\n**{}**\n\n", title.trim());
                result = format!("{}{}{}", &result[..start], bold_block, &result[end..]);
                warnings.push(format!(
                    "Heading overflow at line {line}: converted to bold text (would become H{new_level_raw})"
                ));
            }
        }
    }

    (result, warnings)
}

fn bench_relevel(c: &mut Criterion) {
    let heavy = fixture_text("toc_large");
    let no_headings = heavy.replace('#', "");

    let mut group = c.benchmark_group("f35_2_relevel_prefix_toc_large");
    group.bench_function("baseline", |b| {
        b.iter(|| {
            black_box(baseline_relevel(
                black_box(&heavy),
                black_box(BaselineLevel(2)),
            ))
        })
    });
    group.bench_function("candidate", |b| {
        b.iter(|| {
            black_box(relevel_with_overflow(
                black_box(&heavy),
                black_box(HeadingLevel::H2),
            ))
        })
    });
    group.finish();

    let mut group = c.benchmark_group("f35_2_relevel_overflow_toc_large");
    group.bench_function("baseline", |b| {
        b.iter(|| {
            black_box(baseline_relevel(
                black_box(&heavy),
                black_box(BaselineLevel(6)),
            ))
        })
    });
    group.bench_function("candidate", |b| {
        b.iter(|| {
            black_box(relevel_with_overflow(
                black_box(&heavy),
                black_box(HeadingLevel::H6),
            ))
        })
    });
    group.finish();

    let mut group = c.benchmark_group("f35_2_relevel_extract_only");
    group.bench_function("baseline", |b| {
        b.iter(|| {
            black_box(baseline_relevel(
                black_box(&heavy),
                black_box(BaselineLevel(1)),
            ))
        })
    });
    group.bench_function("candidate", |b| {
        b.iter(|| {
            black_box(relevel_with_overflow(
                black_box(&heavy),
                black_box(HeadingLevel::H1),
            ))
        })
    });
    group.finish();

    let mut group = c.benchmark_group("f35_2_relevel_no_headings");
    group.bench_function("baseline", |b| {
        b.iter(|| {
            black_box(baseline_relevel(
                black_box(&no_headings),
                black_box(BaselineLevel(2)),
            ))
        })
    });
    group.bench_function("candidate", |b| {
        b.iter(|| {
            black_box(relevel_with_overflow(
                black_box(&no_headings),
                black_box(HeadingLevel::H2),
            ))
        })
    });
    group.finish();
}

criterion_group!(benches, bench_relevel);
criterion_main!(benches);
