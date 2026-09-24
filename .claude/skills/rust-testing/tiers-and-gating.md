# Tiers and Gating

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## OS-specific tests are ordinary tests

**Tier is about the resource a test needs, never about the operating system it
targets.** An OS-specific test is a normal test gated by `#[cfg(...)]`; the
matrix already runs the suite on each OS, so the `cfg` alone puts it on the right
leg and nowhere else.

```rust
#[cfg(unix)]                    // runs on the Linux/macOS legs
#[test]
fn sigint_during_prep_exits_130() { … }

#[cfg(windows)]                 // runs on the Windows leg
#[test]
fn ctrl_c_terminates_wrapped_child_on_windows() { … }
```

Both are L1: each synthesizes its signal with a plain API call (`kill`,
`GenerateConsoleCtrlEvent`) and needs no terminal harness. Do **not** reach for
`level2_`/`level3_` just because a test only runs on one platform, and do not
`#[ignore]` it because the dev host cannot run it — that is what CI's other legs
are for.

Getting this wrong is expensive and silent. Claudine's Windows Ctrl+C tests
carried `level3_`/`level2_` prefixes plus an `#[ignore]`, which made them
unreachable by **every** canonical recipe — `just test` filters out `level3_`,
`just test-l2` selects only `level2_`, `just test-l3` neither runs unattended nor
runs ignored tests, and CI's L2 job is Linux-only. Someone then wrote a bespoke
GitHub workflow to invoke one by exact name with `--ignored`. It never passed,
nobody noticed for months, and the fix was to delete the workflow and drop the
prefixes. See `features/2026-07-24-devops/ci-failure-inventory.md`.

Symptoms that you have mis-tiered an OS-specific test:

- it needs a bespoke CI workflow, a hand-written `just` recipe, or an exact-name
  invocation to run at all;
- it is `#[ignore]`d with a reason that names a *platform* rather than a
  *resource*;
- a tier prefix and a `#[cfg]` gate encode the same fact twice.

`biscuit-tui/cli/tests/windows_captured_stdout.rs` had all three at once: an
`#[ignore = "requires a Windows host"]`, a hand-written recipe, and a whole
specialized workflow that invoked it by exact name. `biscuit-tui-cli` already
declared `features = ["terminal-tests"]`, so its CI L1 cell had been *compiling*
that target all along and only the `#[ignore]` kept it from running. Deleting
the attribute, the recipe, and the workflow made it ordinary `windows-latest`
L1 evidence inside the package's own cell; the `#![cfg(windows)]` inner
attribute is the whole Windows-only declaration.

That test also rewires **process-wide** std handles through `SetStdHandle`.
That is safe here only because nextest runs one test per process — under
`cargo test`'s shared-process harness it would corrupt every sibling. Treat
"nextest gives me a process to myself" as a property worth naming in the test's
`//!` docs whenever you rely on it.

**Compile the other platform's arms locally.** An area's `just check-windows`
runs `cargo check -p <crates> --tests --target x86_64-pc-windows-gnu`
(mingw, with `-Wa,-mbig-obj`; `rustup target add x86_64-pc-windows-gnu`
first). Use that target, not `x86_64-pc-windows-msvc`: on a macOS host the
MSVC check dies inside `aws-lc-sys` for want of Windows SDK headers, which is
how one fix concluded its `#[cfg(windows)]` arms were uncompilable off CI.
Compiling is not running — `windows-latest` stays the runtime authority — but
a typo in a Windows arm becomes a local error instead of a CI surprise, and
an import used only inside `#[cfg(unix)]` cases shows up as
`unused_imports` here and nowhere else (gate the import too). A warm re-run
re-emits cached warnings only for what it re-checks; when the warning count
is the evidence, check into a fresh `CARGO_TARGET_DIR`.

## Test Levels

