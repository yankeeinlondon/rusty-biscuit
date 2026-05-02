//! Filesystem-detection Criterion benches.
//!
//! These cover the design's highest-risk Sniff paths: git history,
//! monorepo/package discovery, and language scanning. Everything is
//! built on top of the synthetic monorepo fixture so runs are
//! deterministic regardless of host state.

use criterion::{Criterion, black_box};
use sniff::filesystem::file_types::scan_file_inventory;
use sniff::filesystem::repo::detect_repo_with_inventory;
use sniff::filesystem::{
    detect_filesystem_with_request, detect_git_with_request, detect_languages,
    detect_repo_structure,
};
use sniff::request::{FilesystemRequest, GitRequest, RepoRequest};

use crate::support::{fixtures, util};

pub fn register(c: &mut Criterion) {
    let small = fixtures::small_git_repo();
    let large = fixtures::large_monorepo();
    let langs = fixtures::language_mix_tree();

    // ---------- git ----------
    let mut git_group = util::configure_group(c, "filesystem_git");

    git_group.bench_function("git_summary_small", |b| {
        let req = GitRequest::summary();
        b.iter(|| {
            let info = detect_git_with_request(black_box(small.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_full_small", |b| {
        let req = GitRequest::full();
        b.iter(|| {
            let info = detect_git_with_request(black_box(small.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_summary_monorepo", |b| {
        let req = GitRequest::summary();
        b.iter(|| {
            let info = detect_git_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.bench_function("git_full_monorepo", |b| {
        let req = GitRequest::full();
        b.iter(|| {
            let info = detect_git_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    git_group.finish();

    // ---------- repo / package discovery ----------
    let mut repo_group = util::configure_group(c, "filesystem_repo");

    repo_group.bench_function("repo_structure_monorepo", |b| {
        b.iter(|| {
            let info = detect_repo_structure(black_box(large.path())).unwrap();
            black_box(info);
        });
    });

    repo_group.bench_function("repo_with_inventory_monorepo", |b| {
        b.iter(|| {
            let (repo, inventory) = detect_repo_with_inventory(black_box(large.path())).unwrap();
            black_box((repo, inventory));
        });
    });

    repo_group.finish();

    // ---------- file inventory (parallel walker) ----------
    let mut inventory_group = util::configure_group(c, "filesystem_inventory");

    inventory_group.bench_function("inventory_scan_small", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(small.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.bench_function("inventory_scan_monorepo", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(large.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.bench_function("inventory_scan_language_mix", |b| {
        b.iter(|| {
            let inventory = scan_file_inventory(black_box(langs.path())).unwrap();
            black_box(inventory);
        });
    });

    inventory_group.finish();

    // ---------- languages ----------
    let mut lang_group = util::configure_group(c, "filesystem_languages");

    lang_group.bench_function("languages_shallow_deep_mix", |b| {
        b.iter(|| {
            let breakdown = detect_languages(black_box(langs.path())).unwrap();
            black_box(breakdown);
        });
    });

    lang_group.bench_function("languages_monorepo", |b| {
        b.iter(|| {
            let breakdown = detect_languages(black_box(large.path())).unwrap();
            black_box(breakdown);
        });
    });

    lang_group.finish();

    // ---------- staged filesystem request ----------
    let mut stage_group = util::configure_slow_group(c, "filesystem_staged");

    stage_group.bench_function("filesystem_summary_request", |b| {
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

    stage_group.bench_function("filesystem_full_request", |b| {
        let req = FilesystemRequest::new();
        b.iter(|| {
            let info =
                detect_filesystem_with_request(black_box(large.path()), black_box(&req)).unwrap();
            black_box(info);
        });
    });

    stage_group.finish();
}
