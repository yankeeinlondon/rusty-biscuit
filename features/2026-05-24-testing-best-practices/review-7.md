---
ready: true
agent: kimi_code
model: ""
resolved_by: claude_opus_4_7
resolved_at: 2026-05-24
---

# Review #7 — Testing Best Practices

**Date:** 2026-05-24  
**Reviewer:** kimi_code  
**Scope:** Full spec implementation (Phases 1–6)

---

## Executive Summary

Review-6 blockers H1 and M1 are fully resolved. Claudine's L2 PTY tests no longer carry `#[ignore]`; they gate cleanly via `require_level!`. Biscuit-terminal's L2 tests are uniformly migrated to the unified `require_level!` macro. Schematic's bench opt-out now includes a `reason`, the `biscuit-browser-harness` README is present and complete, and the nightly fuzz workflow replays committed crash fixtures before extending corpora.

However, **review-6 M2 (SharedHarness adoption) remains incomplete**. The spec's Phase 1 explicitly requires the three harness consumers — darkmatter, biscuit-terminal, and biscuit-tui — to migrate to the new helpers. Darkmatter has fully adopted `SharedHarness` (three level2 files). Biscuit-terminal adopted it in only one of eight level2 test files. Biscuit-tui has not adopted it at all. Because `SharedHarness` is the primary mechanism for eliminating per-test harness spawn overhead (2–3 s each), this gap directly impacts CI wall-clock time and local developer experience.

Until biscuit-terminal's remaining level2 files and biscuit-tui's `real_terminal_render.rs` are migrated, the feature cannot be marked production-ready.

---

## Verification Level Audit

| Requirement | Strongest Test Level | Verdict |
|---|---|---|
| `require_level!` skips cleanly when harness unavailable | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `require_level!` panics under `BISCUIT_TEST_LEVEL_REQUIRED` | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `BISCUIT_TEST_LEVEL=1` gates L2/L3 tests | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `RUN_LEVEL3=1` gates L3 tests | **L1** — unit tests in `test-toolkit` | ✅ Pass |
| `require_browser()` skips when Chrome absent | **L1** — unit tests in `biscuit-browser-harness` | ✅ Pass |
| `require_browser()` panics under `BISCUIT_BROWSER_REQUIRED=1` | **L1** — unit tests in `biscuit-browser-harness` | ✅ Pass |
| `SharedHarness` initializes once and cleans up at exit | **L1** — unit tests + `shared_atexit.rs` integration test | ✅ Pass |
| `sanity` completes in ≤15 s (test execution only) | **Operational** — `just sanity` in darkmatter (~0.14 s) | ✅ Pass |
| `just all` runs in D7 order | **Operational** — recipe inspection | ✅ Pass |
| `check-canonical` validates 12-recipe set | **Operational** — executed against all 17 areas; 0 failures | ✅ Pass |
| Nextest filter expressions select correct tiers | **Operational** — `cargo nextest run -E 'test(/level2_/)'` selects only L2 binaries | ✅ Pass |
| Browser harness renders and computes styles | **L2** — `darkmatter/lib/tests/browser_render.rs` | ✅ Pass |
| Claudine L2 tests skip cleanly and run via `test-l2` | **L1 + Operational** — `#[ignore]` removed; `require_level!` gates; `just test-l2` selects them | ✅ Pass |
| Biscuit-terminal L2 tests use unified `require_level!` | **L1 + Operational** — all `level2_*.rs` files migrated | ✅ Pass |
| Darkmatter L2 tests use `SharedHarness` | **L1 + Operational** — 3 of 3 level2 files adopted | ✅ Pass |
| Biscuit-terminal L2 tests use `SharedHarness` | **Partial** — only `level2_layout.rs` adopted; 7 other level2 files spawn per-test | ❌ Gap |
| Biscuit-tui L2 tests use `SharedHarness` | **Not verified** — `real_terminal_render.rs` spawns fresh harness per test | ❌ Gap |

---

## Findings

### 🔴 High

*No new high-severity findings. H1 from review-6 is closed.*

---

### 🟡 Medium

#### M1 — SharedHarness adoption incomplete in biscuit-terminal and biscuit-tui

**Files:**
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`
- `biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs`
- `biscuit-terminal/cli/tests/level2_diagrams.rs`
- `biscuit-terminal/cli/tests/level2_image.rs`
- `biscuit-terminal/cli/tests/level2_prose_styling.rs`
- `biscuit-terminal/cli/tests/level2_render_tree_style.rs`
- `biscuit-tui/cli/tests/real_terminal_render.rs`

**Issue:** These files spawn a fresh harness in every test body (`WezTermHarness::new()`, `KittyHarness::new()`, `TmuxHarness::new()`, `AppleTerminalHarness::new()`). WezTerm spawns cost 2–3 s each. With 60+ level2 tests across biscuit-terminal alone, the cumulative spawn overhead is well over 100 s of avoidable wall-clock time. Several tests already exceed nextest's 5 s `slow-timeout` threshold and are flagged slow.

**Spec violation:** Phase 1 Task 1.4 — migrate biscuit-terminal and biscuit-tui to adopt `SharedHarness`. D5 — tmux is the default L2 backend; `SharedHarness` is explicitly designed for this pattern.

**Fix:** Introduce `static SHARED: SharedHarness<T> = SharedHarness::new();` per harness type at the top of each file that holds many serial tests. Convert test setup to `SHARED.get_or_init(|| { ... })`. Files that mix backends (e.g. `level2_diagrams.rs` with both Kitty and WezTerm) need one static per backend type. Files that intentionally create multiple independent panes in a single test (e.g. `level2_cursor_and_hygiene.rs`) can keep direct `::new()` calls for those specific tests while still sharing the primary harness across the rest.

---

#### M2 — Claudine `test-l2` justfile comment is outdated

**File:** `claudine/justfile`

**Issue:** Line 67 reads:

```just
# Level-2 PTY tests. The transcripts are `#[ignore]`d for timing sensitivity;
# pass `-- --run-ignored=only` to force their execution.
```

This is false. Review-6 H1 required removal of `#[ignore]`; the tests now gate via `require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)")`. The comment misleads developers into believing the suite is invisible to nextest unless `--run-ignored` is passed.

