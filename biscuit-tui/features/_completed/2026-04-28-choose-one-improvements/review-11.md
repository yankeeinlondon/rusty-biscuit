---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 11)

## Summary

The implementation is broadly complete and the focused default verification is green:

```text
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

Both commands pass. The prior review items around unsupported `--file` extensions, markdown frontmatter robustness, TOML docs, duplicate hotkey handling, shell completions, and PTY keyboard protocol coverage appear addressed.

I found one remaining blocker in the library event contract: standalone `question choose-one` handles `Ctrl+C` correctly because the runner intercepts it, but the `ChooseOne` component itself does not implement the design's `Ctrl+C` cancellation behavior. Embedded callers that drive `ChooseOne::handle_event` directly can still treat `Ctrl+C` as a selectable hotkey.

## Findings

### 1. `ChooseOne::handle_event` does not treat `Ctrl+C` as abort

Severity: medium.

The spec says `Ctrl+C` is the only way to abort `ChooseOne` without returning a value, and the technical design explicitly puts `Ctrl-C` first in `ChooseOne::handle_event`: return cancellation before filtering, hotkeys, enter, space, navigation, or escape.

The standalone event loop does intercept `Ctrl+C` before delegating to the component:

- `biscuit-tui/lib/src/core/standalone.rs:209`
- `biscuit-tui/lib/src/core/standalone.rs:289`

That makes the CLI path work. The component path is still incomplete. In `ChooseOne::handle_event`, control chords first arm hotkey badge fallback state, and then the normal Ctrl hotkey dispatch can select and submit:

- `biscuit-tui/lib/src/components/choose_one.rs:541`
- `biscuit-tui/lib/src/components/choose_one.rs:623`

There is no component-level `Ctrl+C` guard in `choose_one.rs`, and there is no focused `ChooseOne` unit test for `Ctrl+C`. An embedded app using `HandleEvent` directly can therefore bind/select an option whose effective hotkey is `Ctrl+C` instead of aborting. For example, a `ChoiceOption` labeled `"Cancel"` auto-derives `Ctrl+c`, so `Ctrl+C` can submit `"Cancel"` when the caller expects abort semantics.

Recommendation: add an early `Ctrl+C` branch in `ChooseOne::handle_event` before modifier badge fallback and hotkey dispatch. Keep the runner-level pre-dispatch intercept so standalone prompts still produce `LoopExit::CtrlC` / exit `130`. Add unit coverage that:

- `ChooseOne::handle_event(Ctrl+C)` returns `EventOutcome::Cancelled` or a dedicated interrupt outcome if one is introduced.
- `Ctrl+C` does not select or submit an option whose explicit or auto-derived hotkey is `Ctrl+c`.
- `drive_event_loop` still maps standalone `Ctrl+C` to `LoopExit::CtrlC` / `CANCELLED_KIND`.

## Non-Blocking Notes

- The horizontal layout, active background span width, radio/checkbox glyph policy, ESC restore-submit behavior, hotkey badge rendering, option-source parsing, sorting vocabulary, padding defaults, and completion scripts are all covered by substantial unit and integration tests.
- The docs are mostly aligned. One minor wording cleanup remains in `docs/components/choose_one.md`: the "Enter Behavior" note says Enter selects the hovered item, then says there is no automatic selection on submit. The intended behavior is clear from code and tests, but that sentence should be simplified.
- A CLI `--orientation <vertical|horizontal>` flag would make the new horizontal layout easier to exercise from `question`, but the spec's CLI section did not explicitly require it, so I am treating that as a follow-up ergonomic enhancement rather than a feature gap.

## Production Readiness

Not ready yet. The CLI behavior is in good shape, but `tui-chrome` is also a reusable library, and the component-level `Ctrl+C` contract from the design is not implemented or tested. Fixing that should be small and low-risk.
