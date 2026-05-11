---
ready: false
agent: claude
model: ""
---

# Review 1 — `2026-05-02-flattened-bridge`

## Summary

The implementation is a credible, partial fix for the **OSC 11 color-query bleed**
in non-interactive Claudine sessions. Phases 2 (`biscuit-terminal` `color_mode`
caching), 3 (`darkmatter` `TerminalOptions` cache), 4 (`claudine`
`crate::log::terminal()` plain-mode rebuild), and 5 (`run_provider_wrapper_inner`
deferred construction) all landed and reduce the symptom from "10–50 OSC
queries per minute" to **one cached query per process** in the worst case.

However, the feature is **not production ready**:

1. The `spec.md` in this directory **describes a completely different
   feature** (hook `when` ↔ `EventMetaExpressionLookup` unification). It does
   not match `plan.md`, `baseline.md`, the commits, or the `SKILL.md` update.
   This is a documentation-integrity issue that must be fixed before closure.
2. The user-observable requirement — "no OSC 11 escape codes appear in
   non-interactive Claudine output under a PTY" — is verified only at
   **Level 1 (in-process unit tests)** and one **manual** Level 2 script
   (`reproduce.sh`) that is not wired into `cargo test`/CI.
3. `reproduce.sh` only exercises `--dry-run`, which short-circuits before any
   child is spawned. The post-spawn streaming path
   (`exec/spawn.rs:739`, `exec/watchdog.rs:194`, `exec/watchdog.rs:554`,
   `live_semantic_sink/mod.rs:198,278`, `composition/summary.rs:41,49`,
   `composition/mod.rs:435`, `policy.rs:313`) still calls
   `crate::log::terminal()` without non-interactive awareness, so a real
   non-interactive run with a spawned child will still issue **one** OSC
   query under a TTY-stdout PTY. The reproduction script does not exercise
   this path.
4. The branch carries unrelated changes (`biscuit-speaks/{cache,types}.rs`,
   `biscuit-terminal/.../table.rs` re-export refactor, plus the `SKILL.md`
   line) that should not bundle with this feature's commit.

## Findings

### High — Spec/plan mismatch (documentation integrity)

`features/2026-05-02-flattened-bridge/spec.md` is titled "Unify the Hook
`when` Lookup with `EventMetaExpressionLookup`" and describes a refactor of
`dispatch::runner::evaluate_when` that has **nothing to do** with the
implementation in this branch.

- `baseline.md` is titled "Phase 1 Baseline: Terminal Escape Code Bleed
  Diagnosis"
- `plan.md` is titled "Execution Plan: Fix Terminal Escape Code Bleed in
  Non-Interactive Sessions"
- All four branch commits and the four merged-to-main commits
  (`70985cb8`, `0144cdf5`, `14d3f126`, `87bcbde5`, plus `e6f35ecf`,
  `18ae8486`) are about the OSC 11 bleed.
- The `SKILL.md` paragraph for this feature describes the OSC 11 fix.

The `spec.md` has been imported from a different feature and never replaced.
**Action:** either replace `spec.md` with the actual problem statement
(escape-code-bleed) or move the existing spec content under its proper
feature directory. This must be resolved before closure because acceptance
criteria are evaluated against the spec.

### High — Level-2 verification is manual and incomplete

