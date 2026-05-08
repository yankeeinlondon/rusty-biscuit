---
agent: ""
model: ""
repo: "rusty-biscuit"
created: "2026-05-02 at 01:38 PM"
---

# Rust Performance Review for sniff

## Executive Summary

I found 9 concrete performance findings: 2 High, 5 Medium, and 2 Low.

Top 3 recommendations:

1. Fix the production parallel file-inventory scanner before relying on inventory/language benchmarks.
2. Replace per-file git diff-stat and diff generation loops with batched git2 diffs.
3. Make repo/package discovery truly index-backed by caching manifest parses and avoiding repeated canonicalization/projection scans.

Areas that already appear performance-conscious:

- `DetectionPlan` and request presets make expensive OS/hardware/network/filesystem work selectable.
- `detect_with_plan` runs independent detection domains concurrently.
- `detect_filesystem_with_request` has a shared filesystem walk for repo/file/docs data.
- `GitRequest::summary()` and `GitRequest::full()` split cheap counts from expensive diffs/network refresh.
- CLI dependency enrichment deduplicates dependencies and uses `buffer_unordered(20)`.
- The default tracing path has near-zero overhead unless `--debug` or `RUST_LOG` is enabled.

Areas where more measurement is needed:

- Git status cost as a function of dirty-file count and file size.
- Repo detection on monorepos with hundreds or thousands of packages.
- Program detection latency across long `PATH` values and slow network/home directories.
- Markdown metadata parsing cost on documentation-heavy repositories.

## Findings

## Production parallel inventory drops all classifications

**Severity:** High  
**Category:** runtime  
**Location:** `sniff/lib/src/filesystem/file_types/classify.rs`, `scan_inventory_parallel`

**Problem**

The non-test parallel inventory path walks files and classifies them into a worker-local `Vec`, but never appends those local results to the shared accumulator. It still pays the full traversal/classification cost and then returns an empty classification vector.

**Why it matters**

This is on the common full filesystem path for language summaries, file inventory, and full repo package enrichment. It also invalidates performance numbers for full inventory paths because production builds are measuring work whose results are thrown away.

**Evidence**

`scan_inventory_parallel` creates `shared: Arc<Mutex<Vec<FileClassification>>>` at lines 73-74 and clones it at line 87, but the clone is named `_shared` and never used. Each worker pushes into `local` at line 106. After the walker finishes, lines 111-117 unwrap `shared`, which was never populated.

The sequential test-only implementation does push into a returned vector, so tests can pass while production behavior diverges.

**Recommendation**

Use the same drop-flush pattern already used by `filesystem/system_view.rs`:

```rust
struct LocalClassifications {
    shared: Arc<Mutex<Vec<FileClassification>>>,
    local: Vec<FileClassification>,
}

impl Drop for LocalClassifications {
    fn drop(&mut self) {
        self.shared.lock().unwrap().append(&mut self.local);
    }
}
```

Then push into `worker.local`, not a bare `local` that is lost. Add a non-test integration test or a unit-test-only hook that exercises the parallel path.

**Expected impact**

Restores correctness for production inventory/language output and makes benchmarks meaningful. It may increase observed memory use because classifications will finally be retained, but that is the intended behavior.

**Risk/tradeoff**

The fix adds one mutex acquisition per worker at drop time, not per file. Ordering still needs the existing final sort.

## Git file-change stats scale with dirty files times full diff work

**Severity:** High  
**Category:** I/O  
**Location:** `sniff/lib/src/filesystem/git/detection.rs`, `get_repo_status_with_changes`, `get_file_diff_stats`, `build_dirty_files`

**Problem**

For every staged/unstaged file, `get_repo_status_with_changes` calls `get_file_diff_stats`, which creates separate pathspec-limited diffs for staged and unstaged changes. When full diffs are requested, `build_dirty_files` loops again and calls `get_file_diff` per file, repeating the same staged/unstaged diff setup.

**Why it matters**

`GitRequest::full()` is used for common repo/filesystem CLI paths and includes per-file change stats. In a large dirty working tree, this becomes roughly O(changed_files * diff_setup_cost), with repeated index/worktree comparisons. Deep mode adds full patch string materialization on top.

**Evidence**

Lines 249-279 call `get_file_diff_stats` per dirty path. Lines 454-470 construct `DiffOptions` and run `diff_tree_to_index` / `diff_index_to_workdir` for one path at a time. Lines 422-435 and 479-511 repeat per-file diff construction for unified diff payloads.

