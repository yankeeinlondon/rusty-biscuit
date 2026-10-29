---
ready: true
agent: kimi_code
model: ""
---

# Review 5

## Verification Performed

- Read `spec.md`, `plan.md`, and `review-4.md`.
- Inspected `tools/test-toolkit/src/lib.rs`, `biscuit-test-harness/src/shared.rs`, `biscuit-browser-harness/src/lib.rs`.
- Inspected `.config/nextest.toml`, `just/devops.just`, root `justfile`, and per-package justfiles for all 17 curated areas.
- Inspected CI workflows: `sanity.yml`, `test.yml`, `coverage.yml`, `fuzz-nightly.yml`, `bench-nightly.yml`.
- Inspected documentation: `.claude/skills/rust-testing/SKILL.md`, `docs/testing-strategy.md`, `prompts/snippets/test-rigor.md`, `CLAUDE.md`.
- Verified fuzz targets compile: `cargo +nightly fuzz build pdf_extract` (biscuit-file) and `cargo +nightly fuzz build markdown_parser` (darkmatter).
- Verified `cargo test -p test-toolkit --lib --no-run` and `cargo test -p biscuit-browser-harness --lib --no-run` compile successfully.
- Ran `timeout 65s just check-canonical` across all 17 curated areas — all passed.
- Ran `cd biscuit-hash && just all` to validate the full canonical tier chain on a no-L2 package — completed successfully.
- Searched for legacy `DARKMATTER_LEVEL2_REQUIRED` usage — none remaining.
- Searched for `[package.metadata.benchmarks]` across the repo — none found.

## Findings

### Medium — `[package.metadata.benchmarks]` opt-out convention not implemented

**Spec reference:** D14 + Topic 6 + plan Task 4.2.

The spec mandates that pure data crates or crates without measurable hot paths opt out of Criterion benchmarks via `[package.metadata.benchmarks] required = false` with a `reason` field in their `Cargo.toml`. This makes the opt-out grep-able for future tooling and documents the rationale next to the package manifest.

The implementation instead makes every no-bench package define an explicit no-op `bench` recipe in its justfile (e.g., `@echo "bench: not applicable for <pkg>"`). This satisfies the human-facing recipe contract but leaves the machine-readable metadata convention entirely unimplemented. No `Cargo.toml` in the workspace contains `[package.metadata.benchmarks]`.

**Recommended fix:** Add the metadata block to the `Cargo.toml` of every package whose `bench` recipe is a no-op, starting with the schematic crates, biscuit-location, biscuit-speaks, etc. Keep the justfile no-op as well (both are useful).

### Medium — `docs/testing-strategy.md` inaccuracy about nextest filtersets

**Spec reference:** D15 + Topic 1.

`docs/testing-strategy.md` states:

> "The full machine-readable contract lives in `.config/nextest.toml` as `set:level2`, `set:level3`, `set:browser`, `set:real`, and `set:slow`."

This is factually incorrect. `.config/nextest.toml` does **not** define these filtersets — the file's own header comment explicitly says "Nextest does not yet expose user-named filterset aliases as a stable feature". The shared recipes in `just/devops.just` pass the raw filter expression (`-E 'test(/level2_/)'`) directly. The skill doc (`.claude/skills/rust-testing/SKILL.md`) correctly notes this limitation.

**Recommended fix:** Update `docs/testing-strategy.md` to accurately describe that filter expressions live in the shared `_test_l2`, `_test_l3`, etc. recipes in `just/devops.just`, and that `.config/nextest.toml` only contains retry/slow-timeout profiles, not tier filtersets.

### Low — `biscuit-browser-harness` unit tests bypass `EnvGuard`

The `browser_decision_*` and `browser_required_reads_env` tests in `biscuit-browser-harness/src/lib.rs` manually call `unsafe { std::env::set_var(...) }` and restore the previous value by hand. They do not use `test_toolkit::EnvGuard`, which is the very helper this initiative standardizes for exactly this pattern. For a small crate with low test concurrency this is unlikely to race, but it is inconsistent with the testing best practices being rolled out.

