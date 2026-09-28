# Sniff filesystem observation performance review

**Date:** 2026-07-14

**Scope:** `sniff/lib/src/filesystem`, its request model, and the filesystem/Git Criterion coverage

**Status:** Review complete; findings are not implemented

## Executive summary

Sniff already has the right foundation for fast, portable filesystem observation: request-gated work, `ignore`-based walking, a shared manifest/inventory/docs pass, `Arc`-backed inventories, pure-Rust `gix`, and bounded expensive modes. The next gains should come from extending that design, not from adding platform-specific scanners or persistent caches.

The largest opportunities are:

1. Make the request planner start only the walks that a request needs and choose the narrowest valid walk root.
2. Fold nested-workspace and membership evidence into the existing shared walk so a full observation does not walk the same tree again.
3. Separate package boundary discovery from package enrichment. Deduplicate package paths first, then read and parse each unique manifest once.
4. Transfer shared results into `FilesystemInfo` instead of deep-cloning `RepoInfo` and Markdown metadata.
5. Make Git status compute each dirty file's inputs and diff once, even when both statistics and unified patches are requested.
6. Give Git metadata its own request flags so asking for recent commits does not implicitly trigger branch-wide graph walks, config, remotes, tracking, and worktrees.

All of these can use the same Rust implementation on macOS, Linux, and Windows. The main cross-platform performance rule is to minimize directory enumeration, metadata probes, canonicalization, and file opens before considering more parallelism. Those operations are particularly expensive on Windows, antivirus-scanned trees, network shares, and cold storage.

## Current evidence

I ran focused Criterion cases against the current source on this macOS/APFS host. These are warm-cache, synthetic-fixture measurements and are directional only; they are not cross-platform budgets.

| Case | Median/typical estimate |
|---|---:|
| Staged filesystem: Git summary + repo structure | 23.0 ms |
| Staged filesystem: all filesystem stages | 43.3 ms |
| Repo structure, “huge” fixture | 53.2 ms |
| Full repo, “huge” fixture | 129.0 ms |
| Git file statistics, 100 dirty files | 5.75 ms |
| Git unified diffs, 100 dirty files | 7.15 ms |

The “huge 500 packages” benchmark name is stale. The fixture constants create 200 Rust + 100 JavaScript + 50 Python + 25 Go packages, or **375 packages**, in `benches/support/builder.rs:36-49`. This review therefore treats the code as authoritative and describes the results as 375-package results.

The 53.2 ms versus 129.0 ms structure/full measurements are about 2.4× apart on this fixture. They do not substantiate the general “10–50× faster” wording currently used for `RepoRequest::structure()` without a more specific workload qualification.

## Existing architecture worth preserving

- `detect_filesystem_with_request` discovers one `GitRepo` handle and reuses it for Git detection and repo-root selection.
- `FilesystemSystemView` combines manifest, inventory, and Markdown collection in one parallel `ignore` walk.
- The no-filter inventory path shares classifications through `Arc`.
- Inventory classification is capped, ignores common dependency/build directories, and usually classifies from names/extensions without opening content.
- `GitRequest::identity()` skips status entirely; `summary()` stops after the first dirty item; richer status levels share one `gix::Repository`.
- Commit walks opt into the commit graph, and object caching is configured lazily for object-heavy Git operations.
- Local Git observation is pure Rust and does not depend on the presence or behavior of a platform `git` executable.

The recommendations below keep these properties.

## Highest-priority findings

### 1. Formatting-only requests accidentally walk the entire tree

`filesystem/mod.rs:91-94` includes `request.include_formatting` in `need_shared_view`. The resulting `SharedWalkOptions` at `filesystem/mod.rs:151-155` can have all three collection flags set to false, but `build_filesystem_system_view` still performs a complete parallel walk at `system_view.rs:75-107`. `detect_formatting` itself only checks the requested directory for `.editorconfig`.

This makes a formatting-only request O(tree entries) instead of O(1) directory probes plus one file read. A unit test currently asserts that formatting must trigger the shared view, so the test protects the costly behavior rather than the intended work contract.

