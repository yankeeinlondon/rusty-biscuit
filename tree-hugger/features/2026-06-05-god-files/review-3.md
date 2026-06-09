---
ready: false
agent: codex
model: ""
---

# Review 3: God Files

## Findings

### High: Level 2 coverage does not verify all specified terminal styling

The new real-terminal tests are a meaningful improvement, but their fixture only produces a high-risk report and `assert_styled_report` checks red, dim, OSC8, and the Unicode glyphs (`cli/tests/level2_god_files.rs:35-49`, `cli/tests/level2_god_files.rs:69-113`). The specification also requires moderate-risk counts and SLOC to render yellow, and section headings to render bold and underlined (`spec.md:249-264`). No Level 2 assertion checks yellow SGR, bold SGR, or underline SGR, and no Level 2 fixture renders a moderate-risk section.

Strongest verification present for those styles: **Level 1** markup/output tests.

Required verification: **Level 2**, because these are user-observable rendering requirements. Extend the real-terminal fixture to include a moderate file and assert the yellow, bold, and underline attributes in raw capture, tolerating backend-specific combined SGR forms.

### High: `just test-l2` bypasses the required serialized harness workflow

The package recipe invokes `cargo nextest run` directly without `-j 1` (`tree-hugger/justfile:41-48`). Nextest runs tests as separate processes, so the `serial_test` annotations in `level2_god_files.rs` do not serialize the tmux, WezTerm, and Kitty tests across those processes. This can spawn or drive multiple terminal panes concurrently, race on GUI/global terminal state, leak panes after failures, and produce nondeterministic results.

The repository testing contract explicitly requires Level 2 tests to run through the broker-backed recipe with one shared pane per backend and nextest `-j 1`; it also says not to invoke the Level 2 filter directly. The comment claiming the harness is unavailable to the workspace is not sufficient justification for dropping those invariants. At minimum this local recipe must serialize with `-j 1` and guarantee cleanup; preferably it should invoke the shared broker using an explicit manifest path if nested-workspace package resolution is the blocker.

Until the Level 2 suite has a reliable canonical execution path, its presence does not provide a dependable production gate.

## Verification Level Matrix

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, thresholds, sorting, and lazy caching | Level 1 unit | Level 1 | OK |
| Effective SLOC, re-filtering, block ranking, call-outs, signals, and hints | Level 1 unit | Level 1 | OK |
| CLI JSON/plain output, filtering, empty scan, invalid roots, and degraded parsing note | Level 1 integration | Level 1 | OK |
| Red/dim styling, OSC8 links, glyphs through a real terminal | Level 2 terminal harness | Level 2 | Covered, but execution recipe is unreliable |
| Yellow moderate styling and bold/underlined section headings | Level 1 only | Level 2 | Gap |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

The targeted library and CLI god-files tests passed. Clippy also passed for `tree-hugger` and `tree-hugger-cli` with all targets and warnings denied. Level 2 tests were not run because the provided recipe violates the repository's required Level 2 execution contract.
