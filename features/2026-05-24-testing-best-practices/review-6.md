---
ready: false
agent: kimi_code
model: ""
---

# Review #6 — Testing Best Practices

**Date:** 2026-05-24  
**Reviewer:** kimi_code  
**Scope:** Full spec implementation (Phases 1–6)

---

## Executive Summary

The testing infrastructure is largely complete and well-structured: `test-toolkit` exposes a sound `require_level!` macro and `Level` enum; `biscuit-browser-harness` wraps chromiumoxide cleanly; shared `just` recipes (`_sanity`, `_test_l2`, `_test_browser`, etc.) are wired across all 17 curated areas; CI workflows (`sanity.yml`, `test.yml`, `fuzz-nightly.yml`, `coverage.yml`, `bench-nightly.yml`) are present; documentation (`.claude/skills/rust-testing/SKILL.md`, `docs/testing-strategy.md`, `prompts/snippets/test-rigor.md`) is accurate and up-to-date; fuzz targets and new Criterion benchmarks compile and run.

However, **Phase 1’s consumer migration is incomplete**. Claudine’s L2 PTY tests remain `#[ignore]`d, which makes them invisible to the canonical `test-l2` recipe. Biscuit-terminal and biscuit-tui have not adopted `SharedHarness`, leaving per-test harness spawn overhead unaddressed. Because the spec explicitly requires these four consumers (darkmatter, biscuit-terminal, biscuit-tui, claudine) to be migrated to the new helpers, the feature cannot be marked production-ready until the gaps below are closed.

---

## Verification Level Audit

| Requirement | Strongest Test Level | Verdict |
|---|---|---|
| `require_level!` skips cleanly when harness unavailable | **L1** — unit tests in `test-toolkit` (`evaluate_level_*` suite) | ✅ Pass |
| `require_level!` panics under `BISCUIT_TEST_LEVEL_REQUIRED` | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `BISCUIT_TEST_LEVEL=1` gates L2/L3 tests | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `RUN_LEVEL3=1` gates L3 tests | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `require_browser()` skips when Chrome absent | **L1** — unit tests in `biscuit-browser-harness` | ✅ Pass |
| `require_browser()` panics under `BISCUIT_BROWSER_REQUIRED=1` | **L1** — unit tests in `biscuit-browser-harness` | ✅ Pass |
| `SharedHarness` initializes once and cleans up at exit | **L1** — unit tests + `shared_atexit.rs` integration test in `biscuit-test-harness` | ✅ Pass |
| `sanity` completes in ≤15 s (test execution only) | **Operational** — timed `just sanity` in darkmatter (110 tests in ~0.14 s) | ✅ Pass |
| `just all` runs in D7 order | **Operational** — recipe inspection | ✅ Pass |
| `check-canonical` validates 12-recipe set | **Operational** — executed against all 17 areas; 0 failures | ✅ Pass |
| Nextest filter expressions select correct tiers | **Operational** — `cargo nextest run -E 'test(/level2_/)'` on darkmatter, biscuit-terminal selects only L2 binaries | ✅ Pass |
| Browser harness renders and computes styles | **L2** — `darkmatter/lib/tests/browser_render.rs` (`browser_code_block_background_computes_in_browser`) | ✅ Pass |
| Claudine L2 tests skip cleanly and run via `test-l2` | **Not verified** — tests are `#[ignore]`d; invisible to nextest filters | ❌ **Gap** |
| Biscuit-terminal L2 tests use unified `require_level!` | **Not verified** — still use manual `skip_with_reason` | ❌ **Gap** |
| Biscuit-tui L2 tests reuse a harness via `SharedHarness` | **Not verified** — spawn per test | ❌ **Gap** |

---

## Findings

### 🔴 High

#### H1 — Claudine L2 PTY tests are `#[ignore]`d and bypass the canonical `test-l2` recipe

**Files:** `claudine/cli/tests/level2_pty_tests.rs`, `level2_context_pty.rs`, `level2_validation_reporter_pty.rs`

**Issue:** Every test in these files carries `#[ignore = "..."]`. Nextest’s default profile (and the `_test_l2` recipe) does **not** pass `--run-ignored`, so the canonical `just test-l2` in claudine selects zero tests. This defeats the purpose of the unified tier recipe: CI and developers typing `just test-l2` get a false sense of coverage.

**Spec violation:** Topic 1 / D2 — tests must skip cleanly via the runtime helper, not via `#[ignore]`. Topic 7 / D6 — `test-l2` must select and run the L2 suite.

**Fix:** Remove `#[ignore]` from all three files; insert `require_level!(Level::L2, <harness>::available(), "label")` at the top of each test body. If the tests are timing-sensitive, use `#[serial_test::serial]` and/or increase nextest’s `slow-timeout` override for those specific test names.

---

### 🟡 Medium

#### M1 — Biscuit-terminal L2 tests not migrated to `require_level!`

**Files:** `biscuit-terminal/cli/tests/level2_*.rs`

