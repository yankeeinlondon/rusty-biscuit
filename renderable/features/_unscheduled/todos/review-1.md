---
ready: false
agent: codex
model: ""
---

# Review: Todos

## Findings

### Critical: Extended percentage and blocked Markdown markers are not parsed

The specification requires `- [ {#}% ]` progress markers and `- [ ! ]` blocked
markers ([spec.md](spec.md:21)). The render-tree state can only represent
`Open`, `InProgress`, `Completed`, `Blocked`, and `Cancelled`; it has no progress
value or quarter states
([attrs.rs](../../../src/tree/attrs.rs:220)). Darkmatter delegates task
recognition exclusively to pulldown-cmark's boolean `TaskListMarker` event and
stores only `Option<bool>`
([fold.rs](../../../../darkmatter/lib/src/markdown/render_tree/fold.rs:207),
[fold.rs](../../../../darkmatter/lib/src/markdown/render_tree/fold.rs:779)). Its
task fixture covers only `[x]` and `[ ]`
([task_list.md](../../../../darkmatter/lib/tests/fixtures/render_tree/task_list.md:1)).

As a result, `[25%]`, `[50%]`, `[75%]`, and `[!]` remain ordinary item text and
never acquire `TaskHints`, state-aware terminal markers, or browser state
presentation. Add an explicit preprocessing/parser extension that recognizes
the complete marker grammar before or during the tree fold, preserve the
progress value in a typed task state, and test valid, invalid, nested, ordered,
and whitespace variants end to end.

### High: Todo activation is not automatic for list components

The specification says the feature activates automatically for
`UnorderedList`, `OrderedList`, and Darkmatter Markdown
([spec.md](spec.md:16)). Both list components project every item through the
same helper and always construct `ListItem { checked: None }`, without inspecting
the item for task syntax or preserving a nested `Todo` projection
([list.rs](../../../../biscuit-terminal/lib/src/components/list.rs:424),
[list.rs](../../../../biscuit-terminal/lib/src/components/list.rs:443)). The
standalone `Todo` component does attach task hints, but it always creates its own
one-item unordered list
([todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:338),
[todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:384)).

This does not satisfy ordered-list activation and makes composing Todo values
inside either list structurally different from making the list item itself a
task. Define a first-class task-list item input/projection shared by both list
components and Darkmatter rather than recognizing display strings separately in
each renderer.

### High: Browser rendering implements the opposite of the specified SVG design

The specification requires inline SVG state icons, an automatically supplied
stylesheet, and CSS-variable-driven stroke, fill, path, and stroke width
([spec.md](spec.md:7), [spec.md](spec.md:25)). The browser implementation
documents and emits a native disabled checkbox
([todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:530)), and
the tests explicitly require `type="checkbox"`
([todo_parity.rs](../../../../biscuit-terminal/lib/tests/todo_parity.rs:592)).
Repository searches find no Todo stylesheet, `--todo-*` variables, or Todo SVG
renderer; the only browser hooks are state classes such as `todo-open`
([todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:280)).

Implement a typed, sanitized inline-SVG node/component and attach the Todo
stylesheet through fragment/page metadata. Verify both DOM structure and
computed CSS custom-property effects in a real browser. String assertions on
generated HTML are insufficient for this visual contract.

### High: Terminal state rendering has no Level 2 verification

The terminal renderer does use terminal capability data to choose the Nerd Font
or fallback marker, which matches the intended architecture
([render.rs](../../../../biscuit-terminal/lib/src/render_tree/render.rs:2242)).
However, the Todo tests are all Level 1. They manufacture terminal capability
states and compare strings; there are no `level2_*` Todo tests. This cannot show
that the selected private-use glyph exists, has the expected width, receives the
intended SGR styling, or aligns correctly in an actual terminal.

The specification's visual terminal requirement therefore needs Level 2 capture
in at least the supported fallback and Nerd Font configurations. Per the review
test-rigor rules, this verification-level mismatch is a production blocker.

### Medium: The public state model is duplicated and already semantically divergent

`biscuit-terminal` exposes `TodoState`, while renderable exposes the nearly
identical `TaskState`, requiring a manual conversion match
([todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:63),
[todo.rs](../../../../biscuit-terminal/lib/src/components/todo.rs:72)). Neither
model can express the specified percentage states, and the current
`InProgress` variant loses the actual percentage. Expanding both enums and their
conversion would increase drift risk.

Use one canonical typed task-state model in renderable, preferably with an
explicit bounded progress value, and have the component API re-export or wrap
that model only when it adds a real invariant.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Standard GFM `[x]` / `[ ]` parsing through Darkmatter | Level 1 fold and round-trip tests | Appropriate for parsing, but covers only the baseline syntax |
| Percentage markers and `[!]` blocked marker | No behavioral test; cleanup-only string tests exist | Missing implementation and Level 1 end-to-end coverage |
| Automatic Todo activation in `UnorderedList` | No task-aware list-component test | Missing implementation |
| Automatic Todo activation in `OrderedList` | No task-aware list-component test | Missing implementation |
| Terminal Nerd Font detection and state marker rendering | Level 1 with manufactured terminal capabilities | Wrong level; needs Level 2 real-terminal capture |
| Browser inline SVG icons | No implementation; Level 1 tests require native checkboxes | Wrong behavior and wrong level; needs real-browser DOM/computed-style verification |
| Todo CSS variables and automatic stylesheet | No implementation or test | Missing implementation and real-browser verification |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- `cargo test --color=never -p renderable task_`: 8 passed.
- `biscuit-terminal` Todo parity and Darkmatter task-list focused commands were
  started, but their initial downstream compilation exceeded the non-interactive
  time budget; those runs were terminated and are not claimed as passing.
- Static inspection confirmed the existing Level 1 Todo parity suite and
  Darkmatter `[x]` / `[ ]` round-trip coverage.
- No Level 2 Todo test or real-browser Todo test exists.

The requested `root` skill is not present in the authoritative local skill
catalog. This review used the `renderable` and `rust-testing` skills, `sniff
repo`, and the repository-root instructions.

## Readiness

Not ready for production. The core extended syntax, automatic list integration,
and browser SVG/CSS design are unimplemented, and the implemented terminal
visual behavior lacks the required Level 2 verification.
