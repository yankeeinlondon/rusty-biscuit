---
kind: plan
name: planner-metadata-no-deps
total_phases: 5
created: 2026-10-07
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Plan: the planner reads workspace metadata without resolving dependencies

Spec: `2026-10-06-planner-metadata-no-deps`. All code lives in `scripts/ci/`
(`affected_scope.py`, `test_affected_scope.py`, `test_inputs.py`, `tool_guard.py`)
plus three docs pages.

## Summary of the work

`load_metadata` (`affected_scope.py:623`) runs a full `cargo metadata`, which
downloads the registry. Only two readers need the resolve:
`reverse_dependency_map` (`:1940`) and `build_closure` (`:1981`), and both use
only member-to-member edges. The work:

1. Switch `load_metadata` to `cargo metadata --no-deps --offline --format-version 1`.
2. Add `member_dependency_graph(metadata)`, which derives the active member
   edges (with kind sets) from member manifests using Cargo's feature-activation
   rules, including the outside-local-package feature feedback chosen below.
3. Point `reverse_dependency_map` / `build_closure` at that graph; compute it
   once per `calculate_scope` (`:3456`) and thread it through every caller
   (`:1968`, `:3567`, `:4007`, `:4035`). `--apply-to` still reads neither
   metadata nor a graph.
4. Migrate ~35 test fixtures that express edges through a synthetic `resolve`
   (`test_affected_scope.py`, line sites listed by `grep -n '"resolve"'`; also
   search every other `scripts/ci` suite and fixture mutations such as lines
   7921/7966/8051).
5. Add Cargo-comparison, rule, no-network, and tool-availability tests.
6. Update three docs pages.

### Definition of success

- The derived graph equals the real workspace's full resolve, edge for edge and
  kind for kind (205 edges at spec time; never hard-coded).
- The resolved plan is byte-identical old vs. new for the five invocations in
  spec acceptance criterion 2, on the same input tree.
- `load_metadata` succeeds with an empty `CARGO_HOME`; no planning path reads
  `resolve`; registry/git caches stay empty; `Cargo.lock` unchanged.
- All `scripts/ci` unittest suites, `just test repo-deps`, `just test test-toolkit`
  and `just lint` pass; no new CI job, cell, or trigger.
- Windows timing (criterion 5) is reported separately as pending hosted
  evidence unless a Windows run already exists.

## Phase 1: Rulings and spike

### Necessary Rules

These resolve the spec's open questions and gaps. Under `yolo: true` the
recommended option is adopted; the author may overturn any before Phase 3.

1. **Real-workspace comparison placement** — adopt the spec's recommendation
   (option 2): small all-local Cargo comparison fixtures run in the existing
   Ubuntu companion suite (`--offline`, empty temp `CARGO_HOME`, no compile);
   the real-workspace comparison is a documented local check run at
   implementation time and on manifest or toolchain changes. No CI cell, job,
   trigger, or committed snapshot. Docs state the reduced recurring coverage.
2. **Outside-workspace local packages** — adopt option 2: propagate feature
   requests through reachable local path packages (e.g. `schematic/schema`),
   parsing each reachable manifest once, but return only direct
   member-to-member edges. Non-members never appear in results, native unions
   or input dirs. `[patch]`/`[replace]`/config overrides that could map to a
   member raise a clear error. **Dependency:** this needs a manifest reader,
   which conflicts with the spec's "do not parse manifests again". Ruling: that
   sentence applies to *member* manifests (use metadata only); outside local
   packages are the sole exception, loaded with `tomllib` and a minimal
   workspace-inheritance resolver, fed by the Phase 1 spike's findings. If the
   spike shows no outside package can feed a member feature today, still
   implement the loader (spec says future-proof), but keep it minimal.
3. **Spike closure:** the spec's measurements already establish the
   performance benefit; no timing spike, no multi-host measurement. If a wider
   measurement seems needed, it is an author decision, not scheduled here.
4. **Dev-dependency of non-members is ignored** when propagating through
   outside packages (spec option 2 con list).
