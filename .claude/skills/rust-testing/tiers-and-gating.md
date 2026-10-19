# Tiers and Gating

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## OS-specific tests are ordinary tests

**Tier is the resource a test needs, never the operating system it targets.**
An OS-specific test is a normal test behind `#[cfg(...)]`; the CI matrix runs
the suite on each OS, so the `cfg` alone puts it on the right leg.

```rust
#[cfg(unix)]    #[test] fn sigint_during_prep_exits_130() { … }
#[cfg(windows)] #[test] fn ctrl_c_terminates_wrapped_child_on_windows() { … }
```

Both are L1: each synthesizes its signal with a plain API call (`kill`,
`GenerateConsoleCtrlEvent`) and needs no harness. Do not add `level2_`/`level3_`
because a test runs on one platform, and do not `#[ignore]` it because the dev
host cannot run it. Either strands the test: `just test` filters out the
prefix, `just test-l2`/`test-l3` select only their own prefix and never pass
`--ignored`, so it runs in no canonical recipe.

Symptoms of a mis-tiered OS-specific test:

- it needs a bespoke CI workflow, a hand-written recipe, or an exact-name
  invocation to run at all;
- it is `#[ignore]`d with a reason naming a *platform* rather than a *resource*;
- a tier prefix and a `#[cfg]` gate encode the same fact twice.

Reference: `biscuit-tui/cli/tests/level2/windows_captured_stdout.rs` is plain
Windows L1 evidence; its `#![cfg(windows)]` inner attribute is the whole
Windows-only declaration.

