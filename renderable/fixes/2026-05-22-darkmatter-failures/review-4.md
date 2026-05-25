---
ready: true
agent: codex
model: ""
---

# Review: Darkmatter Code-Block Rendering Failures

## Findings

### Low: The shared background parser is still narrower than its reusable contract

`biscuit-test-harness/src/layout_invariants.rs:91` only treats extended semicolon SGR backgrounds (`48;5;n`, `48;2;r;g;b`) as background state, and `is_blank` at `biscuit-test-harness/src/layout_invariants.rs:83` only checks for `\x1b[48`. That is enough for the current in-process darkmatter matrix because code blocks emit truecolor semicolon escapes, but it is weaker than the module's stated role as a reusable terminal-layout contract.

If `biscuit-terminal` reuses these predicates against real-terminal captures, WezTerm can re-emit truecolor as colon form (`48:2::r:g:b`), and basic/background SGRs (`40`-`47`, `100`-`107`) are also valid background fills. In those cases `bg_extent` can return `None` and the rectangle/coherence checks can silently skip a background-bearing line, while `is_blank` can classify a background-filled padding row as blank.

This is not a production blocker for the current darkmatter code-block fix, but the harness should grow focused tests for colon truecolor and basic background SGRs before it is treated as a general Level-2/cross-crate invariant engine.

### Low: The idempotence test leaks generated fixtures for convenience

`darkmatter/lib/tests/render_invariants.rs:416` through `:430` uses `Box::leak` to create temporary `&'static str` fixtures inside the shape/scenario loop. The leak is bounded to a test process and does not affect production behavior, but it is avoidable and makes the test helper harder to reuse. A small local render helper that accepts `&str`, or changing `Shape.md` to `Cow<'static, str>`, would keep the same assertions without intentional leaks.

## Test Rigor Classification

- Code-panel rectangle, right-boundary coherence, contrast, and blank-line rhythm: Level 1 invariant coverage exists in `darkmatter/lib/tests/render_invariants.rs`, and Level 2 WezTerm coverage exists in `darkmatter/lib/tests/level2_render_tree_terminal.rs` for the page-path code-panel grid and inter-block blank rows. This matches the user-observable terminal rendering requirements.
- Theme inversion for terminal and HTML: Level 1 deterministic assertions cover terminal SGR/background selection, single-variant no-op behavior, and HTML inversion. HTML computed output is not a terminal-emulator behavior, so Level 1/HTML assertions are appropriate here.
- Mermaid text and forced image fallback: Level 1 coverage is deterministic; `TerminalImageMode::Never` forces the fallback path rather than conditionally skipping it.
- Matrix-wide containment and vertical rhythm: Level 1 coverage now treats render failures as first-class violations and measures terminal cells with `unicode-width`. Level 2 coverage exists for the reported page-path code-panel user-visible behavior, not for every matrix cell.
- No Level 3 requirements found; the spec does not assert OS keyboard-event behavior.

## Verification

Ran:

```bash
cargo test -p biscuit-test-harness layout_invariants --color=never
```

Result: passed, 23 layout-invariant tests.

Attempted:

```bash
cargo test -p darkmatter --test render_invariants --color=never
```

The command was still compiling after roughly 60 seconds, so I stopped it per the non-interactive session constraints. I did not obtain a green darkmatter integration-test result in this review.

## Production Readiness

Ready for production. The previous high-severity review findings are addressed: render failures no longer pass the invariant sweep vacuously, terminal-cell widths are used instead of `char` counts, Mermaid fallback is deterministic, and the contradictory skill heading was corrected. Remaining notes are test-harness cleanup/reuse issues, not blockers for this feature.