5. **Python floor:** `tomllib` requires 3.11. Confirm the repo's CI Python
   version before use; if lower, flag to the author rather than adding a
   dependency (see Task 1.2).
6. **Windows evidence is out of the plan's critical path:** criterion 5 is
   reported as "pending hosted run" unless PR #117 / next `main` push exists.

### Tasks

- [x] **1.1 Baseline capture** (Wave 1)
    - Save one full `cargo metadata --format-version 1` JSON of the current tree
      to a scratch directory outside the repo (needs a warm registry; run once).
    - Record: pinned toolchain, tree SHA, member count, edge count, and the
      canonical resolved plans for `--all`, `-- README.md`,
      `-- claudine/lib/src/lib.rs`, `-- Cargo.lock`,
      `-- biscuit-speaks/lib/src/lib.rs` using the **old** planner. These are the
      criterion-2 golden outputs; hold root, policy, event, date, base/head,
      evidence, constraints and helper constant fixed.
- [x] **1.2 Environment checks** (Wave 1, parallel with 1.1)
    - Confirm the Python version the companion suite runs (`_package-ci.yml`
      `Companion suites` step) supports `tomllib`; confirm `tool_guard.require_tools`
      semantics and the `BISCUIT_REQUIRE_CARGO=1` wiring.
    - Run `cargo metadata --no-deps --offline` with an empty `CARGO_HOME` on
      this host to confirm `"resolve": null` and success (precondition for all
      later work).
- [x] **1.3 Spike: outside-package feature feedback** (Wave 1, single host, runs once)
    - Inspect `schematic/schema/Cargo.toml` (excluded) and any other
      non-member path packages reachable from members: do any request features
      on members that activate an optional member dependency? Record the answer
      as a note appended to this plan's "Spike result" below.
    - Output informs only the size of the Task 3.3 loader, not whether it exists.

### Spike result

Task 1.3 (2026-10-07). Non-member path packages reachable from members:
`schematic/schema` (excluded; depended on by `homelab/*` integrations and
`model-citizen/lib`) is the only one found, plus `schematic/gen/schematic/schema`
which is a generated copy no manifest references by path. `schematic-schema`
depends on members `schematic-define` and `schematic-definitions` with **no
`features` and no `default-features = false`**, and `schematic-define`'s
`default = []`, so today **no outside package activates an optional member
dependency**: feedback is possible in principle but absent. `[patch]`/`[replace]`
appear in no `Cargo.toml`; `~/.cargo/config.toml` holds only `rustc-wrapper`
and the repo has no `.cargo/`. Consequence for Task 3.3: the loader stays
minimal (one `tomllib` read per reachable outside manifest, normal and build
dependencies only, `workspace = true` inheritance for those entries), but
exists per Ruling 2.

Task 1.2 (2026-10-07). `tomllib` is **optional** in `affected_scope.py`
(`tomllib = None` on Python < 3.11): `companion_suites.py` imports the module
on all three native environments where `/usr/bin/python3` is 3.9 (macOS ships
3.9.6 here). The planner itself runs only on the scope job's `ubuntu-latest`
(3.12) and developer hosts. Task 3.3 must therefore import lazily / reuse the
module's `tomllib` guard and refuse by name when the loader is needed and
`tomllib` is absent; it must not add a top-level hard import or a second TOML
reader. `tool_guard.require_tools` raises `AssertionError` when
`BISCUIT_REQUIRE_<TOOL>` is set, else `SkipTest`; `_package-ci.yml`'s
`Companion suites` step sets `BISCUIT_REQUIRE_CARGO=1`.
`CARGO_HOME=<empty> cargo metadata --no-deps --offline --format-version 1`
succeeds on this host (cargo 1.98.1) with `"resolve": null` (key present),
74 members, and leaves the empty `CARGO_HOME` empty.

