---
sub-spec: true
depends-on: ../02-reuse-and-scope/spec.md
phase: 3
status: complete
date: 2026-07-17
---

# Phase 3 — One per-request filesystem observation index

Implement umbrella requirement **R4**: integrated full filesystem requests and standalone full
repo detection must share one compact evidence model, and a full request must perform **one**
repository-wide non-Git directory enumeration.

As with Phase 2, every claim is a counter delta against
[`../01-work-accounting/spec.md`](../01-work-accounting/spec.md) and the Phase 2 corrections, not a
wall-clock comparison.

## The three walks this phase collapses

Full repo detection walks the whole tree three times today. Each is a separate `ignore` walker over
the same subtree with the same ignore/prune policy:

| # | Walk | Site | Evidence produced |
|---:|---|---|---|
| 1 | `ManifestIndex::build(root)` | `repo/detection.rs` | manifest paths + kinds |
| 2 | `walk_for_nested_markers(root)` | `repo/nested.rs` | nested workspace marker dirs |
| 3 | `scan_file_inventory(root)` | `repo/detection.rs` (`repo_inventory`) | file classifications |

Plus per-pattern bounded subtree walks in `repo/glob.rs::walk_manifest_dirs`, each of which probes
every candidate directory with four `exists()` calls (`dir_has_manifest`).

The **integrated** path (`WalkScope::Repository`) already supplies 1 and 3 from the shared view, so
it walks twice (shared view + nested). The **standalone** path (`detect_repo`) walks three times.
`detect_repo_with_inventory` already routes through the shared builder but does not supply nested
evidence, so it too walks twice.

Target after this phase: **one** walk for both, plus zero glob subtree walks when evidence is
available.

## Evidence kinds

`FilesystemSystemView` becomes the request-scoped observation index. It retains, when requested:

| Field | Gate | Contents |
|---|---|---|
| `manifest_index` | `collect_manifests` | `ManifestIndex` — indexable manifest dirs + kinds, canonicalized once |
| `manifest_dirs` | `collect_manifests` | Sorted, deduped parents of **every** observed manifest file, unfiltered |
| `nested_markers` | `collect_nested_markers` | Sorted paths of every observed nested-workspace marker file |
| `inventory` | `collect_inventory` | Capped, sorted `FileInventory` |
| `docs` | `collect_docs` | `Vec<MarkdownMeta>` |

It never retains `DirEntry` values or file bodies, and there is no process-global cache (R4.6).

### Why `manifest_dirs` is separate from `manifest_index`

`ManifestIndex` excludes generated (`is_generated_manifest`) and fixture (`is_fixture_manifest`)
manifests, because those are not package boundaries for discovery purposes. Membership **glob**
expansion has never applied that exclusion — `glob.rs::dir_has_manifest` resolves a boundary by
marker presence alone.

Routing glob expansion through the filtered index would therefore silently drop a declared
workspace member whose manifest happens to contain the string `auto-generated`, and would do so
only in full mode (structure mode has no index), making structure and full disagree about
membership. `manifest_dirs` is the unfiltered evidence set, so glob expansion keeps its exact
predicate.

### Glob parity argument

`walk_manifest_dirs` walks only the literal-prefix subtree of each include glob; `manifest_dirs`
covers the whole observation root. These produce the same match set because every include glob
begins with its own literal prefix, so a manifest directory outside that prefix cannot match the
glob. Filtering by pattern after the fact is therefore equivalent to bounding the walk beforehand.

Root handling is preserved: a manifest at the observation root records the root as a manifest
directory in both designs.

## Ignore, prune, and case behavior (R4.4)

The shared walk and both replaced walks already use identical policy — `hidden(false)`,
`git_ignore(true)`, `git_global(true)`, `git_exclude(true)`, and a `filter_entry` that prunes
`should_skip_directory_name` directories — so consuming shared evidence changes none of it.

Preserved exactly:

- **Root-marker handling.** Nested discovery stays non-root-only; a marker at the observation root
  registers no nested candidate.
- **Fixed-marker case behavior.** `marker_name_matches` remains byte-exact on Unix and ASCII
  case-insensitive on Windows; `*.sln` / `*.slnx` suffix matching remains byte-exact on every
  platform.
- **Directory-name marker contract.** Only non-directory entries are marker evidence.
- **Native path encoding.** Evidence is `PathBuf` throughout; no lossy UTF-8 conversion is
  introduced.

