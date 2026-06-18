---
description: "Reviews a _feature specification_ to make sure that the specification has been fully implemented. This prompt is also aware of the likelihood of more than one review being necessary and therefore names the reviews `review-{iteration}.md` in the same folder where the feature was specified.\n\nThe caller can pass in the **iteration** number but it should be detected automatically."
parameters:
    spec: 
        type: "optional(file)"
        desc: "the file path to the specification file"
    design:
        type: "optional(file)"
        desc: "the file path to the technical design file"
    iteration:
        type: "optional(number)"
        desc: "the iteration count of the review"
dir: "$(dirname '{{spec || design}}')"
iteration: 1
area: "{{ctx.current_package_area == 'root' ? ctx.current_package || '' : ctx.current_package_area}}"
review_file: "{{ctx.area}}/{{dir}}/review-{{iteration}}.md"
start:
    message: "👓 starting feature review #{{iteration}} of `{{parent_dir(spec)}}` (_in the **{{ctx.area}}** package area_)"
success:
    stderr: "Feature review {{iteration}} in the {{ctx.area}} package area has completed"
    message: "✅  feature review #{{iteration}} for `{{dir}}` in the **{{ctx.current_package_area}}** package area has completed. The review can be found at `{{review-file}}`"
    effect: "small-group-cheer"
failure:
    stderr: "Feature Review {{iteration}} in the {{ctx.area}} package area failed to complete!"
    message: "Feature Review #{{iteration}} for `{{ctx.area}}/{{dir}}` failed to complete!"
    effect: two-tone
---
# Review of {{dir}}
> Iteration #{{iteration}}

::file _senior-reviewer.md

## Context

You are performing a review of the functionality defined by the following document(s):

::block when="spec"
- specification: "{{area}}/{{dir}}/{{spec}}"
::end-block
::block when="design"
- technical design: "{{area}}/{{dir}}/{{design}}"
::end-block

::block when="And(spec, design)"
Read both the specification and design documents and then perform a review on the implementation:
::end-block
::block when="spec"
Read both the specification document and then perform a review on the implementation:
::end-block
::block when="design"
Read both the specification document and then perform a review on the implementation:
::end-block

::block when="iteration != 1"
> **Note:** this is _not_ the first review we've done on this functionality but the prior review's
> suggestions have now all been implemented.
::end-block

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

- Save your review suggestions to "{{area}}/{{dir}}/review-{{iteration}}.md"
- based on your review suggestions indicate whether you think this feature is **ready for production** by setting the `ready` frontmatter property on "{{area}}/{{dir}}/review-{{iteration}}.md" to `true` or `false`
- save the `agent` frontmatter property as "{{env.AGENT}}" in the "{{area}}/{{dir}}/review-{{iteration}}.md" file
- save the `model` frontmatter property as "{{env.MODEL}}" in the "{{area}}/{{dir}}/review-{{iteration}}.md" file

**IMPORTANT:**

::block when="ctx.current_package_area"
- use the '{{ctx.current_package_area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