**Recommendation**

Build staged and unstaged diffs once, then aggregate by path:

```rust
let staged = repo.diff_tree_to_index(head_tree.as_ref(), None, None)?;
let unstaged = repo.diff_index_to_workdir(None, None)?;
let mut stats_by_path = HashMap::<PathBuf, LineStats>::new();
staged.foreach(... accumulate by delta path ...)?;
unstaged.foreach(... accumulate by delta path ...)?;
```

For deep mode, either emit patches from the same whole-repo diffs grouped by path or gate full patch payloads behind a more explicit CLI/API request.

**Expected impact**

Large dirty repos should see much lower git-status latency and less repeated libgit2 allocation. The benefit grows with changed-file count.

**Risk/tradeoff**

Grouping patches by file is more complex than pathspec-per-file diffs. Rename/delete handling and staged-plus-unstaged combinations need focused tests.

## ManifestIndex still performs repeated canonicalization syscalls

**Severity:** Medium  
**Category:** data-structure  
**Location:** `sniff/lib/src/filesystem/repo/types.rs`, `ManifestIndex::package_dirs_in_tree`

**Problem**

`ManifestIndex` is meant to avoid repeated tree walks, but each subtree query canonicalizes the search root, repo root, and every manifest path. Full repo detection calls these queries repeatedly for workspace tools and discovered packages.

**Why it matters**

On large monorepos or filesystems with slow metadata operations, repeated `canonicalize` can dominate the work that the manifest index was intended to avoid. It also makes index queries allocate and perform filesystem I/O.

**Evidence**

`package_dirs_in_tree` canonicalizes `search_root` and `root` at lines 473-475, then calls `canonicalize_path(path)` for every indexed manifest key at lines 477-488. `canonicalize_path` uses `std::fs::canonicalize` before falling back to lexical normalization.

**Recommendation**

Normalize once when building the index and store both original and normalized paths:

```rust
struct ManifestEntry {
    original: PathBuf,
    normalized: PathBuf,
    kinds: HashSet<ManifestKind>,
}
```

Then `package_dirs_in_tree` can compare normalized paths without syscalls. If symlink resolution matters, make canonicalization an explicit index-build option rather than a per-query cost.

**Expected impact**

Faster full repo detection on package-heavy trees and more predictable latency on networked or cold filesystems.

**Risk/tradeoff**

Pure lexical normalization may not collapse symlink aliases. If current behavior depends on symlink identity, retain canonicalized entries at build time.

## Package creation reparses manifests multiple times

**Severity:** Medium  
**Category:** parsing/text  
**Location:** `sniff/lib/src/filesystem/repo/detection.rs`, `create_package` and manifest helper functions

**Problem**

`create_package` reads and parses the same manifest multiple times for name, version, features, and dependency extraction. A `ManifestCache` exists near the top of the module but is dead code and not wired into package creation.

**Why it matters**

Full repo detection creates one `Package` per manifest directory. In Rust/Node/Python monorepos this repeats TOML/JSON parsing across every package. It is especially expensive for large `Cargo.toml` files with many dependencies or workspaces with hundreds of packages.

**Evidence**

`create_package` resolves name/version/features/dependencies at lines 1924-1992. Cargo helpers independently read/parse `Cargo.toml`: package name/version helpers, `read_cargo_features`, and `parse_cargo_dependencies`. The module also defines `ManifestCache`, but it is annotated `#[allow(dead_code)]` and unused.

**Recommendation**

Introduce a `PackageBuildContext` passed through discovery:

```rust
struct PackageBuildContext {
    manifests: ManifestCache,
    lock_versions: Option<CargoLockVersions>,
}
```

Parse each manifest once and derive name, version, features, dependency lists, ecosystem, and package manager data from the parsed representation. Prefer typed minimal structs for common manifest fields over broad `serde_json::Value` / TOML value traversal where practical.

**Expected impact**

Lower I/O, lower allocation churn, and faster repo detection in package-heavy workspaces.

**Risk/tradeoff**

This changes internal plumbing across discovery functions. Keep the public `Package` API unchanged and add fixture tests for mixed Cargo/Node/Python/Go packages.

## Full package boundary enrichment is O(packages * files)

