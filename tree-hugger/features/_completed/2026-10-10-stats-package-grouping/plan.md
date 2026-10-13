---
kind: plan
feature: stats-package-grouping
spec: spec.md
total_phases: 5
created: 2026-10-05
phase: 5
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/env/tests.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/tests/l1/main.rs
  - sniff/lib/tests/l1/package_identity.rs
  - sniff/lib/tests/l1/lockfile_provenance.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_1:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/os/macos.md
source_files_during_phase_2:
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
docs_updated_during_phase_2:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_3:
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
docs_updated_during_phase_3:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_3:
  - tree-hugger/docs/release-notes.md
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_4:
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
packages:
  - sniff
  - sniff-cli
  - claudine-cli
  - tree-hugger-cli
source_files_during_phase_5:
  - tree-hugger/cli/src/declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - claudine/cli/src/commands/wrap/env/tests.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/tests/l1/main.rs
  - sniff/lib/tests/l1/package_identity.rs
  - sniff/lib/tests/l1/lockfile_provenance.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/l1/cli.rs
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
documentation:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/os/macos.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/docs/release-notes.md
completed_phase: 5
implemented: true
---

# Plan: Package Grouping in `hug --stats`

## Summary and Definition of Done

`hug {functions,types,symbols,classes} --stats` gains per-package groups next to
the existing overall totals. Package identity and membership come from sniff's
bounded structure discovery; `hug` never walks manifests itself. The feature is
reporting-only: no new selector, extraction, cache-key, or schema change.

The work has four moving parts:

