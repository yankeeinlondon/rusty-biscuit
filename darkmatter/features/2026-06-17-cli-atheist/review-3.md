---
ready: true
agent: codex
model: ""
resolved: 2026-06-17
---

# Review 3

## Findings

### High: Level 2 helpers create the `md` shim before the Level 2 skip gate

`run_md_env` now routes Level 2 tests through `md_shim()`, which correctly fixes the stale-`PATH` problem from review 2. The ordering is wrong, though: `run_md_env` evaluates `md_shim()` before entering `run_md_env_bin`, and `run_md_env_bin` is where `wezterm_decision()` performs the `Level::L2` skip/enforce decision.

Relevant code:

- `darkmatter/cli/tests/common/level2.rs:57`
- `darkmatter/cli/tests/common/level2.rs:160`
- `darkmatter/cli/tests/common/level2.rs:165`
- `darkmatter/cli/tests/common/level2.rs:181`

This means a host with no usable WezTerm still tries to create and canonicalize the shim before the test can skip cleanly. On Unix this usually works by accident; on Windows, `std::os::windows::fs::symlink_file` can require elevated privileges or Developer Mode, so `just test-l2` can fail during harness setup even when the real-terminal tier should skip. The new `level2_harness_integrity` tests also execute `md_shim()` in the Level 1/sanity filter, because `cargo nextest list -p darkmatter-cli -E '!(test(/level2_/) + ...)'` includes all four functions in the `level2_harness_integrity` binary.

Verification level: Level 2 is the right level for the rendering requirements, and the actual rendering tests still call `run_md`/`run_md_env`. But the strongest current evidence is only valid on hosts where shim creation succeeds before the L2 gate. That leaves a high-severity test-rigor gap for the required skip-clean Level 2 contract and for Windows compatibility.

Fix: move the Level 2 gate before shim creation. For example, have `run_md_env` perform `wezterm_decision()` first, then create the shim only in the `Run` branch, or make `run_md_env_bin` accept a lazy `FnOnce() -> &str`/enum so it can gate before resolving the binary. Also make the structural shim tests either avoid symlinks on Windows (hard-link/copy fallback with an adjusted integrity check) or explicitly gate/skip when symlink creation is unavailable.

**Resolution (2026-06-17):** Resolved. `darkmatter/cli/tests/common/level2.rs`
now gates before shim creation and falls back gracefully when symlinks
are unavailable:

- `run_md_env_bin` takes `FnOnce() -> &'static str` for the binary
  path. The Level 2 gate runs before the closure is called, so
  `md_shim()`'s filesystem work happens only when the gate has passed.
  `run_md_env` passes `md_shim` as a function reference, so the shim
  resolves lazily inside the gated helper. A host that would skip the
  Level 2 tier never touches the filesystem for shim creation.
- `md_shim` creates the shim via a new `link_or_copy` helper with a
  fallback ladder: symlink → hard link → copy. Hard links work without
  extra privileges on the same volume; copies handle cross-volume temp
  directories. The Windows shim path uses the `md.exe` extension so the
  pane can resolve it as an executable.
- `assert_shim_resolves_to_built` is rewritten on top of a new
  `is_same_binary` helper that uses file identity (inode on Unix,
  volume serial + file index on Windows) as a fast path and falls back
  to byte-for-byte content comparison. The previous `canonicalize`
  check only worked for symlinks; `is_same_binary` also accepts hard
  links and copies.
- The `level2_harness_integrity` tests use `link_or_copy` and
  `is_same_binary` instead of direct `symlink_file` calls, so the
  structural tests stay valid in the Level 1/sanity filter on Windows
  hosts that lack Developer Mode.

## Requirement Coverage

- Review 2's stale-binary issue is mostly addressed: the default Level 2 helpers now invoke the Cargo-built `md` via `md_shim()`, and the bare `md` path is gone from `run_md`/`run_md_env`.
- `md validate refs --format json` now serializes `ReferenceValidationReport` directly, matching the `md graph --validate --json` validation block shape. The updated Level 1 baseline tests cover local paths, remote URLs, fragments, data URIs, inline records, validation errors, and graph validation output.
- No Level 3 coverage is required by this feature; it does not specify keyboard, paste, mouse, or terminal input-encoder behavior.
- The remaining blocker is test infrastructure rigor: Level 2 skip behavior can be bypassed by eager shim creation, and the new shim integrity checks currently run in the Level 1 filter.

## Verification Run

- `cargo test -p darkmatter-cli --test level2_harness_integrity --color=never` passed on macOS.
- `cargo test -p darkmatter-cli --test validate_refs --test graph --color=never` passed on macOS.
- `cargo clippy -p darkmatter-cli --test level2_harness_integrity --no-deps --color=never` passed.
- `just lint-files` passed and reported only accepted over-cap files.
- `cargo nextest list -p darkmatter-cli -E 'test(/level2_/)' --color=never` did not include `level2_harness_integrity`.
- `cargo nextest list -p darkmatter-cli -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))' --color=never` included all four `level2_harness_integrity` tests.
