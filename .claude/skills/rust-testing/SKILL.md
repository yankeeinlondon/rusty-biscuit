---
name: rust-testing
description: |-
  Monorepo testing guide: L1/L2/L3 taxonomy, canonical just recipes,
  test design, fixture isolation, `require_level!` / `expect_level!` gating,
  nextest filtersets, suite audits, and fuzzing. Load this
  before writing or reviewing tests in the rusty-biscuit workspace.
hash: 61d07be7e22c9f45-43ee6f6bd18f4fb0
last_updated: 2026-09-24
---
# Rust Testing — Rusty Biscuit Monorepo

This page is the index. Read the rules below and the decision tree, then open
the topic page that matches the task; each rule links to its reasoning and
evidence.

## Rules that fail silently when broken

Every rule here once produced a green run that proved nothing. Violating one
rarely causes an error. Usually the test just stops running, or runs against
the wrong thing.

1. **Tier is the resource a test needs, never the OS it targets.** An
   OS-specific test is an ordinary test behind `#[cfg(...)]`. Never give it a
   `level2_`/`level3_` prefix or an `#[ignore]` naming a platform.
   → [tiers-and-gating.md](tiers-and-gating.md)
2. **Gate L1 with `expect_level!`, L2/L3 with `require_level!`, and name the
   `Backend`.** A skip in L1 reads as a pass; a gate without a backend
   identity never proves that backend ran. → [tiers-and-gating.md](tiers-and-gating.md)
3. **A tier marker is a prefix on a name segment**, matched as `(^|::)level2_`.
   A module named `level2_*` puts all its tests in L2. A marker for a tier
   whose recipe is a stub strands the test in no tier at all.
   → [tier-filters.md](tier-filters.md)
4. **Run `level2_*` tests only through `just test-l2`.** The recipe owns
   pane spawning, teardown, and serial-versus-parallel mode, and harness
   spawns never steal focus. → [l2-tests.md](l2-tests.md)
5. **Never run a consolidated suite with `cargo test`.** Nextest gives every
   test its own process; `cargo test` shares one across up to a hundred former
   binaries. → [consolidated-binaries.md](consolidated-binaries.md)
6. **Never resolve a path at compile time.** CI runs your binary from an
   archive on another machine: use `biscuit_test_harness::bin_exe!` and
   `manifest_dir!`, not `env!("CARGO_…")`. → [ci-execution.md](ci-execution.md)
7. **Spell a repository file read in a form the CI index can resolve**, or the
   file's next edit will not run your test:
   - `include_str!("../../docs/x.md")`;
   - a root joined in the same expression — `manifest_dir!().join("…")`,
     `repo_root().join("…")`, or a name bound to one in the same file;
   - a full repository-relative literal, in a file that reads through a root.

   A read in a helper schedules its whole binary, and a shared `tests/common`
   module is a helper in every binary that includes it.
   → [ci-execution.md](ci-execution.md)
8. **Spawn the crate's own binary through the area's one command builder**
   (`CliProcessFixture` in claudine and darkmatter), never a raw
   `Command::cargo_bin`. The ambient checkout, `$HOME`, and `PATH` make a test
   slow and host-dependent. → [spawning-binaries.md](spawning-binaries.md)
9. **No retries, no fixed sleeps, no leaked children.** Both nextest profiles
   use `retries = 0` and fail on `LEAK`. Synchronize on the condition you
   assert. → [test-design.md](test-design.md), [recipes.md](recipes.md)
10. **Scope final gates by blast radius**, never by a workspace-wide run, and
    do not add `--no-fail-fast`: CI already passes it.
    → [verification-scope.md](verification-scope.md), [recipes.md](recipes.md)

## Decision Tree: "What tier should my test live in?"

Start at the **requirement**, not the code:

```text
Does the test need a real terminal, browser, or device to verify behaviour?
├── NO  → Is it slow (>5 s) or does it hammer an external API?
│   ├── NO  → L1 (default). Name it normally.
│   └── YES → L1 with `slow_` prefix so sanity skips it.
├── YES → Is it a headless browser test?
│   ├── YES → Browser tier. Name it `browser_*`.
│   └── NO  → Does it need OS keyboard/mouse injection?
│       ├── YES → L3. Name it `level3_*`. Requires RUN_LEVEL3=1.
│       └── NO  → L2. Name it `level2_*`. Requires a harness (tmux/WezTerm/Chrome).
```

If the only meaningful coverage of a public API requires a real resource,
document the exception in `docs/testing-strategy.md`; do not force it into
`sanity`.

| Level   | Prefix     | Resource                                     | Skip when absent     |
|---------|------------|----------------------------------------------|----------------------|
| L1      | (none)     | In-process or hermetic subprocess/filesystem | Never                |
| L2      | `level2_`  | Real terminal / PTY                          | Harness missing      |
| L3      | `level3_`  | OS keyboard/mouse                            | `RUN_LEVEL3` unset   |
| Browser | `browser_` | Chrome/Chromium                              | Browser missing      |
| Real    | `real_`    | External device/API                          | Resource missing     |
| Slow    | `slow_`    | None (slow L1)                               | Excluded from sanity |

Hard-fail switches, per-backend requirements, the backend-proof bracket, and
the full environment contract are in [tiers-and-gating.md](tiers-and-gating.md).

```rust
use test_toolkit::{expect_level, require_level, Backend, Level};

require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm); // L2/L3: skips cleanly
expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");             // L1: panics
```

