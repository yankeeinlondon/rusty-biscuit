---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T05:44:56-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: true
next: 2026-08-26-finalized-references/review-5.md
implemented_by: claude/default
log: claudine/features/2026-08-26-finalized-references/log.md
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-4.md
previous: 2026-08-26-finalized-references/review-3.md
---

# Review 4: Finalized References

## Verdict

The implementation is not ready for production. Review 3's three named spawn-inventory defects are fixed, and CI run `34192299897` verifies the previously unverified `ebc28e107` fixes. Two high-severity acceptance blockers remain: AC6's inventory still has valid production forms that bypass its census or helper-identity check, and AC10's required final cross-platform matrix is incomplete.

## Findings

### High — The spawn inventory still has production forms that bypass its proof

AC6 requires the guard to fail on any production `std::process::Command` or `tokio::process::Command` construction whose child can execute without the shared environment helper, including aliased constructions. The review-3 fixes correctly recognize module/crate import aliases, reject same-named helper functions, and prevent an indirect governor from covering local execution. Three other valid forms remain outside that proof:

- `cfg_test` and `meta_contains_test` in `claudine/cli/tests/spawn_inventory.rs` lines 999–1023 skip an item whenever `test` occurs anywhere inside its `cfg` expression. Consequently, `#[cfg(not(test))] fn launch() { std::process::Command::new("x").status(); }` is omitted even though it is specifically a production-only function. `#[cfg(any(test, unix))]` is also omitted despite being production code on Unix.
- `AliasCollector` only records `use` bindings, and `kind_for_constructor` only consults those sets. A Rust type alias such as `type ProcessCommand = std::process::Command; ProcessCommand::new("x").status();` contributes no census entry, even though AC6 explicitly includes aliased constructions.
- Helper-module identity can still be replaced after glob resolution. `GlobResolver` may add `child_environment` to `Aliases::helper_module`, but `BareHelperScope` checks only local functions, not local modules. A local `mod child_environment` that shadows a glob-imported module can therefore make `child_environment::contribute_child_environment(&mut command)` look like the shared helper when it is not.

The current production inventory remains governed, so these are guard soundness defects rather than evidence of a currently ungoverned child. They still defeat AC6's required regression barrier: each form can introduce a production child without making the inventory test fail.

Required change: exclude only items proven test-only, resolve process-command type aliases, and apply the same local-shadowing rule to helper-module bindings that already exists for bare helper functions. Add non-vacuous Level 1 fixtures for `cfg(not(test))`, mixed `cfg(any(test, ...))`, std/Tokio type aliases, and a local module shadowing a glob-imported helper module.

Verification level: Level 1 is appropriate for this source-inventory invariant. The mismatch is incomplete semantic coverage, not a need for a terminal emulator.

### High — AC10's required final platform matrix is still incomplete

AC10 requires `just test`, `just test-l2`, and `just lint` in biscuit-file, Darkmatter, and Claudine on local macOS, native Linux, WSL, and native Windows. CI run `34192299897` materially improves the evidence: on commit `ebc28e107`, Level 1 and lint are green on all four platforms, and the Darkmatter/Claudine Level 2 jobs are green on Linux and macOS. It also verifies the two fixes that review 3 identified as not yet tested in CI.

That run does not satisfy AC10:

- Native Windows and WSL still have no provisioned Level 2 backend. `.github/ci/environments.json` records the Windows headless-terminal gap and the WSL broker/tmux gap, while `.github/workflows/_package-ci.yml` excludes both environments from the Level 2 matrix by construction. Ken's 2026-09-08 ruling explicitly says these are temporary policy gaps, not exclusions from AC10.
- The final reviewed tree has no CI run. Commits `9336f0582` and `762c147ab`, plus the uncommitted spawn-inventory change, postdate the green run.
- The current local macOS Claudine Level 2 result is 238/239. `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` is blocked by an Atuin first-run prompt in the inherited interactive shell. That is a host/harness condition rather than evidence of a file-reference defect, but AC10 requires a passing gate or an authorized exclusion; neither exists.

Required change: provision real Level 2 execution on native Windows and WSL, resolve the local WezTerm startup interference without taking terminal focus, and record a complete green matrix for the final tree. A backend being installed or a placeholder job succeeding is not evidence unless a real Level 2 test records execution.

Verification level: AC10 explicitly requires Level 1 and Level 2 evidence. Level 3 is not applicable because the feature does not depend on terminal input encoding or OS keyboard/mouse injection.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser/resolver and CLI fixtures | Appropriate and reported green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures and recorded Level 2 compose/proxy coverage | Appropriate and reported green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and reported green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks and recorded Level 2 nested composition | Appropriate and reported green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix and recorded Level 2 proxy/sequence coverage | Appropriate and reported green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess tests and 15 source-inventory tests | Correct tier, but the inventory proof remains incomplete; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures and recorded Level 2 compose checks | Appropriate and reported green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests and recorded Level 2 magic-reference checks | Appropriate. No input-encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks plus native filesystem fixtures, including Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Green Level 1/lint on four platforms at `ebc28e107`; green Linux/macOS Level 2; incomplete final-tree/local/Windows/WSL evidence | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and reported green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and manual documentation review | Appropriate and reported green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and reported green. |

No requirement asserts modifier-press visibility, hotkey activation, paste, IME, mouse behavior, or another encoder-sensitive interaction requiring Level 3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never` passed 15/15 tests on macOS, including the review-3 regression fixtures and the generated production inventory check.
- GitNexus upstream impact analysis reports LOW risk for `kind_for_constructor`, `is_helper_call`, `scan_function`, and `indirect_governor`; the change is confined to tests and affects no indexed production execution flow.
- The analyzer's alias collection, `cfg` filtering, helper-identity resolution, and flow/governor logic were inspected directly rather than inferred from the implementation log.
- The AC10 claims were compared with the specification, `.github/ci/environments.json`, `.github/workflows/_package-ci.yml`, the final-tree commit history, and the recorded local/CI results.

The passing targeted suite proves the analyzer handles its 15 current fixtures and the present production source. It does not exercise the bypasses in the first finding or supply the missing platform/tier evidence in the second.