The user-observable contract is "OSC 11 escape codes do not appear in
non-interactive output when stdout is a PTY." Per the review framework, this
is a **Level 2** requirement (the bug only manifests under a real terminal
emulator's input/output stream, not in a unit test).

Current verification:

| Surface | Test level present | Test level required |
|---|---|---|
| `terminal()` returns plain terminal under `NO_COLOR` | Level 1 (`log::tests::no_color_disables_terminal_styling`) | Level 1 ✓ |
| `terminal()` returns optimistic terminal under `FORCE_COLOR` | Level 1 (`log::tests::force_color_enables_optimistic_terminal_for_non_tty_runs`) | Level 1 ✓ |
| `optimistic_terminal()` returns plain in plain mode | Level 1 (`log::tests::plain_mode_overrides_force_color`) | Level 1 ✓ |
| `Terminal::new().color_mode()` returns without panic | Level 1 (`integration::test_terminal_dimension_methods`) | Level 1 ✓ |
| **No OSC 11 bytes appear in stdout when running non-interactively under a PTY (`--dry-run`)** | Level 2 manual (`reproduce.sh`, not in CI) | **Level 2 automated — gap** |
| **No OSC 11 bytes appear in stdout when running non-interactively under a PTY (with a real spawned child)** | **None** | **Level 2 automated — gap** |
| `BG_COLOR_CACHE` `OnceLock` actually short-circuits a second `bg_color()` call | None | Level 1 — gap |
| `Terminal` struct's cached `color_mode` field is read by `term.color_mode()` instead of querying again | None (rely on visual code review) | Level 1 — easy gap to close |

The `pty_tests.rs` harness already exists and is `#[ignore]`d as
"timing-sensitive." That harness, or a sibling using `expectrl`/`script`,
should be extended with a test that:

1. Runs `claudine codex --dry-run "hi"` (and a spawning variant) under a PTY
2. Captures stdout
3. Asserts neither `\x1b]11;?` (the query) nor `\x1b]11;rgb:` (the response)
   appears

`reproduce.sh` already contains the assertion logic and could be lifted into
a `#[cfg(unix)] #[ignore]`-gated test that CI opts into via an env flag, the
same way `RUN_LEVEL3=1` gates keyboard-injection tests elsewhere.

### High — Phase 5 fix does not cover the streaming path

Phase 5's claim is: "delays terminal construction until
`effective_non_interactive` is known, then uses
`crate::log::optimistic_terminal(None)` for non-interactive wrapper output
so dry-run / preflight rendering under a PTY cannot issue OSC 11 probes."

This is true for the **pre-spawn** wrapper output. After spawn, eight call
sites still create terminals via `crate::log::terminal()` without consulting
the non-interactive flag:

```
claudine/cli/src/commands/wrap/exec/spawn.rs:739
claudine/cli/src/commands/wrap/exec/watchdog.rs:194
claudine/cli/src/commands/wrap/exec/watchdog.rs:554
claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs:70 (helper)
  → 198, 278 (callers)
claudine/cli/src/commands/wrap/policy.rs:313
claudine/cli/src/commands/wrap/composition/summary.rs:41, 49
claudine/cli/src/commands/wrap/composition/mod.rs:435
```

In a non-interactive run **without** `NO_COLOR`/`--plain`/`FORCE_COLOR`,
`crate::log::terminal()` falls through to `Terminal::new()`, which calls
`color_mode()` → `bg_color()` → `query_osc_actual(11, …)`. The new
`BG_COLOR_CACHE` `OnceLock` ensures only **one** OSC query fires per
process, but that one query is still emitted to stdout the first time
`Terminal::new()` is reached after spawn, defeating the bleed-prevention
goal for any user who hasn't opted into `NO_COLOR`.

**Recommended fix:** either (a) push a non-interactive-aware constructor
into `crate::log` (e.g. an internal `set_non_interactive(true)` flag set
once `effective_non_interactive` is known, that `terminal()` consults
alongside `colors_disabled()`), or (b) replace each unmodal call site with a
helper that takes the resolved interactivity state. Option (a) is preferred
because it propagates without per-call-site plumbing, and because
`set_plain()` already follows that pattern.

`reproduce.sh` will not catch this regression because `--dry-run`
short-circuits at `mod.rs:858` before any of the listed call sites runs.

### Medium — `TerminalBuilder::build()` still triggers detection regardless of overrides

`biscuit-terminal/lib/src/terminal.rs:696` calls `new_terminal()`
unconditionally inside `TerminalBuilder::build()` to obtain "detected"
defaults, then overlays the builder's `Some(_)` fields. This means
`Terminal::builder().is_tty(false).color_depth(None).color_mode(Dark).build()`
**still issues an OSC 11 query** because `new_terminal()` calls
`color_mode()` regardless of which fields are about to be overridden.

The Phase 4 commit acknowledged this by replacing the old builder-based
path in `claudine/cli/src/log.rs::terminal()` with a hand-rolled
`plain_terminal()` helper that mutates `Terminal::new_optimistic(width)`
fields directly. That works, but it leaves the builder API a footgun for
every other caller. Eight test files in `claudine/lib/src/stream/thinking.rs`,
two in `darkmatter/lib/src/markdown/output/terminal.rs`, etc. still use
`Terminal::builder().build()` — each one issues a real OSC query in test
processes (now mitigated by the `OnceLock`, but still).

**Recommended fix:** make `TerminalBuilder::build()` short-circuit
`new_terminal()` if all detection-derived fields are explicitly set, or
introduce a `Terminal::new_blank()` baseline that the builder uses instead
of `new_terminal()`. Alternatively, document that the builder is only safe
for tests that don't run under PTY.

### Medium — `darkmatter` cache vs. fresh-detection inconsistency

Commit `0144cdf5` introduces two contradictory behaviors in `darkmatter`:

- `TerminalOptions::default()` now caches `color_mode` once via
  `static DETECTED_COLOR_MODE: OnceLock<ColorMode>`
  (`darkmatter/lib/src/markdown/output/terminal.rs:756–765`)
- `YamlBlock::render()` now detects fresh on every render
  (`darkmatter/lib/src/markdown/yaml_block.rs:170–172`) with the comment
  "so env-var changes between renders are honoured (e.g. dark/light tests
  that flip COLORFGBG)"

