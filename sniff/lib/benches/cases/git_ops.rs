//! Git-operation Criterion benches backing the gitoxide migration.
//!
//! These benches isolate the individual git operations the migration ports —
//! discovery, working-tree status, revision walks, commit-file diffs, ancestry
//! containment, worktree fan-out, config reads, and ref enumeration — so each
//! has a stable `git_ops/*` identifier to compare a saved `git2` baseline
//! against the `gix` port on the same host.
//!
//! Every benchmark exercises only the public `sniff` API (plus `git2` for the
//! commit-file diff, which needs a `Repository` handle), so the same IDs keep
//! their meaning across the backend swap. Fixtures are materialized outside the
//! timed iterations; the commit-graph variants skip with a clear message when
//! no `git` executable is available.

use chrono::DateTime;
use criterion::{BenchmarkId, Criterion, Throughput, black_box};
use git2::Repository;

use sniff::filesystem::detect_git_with_request;
use sniff::filesystem::git::get_recent_commits_in_range;
use sniff::filesystem::git::{GitRepo, get_commit_files_with_cache, get_recent_commits_by_count};
use sniff::request::GitRequest;

use crate::support::{fixtures, util};

/// Dirty-file counts for the status benches. The `1000` row is opt-in via
/// `SNIFF_BENCH_DEEP_DIRTY=1` so day-to-day runs stay fast.
const BASE_DIRTY_COUNTS: &[usize] = &[10, 100];
const HEAVY_DIRTY_COUNTS: &[usize] = &[10, 100, 1000];

/// Linked-worktree counts for the fan-out bench (the spec's 1/4/8 set).
const WORKTREE_COUNTS: &[usize] = &[1, 4, 8];

/// Commits in the deep-history fixture backing the revwalk benches. The
/// `5000` row is opt-in via `SNIFF_BENCH_DEEP_HISTORY=1`.
const BASE_HISTORY_DEPTH: usize = 500;
const HEAVY_HISTORY_DEPTH: usize = 5000;

/// Commits in the commit-file diff fixture. Smaller than the revwalk depth so
/// per-iteration diff cost (one tree-to-tree diff per commit) stays bounded.
const DIFF_DEPTH: usize = 50;

/// Remote-tracking branch count for the ancestry-containment bench.
const CONTAINMENT_REMOTES: usize = 10;

fn dirty_counts() -> &'static [usize] {
    if std::env::var_os("SNIFF_BENCH_DEEP_DIRTY").is_some() {
        HEAVY_DIRTY_COUNTS
    } else {
        BASE_DIRTY_COUNTS
    }
}

fn history_depth() -> usize {
    if std::env::var_os("SNIFF_BENCH_DEEP_HISTORY").is_some() {
        HEAVY_HISTORY_DEPTH
    } else {
        BASE_HISTORY_DEPTH
    }
}

pub fn register(c: &mut Criterion) {
    register_discovery(c);
    register_status(c);
    register_revwalk(c);
    register_diff(c);
    register_containment(c);
    register_worktrees(c);
    register_config_and_refs(c);
}

/// `git_ops/discover` — repository discovery walking up from a nested path.
fn register_discovery(c: &mut Criterion) {
    let mut group = util::configure_group(c, "git_ops");

    let fixture = fixtures::small_git_repo();
    let nested = fixture.path().join("src");
    group.bench_function("discover", |b| {
        b.iter(|| {
            let repo = GitRepo::discover(black_box(nested.as_path())).unwrap();
            black_box(repo);
        });
    });

    group.finish();
}

