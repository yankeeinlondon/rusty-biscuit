# Spec: Eliminate the `cargo run` CLI-test anti-pattern

- **Date:** 2026-06-26
- **Trigger:** `FLAKY 2/4 [ 2.900s] biscuit-speaks-cli::cli_test test_cli_help_flag`
- **Status:** biscuit-speaks corrected (this session); audit + guard pending

## Problem

CLI integration tests must exercise the package's binary. The anti-pattern is
spawning the binary by shelling out to **`cargo run`** from inside the test:

```rust
// ANTI-PATTERN
let output = Command::new("cargo")
    .args(["run", "-p", "biscuit-speaks-cli", "--", "--help"])
    .output()
    .expect("Failed to execute");
```

### Why it is flaky

`cargo nextest` runs test binaries in parallel. When many tests each invoke
`cargo run`:

1. **Build-lock contention.** Every `cargo run` re-acquires cargo's package
   build lock. Concurrent invocations serialize on it, so a test that should be
   instant blocks for seconds waiting for the lock. The reported failure —
   `test_cli_help_flag` at **2.9 s** for a `--help` that is genuinely
   sub-millisecond — is this wait.
2. **Output contamination.** Under contention or a stale target, `cargo` may
   emit `Compiling …` / `Blocking waiting for file lock …` progress. That noise
   can perturb tests that assert on exact `stdout`/`stderr` content (e.g.
   `stdout.contains("Usage:")`), which is why only the content-asserting tests
   flaked while the exit-code-only tests stayed green.
3. **Wasted wall-clock.** Each `cargo run` reparses the workspace and rechecks
   freshness for all 48 members before launching the binary that nextest has
   **already built**.

### Correct pattern

Cargo builds every binary in a package before running that package's
integration tests and exposes the path via `CARGO_BIN_EXE_<bin-name>`. Spawn
that binary directly — no cargo process, no lock, no rebuild:

```rust
// CORRECT — std, zero new dependency
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_so-you-say"))
}
```

```rust
// CORRECT — assert_cmd (repo's dominant convention)
use assert_cmd::cargo::cargo_bin_cmd;
let mut cmd = cargo_bin_cmd!("sniff");
```

Both resolve the pre-built binary at compile time. Note the env-var/macro key is
the **binary name** (`so-you-say`, `sniff`, `md`), not the package name.

## Audit

Surveyed every `*/cli/tests/**.rs` suite (172 files) plus all test files that
reference `cargo`. Classification by binary-spawn mechanism:

| Mechanism | Count | Verdict |
|---|---|---|
| `assert_cmd` `cargo_bin`/`cargo_bin_cmd!` | 110 files | correct (dominant convention) |
| `env!("CARGO_BIN_EXE_<bin>")` + std `Command` | 18 files | correct |
| `biscuit-test-harness` (L2/L3 real-terminal: sends `bt …` / `question …` into a tmux/WezTerm pane) | several | correct — different mechanism, not in scope |
| **`cargo run` from inside the test** | **1 file** | **anti-pattern** |

**Only offender:** `biscuit-speaks/cli/tests/cli_test.rs` — all 34 tests shelled
out via `cargo run -p biscuit-speaks-cli --`.

### Legitimate `cargo` usages (NOT the anti-pattern — leave unchanged)

These invoke `cargo` as the **subject under test**, not as a way to launch an
already-built CLI:

- `schematic/gen/tests/e2e_generation.rs` — runs `cargo check` / `cargo clippy`
  to prove *generated* code compiles. `#[ignore]`d.
- `sniff/lib/src/programs/test_runner*.rs`, `sniff/lib/tests/{git_parity,integration}.rs`
  — cargo/test-runner detection is the feature being tested.
- `tools/test-toolkit/tests/nextest_config_verification.rs` — deliberately
  spawns `cargo nextest` to verify `.config/nextest.toml` `slow-timeout`.
  `#[ignore]`d, opt-in via `--run-ignored only`.

## Required changes

### 1. biscuit-speaks (DONE — record only)

`biscuit-speaks/cli/tests/cli_test.rs` rewritten to spawn via a `cli()` helper
returning `Command::new(env!("CARGO_BIN_EXE_so-you-say"))`. No dependency added;
existing `std::process::Command` structure preserved.

**Verification (run on macOS host):**

- `cargo nextest run -p biscuit-speaks-cli --test cli_test` → 34 passed.
- Content-asserting tests (`--help`, `--version`, conflict cases) dropped from
  ~3 s to **~0.01–0.04 s**.
- `test_cli_help_flag` run ×3 in a row → 9/9 pass, 0 flake, ~0.03 s each.
- Remaining `SLOW` (~5 s) tests are those that genuinely drive a TTS provider
  (`speak "test"`), unrelated to the build-lock flake.

> Optional follow-up (low priority, consistency only): migrate
> biscuit-speaks-cli to `assert_cmd` `cargo_bin_cmd!` to match the 110-file
> majority. Deferred — the `env!` form is already correct and dependency-free.

### 2. Regression guard (NEW)

