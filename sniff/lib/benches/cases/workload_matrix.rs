//! Specification workload-family Criterion benches.
//!
//! These benchmarks complement the small domain benches with production-shaped
//! scaling families. Every expensive fixture is built inside the benchmark
//! registration closure and outside `b.iter`, so filtering to one ID does not
//! materialize the rest of the matrix and setup never contaminates timing.

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, black_box};

use sniff::filesystem::docs::detect_docs;
use sniff::filesystem::file_types::scan_file_inventory;
use sniff::filesystem::git::{PathHistoryOptions, commits_for_path_at};
use sniff::filesystem::{
    detect_filesystem_with_request, detect_git_with_request, detect_repo, detect_repo_structure,
};
use sniff::request::{FilesystemRequest, GitRequest};
#[cfg(feature = "bench-internals")]
use sniff::services::benchmark::{SyntheticSystemdListing, run_systemd_listing};
#[cfg(feature = "remote")]
use sniff::remote::RemoteRepoProvider;

use crate::support::{fixtures, util};

const MIXED_PACKAGE_COUNTS: &[usize] = &[100, 500, 2_000];
const DIRTY_FILE_COUNT: usize = 100;
const DIRTY_FILE_SIZES: &[usize] = &[1_024, 100 * 1_024, 2 * 1_024 * 1_024];
const OVER_CAP_FILE_COUNT: usize = 10_500;

pub fn register(c: &mut Criterion) {
    register_filesystem_shapes(c);
    register_repo_shapes(c);
    register_inventory_and_assembly(c);
    register_git_shapes(c);
    register_case_modes(c);
    #[cfg(feature = "bench-internals")]
    register_service_shapes(c);
    #[cfg(feature = "remote")]
    register_remote_report(c);
}

