---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-skill/fixes/2026-10-06-planner-metadata-no-deps/spec.md
plan: fixes/2026-10-06-planner-metadata-no-deps/plan.md
implemented_by: claude/sonnet
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - scripts/ci/test_affected_scope.py
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - repo-deps
---

# Implementation Log for 2026-10-06-planner-metadata-no-deps (5 phases)

## Phase 1

- started Phase 1 (rulings and spike): baseline capture, environment checks, outside-package spike.
- Task 1.1 done: full `cargo metadata --locked` saved to `/tmp/pmnd-baseline/full-metadata.json` (74 members, 205 member edges, `Cargo.lock` unchanged). Golden plans (resolved + legacy) for `--all`, `README.md`, `claudine/lib/src/lib.rs`, `Cargo.lock`, `biscuit-speaks/lib/src/lib.rs` written to `/tmp/pmnd-baseline/golden/` by `/tmp/pmnd-baseline/run_plan.py` (patches `load_metadata`; fixed `--event pull_request`, base/head); two runs byte-identical. Helper `target/debug/ci-build` was already built.
- Task 1.2 done: `CARGO_HOME=<empty>` `cargo metadata --no-deps --offline` succeeds, `"resolve": null` present. Finding: `tomllib` is optional in `affected_scope.py` (macOS `/usr/bin/python3` is 3.9.6, `companion_suites.py` imports the module there), so the Phase 3 outside-package loader must use the existing guarded import and refuse by name. `tool_guard`/`BISCUIT_REQUIRE_CARGO=1` wiring confirmed at `_package-ci.yml` "Companion suites".
- Task 1.3 done (spike): the only reachable outside package, `schematic/schema`, requests no features on `schematic-define`/`schematic-definitions`, and `schematic-define` has `default = []`, so no outside-to-member feature feedback exists today. No `[patch]`/`[replace]`/`.cargo/config` replacement of a member found. Loader stays minimal but is still built (Ruling 2). Result recorded in the plan's "Spike result".
- No source, docs, or skill files changed in Phase 1 (planning/measurement only); no tests added, none needed. Rulings 1-6 adopted unchanged under `yolo`.

## Phase 2