/// `git_ops/status_dirty_flag` and `git_ops/status_file_changes` — the
/// short-circuiting summary status versus the full per-file change walk.
fn register_status(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_ops");

    let summary_req = GitRequest::summary();
    let full_req = GitRequest::full();

    for &count in dirty_counts() {
        let fixture = fixtures::git_repo_with_dirty_files(count);
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("status_dirty_flag", count),
            &count,
            |b, _| {
                b.iter(|| {
                    let info =
                        detect_git_with_request(black_box(fixture.path()), black_box(&summary_req))
                            .unwrap();
                    black_box(info);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("status_file_changes", count),
            &count,
            |b, _| {
                b.iter(|| {
                    let info =
                        detect_git_with_request(black_box(fixture.path()), black_box(&full_req))
                            .unwrap();
                    black_box(info);
                });
            },
        );
    }

    group.finish();
}

/// `git_ops/revwalk_recent_gated` and `git_ops/revwalk_recent_full` — a
/// time-gated range walk that stops at a cutoff versus a full count walk. Each
/// runs against a commit-graph-absent and (when `git` is available) a
/// commit-graph-present fixture so the gix port can show the graph win.
fn register_revwalk(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_ops");
    let depth = history_depth();

    // The fixture stamps commit i at epoch + 1000 + 60*i seconds. Cutting at the
    // midpoint makes the gated walk process roughly the newer half before the
    // time gate prunes the rest.
    let since = DateTime::from_timestamp(1000 + 60 * (depth as i64) / 2, 0).unwrap();
    let until = DateTime::from_timestamp(1000 + 60 * (depth as i64) + 1, 0).unwrap();

    let variants: &[(&str, bool)] = &[("nograph", false), ("graph", true)];
    for &(label, with_graph) in variants {
        let Some(fixture) = fixtures::deep_history_repo(depth, with_graph) else {
            eprintln!(
                "git_ops: skipping revwalk_*/{label} (commit-graph requested but `git` is unavailable)"
            );
            continue;
        };

        group.bench_function(BenchmarkId::new("revwalk_recent_gated", label), |b| {
            b.iter(|| {
                let set =
                    get_recent_commits_in_range(black_box(fixture.path()), since, until, "gated")
                        .unwrap();
                black_box(set);
            });
        });

        group.bench_function(BenchmarkId::new("revwalk_recent_full", label), |b| {
            b.iter(|| {
                let set = get_recent_commits_by_count(black_box(fixture.path()), depth).unwrap();
                black_box(set);
            });
        });
    }

    group.finish();
}

/// `git_ops/diff_commit_files` — collect each commit's changed files across a
/// linear history (one tree-to-tree diff per commit).
fn register_diff(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_ops");

    let fixture = fixtures::deep_history_repo(DIFF_DEPTH, false).expect("graph-absent fixture");
    let git2_repo = Repository::discover(fixture.path()).expect("discover diff fixture");
    let mut gix_repo = gix::open(fixture.path()).expect("open diff fixture with gix");
    // Production sizes the object cache before diffing many commits (see
    // `open::configure_cache`, applied in every `recent_commits` entry point).
    // libgit2 enables its ODB cache by default, so an un-sized gix handle here
    // would measure a path production never takes and report a false regression.
    if let Ok(index) = gix_repo.index_or_empty() {
        let size = gix_repo.compute_object_cache_size_for_tree_diffs(&index);
        gix_repo.object_cache_size_if_unset(size);
    }
    let shas = commit_shas(&git2_repo);
    group.throughput(Throughput::Elements(shas.len() as u64));

    let mut cache = gix_repo
        .diff_resource_cache_for_tree_diff()
        .expect("create diff resource cache");
    group.bench_function("diff_commit_files", |b| {
        b.iter(|| {
            for sha in &shas {
                let oid = gix::ObjectId::from_hex(sha.as_bytes()).expect("valid sha");
                black_box(get_commit_files_with_cache(
                    &gix_repo,
                    black_box(oid),
                    &mut cache,
                ));
            }
            // Clear the cache between iterations to avoid unbounded growth
            // while still amortizing allocations within the iteration.
            cache.clear_resource_cache();
        });
    });

    group.finish();
}

/// `git_ops/ancestry_containment` — per-commit remote containment via a single
/// ancestry walk per remote-tracking branch.
fn register_containment(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_ops");

    let fixture = fixtures::git_repo_with_fake_remotes(20, CONTAINMENT_REMOTES);
    let deep_req = GitRequest::deep().commit_count(10);

    group.bench_function("ancestry_containment", |b| {
        b.iter(|| {
            let info =
                detect_git_with_request(black_box(fixture.path()), black_box(&deep_req)).unwrap();
            black_box(info);
        });
    });

    group.finish();
}

/// `git_ops/worktree_fanout` — enumerate and characterize linked worktrees for
/// 1, 4, and 8 worktrees.
fn register_worktrees(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_ops");

    for &count in WORKTREE_COUNTS {
        let fixture = fixtures::git_repo_with_worktrees(count);
        let repo = GitRepo::discover(fixture.path())
            .expect("discover worktrees fixture")
            .expect("worktrees fixture is a repo");
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("worktree_fanout", count),
            &count,
            |b, _| {
                b.iter(|| black_box(repo.worktrees()));
            },
        );
    }

    group.finish();
}

/// `git_ops/config_read` — the 12-key git config read, and
/// `git_ops/refs_enumerate` — ref-decoration enumeration over all refs.
///
/// Both discover a fresh handle each iteration so the read is actually
/// exercised rather than served from the per-handle cache.
fn register_config_and_refs(c: &mut Criterion) {
    let mut group = util::configure_group(c, "git_ops");

    let config_fixture = fixtures::small_git_repo();
    group.bench_function("config_read", |b| {
        b.iter(|| {
            let repo = GitRepo::discover(black_box(config_fixture.path()))
                .unwrap()
                .unwrap();
            black_box(repo.config());
        });
    });

    // The fake-remotes fixture carries many refs/remotes/* entries, so
    // recent_commits forces a full ref-decoration enumeration over all refs.
    let refs_fixture = fixtures::git_repo_with_fake_remotes(20, CONTAINMENT_REMOTES);
    group.bench_function("refs_enumerate", |b| {
        b.iter(|| {
            let repo = GitRepo::discover(black_box(refs_fixture.path()))
                .unwrap()
                .unwrap();
            black_box(repo.recent_commits(1));
        });
    });

    group.finish();
}

/// Collect every commit SHA reachable from HEAD, newest-first.
fn commit_shas(repo: &Repository) -> Vec<String> {
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.push_head().expect("push HEAD");
    revwalk
        .filter_map(|oid| oid.ok().map(|o| o.to_string()))
        .collect()
}