**Recommendation:** remove formatting from `need_shared_view`. Add an end-to-end work-count test proving that a formatting-only request never enters the shared walker. If EditorConfig hierarchy semantics are later expanded, parent traversal should still be O(directory depth), not a descendant-tree walk.

### 2. Git presence widens package-scoped inventory requests to the repo root

When Git is requested, `filesystem/mod.rs:106-108` always makes the Git repository root the shared-walk root. A caller inside one package that asks for Git plus file inventory therefore scans the whole monorepo and filters the inventory afterward at `filesystem/mod.rs:206-243`.

This is unnecessary unless the request needs repo-wide evidence. It is especially costly in a large monorepo on Windows, where each extra directory entry can involve filtering and antivirus activity, and on remote/network filesystems where parallel traversal can amplify random I/O.

**Recommendation:** derive walk scope independently from Git discovery.

- Repo-full and repository-wide docs need the repository root.
- A base-directory file inventory should walk the base directory (or the resolved package root) unless the repo stage explicitly needs a repo-wide inventory.
- Formatting never needs the shared walk.
- Repo-structure alone does not need an inventory walk.

Encode this as a small internal scope decision with table-driven tests. Do not make `GitRepo` presence itself imply repo-wide filesystem scope.

### 3. Full repo observation still performs redundant tree walks

The shared view sees every relevant entry, but repo detection separately calls `discover_nested_workspace_outcomes` at `repo/detection.rs:300-306`. That function performs another serial `ignore` walk in `repo/nested.rs:230-304`. Membership glob expansion can perform additional subtree walks and up to four `exists()` probes per directory in `repo/glob.rs:174-208` and `repo/glob.rs:246-249`.

The standalone `detect_repo` path is worse: it can build a manifest index, walk again for nested markers, and walk again for file inventory. Polyglot leaf-marker detectors can add their own walks when active.

**Recommendation:** extend the existing per-request shared view into a compact observation index containing only reusable evidence:

- manifest-bearing directories and manifest kinds;
- nested workspace marker paths and their parent directories;
- solution/leaf-marker paths needed by active standards;
- the existing file classifications and Markdown metadata when requested.

Pass that evidence into nested discovery and membership expansion. Standalone full repo detection should build the same shared view internally rather than maintaining a separate multi-walk path.

Do not retain every `DirEntry` or add a process-global cache. Evidence-sized, per-request storage preserves freshness and bounds memory. Merely converting the redundant serial walk to a parallel walk is a secondary fallback: it still repeats I/O and can be slower on Windows/network storage.

### 4. Package boundaries are enriched before they are deduplicated

Workspace detectors call `create_package`, which performs broad enrichment immediately (`repo/detection.rs:1318-1407`): ecosystem and package-manager probes, test-runner detection, name/version resolution, features, and dependency parsing. Multiple standards or nested discovery can create the same package more than once; only later does `merge_packages` canonicalize and merge them at `repo/detection.rs:1033-1047`.

The cache is also local to each `create_package` call (`repo/detection.rs:150-163`). It prevents duplicate reads within one construction but not across duplicate candidates or packages that share root-scoped inputs. The `ManifestIndex` already records manifest kinds, yet package construction re-derives presence through many `exists()` calls. Structure-only mode still uses this enriched constructor even though its documented purpose is workspace tools and package boundaries.

Full mode adds more duplication at `repo/detection.rs:346-360`: it clones all workspace packages, scans the whole manifest index once per package subtree, and constructs more `Package` values that are merged later. Root `Cargo.lock`, root-scoped test-runner configuration, and inherited root manifests can also be read or parsed repeatedly.

**Recommendation:** make package processing two-phase.

1. Detectors produce cheap `PackageSeed` values: normalized path, owning standard, provenance, and matched manifest kinds.
2. Merge/deduplicate seeds by a syscall-free normalized path key.
3. Enrich each unique seed exactly once according to the request detail level.