#[cfg(feature = "bench-internals")]
fn register_service_shapes(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "workloads_service_listing");
    for service_count in [500usize, 2_000] {
        let fixture = SyntheticSystemdListing::new(service_count);
        group.throughput(Throughput::Elements(service_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(service_count),
            &fixture,
            |b, fixture| {
                b.iter_batched(
                    || fixture.iteration(),
                    |mut iteration| black_box(run_systemd_listing(&mut iteration)),
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

#[cfg(feature = "remote")]
fn register_remote_report(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "workloads_remote_report");
    group.bench_function("github_provider_request_count_fixture", |b| {
        let fixture = crate::support::remote_report_fixture::github();
        b.iter(|| {
            black_box(
                fixture
                    .runtime
                    .block_on(fixture.provider.fetch_report("bench-owner", "bench-repo"))
                    .unwrap(),
            );
        });
    });
    group.finish();
}

fn register_filesystem_shapes(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "workloads_filesystem");

    group.bench_function("formatting_only_deep_24_wide_32", |b| {
        let fixture = fixtures::deep_wide_formatting_tree(24, 32);
        let request = FilesystemRequest::new()
            .without_git()
            .without_repo()
            .without_docs()
            .without_file_inventory();
        b.iter(|| {
            black_box(
                detect_filesystem_with_request(black_box(fixture.path()), black_box(&request))
                    .unwrap(),
            );
        });
    });

    group.bench_function("package_scoped_git_inventory_in_500_package_monorepo", |b| {
        let fixture = fixtures::nested_mixed_monorepo(500);
        let package = fixture.path().join("crates/rust-0000");
        let request = FilesystemRequest::new()
            .git(GitRequest::identity())
            .without_repo()
            .without_docs()
            .without_formatting();
        b.iter(|| {
            black_box(
                detect_filesystem_with_request(black_box(&package), black_box(&request)).unwrap(),
            );
        });
    });

    group.finish();
}

fn register_repo_shapes(c: &mut Criterion) {
    let mut structure = util::configure_slow_group(c, "workloads_repo_structure_mixed");
    for &package_count in MIXED_PACKAGE_COUNTS {
        structure.throughput(Throughput::Elements(package_count as u64));
        structure.bench_with_input(
            BenchmarkId::from_parameter(package_count),
            &package_count,
            |b, &package_count| {
                let fixture = fixtures::mixed_monorepo(package_count);
                b.iter(|| {
                    black_box(detect_repo_structure(black_box(fixture.path())).unwrap());
                });
            },
        );
    }
    structure.finish();

    let mut observation = util::configure_slow_group(c, "workloads_repo_observation");
    observation.bench_function("standalone_detect_repo_nested_500", |b| {
        let fixture = fixtures::nested_mixed_monorepo(500);
        b.iter(|| black_box(detect_repo(black_box(fixture.path())).unwrap()));
    });
    observation.bench_function("integrated_full_supplied_observation_nested_500", |b| {
        let fixture = fixtures::nested_mixed_monorepo(500);
        let request = FilesystemRequest::new().without_git().without_formatting();
        b.iter(|| {
            black_box(
                detect_filesystem_with_request(black_box(fixture.path()), black_box(&request))
                    .unwrap(),
            );
        });
    });
    observation.finish();
}

fn register_inventory_and_assembly(c: &mut Criterion) {
    let mut inventory = util::configure_slow_group(c, "workloads_inventory_over_cap");
    inventory.throughput(Throughput::Elements(OVER_CAP_FILE_COUNT as u64));

    inventory.bench_function("inventory_only_10500_files", |b| {
        let fixture = fixtures::inventory_docs_tree(OVER_CAP_FILE_COUNT, 0);
        b.iter(|| black_box(scan_file_inventory(black_box(fixture.path())).unwrap()));
    });
    inventory.bench_function("inventory_and_docs_10500_files_2000_docs", |b| {
        let fixture = fixtures::inventory_docs_tree(OVER_CAP_FILE_COUNT, 2_000);
        let request = FilesystemRequest::new()
            .without_git()
            .without_repo()
            .without_formatting();
        b.iter(|| {
            black_box(
                detect_filesystem_with_request(black_box(fixture.path()), black_box(&request))
                    .unwrap(),
            );
        });
    });
    inventory.finish();

    let mut documents = util::configure_slow_group(c, "workloads_document_attribution");
    for &document_count in &[500usize, 2_000] {
        documents.throughput(Throughput::Elements(document_count as u64));
        documents.bench_with_input(
            BenchmarkId::new("package_prefix_assignment", document_count),
            &document_count,
            |b, &document_count| {
                let fixture = fixtures::documented_mixed_monorepo(500, document_count);
                b.iter(|| black_box(detect_docs(black_box(fixture.path()))));
            },
        );
    }
    documents.finish();

    let mut assembly = util::configure_slow_group(c, "workloads_final_assembly");
    assembly.bench_function("full_500_packages_2000_docs", |b| {
        let fixture = fixtures::documented_mixed_monorepo(500, 2_000);
        let request = FilesystemRequest::new().without_git().without_formatting();
        b.iter(|| {
            black_box(
                detect_filesystem_with_request(black_box(fixture.path()), black_box(&request))
                    .unwrap(),
            );
        });
    });
    assembly.finish();
}

fn register_git_shapes(c: &mut Criterion) {
    let mut dirty = util::configure_slow_group(c, "workloads_git_dirty_sizes");
    for &bytes_per_file in DIRTY_FILE_SIZES {
        dirty.throughput(Throughput::Bytes((DIRTY_FILE_COUNT * bytes_per_file) as u64));
        dirty.bench_with_input(
            BenchmarkId::new("100_files", bytes_per_file),
            &bytes_per_file,
            |b, &bytes_per_file| {
                let fixture =
                    fixtures::git_repo_with_dirty_files_of_size(DIRTY_FILE_COUNT, bytes_per_file);
                let request = GitRequest::deep();
                b.iter(|| {
                    black_box(
                        detect_git_with_request(black_box(fixture.path()), black_box(&request))
                            .unwrap(),
                    );
                });
            },
        );
    }
    dirty.finish();

    let mut branches = util::configure_slow_group(c, "workloads_git_branches");
    branches.bench_function("branch_heavy_divergent_32_tips", |b| {
        let fixture = fixtures::git_repo_with_worktrees(32);
        let request = GitRequest::full();
        b.iter(|| {
            black_box(
                detect_git_with_request(black_box(fixture.path()), black_box(&request)).unwrap(),
            );
        });
    });
    branches.bench_function("deep_containment_100_remote_tips", |b| {
        let fixture = fixtures::git_repo_with_fake_remotes(500, 100);
        let request = GitRequest::deep().commit_count(100);
        b.iter(|| {
            black_box(
                detect_git_with_request(black_box(fixture.path()), black_box(&request)).unwrap(),
            );
        });
    });
    branches.finish();

    let mut history = util::configure_slow_group(c, "workloads_git_path_history");
    history.bench_function("2000_commits_sparse_prefix_every_200", |b| {
        let fixture = fixtures::sparse_path_history_repo(2_000, 200);
        let options = PathHistoryOptions::new(10).scan_limit(2_000);
        b.iter(|| {
            black_box(
                commits_for_path_at(black_box(fixture.path()), "wanted/", black_box(options))
                    .unwrap(),
            );
        });
    });
    history.finish();
}

fn register_case_modes(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "workloads_filesystem_case");
    group.bench_function("warm_case_variant_tree", |b| {
        let fixture = fixtures::case_variant_tree(500);
        black_box(scan_file_inventory(fixture.path()).unwrap());
        b.iter(|| black_box(scan_file_inventory(black_box(fixture.path())).unwrap()));
    });
    group.bench_function("coldish_fresh_case_variant_tree", |b| {
        b.iter_batched(
            || fixtures::case_variant_tree(64),
            |fixture| black_box(scan_file_inventory(black_box(fixture.path())).unwrap()),
            BatchSize::NumBatches(1),
        );
    });
    group.finish();
}