**Committed-marker ignore semantics.** Gitignored markers are not detected. This is not a new
change — it is the contract the completed `2026-06-20-faster-package-list` feature established when
`walk_for_nested_markers` replaced the per-directory probe loop. Consuming shared-walk evidence
extends the same already-accepted semantics to glob expansion, whose `dir_has_manifest` probe
previously saw gitignored manifests. Workspace manifests are conventionally committed.

## Allowed specialized fallbacks (R4.5)

Every fallback below uses the same ignore/prune policy as the shared walk and is made explicit by a
counter.

| Fallback | Semantic reason it cannot consume the index | Counter |
|---|---|---|
| `walk_for_nested_markers` | Structure-only detection needs marker evidence but no manifest index, inventory, or docs. Building the full observation index for it would classify every file in the tree for evidence it discards — the Phase 2 "smallest evidence set" rule. | `filesystem.repo.nested_marker_walks` |
| Lazy `ManifestIndex::build` | A root with **no** workspace marker may still hold nested markers deeper down. Outcomes are unknown until after the nested walk, so full enrichment for that topology must build the index afterwards. Rare; gated on `outcomes` being non-empty. | `filesystem.io.read_dirs` |
| `scan_file_inventory` | Same nested-only topology as above. | `filesystem.walk.walks_started` |
| `glob::walk_manifest_dirs` | Structure-only detection has no observation index to query. | `filesystem.repo.membership_glob_walks` |
| Bazel / Pants / Buck2 | Leaf-marker polyglot detectors segment nested workspace roots internally during their own walk; the marker table deliberately excludes them. Pre-existing, unchanged by this phase. | — |

## Where the observation index is built

The `has_workspace_marker(root)` gate is preserved and moves up to `detect_repo_inner`. It is what
keeps `detect_repo` on a large non-repository directory (a system temp dir) from enumerating
unrelated subtrees, and this phase must not regress it:

| Request | Observation index | Walks |
|---|---|---:|
| standalone full, root has a workspace marker | built by `detect_repo_inner` | 1 |
| standalone full, no workspace marker | none; nested fallback walk | 1 (+2 only for a nested-only topology) |
| standalone structure-only | none; nested fallback walk | 1 |
| integrated, `WalkScope::Repository` | built by the planner, passed down | 1 |
| integrated, `WalkScope::Package` / `None` | none (structure-only by construction) | 0–1 |

## `RepoEvidence`

The evidence bundle is borrowed and `Copy`, replacing the two loose `Option<&_>` parameters
`detect_repo_inner_with_shared` carried:

```rust
// sniff/lib/src/filesystem/repo/detection.rs
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RepoEvidence<'a> {
    pub(crate) manifest_index: Option<&'a ManifestIndex>,
    pub(crate) manifest_dirs: Option<&'a [PathBuf]>,
    pub(crate) nested_markers: Option<&'a [PathBuf]>,
    pub(crate) inventory: Option<&'a FileInventory>,
}

impl<'a> RepoEvidence<'a> {
    pub(crate) fn from_view(view: &'a FilesystemSystemView) -> Self;
}
```

`from_view` is the only constructor from a walk, so evidence and the root it was observed from
cannot drift apart at a call site.

## Counters

Two new names (Phase 2 added none):

| Counter | Meaning |
|---|---|
| `filesystem.repo.nested_marker_walks` | Nested-marker fallback walks started |
| `filesystem.repo.membership_glob_walks` | Membership-glob fallback subtree walks started |

Both name a fallback that `filesystem.io.read_dirs` cannot isolate: five unrelated detectors share
that counter (`test_runner_usage.rs` alone increments it at five sites), so it cannot answer "was
the tree enumerated again?".

Phase 3 expectations:

| Counter | Expectation |
|---|---|
| `filesystem.walk.walks_started` | `1` for standalone full and integrated full over a marker-bearing root |
| `filesystem.repo.nested_marker_walks` | `0` when evidence is supplied; `1` for structure-only |
| `filesystem.repo.membership_glob_walks` | `0` when evidence is supplied |
| `filesystem.io.metadata_probes` | falls by the `dir_has_manifest` probe storm (4 per candidate dir) |

## Acceptance

Commands run from `sniff/`:

| Command | Requirement |
|---|---|
| `just test` | pass, modulo the two Phase 1 pre-existing failures |
| `just lint` | pass |
| `just build` | pass |
| `cargo run -p sniff --release --example work_counts` | counters compared against Phase 2, every delta explained |

Equivalence: integrated and standalone full results must agree on topology, inventory, docs, nested
workspaces, solutions, and leaf packages.
</content>

## As built

### Walks removed

Measured with `performance::testing::measure` on the Cargo-workspace-plus-nested-pnpm fixture,
comparing the pre-index path (`detect_repo_inner_with_shared` with no evidence — still reachable by
construction, and exactly what standalone full detection used to do) against the index path. Both
arms ran in the same process on the same tree, so this is a drift-free comparison rather than a
cross-run one.

| counter | pre-index | index | delta |
|---|---:|---:|---:|
| `filesystem.io.read_dirs` | 23 | **19** | −4 |
| `filesystem.repo.nested_marker_walks` | 1 | **0** | −1 |
| `filesystem.repo.membership_glob_walks` | 2 | **0** | −2 |
| `filesystem.io.metadata_probes` | 309 | **288** | −21 |
| `filesystem.io.file_opens` | 17 | 17 | 0 |
| `filesystem.io.bytes_read` | 543 | 543 | 0 |
| `filesystem.repo.manifest_parses` | 10 | 10 | 0 |
| `filesystem.walk.walks_started` | 1 | 1 | 0 |

The −4 `read_dirs` are the manifest index's walk, the nested-marker walk, and two membership-glob
subtree walks. The −21 probes are the `dir_has_manifest` storm those glob walks performed. Reading
is unchanged, which is the point: the same manifests are read the same number of times, from one
walk instead of three. `observing_once_changes_work_not_results` asserts both halves and compares
the serialized `RepoInfo` for equality.

### `walks_started` cannot show this win

`filesystem.walk.entries_visited` and `filesystem.walk.walks_started` are incremented only by
`build_filesystem_system_view`. The three walks this phase collapses were `ManifestIndex::build`,
`walk_for_nested_markers`, and `glob::walk_manifest_dirs` — each a full `ignore` walker recording
**one** `read_dirs` and nothing per entry. So a removed whole-tree enumeration shows up as a single
`read_dirs` decrement, badly understating it.

This is why the two new counters exist: `nested_marker_walks` and `membership_glob_walks` name the
fallbacks so a reviewer can see whether the tree was enumerated again, which `read_dirs` cannot show
(five unrelated detectors share it — test-runner usage alone increments it at five sites).

### Defect found: `ManifestIndex::build` reported none of its reads

`ManifestIndex::build`'s `build_parallel` workers carried **no `performance::WorkerCollector`**.
Recording writes to a thread-local buffer that only `PerformanceCollector::snapshot` (on the
snapshotting thread) and `WorkerCollector`'s drop ever drain, so every `is_generated_manifest` read
those workers performed — one per `Cargo.toml`/`pyproject.toml` in the tree — was silently
discarded.

This is the same class of failure Phase 2 found in `with_current_collector`, and the same one R1
exists to prevent. Phase 1 closed it for `ignore::build_parallel` in `system_view` and for Rayon
pools, and did not reach this walker.

**How it surfaced.** Routing standalone full detection through the (correctly instrumented) shared
walk made `repo_full_huge_375_packages` report `file_opens` 758 → 1009 and `bytes_read`
47708 → 66348. That looked like a regression from a change that strictly removes work. It was not:
+251 is exactly the fixture's `Cargo.toml` count — reads that always happened and were never
counted.

**Fix.** `ManifestWorker` gained a `WorkerCollector` (`inherit()` on the spawning thread,
`activate()` in the first callback, flush on drop), matching `system_view`'s `WorkerBuffers`.
Regression test: `manifest_index_build_reports_its_reads`.

**Consequence for baselines.** Phase 1's archived `file_opens` and `bytes_read` under-report every
case that built a manifest index. The corrected values are below. This is newly-*visible* work, not
new work — after the fix, the pre-index and index arms agree exactly on `file_opens` (17 = 17),
which is the proof.

### Phase 3 counters

`cargo run -p sniff --release --example work_counts`, same host and fixtures as Phases 1–2.