Use one per-detection `ManifestStore` keyed by path for raw/parsed Cargo, Node, Python, Go, lockfile, and root-scoped configuration data. `RepoRequest::structure()` should perform only the minimum parsing required for package identity and membership; it should not scan dependencies, test-runner configuration, features, or languages. Full mode can populate those fields after deduplication.

This change should provide a larger relative win on Windows than the current macOS warm-cache benchmark because it eliminates file probes rather than merely reducing CPU work.

### 5. The reuse phase deep-clones its richest results

After repo detection, `filesystem/mod.rs:200-203` clones `repo_context` to populate the public `repo` field. A full `RepoInfo` can contain hundreds of packages with dependency lists, language/file vectors, configuration paths, and topology data. Docs similarly clone the whole shared Markdown vector at `filesystem/mod.rs:263-267` before assigning packages.

These copies scale with output richness and are avoidable. The shared inventory's `Arc` does not help the vectors nested inside `RepoInfo` or `MarkdownMeta`.

**Recommendation:** keep the shared view and repo context mutable, borrow them while inventory/docs enrichment runs, then move the finished values into `FilesystemInfo`. Use `Option::take` for docs. When docs need repo context but repo output was not requested, drop the context after assignment instead of cloning it.

Add an isolated benchmark for the final assembly step on hundreds of packages/docs; the current staged benchmark cannot distinguish detection from result-copy cost.
## Git observation findings

### 6. Rich status computes dirty-file inputs and diffs more than once

The status walk correctly collects paths once, but `status.rs:215-246` calls a statistics helper and then, in deep mode, a patch helper for the same staged/unstaged side. Those helpers independently reload the HEAD tree/index and blobs (`status.rs:350-590`). A modified worktree file is read once for statistics and again for its patch; the histogram diff is also run once for counts and once for hunks.

The current 100-small-file fixture measures 5.75 ms for statistics and 7.15 ms for patches, but it understates this duplication for large generated files, lockfiles, or cold storage.

**Recommendation:** build a per-status-call context containing the index, HEAD tree, workdir, and reusable diff resources. Preserve object IDs from status items where possible. For each dirty side:

- load each blob/worktree file once;
- run one diff;
- accumulate added/removed counts from that diff;
- optionally render hunks from the same result.

Ensure the object cache is configured for a pure file-change request too, not only when commit/ref work happened earlier. Add benchmark rows that vary both dirty-file count and bytes per file.

### 7. Git metadata gating is too coarse for focused callers

`GitRequest::wants_repo_metadata()` becomes true when commits, worktrees, or remote refresh are requested. In `git/types.rs:932-983`, this one predicate brings in remotes, config, all local branches, and tracking even when a caller only wanted recent commits.

Local branch collection performs two reachability walks for each non-current branch (`git/remote_refresh.rs:252-305`). Tracking performs additional walks per remote (`git/remote_refresh.rs:312-354`). Recent-commit collection also deep-clones the supplied ref-decoration map in `git/discovery.rs:255-266`, defeating part of `GitRepo`'s cache benefit.

**Recommendation:** add explicit internal/public request controls for recent commits, ref decorations, branch list, branch divergence, remotes, tracking, config, and worktrees. Preserve existing preset output for compatibility, but allow focused callers to avoid unrelated graph work. Borrow the decoration map and clone only the small decoration vector attached to each returned commit.

For deep containment, stop each ancestry walk once all requested recent commit IDs have been found. For path-history APIs, add a documented maximum scan bound and path-filter the tree diff rather than collecting every changed path in every commit.

## Inventory, docs, and path findings

### 8. The 10,000-file inventory cap does not cap traversal

Both parallel inventory paths continue enumerating the full tree after `MAX_FILES` classifications have been claimed. `system_view.rs:153-165` continues incrementing the atomic counter and returns `WalkState::Continue`; the standalone scanner has the same shape. When docs/manifests are also requested, continued traversal is necessary, but an inventory-only request can stop globally once its limit is reached.

**Recommendation:** track saturation separately from other observers. For inventory-only walks, terminate at the cap. For combined walks, stop classification work at the cap but continue only the active manifest/docs observers. Expose a `truncated`/limit signal so callers do not mistake 10,000 for the actual tree size.

