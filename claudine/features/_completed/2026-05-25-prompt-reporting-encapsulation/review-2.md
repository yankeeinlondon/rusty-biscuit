---
ready: false
agent: codex
model: ""
---

# Review: Prompt Reporting Encapsulation — Iteration 2

## Verdict

Not ready for production. The encapsulation refactor itself is in good shape: the public `prompt_reporting` re-export surface is now the specified seven items, the call sites construct one report after one resolver call, `EffectiveSystemPrompt` is gone from Rust source, and `cargo doc -p claudine --no-deps` is clean.

One user-visible rendering regression remains, and its test coverage is at the wrong verification level for the visual contract.

## Findings

### High: Nerd Font repo-root hyperlink label changed and is only covered at Level 1

The spec's non-goal is explicit: this stage must not change visual output. The existing prompt-reporting contract for in-repo Nerd Font terminals renders the system-prompt source label as the repo glyph directly followed by the relative path, e.g. `\u{f02a2}/system-prompt.md`.

The new implementation renders an extra visible space between the glyph and slash:

```rust
(Some(rel), Some(true)) => format!("{NERD_FONT_REPO_GLYPH} /{}", rel.display()),
```

That is at [claudine/lib/src/prompt_reporting/system_prompt.rs:48](../../lib/src/prompt_reporting/system_prompt.rs). The unit test now locks in the changed output at [claudine/lib/src/prompt_reporting/system_prompt.rs:395](../../lib/src/prompt_reporting/system_prompt.rs), expecting `format!("{NERD_FONT_REPO_GLYPH} /.claude/system-prompt.md")`.

This is a visual-output behavior change, not an encapsulation-only refactor. It also has the wrong strongest verification level: the current coverage is Level 1 with a synthetic `Terminal { is_nerd_font: Some(true) }`. The new Level 2 tests cover colors, wrapping, truncation, and OSC8 link bytes, but they do not verify the visible Nerd Font repo-root label in a real terminal capture. Under the requested review rubric, this user-observable label/chrome contract needs Level 2 coverage before it can be considered production-ready.

Recommended fix: restore the no-space label (`format!("{NERD_FONT_REPO_GLYPH}/{}", rel.display())`) and add a Level 2 capture assertion that exercises a Nerd Font-capable terminal path label, or otherwise documents why the host terminal cannot provide that capability and keeps a lower-level regression test for the exact no-space string.

## Coverage Notes

Requirement-to-level summary:

| Requirement | Strongest observed verification | Ready? |
|---|---:|---|
| Seven-item `prompt_reporting` public re-export surface | Code review | Yes |
| `ResolvedSystemPrompt` rename and removal of `EffectiveSystemPrompt` from Rust source | Code review / `rg` | Yes |
| System and agent report construction at CLI call sites | Code review / compile | Yes |
| Precedence and frontmatter parsing | Level 1 unit tests | Yes |
| `SystemPromptReport::render` dispatch for `Ready`, `None`, and `Disabled` | Level 1 unit tests | Yes |
| Prompt-reporting CLI behavior for silent/quiet/verbose/env/frontmatter/length | Level 1 CLI integration tests | Yes |
| Colors, OSC8, wrapping, and front/back truncation in real terminal | Level 2 test file exists and builds | Yes, assuming normal `just test-l2` backend availability |
| Nerd Font repo-root visible label remains unchanged | Level 1 only, and currently asserts changed output | No |

## Verification

Commands run:

- `cargo check --color=never -p claudine -p claudine-cli` — passed.
- `cargo doc --color=never -p claudine --no-deps` — passed without warnings.
- `cargo test --color=never -p claudine prompt_reporting --lib` — 111 passed.
- `cargo test --color=never -p claudine-cli --test prompt_reporting` — 12 passed.
- `cargo test --color=never -p claudine-cli --test level2_prompt_reporting_capture --no-run` — passed.

I did not run the Level 2 tests themselves because they require real terminal harness availability and should normally be run through the package area's `just test-l2` recipe.
