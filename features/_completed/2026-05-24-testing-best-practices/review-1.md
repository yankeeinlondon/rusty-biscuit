---
ready: true
agent: codex
model: ""
---

# Review 1

## Findings

### High — Canonical recipe validation is implemented but not enforced

Spec D6 requires `_check_canonical` to validate every curated package-area justfile, and Phase 3 explicitly says to add the validator plus CI workflow. The validator exists in `just/devops.just`, but CI only runs `just all` (`.github/workflows/test.yml:43-44`) and never runs `_check_canonical`. Worse, the root orchestrator treats a missing recipe as a warning in the all-areas path: it prints `- no ... command` and does not add the area to `failed_areas` (`justfile:411-424`).

Requirement verification level: L1 is appropriate for this workflow requirement. Current strongest verification is effectively none in CI; the helper exists but is not selected. This means future drift in a package justfile can pass the PR gate, which is exactly the drift this feature is supposed to prevent.

Recommended fix: add a root recipe that iterates `areas` and runs `just _check_canonical` in each package area, then call it from the PR workflow before `just all`. Also make `_orchestrate` fail on missing recipes in the no-args path, or keep missing-recipe tolerance out of canonical commands.

### High — Coverage CI omits the required per-package reports

The selected coverage design requires per-package LCOV plus a workspace aggregate (`spec.md:399-401`), and the CI deliverable says `coverage.yml` runs `just coverage` per package plus the workspace aggregator (`spec.md:480-484`). The workflow only runs one workspace command and uploads one artifact (`.github/workflows/coverage.yml:49-56`).

Requirement verification level: L1 is appropriate. Current strongest verification covers only the workspace artifact, not the per-package outputs that reviewers are supposed to inspect.

Recommended fix: invoke `just coverage` in the workflow, preserve the per-package `lcov-*.info` files, then run/upload the workspace aggregate as a separate artifact.

### High — The fuzz workflow does not open an issue on new crashes

The spec states `fuzz-nightly.yml` should run nightly and open an issue on a new crash (`spec.md:480-483`). The workflow comments repeat that promise, but the jobs only checkout, install tooling, and run `cargo fuzz` targets (`.github/workflows/fuzz-nightly.yml:20-83`). There is no `issues: write` permission, no crash artifact upload, and no issue-creation step.

Requirement verification level: L1 is appropriate for workflow behavior. Current strongest verification is missing.

Recommended fix: upload `fuzz/artifacts/**` on failure and add an issue creation step gated on `failure()`, using `actions/github-script`, `gh issue create`, or an equivalent maintained action with `permissions: issues: write`.

### High — Fuzz corpus policy is violated by committed generated corpus entries

D10 says only small hand-curated `corpus-seed/` directories are committed, ephemeral fuzz corpora are discarded, and only minimized crash inputs are committed back under `fuzz/crashes/<target>/` (`spec.md:527`). The implementation has 83 tracked files under `biscuit-file/lib/fuzz/corpus/**` and `darkmatter/lib/fuzz/corpus/**`. These are generated corpus entries, not `corpus-seed` files or crash regression fixtures.

Requirement verification level: L1/static repo inspection is appropriate. Current state directly contradicts the storage policy and risks unbounded repo churn as fuzz runs grow corpora locally.

Recommended fix: remove tracked `fuzz/corpus/**`, add ignore rules for generated `corpus/`, `artifacts/`, and `target/` directories, and keep only `corpus-seed/**` plus future minimized `crashes/**` fixtures.

### High — `biscuit-tui` was not migrated to the new testing lifecycle or level helpers

Phase 1 requires migrating the current harness consumers: darkmatter, biscuit-terminal, biscuit-tui, and claudine (`spec.md:499`). `biscuit-tui/justfile` still has its older flow: `test` mixes Level 1 and Level 2 execution (`biscuit-tui/justfile:40-55`), Level 3 is exposed as `test-level-3` rather than the canonical `test-l3` (`biscuit-tui/justfile:57-73`), and the file lacks the canonical 12-recipe lifecycle. The Level 3 recipe also dispatches a new terminal and waits for a human to press Enter in the generated script (`biscuit-tui/justfile:92-123`), which is incompatible with the non-interactive CI/agent model.

Requirement verification level: for the Level 3 keyboard UX tests, Level 3 remains the correct verification level. Current strongest verification for the migration requirement is incomplete: the package still uses its bespoke flow and is excluded from the curated root `areas` list, so root `just all` and canonical validation cannot catch it.

Recommended fix: either migrate `biscuit-tui` to the canonical recipe names and `test-toolkit::require_level!`, or explicitly update the spec/docs to remove it from this initiative. The current state is a spec/implementation mismatch.

## Test Rigor Notes

- Terminal rendering and keyboard UX requirements still require Level 2 or Level 3 according to the spec's taxonomy. This review did not find a new user-facing terminal behavior being declared production-ready solely from Level 1 tests, but the `biscuit-tui` migration gap leaves existing L2/L3 workflows outside the new enforcement model.
- CI, justfile, coverage, benchmark, fuzz, and documentation requirements are workflow-visible but not terminal-emulator-visible; L1/static verification is the appropriate minimum for those requirements.

## Verification Performed

- Read the specification and phase plan.
- Inspected current just recipes, CI workflows, fuzz target layout, browser harness, test-toolkit, and shared terminal harness implementation.
- Ran `cargo test --color=never -p biscuit-test-harness -p test-toolkit -p biscuit-browser-harness --no-run` successfully.

## Production Readiness

Not ready for production. The core library pieces compile, but required CI enforcement and drift controls are incomplete, and the fuzz corpus policy is already violated in-tree.
