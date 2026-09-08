---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-07T23:36:28-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: true
implemented_by: claude/default
log: claudine/features/2026-08-26-finalized-references/log.md
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-3.md
previous: 2026-08-26-finalized-references/review-2.md
next: 2026-08-26-finalized-references/review-4.md
---

# Review 3: Finalized References

## Verdict

The implementation is not ready for production. Review 2's documentation finding is fixed, and the spawn-inventory analysis now catches the specific receiver, ordering, branch, loop, closure, and glob-import counterexamples requested there. Two high-severity acceptance blockers remain: the inventory can still omit or incorrectly approve executable commands, and AC10's required final cross-platform matrix is not yet green on the final tree, and the Windows/WSL Level 2 CI legs it depends on have not yet been provisioned.

## Findings

### High — The spawn inventory still fails open for valid construction and governance forms

AC6 requires the guard to classify every production `std::process::Command` and `tokio::process::Command` construction, including aliases, and to fail whenever a child can execute without the shared contribution helper. The new analyzer is substantially stronger, but it still has concrete false-negative paths:

- `Aliases::kind_for_constructor` in `claudine/cli/tests/spawn_inventory.rs` lines 106–136 recognizes an imported `Command` type alias or a fully qualified `std::process::Command::new` / `tokio::process::Command::new`. It does not recognize valid module imports such as `use std::process; process::Command::new(...)` or `use std::process as p; p::Command::new(...)`. Such a child is absent from the generated inventory rather than marked `UNCONTROLLED`.
- Helper recognition at lines 562–575 accepts any call whose last path segment is named `contribute_child_environment`. A local function or unrelated module function with that name is treated as the shared Claudine helper, so an ungoverned command can be marked governed.
- `scan_function` at lines 764–773 applies an indirect governor to every command in an allowlisted `(path, function)` pair even when flow analysis has already found an ungoverned execution. Adding `cmd.status()` inside either allowlisted `system_shell_command` factory would therefore still be reported as caller-governed. The allowlist test only checks the present factory-return shape; it does not assert that an execution inside the factory fails.

The ten targeted scanner tests pass, including the new adversarial cases, but none covers these counterexamples. This means the generated inventory is accurate for the currently recognized source shapes, not the future-preserving proof AC6 specifies.

Required change: resolve actual constructor/helper identities rather than matching a limited path spelling or terminal function name. At minimum, add module-import/module-alias fixtures, reject shadowed or unrelated helper functions, and make indirect governance conditional on the command leaving the factory without any local execution violation. A typed command factory or governed wrapper remains the simpler long-term way to make execution impossible before environment contribution.

Verification level: Level 1 is appropriate for this source-inventory invariant. The gap is semantic coverage and fail-open analysis, not test tier.

### High — AC10 remains incomplete; the Windows/WSL Level 2 CI legs it requires are not yet provisioned

AC10 requires `just test`, `just test-l2`, and `just lint` for biscuit-file, Darkmatter, and Claudine on macOS, native Linux, WSL, and native Windows. The implementation log now documents that Windows and WSL Level 2 jobs are absent by construction in `.github/workflows/_package-ci.yml`, where their absence is recorded as a policy gap in the rollup.

Ruling (Ken, 2026-09-08): that policy gap was always intended to be temporary. It is a record of what CI could not yet host at the time, not an authorized exclusion, and it does not soften AC10. AC10 is a real, standing requirement: the full platform matrix, including Level 2 on native Windows and WSL, must be green before this feature is accepted. The implementation log's framing that "part of AC10 is unsatisfiable as written and needs a specification amendment" is therefore rejected; the gap is closed by provisioning the missing environments, not by amending the criterion.

The available final-tree evidence is also not wholly green. The log records green macOS Level 1 and lint runs for all three areas and a green Darkmatter Level 2 run, but Claudine Level 2 still has a reproducible WezTerm failure caused by an interactive Atuin startup prompt. The prior CI run tested an older tree and had one Windows Level 1 and one macOS Level 2 failure; commit `ebc28e107` contains plausible targeted fixes, but no post-fix CI run verifies them. A host condition may explain a failure, but the acceptance criterion requires a passing gate or an authorized exclusion.

Required change: provision the missing Windows and WSL Level 2 CI environments so the rollup no longer records a policy gap for those cells, then record a complete green run for the final tree across all four platforms. Amending AC10 to a narrower matrix is not an available route. Resolve the macOS WezTerm host-startup failure (or harden the L2 harness against interactive shell-startup output) and verify `ebc28e107` in CI.

Verification level: AC10 explicitly requires Level 1 and Level 2 evidence. Level 3 is not applicable because no requirement depends on OS keyboard or mouse encoding.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser/resolver tests from the prior full-suite run | Appropriate and reported green. |
| AC2 — implicit reference precedence | Level 1 collision tests; recorded Level 2 compose/proxy checks | Appropriate and reported green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory tests | Appropriate and reported green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/no-discovery tests; recorded Level 2 nested-compose checks | Appropriate and reported green. |
| AC5 — `ctx.cwd` and materialization | Level 1 schema/orchestration tests; recorded Level 2 proxy/sequence checks | Appropriate and reported green. |
| AC6 — subprocess context propagation | Level 1 subprocess tests and a ten-test source-inventory suite | Correct tier, but the inventory remains fail-open; see the first finding. |
| AC7 — candidate aggregation and deduplication | Level 1 collision/deduplication tests; recorded Level 2 compose checks | Appropriate and reported green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests; recorded Level 2 magic-reference checks | Appropriate. No input-encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 platform-specific parser/filesystem tests, including recorded native Windows junction execution | Appropriate for syntax and filesystem semantics; final platform closure is governed by AC10. |
| AC10 — final quality gates | Current-tree macOS Level 1/lint and partial Level 2 evidence; older multi-platform CI | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and reported green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks plus manual documentation review | Appropriate. Review 2's active documentation drift is fixed. |
| AC13 — reserved syntax | Level 1 grammar tests and design review | Appropriate and reported green. |

No requirement asserts modifier-press visibility, hotkey activation, paste, IME, mouse behavior, or another encoder-sensitive interaction requiring Level 3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never` passed all 10 tests on macOS.
- The revised Darkmatter CLI reference and compose page now document `@`, `&`, `^`, implicit ordering, containment, recursive/interpolation modifiers, and the canonical biscuit-file reference.
- The feature log and CI workflow were inspected for the exact final-tree and platform/tier evidence claimed by AC10.
- `git diff --check` was run after writing this review and the linked frontmatter updates.

The targeted passing test confirms the new analyzer works for its fixtures. It does not close the untested fail-open constructions above or substitute for AC10's missing final matrix.