The current parallel “first 10,000 workers win” behavior is not deterministic near the limit. Decide whether the inventory contract requires a deterministic subset. If it does, use a deterministic bounded strategy rather than relying on parallel scheduling order.

### 9. Canonicalization is repeated in hot package lookups

`RepoInfo::package_for_dir` canonicalizes the query once and each package path twice (`repo/types.rs:273-280`). Related area methods canonicalize the same root/query again and can call `package_for_dir` recursively (`repo/types.rs:340-368`). `merge_packages` also canonicalizes every candidate.

`std::fs::canonicalize` is a filesystem operation, not a cheap string normalization. Repeating it is avoidable and particularly expensive on Windows. It also makes performance sensitive to symlink/permission behavior.

**Recommendation:** canonicalize the observation root and caller path once at the API boundary only when symlink semantics require it. Store/reuse normalized absolute package keys during detection and use lexical prefix/depth comparisons afterward. Document whether package ownership follows lexical paths or resolved symlinks, then test that contract on all three OSes.

### 10. Smaller per-file costs remain after walk consolidation

- File classification can allocate lowercase filename/extension strings several times per file. Prefer a case-sensitive ASCII fast path and only allocate/fold when uppercase input requires it.
- Framework classification reads the entire framework file to find a language hint. If the public contract permits, use a documented bounded prefix; otherwise reuse content already read by another active observer.
- Full Markdown parsing performs a separate metadata call for mtime after reading the file. The shared walker can carry entry metadata when mtime is needed, avoiding an extra path lookup.
- Package assignment is O(documents × packages) and returns the first prefix match. Reuse the same deepest-prefix package index used by inventory ownership.
- `ManifestIndex::package_dirs_in_tree` scans all manifest entries for each package subtree. Sort/index entries by normalized path so prefix ranges are found without O(packages × manifests) scans.

These are medium/low priorities. They should follow walk consolidation and be justified with isolated benchmarks.

## Measurement and cross-platform gaps

### Performance collection misses parallel worker detail

The current collector is thread-local. `detect_filesystem_with_request` installs it on the scoped shared-view thread, but `ignore::build_parallel()` executes callbacks on its own worker threads. Per-entry `current_collector()` calls in `system_view.rs:156-178` therefore do not reliably contribute to the request collector unless collector context is explicitly installed/flushed in those workers. The calls still add timing/TLS overhead.

Pass the collector into worker closures and flush worker-local aggregates, or record coarse worker totals in `WorkerBuffers`. Keep all per-entry instrumentation compiled/gated out of the default hot path when neither structured performance output nor the metrics feature is active.

### Benchmark coverage is Linux-centric and misses the proposed work boundaries

`.github/workflows/sniff-performance.yml:28-87` runs Criterion only on `ubuntu-latest` and uploads artifacts without enforcing regression thresholds. Cross-platform CI compiles/runs tests on macOS, Linux, and Windows, but does not measure their filesystem behavior.

Add the following cases:

- formatting-only request on a deep/wide tree;
- package-scoped inventory with Git enabled inside a large monorepo;
- full integrated observation versus standalone `detect_repo`;
- nested-marker discovery with and without a supplied observation index;
- structure-only package discovery with 100/500/2,000 mixed-ecosystem packages;
- more than 10,000 files, with inventory-only and inventory+docs modes;
- 100 dirty files at 1 KiB, 100 KiB, and multi-megabyte sizes;
- many refs/local branches and large branch divergence;
- hundreds/thousands of Markdown docs and package-prefix assignment;
- warm and cold-ish runs on case-sensitive and case-insensitive filesystems.

Keep the Linux PR artifact job for quick feedback. Add a scheduled macOS/Linux/Windows benchmark matrix and compare results only within the same OS/runner class. Hosted-runner timing is noisy, so use it first for artifacts and major-regression detection; reliable percentage gates require stable self-hosted runners or a sufficiently characterized noise band.

On every OS, report work counts alongside time: directory entries visited, file opens, metadata probes, canonicalizations, bytes read, manifest parses, status walks, blob loads, and graph walks. Work-count assertions are more portable than one universal millisecond budget.