**Fix:** Replace the comment with:

```just
# Level-2 PTY tests. Gated by `require_level!(Level::L2, pty_available(), ...)`;
# skip cleanly when /dev/ptmx is unavailable, hard-fail when
# BISCUIT_TEST_LEVEL_REQUIRED=2 is set.
```

---

#### M3 — Nextest filtersets documented only in comments

**File:** `.config/nextest.toml`

**Issue:** From review-6 M3, unchanged. The file contains only a header comment describing the filter expressions; no actual `[filtersets]` block or `default-filter` profile configuration is present. Nextest still lacks stable named filterset aliases, so this is a minor discoverability drift rather than a functional bug.

**Fix:** Keep the header comment current. If nextest stabilizes named filtersets, migrate the expressions from `just/devops.just` into `.config/nextest.toml`.

---

### 🟢 Low

#### L1 — `docs/testing-strategy.md` incorrectly excludes biscuit-tui from root areas

**File:** `docs/testing-strategy.md`

**Issue:** Under "Areas intentionally excluded from the canonical set", the document states:

> `biscuit-tui` — Mature package area with its own `justfile`, but historically not part of the root orchestrator.

This is outdated. `biscuit-tui` **is** present in the root `justfile`'s `areas` variable and passes `just check-canonical`.

**Fix:** Remove `biscuit-tui` from the exclusions table. If there is a historical reason to call it out, rephrase to "migrated in Phase 2; now part of the curated list."

---

## Recommendations

1. **Close M1 first.** Migrate the remaining biscuit-terminal level2 files and biscuit-tui's `real_terminal_render.rs` to `SharedHarness`. This is the single remaining spec-mandated migration gap from Phase 1.
2. **Fix M2** so the claudine justfile comment matches the actual test gating behavior.
3. **Fix L1** to keep `docs/testing-strategy.md` synchronized with the root `areas` list.

---

## Conclusion

The infrastructure (crates, recipes, CI, docs, fuzz, benchmarks, canonical recipe validation) is solid. Review-6's highest-severity finding (H1) is closed, and the migration to `require_level!` (M1) is complete across all consumers.

The remaining blocker is **partial SharedHarness adoption**. Darkmatter is fully migrated, but biscuit-terminal and biscuit-tui still pay per-test spawn costs that `SharedHarness` was designed to eliminate. Once those files are migrated, this feature will be production-ready.

---

## Resolution (2026-05-24)

All findings addressed:

- **M1 (SharedHarness adoption)** — Resolved.
    - **biscuit-terminal**: All six level-2 test files now use `SharedHarness`:
      `level2_apple_terminal_prose.rs` (added `SHARED_APPLE` initialised with
      `preserve_capabilities(true)`; lifecycle test kept ownership for Drop
      coverage), `level2_cursor_and_hygiene.rs` (added `SHARED_WEZTERM`;
      `no_orphan_save_restore_sequences` kept independent panes by design),
      `level2_diagrams.rs` (`SHARED_KITTY`/`SHARED_WEZTERM`/`SHARED_TMUX`;
      also added missing `#[serial(level2_terminal)]` to the tmux fallback
      test), `level2_image.rs`, `level2_prose_styling.rs`, and
      `level2_render_tree_style.rs`. Each migrated test sends `clear\n` +
      `settle()` before its first interaction.
    - **biscuit-tui**: `real_terminal_render.rs` rewritten to use
      `SharedHarness<WezTermHarness>` / `SharedHarness<KittyHarness>` /
      `SharedHarness<TmuxHarness>` shell panes; each level-2 test sends
      `clear` + the question binary as a shell command, captures, then
      sends `Ctrl+C` to return to the prompt. Level-3 tests retain their
      own foreground-visible harness because AXRaise requires the spawn
      to be on the active workspace.
    - `cargo check --tests` and `cargo clippy --tests` both pass with zero
      warnings for `biscuit-terminal-cli` and `tui-chrome-cli`.

- **M2 (claudine `test-l2` comment)** — Resolved. `claudine/justfile` line 67
  now reads:
  ```just
  # Level-2 PTY tests. Gated by `require_level!(Level::L2, pty_available(), ...)`;
  # skip cleanly when /dev/ptmx is unavailable, hard-fail when
  # BISCUIT_TEST_LEVEL_REQUIRED=2 is set.
  ```

- **L1 (testing-strategy.md)** — Resolved. The `biscuit-tui` row was removed
  from the "Areas intentionally excluded" table, and `biscuit-tui` was added
  to the curated `areas` snapshot to match the root `justfile`.