**Severity:** Medium  
**Category:** data-structure  
**Location:** `sniff/lib/src/filesystem/repo/detection.rs`, `refresh_package_boundaries`, `detect_package_languages`

**Problem**

After scanning a repo-wide inventory, package enrichment projects that inventory separately for every package. Projection iterates the inventory and clones matching classifications for each package. Nested-package detection also does a pairwise package-prefix scan.

**Why it matters**

This cost grows poorly for monorepos with many packages and many files. With P packages and F classifications, package enrichment is roughly O(P * F) after the initial filesystem walk, plus O(P²) nested package checks.

**Evidence**

`refresh_package_boundaries` builds descendants with nested loops at lines 1682-1688. It then calls `detect_package_languages` for every package at lines 1691-1708. That function uses `project_package_inventory` when a repo inventory is provided; `project_package_inventory` scans `repo_inventory.classifications` and clones matching entries for each package.

**Recommendation**

Build a package-prefix index once:

- Sort packages by relative path.
- For each classification, assign it to the deepest containing package in one pass.
- Accumulate language/file stats directly per package instead of materializing a cloned `FileInventory` per package.

For nested packages, use the same sorted package paths or a prefix trie to derive descendants without the full pairwise scan.

**Expected impact**

Large monorepos should see significantly lower CPU and allocation during full repo detection. The benefit grows with both package count and file count.

**Risk/tradeoff**

The one-pass assignment must preserve current nested-package exclusion semantics. Add tests for root packages, nested packages, and package areas.

## Program detection lost the intended shared PATH index

**Severity:** Medium  
**Category:** I/O  
**Location:** `sniff/lib/src/programs/find_program.rs`, `ExecutableIndex`; `sniff/lib/src/programs/types.rs`, `CategoryDetector::new_with_index`

**Problem**

`ProgramsInfo::detect` builds a shared `ExecutableIndex`, but the index no longer indexes PATH. Each `find_with_source` delegates to `which(program)`, so detecting all categories performs one PATH search per binary name.

**Why it matters**

The program catalog includes well over 100 binary names. On systems with long PATH values, slow mounted directories, or shell-managed shim directories, this can turn a single inventory command into hundreds of directory scans. The module docs still describe shared O(1) lookup, so callers may assume this path is cheaper than it is.

**Evidence**

`ExecutableIndex::build` records only `path_dir_count` and platform fallback indexes at lines 90-130. `find_with_source` calls `which(program)` at lines 142-145. `CategoryDetector::new_with_index` builds `names_to_search` and calls `find_programs_with_source_from_index` at lines 480-488, which loops names and calls `index.find_with_source`.

**Recommendation**

Restore an eager PATH index for bulk detection while keeping lazy lookup for one-off APIs:

- `ExecutableIndex::build_eager_path()` scans PATH once into `HashMap<OsString, PathBuf>`.
- `ProgramsInfo::detect` uses the eager index.
- `ExecutableIndex::build_path_only` or `find_program` can keep lazy `which` behavior for single lookups.

Deduplicate `names_to_search` before lookup to avoid repeated aliases.

**Expected impact**

Faster `sniff programs` and category commands on hosts with long or slow PATHs. More predictable latency because PATH is traversed once.

**Risk/tradeoff**

Eager indexing can be slower for a single lookup and uses memory proportional to PATH executable count. Use it only for bulk category detection.

## Blast-radius reparses every markdown document after collecting changes

**Severity:** Medium  
**Category:** I/O  
**Location:** `sniff/lib/src/filesystem/blast_radius.rs`, `find_blast_radius_documents`; `sniff/lib/src/filesystem/docs.rs`, `parse_markdown_meta`

**Problem**

Blast-radius matching needs only docs with `blast_radius` frontmatter, but it calls `detect_docs`, which reads every markdown file, parses frontmatter, extracts headings, stats metadata, and hashes the full document body.

**Why it matters**

Documentation-heavy repositories pay full docs metadata cost every time a blast-radius query runs, even if only a small fraction of docs have `blast_radius`. Large markdown files also force full-file reads and full-body hashing.

**Evidence**

`find_blast_radius_documents` calls `detect_docs(&result.repo_root)` at lines 351-354 and filters `has_blast_radius` afterward at lines 356-364. `parse_markdown_meta` reads the whole file at lines 239-245, extracts title/body, calls `metadata` via `resolve_last_updated`, hashes `body` at line 259, and sorts all frontmatter keys at lines 264-265.

