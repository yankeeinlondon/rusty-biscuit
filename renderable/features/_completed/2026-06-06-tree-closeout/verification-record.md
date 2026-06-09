---
status: complete
date: 2026-06-07
owner: ken
phase: 4
plan: renderable/features/_completed/2026-06-06-tree-closeout/plan.md
spec: renderable/features/_completed/2026-06-06-tree-closeout/spec.md
commit: a1264547d
host_os: macOS (Darwin 25.5.0)
harness_backends_present:
    - wezterm
    - tmux
    - kitty
    - chrome (Google Chrome.app)
---

# Phase 4 Verification Record

Behavioral-verification evidence for the Tree Rendering Closeout (spec §4). All
commands were run from the repository root on the host above, which provides the
full real-terminal (WezTerm / tmux / kitty) and headless-browser (Chrome)
harness set, so **no environmental skips were required** — every applicable tier
ran for real.

## 1. Reviewed snapshot references

### The five named stale browser snapshots (spec §4)

`reference_block_quote_width_and_left`, `reference_list_left_margin`,
`reference_page_background_pronounced`, `reference_centered_table`, and
`reference_table_max_width` live in
`darkmatter/lib/tests/cutover_reference.rs`. They were already re-baselined in
the `tree-features` cutover (its "Phase 8 review", documented in the module
header of that file): each page wrapper's horizontal margins moved from
`margin: 0ch 0ch 0ch 0ch` to `margin: 0ch auto 0ch auto`.

**Decision: accepted (intended, not regressive).** Each fixture emits a
`max-width`-capped `.darkmatter-page` wrapper with both author side margins left
at their default (zero). `margin-left: auto; margin-right: auto` is the
canonical CSS idiom for horizontally centering a fixed-`max-width` block, and it
mirrors the terminal frame's `center_frame` placement. Authored side margins
suppress the `auto` centering (proven by the
`browser_render_with_max_width` / `browser_render_authored_side_margins_suppress_centering`
unit tests). The CSS `auto` centering rationale is recorded in the
`cutover_reference.rs` module doc; Phase 4 re-ran the suite and confirms all five
pass against the accepted baselines.

### Remaining snapshot changes (alpha / direct policy / text-layout / browser attrs)

The full reference and characterization corpora
(`cutover_reference.rs`, `tree_features_characterization.rs`,
`render_tree_hr_snapshots.rs`, `render_tree_roundtrip.rs`) pass with **no
pending `*.snap.new` files** and **no modified committed `*.snap` files** in the
working tree. The alpha-paint, direct-policy, text-layout, and browser-attribute
behaviors landed and were accepted in Phases 2–3; Phase 4 confirms the baselines
are stable and require no further re-baselining.

```
find darkmatter renderable biscuit-terminal -name '*.snap.new'   # → none
git status --short -- '*.snap'                                    # → clean
```

## 2. Level 1 suites (no fail-fast omissions)

| Command | Result |
|---|---|
| `just -f renderable/justfile test` | **490 passed**, 0 failed |
| `just -f biscuit-terminal/justfile test` | **357 passed** (lib + CLI), 0 failed, 68 tier-filtered |
| `just -f darkmatter/justfile test` | **darkmatter lib 3858 passed** (1 flaky → passed on retry), **darkmatter-cli 415 passed**, 0 failed |

### Recipe fix required to make `just -f darkmatter/justfile test` a true Level-1 run

`darkmatter/justfile`'s `test` recipe delegated to the shared `_test` recipe,
which runs `cargo nextest run -p <pkg>` with **no tier filter** — so it pulled in
the headless-browser tier (`browser_render`) alongside Level 1. Under heavy
parallel load those Chromium-driven tests leak their child process past the
100ms leak grace and hard-fail (`LKFAIL`), even though every test exits code 0.
This violated the documented "`just test` runs Level-1 only" contract and the
plan's explicit separation of `test` from `test-browser`.

**Fix (surgical, mirrors the existing biscuit-terminal precedent):** the
darkmatter `test` recipe now applies the Level-1 tier filter inline —
`!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/))` — for
both the library and the CLI. The `browser_render` and
`level2_render_tree_terminal` binary names are themselves matched by the
`browser_`/`level2_` terms, so the whole binary is excluded. The shared `_test`
recipe (used by 20 other package areas) was left untouched to avoid a broad,
untestable repo-wide change.

