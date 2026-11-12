---
kind: log
feature: 2026-11-11-precompiled-shipped-schemas
implementation_1: "2026-11-11T18:32:51-08:00"
---

# Review/fix log: precompiled shipped schemas

## Implementation of Review Findings #1

> **started at:** 2026-11-11T18:32:51-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-11-precompiled-shipped-schemas/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Downstream freshness scheduling omits stored-digest preparation code | fixed | all children fixed/clean; sweep added 13 more preparation inputs (see Progress) |
| R1-F1.1 | Darkmatter freshness test + manifest: `simplified-schema/lib/src/validate.rs` change schedules an Ubuntu L1 cell selecting `shipped_artifact_is_fresh` | fixed | `affected_scope.py --resolved-plan --event pull_request simplified-schema/lib/src/validate.rs` → `darkmatter ubuntu-latest L1` filter includes `test(=shipped_artifacts::shipped_artifact_is_fresh)` (orchestrator re-ran); planner regression case added |
| R1-F1.2 | Claudine freshness test + manifest: same change schedules its freshness test | fixed | same plan → `claudine ubuntu-latest L1` filter `binary_id(claudine::l1) & test(=shipped_artifacts::shipped_artifact_is_fresh)`; planner regression case added |
| R1-F1.3 | Simplified Schema's own freshness test: owner suite runs on the same change | clean | same plan keeps simplified-schema unfiltered L1 on ubuntu + macOS (owner suite runs; it does not verify higher artifacts) |
| R1-F1.4 | Both downstream tests selected on `simplified-schema/lib/src/shipped/wire.rs` change | clean | `wire.rs` plan still selects both narrowed freshness tests; existing planner case passes |
| R1-F1.5 | Both downstream tests selected on `simplified-schema/schemas/schema.yaml` change | clean | `schema.yaml` plan still selects both; existing planner case passes |
| R1-F1.6 | Both downstream tests selected on Simplified Schema artifact-only change | clean | simplified-schema artifact-only plan still selects both |
| R1-F1.7 | Claudine freshness test selected on Darkmatter artifact-only change | clean | darkmatter artifact-only plan still selects Claudine's freshness test |
| R1-F2   | Root regeneration replaces artifacts before full validation | fixed | all children fixed/clean; generator emit mode accepts candidates before printing; coordinator publishes only accepted, mutually consistent candidates |
| R1-F2.1 | Root coordinator: Simplified Schema check refusal leaves all three files unchanged | fixed | emit runs `registry::check_artifact(&fresh)`, exit 1 with empty stdout on refusal; coordinator test `any_owner_s_refused_candidate_replaces_nothing` (simplified-schema case): all three files byte-identical, no `.tmp`; L1 `emitting_refused_bytes_prints_nothing` |
| R1-F2.2 | Root coordinator: Darkmatter check refusal leaves all three files unchanged | fixed | emit accepts via `Registry::load_over(fresh lower).check_all()` and requires `--lower simplified-schema=<candidate>` to equal its fresh lower; coordinator per-owner case + `a_lower_candidate_other_than_the_one_a_generator_built_over_replaces_nothing`; L1 accept/refuse/wrong-owner/lower-mismatch/usage tests |
| R1-F2.3 | Root coordinator: Claudine check refusal leaves all three files unchanged | fixed | same over both fresh lowers (`--lower simplified-schema=…`, `--lower darkmatter=…`); coordinator per-owner case; L1 mismatch case per lower owner |
| R1-F2.4 | Root coordinator control: all checks succeed → all three published | clean | `a_successful_run_replaces_every_artifact_and_a_repeat_is_a_no_op` passes; stand-in exits 2 unless given the expected `--lower` args; real `just shipped-schemas` exit 0, artifacts unchanged |
| R1-F2.5 | Simplified Schema example `write`: refusal preserves existing file | clean | `write()` calls `check_artifact(fresh)` before writing; existing L1 `a_failed_generation_leaves_the_artifact_in_place` asserts file unchanged and no `.tmp` |
| R1-F2.6 | Darkmatter example `write`: refusal preserves existing file | clean | same invariant and existing L1 test in Darkmatter's suite |
| R1-F2.7 | Claudine example `write`: refusal preserves existing file | clean | same invariant and existing L1 test in Claudine's suite |

### Progress