- started Phase 2 (fixture migration and no-resolve guards, tests first). Only `scripts/ci/test_affected_scope.py` changed; no `affected_scope.py` change (that is Phase 3).
- Task 2.1: added `declare_dependencies` (puts member edges on package records in the `cargo metadata --no-deps` entry shape; `path` is the dependency's manifest directory; kinds `None`/`dev`/`build`), `assert_no_synthetic_resolve` (rejects a populated `resolve`, accepts `null`/absent), and `ResolveForbiddenMetadata` (a dict wrapper that raises on `["resolve"]`, `.get("resolve")`, `in`, iteration, `items`, `keys`, `copy`). The shared `package()` helper now emits `dependencies: []` and `features: {}`, the shape every real `--no-deps` record carries, so `rust_package` and every hand-built fixture inherits it.
- Tasks 2.2/2.3: a one-off script (kept in `/tmp/pmnd-p2/`, not in the repo) rewrote all 27 synthetic `"resolve": {...}` literals to `"resolve": None`; the 7 fixtures that carried real edges (including the `dev` kinds in the closure fixture) now call `declare_dependencies` with the same edges and kinds, so each test keeps its expected dependents, native union and closure. The three `self.metadata["resolve"]["nodes"]...` mutations (gated, shared, reader/other) were replaced (two deleted as no-ops for a null resolve, one became `declare_dependencies(self.metadata, {"reader": ["other"]})`). `grep -rn '"resolve"' scripts/ci/test_*.py` now finds only `None` and the helper code.
- Task 2.4: no other suite feeds a synthetic resolve to the planner. `test_inputs.targets_from_metadata` reads only package records; asserted by `NoResolveFixtureTests.test_input_directories_need_only_the_package_records`.
- New tests (`NoResolveFixtureTests`): resolve-shape acceptance/rejection, record shape and dependency `path`, wrapper refusal of every read, `calculate_scope` through the wrapper finding a direct dependent from declared edges alone, and a dev edge both reporting a dependent and feeding the seed's native union.
- Checkpoint 2 result (macOS, local `python3`): `test_affected_scope` 407 tests, 217 not passing, **all** for the expected reason: 215 are `TypeError: 'NoneType' object is not subscriptable` at the two old resolve reads (214 at `reverse_dependency_map`, 1 at `build_closure`), 2 are `NoResolveFixtureTests` planning tests tripping the wrapper at `reverse_dependency_map`. No other failure. `test_resolved_plan`, `test_completion`, `test_cross_check`, `test_evidence_reuse`, `test_local_evidence`, `test_consolidation` pass; `test_ci_local` uses the real (still full) metadata and was not affected. These red tests are the Phase 3 target; the suite is intentionally not green at the end of this phase.
- Not run: `just test`/`just lint` (Rust gates, no Rust changed); `cross-check` (Python-only fixture change, no OS-dependent logic).

## Phase 3

- started Phase 3 (member graph implementation). Changed `scripts/ci/affected_scope.py` and, for the two `build_closure` callers whose signature changed, `scripts/ci/test_affected_scope.py`.
- Task 3.1: `load_metadata` now runs `cargo metadata --no-deps --offline --format-version 1` (UTF-8, `check=True`, no fallback); docstring says nothing may read `resolve`.
- Task 3.2: added `member_dependency_graph(metadata)` and a worklist solver (`_solve_member_graph`) over accumulating facts: enabled `(node, feature)`, active `(node, dependency key)`, active entries, and per-`(node, key)` feature requests. `k/g` and `k?/g` share the request set (requests apply to every active entry with that key, whenever it activates); only the non-weak form also activates `k` and enables a same-named optional feature. `dep:k`, plain features (metadata feature map only, nothing manufactured), `uses_default_features`, kind union over active entries and ignoring `target` follow spec section 2. Path identity: one `normcase(realpath(...))` per distinct path (cached); two members sharing a directory raise. Field validation is strict (absent/null/wrong-type `features`, `dependencies`, `optional`, `uses_default_features`, dependency `features`, `workspace_members`, `packages` raise `RuntimeError` naming package and entry; `kind` absent/null is normal; unsupported kinds raise; `resolve` is never read). Unknown member feature, unknown dependency key in a `k/g` item, and a feature naming nothing raise.
- Task 3.3: outside local path packages are read from their own `Cargo.toml` by `_MemberGraphBuilder` (one read per package, cached; `workspace = true` inherited from the nearest workspace root's `[workspace.dependencies]`; normal and build deps only; implicit optional-dependency features added the way Cargo does; cycles safe because the node is registered before its entries are built). They feed feature requests into the fixpoint but are never in the result, so no `a -> outside -> b` edge. The `tomllib` need is by name and lazy (`document()` raises only when an outside manifest or a replacement file must be parsed), so `import affected_scope` still works on Python 3.9 (verified with `/usr/bin/python3`). `[patch]`/`[replace]` in the root manifest or any `.cargo/config(.toml)` from the root upward or under `CARGO_HOME` raises when it names a member or points at a member directory (text probe first, so most hosts never parse). Departure from the spec text "do not parse manifests again": applies to members only, per plan Ruling 2. A fixture whose metadata lacks `workspace_root` skips the replacement guard (a real run always has it).
- Task 3.4: `reverse_dependency_map`, `direct_dependents`, `build_closure`, `native_closure`, `closure_directories` and `select_test_inputs` now take the member graph; `calculate_scope` computes it once and passes it to every caller. `--apply-to` is untouched. `lockfile_impacted_names` untouched. `ResolveForbiddenMetadata` did not need relaxing (`calculate_scope` only subscripts `packages`/`workspace_members`).
- Task 3.5: docstrings of `load_metadata`, `build_closure` (member graph; `biscuit-speaks` -> `playa` explanation kept) and the new functions reviewed in the same change; no other touched docstring described the resolve.
- Checkpoint 3 evidence (macOS, cargo 1.98.1, tree HEAD `5e5fb189d` + working changes): derived graph on the real workspace vs the saved full resolve `/tmp/pmnd-baseline/full-metadata.json` (`--locked` capture from Phase 1): 205 edges both sides, no missing, no extra, no kind difference (compared by package name, not id); `Cargo.lock` unchanged. Smoke check with a local fixture (`a` -> outside -> `b` with `features = ["extra"]`, `b.extra = ["dep:c"]`): graph `{a: {}, b: {c: normal}, c: {}}`, identical to `cargo metadata --offline`'s resolve.
- Tests: the 2 `build_closure` call sites were updated to build the graph first. `python3 -m unittest test_affected_scope` 407 tests OK (the 217 Phase-2 reds are green; includes the real-workspace `RealWorkspaceNativeGuardTests`, now through `--no-deps` + the graph); `python3 -m unittest discover` in `scripts/ci`: 1224 tests OK. Not yet added (Phase 4 scope): Cargo-comparison fixtures, per-rule fixtures, the robustness-matrix test, the no-network guard, outside-package tests, the playa native-requirement assertion.
- Not run: `just test`/`just lint` (no Rust changed); `cross-check` (path identity uses `normcase(realpath)`; Windows-spelling coverage is Phase 4 task 4.2).
