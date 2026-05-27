---
$schema:
    - iteration: number
      spec: string(required)
    - review: string(required)
      iteration: number
name: Implement Review Suggestions
description: Implements all the recommendations/suggestions produced in a review. Provide review path and/or iteration number (if not 1) and optionally the spec path if this is a review of a specification.
iteration: 1
area: "{{ ctx.current_package ? ctx.current_package : ctx.current_package_area }}"
dir: "$(dirname '{{review || spec}}')"
review_path: "{{ctx.repo_root}}/{{area}}/{{review ? review : '{{dir}}/review-{{iteration}}' }}"
spec_path: "{{ctx.repo_root}}/{{area}}/{{spec}}"
---
::block when="spec"
## Context

- Use the '{{area}}' agent skill when reviewing
- This review is focused on the '{{area}}' package area which has the following packages:

    ::shell sniff repo packages

You will review the implementation's fidelity to the specification file:

- {{spec_path}}

Your review suggestions should be written to:

- {{review_path}}

::block when="iteration == 1"
This is the first review of this specification document since the implementation.
::end-block
::block when="iteration != 1"
A prior review of the implementation for this specification did NOT deem the implementation
to be "production ready" but we have now implemented all of the suggestions from that review
and your task will be to again compare the implementation of the specification relative to
the written intention of the specification.

> Note: you should _also_ validate that all of the "complaints/suggestions" of the prior review have been fully addressed. You are current performing review #{{iteration}} so you should be looking for review in the 
{{dir}} directory with a name similar to "review-{{decrement(iteration)}}.md"
::end-block

## Task

Read the specification document and then perform a review on the implementation:

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality which is light on test coverage (we expect strong unit and integration testing for everything)
- are there any changes which would make the code more ergonomic, more performant, or both?

## Test Rigor — Level 1 / Level 2 / Level 3

Test count is not test rigor. Phrases like "covered by substantial unit and integration tests" are
banned from this review unless you can pair each user-facing requirement with a verification level:

- **Level 1 (in-process / PTY).** Unit tests, plus tests that spawn the binary in a pseudo-TTY and
  feed it manufactured input bytes. Useful and necessary, but does NOT verify the terminal emulator's
  encoder/decoder behaviour — *we* generate those bytes. Cannot catch bugs like "WezTerm does not
  emit bare-modifier press events because we forgot to push `REPORT_ALL_KEYS_AS_ESCAPE_CODES`."

- **Level 2 (run-in-real-terminal with IPC).** Spawn the binary inside an actual terminal emulator
  (WezTerm, Kitty) or multiplexer (tmux), capture the rendered pane text via the terminal's CLI
  (`wezterm cli get-text`, `kitty @ get-text`, `tmux capture-pane`). Verifies that glyphs, widths,
  SGR styling, and scrolling render correctly through the real terminal. Input is still byte-level
  injected via the terminal's CLI, so the terminal's input encoder is NOT exercised.

- **Level 3 (OS keyboard injection).** Real OS keyboard events (`cliclick` on macOS, `xdotool` on
  Linux) injected into the spawned terminal window. The terminal's input encoder fires — this is
  the only level that can verify "what bytes does the terminal actually emit when the user presses
  bare Ctrl?" Required for any UX requirement of the form "when the user holds/presses key X, Y
  happens." Currently env-gated behind `RUN_LEVEL3=1` because focus stability is platform-specific.

When reviewing, for each requirement that asserts user-observable behaviour (modifier-press
visibility, hotkey activation, keybinding behaviour, paste / IME / mouse, scroll on overflow, etc.),
classify the verification level present and call out any mismatch:

- "Spec requires modifier-press to surface badges" + only Level-1 tests = **gap, not "ready"**.
- "Spec requires hotkey chord activation" + Level-2 in tmux but no Level-1 chord-byte test = fine.
- "Spec requires `^X` badges with specific colors" + Level-1 unit tests on style only = needs
  Level-2 capture verifying real-terminal rendering.

A feature MAY be marked production-ready only when each user-observable requirement has at minimum
the level of verification appropriate for it. Reviewers MUST list any requirement whose strongest
test is at the wrong level under "Findings" with severity at least "high".

## Closure

- Save your review suggestions to "{{review_path}}"
- based on your review suggestions indicate whether you think this feature is **ready for production** by setting the `ready` frontmatter property on "{{review_path}}" to `true` or `false`
- save the `agent` frontmatter property as "{{env.AGENT}}" in the "{{review_path}}" file
- save the `model` frontmatter property as "{{env.MODEL}}" in the "{{review_path}}" file

## **IMPORTANT:**

- do NOT change the `ready` property in the review file after implementing
    - you may feel that everything in that review was fixed but the review's assessment at that time should not change
    - furthermore, we will be running another review _after_ you've completed here to validate that everything is fixed
- do not run `cargo fmt` ... we want functional changes during this work not formatting changes
- do not commit your work to git (this will be done as an independent process which you are not responsible for)
::file ./you-are-non-interactive.md
- communicate as much as possible so that the caller can keep track of progress

::end-block

::block when="!spec && review"

The following review has just completed:

- {{review_path}}

::block when="iteration != 1"
A prior review did NOT deem the implementation to be "production ready" but we have now implemented all of the suggestions from that review and your task will be to again compare the implementation of the specification relative to the written intention of the specification.

> Note: you should also validate that all of the "complaints/suggestions" of the _prior_ review have now been fully addressed. You are current performing review #{{iteration}} so you should be looking for review in the 
{{dir}} directory with a name similar to "review-{{decrement(iteration)}}.md"
::end-block

Your task is to implement all the suggestions in that review.

::end-block
