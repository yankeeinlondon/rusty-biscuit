//! Criterion benchmark for [`DarkmatterSchemas::validate`] across a
//! 1000-document corpus.
//!
//! The benchmark exercises the full hot path that production callers use:
//!
//! 1. Parsing the markdown document into a [`Markdown`] value.
//! 2. Resolving `$schema` and merging baselines via [`DarkmatterSchemas`].
//! 3. Running the cached `jsonschema::Validator` over the frontmatter.
//!
//! It generates a deterministic corpus of 1,000 documents pinned to the same
//! inline SimplifiedSchema so that the validator cache is exercised exactly
//! once per benchmark sample. Splitting the corpus into mostly-valid /
//! mostly-invalid documents prevents an `iter_errors` short-circuit from
//! dominating the timings.
//!
//! The `claudine_cold` and `claudine_warm` groups validate three Claudine
//! documents against the frozen bundle in `benches/fixtures/claudine-schemas/`,
//! discovered as trigger files in a temporary repository's `schemas/` root.
//! A cold iteration first calls
//! [`reset_process_caches_for_bench`](darkmatter::markdown::schemas::reset_process_caches_for_bench)
//! outside the measured work; a warm one reuses everything a previous
//! validation cached. Both build a fresh [`DarkmatterSchemas`] with trigger
//! discovery, as `md schema validate --disable baseline-schema` does.
//!
//! The `resolve_frozen` groups time the frozen recursive and depth-5 unrolled
//! lifecycle fixtures in `tests/fixtures/recursive_schema_frozen/`, wired to
//! the seven trigger-payload keys, and separately the raw file reads and
//! Darkmatter's validator build for each resolved schema.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::triggers::PayloadMemoTally;
use darkmatter::markdown::schemas::validate::ValidatorCache;
use darkmatter::markdown::schemas::{DarkmatterSchemas, reset_process_caches_for_bench};
use serde_yaml_ng::Value as YamlValue;
use std::collections::HashMap;
use std::hint::black_box;
use std::path::{Path, PathBuf};

const CORPUS_SIZE: usize = 1_000;

/// Inline schema used by every generated document. Touches strings, numbers,
/// enums, and arrays so the validator does meaningful work per document.
const SCHEMA_FRONTMATTER: &str = "$schema:\n  \
    title: 'string(required)'\n  \
    slug: 'string(pattern(^[a-z][a-z0-9-]*$);required)'\n  \
    views: 'number(min(0);required)'\n  \
    status: 'enum(draft, published, archived; required)'\n  \
    tags: 'string[]'\n";

/// Anchors schema resolution at the process directory with the process's home
/// and environment, as the context-free constructors did.
fn process_context() -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(std::env::current_dir().expect("process directory"))
}

fn build_corpus() -> Vec<Markdown> {
    (0..CORPUS_SIZE)
        .map(|i| {
            // Roughly 1-in-10 documents fail validation so iter_errors traverses
            // a real problem set rather than always short-circuiting at success.
            let invalid_slug = i % 10 == 0;
            let slug = if invalid_slug {
                format!("INVALID slug {i}")
            } else {
                format!("post-{i}")
            };
            let status = match i % 3 {
                0 => "draft",
                1 => "published",
                _ => "archived",
            };
            let body = format!(
                "---\n{SCHEMA_FRONTMATTER}title: 'Post {i}'\nslug: '{slug}'\n\
                 views: {i}\nstatus: {status}\ntags:\n  - rust\n  - markdown\n---\n\
                 Body of post {i}\n"
            );
            Markdown::from(body.as_str())
        })
        .collect()
}

fn validate_corpus(api: &DarkmatterSchemas, corpus: &[Markdown]) -> (usize, usize) {
    let mut ok = 0usize;
    let mut bad = 0usize;
    for md in corpus {
        let report = api.validate(md).expect("validate must not fault");
        if report.valid {
            ok += 1;
        } else {
            bad += 1;
        }
    }
    (ok, bad)
}

