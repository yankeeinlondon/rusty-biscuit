---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: terminal code blocks reset and drop the enclosing span style for following text

The spec requires the terminal renderer to preserve existing ANSI behavior, including layer restoration for nested spans (`renderable/features/2026-05-17-prose-cross-target/spec.md`, FR-3 and Target Rendering / Terminal). The new IR can now represent ordinary text before and after an opaque code block (`biscuit-terminal/lib/src/components/prose/ir.rs:249`), but `render_code_block` emits a hard `\x1b[0m` for every code line (`biscuit-terminal/lib/src/components/prose/terminal.rs:46`). That reset clears all terminal attributes while `StyleState` still believes the parent span is active, so sibling text after the code block is rendered plain until the parent closes.

Manual repro:

```bash
env -u NO_COLOR FORCE_COLOR=1 cargo run --quiet --color=never -p biscuit-terminal-cli -- \
  prose --force-color --print-bytes $'<red>before\n```\ncode\n```\nafter</red>'
```

Observed byte stream:

```text
1b5b33316d6265666f72650a1b5b326d2020636f64651b5b306d0a61667465721b5b33396d1b5b306d
```

Decoded, that is `ESC[31m before`, then `ESC[2m code ESC[0m`, then plain `after`, then `ESC[39m ESC[0m`. The expected behavior is for `after` to still be red, either by restoring active parent layers after the code-block reset or by making code-block dimming layer-aware instead of using a hard reset.

Current tests cover standalone code-block dimming (`biscuit-terminal/lib/src/components/prose/mod.rs:600`) and Level-2 rich styling where the code block is a final sibling (`biscuit-terminal/cli/tests/level2_prose_styling.rs:363`). They do not cover a code block inside an active style with following sibling text, which is the restoration case that fails.

Verification level present: none for this requirement. Required: Level 1 unit coverage for the exact emitted SGR sequence and Level 2 capture proving the real terminal keeps the post-code-block sibling styled.

## Test Rigor Assessment

- Parser requirements: Level 1 coverage now includes the prior code-block opacity regression, unknown tags, escaped text, former atomic syntax, nesting, href protection, and multiple fenced blocks.
- Browser requirements: Level 1 output tests cover escaped text, escaped code-block bodies, semantic tags, links, style spans, and unknown tags.
- Plain Markdown and MarkdownPlus requirements: Level 1 tests cover semantic styles, links with delimiter-bearing destinations, code blocks, color degradation/preservation, underline preservation in MarkdownPlus, and JavaScript absence.
- Terminal requirements: Level 1 and Level 2 coverage exists for many SGR effects, OSC8, NO_COLOR, layout, and standalone code blocks. The nested code-block restoration case above is unverified and currently broken, so the terminal behavior is not production-ready.
- Keyboard/input UX requirements: none in this spec, so Level 3 is not required.

## Verification Performed

- Source review of the spec and changed Prose parser/renderers.
- Manual CLI repro for nested styled fenced code block with `NO_COLOR` unset and `FORCE_COLOR=1`, showing the parent red style is not restored after the code block.
- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 154 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test prose` passed: 5 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture` did not complete within the 60-second non-interactive bound. The run compiled successfully and several tests passed, including `level2_prose_emits_sgr_in_real_terminal`, `level2_prose_nested_emphasis_visible_text_in_wezterm`, `level2_no_color_strips_sgr_in_real_terminal`, `level2_pad_columns_respect_actual_pane_width`, and `level2_columns_word_wrap_in_pane`, but the full suite timed out before completion.

## Production Readiness

Not ready. The prior fenced-code opacity issue appears fixed, but terminal rendering still loses enclosing styles after code blocks nested inside styled spans.
