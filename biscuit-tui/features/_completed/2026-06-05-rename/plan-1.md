---
agent: codex/default
phase: 2
total_phases: 3
created: 2026-06-30T15:17:15
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - biscuit-tui/features/2026-06-05-rename/spec.md
  - biscuit-tui/features/2026-06-05-rename/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2:
  - biscuit-tui/features/2026-06-05-rename/spec.md
  - biscuit-tui/features/2026-06-05-rename/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - biscuit-tui/features/2026-06-05-rename/plan-1.md
docs_created_during_phase_3:
  - biscuit-tui/features/2026-06-05-rename/validation-1.md
skills_files_updated_during_phase_3: []
source_code: []
documentation:
  - biscuit-tui/features/2026-06-05-rename/spec.md
  - biscuit-tui/features/2026-06-05-rename/plan.md
  - biscuit-tui/features/2026-06-05-rename/plan-1.md
  - biscuit-tui/features/2026-06-05-rename/validation-1.md
packages:
  - biscuit-tui
  - claudine
  - biscuit-icon
---

# Plan: Review Remediation for Rename Validation

This plan resolves the findings from `review-1.md`. The rename implementation itself appears mostly complete; the remaining work is to make the success criteria reproducible and validate every current live workspace caller of `biscuit-tui`.

## Phase 1 - Correct the Verification Boundary

Goal: replace the impossible residual-reference check with one that distinguishes stale identifiers from expected `biscuit-tui` references.

- [x] Update `biscuit-tui/features/2026-06-05-rename/spec.md` so the residual-reference command searches only stale identifiers: `tui_chrome|tui-chrome`.
- [x] Remove `biscuit-tui` from the residual-reference failure pattern anywhere it is treated as unexpected live output.
- [x] Add a short note to the verification text that `biscuit-tui` is the intended live package/area name and should remain in manifests, docs, workflows, skills, and source comments.
- [x] Update `biscuit-tui/features/2026-06-05-rename/plan.md` Phase 4 and Phase 5 validation text so it uses the corrected stale-identifier search.
- [x] Keep historical-record exclusions unchanged unless the corrected stale-identifier search proves an exclusion is unnecessary.

Parallelizable: the `spec.md` and `plan.md` wording updates can be done in parallel after the exact stale-reference command is agreed.

Validation checkpoint:

```bash
rg -n --hidden 'tui_chrome|tui-chrome' . \
  -g '!target/**' \
  -g '!.git/**' \
  -g '!**/features/_completed/**' \
  -g '!**/reviews/**' \
  -g '!features/2026-05-24-testing-best-practices/review-*.md' \
  -g '!.claude/skills/claudine/timeline.md' \
  -g '!claudine/claudine-output/**' \
  -g '!biscuit-tui/features/2026-06-05-rename/**'
```

Expected result: no live matches. If a live match appears, update that file rather than adding a new exclusion.

## Phase 2 - Refresh Dependent Caller Inventory

Goal: make the validation scope match current workspace metadata instead of the stale assumption that `claudine-cli` is the only external caller.

- [x] Run a metadata-based reverse-dependency scan for packages that depend on `biscuit-tui`.
- [x] Record the current dependent set in `biscuit-tui/features/2026-06-05-rename/spec.md`: `biscuit-tui-cli`, `claudine-cli`, and `biscuit-icon-cli`.
- [x] Update `biscuit-tui/features/2026-06-05-rename/plan.md` frontmatter `packages` list to include `biscuit-icon/cli`.
- [x] Add `biscuit-icon-cli` to the Phase 5 validation package set.
- [x] Inspect `biscuit-icon/cli/Cargo.toml` and confirm the dependency key already uses `biscuit-tui` with `path = "../../biscuit-tui/lib"`.
- [x] Search `biscuit-icon/cli` for stale `tui_chrome` and `tui-chrome` references.

Parallelizable: one person can update the feature documents while another verifies `biscuit-icon/cli` source and manifest state, after the reverse-dependency scan confirms the package set.

Validation checkpoint:

```bash
cargo metadata --no-deps --format-version 1
rg -n 'tui_chrome|tui-chrome' biscuit-icon/cli -g '!target/**'
```

Expected result: metadata confirms all current dependents are accounted for, and the `biscuit-icon/cli` stale-reference search returns no output.

## Phase 3 - Run Final Validation Matrix

Goal: produce reproducible evidence that the rename is healthy across the package area and all live dependents.

- [x] Run `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'` and confirm it prints exactly `biscuit-tui` and `biscuit-tui-cli`.
- [x] Run `sniff repo packages --package-area biscuit-tui --list` and confirm it reports `biscuit-tui-cli` and `biscuit-tui`.
- [x] Run the corrected stale-reference search from Phase 1 and confirm it returns no live matches.
- [x] From `biscuit-tui/`, run `just build`, `just test`, `just doctest`, and `just lint`.
- [x] From `claudine/`, run `just build`, `just test`, `just doctest`, and `just lint`.
- [x] From `biscuit-icon/`, run `just build`, `just test`, `just doctest`, and `just lint`, or document the closest package-specific commands if the area does not expose the full shared recipe set.
- [x] If a root `Cargo.lock` is present or generated, confirm it contains no stale `tui-chrome` package names.
- [x] Update the review remediation notes in the feature directory with the exact commands run and any failures, skips, or package-specific substitutions. (See `validation-1.md`.)

Parallelizable: `biscuit-tui`, `claudine`, and `biscuit-icon` build/test/doctest/lint runs can be split across implementers once Phase 1 and Phase 2 document updates are complete.

Validation checkpoint:

All commands above pass, or any failure is documented with a concrete follow-up task. Level 2 and Level 3 terminal tests are not required because this remediation changes validation/documentation scope, not TUI rendering, input handling, keybindings, paste, mouse, or modifier behavior.