fn bench_validate_corpus(c: &mut Criterion) {
    let corpus = build_corpus();
    let mut group = c.benchmark_group("schema_validation/corpus_1000");
    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));
    group.sample_size(20);

    let context = process_context();
    group.bench_function("validate_1000_docs", |b| {
        b.iter_batched(
            || DarkmatterSchemas::new(context.clone()),
            |api| {
                let (ok, bad) = validate_corpus(black_box(&api), black_box(&corpus));
                black_box((ok, bad));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("validate_1000_docs_warm_cache", |b| {
        let api = DarkmatterSchemas::new(context.clone());
        // Prime the validator cache so each iteration measures hot-path work.
        let _ = validate_corpus(&api, &corpus);
        b.iter(|| {
            let (ok, bad) = validate_corpus(black_box(&api), black_box(&corpus));
            black_box((ok, bad));
        });
    });

    group.finish();
}

/// The frozen Claudine bundle, written into the scenario's `schemas/` root.
const CLAUDINE_SCHEMAS: [(&str, &str); 9] = [
    ("action.yaml", include_str!("fixtures/claudine-schemas/action.yaml")),
    ("claudine.yaml", include_str!("fixtures/claudine-schemas/claudine.yaml")),
    ("globals.yaml", include_str!("fixtures/claudine-schemas/globals.yaml")),
    ("inline-compose.yaml", include_str!("fixtures/claudine-schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("fixtures/claudine-schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("fixtures/claudine-schemas/linking.yaml")),
    ("review.yaml", include_str!("fixtures/claudine-schemas/review.yaml")),
    ("sequence-types.yaml", include_str!("fixtures/claudine-schemas/sequence-types.yaml")),
    ("sequence.yaml", include_str!("fixtures/claudine-schemas/sequence.yaml")),
];

/// No trigger matches this document.
const NO_TRIGGER_MATCH: &str = "---\ntitle: x\n---\nbody\n";

/// Document-level lifecycle events only.
const DOCUMENT_EVENTS: &str = r#"---
prompt: Summarize the change
agent: claude
start:
  - info: "starting"
  - say: "starting the review"
success:
  - effect: "crowd-applause"
  - say: "the build is green"
  - when: "fm.notify"
    then:
      - info: "notified"
failure:
  - warn: "giving up"
  - shell: "git status"
---
body
"#;

/// A nested `sequence` and a group, each with events, beside document events.
const NESTED_SEQUENCE_GROUP: &str = r#"---
start:
  - info: "sequence start"
success:
  - say: "all done"
sequence:
  - name: build
    shell: cargo build
    success:
      - info: "built"
  - name: review
    start:
      - info: "nested start"
    sequence:
      - name: lint
        prompt: ./lint.md
        failure:
          - warn: "lint failed"
  - name: fan-out
    group:
      name: checks
      tasks:
        - name: a
          prompt: ./a.md
        - name: b
          prompt: ./b.md
      success:
        - info: "group done"
      failure:
        - warn: "a check failed"
---
body
"#;

/// A temporary repository holding the frozen bundle in its `schemas/` root
/// and one file per benchmarked document.
struct ClaudineScenario {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl ClaudineScenario {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(root.join("schemas")).expect("schemas root");
        for (name, text) in CLAUDINE_SCHEMAS {
            std::fs::write(root.join("schemas").join(name), text).expect("write frozen schema");
        }
        Self { _dir: dir, root }
    }

    /// Writes `text` as `name` and opens it, so the document carries its file
    /// source as production documents do.
    fn document(&self, name: &str, text: &str) -> Markdown {
        let path = self.root.join(name);
        std::fs::write(&path, text).expect("write document");
        Markdown::try_from(path.as_path()).expect("open document")
    }

    /// The document's own context, anchored in the scenario, never the
    /// process directory, home, or environment.
    fn context(&self) -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(&self.root, None, HashMap::new())
            .with_repository_root(&self.root)
    }
}

/// One validation as a fresh request: trigger discovery, then validation.
fn validate_claudine(context: &biscuit_file::FileResolutionContext, document: &Markdown) -> bool {
    DarkmatterSchemas::new(context.clone())
        .with_trigger_discovery()
        .expect("trigger discovery")
        .validate(document)
        .expect("validate must not fault")
        .valid
}

/// What one validation did to the process caches.
#[derive(Debug, PartialEq)]
struct CacheEffect {
    valid: bool,
    validators_cached_before: usize,
    validators_cached_after: usize,
    payload_hits: u64,
    payload_misses: u64,
}

fn observe(context: &biscuit_file::FileResolutionContext, document: &Markdown) -> CacheEffect {
    let probe = DarkmatterSchemas::new(context.clone());
    let validators_cached_before = probe.cache().len();
    let tally = PayloadMemoTally::start();
    let valid = validate_claudine(context, document);
    let counts = tally.counts();
    CacheEffect {
        valid,
        validators_cached_before,
        validators_cached_after: probe.cache().len(),
        payload_hits: counts.hits,
        payload_misses: counts.misses,
    }
}

/// Proves, before any timing, that a reset makes the next validation compile
/// and resolve again and that a repeat reuses both. Asserts cache behavior
/// only, never elapsed time.
fn check_cold_and_warm(case: &str, context: &biscuit_file::FileResolutionContext, document: &Markdown) {
    reset_process_caches_for_bench();
    let cold = observe(context, document);
    let warm = observe(context, document);
    eprintln!("{case}: cold {cold:?}; warm {warm:?}");
    assert_eq!(cold.validators_cached_before, 0, "{case}: a reset empties the validator cache");
    assert_eq!(cold.valid, warm.valid, "{case}: warm and cold verdicts agree");
    assert_eq!(
        warm.validators_cached_after, warm.validators_cached_before,
        "{case}: a warm validation compiles nothing"
    );
    assert_eq!(warm.payload_misses, 0, "{case}: a warm validation resolves no payload");
    if case != "no_trigger_match" {
        assert!(cold.validators_cached_after > 0, "{case}: a cold validation compiles its validators");
        assert!(cold.payload_misses > 0, "{case}: a cold validation resolves its trigger payloads");
    }
}

fn bench_claudine(c: &mut Criterion) {
    let scenario = ClaudineScenario::new();
    let context = scenario.context();
    let cases = [
        ("no_trigger_match", scenario.document("no-trigger.md", NO_TRIGGER_MATCH)),
        ("document_events", scenario.document("document-events.md", DOCUMENT_EVENTS)),
        ("nested_sequence_group", scenario.document("nested-sequence.md", NESTED_SEQUENCE_GROUP)),
    ];
    for (case, document) in &cases {
        check_cold_and_warm(case, &context, document);
    }

    let mut cold = c.benchmark_group("schema_validation/claudine_cold");
    cold.sample_size(20);
    for (case, document) in &cases {
        cold.bench_function(*case, |b| {
            // `PerIteration` runs each reset immediately before its own
            // validation; a larger batch would run several resets first and
            // leave every validation after the first warm.
            b.iter_batched(
                reset_process_caches_for_bench,
                |()| black_box(validate_claudine(&context, black_box(document))),
                BatchSize::PerIteration,
            );
        });
    }
    cold.finish();

    let mut warm = c.benchmark_group("schema_validation/claudine_warm");
    warm.sample_size(20);
    for (case, document) in &cases {
        warm.bench_function(*case, |b| {
            let _ = validate_claudine(&context, document);
            b.iter(|| black_box(validate_claudine(&context, black_box(document))));
        });
    }
    warm.finish();
}

/// The six event keys of the frozen fixture's measurement; `loop` is the seventh.
const FROZEN_EVENTS: [&str; 6] = ["initialize", "start", "blocked", "success", "failure", "finalize"];

/// `(case, lifecycle file)` for each frozen fixture.
const FROZEN_CASES: [(&str, &str); 2] =
    [("unrolled_depth5", "lifecycle.yaml"), ("recursive", "lifecycle-recursive.yaml")];

fn frozen_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recursive_schema_frozen")
}

/// Every event typed as the fixture's `stack`, and `loop` as its `loop`.
fn frozen_wiring(lifecycle: &str) -> YamlValue {
    let mut wiring = serde_yaml_ng::Mapping::new();
    for event in FROZEN_EVENTS {
        wiring.insert(event.into(), format!("stack@./{lifecycle}").into());
    }
    wiring.insert("loop".into(), format!("loop@./{lifecycle}").into());
    YamlValue::Mapping(wiring)
}

/// Resolution (including lowering) of the frozen fixtures, the raw reads of
/// the files it loads, and Darkmatter's validator build for its output, each
/// timed on its own.
fn bench_resolve_frozen(c: &mut Criterion) {
    let dir = frozen_dir();
    let context = biscuit_file::FileResolutionContext::new(&dir);

    let mut resolve = c.benchmark_group("resolve_frozen");
    resolve.sample_size(10);
    for (case, lifecycle) in FROZEN_CASES {
        let wiring = frozen_wiring(lifecycle);
        let resolved = resolve_yaml_schema(&wiring, &dir, &context).expect("the frozen fixture resolves");
        let bytes = serde_json::to_string(&resolved.json_schema).expect("the schema serializes").len();
        eprintln!("resolve_frozen/{case}: {bytes} bytes of serialized JSON");
        resolve.bench_function(case, |b| {
            b.iter(|| black_box(resolve_yaml_schema(black_box(&wiring), &dir, &context).expect("resolves")));
        });
    }
    resolve.finish();

    let mut io = c.benchmark_group("resolve_frozen_io");
    io.sample_size(10);
    for (case, lifecycle) in FROZEN_CASES {
        let files = [dir.join("action.yaml"), dir.join(lifecycle)];
        io.bench_function(case, |b| {
            b.iter(|| {
                for file in &files {
                    black_box(std::fs::read(file).expect("read fixture"));
                }
            });
        });
    }
    io.finish();

    let mut compile = c.benchmark_group("resolve_frozen_compile");
    compile.sample_size(10);
    for (case, lifecycle) in FROZEN_CASES {
        let json = resolve_yaml_schema(&frozen_wiring(lifecycle), &dir, &context)
            .expect("the frozen fixture resolves")
            .json_schema;
        compile.bench_function(case, |b| {
            // A fresh cache per iteration makes every build a miss; its
            // content hash is part of Darkmatter's compile, so it is timed.
            b.iter_batched(
                || ValidatorCache::with_capacity(1),
                |cache| black_box(cache.structural_validator_for(&json).expect("compiles")),
                BatchSize::PerIteration,
            );
        });
    }
    compile.finish();
}

criterion_group!(benches, bench_validate_corpus, bench_claudine, bench_resolve_frozen);
criterion_main!(benches);
