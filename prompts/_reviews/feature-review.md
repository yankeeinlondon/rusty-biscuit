---
$schema:
    spec: file(required;eager) -> the specification file providing the basis for this review's findings
    design: file -> the design file (_optional_) that compliments the spec
    iteration: number -> the review's iteration number
    review: file -> the review file which will be created based on this prompt's execution
description: "Reviews a _feature specification_ to make sure that the specification has been fully implemented. This prompt is also aware of the likelihood of more than one review being necessary and therefore names the reviews `review-{iteration}.md` in the same folder where the feature was specified.\n\nThe caller can pass in the **iteration** number but it should be detected automatically."

dir: "{{dirname(spec)}}"
design: "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}"
iteration: "{{ file_exists(spec) ? (frontmatter(spec, 'review_iterations') || 0) + 1  : 1   }}"
review: "{{ dirname(spec) + '/review-' + iteration + '.md' }}"
previous: {{ iteration < 2 ? null : decrement_file_index(review) }}
feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
start:
    message: "👓 starting {{feature_or_fix}} review #{{iteration}} of `{{parent_dir(spec)}}` (_in the **{{ctx.area}}** package area_)"
    info: "spec [{{spec}}]: {{file_exists(spec)}}"
success:
    stack:
        - when: "frontmatter(review,'ready') == true"
          action:
              - success: "{{feature_or_fix}} review {{iteration}} of `{{ parent_dir(spec) }}` in **{{ctx.area}}** finished and deemed code to be **production ready**"
              - message: "✅  {{feature_or_fix}} review #{{iteration}} for `{{parent_dir(spec)}}` in the **{{ctx.area}}** package area completed successfully (_**production ready**_)"
              - effect: small-group-cheer
        - when: "frontmatter(review,'ready') != true"
          action:
              - warn: "{{feature_or_fix}} review {{iteration}} of `{{ parent_dir(spec) }}` in the {{ctx.area}} package area has completed successfully but <i><yellow>not</yellow></i> production ready: <blue>{{link(review)}}</blue>"
              - message: "⚠️  {{feature_or_fix}} review #{{iteration}} for `{{parent_dir(spec)}}` in the **{{ctx.area}}** package area completed but was deemed NOT production ready"
              - effect: sad-trombone
failure:
    stderr: "{{feature_or_fix}} review {{iteration}} for `{{parent_dir(spec)}}` in the {{ctx.area}} package area failed to complete!"
    message: "💥 {{feature_or_fix}} review #{{iteration}} for `{{parent_dir(spec)}}` in **{{ ctx.area }}** failed to complete ({{err.msg}})!"
    effect: phase-jump-3
---
# Review of {{title_case(without_date(parent_dir(spec)))}}

> - {{capitalize(feature_or_fix)}}: `{{parent_dir(spec)}}`
> - Review File (_output_): `@{{review}}`
> - Review Iteration: #{{iteration}}

::file ../_senior-reviewer.md

## Context

You are performing a review of the functionality defined by the following document(s):

::block when="spec"
- **Specification:** "@{{spec}}"
::end-block
::block when="design"
- **Technical Design:** "@{{design}}"
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

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality which is light on test coverage (we expect strong unit and integration testing for everything)
- are there any changes which would make the code more ergonomic, more performant, or both?

## Test Rigor — Level 1 / Level 2 / Level 3

Test count is not test rigor. Phrases like "covered by substantial unit and integration tests" are banned from this review unless you can pair each user-facing requirement with a verification level:

- **Level 1 (in-process / PTY).** 

    Unit tests, plus tests that spawn the binary in a pseudo-TTY and feed it manufactured input bytes. Useful and necessary, but does NOT verify the terminal emulator's encoder/decoder behaviour — *we* generate those bytes. Cannot catch bugs like "WezTerm does not emit bare-modifier press events because we forgot to push `REPORT_ALL_KEYS_AS_ESCAPE_CODES`."

- **Level 2 (run-in-real-terminal with IPC).** 

    Spawn the binary inside an actual terminal emulator (WezTerm, Kitty) or multiplexer (tmux), capture the rendered pane text via the terminal's CLI(`wezterm cli get-text`, `kitty @ get-text`, `tmux capture-pane`). Verifies that glyphs, widths, SGR styling, and scrolling render correctly through the real terminal. Input is still byte-level injected via the terminal's CLI, so the terminal's input encoder is NOT exercised.

- **Level 3 (OS keyboard injection).** 
 
    Real OS keyboard events (`cliclick` on macOS, `xdotool` on
    Linux) injected into the spawned terminal window. The terminal's input encoder fires — this is the only level that can verify "what bytes does the terminal actually emit when the user presses bare Ctrl?" Required for any UX requirement of the form "when the user holds/presses key X, Y happens." Currently env-gated behind `RUN_LEVEL3=1` because focus stability is platform-specific.

When reviewing, for each requirement that asserts user-observable behaviour (modifier-press visibility, hotkey activation, keybinding behaviour, paste / IME / mouse, scroll on overflow, etc.), classify the verification level present and call out any mismatch:

- "Spec requires modifier-press to surface badges" + only Level-1 tests = **gap, not "ready"**.
- "Spec requires hotkey chord activation" + Level-2 in tmux but no Level-1 chord-byte test = fine.
- "Spec requires `^X` badges with specific colors" + Level-1 unit tests on style only = needs
  Level-2 capture verifying real-terminal rendering.

A feature MAY be marked production-ready only when each user-observable requirement has at minimum
the level of verification appropriate for it. Reviewers MUST list any requirement whose strongest
test is at the wrong level under "Findings" with severity at least "high".

## Closure

- Save your review suggestions to "@{{review}}"
- Save the following frontmatter properties to the review file (@{{review}}):
    - set `$schema` to "feature-review.yaml"
    - set `ready` to whether you think this feature is **ready for production** (boolean)
    - set the `agent` property to "{{ctx.agent}}/{{ctx.model}}" 
    - set the `created` property to "{{ctx.now}}"
    - set the `spec` property to "{{ parent_dir(spec) }}/{{ basename(spec) }}"
    - set the `implemented` property to `false`
    - set the `description` property to "A **{{feature_or_fix}}** review of `{{ parent_dir(spec) }}/{{ basename(spec) }}`"
    - set the `{{feature_or_fix}}` property to "{{ parent_dir(review) }}/{{ basename(review) }}"
    ::block when="iteration < 2"
    - set the `previous` property to "{{parent_dir(previous)}}/{{basename(previous)}}"
    ::end-block
::block when="iteration >  1"
- Now set the frontmatter properties of the _previous review_ located at @{{previous}}:
    - set the `next` property on the _previous review_ to "{{parent_dir(review)}}/{{basename(review)}}"
    - set the `implemented` property to `true`
::end-block
- Set the spec file's ({{spec}}) `review_iterations` Frontmatter property to '{{iteration}}'
- Summarize to the caller what was found and be sure to mention whether the review deemed the {{feature_or_fix}} to be **production ready** or not.

::block when="iteration != 1"
> **Note:** this is _not_ the first review we've done on this functionality but the prior review's suggestions have now all been implemented (or at least the developer has claimed that they are).
::end-block

**IMPORTANT:**

::block when="ctx.area != 'root'"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
