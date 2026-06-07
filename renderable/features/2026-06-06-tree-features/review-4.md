---
ready: false
agent: codex
model: ""
---

# Review: Tree Features

## Findings

### High: Styled image truncation still lacks real-terminal verification

The iteration-3 defect is fixed in both terminal renderer branches:
`truncate_keeping_trailing_escapes` preserves the closing SGR envelope for
links and image placeholders
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:903),
[render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:3140)).
Separate Level 1 tests now exercise colored truncation for both node kinds.

The new Level 2 regression covers only a styled, truncated hyperlink
([level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2385)).
The existing image Level 2 tests separately verify color and exact-width
truncation, but no real-terminal case combines them with following inline text
([level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2434),
[level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2507)).
Because links and images use distinct renderer branches, the hyperlink capture
does not verify that a terminal emulator observes the image placeholder's reset
before subsequent text.

Add a Level 2 image case with local image color, a truncating width, and an
unstyled trailing marker on the same line. Assert that the marker is outside
the image's color run. Under the review's mandatory rigor rules, this
user-visible terminal behavior cannot be marked ready with Level 1 coverage
alone.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Browser and MarkdownPlus alpha lowering | Level 1 plus real-browser computed style | Appropriate |
| Terminal alpha/color degradation | Level 1 plus Level 2 color capture | Appropriate |
| Styled link truncation restores following text | Level 1 plus a Level 2 test | Appropriate test level |
| Styled image truncation restores following text | Level 1 only | Gap: requires Level 2 real-terminal capture |
| Link exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Image exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Appropriate apart from styled-reset combination above |
| List-item placement | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Structured link/image browser attrs and CSS precedence | Level 1 plus real-browser computed style | Appropriate |
| Root foreground inheritance and frame-only page background | Level 1 structural tests plus real-browser computed style | Appropriate |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- Targeted Level 1 link and image styled-truncation tests passed.
- All three `split_trailing_escapes` unit tests passed.
- `git diff --cached --check` passed.
- The combined Level 1 package run was stopped after exceeding the session's
  60-second command limit during a broad feature-unified rebuild.
- The targeted canonical Level 2 run was stopped at the same limit as the CLI
  test began. Its resulting `no such pane` failure was caused by teardown of
  the shared WezTerm pane, so it is not treated as an implementation failure or
  a successful verification.

The requested `root` skill is not present in the authoritative local skill
catalog. This review used `renderable`, `rust-testing`, and
`biscuit-test-harness` plus the repository-root instructions.

## Readiness

Not ready for production. The iteration-3 implementation defect appears fixed,
but the image branch remains below the required verification level.