Either env-var changes matter or they don't. If they matter, the
`TerminalOptions::default()` cache is wrong and tests that flip
`COLORFGBG` after the first call will see stale results. If they don't
matter, the `YamlBlock::render()` fresh detection should also be cached.

This is acceptable for the bleed fix only because `darkmatter`'s
`detect_color_mode()` is not OSC-based (it's `NO_COLOR`/`COLORFGBG` env
inspection per `baseline.md`), so neither path bleeds. But the inconsistency
will surprise the next reader. Pick one strategy and document it.

### Medium — `BG_COLOR_CACHE` is a process-global `OnceLock<Option<…>>` with no test reset

`biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:72` declares:

```rust
static BG_COLOR_CACHE: OnceLock<Option<RgbValue>> = OnceLock::new();
```

This means:

- The first test to call `bg_color()` (or any code that reaches it via
  `Terminal::new()`) seeds the cache for the whole process.
- Subsequent tests that try to verify `bg_color()` re-detection (e.g.
  changing `COLORFGBG` and re-detecting) will fail silently.
- Parallel `cargo test` runs share state across the test binary's threads,
  but at least it's idempotent.

This is acceptable for production but precludes Level-1 testing of the
caching behavior itself. Consider adding a `#[cfg(test)] fn reset_cache()`
or wrapping in a `Mutex<Option<Option<RgbValue>>>` if testing the cache
contract becomes important.

### Medium — Branch contains drift unrelated to this feature

The uncommitted working tree has changes in:

- `biscuit-speaks/lib/src/cache.rs` and `biscuit-speaks/lib/src/types.rs` —
  refactor that swaps `installed.say()` / `installed.echogarden()` etc. for
  a typed `installed.is_installed(TtsClient::Say)` API. Real refactor, but
  unrelated to OSC bleed.
- `biscuit-terminal/lib/src/components/table/table.rs` — `pub use` re-exports
  for `TableCellContent` and `TableColumn` plus minor formatting.
  Unrelated to OSC bleed.

These should be split into their own feature/commit. They are not described
by `spec.md`, `plan.md`, or any phase. If they were intentional follow-ups
discovered during implementation, they belong in a separate commit so the
history reflects the scope.

### Low — `crate::log::terminal()` ergonomics

`terminal()` now has three branches with subtle ordering:

```rust
pub fn terminal() -> Terminal {
    if colors_disabled() {           // plain mode or NO_COLOR → no OSC
        plain_terminal(forced_width(80))
    } else if force_color_enabled() { // FORCE_COLOR → no OSC
        Terminal::new_optimistic(forced_width(80))
    } else {
        Terminal::new()              // ← can issue ONE OSC
    }
}
```

The third branch is the only one that can bleed. As called out in the
"Phase 5 doesn't cover streaming" finding, a fourth condition for
non-interactive mode would close the gap:

```rust
pub fn terminal() -> Terminal {
    if colors_disabled() {
        plain_terminal(forced_width(80))
    } else if force_color_enabled() {
        Terminal::new_optimistic(forced_width(80))
    } else if non_interactive_mode() {  // ← consult a global flag
        plain_terminal(forced_width(80))
    } else {
        Terminal::new()
    }
}
```

The `set_plain` / `is_plain` pair already provides the precedent for a
process-global flag with `AtomicBool`. Adding `set_non_interactive` /
`is_non_interactive_session` is a small addition that would let every
existing call site (`spawn.rs:739`, `watchdog.rs:194,554`, etc.) get the
fix without per-site plumbing.

### Low — `Terminal::clone()` cost grew slightly

The new `color_mode: ColorMode` field is `Clone`, so `let termination_term =
stderr_term.clone();` (added in `spawn.rs:740`) is fine. No action; noting
because the `Terminal` struct is now cloned in a hot-ish path.

### Low — Docs: `terminal()` rustdoc is stale

`claudine/cli/src/log.rs:44–49` says:

> Returns a [`Terminal`] appropriate for the current mode.
>
> In plain mode, returns a terminal with `is_tty: false` and
> `color_depth: None` so components render with correct alignment but no
> ANSI escape codes. In normal mode, returns a standard detected terminal.