- starting the work on 'R1-F1: downstream freshness scheduling' at 18:33:11
        - defect class: a downstream generated-artifact freshness test does not declare every cross-package code path whose behavior changes the bytes it compares, so CI never schedules it when that code changes
        - delegated to one foreground subagent (serial orchestration); its log items are recorded below on return
        - discovered: coverage-driven sweep (`cargo llvm-cov run --example shipped_schemas` for Darkmatter and Claudine, generated bytes equal to checked-in artifacts) found the same omission in more files than the one the review named
        - completed: spelled and declared these additional preparation inputs (each in the owner's `shipped_artifact_is_fresh` and its `source-inputs`)
                - Darkmatter: `simplified-schema/lib/src/validate.rs`, `biscuit-hash/lib/src/{json,xx}.rs`, `resolve/recursive.rs`, `simplified/{source,yaml_scalar,query}.rs`, `numeric.rs`, `numeric/ingress.rs`, `format.rs`, `file_match.rs`, `claims.rs`, `origin.rs`, `example.rs`, `expression/{ast,lexer,parser}.rs`, `simplified/serialize.rs`
                - Claudine: `simplified-schema/lib/src/validate.rs`, `biscuit-hash/lib/src/{json,xx}.rs`, `darkmatter/lib/src/markdown/schemas/triggers/grammar.rs`, plus the simplified-schema language files above that its payload preparation executes
                - Simplified Schema: `biscuit-hash/lib/src/xx.rs` (header source digests)
        - checked and excluded (with reason): check-only (`prepared.rs`, `coerce.rs`, `scope.rs`, `slots*.rs`, `expression/scan.rs`, `narrowing.rs`, `signature.rs`, `subtype.rs`, `simplified/lint.rs`, triggers `lint.rs`); diagnostics-only (`positions.rs`, `span.rs`, `timings.rs`, `schema/layer.rs`); not executed for shipped text (`resolve/buffers.rs`, `resolve/select.rs`, `alphanumeric.rs`); covered via a spelled lower artifact (`shipped/registry.rs`; Darkmatter-only inputs for Claudine); not written into artifacts (biscuit-file path code)
        - completed: planner regression `test_shipped_schema_inputs_select_each_downstream_freshness_test` gained cases for `validate.rs` (Darkmatter + Claudine), `claims.rs` (Darkmatter), `biscuit-hash/lib/src/xx.rs` (all three), `triggers/grammar.rs` (Claudine)
        - completed: docs — `.claude/skills/darkmatter/SKILL.md` and `simplified-schema/docs/topics/shipped-artifacts.md` now state that the source-input rule covers preparation code, including stored-digest code, for both downstream crates
        - verification: `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests` 10 OK (518 s, Python planner suite); Darkmatter, Claudine, Simplified Schema `shipped_artifact_is_fresh` pass (<1 s); `just lint` exit 0 in all three areas
- work completed for 'R1-F1: downstream freshness scheduling' at 18:55:41
- starting the work on 'R1-F2: root regeneration publishes before validation' at 18:55:41
        - defect class: a multi-artifact publication step writes outputs before running the validation that can reject them, so a refusal leaves rejected content installed
        - delegated to one foreground subagent; its log items are recorded below on return
        - decision: full acceptance moved into each generator's emit mode (Rust), over the fresh lowers of the same generation via the existing `Registry::load_over`; the coordinator passes earlier candidates as `--lower <owner>=<path>` scratch files and a generator refuses if its own fresh lower differs (separate `cargo run -p` builds may resolve features differently)
                - alternatives: a separate `--check-candidate` mode (three more generator runs, same checks); emit-only validation without `--lower` (cross-candidate consistency unproven); Python hashing (ruled out by the review)
        - departure: the coordinator no longer runs `--check` after publication; a post-write check can only fail with rejected content installed and forced a rebuild per crate; each crate's `shipped_artifact_is_fresh` remains the standing check
        - sibling sweep: area `--write` paths and the area/root justfile recipes only delegate to validated writes or the coordinator — clean
        - tests: `a_failed_check_after_writing_is_reported` (which codified the defect) replaced by `any_owner_s_refused_candidate_replaces_nothing` and `a_lower_candidate_other_than_the_one_a_generator_built_over_replaces_nothing`; per-crate L1 emit/acceptance tests appended to each `shipped_artifacts` suite; mutation check (Darkmatter emit acceptance disabled) made both refusal tests fail
        - docs: coordinator docstring, example module docs, `simplified-schema/docs/topics/shipped-artifacts.md` (flowchart and steps), darkmatter skill sentence
        - verification: `cargo nextest run -p test-toolkit --test shipped_schemas_coordinator_contracts` 5/5 (slowest 0.43 s; test-toolkit has no `test` recipe); shipped-artifact filters pass in all three areas (100, 54, 27 tests; none ≥5 s); `just lint` exit 0 in all three; `just shipped-schemas` exit 0, "every shipped artifact is current"
        - note: planner test `test_every_declared_source_input_is_read_by_its_declarer` takes ~441 s on this host (Python suite, not L1; unchanged by this work)
- work completed for 'R1-F2: root regeneration publishes before validation' at 19:21:44
- starting orchestrator reconciliation and full area gates at 19:21:44
        - reconciled both subagent reports against the combined diff (15 files) and re-ran the planner for `validate.rs` and the coordinator contract tests independently
- work completed for orchestrator reconciliation and full area gates at 19:28:41

### Completion Report

- **outcome:** complete, ready for review
- **elapsed:** 2026-11-11T18:32:51-08:00 → 19:28:41 (about 55 minutes)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - child rows: 15 total; fixed 5 (R1-F1.1, R1-F1.2, R1-F2.1, R1-F2.2, R1-F2.3), clean 10 (R1-F1.3–R1-F1.7, R1-F2.4–R1-F2.7)
- **verified repairs**
        - R1-F1: `simplified-schema/lib/src/validate.rs` and 13 more preparation inputs found by a coverage-driven sweep are now spelled in each owner's `shipped_artifact_is_fresh` and declared in its `source-inputs`; the planner schedules the narrowed freshness cells; planner regression cases added
        - R1-F2: each generator's emit mode fully accepts its candidate over the fresh lowers of the same generation and refuses a mismatched `--lower` candidate before printing anything; the coordinator publishes only after all three are accepted; refusal from any owner leaves all three files unchanged
- **open findings:** none
- **cross-OS:** no notable OS risk (Python coordinator uses `tempfile`/`pathlib`; Rust changes are argument parsing and in-memory acceptance); no `just cross-check` run; normal CI covers Linux/macOS, the nightly covers Windows/WSL2
- **changed files**
        - R1-F1: `darkmatter/lib/Cargo.toml`, `darkmatter/lib/tests/l1/shipped_artifacts.rs`, `claudine/lib/Cargo.toml`, `claudine/lib/tests/l1/shipped_artifacts.rs`, `simplified-schema/lib/Cargo.toml`, `simplified-schema/lib/tests/l1/shipped_artifacts.rs`, `scripts/ci/test_affected_scope.py`
        - R1-F2: `scripts/shipped_schemas.py`, `{simplified-schema,darkmatter,claudine}/lib/examples/shipped_schemas.rs`, `{simplified-schema,darkmatter,claudine}/lib/tests/l1/shipped_artifacts.rs`, `tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs`
        - both: `simplified-schema/docs/topics/shipped-artifacts.md`, `.claude/skills/darkmatter/SKILL.md`
- **final verification** (run by the orchestrator after the last change)
        - simplified-schema `just test`: 1695/1695 passed; `just lint` exit 0 — R1-F1, R1-F2
        - darkmatter `just test`: 16432 passed, 10 skipped, none ≥5 s; `just lint` exit 0 — R1-F1, R1-F2
        - claudine `just test`: 11946 passed, 15 skipped, none ≥5 s; `just lint` exit 0 (existing claudine-cli linker `__eh_frame` warning only) — R1-F1, R1-F2
        - `cargo nextest run -p test-toolkit --test shipped_schemas_coordinator_contracts`: 5/5 — R1-F2
        - `python3 -m unittest` planner cases `test_shipped_schema_inputs_select_each_downstream_freshness_test` and `test_the_shipped_schema_coordinator_selects_its_contracts`: OK — R1-F1, R1-F2
        - `affected_scope.py --resolved-plan --event pull_request simplified-schema/lib/src/validate.rs`: narrowed Darkmatter and Claudine freshness cells — R1-F1
        - `just shipped-schemas` (subagent): exit 0, every artifact current — R1-F2