| Level   | Prefix     | Resource            | Skip when absent     | Hard-fail env                                              |
|---------|------------|---------------------|----------------------|------------------------------------------------------------|
| L1      | (none)     | In-process or hermetic subprocess/filesystem | Never | — |
| L2      | `level2_`  | Real terminal / PTY | Harness missing      | `BISCUIT_TEST_REQUIRED_BACKENDS` (per-backend, preferred); `BISCUIT_TEST_LEVEL_REQUIRED=2` (all-or-nothing) |
| L3      | `level3_`  | OS keyboard/mouse   | `RUN_LEVEL3` unset   | `BISCUIT_TEST_LEVEL_REQUIRED=3`                            |
| Browser | `browser_` | Chrome/Chromium     | Browser missing      | `BISCUIT_BROWSER_REQUIRED=1`                               |
| Real    | `real_`    | External device/API | Resource missing     | Per-package env vars; see "Requiring a real resource individually" |
| Slow    | `slow_`    | None (slow L1)      | Excluded from sanity | —                                                          |

### Requiring L2 backends individually

`BISCUIT_TEST_LEVEL_REQUIRED=2` is all-or-nothing: it panics *every* L2 gate,
including the GUI-backed ones a headless runner cannot host, which is why CI
used to check `tmux -V` as a proxy instead of demanding anything.

`BISCUIT_TEST_REQUIRED_BACKENDS` names the backends whose absence must be fatal
while every other backend still skips cleanly. It is a comma-separated,
case-insensitive list of the stable identifiers `tmux`, `wezterm`, `kitty`,
`apple-terminal` — matched **exactly**, so `wez` and `tmux2` are errors rather
than near-misses. Unset or all-whitespace means "no backend is required" and
every gate keeps its skip behavior. The same vocabulary appears in each
package's `[package.metadata.ci.tests]` `l2-backends` and
`scripts/ci/affected_scope.py`. CI's per-package L2 legs set it to the cell's
`backends` — the planner's intersection of the package's declaration with what
the environment hosts (tmux alone on every hosted runner today) — so an
installed-but-never-exercised backend fails the `_test_l2` backend-proof
bracket instead of rendering a green cell with zero executed L2 tests, and
`backend-proof verify` writes the per-backend verdict `completion.py` certifies
the cell from (`$STAGE/backend-proofs.json`).

```bash
BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2        # tmux fatal, GUI backends skip
```

**Availability is not execution.** A leg can install `tmux`, run a tier that
happens to select no tmux-backed tests, and exit 0 — a green cell that verified
nothing, indistinguishable from a real pass. An installed `tmux` plus zero tmux
tests is not evidence. So when the variable is set, every gate appends one
`{backend, test, decision}` record to `$STAGE/backend-executions.jsonl`
(`$STAGE` = `$BISCUIT_JUNIT_STAGE_DIR`, else `target/nextest/ci-reports`), and
the tier is bracketed by the `backend-proof` binary
(`tools/test-toolkit`, `--features backend-proof`):

- `backend-proof reset` before the run, discarding the previous run's records —
  without it, stale evidence satisfies the check and the mechanism silently
  degrades to a no-op;
- `backend-proof verify` after it, failing when a required backend produced no
  `run` record. Exit `0` proved (or nothing required), `1` unproven, `2` bad
  config / unreadable evidence.

`just/devops.just` wires both in, **once per tier rather than per package**:
`_test_l2` brackets itself, except when `_test_l2_all` (the multi-package
`_run_all` path) has claimed ownership via `BISCUIT_BACKEND_PROOF_OWNER`. A
per-package `reset` would erase earlier packages' evidence. An unproven backend
fails the tier without masking a genuine test failure, and the whole mechanism
is inert — no output, no file I/O, no added latency — when the variable is
unset.

### Requiring a real resource individually

The `real_` tier has the same availability-versus-execution problem as L2, and
solves it the same way. A `real_*` test skips when its backend is absent, so a
green `just test-real` can mean "the resource was there and playback completed"
or "nothing ran". A repository-wide switch (`PLAYA_REAL_AUDIO_REQUIRED=1`,
honored by `playa` and `biscuit-speaks`) turns *every* such skip into a failure,
which is right for a fully provisioned host and wrong for a runner that has one
backend and not another.

The per-resource form names only what must be present. `biscuit-speaks` is the
reference implementation: `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts`
is comma-separated, case-insensitive, whitespace-trimmed, matched exactly
against the identifiers in `biscuit-speaks/lib/src/test_support.rs`, and a
misspelled entry fails rather than silently disabling the requirement. Unset
means nothing is required. Every skip branch of every `real_*` test routes
through one shared helper (`skip_or_require`), so a new skip path cannot be
added that bypasses the switch.

