---
ready: false
agent: codex
model: ""
---

# Review: Inline Compose Sequence Mismatch

## Findings

### High: The shell side-effect test does not configure an executable shell directive

The fail-fast test places `$(touch ...)` in the Markdown body:

- [inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:484)

Darkmatter discovers body commands from `::shell` directives; `$(...)` shell
expressions are a frontmatter feature. The current body text is therefore inert,
so the absent sentinel does not prove rejection occurred before shell discovery,
approval, or execution. This leaves acceptance criterion 15's explicit shell
side-effect requirement unverified.

Use an actual command surface, such as
`::shell touch <sentinel>`, or a top-level frontmatter `$(touch <sentinel>)`
value. Keep the provider sentinel and byte-for-byte source assertion.

### High: Styling and hyperlink behavior remain verified at the wrong level

The raw PTY tests are Level 1 under the supplied taxonomy, not Level 2:

- [level2_inline_compose_mismatch_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_inline_compose_mismatch_pty.rs:1)

They prove the real process emits SGR and OSC 8 bytes, but no terminal emulator
has decoded them. The genuine Level 2 test uses tmux:

- [level2_inline_compose_mismatch_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_inline_compose_mismatch_capture.rs:77)

tmux `capture-pane` does not preserve OSC 8 links, so this test cannot verify the
linked-document contract. Its styling assertion is also only
`frame.raw.contains(ESC)`; the captured shell prompt can satisfy that even if
the diagnostic itself is unstyled.

Add a WezTerm Level 2 capture that asserts an OSC 8 `file://` target for the
resolved document and diagnostic-specific SGR styling. Rename/reclassify the
PTY tests as Level 1 so they run in the correct tier.

### Medium: YAML capture rejects delimiter forms accepted by the parser

Darkmatter recognizes opening and closing delimiter lines with surrounding
whitespace using `line.trim() == "---"`:

- [frontmatter.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/frontmatter.rs:219)

The new capture helper requires the opening delimiter to be exactly `---` and
only trims line endings:

- [mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/mismatch.rs:43)

Consequently, a valid parsed mismatch with a delimiter such as ` --- ` is
rejected correctly, but `capture_frontmatter_yaml` returns `None`;
[compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs:810)
silently converts that to an empty YAML block. This violates the TTY full-YAML
contract.

Match the parser's delimiter recognition while retaining byte offsets and add
fixtures for whitespace-padded opening and closing delimiters.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Mismatch truth table and validation precedence | Level 1 unit/process | Adequate |
| Authored-value override isolation | Level 1 process | Adequate |
| Non-TTY withholding and plain output | Level 1 process | Adequate |
| TTY branch and YAML inclusion | Level 2 tmux | Adequate for ordinary delimiters |
| YAML fidelity and LF/CRLF boundaries | Level 1 unit plus Level 2 fragments | Gap for parser-accepted padded delimiters |
| Diagnostic styling | Level 2 tmux | Gap: assertion is not diagnostic-specific |
| OSC 8 resolved-document link | Level 1 PTY | Gap: requires Level 2 capable capture |
| Shell/provider/source fail-fast behavior | Level 1 process | Provider/source adequate; shell surface is inert |
| Keyboard-driven behavior | Not applicable | No Level 3 required |

## Verification

The prebuilt Level 1 integration binary passed all 17 tests. Both staged and
working-tree `git diff --check` passed.

Current source could not be rebuilt because rustup has no installed toolchain.
The Level 2 recipe therefore could not be run; invoking Level 2 binaries
directly would violate the repository's harness rules.

## Verdict

Not ready for production. Core detection and precedence are sound, but two
explicit acceptance contracts remain unverified at the required level, and a
parser-compatible delimiter form produces an incomplete TTY diagnostic.