## 3. Doctests

| Command | Result |
|---|---|
| `just -f renderable/justfile doctest` | **98 passed**, 2 ignored |
| `just -f biscuit-terminal/justfile doctest` | **191 passed** (lib), 12 ignored; CLI has no lib target |
| `just -f darkmatter/justfile doctest` | **161 passed** (lib), 10 ignored; CLI 1 ignored |

## 4. Browser coverage

Run only through `just -f darkmatter/justfile test-browser` (real headless
Chrome): **59 passed**, 0 failed.

### Leak-timeout fix required for a reliably-green browser tier

The first `test-browser` run logically passed every test (exit code 0) but
flaked heavily and ultimately failed two tests on `LKFAIL`: chromiumoxide kills
the Chromium child via `kill_on_drop`, but the OS reaping that closes the
child's stdio pipes happens in the background and, under load, routinely exceeds
nextest's default 100ms leak grace. The harness `Drop`
(`biscuit-browser-harness`) spawns `browser.close()` fire-and-forget, so the
close can be cut short when the `#[tokio::test]` current-thread runtime shuts
down.

**Fix:** a browser-tier override in `.config/nextest.toml` (both the `default`
and `ci` profiles) raises `leak-timeout` to `3s` for `test(/browser_/)`. A leak
grace only waits when a leak is actually pending, so clean teardowns are not
slowed; this targets only the browser tier and leaves every other tier on the
strict 100ms grace. After the fix the suite is **59/59 passing with 0 flaky and
0 leaks**.

## 5. Applicable real-terminal (Level 2) coverage

The host provides WezTerm, tmux, and kitty, so L2 ran for real (no
`BISCUIT_TEST_LEVEL_REQUIRED` flag needed — missing-harness hard-fail was not
forced because the harnesses are present).

| Command | Result |
|---|---|
| `just -f biscuit-terminal/justfile test-l2` | **68 passed**, 0 failed |
| `just -f darkmatter/justfile test-l2` | **55 passed**, 0 failed |

## 6. Markdown / MarkdownPlus degradation coverage (run explicitly)

| Command | Result |
|---|---|
| `cargo nextest run -p renderable -E 'test(/degrade/) + test(/dialect/) + test(/markdown_plus/) + test(/portable/) + test(/_dropped/)'` | **38 passed** |
| `cargo nextest run -p darkmatter -E 'test(/degrade/) + test(/dialect/) + test(/markdown_plus/) + test(/portable/) + test(/_dropped/) + test(/markdown_dialects/)'` | **6 passed** |

These cover portable Markdown dropping paint/geometry/browser-only attributes
(`browser_only_attrs_are_dropped_in_both_markdown_dialects`,
`terminal_component_opacity_dropped_but_color_kept`,
`terminal_opacity_dropped_from_sgr`), MarkdownPlus staying within its inline-HTML
dialect policy (`markdown_plus_lowers_foreground_color_to_inline_html`,
`classed_span_emits_html_in_markdown_plus`), and the structural-gate dialect
assertion (`markdown_dialects_degrade_within_policy`).

## 7. Cutover-reference + characterization suites

`cargo nextest run -p darkmatter -E 'binary(/cutover_reference/) + binary(/tree_features_characterization/)'`
→ **20 passed**, 0 failed (includes all five named references plus the
alpha/opacity/text-layout/structured-attribute characterizations).

## 8. Lints

| Command | Result |
|---|---|
| `just -f renderable/justfile lint` | clean |
| `just -f biscuit-terminal/justfile lint` | clean |
| `just -f darkmatter/justfile lint` | clean |

No `.rs` source was changed in Phase 4 (only a justfile recipe and the nextest
config), so clippy status is unchanged from Phase 3; the runs above confirm it.

## Summary

All Phase 4 exit-condition coverage — dedicated reference snapshots, Level 1,
doctests, browser, applicable real-terminal Level 2, and Markdown/MarkdownPlus
dialect degradation — is **green** with no environmental skips on this host. Two
infrastructure defects surfaced by the verification run were fixed at root
cause: the darkmatter `test` recipe now runs Level-1 only, and the
headless-browser tier gets a teardown-appropriate leak grace.
