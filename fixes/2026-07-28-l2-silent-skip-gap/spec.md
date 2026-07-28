---
status: ready for implementation
created: 2026-07-28
area: repo-wide
packages:
  - test-toolkit
discovered-by: darkmatter/fixes/2026-07-27-alias-resolution-hang
---

# L2/L3 Harness Skips Report Success When Nothing Ran

## Summary

A test tier that skips every test still exits `0` and reports those tests as
**passing**.

**`require_level!` skips by `return`ing from the test function, so nextest
scores every silent skip as a passing test.** Measured on one host, one test,
varying only whether `tmux` is on `PATH`:

| `tmux` on `PATH` | nextest result | Wall clock |
|---|---|---|
| hidden | `1 passed`, exit 0 | **0.004 s** |
| present | `1 passed`, exit 0 | **7.981 s** |

The tier is green either way and the counts are identical. Only elapsed time
distinguishes "ran" from "did nothing". Whole packages behave the same way:
darkmatter's L2 tier reports `18 tests run: 18 passed` in **0.138 s** without
tmux and `18 tests run: 18 passed` in **13.28 s** with it.

Note that no `✅ all tests passing` banner is involved — that banner belongs to
the L1 `_test` recipe, not the L2 path, and an earlier draft of this spec
conflated the two. The L2 tier is quieter than that and still wrong: it simply
reports passes.

This is the crux of the defect, and it constrains the fix: **asserting that the
run count is non-zero does not close the gap.** A zero-run assertion catches
only the case where nextest *selects* nothing. The case that actually shipped a
bug is N tests selected, all silently skipped, reported as N passed.

That is a gate reporting success for work it never performed. It was found
while fixing `darkmatter/fixes/2026-07-27-alias-resolution-hang`: the L2
regression test written to prove that hang would have skipped silently on the
developer host, and the tier would have stayed green with the bug live.

## The enforcement mechanism exists and is dead

`test_toolkit::evaluate_level` already converts an unavailable harness into a
panic when `BISCUIT_TEST_LEVEL_REQUIRED` matches the level.

**Nothing in the repository sets it.** Every occurrence across 30 files is a
doc comment. The single non-test mention is a comment at `just/devops.just:452`
stating that the variable exists.

### Why it is unset: the granularity is wrong

`.github/workflows/_area-ci.yml:228-234` records the reason:

> We hard-require the provisioned backend by VERIFYING tmux is reachable rather
> than setting a global `BISCUIT_TEST_LEVEL_REQUIRED`, which would panic the
> GUI-backed tests this runner cannot host.

`BISCUIT_TEST_LEVEL_REQUIRED` is a single switch over a **whole level**, but a
level contains tests on **heterogeneous backends**. `darkmatter` declares
`backends: ["tmux", "wezterm"]`; headless CI can host tmux but not WezTerm,
whose `available()` needs a live `WEZTERM_UNIX_SOCKET`. Turning the switch on
fails tests that are *correctly* skipping.

The mechanism is unusable as designed, so it is unused. Enforcement granularity
must match skip granularity: **per backend, not per level.**

## Scope of exposure

| Fact | Value |
|---|---|
| `level2_*.rs` / `level3_*.rs` files | 69 |
| Areas with `l2: true` | 5 — biscuit-terminal, darkmatter, claudine, biscuit-icon, worktree |
| `require_level!` call sites labelled `"tmux"` | 239 |
| Repository locations setting a required level | 0 |

## What is already correct — do not regress it

CI's tmux path is sound and must keep working:

- `_area-ci.yml` provisions tmux and verifies reachability with a named
  `tmux -V` step.
- `TmuxHarness::available()` is exactly `which("tmux")`, the same predicate, so
  that verification is equivalent to the runtime gate.
- GUI backends (WezTerm, Kitty, Apple Terminal) **must continue to skip
  cleanly** in headless CI. Any fix that fails them is worse than the gap.

The exposure that bit us is therefore **local developer runs**, plus any skip
arising from a cause other than a missing binary.

## Requirements

### R1 — Enforcement must be per backend

A required-backend set must name individual backends. Declaring `tmux` required
must not affect a WezTerm-labelled test.

### R2 — A tier that executed nothing must not report success

`just test-l2` / `test-l3` must not print a success banner or exit `0` when the
run executed no tests.

Per the Summary, a run-count assertion is the floor, not the fix: silent skips
are counted as passes, so the recipe must additionally establish that at least
one declared backend is actually hostable — the local analogue of CI's
`tmux -V` hard-require — or set `BISCUIT_REQUIRED_BACKENDS` itself so silent
skips become panics through R1's mechanism. Reusing R1 is preferred over adding
a second, parallel guard.