## Canonical Just Recipes

Every curated package area defines these recipes, delegating to the shared
`_*` recipes in `just/devops.just` (e.g. `@just _test my-crate`):

| Recipe         | Meaning                                                                 |
|----------------|-------------------------------------------------------------------------|
| `sanity`       | Fast confidence (≤15 s). `cargo nextest run --lib --bins -E '!set:slow'`. |
| `test`         | Full L1 suite.                                                          |
| `test-l2`      | Real-terminal tests; serial with a shared pane by default, parallel with `BISCUIT_L2_THREADS=N` — see [l2-tests.md](l2-tests.md). |
| `test-l3`      | OS keyboard/mouse tests.                                                |
| `test-browser` | Headless browser tests, one Chrome at a time (`-j 1`).                  |
| `test-real`    | External resource tests.                                                |
| `lint`         | Clippy + fmt check.                                                     |
| `bench`        | Criterion benchmarks (no-op if opted out).                              |
| `coverage`     | Per-package LCOV (local only; CI produces none).                        |
| `doctest`      | `cargo test --doc`.                                                     |
| `fuzz`         | `cargo +nightly fuzz run` (no-op if no targets).                        |
| `all`          | `sanity → lint → doctest → test → test-l2 → test-browser`.              |

To narrow a recipe to one module, pass a positional filter:
`just test-cli context_command::` — see [tier-filters.md](tier-filters.md).

## Key Crates

| Crate                     | Purpose |
|---------------------------|---------|
| `test_toolkit`            | `require_level!` / `expect_level!`, `EnvGuard`, `trace_phase!` |
| `biscuit_test_harness`    | Terminal harnesses (WezTerm, Kitty, tmux, Apple Terminal), `SharedHarness`, `bin_exe!` / `manifest_dir!`. Load the `biscuit-test-harness` skill for backend selection and the harness API. |
| `biscuit_browser_harness` | Headless Chrome harness (`ChromeHarness`, `require_browser`) |
| `rstest` / `serial_test`  | Fixtures and parameterization / serializing tests that share a real resource |
| `insta` / `pretty_assertions` | Snapshots / readable diffs |
| `criterion`               | Benchmarking |

## Topic Pages

**This repository's contracts:**

| Open when you are…                                                          | File |
|-----------------------------------------------------------------------------|------|
| designing or reviewing any test: assertions, boundaries, timing, performance evidence | [test-design.md](test-design.md) |
| choosing a tier, gating, requiring a backend or real resource, reading env switches | [tiers-and-gating.md](tiers-and-gating.md) |
| naming a test or module, writing a filterset, narrowing a recipe            | [tier-filters.md](tier-filters.md) |
| running or writing `level2_*` tests, or making them parallel-safe           | [l2-tests.md](l2-tests.md) |
| adding a test file to a package with `autotests = false`                    | [consolidated-binaries.md](consolidated-binaries.md) |
| spawning the crate's own binary from an L1 test                             | [spawning-binaries.md](spawning-binaries.md) |
| writing fixtures, env guards, or a pending-contract fixture                 | [fixtures.md](fixtures.md) |
| making a test work from a CI archive, or reading a repository file          | [ci-execution.md](ci-execution.md) |
| choosing which packages to build, test, and lint before reporting done      | [verification-scope.md](verification-scope.md) |
| deciding on fail-fast, restoring sources after a proof, hunting leaked processes | [recipes.md](recipes.md) |
| auditing a whole suite or writing a test-performance spec                   | [test-suite-audits.md](test-suite-audits.md), [test-audit-tooling.md](test-audit-tooling.md) |

**Techniques:**

| Topic | File |
|---|---|
| L2 WezTerm capture gotchas (SGR collapsing, semicolon vs colon form) | [wezterm-harness-pitfalls.md](wezterm-harness-pitfalls.md) |
| L2 Apple Terminal pitfalls (`do script` reuse, focus steal, sentinel waits) | [apple-terminal-harness-pitfalls.md](apple-terminal-harness-pitfalls.md) |
| Browser tests: the headless invariant and computed-style assertions | [browser-testing.md](browser-testing.md) |
| CLI output (channels, color modes, completions, snapshots) | [cli-output-testing.md](cli-output-testing.md) |
| TUI rendering and event/reducer tests | [tui-testing.md](tui-testing.md) |
| Unit, integration, and doc tests | [unit-tests.md](unit-tests.md), [integration-tests.md](integration-tests.md), [doc-tests.md](doc-tests.md) |
| Snapshots and redaction | [snapshots.md](snapshots.md), [snapshot-redaction.md](snapshot-redaction.md) |
| Mocking and property testing | [mocking.md](mocking.md), [property-testing.md](property-testing.md) |
| Fuzzing (nightly only, never a PR gate) | [fuzzing.md](fuzzing.md) |
| Benchmarks: tool choice, then Criterion | [performance-testing.md](performance-testing.md), [criterion.md](criterion.md) |
| Nextest itself | [nextest.md](nextest.md) |

## Resources

- `docs/testing-strategy.md` — human-facing deep dive
- `docs/cicd/test-inputs.md` — why and how a repository file read schedules a test
- `just/devops.just` — shared `_*` lifecycle recipes, `_tier_filter`
- `.config/nextest.toml` — timeouts, leak policy, and retries
- `just check-tier-coverage` — fails when a tier's tests are stranded behind a
  stub `test-<tier>` recipe
