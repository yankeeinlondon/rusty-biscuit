---
ready: true
agent: codex
model: ""
---

# Review 2

## Findings

### High — `biscuit-tui` still bypasses the shared L2/L3 enforcement contract

Spec D2 requires migrated tiered tests to use the shared `test-toolkit` level policy so `BISCUIT_TEST_LEVEL_REQUIRED=2|3` converts missing harnesses into hard failures. The iteration adds canonical `biscuit-tui` recipes and documents that behavior in `biscuit-tui/justfile:54-62`, but the actual tests still use local hand-rolled gates. For example, `level2_wezterm_renders_option_labels` returns early when WezTerm is absent (`biscuit-tui/cli/tests/real_terminal_render.rs:46-51`), the tmux case does the same (`biscuit-tui/cli/tests/real_terminal_render.rs:90-95`), and L3 still checks only `RUN_LEVEL3` through a local `level3_enabled()` helper (`biscuit-tui/cli/tests/real_terminal_render.rs:391-399`, `biscuit-tui/cli/tests/real_terminal_render.rs:474-475`). There is no `require_level!` / `evaluate_level` call in the `biscuit-tui` test tree.

Requirement verification level: Level 1/static plus env-gated test execution is appropriate for this infrastructure requirement. Current strongest verification is incomplete: `just check-canonical` proves the recipe names exist, but not that selected tests honor the shared skip-vs-fail env contract. As a result, `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 biscuit-tui` can still silently skip missing real-terminal coverage instead of failing the CI/job that explicitly requested enforcement.

Recommended fix: replace the local `if !Harness::available() { return; }` and `level3_enabled()` gates in `biscuit-tui/cli/tests/real_terminal_render.rs` with `test_toolkit::evaluate_level` or `require_level!` for `Level::L2` / `Level::L3`, matching the migrated darkmatter pattern. Keep the existing `level2_` / `level3_` names for nextest filtering.

### Medium — Fuzz crash issues are opened for any job failure and will duplicate nightly

Spec D4/D10 says the nightly fuzz workflow opens an issue on a new crash. The new workflow does open issues, but both issue steps are gated only by `if: failure()` (`.github/workflows/fuzz-nightly.yml:72-100`, `.github/workflows/fuzz-nightly.yml:135-163`). That will file "crash" issues for non-crash failures such as `cargo-fuzz` install failures, dependency build failures, GitHub runner outages, or cache/toolchain problems. It also creates a date-based title every run, so the same still-unfixed crash can create a fresh issue every night.

Requirement verification level: Level 1 workflow/static verification is appropriate. The current implementation satisfies "opens an issue on failure" but not "opens an issue on new crash."

Recommended fix: gate issue creation on the presence of `fuzz/artifacts/**` crash artifacts, upload logs separately for non-crash failures, and de-duplicate by target plus crash signature or by searching open `fuzz` issues before creating a new one.

## Test Rigor Notes

- `biscuit-tui` terminal UX tests are correctly named for Level 2 / Level 3 nextest selection, but the enforcement level is still wrong because missing harness behavior is local and does not honor `BISCUIT_TEST_LEVEL_REQUIRED`.
- The CI, justfile, coverage, and fuzz workflow requirements are workflow-visible rather than terminal-emulator-visible; Level 1/static verification is the right minimum for those requirements.

## Verification Performed

- Read `features/2026-05-24-testing-best-practices/spec.md`, `plan.md`, and `review-1.md`.
- Inspected the staged iteration diff for CI workflows, root orchestration, fuzz corpus cleanup, and `biscuit-tui` migration.
- Ran `just check-canonical` successfully across all 17 curated areas.
- Confirmed no tracked files remain under `biscuit-file/lib/fuzz/corpus/**` or `darkmatter/lib/fuzz/corpus/**` in the working tree.

## Production Readiness

Not ready for production. The first-review CI and corpus-policy gaps are mostly addressed, but `biscuit-tui` has only been migrated at the recipe layer; its actual L2/L3 tests still bypass the shared enforcement contract this feature is meant to standardize.