### R3 — CI's required set must derive from declared data

`.github/ci/areas.json` already declares `backends` per area. The required set
must be computed from it intersected with what the runner can host, so the two
cannot drift. It must not be a hand-maintained literal in a workflow file.

### R4 — GUI-backed tests must still skip cleanly in headless CI

R1's mechanism must leave WezTerm/Kitty/Apple-Terminal tests skipping on a
headless runner.

### R5 — No call-site churn

239 `require_level!` sites already pass the backend name as `harness_label`.
The fix must consume that, not require editing every site.

## Design

### `BISCUIT_REQUIRED_BACKENDS`

A comma-separated, case-insensitive list of backend names, e.g.
`BISCUIT_REQUIRED_BACKENDS=tmux`.

In `evaluate_level`, when `available == false`, before returning `Skip`: if the
normalized `harness_label` matches any entry in the set, return `Panic`.

Vocabulary is `KNOWN_L2_BACKENDS` from `scripts/ci/affected_scope.py`:
`{tmux, wezterm, kitty, apple-terminal}`.

**Normalization.** Labels are free-form today:

| Label | Count | Backend |
|---|---:|---|
| `tmux` | 239 | tmux |
| `PTY (/dev/ptmx)` | 30 | — |
| `WezTerm` | 8 | wezterm |
| `nvim` | 2 | — |
| `kitty` | 1 | kitty |
| `bash-family shell` | 1 | — |
| `window capture (screen recording permission)` | 1 | — |

Match on the lowercased label being equal to a required entry. `"WezTerm"` →
`wezterm` matches; `"PTY (/dev/ptmx)"` matches nothing, so non-backend
capability probes are never enforced by this switch. Exact match on the
lowercased label, not substring — substring matching would make
`"bash-family shell"` collide with a future `sh` backend.

Labels that are not in the vocabulary keep today's skip behavior. That is
deliberate: this switch governs *backends*, and a capability like a controlling
PTY needs its own decision.

`BISCUIT_TEST_LEVEL_REQUIRED` is retained. The two are independent and either
may fire.

### Enforcement points

1. **`tools/test-toolkit`** — the `BISCUIT_REQUIRED_BACKENDS` const,
   `evaluate_level` logic, module docs, unit tests. (R1, R4, R5)
2. **`just/devops.just`** — `_test_l2` / `_test_l3` treat a zero-run outcome as
   failure rather than printing the success banner. (R2)
3. **`.github/workflows/_area-ci.yml`** + `scripts/ci/affected_scope.py` —
   export the required set from `areas.json ∩ hostable`. On `ubuntu-latest`
   that is `{tmux}`. (R3)

## Testing Contract

1. **`evaluate_level` unit tests.** Required backend absent → `Panic`;
   required backend present → `Run`; a *different* backend's label absent →
   `Skip` (this is R4, and is the test that would have caught the original
   design flaw); unknown label with a non-empty required set → `Skip`;
   case-insensitivity (`WezTerm` vs `wezterm`); empty/unset var → today's
   behavior exactly.
2. **Zero-run guard.** Invoke the L2 recipe with a `PATH` that hides `tmux` and
   assert a non-zero exit and no success banner.
3. **CI derivation.** Extend `scripts/ci/test_affected_scope.py` for the
   intersection: an area declaring `["tmux","wezterm"]` on a headless runner
   yields `tmux` only.
4. **Regression floor.** With `BISCUIT_REQUIRED_BACKENDS=tmux` on a host with
   tmux, `just test-l2` for darkmatter still passes, and WezTerm-labelled tests
   still skip rather than panic.

## Acceptance Criteria

- [ ] `just test-l2` on a host without tmux exits non-zero and prints no
      success banner.
- [ ] `BISCUIT_REQUIRED_BACKENDS=tmux` panics tmux-labelled tests when tmux is
      absent, and leaves WezTerm-labelled tests skipping.
- [ ] CI exports the required set derived from `areas.json`, not a literal.
- [ ] The 5 L2-enabled areas' CI stays green with GUI backends still skipping.
- [ ] No `require_level!` call site is edited.

## Out of Scope

- Making non-backend capability probes (`PTY (/dev/ptmx)`, `nvim`,
  `bash-family shell`, window capture) enforceable — they need their own
  vocabulary decision.
- Provisioning GUI backends in CI.
- The two pre-existing `level2_code_block_styling` capture flakes in darkmatter.
- Retiring `BISCUIT_TEST_LEVEL_REQUIRED`.
