//! Filesystem-detection Criterion benches.
//!
//! These cover the design's highest-risk Sniff paths: git history,
//! monorepo/package discovery, and language scanning. Everything is
//! built on top of the synthetic monorepo fixture so runs are
//! deterministic regardless of host state.

use criterion::{Criterion, Throughput, black_box};
use sniff::filesystem::docs::{detect_blast_radius_docs, detect_docs};
use sniff::filesystem::file_types::scan_file_inventory;
use sniff::filesystem::repo::detect_repo_with_inventory;
use sniff::filesystem::repo::detection::refresh_package_boundaries;
use sniff::filesystem::{
    detect_filesystem_with_request, detect_git_with_request, detect_languages, detect_repo,
    detect_repo_structure,
};
use sniff::request::{FilesystemRequest, GitRequest, RepoRequest};

use crate::support::{fixtures, util};

/// Total markdown documents in the docs-parser fixture used by the
/// `filesystem_docs` group. Sized to surface frontmatter-parse cost
/// without dominating wall-clock time.
const DOCS_FIXTURE_TOTAL: usize = 200;

/// Fraction of the docs fixture that declares a `blast_radius` frontmatter
/// list. The remainder carries typical metadata but no `blast_radius`,
/// so the blast-radius-only parser short-circuits on most files while the
/// full parser still hashes and resolves them.
const DOCS_FIXTURE_WITH_BR: usize = 40;

pub fn register(c: &mut Criterion) {
    let small = fixtures::small_git_repo();
    let large = fixtures::large_monorepo();
    let huge = fixtures::huge_monorepo();
    let langs = fixtures::language_mix_tree();
    let docs = fixtures::docs_repo(DOCS_FIXTURE_TOTAL, DOCS_FIXTURE_WITH_BR);

    // ---------- git ----------
    let mut git_group = util::configure_group(c, "filesystem_git");

    git_group.bench_function("git_summary_small_repo_5_commits", |b| {
        let req = GitRequest::summary();
        b.iter(|| {
            let info = detect_git_with_request(black_box(small.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_full_small_repo_10_commits_stats", |b| {
        let req = GitRequest::full();
        b.iter(|| {
            let info = detect_git_with_request(black_box(small.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_summary_monorepo_5_commits", |b| {
        let req = GitRequest::summary();
        b.iter(|| {
            let info = detect_git_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_full_monorepo_10_commits_stats", |b| {
        let req = GitRequest::full();
        b.iter(|| {
            let info = detect_git_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.finish();

    // ---------- repo / package discovery ----------
    let mut repo_group = util::configure_group(c, "filesystem_repo");

    repo_group.bench_function("repo_structure_monorepo_manifest_discovery", |b| {
        b.iter(|| {
            let info = detect_repo_structure(black_box(large.path())).unwrap();
            black_box(info);
        });
    });

    repo_group.bench_function("repo_with_shared_inventory_monorepo", |b| {
        b.iter(|| {
            let (repo, inventory) = detect_repo_with_inventory(black_box(large.path())).unwrap();
            black_box((repo, inventory));
        });
    });

    // Full per-package detection (language scan, framework heuristics,
    // dependency parsing). This is the hot path that
    // `refresh_package_boundaries` feeds into and the dominant cost of
    // `RepoRequest::full()` on large monorepos.
    repo_group.bench_function("repo_full_monorepo_language_scan", |b| {
        b.iter(|| {
            let info = detect_repo(black_box(large.path())).unwrap();
            black_box(info);
        });
    });

    // Huge monorepo benchmarks stress manifest caching and index normalization.
    // The fixture holds 375 packages (200 Rust, 100 JS, 50 Python, 25 Go); the
    // name says so because a benchmark id that misstates its own workload
    // invalidates every baseline compared against it.
    repo_group.bench_function("repo_structure_huge_375_packages", |b| {
        b.iter(|| {
            let info = detect_repo_structure(black_box(huge.path())).unwrap();
            black_box(info);
        });
    });

    repo_group.bench_function("repo_full_huge_375_packages", |b| {
        b.iter(|| {
            let info = detect_repo(black_box(huge.path())).unwrap();
            black_box(info);
        });
    });

    // Isolated package-enrichment benchmark: structure + inventory are
    // prepared outside the timed loop so only `refresh_package_boundaries`
    // is measured.
    repo_group.bench_function("package_boundary_refresh_huge", |b| {
        let mut repo_info = detect_repo_structure(huge.path()).unwrap().unwrap();
        let inventory = scan_file_inventory(huge.path()).unwrap();
        let mut packages = repo_info.packages.take().unwrap();
        b.iter(|| {
            refresh_package_boundaries(black_box(&mut packages), black_box(Some(&inventory)));
            black_box(&packages);
        });
    });

    repo_group.finish();

    // ---------- file inventory (parallel walker) ----------
    let mut inventory_group = util::configure_group(c, "filesystem_inventory");

    inventory_group.bench_function("file_inventory_walk_small_repo", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(small.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.bench_function("file_inventory_walk_monorepo", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(large.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.bench_function("file_inventory_walk_mixed_langs", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(langs.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.finish();

    // ---------- languages ----------
    let mut lang_group = util::configure_group(c, "filesystem_languages");

    lang_group.bench_function("language_detection_mixed_depths", |b| {
        b.iter(|| {
            let breakdown = detect_languages(black_box(langs.path())).unwrap();
            black_box(breakdown);
        });
    });

    lang_group.bench_function("language_detection_monorepo", |b| {
        b.iter(|| {
            let breakdown = detect_languages(black_box(large.path())).unwrap();
            black_box(breakdown);
        });
    });

    lang_group.finish();

    // ---------- docs parsing modes ----------
    let mut docs_group = util::configure_group(c, "filesystem_docs");
    docs_group.throughput(Throughput::Elements(DOCS_FIXTURE_TOTAL as u64));

    docs_group.bench_function("docs_parse_full_frontmatter_200_files", |b| {
        b.iter(|| {
            let parsed = detect_docs(black_box(docs.path()));
            black_box(parsed);
        });
    });

    docs_group.bench_function("docs_parse_blast_radius_only_40_of_200", |b| {
        b.iter(|| {
            let parsed = detect_blast_radius_docs(black_box(docs.path()));
            black_box(parsed);
        });
    });

    docs_group.finish();

    // ---------- staged filesystem request ----------
    let mut stage_group = util::configure_slow_group(c, "filesystem_staged");

    stage_group.bench_function("staged_filesystem_summary_git_plus_repo", |b| {
        let req = FilesystemRequest::new()
            .git(GitRequest::summary())
            .repo(RepoRequest::structure())
            .without_docs()
            .without_formatting()
            .without_file_inventory();
        b.iter(|| {
            let info =
                detect_filesystem_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    stage_group.bench_function("staged_filesystem_full_all_stages", |b| {
        let req = FilesystemRequest::new();
        b.iter(|| {
            let info =
                detect_filesystem_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    stage_group.finish();
}