**Issue:** These tests still call `WezTermHarness::available()` manually and use `biscuit_test_harness::skip_with_reason` to emit skip messages. They do not use the unified `test_toolkit::require_level!` macro, so they do not participate in the `BISCUIT_TEST_LEVEL_REQUIRED` hard-fail contract.

**Spec violation:** Phase 1 Task 1.4 — migrate biscuit-terminal to the new `test-toolkit` level helpers.

**Fix:** Replace the manual `if !available { skip_with_reason(...); return; }` blocks with `require_level!(Level::L2, WezTermHarness::available(), "WezTerm")`.

#### M2 — Biscuit-terminal and biscuit-tui L2 tests do not use `SharedHarness`

**Files:** `biscuit-terminal/cli/tests/level2_*.rs`, `biscuit-tui/cli/tests/real_terminal_render.rs`

**Issue:** Every test spawns a fresh harness (`WezTermHarness::new()`, `TmuxHarness::new()`, etc.). WezTerm spawns cost 2–3 s each; with many serial tests in a binary the total wall time grows quickly. In biscuit-terminal, several L2 tests already exceed the 5 s nextest `slow-timeout` threshold and are flagged slow. In biscuit-tui, the lack of a shared pane contributed to observed `tmux new-session failed` errors when tests ran concurrently (nextest parallelizes across binaries, but serial tests within a binary still pay the spawn cost repeatedly).

**Spec violation:** Phase 1 Task 1.4 — adopt `SharedHarness`. D5 — tmux is the default L2 backend; SharedHarness is explicitly designed for this pattern.

**Fix:** Introduce a `static SHARED: SharedHarness<WezTermHarness> = SharedHarness::new();` (or `TmuxHarness`) at the top of each test file that holds many `#[serial(level2_...)]` tests. Convert test setup to `SHARED.get_or_init(|| { ... })`.

#### M3 — Nextest filtersets are documented only in comments, not configured

**File:** `.config/nextest.toml`

**Issue:** The spec (D15) calls for nextest filterset aliases (`set:level2`, `set:level3`, etc.). The file contains a comment stating that nextest “does not yet expose user-named filterset aliases as a stable feature.” While nextest indeed lacks named aliases, it **does** support `default-filter` (a filterset expression) in profiles. The current workaround (hard-coding expressions in `just/devops.just`) works, but the config file does not actually define any filter machinery, which means agents reading `.config/nextest.toml` must parse the comment to discover the expressions.

**Severity:** Medium only because it creates a minor discoverability drift; the recipes themselves are correct.

**Fix:** Add a `[profile.default]` `default-filter` comment or a top-level `[filtersets]` block if/when nextest stabilizes the feature. At minimum, keep the header comment current.

---

### 🟢 Low

#### L1 — `schematic/schema/Cargo.toml` missing `reason` in bench opt-out metadata

**File:** `schematic/schema/Cargo.toml`

**Issue:** The `[package.metadata.benchmarks]` block ends at `required = false` with no `reason = "..."` field. Every other opt-out crate includes the reason.

**Fix:** Add `reason = "Generated code; rebuilt via just generate, no runtime hot paths."`

#### L2 — `biscuit-browser-harness` lacks a `README.md`

**File:** (missing) `biscuit-browser-harness/README.md`

**Issue:** `docs/testing-strategy.md` links to it. Agents and humans navigating the crate have no local entry point.

**Fix:** Create a short `README.md` covering the trait surface, skip-clean contract, and `CHROME` env override.

#### L3 — Nightly fuzz workflow does not explicitly replay committed crash fixtures

**File:** `.github/workflows/fuzz-nightly.yml`

**Issue:** The workflow runs `cargo +nightly fuzz run <target> -- -runs=10000 ...` but does not pass the `crashes/<target>/` directory as a seed. `cargo-fuzz`/`libFuzzer` does **not** automatically discover the `crashes/` folder; crashes must be fed in as positional arguments or copied into `corpus/`. The `crashes/README.md` claims the workflow replays them, which is not currently true.

**Fix:** Add a step before each fuzz run:
```bash
cargo +nightly fuzz run <target> crashes/<target>/ -- -runs=0
```
or merge crashes into the corpus directory.

---

## Recommendations

1. **Close H1 first.** Removing `#[ignore]` from claudine’s L2 tests and gating them with `require_level!` is the single highest-impact fix; it makes the canonical recipe contract real.
2. **Migrate biscuit-terminal L2 tests to `require_level!`** for consistency with the unified env contract.
3. **Adopt `SharedHarness` in biscuit-terminal and biscuit-tui** to cut per-test spawn overhead. This is a performance win, not just a consistency win.
4. **Address L3** so that committed crash fixtures act as a regression gate rather than documentation-only.

---

## Conclusion

The infrastructure (crates, recipes, CI, docs, fuzz, benchmarks) is solid and ready. The remaining blockers are **consumer migration gaps** in claudine, biscuit-terminal, and biscuit-tui. Once H1 is resolved and the medium-severity migration items (M1, M2) are addressed, this feature will be production-ready.
