---
sub-spec: true
depends-on: ../03-observation-index/spec.md
phase: 4
status: in-progress
date: 2026-07-17
---

# Phase 4 — Deduplicate packages, parsing, and ownership

Implement umbrella requirements **R5** (discovery vs. enrichment) and **R6** (normalized ownership) on
top of the Phase 3 observation index. The governing rule: **discovery must finish and deduplicate
boundaries before enrichment begins.**

As with Phases 1–3, every claim is a counter delta, not a wall-clock comparison. Baselines are the
**Phase 3** table (`../03-observation-index/spec.md`), not the archived Phase 1 values — Phase 3
corrected a visibility defect that made Phase 1's `file_opens`/`bytes_read` under-report every
manifest-index case.

## The defect this phase closes

Measured on the current `HEAD` with `cargo run -p sniff --release --example work_counts`:

| Case | `package_enrichments` | Unique package boundaries |
|---|---:|---:|
| `staged_filesystem_summary_git_plus_repo` (structure) | 90 | 90 |
| `staged_filesystem_full_all_stages` (full) | **180** | 90 |
| `repo_structure_huge_375_packages` | 300 | 300 |
| `repo_full_huge_375_packages` | **600** | 300 |

**Full detection enriches every package exactly twice.** `create_package` — which reads and parses
manifests, probes the ecosystem, and runs the per-package test-runner search — runs once per
boundary in structure mode and twice per boundary in full mode. `merge_packages` then dedupes by
canonicalized path, so the duplicate is invisible in the output and the entire second enrichment is
discarded work.

### Root cause

`detect_repo_inner_with_shared`'s full-mode block re-discovers packages *inside each already
discovered package's own subtree*:

```rust
for package in &workspace_packages {
    packages.extend(discover_packages_from_index(&package.path, root, …, index));
}
```

`ManifestIndex::package_dirs_in_tree(search_root, root)` filters on
`entry.canonical.starts_with(search_root) && entry.canonical != root`. A package's own directory
satisfies both — it starts with itself and is not the repo root — so the query returns the search
root itself and re-enriches it.

The walking predecessor this replaced, `discover_packages_from_manifests_in_tree`, excluded the
search root explicitly (`if parent == search_root { continue; }`). The exclusion was lost when the
index path was introduced; `discover_packages_with_optional_index` still carries the comment
asserting the old contract ("matches original … which skips search_root"), and is correct only
because it happens to pass `root, root`.

### Why the fix is architectural, not a one-line patch

Restoring the `search_root` exclusion would fix this instance. It would not fix the class: any two
detectors that resolve the same boundary — overlapping globs, a nested marker under a globbed
member, a lockfile-corroborated member also matched by a manifest scan — pay full enrichment twice
and rely on `merge_packages` to hide it.

R5 prescribes the structural cure: detectors return cheap **seeds**, seeds are merged by normalized
key, and only the surviving unique seeds are enriched. Deduplication moves *before* the cost instead
of after it, which makes double enrichment unrepresentable rather than merely absent.

## `PackageSeed`

A boundary descriptor cheap enough to produce speculatively and discard.

```rust
// sniff/lib/src/filesystem/repo/seed.rs
pub(crate) struct PackageSeed {
    /// Native path exactly as discovered; the enrichment input.
    pub(crate) path: PathBuf,
    /// Normalized absolute key. Whole-component comparison only.
    pub(crate) key: PathBuf,
    /// Relative path in the frame the producing detector used.
    pub(crate) relative: String,
    pub(crate) standard: MonorepoStandard,
    pub(crate) provenance: PackageProvenance,
    /// Manifest kinds observed at this boundary, from the index when available.
    pub(crate) evidence: BTreeSet<ManifestKind>,
    pub(crate) is_excluded: bool,
}
```

Producing a seed performs **no** file read, no manifest parse, and no test-runner search. It costs
one normalization of an already-owned path.

`DetectorOutcome.packages: Vec<Package>` becomes `DetectorOutcome.seeds: Vec<PackageSeed>`.
`build_monorepo_layers` consumes `seed.relative`, which is the only `Package` field it ever read.

### Merge semantics