1. **sniff** (new to this feature's package scope): expose whether a package
   `name` came from a manifest, and expose a reusable, public, component-based
   ownership lookup (today `PackageOwnershipIndex` is `pub(crate)` in
   `sniff/lib/src/filesystem/repo/ownership.rs`, and `RepoInfo::package_for_dir`
   rebuilds it per call).
2. **tree-hugger-cli grouping core**: capture one structure-only catalog per
   stats request, resolve each selected file's owner once, and feed the existing
   `Stats` accumulator (`cli/src/stats.rs`) per group while consuming results in
   `write_tally` (`cli/src/declarations.rs`).
3. **Reports**: terminal sections and a JSON `package_root_dir` + `groups`
   envelope addition, with `report_schema_version` kept at `1`.
4. **Verification and docs**: acceptance matrix from the spec, plus README,
   `docs/symbol-reports.md`, skill, dependency docs, and release notes.

Success looks like:

- [x] Every row of the spec's acceptance table has declared, runnable coverage;
  the full macOS suite passes. Platform-specific tests remain in the existing
  Linux/Windows/WSL2 CI schedule, with no new matrix cells.
  - The originally requested local Linux execution was not produced: no
    BUILD_* variables are set, and cross-check creates a synthetic commit
    contrary to this session's no-stage/no-commit instruction. This is a
    recorded validation exception, not Linux/Windows/WSL2 passing evidence.
- [x] For every fixture, the sum of group `by_kind`/`by_visibility`/`by_language`,
  `documented`/`total`/`unavailable`, and `synthetic_exports` equals the overall
  fields; overall `ratio` is computed from summed counts (an unequal-size case
  proves no averaging).
- [x] Non-stats modes (listing, `--count`, JSONL, `--json-full`) request no
  package catalog (asserted by a counter or call-site test, not by timing).
- [x] A package rename on a warm cache changes the label while source files stay
  actual cache hits.
- [x] `just test` and `just lint` pass in `tree-hugger` and `sniff`; docs updated
  in the same change; implementation log records any departures from the spec.
  - Tree-hugger gates passed in Phase 5; sniff gates passed in Phase 1.
    The Phase 5 prompt limits reruns to tree-hugger, and no sniff file changed
    in this phase. Retained sniff evidence is not a new Phase 5 sniff run.

## Phase 1: Rulings, Spike, and sniff API

### Necessary Rules

The author must confirm (or overrule) these before Phase 2; Phase 1 tasks assume
the recommended answer and are cheap to redo if overruled.

1. **Grouping is automatic** (spec Open Question 1, recommended). Ordinary
   `--stats` always groups; no `--group-by` flag. Planned on this basis.
2. **sniff exposes explicit name-source metadata** (spec Open Question 2,
   recommended). Consequence: `sniff` joins this feature's declared package
   scope (`packages:` frontmatter currently lists only `tree-hugger-cli`); the
   spec frontmatter must be updated by the author when finalizing.
3. **Name-source shape.** Proposed: a `Package.name_source` field of a new
   `PackageNameSource { Manifest, PathFallback }` enum with `#[serde(default)]`
   defaulting to `PathFallback` so existing serialized catalogs stay readable and
   existing sniff output keeps its meaning. `hug` treats only `Manifest` as a
   usable identity; `PathFallback` goes to Unassigned.
4. **Ownership lookup shape.** Proposed: make a minimal public wrapper over the
   existing `PackageOwnershipIndex` (build once from a `RepoInfo`, look up an
   absolute native path, return the owning `&Package`) rather than a new index.
   It must keep the component-based, deepest-prefix semantics already in
   `ownership.rs`.
5. **Unnamed nested package.** Ownership is decided before name usability: a
   file in an unnamed nested package goes to Unassigned even when a named parent
   exists (spec "Match boundaries before checking the owner's name").
6. **Root-package path label.** `package.path` is `.` for a root package; the
   terminal section label for it is `name (.)`.
7. **Discovery-root resolution.** Active worktree root for a Git checkout
   (including from inside a package and from a linked worktree), the existing
   package scan root otherwise. Needs an existing helper; the spike confirms
   which (prefer `sniff` repo-root detection already used elsewhere; do not add
   a git2 dependency to `hug` unless sniff cannot supply it).
8. **Sniff feature set.** `hug` depends on sniff with `default-features = false`
   (no `network`/`remote`). If sniff's default build pulls platform crates
   (coreaudio, zbus, windows) unconditionally, that is acceptable as it is
   target-gated; the spike records the compile-time impact for the author.
9. **Wider measurement (author decides, not scheduled).** The spec says no
   performance spike is needed. If the author wants cold-start cost of sniff
   linking into `hug` or catalog time on the monorepo itself measured, that is
   an additional, separate decision.
10. **Failure mapping.** Any `Err` returned by sniff discovery maps to exit `1`
    with an actionable stderr message before any stats envelope is written.
    `Ok(None)` (no packages) is success and yields Unassigned.

### Spike (runs once, before the tasks it informs)

- [x] **S1: structure API coverage.** On this host only, run
  `detect_repo_structure_or_root_package` against: a Cargo workspace, a virtual
  workspace, a standalone Cargo package, a standalone `package.json` package, a
  manifest-less directory, and an unnamed nested package. Record, per case:
  whether `packages` is populated, what `Package.name`/`relative`/`path` are, and
  whether `PathFallback` names occur. Output feeds Tasks 1.1 and 1.2 and the
  rulings above. Capture results in the implementation log; no code lands from
  the spike.

### Wave 1 (sequential after S1; Tasks 1.1 and 1.2 touch different files)

- [x] **Task 1.1: Name-source metadata** (`sniff/lib/src/filesystem/repo/types.rs`, `detection.rs`)
  - add `PackageNameSource` and `Package.name_source`; have `resolve_package_name`
    report whether it returned a manifest name or the relative-path fallback,
    keeping the single manifest reader
  - update every `Package` construction site; confirm existing sniff output and
    snapshots are unchanged except for the new field (serde default, skip when
    default if that keeps existing JSON byte-stable; decide and document)
  - land the **Input Robustness Matrix** tests below in sniff

- [x] **Task 1.2: Public ownership lookup** (`sniff/lib/src/filesystem/repo/ownership.rs`, `types.rs`, `lib.rs`)
  - expose a reusable lookup built once per catalog; native-path in, `&Package`
    out; deepest component-prefix wins; `pkg-a` never claims `pkg-a2`
  - reuse `PackageOwnershipIndex`; do not fork the semantics. `package_for_dir`
    keeps its behavior and may delegate to the new type
  - tests: nested owner, prefix-sharing siblings, root package, symlink alias,
    path outside the catalog root returns `None`

### Input Robustness Matrix (load-bearing field: package `name`, per manifest format sniff reads)

The `name` decides whether a group exists at all (declared identity vs fallback),
so it is load-bearing. Defined outcome for every shape, asserted through the
public result (`Package.name_source`, then `hug`'s group presence), not the
parser's return value. Formats: Cargo.toml, package.json, pyproject.toml,
go.mod, and any other ecosystem manifest sniff's `resolve_package_name` reads.

| Shape | Example | Required outcome |
|---|---|---|
| control (unedited real-tool fixture) | `name = "utils"` | `Manifest`, group `utils` |
| absent | key omitted | `PathFallback` → Unassigned; never an empty name treated as declared |
| explicit null | JSON `"name": null`; YAML-style empty | `PathFallback`; not conflated with a declared empty string, not an error |
| wrong type, whole field | `"name": 123`, `name = 123` | `PathFallback`; never stringified into a name |
| wrong type, one element | n/a (scalar field); for formats with array/table nesting (e.g. `[package]` as an array) | `PathFallback`; never partially read |
| wrong type, every element | n/a as above | `PathFallback` |
| empty | `""`, whitespace only | `PathFallback`; not a name |
| duplicate key | `name` twice | follows the format: TOML rejects (manifest unparsable → `PathFallback`); JSON must not silently pick one if sniff's parser can detect it, else document last-wins as sniff's existing behavior |
| trailing or invalid content | valid manifest plus garbage | unparsable → `PathFallback`; never accepted |

Rules for this matrix: one test per format walks every row from a real-tool
fixture with one edit per cell; the control row proves each edit is load-bearing.
Before declaring it done, grep the touched reader for `#[serde(default)]` on
`name`, `Option<String>` where absent and null must differ, `filter_map(.. as_str())`,
`unwrap_or_default()`, and `.ok()` on the name parse. Any finding about one cell
requires listing the state of every other cell in that row and column.

### Per-format outcomes confirmed by Phase 1

`M` = Manifest; `F` = PathFallback; `E` = rejected catalog deserialization.
Manifest columns describe known member identity (Composer is standalone).
Malformed Cargo/npm/Python **root** manifests can instead return discovery
errors before a package exists; retain those errors. Go values are path tokens,
so bare `null` and `123` are strings, not typed null/numeric values.

| Shape | Cargo package.name | npm name | pyproject project.name | Go module | Composer name | JSON catalog name_source |
| --- | --- | --- | --- | --- | --- | --- |
| control | M | M | M | M | M | M |
| absent | F | F | F | F | F | F (legacy) |
| null | F (invalid TOML) | F | F (invalid TOML) | bare `null`: M | F | E |
| wrong field type | F | F | F | list/object syntax: F; bare `123`: M | F | E |
| one wrong element | F | F | F | F | F | E |
| all wrong elements | F | F | F | F | F | E |
| empty | F | F | F | F | F | E |
| duplicate | F | existing last-wins | F | F | existing last-wins | E |
| trailing/invalid | F | F | F | F | F | E |

Tests: `package_identity::{cargo_name_matrix,npm_name_matrix,python_name_matrix,
go_name_matrix,composer_name_matrix,catalog_round_trip}` walk these columns;
`malformed_root_discovery_is_an_error` separately preserves returned failures.

### Phase 1 validation checkpoint

- [x] `just test` and `just lint` pass in `sniff/`
- [x] Spike notes and provisional answers to the Necessary Rules are in the
  implementation log; spec frontmatter `packages:` includes sniff. Author
  confirmation before Phase 2 is recorded in `human_review_items` because the
  Phase 1 session is non-interactive.

## Phase 2: Grouping Core in tree-hugger-cli

Depends on Phase 1. New module `tree-hugger/cli/src/grouping.rs` (name may vary).

### Wave 2 (Tasks 2.1 and 2.2 are independent; 2.3 follows both)

- [x] **Task 2.1: Catalog capture**
  - add `sniff` to `tree-hugger/cli/Cargo.toml` with `default-features = false`;
    check the dependency graph for a cycle (sniff must not depend on tree-hugger)
  - resolve the discovery root (Rulings 7), call the structure-only API once per
    stats request, and build the ownership lookup once
  - returned `Err` → exit `1`, actionable stderr, no envelope; `Ok(None)` →
    empty catalog, all files Unassigned
  - only invoked when the report format is `Stats`; no catalog for other modes
  - do not canonicalize with lossy strings; use sniff's/biscuit-file's
    established helpers (see the `os` skill before touching path code)

- [x] **Task 2.2: Group accumulator**
  - a map keyed by package **directory identity** (native path), holding
    `{ name, portable relative path, Stats }`, plus an optional Unassigned
    `Stats`; overall `Stats` stays separate
  - a group is created for each eligible selected file before declaration
    filtering, including files whose analysis failed (zero group), but never for
    catalog packages with no selected files
  - ordering: portable path, then name; Unassigned last; independent of worker
    completion (`in_file_order` already serializes delivery; sort at emit)

- [x] **Task 2.3: Wire into `write_tally`**
  - resolve owner once per selected file (before the records loop), then
    `stats.add(record)` on both overall and that group's `Stats`
  - synthetic prelude exports: owner follows the export record's identity file
    (the prelude), using the native analysis path, not formatted locations or
    qualified names; no extra target resolution
  - `classes --stats` counts containers (unchanged); `--count` path untouched
  - empty selection (valid `--changed`) → overall zeros, `groups: []`; the
    ordinary `NoSourceFiles` error is unchanged
  - hold only counters and the catalog, not selected records

### Phase 2 validation checkpoint

- [x] unit tests in `grouping.rs` for ownership/ordering/zero groups; the
  existing `stats.rs` tests still pass unchanged
- [x] `just test` and `just lint` pass in `tree-hugger/`

## Phase 3: Terminal and JSON Reports

Depends on Phase 2.

### Wave 3 (Tasks 3.1 and 3.2 are independent: different writers in `stats.rs`)

- [x] **Task 3.1: JSON envelope**
  - add `package_root_dir` (portable absolute discovery root) before `stats`,
    `groups` before `errors`; `errors` stays last; trailing newline retained
  - group entry `{ "package": {"name","path"} | null, "stats": <same shape> }`;
    `path` is relative to the discovery root with `/`, `.` for a root package
  - emit `package_root_dir` whenever grouping is active, including empty and
    Unassigned-only reports; keep `report_schema_version: 1`

- [x] **Task 3.2: Terminal sections**
  - heading **All selected declarations**, then one section per group labeled
    `name (path)`, and an explicitly labeled Unassigned section
  - same category tables, coverage line, and conditional synthetic-export count
    in every section, including zero groups (`n/a` ratio)
  - use biscuit-terminal `TerminalRenderable` components (consult the
    `biscuit-terminal` skill); names and paths rendered literally so manifest
    text cannot inject markup, links, or control characters; plain output has no
    escapes or hyperlinks; existing width/styling behavior preserved

- [x] **Task 3.3: Option-conflict regression** (after 3.1/3.2)
  - confirm `--stats` still accepts `--json`, default, `--format plain|table` and
    rejects `--count`, `--jsonl`, `--json-full`, `--sort`, `--tree`, explicit
    `--signatures` (`cli/src/output.rs` tests)

### Phase 3 validation checkpoint

- [x] golden/snapshot updates are intentional, reviewed, and listed for release
  notes; `just test` and `just lint` pass in `tree-hugger/`

## Phase 4: Acceptance Test Matrix

Depends on Phase 3. Use the test toolkit and temporary local fixtures
(`tree-hugger/cli/tests/symbols/`, e.g. a new `grouping.rs` registered in
`main.rs`). Real terminal windows are unnecessary; any L2 harness used must not
steal focus. Prefer the real-tool fixture style (real `Cargo.toml`, `package.json`
files) over hand-rolled structs.

### Wave 4 (independent test files; run concurrently)

- [x] **Task 4.1: Membership and ownership cases**
  - Cargo workspace plus another ecosystem; standalone with and without Git;
    virtual workspace, manifest-less tree, unnamed nested package; root plus
    nested package; prefix-sibling names; library and CLI in one area;
    duplicate manifest names across packages

- [x] **Task 4.2: Roots, paths, and aliases**
  - invocation inside a package and inside a linked worktree (normal scan root
    and `root_dir` kept; discovery from active worktree root; `package_root_dir`
    and path framing correct); explicit file outside the discovery root →
    Unassigned with no second repository or inventory; symlink aliases, native
    Windows spellings, non-UTF-8 paths where supported (follow the `os` skill;
    gate platform-specific cases with `cfg`, not skips)

- [x] **Task 4.3: Counting semantics**
  - filters (file, name, scope, others) → group totals equal ordinary stats;
    zero-match and failed-analysis packages retained once with `null` ratio and
    one error entry; empty selections incl. valid `--changed` (only if that
    feature has landed; otherwise cover via an equivalent empty selection);
    unknown documentation; cross-package synthetic prelude export owned by the
    exporter; summed-equals-overall invariant including an unequal-group-size
    ratio case; `classes` counts containers

- [x] **Task 4.4: Output and safety**
  - terminal, plain, table, JSON: same totals and group order; plain has no
    escapes; names with spaces, non-ASCII, markup, control characters are
    literal/escaped and never lookup keys; returned discovery failure → exit `1`
    with actionable stderr and no success envelope, absence → Unassigned

- [x] **Task 4.5: Cache and cost**
  - warm cache, rename the manifest package → new label, unchanged totals,
    actual source cache hits (existing cache counters); non-stats modes request
    no catalog and no inventory/network enrichment (counter or call-site test)

### Phase 4 validation checkpoint

- [x] full `just test`, `just test-l2` (if any L2 files touched), and `just lint`
  in `tree-hugger/` and `sniff/`
  - This phase's prompt limits gates to tree-hugger: 1,938 L1 tests passed;
    lint and tier coverage passed. No sniff or L2 files changed this phase.
    Sniff and L2 gates were not run, as recorded in the implementation log.
- [x] confirm the new tests carry no hard-coded Unix paths; Windows/WSL2 are
  covered by the existing CI schedule, with no new matrix cells
  - Portability reviewed; native Windows/Linux-only cases await CI execution.
    Local evidence in this phase is macOS, not a claim of other-OS passes.

## Phase 5: Documentation and Close-out

Depends on Phase 4 (docs state behavior as built; no **planned** markers needed
unless something is deferred).

### Wave 5 (independent files; run concurrently)

- [x] **Task 5.1: tree-hugger docs**
  - CLI README and `docs/symbol-reports.md`: automatic grouping, overall vs group
    fields, discovery-root path framing, zero groups, Unassigned, synthetic-export
    ownership, failure behavior, the additive-envelope policy for
    `report_schema_version`; add a before/after terminal example and a Mermaid
    flow for file → owner → group
  - `.claude/skills/tree-hugger/SKILL.md` guidance
  - topic pages never link to or name this feature spec

- [x] **Task 5.2: sniff and dependency docs**
  - `sniff/docs` API docs and `.claude/skills` sniff guidance for name-source
    and the public ownership lookup
  - `tree-hugger/docs/dependencies.md` (and sniff's if affected) for the new
    sniff dependency and its feature set

- [x] **Task 5.3: Release notes and log**
  - release notes with before/after terminal output (terminal output changes
    intentionally; JSON overall `stats` unchanged)
  - implementation log: spike results, rulings as decided, departures from the
    spec; check `///`/`//!` comments in touched code for drift

### Phase 5 validation checkpoint

- [x] drift pass: README, topic pages, skills, and comments match the code
- [x] final `just test` / `just lint` in both areas; stop at "implementation
  complete, ready for review" (the author, not the agent, moves the spec to
  `_completed`)
  - Phase 5 tree-hugger: 1,938 L1 tests passed, lint passed, tier audit passed.
    Final-tree grouping check: 30 passed. No sniff rerun under this phase's
    explicit area limit; Phase 1 passing sniff gates are retained above.