**Recommendation**

Add a blast-radius-specific scanner:

- Read only until the closing frontmatter delimiter plus a bounded prefix for title if needed.
- Parse only the `blast_radius` key.
- Skip content hashing and mtime unless the caller asks for full `MarkdownMeta`.

Alternatively extend `parse_markdown_meta` with a `DocParseMode::BlastRadiusOnly`.

**Expected impact**

Lower I/O and allocation for blast-radius workflows, especially in repos with many or large markdown files.

**Risk/tradeoff**

Returning full `MarkdownMeta` currently gives rich output. A lightweight scanner either needs a second parse for matched docs or a smaller output type internally.

## Deep git remote refresh and containment are serial and multiplicative

**Severity:** Low  
**Category:** async/concurrency  
**Location:** `sniff/lib/src/filesystem/git/detection.rs`, `refresh_remote_tracking_refs`, `populate_recent_commit_remotes`

**Problem**

Deep git detection fetches remotes serially and then checks every recent commit against every remote-tracking branch using `graph_descendant_of`.

**Why it matters**

Most repos have one or two remotes and a small history count, so this is usually fine. It becomes visible for repos with many remotes, many remote-tracking branches, or high commit counts.

**Evidence**

`refresh_remote_tracking_refs` loops remotes and runs `git fetch --quiet --prune <remote>` one at a time. `populate_recent_commit_remotes` loops commits and remote tips at lines 835-848, doing graph reachability checks for each pair.

**Recommendation**

Keep the current behavior for defaults, but cap or batch deep containment:

- Fetch remotes with small bounded parallelism if multiple remotes are configured.
- For containment, walk ancestry once from each remote tip up to the oldest requested commit and build `HashMap<Oid, Vec<remote>>`.
- Add a request knob for max remote branches inspected.

**Expected impact**

Improves worst-case deep git latency without changing summary/full defaults.

**Risk/tradeoff**

Parallel fetches can increase network load and credential-helper contention. Keep concurrency low and preserve `GIT_TERMINAL_PROMPT=0`.

## CLI async entrypoint mostly runs synchronous blocking work

**Severity:** Low  
**Category:** async/concurrency  
**Location:** `sniff/cli/src/commands.rs`, `run`; `sniff/cli/src/main.rs`, `#[tokio::main]`

**Problem**

The CLI uses a Tokio multi-thread runtime, but most command paths execute synchronous git/filesystem/subprocess work directly inside the async `run` function. This is not a throughput bug for a single-shot CLI, but it makes async boundaries misleading and can block runtime worker threads around remote commands.

**Why it matters**

If the CLI grows long-lived modes, concurrent remote operations, progress UI tasks, or cancellation, synchronous detection inside async tasks can cause latency spikes. Today this mainly adds runtime/dependency overhead and makes it easy to accidentally mix blocking work with async network operations.

**Evidence**

`main` uses `#[tokio::main]`. `commands::run` is async, but directly calls `detect_with_plan`, `git2::Repository::discover`, `detect_services`, `ProgramsInfo::detect`, and filesystem walkers in the async body. Remote provider calls are async, but local prep and postprocessing remain synchronous.

**Recommendation**

For the current CLI, either accept this and document it as a single-shot command model, or move heavyweight local detection behind `tokio::task::spawn_blocking` only for command paths that concurrently await network operations. Avoid sprinkling `spawn_blocking` everywhere unless there is real concurrent async work.

**Expected impact**

Clearer async architecture and safer future concurrency. Runtime speed may not change for current single-command execution.

**Risk/tradeoff**

`spawn_blocking` adds scheduling overhead and complicates error handling. It is not worth applying broadly without concurrent async work.

## Narrow compile-time cleanup: unused or broad dependency features

**Severity:** Low  
**Category:** dependency  
**Location:** `sniff/lib/Cargo.toml`, `sniff/cli/Cargo.toml`

**Problem**

The package area pulls some heavier features/dependencies for narrow use cases. `which` is built with `regex` and `tracing` features even though current program lookup uses direct binary names. `sniff-cli` always enables `sniff`'s `network` and `remote` features, so the CLI always pays compile cost for async HTTP providers even for local-only workflows.

**Why it matters**

This does not affect runtime hot paths much, but it affects developer build time and binary size. The cost matters in this monorepo because many crates share CI/dev loops.