Prevent reintroduction. Add a workspace-wide source check that fails if any
`*/cli/tests/**.rs` file launches the package binary via `cargo run`.

- **Location:** new test in `tools/test-toolkit/tests/` (e.g.
  `no_cargo_run_in_cli_tests.rs`), matching the existing config-verification
  pattern there.
- **Logic:** walk `<workspace-root>/*/cli/tests/**/*.rs`; fail on any file
  matching the anti-pattern regex `Command::new("cargo")` combined with a
  `"run"` arg (or the literal `args(["run", "-p"`).
- **Allowlist:** the legitimate-`cargo` files listed above are outside
  `*/cli/tests/` and so are not matched; no allowlist needed. If a future CLI
  test must invoke cargo as subject-under-test, gate it behind an explicit
  `// allow: cargo-run-subject-under-test` marker the check skips.
- **Not `#[ignore]`d:** this is a fast static scan; it should run in the default
  suite so violations surface immediately.

## Success criteria

1. No `*/cli/tests/**.rs` file spawns its binary via `cargo run`.
2. `biscuit-speaks-cli::cli_test` is stable: 0 flakes across ≥10 consecutive
   `cargo nextest run` invocations; content-asserting tests complete in <100 ms.
3. The regression guard fails on a deliberately reintroduced `cargo run` and
   passes on the current tree.
4. No behavior change to the legitimate `cargo`-invoking tests in `schematic`,
   `sniff/lib`, and `test-toolkit`.

## Convention (for new CLI test suites)

- **Default to** `assert_cmd` `cargo_bin_cmd!("<bin-name>")` — the repo norm.
- `env!("CARGO_BIN_EXE_<bin-name>")` with std `Command` is acceptable when a
  suite wants zero extra dependencies.
- **Never** spawn the package binary via `cargo run` / `Command::new("cargo")`.
- Real-terminal (L2/L3) suites use `biscuit-test-harness`; see the
  `biscuit-test-harness` and `rust-testing` skills.

## Addendum — second, distinct flake: TTS/audio contention LEAK-FAILs

After the `cargo run` fix the suite went green but still showed an *intermittent*
`FLAKY` that **hopped between tests** each run (`test_cli_gender_flag_invalid`,
`test_cli_help_shows_voice_option`, `test_cli_empty_stdin`,
`test_cli_background_flag`, …), always with nextest's `LKFAIL` (LEAK-FAIL)
marker on TRY 1, then passing on retry.

### Root cause

Unrelated to `cargo run`. The repo's `.config/nextest.toml` sets
`leak-timeout = { period = "100ms", result = "fail" }`. biscuit-speaks-cli's
suite spawns the `so-you-say` binary, which drives **real OS TTS/audio** (`say`
+ audio playback). Under full-suite parallelism a dozen of those run at once and
saturate the shared audio subsystem; process teardown lags, so nextest's
post-test pipe-drain check trips a *spurious* LEAK-FAIL on whichever fast test
loses the race — including tests that spawn nothing (clap rejects
`--gender invalid` before any child starts) and the `--background` test whose
child is redirected to `Stdio::null()` (main.rs:1223–1225), proving no real pipe
is held. This is the same documented false-positive the existing `browser_`,
`preflight_graph_`, and `worktree` overrides already work around.

### Evidence

- In isolation `test_cli_gender_flag_invalid` passed 20/20; flakes appear **only**
  under full-suite parallel load.
- `-j 2` → 0 flakes / 5 runs; `-j 3` and `-j 4` → 0 flakes / 6 runs each. Lower
  concurrency also collapsed the "slow" count (≈12-14 → 0-7), confirming
  contention — not test logic — was inflating both signals.
- A 1s leak-grace alone cut the rate (3/6 → 2/10) but did not eliminate it; the
  residual always burned the full ~1.04s window, i.e. genuine contention, not
  sub-100ms lag.

### Fix (applied, `.config/nextest.toml`)

Root-cause cap on concurrency plus a residual-lag grace, in **both** `default`
and `ci` profiles:

```toml
[test-groups]
tts-audio = { max-threads = 4 }

[[profile.default.overrides]]      # and the matching [[profile.ci.overrides]]
filter = 'package(biscuit-speaks-cli)'
test-group = 'tts-audio'
leak-timeout = { period = "1s", result = "fail" }
```

`result = "fail"` is retained so a genuinely runaway TTS child still fails. The
test-group is self-contained (no per-developer `-j` flag needed; applies in CI).

### Verification (macOS host)

- `cargo nextest show-config test-groups -p biscuit-speaks-cli` →
  `group: tts-audio (max threads = 4)` bound to the package. ✓
- Default profile (no manual `-j`), 10 consecutive full-suite runs →
  **34 passed, 0 flaky** every time, ~13-19 s. ✓

### Note for the regression guard

The `no_cargo_run_in_cli_tests` guard (above) covers only the `cargo run`
anti-pattern. This second class — real-audio/heavy-subprocess CLI suites tripping
LEAK-FAIL under parallel load — is a config concern, not a source pattern; the
remedy is a per-package `test-group` cap + leak grace, as precedented here and in
the `browser_`/`worktree` overrides.
