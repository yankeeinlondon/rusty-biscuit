---
ready: true
agent: open_code
model: ""
---

# Review: Inline Compose Sequence Mismatch

> Iteration #4. The three findings raised in `review-3.md` (shell side-effect
> surface, Level-2 styling/hyperlink verification, parser-compatible delimiter
> capture) have all been addressed. This iteration re-verifies those fixes and
> looks for new gaps.

## Status of Prior Findings

### High — Shell side-effect test did not use an executable directive (resolved)

The fail-fast test now embeds a real Darkmatter body directive instead of an
inert `$(touch ...)` expression:

- [inline_compose_sequence_mismatch.rs:488](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:488) — `::shell touch {sentinel}`

Darkmatter discovers `::shell` directives from the body, so the absent sentinel
now genuinely proves rejection preceded shell discovery, approval, and
execution. The provider sentinel and byte-for-byte source assertion remain in
place. Criterion 15 is now verified at the correct strength.

### High — Styling / hyperlink verified at the wrong level (resolved)

The raw PTY tests were reclassified from `level2_*` to
`level1_inline_compose_mismatch_pty.rs` and a genuine Level-2 capture was added
that closes both sub-gaps:

- [level2_inline_compose_mismatch_capture.rs:198](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_inline_compose_mismatch_capture.rs:198) — asserts an OSC 8 `file://` hyperlink target for the resolved document (the link contract tmux `capture-pane` cannot preserve).
- [level2_inline_compose_mismatch_capture.rs:207](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_inline_compose_mismatch_capture.rs:207) — asserts **diagnostic-specific** cyan SGR (`\x1b[36m`) rather than the bare `frame.raw.contains(ESC)` that a shell prompt could satisfy.

The WezTerm backend ran and passed in this environment, exercising a real
terminal emulator's decoder for both the hyperlink and the styling.

### Medium — YAML capture rejected delimiter forms the parser accepts (resolved)

`capture_frontmatter_yaml` now matches the parser's `line.trim() == "---"`
recognition for both opening and closing delimiters:

- [mismatch.rs:47](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/mismatch.rs:47) and [mismatch.rs:54](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/mismatch.rs:54)

Padded opening, closing, and both-delimiter fixtures were added
([mismatch.rs:224](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/mismatch.rs:224)–250). Darkmatter only supports `---` frontmatter (verified at
[frontmatter.rs:220](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/frontmatter.rs:220)), so the capture helper is now fully consistent with the parser
and there is no non-`---` delimiter gap to worry about.

## New Findings

### Low — The status-block hint renders between the YAML intro and the YAML block

The diagnostic builds its normative paragraphs as the StatusBlock `body`
(`opening`, `explanation`, `sections_note`, `yaml_note`) and then attaches a
`.hint(...)`. In the bespoke render path used for this block, the hint renders
**below** the bordered body, so the emitted order is:

```
… body …
┃ Below is the full YAML definition of the document:
Run the document with `claudine sequence <file>`.
                              ← blank line
<verbatim YAML>
```

The spec's normative layout (Diagnostic Contract items 4–5) calls for the YAML
introduction to be "a blank line followed by a YAML-rendered block." The hint
interposes between the intro and that block, slightly muddying the "follows"
flow. The hint is also somewhat redundant with the explanation, which already
directs the user to `claudine sequence`.

This is cosmetic, not functional — every required element is present and
asserted by the L1 render tests. It could be resolved by dropping the hint or
by emitting the YAML intro outside the bordered body alongside the appended
YAML. Not a blocker.

- [error.rs:1061](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1061) (YAML intro inside body) · [error.rs:1076](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1076) (hint) · [error.rs:982](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:982) (appended YAML)

### Low / Informational — Styling can leak when stdout is a TTY but stderr is redirected

The mismatch feature correctly gates the YAML payload on `stderr_is_tty`
(captured at detection time, [compose.rs:812](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs:812)). However, the diagnostic's SGR/OSC 8 styling
is driven by the shared `Terminal`, which `compute_terminal` builds from
**stdout OR stderr**. In the unusual split `cmd 2> err.log` (stdout = TTY,
stderr = pipe), the terminal is colored, `stderr_is_tty` is false, the YAML is
correctly withheld, but the escape-strip in
[error.rs:998](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:998) only triggers at `ColorDepth::None`, so SGR bytes reach the piped
stderr.

This is a pre-existing, claudine-wide terminal-construction characteristic
(every `BlockError` renders through the unified terminal), not something this
feature introduced, and the spec's hard normative requirement (criterion 12 —
YAML omission) is satisfied. The strict "no escape byte" contract (criterion
16) is verified for the common fully-piped case (both streams redirected, the
actual log/CI scenario). Flagged only for completeness; no change required to
ship this feature.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Mismatch truth table & validation precedence (1–3, 7) | L1 unit + L1 process | Adequate |
| Negative / existing-behavior paths (4, 5, 6, 8) | L1 process (provider stub) | Adequate |
| Malformed-frontmatter precedence (9) | L1 process (positive identity) | Adequate |
| Authored-value override isolation (10) | L1 process (`--set` both directions) | Adequate |
| Non-TTY withholding & plain output (12, 16) | L1 process raw bytes + L1 render | Adequate |
| TTY branch & YAML inclusion (11) | L1 PTY + L2 tmux/WezTerm | Adequate |
| YAML fidelity & LF/CRLF boundaries (13, 14) | L1 unit (incl. padded delimiters) + L1 render + L1 PTY fragments + L2 fragments | Adequate |
| Diagnostic-specific styling | L2 WezTerm (`\x1b[36m` cyan) | Gap closed |
| OSC 8 resolved-document link (11) | L1 PTY + L2 WezTerm (`file://` target) | Gap closed |
| Shell / provider / source fail-fast (15) | L1 process (`::shell` directive + provider stub + byte-diff source) | Gap closed |
| Keyboard-driven behavior | Not applicable | No Level 3 required — fail-fast rejection, not an interactive UX flow |

## Verification

- **Build** — `cargo build -p claudine -p claudine-cli`: clean.
- **Lint** — `cargo clippy -p claudine -p claudine-cli --all-targets -D warnings`: clean, zero warnings.
- **Targeted mismatch suite** — `cargo nextest run mismatch`: **32/32 passed**, spanning L1 unit (capture + detection), L1 render (TTY / non-TTY / plain), L1 process (criteria 1–10, 12, 15), L1 PTY, L2 tmux, and L2 WezTerm (OSC 8 `file://` + cyan SGR).
- **Regression** — `cargo nextest run 'compose' 'sequence' 'inline'`: **447/448 passed**. The single timeout (`compose_sigint_during_prep_exits_130_with_notice`) is a pre-existing, timing-sensitive SIGINT-handling test unrelated to this feature (it exercises Ctrl+C during prep, not the early mismatch rejection).
- **Docs** — `composition.md` "Inline-Compose / Sequence Mismatch" subsection and `timeline.md` entry present and accurate; Definition of Done satisfied.

## Verdict

Ready for production. All three prior-review findings are resolved at the
correct verification levels, including a genuine Level-2 WezTerm capture that
proves the OSC 8 hyperlink and diagnostic-specific styling render through a
real terminal emulator. All 16 acceptance criteria are covered. The two new
observations are low-severity and cosmetic/pre-existing and do not block ship.