This omits the `color_mode = Dark` clamp added by `plain_terminal()` and
the OSC-suppression rationale. Update to mention that plain mode also avoids
OSC 11 probes — that's the point of the rebuild — and that "normal mode"
will issue exactly one OSC query per process due to `BG_COLOR_CACHE`.

### Low — `Terminal::color_mode()` instance method comment

`biscuit-terminal/lib/src/terminal.rs:485–502` comment now says:

> Returns the cached color mode for this terminal instance.
>
> The value was detected once during construction via OSC heuristics
> and is cached to avoid repeated terminal queries.

Strictly this is true only for `Terminal::new()`. For `Terminal::builder()
.color_mode(X).build()`, the cached value comes from the override; for
`Terminal::new_optimistic()`, the cached value is `ColorMode::Dark`
unconditionally. The doc reads as if it always reflects detection. A line
or two clarifying that the field is "the value at construction time
(detected, overridden, or defaulted, depending on constructor)" would help.

## Test Rigor Classification

| User-observable requirement | Strongest test level present | Required level | Status |
|---|---|---|---|
| "No OSC 11 bytes appear in non-interactive PTY output during `--dry-run`" | Level 2 manual (`reproduce.sh`) | Level 2 automated | **gap** |
| "No OSC 11 bytes appear in non-interactive PTY output during a real spawned-child run" | None | Level 2 automated | **gap** |
| `terminal()` returns the right shape under `NO_COLOR` / `FORCE_COLOR` / plain mode | Level 1 unit tests (`log::tests`) | Level 1 | ✓ |
| `term.color_mode()` returns the cached field, not a fresh detection | None (visual review only) | Level 1 | gap |
| `BG_COLOR_CACHE` short-circuits the second call | None | Level 1 | gap |

Two requirement rows match the framework's "high severity" criterion:
"Spec requires X happens at the terminal byte level" + only Level-1 unit
tests = gap, not "ready".

## Acceptance Criteria Status (against `plan.md`, since `spec.md` is wrong)

| Plan acceptance criterion | Status |
|---|---|
| Phase 1 — Diagnosis & baseline | ✅ Complete (`baseline.md`, `reproduce.sh`) |
| Phase 2 — `Terminal` caches `color_mode`; instance method; `OnceLock` for `bg_color()` | ✅ Complete (`70985cb8`) — but builder still detects internally |
| Phase 3 — `darkmatter` `TerminalOptions::default()` caches `color_mode` | ✅ Complete (`0144cdf5`) — inconsistent with `YamlBlock::render` |
| Phase 4 — `claudine` `log::terminal()` non-OSC path; cache terminal in hot paths | ⚠ Partial — `log.rs` correct; many call sites still unmodal |
| Phase 5 — full test suite + reproduction verification | ⚠ Partial — `cargo test -p claudine-cli` not re-run with the live diff; `reproduce.sh` only covers `--dry-run`; no regression test added; interactive regression check not documented |

## Recommended Closure Steps

1. **Fix the `spec.md` mismatch.** Either replace it with the real
   escape-code-bleed spec or move it to its proper feature folder. Without
   this, the feature's acceptance contract is undefined.
2. **Add a process-level non-interactive flag** to `claudine::log` (mirroring
   `set_plain`/`is_plain`) and consult it in `terminal()`. Set it once in
   `run_provider_wrapper_inner` after `effective_non_interactive` resolves.
   This closes the post-spawn gap with one knob.
3. **Wire `reproduce.sh`'s assertion into an `#[ignore]`d PTY integration
   test** under `claudine/cli/tests/pty_tests.rs` (or a new file) using
   `expectrl::Session` to capture stdout. Gate on `RUN_PTY=1` if needed for
   stability. Add at least two cases: `--dry-run` (matches today's
   reproduce) and a real spawn against a stubbed child binary
   (matches the gap above).
4. **Resolve the `darkmatter` cache vs. fresh-detection inconsistency.**
   Either cache uniformly or detect uniformly; pick one and update the
   comments. (The bleed fix doesn't depend on this, but it's a trap.)
5. **Split unrelated drift** (`biscuit-speaks` and the `table.rs` re-export
   refactor) into their own commit/feature.
6. **Update `crate::log::terminal()` rustdoc** to reflect the OSC-avoidance
   contract and the once-per-process query in normal mode.
7. **Optional: harden `TerminalBuilder::build()`** so explicitly-set fields
   skip detection. This is a follow-up; not blocking.

## Verdict

`ready: false`. The fix is on the right track and the worst-case bleed is
already reduced from "many per minute" to "one per process," but the
implementation does not fully close the user-observable contract for the
streaming path, and the verification surface is below what the framework
requires for a UX bug like this. Closure should pull in items 1–3 at a
minimum.