| Case | Phase 1/2 | Phase 3 |
|---|---|---|
| `repo_structure_huge_375_packages` | 13274 probes / 702 read_dirs / 454 parses / 457 opens | identical, **plus `nested_marker_walks: 1`** (the documented structure-only fallback, now explicit) |
| `repo_full_huge_375_packages` | 1403 read_dirs / 758 opens / 47708 bytes | **1401 read_dirs**; `1009` opens / `66348` bytes — the visibility correction above, not a regression |

`repo_full`'s `read_dirs` falls by only 2 because the 375-package fixture declares **explicit**
Cargo members (`"crates/pkg000"`, …), not globs, so `expand_membership_globs` never took its walking
path for this fixture — it resolves literal members by `probe_exists`. The two removed walks are the
manifest index's and the nested-marker walk. The remaining ~1400 `read_dirs` are per-package
test-runner and enrichment scans (five `read_dirs` sites in `test_runner_usage.rs`), which **Phase 4**
owns (R5.3: enrich each unique seed exactly once).

The fixture therefore does not exercise the glob path at all. That path is covered by test instead
(`observing_once_changes_work_not_results`, whose fixture uses `crates/*`). A benchmark fixture with
globbed membership would be a fair Phase 4 addition.

### Deviations from the design above

None. `manifest_dirs` is carried unfiltered as designed, so glob expansion keeps its exact predicate
and structure/full cannot disagree about membership.

### Files

Primary: `filesystem/system_view.rs`, `filesystem/repo/nested.rs`, `filesystem/repo/detection.rs`,
`filesystem/repo/glob.rs`, `filesystem/repo/manifest_index.rs`, `filesystem/mod.rs`,
`filesystem/repo/types.rs`, `performance/counters.rs`.

Evidence threading (mechanical, `RepoEvidence` replacing `Option<&ManifestIndex>` or adding a
parameter): `repo/cargo.rs`, `repo/npm.rs`, `repo/uv.rs`, `repo/nx_turbo.rs`.

### Unrelated test corrected

`sniff::integration::test_performance_collector_thread_local_aggregation` asserted the
**pre-Phase-2** contract: its body says "with_current_collector does NOT flush on exit" and spawned
two batches of worker threads, the first of which existed only to demonstrate that its data was
lost. Phase 2 made `with_current_collector` flush, so the first batch's records now land too and the
counts became 8/11 rather than 5/8. Per the repo's drift rule the code is authoritative: the
redundant second batch and its stale commentary are removed, and the test asserts the original
intent (2 main + 3 spawned) against the current contract.

## Gate for Phase 4

R4 is implemented and asserted by counter. Carried forward deliberately:

- **Phase 1 baselines for `file_opens`/`bytes_read` are wrong** for every manifest-index case. Phase
  4 must compare against the Phase 3 table above, not the archived Phase 1 values.
- `ManifestIndex::package_dirs_in_tree` still scans the full entry list per query. Phase 4 owns the
  prefix-range index (R5.8) and the deepest-prefix ownership index (R6.4).
- Per-package test-runner/enrichment `read_dirs` dominate the 375-package full case (~1400 of 1401).
  Phase 4's R5.3/R5.6 own that.

### Criterion: not measurable on this host, and the run proves it

`cargo bench -p sniff --features network --bench perf -- filesystem_repo` was run and its timing
comparison is **rejected as noise**, not reported as a result.

The host carried a load average of **57–87 on 16 cores** (4–5× oversubscribed) during the run.
`filesystem_repo/repo_structure_huge_375_packages` is the drift bracket that settles it: Phase 3
does not change structure-only detection at all — its counters are byte-identical to Phase 1
(13274 probes / 702 read_dirs / 454 parses / 457 opens) — yet Criterion reported **+330%** for it.
A number that large on provably unchanged work measures the host, not the code. Every other
`change:` line in the same run is therefore uninterpretable, in either direction.

This is the failure mode Phase 1's "Timing-noise warning" and Phase 2's "compare bracketed on one
idle host" rule exist to prevent, and it is why the umbrella plan makes work counters the primary
structural acceptance evidence. The counter deltas above are unaffected by load and stand on their
own.

**Outstanding for Phase 8:** re-run `filesystem_repo` and `filesystem_staged` on an idle host with
the structure-only case retained as the bracket. Directionally, the change removes whole-tree
enumerations and adds none, and no counter moved in the wrong direction after the visibility fix.
