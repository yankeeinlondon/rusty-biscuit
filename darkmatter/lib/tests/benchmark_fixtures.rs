//! Guards the performance follow-up's benchmark fixture manifest.
//!
//! The immutable fixture manifest at
//! `darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml`
//! is the single authority for fixture identity (Architecture Decision A). This
//! test recomputes every recorded identity from the committed fixture bytes and
//! asserts they still match — so a fixture edit that would silently invalidate a
//! measured checkpoint fails here instead. It also proves the non-quadratic TOC
//! `line_at_offset` path stays byte-identical to the naive line count over the
//! shipped fixtures (including the 1000-heading large tier).
//!
//! Regenerate the manifest after a deliberate fixture change:
//! `bash generate.sh` (rewrites the fixtures) then
//! `DM_BENCH_EMIT=1 cargo nextest run -p darkmatter --test benchmark_fixtures`
//! (rewrites `manifest.yaml` from the new bytes). Bump `generator.version` in
//! `generate.sh` and the manifest whenever the emitted bytes change.

use std::path::{Path, PathBuf};

use biscuit_file::serde_yaml_ng;
use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::remote::discover_remote_urls_from_expressions;
use darkmatter::markdown::{Markdown, extract_headings};
use serde::{Deserialize, Serialize};

/// Absolute path to the feature-local `benchmarks/` directory.
fn benchmarks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks")
}

fn manifest_path() -> PathBuf {
    benchmarks_dir().join("manifest.yaml")
}

/// One immutable fixture identity. Field order here is the serialized order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureEntry {
    /// Stable fixture id (the file stem).
    id: String,
    /// Path relative to the `benchmarks/` directory.
    path: String,
    /// `generated` — every fixture is produced by `generate.sh`.
    provenance: String,
    /// Exact byte size of the fixture file.
    bytes: u64,
    /// Newline count (structural size).
    lines: usize,
    /// Number of ATX/Setext headings the parser reports (structural count).
    headings: usize,
    /// Darkmatter Markdown-aware frontmatter hash (`hash_frontmatter`), hex.
    frontmatter_hash: String,
    /// Darkmatter Markdown-aware body hash (`hash_body`), hex.
    body_hash: String,
    /// `md hash <file>` frontmatter-body identity (`hash_frontmatter-hash_body`).
    darkmatter_hash: String,
    /// `biscuit-hash` xxHash whole-file byte identity, hex.
    xxhash64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Generator {
    version: String,
    command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Manifest {
    generator: Generator,
    fixtures: Vec<FixtureEntry>,
}

/// Recomputes every fixture identity from committed bytes, in stable id order.
fn compute_fixtures() -> Vec<FixtureEntry> {
    let dir = benchmarks_dir().join("fixtures");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures directory readable")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).expect("fixture readable");
            let text = String::from_utf8(bytes.clone()).expect("fixture is UTF-8");
            let md: Markdown = text.as_str().into();

            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("fixture stem")
                .to_string();
            let fm = md.hash_frontmatter(false);
            let body = md.hash_body(false);

            FixtureEntry {
                id,
                path: format!("fixtures/{}", path.file_name().unwrap().to_str().unwrap()),
                provenance: "generated".to_string(),
                bytes: bytes.len() as u64,
                lines: text.bytes().filter(|&b| b == b'\n').count(),
                headings: extract_headings(&text).len(),
                frontmatter_hash: format!("{fm:016x}"),
                body_hash: format!("{body:016x}"),
                darkmatter_hash: format!("{fm:016x}-{body:016x}"),
                xxhash64: format!("{:016x}", xx_hash_bytes(&bytes)),
            }
        })
        .collect()
}

/// Recomputed fixture identities must match the recorded manifest exactly.
///
/// Set `DM_BENCH_EMIT=1` to rewrite `manifest.yaml` from the current fixture
/// bytes instead of asserting (see the module doc).
#[test]
fn benchmark_manifest_matches_recorded_identities() {
    let fixtures = compute_fixtures();
    assert!(!fixtures.is_empty(), "expected committed fixtures");

    if std::env::var_os("DM_BENCH_EMIT").is_some() {
        let manifest = Manifest {
            generator: Generator {
                version: "1.3.0".to_string(),
                command: "bash generate.sh".to_string(),
            },
            fixtures,
        };
        let header = "# Immutable fixture manifest — 2026-07-15 performance follow-up (AD-A).\n\
                      # Single authority for fixture identity. Regenerate with\n\
                      # `bash generate.sh` then `DM_BENCH_EMIT=1 cargo nextest run -p darkmatter\n\
                      # --test benchmark_fixtures`. Verified by benchmark_fixtures.rs.\n";
        let body = serde_yaml_ng::to_string(&manifest).expect("serialize manifest");
        std::fs::write(manifest_path(), format!("{header}{body}")).expect("write manifest");
        eprintln!("DM_BENCH_EMIT: wrote {}", manifest_path().display());
        return;
    }

    let raw = std::fs::read_to_string(manifest_path()).expect("manifest.yaml present");
    let manifest: Manifest = serde_yaml_ng::from_str(&raw).expect("manifest parses");

    assert_eq!(
        manifest.generator.version, "1.3.0",
        "generator version drifted from the fixture test"
    );
    assert_eq!(
        manifest.fixtures, fixtures,
        "recorded fixture identities do not match recomputed bytes — \
         re-run `bash generate.sh` and `DM_BENCH_EMIT=1 ...` if the change is intentional"
    );
}

