---
status: ready for planning
date: 2026-07-16
reviewed: true
review_iterations: 12
reviewed_by: codex/default
reviewed_on: 2026-07-16
source_reviews:
  - ../../reviews/2026-07-13-perf/spec.md
  - ../../reviews/2026-07-14-filesystem-observation/review.md
---

# Sniff Performance Improvement Specification

Improve Sniff's performance by eliminating duplicated, unrequested, and unbounded work while preserving its public data contracts and cross-platform behavior on macOS, Linux, and Windows.

The governing design is **observe once, project many times**. A detection request should acquire each expensive host fact once, retain only the reusable evidence needed for that request, and let the library project that evidence into its public results. The CLI remains a thin renderer of library-provided information.

This specification combines the Sniff-wide performance review from 2026-07-13 with the deeper filesystem observation review from 2026-07-14. The later review refines two conclusions from the earlier review:

- Formatting detection is cheap, but the filesystem planner currently starts an empty-purpose descendant walk for a formatting-only request.
- The shared filesystem view successfully reuses manifests, inventory, and Markdown collection, but nested-workspace discovery, membership expansion, and package enrichment still perform redundant filesystem work outside that view.

These are refinements of the shared-work model, not reasons to replace it.

## Reader's note — inline review decisions

This review keeps the “observe once, project many times” design and makes the following decisions explicit:

- This is an umbrella specification. Implementation is split into phase-level sub-specs so filesystem, Git, remote/network, and subprocess changes can be reviewed and verified independently. Phase 0 work accounting is a dependency of every optimization phase.
- `RepoRequest::structure()` is defined by its documented shallow contract, not by fields the current implementation happens to over-populate. Structure mode keeps package identity, ownership, provenance, and topology, but leaves enrichment-only fields empty or absent; callers that need dependencies, test runners, features, languages, frameworks, or file lists must request full repository detail.
- A truncated inventory has a stable count and ordering contract, but not a stable selected subset. Making the first 10,000 paths deterministic would require enumerating and ordering the entire tree, defeating global early termination. Complete inventories remain deterministic after sorting.
- Normalized path keys remain `PathBuf`-native and component-aware. They must not be lowercased or converted to slash-delimited strings; resolved-symlink and platform case behavior remain unchanged.
- A provider-reported truncated remote tree is not complete evidence. The shared recursive response may be followed by bounded provider-specific pagination or subtree requests; the optimization removes duplicate equivalent requests, not correctness-preserving continuation requests.
- Fine-grained Git metadata controls and inventory completeness fields extend public structs and are therefore source-breaking for downstream code that constructs those structs with literals. This is accepted while the codebase has no established users; serialized legacy requests and existing preset result values remain compatible.