## Cross-platform implementation guardrails

1. Continue using `ignore`, `std::fs`, and `gix`; do not shell out to `find`, `dir`, or platform Git for local observation.
2. Keep paths as `Path`/`PathBuf`/`OsStr` (and byte-native Git paths) until the serialization boundary. Avoid slash-string conversion in hot matching loops.
3. Prefer per-request reuse over persistent caching. Persistent filesystem caches need invalidation and can return stale host facts.
4. Make walker parallelism configurable internally and benchmark it. More threads help warm SSD scans but can hurt HDDs, Windows Defender-scanned trees, and network shares. Cap fan-out rather than tying it directly to large CPU counts.
5. Preserve ignore semantics and skipped-directory policy across every fallback path. A faster path that observes a different tree is not equivalent.
6. Avoid changing symlink traversal or case-sensitivity semantics as an incidental optimization; test those behaviors on macOS, Linux, and Windows.

## Recommended sequence

| Phase | Work | Expected effect |
|---|---|---|
| 1 | Remove formatting-triggered walk; choose narrow walk root; move instead of clone; stop inventory-only at cap | Small, low-risk changes with immediate everyday wins |
| 2 | Extend shared observation evidence; route standalone repo detection through it; consume the index in nested/glob detection | Removes repeated directory enumeration and metadata storms |
| 3 | Introduce deduplicated `PackageSeed` + per-run manifest store; make structure-only genuinely shallow | Largest repo/package scaling improvement, especially on Windows |
| 4 | Consolidate Git diff inputs/results; split Git metadata request flags; borrow ref caches | Improves dirty-file, branch-heavy, and focused-history workloads |
| 5 | Optimize canonicalization, docs/package prefix lookup, classification allocations, and manifest prefix queries | Targeted cleanup after profiles identify remaining hot spots |
| 6 | Add scheduled cross-platform performance matrix and work-count regression tests | Prevents platform-specific regressions and validates the gains |

## Acceptance criteria

- A formatting-only request performs no descendant directory walk.
- A package-scoped Git+inventory request visits only the requested/package subtree unless repo-wide data was explicitly requested.
- Integrated full filesystem observation performs one repo-wide directory enumeration for non-Git filesystem evidence; active specialized detectors consume its index instead of walking again.
- Each unique package manifest is opened/parsed at most once per detection detail phase, and structure-only detection performs no dependency/test-runner/language enrichment.
- Final `FilesystemInfo` assembly does not deep-clone `RepoInfo` or the Markdown metadata vector.
- A dirty file's worktree bytes and blob sides are loaded once per status request, and deep mode derives counts and patches from one diff.
- Focused commit requests do not execute branch divergence/tracking walks unless requested.
- Inventory-only observation terminates at its declared cap and reports truncation.
- Work-count tests and output parity tests pass on macOS, Linux, and Windows; performance comparisons are made within each platform.

## Documentation drift found during review

Per repository convention, the code was treated as correct and the comments/docs as stale:

- The huge fixture is 375 packages, while benchmark IDs/README call it 500.
- `formatting.rs` says `.editorconfig` detection traverses parent directories, while `find_editorconfig` checks only the provided directory.
- `filter_inventory` says it uses `Arc::make_mut`; it actually shares the `Arc` only for the no-filter case and allocates a filtered vector otherwise.
- The architecture description says the shared walk collects exactly what is needed, but formatting-only requests currently start an empty-purpose full walk.
- The general “10–50×” structure/full claim is not supported by the current 375-package synthetic measurement (about 2.4× on this host) without workload qualification.

Correct these alongside the behavior/benchmark changes that make each statement authoritative again.

## Bottom line

The central design should be “observe once, project many times.” Sniff already applies that principle to inventory/docs/manifests, but nested detection, package enrichment, result assembly, and rich Git status still escape the shared context. Closing those gaps should improve macOS and Linux immediately and should yield even larger gains on Windows, where redundant stats, canonicalizations, and file opens are generally more expensive.