Two rules follow from the same reachability contract as OS-specific tests:

- **Never `#[ignore]` a real-resource test.** `just test` filters `real_` out
  and `just test-real` does not pass `--ignored`, so an ignored `real_*` test
  runs in no tier at all. Availability gating belongs in the test body, behind
  the switch, not in an attribute.
- **A `real_*` name inside `#[cfg(test)] mod tests` in `src/` is selected**, and
  intentionally so: the filterset matches `(^|::)real_`, and the module path
  makes the marker the first segment of the test's final name.

## Gating Tests

Use `test_toolkit::require_level!` at the top of a test body. Pass a `Backend`
so the gate carries a machine identity, not just a diagnostic label:

```rust
use test_toolkit::{require_level, Backend, Level};

#[test]
#[serial_test::serial]
fn level2_renders_in_real_terminal() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    // ... test body
}
```

A plain string label still works and is the right choice for composite or
non-backend requirements — `"PTY (/dev/ptmx)"`, `"WezTerm + cliclick"` — where
no single backend identity applies. Such gates deliberately contribute no
execution evidence: they can neither satisfy nor block
`BISCUIT_TEST_REQUIRED_BACKENDS`. A gate that *does* correspond to one backend
must name it, or its tests run without ever proving that backend.

Where the gate lives in a helper that cannot `return` — an `-> Option<T>`
fixture builder, say — use `decide_harness!`, which records evidence and yields
the `LevelDecision`. Calling `evaluate_harness` directly skips the recording and
leaves the backend unproven even though its tests ran.

**Level 1 gates use `expect_level!`, not `require_level!`.** The clean skip is
right for L2/L3, where the harness is genuinely optional. L1 is the mandatory
suite and has no optional-harness contract, so a skip there is
indistinguishable from a pass: an unprovisioned runner reports green while
proving nothing. `expect_level!` takes the same arguments and panics — naming
the missing requirement, so the skip's diagnostic survives as the failure
message. Keep such a test off a platform with a compile-time exclusion
(`#![cfg(unix)]` at the top of the binary), never a runtime probe. An
operator-selected exclusion (`BISCUIT_TEST_LEVEL`, `RUN_LEVEL3`) still skips.

```rust
expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
```

Claudine's 32 L1 PTY gates were `require_level!` until review-3 of
`fixes/2026-09-07-faster-claudine-tests`; a host without `/dev/ptmx` skipped all
seven binaries and the run stayed green.

For browser tests:

```rust
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_computed_style_matches() {
    if !biscuit_browser_harness::require_browser() { return; }
    // ... test body
}
```

## Environment Contract

| Variable                             | Purpose                                                                                                        |
|--------------------------------------|----------------------------------------------------------------------------------------------------------------|
| `BISCUIT_TEST_LEVEL=1\|2\|3`           | Max level to run; higher tiers skip cleanly.                                                                   |
| `BISCUIT_TEST_LEVEL_REQUIRED=2\|3`    | Missing harness panics instead of skipping. All-or-nothing; for L2 prefer `BISCUIT_TEST_REQUIRED_BACKENDS`.     |
| `BISCUIT_TEST_REQUIRED_BACKENDS`     | Comma-separated `tmux,wezterm,kitty,apple-terminal`. Named backends hard-fail; others still skip. Also turns on execution recording, which `backend-proof verify` checks. See "Requiring L2 backends individually". |
| `BISCUIT_BROWSER_REQUIRED=1`         | Missing Chrome panics instead of skipping.                                                                     |
| `PLAYA_REAL_AUDIO_REQUIRED=1`        | Missing real audio resource panics instead of skipping. All-or-nothing; prefer the per-resource form below.     |
| `BISCUIT_SPEAKS_REQUIRED_PROVIDERS`  | Comma-separated TTS provider identifiers (`echogarden`, `gtts`). Named providers hard-fail; others still skip. See "Requiring a real resource individually". |
| `RUN_LEVEL3=1`                       | Opt-in for OS-keyboard-injection tests.                                                                        |
| `BISCUIT_JUNIT_STAGE_DIR`            | Staging root for JUnit reports and the backend-execution evidence file. Defaults to `target/nextest/ci-reports`.|