Task 1.1 baseline (scratch `/tmp/pmnd-baseline/`, outside the repo; see its
`BASELINE.md`): toolchain 1.98.1, HEAD `c95bbbc0d` (tree `f1f95fa7`), 74
members, **205** member-to-member edges in the full resolve (never hard-code).
Golden plans for all five criterion-2 invocations (resolved and legacy forms,
10 files, byte-identical across two runs) produced by `run_plan.py`, which
patches `load_metadata` to return the saved full-resolve JSON and fixes
`--event pull_request --base 0*40 --head 1*40`.

### Checkpoint 1

- [x] Golden plans and baseline metadata saved; Python/`tomllib` question
      answered; spike note recorded; rulings reviewed by the author or accepted
      by `yolo`.

## Phase 2: Fixture migration and no-resolve guards (tests first)

Goal: make the suite express edges the way `--no-deps` does, *before* changing
the code, so the failing tests define the new behavior.

- [ ] **2.1 Shared fixture shape** (Wave 1)
    - Introduce one shared helper (in `test_affected_scope.py`) that builds
      package records with explicit `dependencies: []` and `features: {}`, and a
      shape assertion rejecting a populated synthetic `resolve` while accepting
      `null`/absent.
    - Add a `resolve`-rejecting mapping wrapper (raises on `[]`, `.get`, `in`)
      and run representative planning inputs through it to prove no read.
- [ ] **2.2 Migrate edge-bearing fixtures** (Wave 2; split by line range across
      up to three subagents, one file region each — same file, so assign
      disjoint line ranges and merge sequentially)
    - Convert every fixture with populated `resolve.nodes[].deps` (sites near
      lines 206, 675, 757, 1626, 1959, 2015, 2065, 2143, 2207, 2946, 3041, 3964,
      8051 and any found by search) to `dependencies` on package records
      (`name`, `path`, `kind`, `optional`, `uses_default_features`, `features`,
      `rename`).
    - Each migrated test keeps its expected dependents / native requirements /
      input directories. Deleting edges and asserting only plan success is not
      acceptable.
- [ ] **2.3 Migrate empty-resolve fixtures** (Wave 2, parallel with 2.2 on other
      line ranges)
    - Replace `"resolve": {"nodes": [... "deps": []]}` literals and the
      `.append`/assign mutations (7921, 7966) with null/absent `resolve`.
- [ ] **2.4 Other suites** (Wave 2)
    - `grep -rn 'resolve' scripts/ci/test_*.py` (excluding `test_schema.py`
      unrelated hits); update any fixture that feeds metadata to the planner.
      `test_inputs.py`'s `targets_from_metadata` reads only members/packages and
      should need no change; assert that.

### Checkpoint 2

- [ ] `python3 -m unittest discover` in `scripts/ci`: migrated fixtures fail
      only where the old code reads `resolve` (expected), and nothing else
      regresses. Fixture-shape and no-resolve assertions exist.

## Phase 3: Member graph implementation

- [ ] **3.1 `load_metadata` flags** (Wave 1)
    - Command becomes `cargo metadata --no-deps --offline --format-version 1`;
      keep `encoding="utf-8"`, `check=True`, error propagation; no fallback, no
      `cargo fetch`, no lockfile writes.
    - Update its comment/docstring; tolerate null or absent `resolve`.
- [ ] **3.2 `member_dependency_graph` core** (Wave 1, parallel with 3.1)
    - Return `dict[package_id, dict[target_id, set[kind]]]`; every member has a
      key (empty map if no edges).
    - Implement spec §2 rules 1–6 as a fixpoint over accumulating sets of
      (member, feature) and (member, dependency key): `dep:k`, `k/g`, `k?/g`
      (weak requests retained until `k` becomes active), plain feature names
      from the metadata feature map only, no manufactured implicit features,
      `uses_default_features`, kind union over *active* entries, `target`
      ignored.
    - Path matching: normalize entry `path` and member manifest directories
      with one host-native routine (realpath/normcase semantics: no lowercasing
      on Unix, symlink-aware for macOS `/var`, Windows separators/case),
      computed once per distinct path; never match by name. Ambiguous matches,
      unknown member feature requests, and unsupported kinds raise errors naming
      package and entry. Member without `default` is valid.
