---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 13)

## Summary

The implementation is very close. I verified the current working tree, including the local fixes after Review 12, with:

```text
cargo test -p biscuit-tui -p biscuit-tui-cli
```

That passed on this host. The previous `Ctrl+C` findings appear addressed: `ChooseOne::handle_event` now cancels before Ctrl-hotkey dispatch, and the real-terminal suite includes a Level 2 tmux assertion for `Ctrl+C` exiting `130`.

I found one remaining production blocker around hotkey badge styling, plus the corresponding test-rigor mismatch.

## Findings

### 1. Hotkey badge foreground colours do not match the spec

Severity: high.

The spec requires hotkey badges to show with orange/yellow backgrounds and **white text**:

- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:114`

The implementation intentionally uses black foreground text for both Ctrl and Alt badges:

- `biscuit-tui/lib/src/components/choice_render.rs:35`
- `biscuit-tui/lib/src/components/choice_render.rs:39`
- `biscuit-tui/lib/src/components/choice_render.rs:40`
- `biscuit-tui/lib/src/components/choice_render.rs:43`

The unit tests also lock this in as expected behavior:

- `biscuit-tui/lib/src/components/choice_render.rs:1293`
- `biscuit-tui/lib/src/components/choice_render.rs:1327`
- `biscuit-tui/lib/src/components/choice_render.rs:1369`
- `biscuit-tui/lib/src/components/choice_render.rs:1404`

The comments explain the contrast rationale, and black-on-yellow is defensible from an accessibility standpoint, but it is still a spec change rather than an implementation of the documented requirement. Either update the spec/design/docs to say high-contrast black foreground is the intended badge text color, or change the renderer/tests back to white text.

### 2. Specific hotkey badge colour rendering is only verified at Level 1

Severity: high.

The review rubric explicitly calls out requirements like "`^X` badges with specific colours" as needing Level 2 capture through a real terminal. Current Level 2 and Level 3 tests assert that badge glyphs appear, but they assert against `frame.plain`, not the raw SGR/style output:

- `biscuit-tui/cli/tests/real_terminal_render.rs:115`
- `biscuit-tui/cli/tests/real_terminal_render.rs:134`
- `biscuit-tui/cli/tests/real_terminal_render.rs:199`
- `biscuit-tui/cli/tests/real_terminal_render.rs:226`
- `biscuit-tui/cli/tests/real_terminal_render.rs:232`

The harnesses already capture ANSI escapes (`tmux capture-pane -e`, WezTerm `get-text --escapes`, kitty `--ansi`), but no test asserts that a visible badge arrives with the expected Ctrl/Alt SGR background, foreground, and bold/non-bold distinction. The strongest colour verification is therefore unit-buffer style inspection in `choice_render.rs`, which is Level 1 for this user-observable rendering requirement.

Recommendation: add at least one Level 2 test that spawns `question choose-one "[CTRL+r] Red" "[ALT+b] Blue"`, forces badges visible with `--hotkey-badges ctrl` or sends `Ctrl+Space`, captures `frame.raw`, and asserts the SGR sequence around `^R`/`⌥B` includes the agreed foreground/background/bold styling. If the foreground remains black, make that explicit in the spec first.

## Verification Notes

- `cargo test -p biscuit-tui -p biscuit-tui-cli` passed.
- Completion behavior has Level 1 script tests plus PTY-driven zsh/bash tests.
- Bare modifier badge visibility has Level 2 WezTerm raw-kitty-byte coverage and Level 3 cliclick coverage on this host.
- Hotkey chord activation has Level 3 coverage for `Ctrl+R`.
- `Ctrl+C` abort now has component-level unit coverage and Level 2 tmux coverage.

## Production Readiness

Not ready. The remaining blocker is narrow, but the feature includes a user-facing colour contract for hotkey badges, and the current implementation both differs from the spec and lacks the required Level 2 style verification.
