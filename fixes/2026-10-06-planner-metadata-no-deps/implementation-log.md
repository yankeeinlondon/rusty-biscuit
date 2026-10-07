---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-skill/fixes/2026-10-06-planner-metadata-no-deps/spec.md
plan: fixes/2026-10-06-planner-metadata-no-deps/plan.md
implemented_by: claude/sonnet
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-10-06-planner-metadata-no-deps (5 phases)

## Phase 1

- started Phase 1 (rulings and spike): baseline capture, environment checks, outside-package spike.
- Task 1.1 done: full `cargo metadata --locked` saved to `/tmp/pmnd-baseline/full-metadata.json` (74 members, 205 member edges, `Cargo.lock` unchanged). Golden plans (resolved + legacy) for `--all`, `README.md`, `claudine/lib/src/lib.rs`, `Cargo.lock`, `biscuit-speaks/lib/src/lib.rs` written to `/tmp/pmnd-baseline/golden/` by `/tmp/pmnd-baseline/run_plan.py` (patches `load_metadata`; fixed `--event pull_request`, base/head); two runs byte-identical. Helper `target/debug/ci-build` was already built.
- Task 1.2 done: `CARGO_HOME=<empty>` `cargo metadata --no-deps --offline` succeeds, `"resolve": null` present. Finding: `tomllib` is optional in `affected_scope.py` (macOS `/usr/bin/python3` is 3.9.6, `companion_suites.py` imports the module there), so the Phase 3 outside-package loader must use the existing guarded import and refuse by name. `tool_guard`/`BISCUIT_REQUIRE_CARGO=1` wiring confirmed at `_package-ci.yml` "Companion suites".
- Task 1.3 done (spike): the only reachable outside package, `schematic/schema`, requests no features on `schematic-define`/`schematic-definitions`, and `schematic-define` has `default = []`, so no outside-to-member feature feedback exists today. No `[patch]`/`[replace]`/`.cargo/config` replacement of a member found. Loader stays minimal but is still built (Ruling 2). Result recorded in the plan's "Spike result".
- No source, docs, or skill files changed in Phase 1 (planning/measurement only); no tests added, none needed. Rulings 1-6 adopted unchanged under `yolo`.