- [ ] **3.3 Outside local package loader** (Wave 2; depends on 3.2; per Ruling 2)
    - For path dependencies outside the member set, load each reachable
      manifest once (`tomllib`), resolve `workspace = true` inheritance for
      those entries only, ignore non-member dev dependencies, handle cycles,
      and feed their feature requests on members into the fixpoint. Never emit
      non-member nodes or a bridged `a → b` edge.
    - Detect `[patch]`/`[replace]`/`.cargo/config` replacements that could map
      to a member and raise a clear error.
- [ ] **3.4 Rewire readers** (Wave 3; depends on 3.2/3.3)
    - `reverse_dependency_map(graph, packages)` and `build_closure(id, graph,
      packages)` consume the graph; dedupe, keep sorted output, keep semantics
      (direct dependents only; dev-deps of dependencies not propagated; targets
      not evaluated).
    - `calculate_scope` computes the graph once and passes it through all
      callers (direct dependents, native requirements, build-input dirs,
      unchanged dependents in check cells, test-input-selected packages).
      `--apply-to` stays metadata- and graph-free.
    - Update docstrings and failure messages; keep the `biscuit-speaks` →
      `playa` explanation. Leave `lockfile_impacted_names` untouched and
      uncalled.
- [ ] **3.5 Comment pass** (Wave 3)
    - Per repo rules, review `///`-equivalent docstrings and inline comments of
      every touched function; fix drift in the same change.

### Input Robustness Matrix

The graph builder is a reader of `cargo metadata` JSON (single format). Each
load-bearing field has a defined outcome; one test walks the matrix from a
real-tool fixture (a path-only workspace's `--no-deps` output) with one edit
per cell, plus an unedited control row proving the positive result.

| Field (load-bearing) | absent | explicit null | wrong type (whole) | wrong type (one element) | empty | duplicate key / entry |
|---|---|---|---|---|---|---|
| `packages[].features` | error naming package (never `{}`) | error | error | error (non-string item) | `{}` valid: no features, default absent | JSON duplicates: last-wins is Python's; assert via invalid-doc test only if parser strictness added (see note) |
| `packages[].dependencies` | error | error | error | error naming entry | `[]` valid: no edges | duplicate entries merge kinds (valid) |
| dep `path` | external/registry entry: no edge (valid) | same as absent | error | n/a | `""` error | two entries same path: kinds unioned |
| dep `kind` | normal (spec: `null` means normal) | normal | error | n/a | n/a | union |
| dep `optional`, `uses_default_features` | error (never defaulted) | error | error | n/a | n/a | n/a |
| dep `features` / `rename` | `features` error; `rename` absent=name | `rename: null`=name | error | error | `[]` valid | n/a |
| `workspace_members` | error | error | error | error | `[]` valid: empty graph | n/a |
| `resolve` | ignored | ignored | ignored | n/a | n/a | n/a |
| trailing/invalid content | `json.loads` raises; no fallback | | | | | |

Note: `kind: null` is *defined* as normal by the Cargo format, and
`resolve: null` is expected; those are the only null cells that are valid.
The matrix test asserts outcomes through `calculate_scope`/the public plan, not
the parser's return. Before declaring done, grep the new code for
`.get(...) or`, `or []`, `or {}`, `filter_map`, `unwrap_or_default`-style
defaults on these fields (Python equivalents: `.get(k, [])`, `or []`,
`isinstance` filters that drop elements silently).

### Checkpoint 3

- [ ] `python3 -m unittest discover` in `scripts/ci` passes with migrated
      fixtures.
- [ ] Graph on the real workspace matches the saved full-resolve baseline from
      Task 1.1 (named edges and kind sets), run locally with `--locked`.

## Phase 4: New tests and acceptance verification

- [ ] **4.1 Cargo-comparison fixtures** (Wave 1)
    - Small all-local temp workspaces; compare derived graph vs a full
      `cargo metadata --offline` resolve with an empty temp `CARGO_HOME`, no
      compile. Use `tool_guard.require_tools`; correct its description to name
      `_package-ci.yml`'s `Companion suites` step. A skip never counts as
      acceptance evidence.
- [ ] **4.2 Rule fixtures** (Wave 1, parallel with 4.1; may split across two
      subagents by rule group)
    - One case per rule from spec §4's list (default features; `dep:`, `k/g`,
      weak `k?/g` active and inactive; implicit feature; rename; member
      requesting features on member; default despite incoming
      `default-features=false`; `k/g` vs `dep:k` same-named feature; implicit
      feature suppression; unknown feature error; weak request activated in a
      later pass; dev + inactive-optional-normal; multi-declaration kinds and
      target-specific entries; dependency and feature cycles; inherited
      workspace dep; non-member package record; registry dep named like a
      member; outside local package feature feedback; host-native path
      identity incl. Windows spelling in the existing Windows suite).
      Expected values hand-written, with the subtle activation cases cross-checked
      against path-only Cargo comparisons.