/// Naive line for a byte offset: `content[..offset].lines().count() + 1`. This
/// is the formula the non-quadratic `line_at_offset` path must reproduce.
fn naive_line(content: &str, offset: usize) -> usize {
    content[..offset].lines().count() + 1
}

/// Over every shipped fixture, each heading's reported line must equal the
/// naive line count at its span start — guarding the non-quadratic TOC path at
/// scale (the large tier carries 1000 headings).
#[test]
fn toc_line_positions_match_naive_over_fixtures() {
    let dir = benchmarks_dir().join("fixtures");
    let mut checked_headings = 0usize;
    for entry in std::fs::read_dir(&dir).expect("fixtures readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let content = std::fs::read_to_string(&path).expect("fixture readable");
        for heading in extract_headings(&content) {
            assert_eq!(
                heading.line,
                naive_line(&content, heading.heading_span.start),
                "line mismatch in {} for heading {:?}",
                path.display(),
                heading.title
            );
            checked_headings += 1;
        }
    }
    // The large tier alone contributes 1000 headings; guard against a fixture
    // set that silently stopped exercising scale.
    assert!(
        checked_headings >= 1000,
        "expected the large TOC tier's headings to be exercised, saw {checked_headings}"
    );
}

/// Over every shipped fixture, each discovered remote URL must actually appear
/// on the line the discovery reports — guarding the Finding 33 offset-table
/// path against the whole committed corpus at scale (the `remote_heavy` tier
/// alone carries 300 expressions, each further into the document than the last).
///
/// This checks the reported line against the fixture's real text rather than
/// recomputing the offset arithmetic, so it stays an independent oracle.
#[test]
fn remote_discovery_line_positions_match_fixture_text() {
    let dir = benchmarks_dir().join("fixtures");
    let mut checked_urls = 0usize;
    for entry in std::fs::read_dir(&dir).expect("fixtures readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let content = std::fs::read_to_string(&path).expect("fixture readable");
        let source = ComposeSource::File(path.clone());
        for found in discover_remote_urls_from_expressions(&content, &source) {
            let line_text = content
                .lines()
                .nth(found.line - 1)
                .unwrap_or_else(|| panic!("line {} exists in {}", found.line, path.display()));
            assert!(
                line_text.contains(found.url.as_str()),
                "{} reported {} on line {} ({line_text:?}), which does not contain it",
                path.display(),
                found.url,
                found.line,
            );
            checked_urls += 1;
        }
    }
    assert!(
        checked_urls >= 300,
        "expected the remote_heavy tier's expressions to be exercised, saw {checked_urls}"
    );
}

proptest::proptest! {
    /// Property coverage for the non-quadratic path: for arbitrary line-structured
    /// content, every heading's reported line equals the naive line count at its
    /// span start. Lines mix prose, headings, and blank lines so heading offsets
    /// land after varied prefixes (mid-document, after blanks, back-to-back).
    #[test]
    fn prop_extract_headings_line_matches_naive(
        lines in proptest::collection::vec(
            proptest::prop_oneof![
                proptest::string::string_regex("# [A-Za-z0-9 ]{1,12}").unwrap(),
                proptest::string::string_regex("## [A-Za-z0-9 ]{1,12}").unwrap(),
                proptest::string::string_regex("[A-Za-z0-9 ]{0,20}").unwrap(),
                proptest::prelude::Just(String::new()),
            ],
            0..40,
        ),
    ) {
        let content = lines.join("\n");
        for heading in extract_headings(&content) {
            proptest::prop_assert_eq!(
                heading.line,
                naive_line(&content, heading.heading_span.start),
                "line mismatch for heading {:?} in {:?}",
                heading.title,
                content
            );
        }
    }
}