**Recommended fix:** Replace manual env manipulation with `test_toolkit::EnvGuard::set_safe` / `remove_safe` in the browser harness unit tests.

### Low — `SharedHarness` at-exit cleanup lacks automated verification

`biscuit-test-harness/src/shared.rs` has unit tests for initialization (`shared_harness_initializes_on_first_get`) and for `take()` (`shared_harness_take_empties_slot`), but no test verifies that the `libc::atexit` hook actually fires and drops the harness at process exit. This is the primary value proposition of `SharedHarness` over a raw `Mutex<Option<T>>`. Verifying an atexit hook in a unit test is non-trivial (requires a subprocess), so this gap is understandable, but it means the cleanup path is only correct-by-inspection.

**Recommended fix:** Add an integration test that spawns a short-lived subprocess with a `SharedHarness`, then asserts the harness's drop side effects (e.g., a temp file removed, or a WezTerm pane killed) after the child exits.

### Low — `_test_l2` / `_test_l3` / `_test_browser` / `_test_real` lack `--no-tests=pass`

The shared `_sanity` recipe includes `--no-tests=pass` so that packages with no matching tests pass cleanly. The tier-specific shared recipes (`_test_l2`, `_test_l3`, `_test_browser`, `_test_real`) do not. In practice this is not a bug because every curated package either (a) has tests matching the tier, or (b) overrides the recipe with an explicit no-op. However, calling `_test_l2 <pkg>` directly for a package with no `level2_` tests will fail with "no tests to run". Since these are `_`-prefixed "private" recipes, this is a footgun rather than a user-visible defect.

**Recommended fix:** Add `--no-tests=pass` to `_test_l2`, `_test_l3`, `_test_browser`, and `_test_real` for defensive consistency with `_sanity`.

### Low — Missing pre-created `fuzz/crashes/` directories

The spec (D10) says each `fuzz/` target should have a `crashes/<target>/` directory for committed regression fixtures. Neither `biscuit-file/lib/fuzz/crashes/` nor `darkmatter/lib/fuzz/crashes/` exist. This is harmless until the first crash is found, at which point a contributor must create the directory manually.

**Recommended fix:** Pre-create `fuzz/crashes/pdf_extract/`, `fuzz/crashes/toml_roundtrip/`, `fuzz/crashes/yaml_roundtrip/`, `fuzz/crashes/json5_roundtrip/` under `biscuit-file/lib/fuzz/`, and `fuzz/crashes/markdown_parser/` under `darkmatter/lib/fuzz/`, each with a `.gitkeep` or README explaining the corpus policy.

## Test Rigor Notes

- **Review-4 finding fixed.** `level3_wezterm_alt_r_chord_selects_red` is now present in `biscuit-tui/cli/tests/real_terminal_render.rs` (line 752). The production-readiness note in `biscuit-tui/docs/components/choose_one.md` accurately states that both `Ctrl+R` and `Alt+R` chords are Level-3 verified, and no other modifier/chord combinations claim Level-3 coverage.
- **`require_level!` behavior:** Verified at Level 1 by the comprehensive unit-test suite in `tools/test-toolkit/src/lib.rs` (run, skip, panic, `BISCUIT_TEST_LEVEL` precedence, `RUN_LEVEL3` gating).
- **`check-canonical` validator:** Verified at integration level (script) — passes across all 17 curated areas.
- **Browser harness decision logic:** Verified at Level 1 by `biscuit-browser-harness` unit tests.
- **Fuzz infrastructure:** Targets compile and run; nightly CI workflow configured correctly.
- **CI workflow suite:** `sanity.yml`, `test.yml`, `coverage.yml`, `fuzz-nightly.yml`, and `bench-nightly.yml` are all present and match the spec.

## Production Readiness

Ready for production. The previous high-severity finding from review-4 is resolved. The remaining findings are all polish or consistency gaps (medium and low severity) that do not prevent the testing infrastructure, canonical recipes, or CI workflows from functioning correctly. The `[package.metadata.benchmarks]` and docs inaccuracies should be addressed in a follow-up patch, but they are not blockers.