**Evidence**

`sniff/lib/Cargo.toml` declares `which = { version = "8.0.0", features = ["regex", "tracing"] }`. The lookup paths in `find_program.rs` call `which(program)` with exact names. `sniff/cli/Cargo.toml` depends on `sniff = { path = "../lib", features = ["network", "remote"] }` unconditionally.

**Recommendation**

Measure first with `cargo bloat` and `cargo llvm-lines`. If confirmed, remove unused `which` features and consider a CLI feature split such as `sniff-cli/remote` for commands that need remote providers.

**Expected impact**

Potentially faster clean builds and smaller binaries.

**Risk/tradeoff**

Feature splitting can complicate CLI distribution and docs. Do not split remote support unless build-size data justifies it.

## Quick Wins

| Change | Location | Why it is low risk | Expected benefit |
|--------|----------|--------------------|------------------|
| Flush worker-local classifications in `scan_inventory_parallel` | `sniff/lib/src/filesystem/file_types/classify.rs` | Matches the already-used `system_view` accumulator pattern | Restores production inventory results and benchmark validity |
| Deduplicate `names_to_search` before program lookups | `sniff/lib/src/programs/types.rs` | Purely removes repeated identical lookups | Fewer PATH scans and less HashMap churn |
| Reuse parsed manifests within `create_package` | `sniff/lib/src/filesystem/repo/detection.rs` | Internal-only refactor; public output unchanged | Less repeated TOML/JSON/YAML parsing |
| Store normalized paths in `ManifestIndex` at build time | `sniff/lib/src/filesystem/repo/types.rs` | Keeps current query API | Removes repeated canonicalization from index queries |
| Add a blast-radius-only docs parse mode | `sniff/lib/src/filesystem/docs.rs` | Can be added alongside existing full parser | Faster blast-radius queries |
| Benchmark dirty-file scaling explicitly | `sniff/lib/benches/cases/filesystem.rs` | Adds measurement without behavior change | Validates git batching priority |

## Benchmarking and Profiling Recommendations

- Add Criterion cases for git dirty-file scaling: 10, 100, and 1,000 dirty files, with `GitRequest::full()` and `GitRequest::deep()`.
- Add a full repo fixture with hundreds of package manifests and thousands of classified files to isolate `refresh_package_boundaries`.
- Add a docs fixture with many small markdown files and a few large markdown files; compare `detect_docs` vs a blast-radius-only parser.
- Use `cargo bench -p sniff --bench perf -- filesystem_git filesystem_repo filesystem_staged inventory`.
- Use `cargo flamegraph -p sniff-cli --bin sniff -- repo git-status` on a dirty large repo.
- Use `hyperfine 'target/release/sniff programs' 'target/release/sniff editors'` with long `PATH` values.
- Use `cargo bloat -p sniff-cli --release --bin sniff` and `cargo llvm-lines -p sniff-cli --release` before dependency feature cleanup.

## Non-Issues / Things I Would Not Change Yet

- The top-level scoped-thread concurrency in `detect_with_plan` is appropriate for a synchronous library API.
- `Arc` on `FileInventory.classifications` is a good fit for shared repo inventory; the zero-copy no-filter case is already handled.
- Most output-layer `format!` calls are not worth optimizing; they are terminal rendering paths, not detection hot paths.
- The `HashSet` dedupe in git status and changed-path collection is appropriate and avoids obvious O(n²) `Vec::contains` behavior.
- The bounded `buffer_unordered(20)` in CLI dependency enrichment is already the right shape; do not replace it with unbounded `join_all`.
- Per-file `Instant::now()` and counters are mostly gated behind the `metrics` feature; avoid changing this without benchmark evidence.

## Suggested Implementation Order

1. Highest impact / lowest risk: fix `scan_inventory_parallel`, add a production-path regression test, and rerun inventory/filesystem benchmarks.
2. High impact but requires design care: batch git diff stats and patch generation for `get_repo_status_with_changes`.
3. Medium-impact cleanup: cache manifest parsing, normalize `ManifestIndex` entries once, and deduplicate program lookup names.
4. Benchmarking and validation: add dirty-file, package-count, docs-count, and PATH-length benchmarks before/after changes.
5. Optional compile-time or dependency cleanup: measure `which` features and CLI remote feature cost before changing Cargo features.