The one remaining owner-level decision is the public return contract for bounded path history. It is recorded in [Open Questions](#open-questions) with a recommendation rather than hidden in implementation planning.

## Problem statement

Sniff already gates most expensive capabilities behind request flags and runs independent top-level domains concurrently. Its remaining performance problems are concentrated at reuse boundaries:

- `sniff repo --json` repeats Git status and repository discovery after full detection has already produced the same facts.
- Filesystem planning sometimes starts a walk that the request does not need or widens a package-scoped walk to the repository root merely because Git was requested.
- Full and standalone repository detection enumerate the same tree multiple times for manifests, nested markers, membership globs, inventory, and leaf evidence.
- Package candidates are enriched before duplicate boundaries are merged, causing repeated probes and manifest parsing.
- Rich Git status, history, branch, and containment paths reload inputs or walk graphs more often or farther than the requested result requires.
- Remote reports fetch the same metadata and repository tree multiple times.
- Default NTP observation can add a network round trip, while service and other subprocess-backed probes can multiply process startup cost or hang without a deadline.
- Current performance instrumentation and CI do not reliably capture work performed by parallel filesystem workers or platform-specific regressions.

These costs are amplified on Windows, antivirus-scanned directories, network shares, cold storage, branch-heavy repositories, and large polyglot monorepos. Reducing filesystem operations and subprocess/network round trips is more valuable and portable than adding more parallelism.

## Goals

1. Ensure each requested local filesystem tree is enumerated at most once per detection scope, except where a specialized algorithm demonstrably requires different traversal semantics.
2. Ensure each repository discovery, status walk, file diff, manifest parse, Git graph walk, subprocess query, and remote payload is performed no more often than its output contract requires.
3. Make structure-only and focused request modes genuinely shallow.
4. Bound operations whose cost currently grows to full history, all refs, all services, or an unbounded subprocess wait.
5. Preserve the public `sniff-lib` result contracts and the CLI's text, plain, JSON, stdout/stderr, and exit-code behavior unless this specification explicitly identifies an intentional change and its migration impact.
6. Verify work reduction with deterministic counters and cross-platform tests, using wall-clock benchmarks as directional evidence rather than a universal budget.

## Non-goals

- A persistent or cross-invocation filesystem cache. Host and repository observations must remain fresh without invalidation machinery.
- Platform-specific filesystem scanners or shelling out to `find`, `dir`, or platform Git for local observation.
- Changing ignore, skipped-directory, symlink, or case-sensitivity semantics as an incidental optimization.
- Redesigning CLI output schemas or terminal presentation.
- Optimizing every low-severity allocation identified by the reviews without profiling evidence after the structural work lands.
- Enforcing one wall-clock threshold across hosted macOS, Linux, and Windows runners.
- Changing correctly gated costs such as explicit deep remote refresh or explicit macOS audio detection, except to add safe subprocess deadlines where required.

## Baseline evidence

The 2026-07-14 measurements were warm-cache, synthetic-fixture results on macOS/APFS. They are directional baselines, not cross-platform service-level objectives.

| Case | Median/typical estimate |
|---|---:|
| Staged filesystem: Git summary + repo structure | 23.0 ms |
| Staged filesystem: all filesystem stages | 43.3 ms |
| Repo structure, 375-package fixture | 53.2 ms |
| Full repo, 375-package fixture | 129.0 ms |
| Git file statistics, 100 dirty files | 5.75 ms |
| Git unified diffs, 100 dirty files | 7.15 ms |

The fixture currently called “huge 500 packages” contains 375 packages: 200 Rust, 100 JavaScript, 50 Python, and 25 Go. The measured structure/full ratio on that fixture was about 2.4×, so the existing general “10–50×” claim must not be used without a qualified workload.

Performance acceptance is primarily structural: fewer directory entries visited, file opens, metadata probes, parses, status walks, blob loads, graph visits, subprocesses, and HTTP requests for equivalent output.

## Architectural constraints

- Business logic belongs in `sniff-lib`. `sniff-cli` may select a request, call the library, and serialize or render the returned facts; it must not independently rediscover host or repository state.
- Local Git observation continues to use the existing `gix` repository model.
- Filesystem observation continues to use `ignore`, `std::fs`, `Path`/`PathBuf`/`OsStr`, and the existing prune policy.
- Reusable state is per detection request. It must not outlive the request unless an existing public cache contract already allows it, such as the WAN IP TTL cache.
- Evidence indexes must store compact facts, not every `DirEntry` or full file content.
- Parallelism must be bounded and internally configurable for benchmarks. It must not scale blindly with CPU count on storage-bound workloads.
- Equivalent requests must observe the same tree and produce the same results on macOS, Linux, and Windows, except for the explicitly unspecified selected subset of a truncated parallel inventory.

## Requirements

### R1. Establish reliable work accounting

Before structural changes are evaluated, make performance collection accurately include work performed by `ignore::build_parallel()` workers.

1. Pass the active collector into walker worker closures and flush per-worker aggregates, or record coarse worker totals in `WorkerBuffers`.
2. Compile or gate per-entry timing/TLS work out of the default hot path when structured performance output and metrics are both inactive.
3. Add counters for at least:
   - directory entries visited;
   - files classified and inventory saturation;
   - file opens, metadata probes, canonicalizations, and bytes read where Sniff controls the operation;
   - manifest and lockfile parses;
   - repository discoveries and status walks;
   - blob loads, file diffs, ref walks, and commit visits;
   - subprocess spawns and timeouts;
   - remote API requests by operation.
4. Work-count assertions must be available to tests without making production APIs depend on test-only state.

The counters are acceptance evidence, not a requirement to wrap every standard-library call. Instrument the owned boundaries where one counter corresponds to a meaningful unit of work.

### R2. Reuse the aggregate repository observation

`sniff repo --json` must use the `GitInfo` and `RepoInfo` already produced by its library detection request.

1. Build dirty, staged, unstaged, and untracked buckets from the one detected `file_changes` collection. Scope and file-kind splits are projections, not new observations.
2. Use detected worktrees, branches, repository root, current worktree identity, and conflict data. If an aggregate-required fact is not present, extend the library result or add one library projection rather than implementing detection in the CLI.
3. The aggregate path must perform exactly one status walk in total when its request includes detailed status, rather than one detection walk plus eight post-detection walks.
4. Repository discovery must be shared for all aggregate-local facts. Focused CLI commands may continue to use focused library entry points.
5. Preserve the aggregate JSON schema, ordering guarantees encoded by its serializers, exit behavior, and valid-JSON-only stdout contract. Ordinary successful `--json` execution should not emit stderr output.
6. Remove the aggregate path's independent `detect_repo_identity`, single-package fallback detection, worktree/branch queries, merge-conflict query, and current-worktree query. Extend the one library request or add a library-owned aggregate projection when a fact is missing.
7. `build_aggregate_value` must be a pure projection over already-observed library results and explicit render options. It must not read the filesystem, open a repository, spawn a process, or make a network request.
8. The three commit-family projections may share one library history observation. They must not each walk history, but they are not required to reuse `GitInfo.commits` when that collection lacks the file evidence needed to classify source and documentation changes.

### R3. Make filesystem planning request- and scope-aware

The planner must decide whether and where to walk from the consumers of the shared evidence, independently of Git discovery.

1. Remove formatting from the condition that starts the shared descendant walk. A formatting-only request may probe/read the requested `.editorconfig` behavior but must not enumerate descendants.
2. Introduce a small internal walk-scope decision with table-driven tests. At minimum:
   - full repository detection and repository-wide docs use the repository root;
   - package/base-directory inventory uses the requested or resolved package root when no repo-wide consumer is active;
   - repository structure alone does not require inventory collection;
   - Git handle presence does not itself widen the filesystem scope.
3. Preserve the existing behavior when multiple active consumers require different scopes by choosing the smallest set of walks that satisfies them. Prefer one repository-wide walk only when it can legitimately serve all consumers.
4. Move completed `RepoInfo` and Markdown vectors into `FilesystemInfo` after dependent projections finish. Do not deep-clone them merely to satisfy assembly order; use borrowing and `Option::take`/moves.
5. When docs need repository context but repository output was not requested, discard the internal context after package assignment.

### R4. Extend the shared view into a per-request observation index

Full integrated and standalone repo detection must consume the same compact evidence model.

The observation index must be able to carry, when requested:

- manifest paths, owning directories, and manifest kinds;
- nested workspace marker paths and parent directories;
- solution and leaf-marker evidence required by active standards;
- file classifications up to the declared inventory cap;
- Markdown metadata;
- entry metadata already available from the walker when a consumer needs it.

Requirements:

1. Match nested markers during the existing shared walk. Do not run `walk_for_nested_markers` over a tree the shared view has already enumerated.
2. Route standalone full `detect_repo` through the same shared-view builder so manifest indexing, nested evidence, and inventory share one walk.
3. Make Cargo membership glob expansion and other compatible standards match against indexed manifest/evidence paths rather than walking the prefix tree and probing manifests per directory.
4. Preserve the single-pass nested-marker contract established by the completed faster-package-list work: committed marker evidence follows the walker's ignore/prune behavior, root-marker handling remains unchanged, and fixed marker matching retains current platform case behavior.
5. Specialized fallback detectors may walk only when the requested evidence cannot represent their semantics. Such fallbacks must use the same ignore and prune policy and have an explicit work-count test.
6. Do not retain all directory entries or introduce a process-global observation cache.

For a full integrated filesystem request, the target is one repository-wide directory enumeration for all non-Git filesystem evidence. Standalone full repo detection has the same target.

### R5. Separate package discovery from enrichment

Repository detection must deduplicate package boundaries before performing expensive package work.

1. Workspace and nested detectors produce cheap internal `PackageSeed` values containing:
   - a normalized path key;
   - owning `MonorepoStandard`;
   - provenance;
   - matched manifest/evidence kinds.
2. Merge duplicate seeds by normalized path before name/version resolution, language scanning, framework detection, test-runner detection, feature extraction, or dependency parsing.
3. Enrich each unique seed exactly once for the requested detail level.
4. `RepoRequest::structure()` performs only membership and the minimum parsing required for package identity. It must not parse dependencies, scan languages/frameworks, or probe test-runner configuration.
5. Use one per-detection `ManifestStore` keyed by path for parsed/raw Cargo, Node, Python, Go, lockfile, inherited version, and root-scoped configuration inputs.
6. Parse a root `Cargo.lock`, inherited root `Cargo.toml`, and root-scoped test-runner configuration at most once per detection.
7. Use the observation index's manifest kinds rather than repeating broad `exists()` probes. A fallback probe is allowed only for evidence types the index deliberately does not collect.
8. Index manifest entries by normalized path so package-subtree queries use prefix ranges instead of scanning the complete manifest list once per package.

Structure-only compatibility is intentionally semantic rather than value-for-value. The serialized `Package` and `RepoInfo` shapes remain valid, but enrichment-only optional/vector fields are absent or empty in structure mode even if the current implementation populates some of them accidentally. Package name, relative/absolute path, package area, ecosystem, owning standard, provenance, exclusion state, detected standards, and monorepo layers remain populated. README and rustdoc examples must direct callers to `RepoRequest::full()` for enriched fields.

### R6. Reuse normalized path and ownership indexes

1. Canonicalize the observation root and caller path once at the API boundary only where resolved-symlink semantics require it.
2. Store reusable normalized absolute package keys during detection. Hot ownership lookups use lexical prefix/depth comparisons rather than repeated `std::fs::canonicalize` calls.
3. Define and document whether ownership follows lexical paths or resolved symlinks. Preserve current behavior unless a separate, reviewed compatibility change is required.
4. Use one deepest-prefix package ownership index for inventory, docs, and commit-file attribution.
5. Package assignment must choose the deepest matching package, not whichever prefix happens to appear first.
6. A normalized key is an absolute, component-normalized `PathBuf` derived from the chosen observation root. It must preserve native path encoding and separators, must not use lossy UTF-8 or Unicode case folding, and must compare whole path components rather than string prefixes.
7. On Windows, normalize drive-prefix representation consistently at the boundary. Do not add ad hoc case folding: reuse the canonical/resolved casing when resolved-symlink semantics are required and otherwise preserve the existing lexical behavior.

### R7. Make inventory saturation observable and actionable

1. Inventory-only observation must terminate globally when `MAX_FILES` classifications have been accepted.
2. A combined walk continues after inventory saturation only for still-active manifest, marker, or docs observers; it must stop classification and classification instrumentation.
3. Add `truncated: bool` and `limit: Option<usize>` to `FileInventory`, `FileAssociationBreakdown`, and `LanguageSummary`. Use Serde defaults and omit `false`/`None` so uncapped JSON remains compatible. When truncated, `limit` is the accepted-classification cap.
4. Keep `total_files_scanned`/`total_files` as the number of classifications represented in the result, not an estimate of the tree's actual file count. When truncated, callers use the new fields to distinguish the represented count from a complete count.
5. The selected subset is intentionally unspecified when `truncated` is true because parallel workers race to claim the bounded slots. Sort accepted classifications before projection so ordering is stable, but do not promise the same selected paths across runs. When `truncated` is false, the complete sorted result remains deterministic for an unchanged tree.
6. Correct comments and tests that currently describe the parallel inventory result as unconditionally deterministic. Tests for truncated runs assert the cap, flags, sorting, and path validity rather than exact selected-path equality.

### R8. Compute rich Git status once per file side

1. Build one status-call context containing the index, HEAD tree, worktree, object cache, and reusable diff resources.
2. Preserve object IDs supplied by status items where possible.
3. For each staged or unstaged side, load each blob or worktree file once and run one diff.
4. Derive line statistics and optional unified hunks from that one diff result.
5. Configure object caching for pure file-change requests even when no earlier commit/ref operation initialized it.
6. Keep counts-only, dirty-flag-only, and identity paths shallow; this work must not make them pay for blobs or diffs.

### R9. Split focused Git metadata controls

Add explicit request controls for recent commits, ref decorations, branch list, branch divergence, remotes, tracking, config, and worktrees.

1. Add a `GitMetadataRequest` value to `GitRequest`, exposed through builders. The optional value is omitted when serializing legacy-compatible presets; absence during deserialization derives the current behavior from the existing fields. This preserves old serialized plans and the serialized shape of existing presets while allowing focused callers to opt into the new controls.
2. A focused recent-commit request must not implicitly execute branch divergence, tracking, remotes, config, or worktree work.
3. Plain `full()` must not perform two reachability walks per non-current branch unless branch divergence is explicitly part of the preserved preset contract. If compatibility requires the existing field values, optimize or batch the computation without silently changing them.
4. Borrow the ref-decoration cache and clone only the small decoration vector attached to a returned commit.
5. Reuse remote-tracking tip sets within a request instead of re-globbing/peeling them per tracking query.
6. Worktree listing should reuse existing worktree metadata and avoid serially opening every linked worktree as a full repository. Parallel opening is an acceptable fallback when direct metadata is insufficient.
7. Adding the request field is an accepted Rust source-compatibility break for external struct literals. Update every in-repo literal and make presets/builders the documented construction path; do not change existing preset result values merely because storage was split.
8. Add serialization tests for legacy request JSON without the new field, round-trips for focused controls, and unchanged JSON for `identity()`, `minimal()`/`summary()`, `full()`, and `deep()`.

### R10. Bound Git history and containment work

1. Path-history queries must path-filter the tree diff and short-circuit as soon as a commit is known to touch the requested prefix.
2. Add a work-unit bound on commits examined for path history. Do not use elapsed wall time as the bound because identical repositories must have equivalent completeness across machines. The public return-shape decision and default limit are resolved by [Open Question 1](#open-question-1-path-history-completeness-contract) before this requirement is implemented.
3. Deep remote containment builds a target set from the requested recent commit IDs. Each remote-tip ancestry walk stops once all targets found on that walk have been observed.
4. Store remote identifiers compactly during containment and resolve names when assembling results instead of cloning a remote name for every visited commit.
5. Long history walks must periodically clear bounded diff resource caches.
6. Date-window history remains correct under skewed commit timestamps. Any future age-based early stop requires an explicit, tested skew contract and is not part of this feature.

### R11. Reuse remote and network inputs

1. A remote report resolves repository metadata/default branch once and fetches a recursive repository tree at most once per report.
2. Provider-specific document and CI/CD projections consume the shared metadata/tree. Apply the same contract to GitHub, GitLab, Gitea, and Bitbucket.
3. Independent Bitbucket directory listings may run concurrently after their shared branch input is available.
4. Preserve the public `RemoteRepoProvider` methods and graceful-degradation contract. Use an additive defaulted trait hook or provider-private `RemoteRepoSnapshot`; do not require downstream provider implementations to adopt a new required trait method.
5. A provider-reported truncated recursive tree must not be treated as complete. Continue with provider-supported pagination or bounded subtree/path requests needed to preserve document and CI/CD detection. Count these separately from duplicate root-tree requests, and add fixtures for both complete and truncated responses.
6. If the shared tree request fails, preserve current graceful degradation: metadata remains required, document results may be empty, and CI/CD may still come from workflow-run APIs or another independent source.
7. WAN IP detection uses one reusable blocking client and at least two default HTTPS endpoints so retry behavior provides an actual fallback. Query endpoints sequentially and stop after the first valid IP response; do not race providers and disclose the caller's address to every endpoint on a successful first attempt.
8. Apply the same connect/request deadlines and strict IP-address validation to every WAN endpoint. Response bodies and credentials must not be included in errors or performance output.
9. Preserve explicit refresh and existing TTL behavior. Do not return stale data beyond the documented cache policy.

### R12. Remove default latency cliffs and bound subprocesses

1. Keep explicit `OsRequest::full()` behavior compatible, including NTP when explicitly selected.
2. Change the Tier-1/default detection plan to disable the live NTP probe. Callers that need NTP status opt in through `OsRequest`.
3. If NTP caching is retained or added, use a documented short TTL and preserve force/explicit refresh semantics. Caching is defense in depth, not a substitute for default request gating.
4. Batch systemd PID collection and runit status queries into a constant or bounded number of subprocesses rather than one process per service. Chunk only when command-line length limits require it.
5. Every service backend subprocess and the macOS `diskutil` probe must have a deadline and a defined partial/unavailable result on timeout.
6. The shared subprocess timeout implementation must drain stdout/stderr while the child runs so a full pipe cannot deadlock the child before exit.
7. Apply the pipe-draining timeout fix to program schema and host-capability probes that can emit large output.
8. The Windows locale PowerShell fallback must also have a deadline.
9. The helper accepts an explicit per-probe `Duration`, never invokes a shell, drains both pipes concurrently, kills on timeout, and waits to reap the child before returning. Tests use injected short durations rather than sleeping for production deadlines.
10. Use named defaults of 3 seconds for service and Windows-locale commands and 5 seconds for `diskutil`; retain the existing 2-second host-capability and 3-second program-schema/NTP bounds. A future timeout change is a policy change, not an incidental refactor.
11. Preserve existing public result shapes on timeout. Failure of a primary listing returns the backend's current unavailable/empty result; timeout during enrichment returns the successfully parsed partial list. Emit a tracing diagnostic and increment a timeout counter so “no services” can be distinguished during diagnosis without adding terminal noise.

Changing the default plan's NTP behavior is intentional: OS observation no longer initiates an implicit network probe, while the network domain continues to govern WAN IP requests and `OsRequest::full()` continues to mean full OS observation for explicit callers.

`DetectionPlan::default()` therefore becomes “all domains with safe defaults,” not “every domain at `full()`.” It should construct the default OS request with NTP disabled while `OsRequest::full()` remains unchanged. Update the top-level rustdoc, examples, tests, and Sniff skill wording that currently equate the default plan with `OsRequest::full()`.

### R13. Reuse executable indexes

1. `HostCapabilities::detect()` uses the eager PATH index rather than performing a `which()`-style PATH walk for each candidate name.
2. macOS bundle-inclusive index construction goes through the existing bundle cache.
3. Program and install-method name lookups may use static maps when profiling confirms the current linear scans remain visible after the index fixes.
4. Test-runner local-bin ancestor discovery may be memoized within the request; parallel resolution is optional and must be justified by measurement.

### R14. Optimize remaining hot loops only after structural work

After R2–R13, profile before implementing the following:

- ASCII case-sensitive fast paths for filename/extension classification;
- bounded prefix reads for framework hints, if compatible with the detection contract;
- reuse of walker metadata for Markdown modification time;
- static regex initialization;
- allocation-free or set-backed path-list merges;
- static program-name maps;
- rate-limit matching without whole-body lowercasing;
- interface-cache clone reduction;
- small cache/index improvements in test-runner resolution.

These are valid observations, but they must not delay the structural work or introduce abstractions without a measured benefit.

## Compatibility and intentional behavior changes

### Preserved contracts

- Existing `GitRequest` preset output remains compatible.
- Existing repo/package, file-change, remote-report, and service result shapes remain compatible except for the additive inventory truncation signal and the explicitly shallow values returned by `RepoRequest::structure()`.
- CLI text and `--plain` output remain semantically and visually unchanged.
- `--json` stdout remains a complete, valid JSON document with no informational text mixed into it.
- Ignore/prune, symlink, path encoding, and platform case behavior remain unchanged.
- No CLI-side detector or cache is introduced.
- Remote reports preserve required-metadata and optional-section graceful degradation, including when shared tree evidence is unavailable or truncated.

### Intentional changes

- The default/Tier-1 plan no longer performs live NTP observation. Explicit `OsRequest::full()` retains NTP.
- Structure-only repo detection no longer populates enrichment-only package values that were outside its documented contract.
- Inventory output gains additive serialized `truncated` and `limit` fields, and a truncated subset is explicitly unspecified across runs. Adding fields to the public Rust structs is an accepted source break for struct-literal callers.
- `GitRequest` gains fine-grained metadata storage/builders; adding a field to the public struct is an accepted source break for struct-literal callers, while legacy serialized plans remain compatible.
- Focused path-history APIs gain an explicit incomplete/bounded-history indication, with the exact public migration subject to Open Question 1.
- Installation timeout becomes a first-class outcome rather than an ordinary failure, because R12.5 requires a defined result on timeout and the Unix kill is only best-effort. This is one reviewed contract spanning four accepted source breaks: `InstallCapturedResult` gains `timed_out` (source break for struct-literal callers); `SniffInstallationError` gains `InstallationTimedOut { pkg, manager, timeout_secs }`, which `execute_install`/`execute_versioned_install` now return instead of `PackageManagerFailed`; `InstallInterviewEvent` gains `TimeoutWarning`, emitted after the failure status and before any retry prompt; and `InstallInterviewOutcome` gains `TimedOut`. Both enums are non-`non_exhaustive`, so the added variants are accepted source breaks for exhaustive matchers. Migration: match the new variants; the pre-existing `PackageManagerFailed` and `Failed` meanings narrow to non-timeout failures only.

Any additional behavior or schema change requires a separate review and must not be smuggled in as a performance optimization.

## Verification strategy

### Correctness and work-count tests

Add table-driven/unit/integration coverage for:

- formatting-only requests entering no descendant walker;
- package-scoped Git + inventory walking only the package/request scope;
- integrated and standalone full repo detection performing one non-Git tree enumeration;
- nested and Cargo glob discovery consuming supplied observation evidence;
- duplicate package seeds causing one enrichment pass;
- structure-only mode avoiding dependency, test-runner, feature, framework, language, and file-list work;
- one parse per unique manifest/lockfile/config input;
- inventory-only termination and combined-walk saturation behavior;
- one aggregate status walk and one repository discovery context;
- a pure aggregate projection that performs no filesystem, repository, subprocess, or network observation;
- one blob/worktree load and diff per dirty file side;
- focused commit requests avoiding unrelated ref graph work;
- legacy and focused `GitRequest` serialization behavior;
- bounded path-history and target-set containment walks;
- one metadata and one root recursive-tree request per remote report, plus correctness-preserving continuation for provider-reported truncation;
- sequential WAN fallback that stops after the first valid response;
- batched service commands, subprocess deadlines, and pipe outputs larger than a typical pipe buffer;
- primary-listing timeout, enrichment timeout with partial results, and child reaping;
- eager PATH/bundle index reuse;
- output parity for macOS, Linux, and Windows path/case/ignore fixtures.

Tests must avoid Unix-only imports and path separators unless `cfg`-gated. Build PATH fixtures with `std::env::join_paths`.

### Benchmarks

Extend Criterion coverage with:

- formatting-only on a deep/wide tree;
- package-scoped inventory with Git inside a large monorepo;
- integrated full observation versus standalone `detect_repo`;
- nested discovery with and without supplied observation evidence;
- structure-only discovery at 100, 500, and 2,000 mixed-ecosystem packages;
- inventory-only and inventory+docs trees above 10,000 files;
- final result assembly with hundreds of packages/docs;
- 100 dirty files at 1 KiB, 100 KiB, and multi-megabyte sizes;
- branch-heavy/divergent repositories;
- path history in long histories with sparse prefix matches;
- deep containment across many remote tips;
- hundreds/thousands of Markdown documents and package-prefix assignment;
- remote report request counts with provider fixtures;
- service listing with large synthetic service sets;
- warm and cold-ish runs on case-sensitive and case-insensitive filesystems.

Rename the stale “huge 500 packages” benchmark to describe its actual 375-package fixture, or change the fixture to match the name. Do not silently change both the workload and baseline in one comparison.

### CI

- Keep Linux Criterion artifacts on pull requests for quick directional feedback.
- Add a scheduled macOS/Linux/Windows performance matrix.
- Compare results only within the same OS and runner class.
- Use work-count regressions and major timing regressions as hosted-runner signals. Percentage gates require stable runners or a characterized noise band.
- Run the package area's canonical test, lint, build, and read-only formatting checks during implementation verification.

## Acceptance criteria

The feature is complete when all of the following are true:

- A formatting-only request performs no descendant directory walk.
- A package-scoped Git + inventory request does not scan the monorepo root unless another requested result requires repository-wide evidence.
- Integrated full filesystem observation and standalone full repo detection each perform one non-Git repository enumeration for compatible evidence.
- `sniff repo --json` performs one status walk total and does not rediscover worktrees, branches, or scope buckets independently.
- The aggregate JSON builder is a pure projection and performs no host or repository observation.
- Every unique package is enriched once, and every unique manifest/lockfile/root config is parsed at most once per detail phase.
- Structure-only detection performs no dependency, test-runner, feature, framework, language, or file-list enrichment, and its intentionally empty fields are documented.
- Final filesystem assembly does not deep-clone `RepoInfo` or the shared Markdown vector.
- Inventory-only observation stops at its cap, reports `truncated` and `limit` through every public inventory projection, and does not promise an exact truncated subset.
- Rich status loads each dirty side and computes its diff once.
- Focused Git requests do not execute unrelated branch, remote, tracking, config, or worktree graph work.
- Path-history and remote-containment walks have tested work bounds without using incorrect timestamp assumptions.
- Each remote report fetches repository metadata and makes its root recursive-tree request no more than once; provider-reported truncation triggers tested continuation rather than silent incomplete output.
- Default/Tier-1 detection performs no NTP network request; explicit full OS observation still can.
- Service listing uses batched subprocesses, and every covered subprocess can time out without a pipe deadlock.
- Host-capability and macOS bundle detection reuse eager indexes.
- Existing library/CLI output parity tests pass, including valid JSON-only stdout.
- Cross-platform tests pass on macOS, Linux, and Windows, and the scheduled benchmark matrix emits comparable work-count artifacts.

## Implementation sequence

| Phase | Requirements | Outcome |
|---|---|---|
| 0 | R1 | Trustworthy counters and baselines before changing hot paths |
| 1 | R2, R3, R7, R13.1–R13.2 | Low-risk removal of repeated CLI/status work, accidental walks, avoidable clones, cap overrun, and PATH/bundle rescans |
| 2 | R4 | One shared observation index for integrated and standalone repo detection |
| 3 | R5, R6 | Deduplicated package boundaries, per-request parsing, and shared ownership lookups |
| 4 | R8–R10 | Single-pass status diffs and bounded/focused Git graph work |
| 5 | R11, R12 | Shared remote payloads, real WAN fallback, default NTP gating, batched and bounded subprocess probes |
| 6 | R14 and remaining R13 items | Profile-guided micro-optimizations only where still material |
| 7 | Cross-platform benchmark CI | Ongoing regression visibility after implementation stabilizes |

Each phase must preserve output parity and land its relevant work-count tests. A later phase must not be used to justify leaving a known regression in an earlier phase.

### Delivery as sub-specs

This file remains the umbrella feature and should not be implemented as one pull request. Before coding a phase, create a phase-level specification with `sub-spec: true` and a `depends-on` link:

1. Phase 0 depends on this umbrella specification.
2. Each later phase depends on the phase that establishes the evidence or API it consumes; independent phases may share the same completed dependency when they do not touch overlapping contracts.
3. Phase sub-specs must name the exact counters, public fields, migration steps, platform fixtures, and acceptance commands for their scope.
4. A phase may move to `_completed` independently, but this umbrella feature moves to `_completed` only after the completion boundary below is met.

## Observation disposition

This table ensures every source-review observation has an explicit home.

| Source observation | Disposition |
|---|---|
| 2026-07-13 H1 | R2 |
| H2, H3; 2026-07-14 findings 1–3 | R3–R4 |
| H4, H5 | R10 |
| H6 | R11 |
| H7 | R12 plus documentation corrections |
| H8 | R12 |
| M1–M3; 2026-07-14 findings 6–7 | R8–R10 |
| M4–M8; 2026-07-14 finding 4 | R4–R5 |
| M9 | R3 |
| M10 | R11 |
| M11 | R12 |
| M12 | R13 |
| M13 | R9 |
| L1 | R6 |
| L10 | R5 |
| L12; 2026-07-14 findings 9–10 | R6 and R14 |
| L2 | Explicitly deferred by R10 because timestamp-skew correctness outweighs an unproven optimization |
| L3–L4 | R9–R10 |
| L5 | Covered by R2/R9's per-request reuse; no new public batched API is required unless a caller demonstrates need |
| L6–L9 | Profile-gated by R14 |
| L11 | R12 |
| L13–L14 | R13 |
| L15 | Resolved by the intentional default NTP policy in R12; other default full-domain costs remain request-policy choices |
| L16 | Deferred; macOS audio is correctly gated and a TTL cache requires a demonstrated repeated-detection use case |
| 2026-07-14 finding 5 | R3 |
| 2026-07-14 finding 8 | R7 |
| Measurement and CI gaps | R1 and Verification strategy |

## Documentation maintenance

Update comments and documentation alongside the phase that makes each statement authoritative. Code remains the source of truth where drift was found.

- Rename or correct the 375-package benchmark description.
- Correct `.editorconfig` lookup documentation to match its actual directory/parent semantics.
- Correct `filter_inventory` sharing/copy language.
- Correct the inventory scanner's unconditional determinism claim: ordering is deterministic, while capped parallel selection is not.
- Update the shared-work architecture after the planner and observation index changes.
- Replace the unsupported general “10–50×” structure/full claim with qualified evidence.
- Correct NTP timeout/platform statements: the macOS external bound is 3 seconds in the reviewed code, while Linux uses local `timedatectl` behavior rather than the documented 10-second network cost.
- Document the default-plan NTP policy, explicit opt-in, inventory truncation, bounded path-history behavior, and any new Git request flags.

## Open Questions

### Open Question 1: Path-history completeness contract

Path history currently returns `Vec<CommitInfo>`. Once a scan bound is introduced, that shape cannot distinguish “history exhausted” from “limit reached before enough matches,” and silently returning a short vector would make an incomplete result look authoritative.

#### Option A — replace the focused API with a bounded result (recommended)

Change the fallible/public focused path-history entry points to accept `PathHistoryOptions` and return a `PathHistoryResult` containing `commits`, `commits_scanned`, `history_exhausted`, and `limit_reached`. Use a deterministic default work bound of 10,000 commits and allow callers to choose a lower or higher nonzero bound.

**Pros:** completeness is explicit; every normal path is bounded; one clean API avoids permanent duplicate semantics; work-count tests are deterministic across machines.

**Cons:** Rust callers must migrate; CLI serialization/rendering must deliberately decide whether to expose the metadata; 10,000 is a policy default that may need later tuning from evidence.

#### Option B — add a parallel bounded API and deprecate the current API

Keep `commits_for_path_at`/`get_commits_for_path` temporarily unbounded, add bounded variants returning `PathHistoryResult`, and migrate in-repo callers first.

**Pros:** downstream callers receive a migration window; legacy behavior remains available when completeness is more important than latency.

**Cons:** the original unbounded latency cliff remains public; two nearly identical API families invite the wrong choice; removal creates a second migration later.

#### Option C — retain the vector and return an error at the bound

Keep the existing return type and fail the whole request with a distinct limit-reached error when the scan cap is reached before `count` matches are found.

**Pros:** the success type remains unchanged; incomplete data is never silently presented as complete.

**Cons:** useful partial matches are discarded; callers cannot distinguish an empty history from a costly near-match without error handling; this is awkward for CLI output and graceful degradation.

**Recommendation:** Option A. This repository has no established user base, so accepting one explicit migration now is preferable to preserving an unbounded footgun or carrying two APIs indefinitely. A commit-count bound is portable and reproducible, unlike a wall-clock deadline. The phase sub-spec must confirm or revise the 10,000-commit default with fixture measurements before implementation.

## Completion boundary

Completion requires Open Question 1 to be resolved in its phase sub-spec, R1–R13 to be implemented, all acceptance criteria to pass, output parity to hold except for the documented intentional changes, and cross-platform correctness to be demonstrated. R14 items are included only when post-structural profiling shows a material remaining cost; unimplemented R14 candidates do not block completion when evidence shows they are negligible.