- [ ] **4.3 Robustness matrix test** (Wave 1, parallel) — as tabulated in Phase 3.
- [ ] **4.4 Native guard + no-network guard** (Wave 1, parallel)
    - Extend `RealWorkspaceNativeGuardTests` to assert `playa`'s system-library
      requirements via `biscuit-speaks`.
    - Capture the `load_metadata` subprocess call: assert both flags, UTF-8, no
      fallback on Cargo failure. One real run against the shipped workspace with
      an empty temp `CARGO_HOME` asserting null resolve, empty registry/git
      caches, unchanged `Cargo.lock`.
- [ ] **4.5 Acceptance runs** (Wave 2; after all above)
    - Criterion 2: with the saved metadata, compare canonical serialized plans
      and legacy projections for the five invocations against Task 1.1 goldens;
      also kinds, test-input-only selection, and unchanged-dependent native
      requirements. Report mismatches by field.
    - Criterion 3: a complete local planner run with empty child-only
      `CARGO_HOME`, `RUSTUP_HOME` retained, prebuilt helper via the existing
      override; clean up temp dir.
    - Record the real-workspace comparison command, toolchain, tree, and any
      differences (acceptance 1).
    - Run `just test repo-deps`, `just test test-toolkit`, `just lint`, and
      `python3 -m unittest discover` in `scripts/ci`; review `just ci-local --plan`.

### Checkpoint 4

- [ ] All of the above green with no skips among the new Cargo-dependent tests
      on this host; acceptance criteria 1–4 evidenced.

## Phase 5: Documentation and closure

- [ ] **5.1 Docs** (Wave 1; three files in parallel)
    - `.github/ci/README.md`: scope job reads `cargo metadata --no-deps
      --offline` and derives member edges itself; registry-free metadata and
      graph.
    - `docs/topics/ci-cd.md`: same statement where scope reading is described;
      document default-feature all-platform view, pinned toolchain + prebuilt
      helper still required, the outside-local-package boundary, the
      unsupported replacement error, and the real-workspace comparison workflow
      with its reduced recurring coverage. Developer-with-no-context audience;
      compact example per rule; a Mermaid diagram of the activation flow.
    - `.claude/skills/rust-devops/ci-cd.md`: one line that the planner metadata
      read is registry-free and a new resolve-needing input needs its own
      decision.
    - None of these link to or name this fix.
- [ ] **5.2 Drift check** (Wave 2) — grep docs, README, and skills for
      `cargo metadata` descriptions that still imply a resolve; fix.
- [ ] **5.3 Report** (Wave 2)
    - Implementation log in the fix directory: departures from spec (notably
      the outside-package manifest-reader ruling), the Windows criterion 5
      status as *pending hosted evidence* (or the run IDs if available).
    - Terminal state: "implementation complete, ready for review". Do not move
      the fix to `_completed`; do not commit unless asked.

### Checkpoint 5

- [ ] Docs updated, no spec links in docs, implementation log written, spec
      frontmatter `implemented` left for the author's review flow.