Seeds merge by `key` under the same precedence `merge_package_into` applies today, so the merged
result is identical to what post-hoc merging produced:

- A non-`Unknown` `standard` wins over `Unknown`; `provenance` follows the surviving authority.
- `is_excluded` is the OR across merged seeds.
- `evidence` is the union.
- First-seen `path` and `relative` win, preserving current ordering.

## `ManifestStore`

One store per **detection**, replacing the per-`create_package` `ManifestCache`.

`ManifestCache` is constructed fresh inside `create_package` (via `PackageBuildContext::new`), so
its cache lifetime is a single package. Every input outside that package's own directory is
therefore re-read once per package. On a 375-package Cargo workspace whose members declare
`version.workspace = true`, `cargo::read_toml_at` reads and parses the **root `Cargo.toml` 300
times** — it bypasses the cache entirely by design ("without disturbing the caller's manifest
cache").

`ManifestStore` is keyed by normalized native path and holds parsed Cargo/Node/Python, raw `go.mod`
and generic text, plus root-scoped inputs (`Cargo.lock`, inherited root `Cargo.toml`, root-scoped
test-runner config). Detection is single-threaded, so the store is `RefCell`-backed and hands out
`Rc<T>` rather than borrows.

Counter contract: **one parse per unique input per detection.**

## Normalized-path boundary operation (R6.1)

One operation, `normalized_key`, is the single place a path becomes a comparison key:

- Canonicalizes **only** where existing resolved-symlink semantics already require it — i.e. where
  `merge_packages` calls `canonicalize_path` today — and falls back to lexical `normalize_path`.
- Compares whole components; never string prefixes. `crates/pkg-a2` must not match a `crates/pkg-a`
  prefix.
- Preserves native encoding and separators; no lossy UTF-8 conversion.
- Normalizes Windows drive prefixes without ad hoc case folding, and preserves existing lexical case
  behavior on every platform.

## Deepest-prefix ownership index (R6.4)

One index answers "which package owns this path?" for inventory, docs, aggregate buckets, and
commit-file attribution. Today three implementations answer it independently:

| Site | Current mechanism |
|---|---|
| `detection::refresh_package_boundaries` | walks each classification's parents against a `HashMap<&Path, usize>` |
| `types::RepoInfo::package_for_dir` | scans all packages, keeps longest match |
| `polyglot.rs` owner resolution | deepest `nested_root` marker |

The index compares **path components and depth**, so a nested package always beats a shallower
prefix and `pkg-a2` never matches `pkg-a`.

## Structure-only migration (R5.6) — BLOCKED, needs owner decision

The plan's R5.6 task reads:

> Make `RepoRequest::structure()` perform membership and minimum package identity parsing only;
> leave dependency, test-runner, feature, framework, language, and file-list enrichment fields
> absent/empty …

**Implemented literally, this silently empties three shipped CLI commands.** Upstream impact
analysis of `detect_repo_structure` / `detect_repo_structure_or_root_package`:

| CLI command | Entry point | Field it reads | Effect of literal R5.6 |
|---|---|---|---|
| `sniff repo test-runner` | `detect_repo_structure` (`cli/src/commands/repo.rs:537`) | `Package.test_runners` | **empty output** |
| `sniff repo dependencies` | `detect_repo_structure_or_root_package` (`repo.rs:302`) | `Package.dependencies` via `collect_external_dependencies` | **empty output** |
| `sniff repo package-manager` | `detect_repo_structure_or_root_package` (`repo.rs:370`) | `Package.package_managers` | **empty output** |

`package_managers` is not in the plan's own retained-field list, yet `sniff repo package-manager`
exists solely to report it. `sniff repo version` (`repo.rs:665`) survives, because `version` is
retained identity.

The plan's companion task — "direct callers to `RepoRequest::full()` for enriched fields" — is not a
viable resolution here: routing these three commands through `full()` makes them 10–50× slower, a
direct performance regression introduced by a performance feature.

### Why the cost model does not support the split as written

The fields R5.6 would drop are not equally expensive. Once a manifest is parsed for identity
(`name`, `version` — both retained), these come free from the **same parsed value**:

- `dependencies` / `dev_dependencies` / … — `cargo_dependencies_from_value(parsed, …)`
- `features` — `cargo_features_from_value(parsed)`
- `package_managers`, `ecosystem` — marker probes only

The genuinely expensive per-package work in structure mode is:

- **test-runner detection** — `read_dir` × ~4 per package plus a config-glob probe storm; the
  dominant term in `repo_structure_huge_375_packages` (13,274 probes / 702 `read_dirs` / 300
  packages), and the one Phase 3 explicitly handed to Phase 4.
- **language/file enrichment** — already skipped in structure mode today.

So the productive line is not "identity vs. everything else"; it is "what the already-parsed
manifest yields" vs. "what costs another observation". Dropping dependencies and package managers
saves ~nothing and breaks two commands; dropping test-runner detection saves nearly all of it and
breaks one.

### Recommendation (awaiting owner decision)

1. Structure mode keeps everything derivable from the manifest it already parses for identity:
   `ecosystem`, `package_managers`, `features`, and the dependency lists. `sniff repo dependencies`
   and `sniff repo package-manager` keep working at structure cost.
2. Test-runner detection becomes an explicit, opt-in `RepoRequest` control rather than an implicit
   consequence of the detail level. `sniff repo test-runner` opts in and pays only the test-runner
   cost — not `full()`'s language scan. Every other structure-mode caller stops paying it.
3. Language, framework, and file-association enrichment remain `full()`-only, as today.

This delivers R5.6's actual objective — structure mode stops doing work its callers discard — with
no silent output regression, and it is strictly cheaper for `sniff repo test-runner` than the plan's
own suggested migration to `full()`.

Per the plan's execution rule ("stop for review before any HIGH or CRITICAL-risk edit"), R5.6 is
**not implemented in this phase** pending that decision. R5.1–R5.5, R5.7, R5.8, and R6 do not depend
on it and proceed.

## Counters

No new names. Phase 4 is asserted against existing counters:

| Counter | Phase 3 | Phase 4 target |
|---|---|---|
| `filesystem.repo.package_enrichments` (`repo_full_huge_375_packages`) | 600 | **300** — one per unique boundary |
| `filesystem.repo.package_enrichments` (`staged_filesystem_full_all_stages`) | 180 | **90** |
| `filesystem.repo.manifest_parses` | 754 / 454 | falls by the root-manifest re-reads the store collapses |
| `filesystem.io.canonicalizations` | 600 / 300 | falls with the enrichment halving |

## Acceptance

Commands run from `sniff/`:

| Command | Requirement |
|---|---|
| `just test` | pass, modulo the Phase 1 pre-existing `detect_area` temp-dir timeout |
| `just lint` | pass |
| `just build` | pass |
| `just doctest` | pass |
| `cargo run -p sniff --release --example work_counts` | every delta against Phase 3 explained |

Equivalence: serialized `RepoInfo` must be byte-identical across the refactor for every fixture —
this phase changes work, not results.

## As built

### Work removed

`cargo run -p sniff --release --example work_counts`, same host and fixtures as Phases 1–3.

| Case | counter | Phase 3 | Phase 4 | delta |
|---|---|---:|---:|---:|
| `repo_full_huge_375_packages` | `package_enrichments` | 600 | **300** | −50% |
| | `filesystem.io.read_dirs` | 1401 | **701** | −50% |
| | `filesystem.io.metadata_probes` | 25975 | **13275** | −49% |
| | `filesystem.repo.manifest_parses` | 754 | **454** | −40% |
| | `filesystem.io.file_opens` | 1009 | **708** | −30% |
| | `filesystem.io.bytes_read` | 66348 | **50548** | −24% |
| `staged_filesystem_full_all_stages` | `package_enrichments` | 180 | **90** | −50% |
| | `filesystem.io.read_dirs` | 421 | **211** | −50% |
| | `filesystem.io.metadata_probes` | 7885 | **4075** | −48% |

**The drift bracket settles it.** `repo_structure_huge_375_packages` and
`staged_filesystem_summary_git_plus_repo` are **byte-identical** to Phase 3 on every counter (13274
probes / 702 `read_dirs` / 454 parses / 457 opens / 300 enrichments). Phase 4 does not touch
structure-only detection — R5.6 is blocked — so an unchanged control is exactly what should appear,
and it is what makes the full-mode deltas above attributable to the change rather than to the host.

After this phase `repo_full`'s probes and `read_dirs` land within **1** of `repo_structure`'s
(13275 vs 13274; 701 vs 702). Full detection now costs structure detection plus the inventory,
which is the shape R5 predicted once the second enrichment pass is gone.

`canonicalizations` is unchanged at 600 by design: producing the duplicate seed still costs one
normalization each. That is the whole point — a duplicate now costs one `canonicalize` instead of a
full enrichment.

### Results are unchanged

`just test`: **1378/1379**. The sole failure is
`filesystem::repo::area::tests::detect_area_errors_when_not_in_repo`, the Phase 1 pre-existing
temp-dir timeout verified on clean `HEAD`. `just lint`, `just build`, and `just doctest` are clean.

### Deviations from the design above

- **`ManifestStore` is a `LockStore`.** Only the lockfile half is built: one `Cargo.lock` parse per
  unique `owner_root`, replacing one parse per detector plus one per full-mode scan. The parsed
  Cargo/Node/Python manifest store is not built, so `cargo::read_toml_at` still re-reads the
  inherited root `Cargo.toml` once per member. The 375-package fixture does not use
  `version.workspace = true`, so this does not show in the table above; a fixture that does would
  make it visible, and is the natural first Phase 4 follow-up.

- **`owner_root` was not in the designed seed and had to be.** Enrichment is frame-sensitive in a way
  the design missed: Cargo `version.workspace = true`, npm's root-version fallback, and `Cargo.lock`
  resolution all resolve against the *owning workspace's* root, not the repo root. The pre-seed code
  got this right implicitly by calling `create_package` with the detector's root and only then
  calling `rebase_package_to_root`. Seeds must therefore carry the frame they were resolved in and
  enrich in it, re-framing only `relative` / `package_area`. Without `owner_root`, a nested Cargo
  workspace's members would silently resolve inherited versions against the outer repo's manifest.

- **`seed.evidence` is carried but not yet consumed.** Skipping `detect_package_ecosystem`'s probes
  on index evidence is only sound in the *positive* direction: the index omits generated and fixture
  manifests, so an empty evidence set does not mean "no manifest here". A fast path may skip probes
  only for kinds known present. Left unwired rather than shipped as a subtly wrong optimization.

### Not done

- **R5.6 structure-only migration** — blocked above, awaiting an owner decision.
- **R6.4 deepest-prefix ownership index** — the three existing implementations are unchanged and
  correct; unifying them is pure consolidation with no measured cost attached, and it was ranked
  below the enrichment defect.
- **100/500/2,000-package and symlink / non-UTF-8 / Windows-drive fixtures.**
- **README/rustdoc structure-only contract** — deferred with the migration it would document.

### Files

Primary: `repo/seed.rs` (new), `repo/detection.rs`, `repo/topology.rs`, `repo/manifest_index.rs`,
`repo/glob.rs`.

Detector conversion to seeds (mechanical): `repo/cargo.rs`, `repo/npm.rs`, `repo/uv.rs`,
`repo/nx_turbo.rs`, `repo/go.rs`, `repo/gradle.rs`, `repo/maven.rs`, `repo/dotnet.rs`,
`repo/polyglot.rs`, `repo/nested.rs`.

### Code deleted

`merge_packages`, `merge_package_into`, `merge_path_lists`, `dedupe_packages`, and
`rebase_package_to_root` are gone. All five existed to reconcile enriched duplicates after the fact;
with deduplication moved ahead of enrichment there is nothing left to reconcile. Their removal is
the structural evidence that the defect is closed rather than patched — the compiler now rejects the
shape that allowed it.

## Gate for Phase 5

- Phase 4's counter table above supersedes Phase 3's for full-mode cases.
- `repo_full` ≈ `repo_structure` + inventory. Any future full-mode counter materially above
  structure's is a re-introduced second pass.
- The blocked R5.6 decision also governs Phase 8's "structure/full semantics" documentation task.
