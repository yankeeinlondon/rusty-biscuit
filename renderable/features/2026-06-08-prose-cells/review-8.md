---
ready: false
agent: codex
model: ""
---

# Review 8

## Findings

### Medium: The fenced-code degradation test never produces a code node

The specification requires fenced code inside a Prose cell to degrade to its
literal body and produce a valid table tree. The test fixture starts its fence
after ordinary text on the same line:
`"before ```rust\nfn main() {}\n``` after"`
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:226`).

Prose recognizes a fence only when the trimmed line starts with three
backticks (`biscuit-terminal/lib/src/components/prose/markdown.rs:103`).
Consequently, this fixture remains ordinary text. The assertions that no
`NodeKind::Code` survives and that `fn main()` remains visible pass without
exercising `degrade_code_nodes`.

Use a fixture with standalone fence lines, such as
`"before\n```rust\nfn main() {}\n```\nafter"`. First assert that
`Prose::to_render_nodes()` contains a `Code` node, then assert that table
projection replaces that node with text containing only the code body, omits
the fence/language metadata, and validates cleanly. Render the projected table
to Browser, Markdown, MarkdownPlus, and Terminal to verify the deterministic
cross-target degradation required by acceptance criterion 6.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, structured projection, hints, and layout isolation | Level 1 | Appropriate |
| Fenced-code degradation | None effective | Gap; the Level 1 fixture does not parse as fenced code |
| No-color terminal output | Level 1 | Appropriate |
| Bold, dim, red, and OSC8 containment including leading/trailing geometry | Level 2 | Appropriate; iteration 8 closes review 7's false-positive window |
| Wrapped and multiline visible geometry | Level 2 | Appropriate |
| Standard and cursor-alignment path styling | Level 2 | Appropriate |
| Mixed typed formatting and alignment in both terminal paths | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and supported visual styles | Browser tier | Appropriate |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal-cli --test level2_prose_cells containment
  --color=never`: 38 passed.
- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`:
  58 passed.
- `cargo test -p biscuit-terminal-cli --test integration_test test_table_
  --color=never`: 16 passed.
- `cargo check -p biscuit-terminal-cli -p biscuit-terminal -p renderable
  --color=never`: passed.
- `just -f biscuit-terminal/justfile test-browser`: 81 passed, including all
  three real-browser Prose-cell computed-style tests.
- `just -f biscuit-terminal/justfile test-l2 prose_cells`: the complete Kitty
  Prose-cell suite passed. The tmux harness then failed to create a session
  after four attempts, so nextest canceled WezTerm; no feature assertion failed.

Iteration 8 closes review 7's leading-edge containment gap with stateful
leading-border checks and focused negative tests for bold, dim, foreground
color, and OSC8 links. Production readiness remains blocked by the ineffective
fenced-code acceptance test.