**Name process-wide side effects that rely on nextest.** That test rewires
std handles with `SetStdHandle`, which is safe only because nextest runs one
test per process (see [consolidated-binaries.md](consolidated-binaries.md#process-isolation-is-a-nextest-guarantee-not-a-rust-one)).
Say so in the test's `//!` docs whenever you rely on it.

**Compile the other platform's arms locally** with the area's
`just check-windows` (`claudine/justfile`, `sniff/justfile`): `cargo check
--tests --target x86_64-pc-windows-gnu` with `-Wa,-mbig-obj` (run `rustup
target add x86_64-pc-windows-gnu` first).

- Use the GNU target, not `x86_64-pc-windows-msvc`; on macOS the MSVC check
  fails inside `aws-lc-sys` for lack of Windows SDK headers, which says nothing
  about your code.
- Compiling is not running; `windows-latest` stays the runtime authority. The
  check does catch typos in Windows arms and imports used only under
  `#[cfg(unix)]` (`unused_imports`; gate the import too).
- A warm re-run re-emits warnings only for what it re-checks; when the warning
  count is the evidence, use a fresh `CARGO_TARGET_DIR`.

## Test levels

| Level   | Prefix     | Resource            | Skip when absent     | Hard-fail env |
|---------|------------|---------------------|----------------------|---------------|
| L1      | (none)     | In-process or hermetic subprocess/filesystem | Never | — |
| L2      | `level2_`  | Real terminal / PTY | Harness missing      | `BISCUIT_TEST_REQUIRED_BACKENDS` (preferred); `BISCUIT_TEST_LEVEL_REQUIRED=2` |
| L3      | `level3_`  | OS keyboard/mouse   | `RUN_LEVEL3` unset   | `BISCUIT_TEST_LEVEL_REQUIRED=3` |
| Browser | `browser_` | Chrome/Chromium     | Browser missing      | `BISCUIT_BROWSER_REQUIRED=1` |
| Real    | `real_`    | External device/API | Resource missing     | Per-package; see below |
| Slow    | `slow_`    | None (slow L1)      | Excluded from sanity | — |

## Gating

Put the gate at the top of the test body.

- **L2/L3: `require_level!`, naming the `Backend`.** It skips cleanly when the
  harness is missing, and the `Backend` gives the gate a machine identity that
  `BISCUIT_TEST_REQUIRED_BACKENDS` can require and prove. A plain string label
  (`"PTY (/dev/ptmx)"`, `"WezTerm + cliclick"`) is right only for composite or
  non-backend requirements; such gates contribute no execution evidence. A gate
  that corresponds to one backend must name it.
- **Gate in a helper that cannot `return`** (an `-> Option<T>` fixture builder):
  use `decide_harness!`, which records evidence and yields the `LevelDecision`.
  Calling `evaluate_harness` directly skips the recording, leaving the backend
  unproven although its tests ran.
- **L1: `expect_level!`, never `require_level!`.** L1 is mandatory, so a skip
  there reads as a pass on an unprovisioned runner. `expect_level!` takes the
  same arguments and panics with the missing requirement. Keep such a test off
  a platform with a compile-time exclusion (`#![cfg(unix)]` on the binary),
  never a runtime probe. Operator-selected exclusions (`BISCUIT_TEST_LEVEL`,
  `RUN_LEVEL3`) still skip.
- **Browser:** `if !biscuit_browser_harness::require_browser() { return; }`,
  with `#[serial_test::serial(browser)]`.

The macros live in `tools/test-toolkit/src/lib.rs`.

## Requiring L2 backends individually

`BISCUIT_TEST_LEVEL_REQUIRED=2` is all-or-nothing: it panics every L2 gate,
including GUI backends a headless runner cannot host.
`BISCUIT_TEST_REQUIRED_BACKENDS` makes only the named backends fatal; every
other backend still skips.

- Comma-separated, case-insensitive, matched **exactly** against `tmux`,
  `wezterm`, `kitty`, `apple-terminal` (`wez` is an error, not a near miss).
  Unset or whitespace means nothing is required.
- The same vocabulary is each package's `[package.metadata.ci.tests]
  l2-backends`. CI's L2 cells set the variable to the planner's intersection of
  that declaration with what the environment hosts.

```bash
BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2   # tmux fatal, GUI backends skip
```

**Availability is not execution.** A cell can install tmux, select no
tmux-backed tests, and exit 0. So when the variable is set, every gate appends
a `{backend, test, decision}` record to `$STAGE/backend-executions.jsonl`
(`$STAGE` = `$BISCUIT_JUNIT_STAGE_DIR`, else `target/nextest/ci-reports`), and
the tier is bracketed by `backend-proof` (`tools/test-toolkit/src/bin/backend-proof.rs`):

- `backend-proof reset` before the run discards stale records; without it, old
  evidence satisfies the check.
- `backend-proof verify` after it fails when a required backend produced no
  `run` record (exit `0` proved, `1` unproven, `2` bad config or evidence) and
  writes `$STAGE/backend-proofs.json`, which `scripts/ci/completion.py` reads.

`just/devops.just` brackets **once per tier, not per package**: `_test_l2`
brackets itself unless `_test_l2_all` has claimed ownership through
`BISCUIT_BACKEND_PROOF_OWNER`, because a per-package `reset` would erase earlier
packages' evidence. The mechanism is inert when the variable is unset.

## Requiring a real resource individually

`real_*` tests have the same availability-versus-execution problem. The
all-or-nothing switch `PLAYA_REAL_AUDIO_REQUIRED=1` (honored by `playa` and
`biscuit-speaks`) suits a fully provisioned host only. The per-resource form
names what must be present; `biscuit-speaks` is the reference:

- `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts` is comma-separated,
  case-insensitive, trimmed, and matched exactly against the identifiers in
  `biscuit-speaks/lib/src/test_support.rs`; a misspelling fails.
- Every skip branch of every `real_*` test goes through one helper
  (`skip_or_require`), so no new skip path can bypass the switch.

Two rules follow:

- **Never `#[ignore]` a real-resource test.** `just test` filters `real_` out and
  `just test-real` does not pass `--ignored`, so it would run in no tier. Gate
  availability in the body.
- **A `real_*` test in `#[cfg(test)] mod tests` under `src/` is selected**, by
  design: `(^|::)real_` matches the marker after the module path.

## Environment contract

| Variable | Purpose |
|----------|---------|
| `BISCUIT_TEST_LEVEL=1\|2\|3` | Max level to run; higher tiers skip cleanly. |
| `BISCUIT_TEST_LEVEL_REQUIRED=2\|3` | Missing harness panics instead of skipping. All-or-nothing. |
| `BISCUIT_TEST_REQUIRED_BACKENDS` | Named backends hard-fail; others skip. Turns on execution recording. |
| `BISCUIT_BROWSER_REQUIRED=1` | Missing Chrome panics instead of skipping. |
| `PLAYA_REAL_AUDIO_REQUIRED=1` | Missing real audio resource panics. All-or-nothing. |
| `BISCUIT_SPEAKS_REQUIRED_PROVIDERS` | Named TTS providers hard-fail; others skip. |
| `RUN_LEVEL3=1` | Opt-in for OS-keyboard-injection tests. |
| `BISCUIT_JUNIT_STAGE_DIR` | Staging root for JUnit reports and backend evidence. Default `target/nextest/ci-reports`. |
