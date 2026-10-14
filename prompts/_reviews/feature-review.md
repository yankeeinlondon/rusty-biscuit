---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification file providing the basis for this review's findings
    design: file(match(**/*design*.md)) -> the design file (_optional_) that compliments the spec
    iteration: number -> the review's iteration number
    review: file -> the review file which will be created based on this prompt's execution
    spec_name: string -> the name of the spec (which is really just the spec's folder name)
    in_loop: boolean -> set by the `review-loop` sequence; when true this review opts out if the previous review already ended the loop
    next_spec: file(match(**/*spec*.md)) -> if the current spec is part of a multi-spec/multi-plan process then this points to the NEXT spec in the sequence
    next_plan: file(match(**/*plan*.md)) -> if the current spec is part of a multi-spec/multi-plan process then this points to the NEXT plan in the sequence
    human_in_the_loop: string[] -> filled out with review items when it has determined that a human review is required
description: |-
    Reviews a _feature specification_ to make sure that the specification has been fully implemented. This prompt is also aware of the likelihood of more than one review being necessary and therefore names the reviews `review-{iteration}.md` in the same folder where the feature was specified.

    > **Note:** the caller _can_ pass in the **iteration** number but it should be detected automatically.

    There are two special cases in the review cycle that can be activated:

    1. **First Review**

        When we are in the first review cycle, the very first thing we will consider is whether there is a need for human-in-the-loop review.

        - if there is then this review will be repurposed to focus on getting the review feedback from the caller
        - if there is not then this review will be conducted like a normal review that compares the implementation against what was specified
    
    2. **Last Review** (_in a multi-spec pipeline_)

        In the last review cycle (aka, when `ready` has been set to `true`) -- but _only_ if this spec is part of a multi-spec pipeline -- we will
        look at whether the next spec and plan need to be refreshed based on the current spec's findings and decisions.
    
dir: "{{dirname(spec)}}"
design: |-
    {{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}
iteration: "{{ file_exists(spec) ? (frontmatter(spec, 'review_iterations') || 0) + 1  : 1   }}"
review: "{{ dirname(spec) + '/review-' + iteration + '.md' }}"
# A sibling of `spec` in whatever form `spec` arrived, as the implement router
# does. `decrement_file_index(review)` is not used because it re-anchors a
# relative `review` on this document's directory, and a sequence step passes a
# repo-relative `dir`.
previous_review: "{{ iteration > 1 ? replace(spec, basename(spec), 'review-' + (iteration - 1) + '.md') : null }}"
spec_name: "{{ parent_dir(spec) }}"
next_plan: |-
    {{
        next 
            ? replace(next_spec, "spec","plan")
            : null
    }}


feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
initialize:
    # Inside the review-loop sequence the step list is static, so a review
    # whose predecessor already closed the loop opts out here and launches
    # nothing. A direct `claudine compose` never sets `in_loop`.
    - when: "in_loop && previous_review && frontmatter(previous_review, 'ready') == true"
      then:
          - message: "review #{{iteration}} skipped: {{basename(previous_review)}} already marked `{{parent_dir(spec)}}` production ready"
          - skip
start:
    message: "🏃‍♂️ starting review #{{iteration}} of `{{parent_dir(spec)}}` (_{{feature_or_fix}} in the **{{ctx.area || ctx.repo}}**_)"
success:
    - when: "frontmatter(review,'ready') == true"
      then:
          - success: "{{feature_or_fix}} review {{iteration}} of `{{ parent_dir(spec) }}` in **{{ctx.area}}** finished and deemed code to be **production ready**"
          - message: |-
                ✅  review #{{iteration}} of `{{parent_dir(spec)}}` in **{{ctx.area || ctx.repo}}** completed successfully (_**production ready**_)
          - effect: small-group-cheer
    - when: "frontmatter(review,'ready') == true && frontmatter(review,'human_review') == true"
      then:
          - message: |-
                🤷  {{feature_or_fix}} review of `{{ parent_dir(spec) }}` -- while production ready -- requires human review. Review items include:

                {{ as_ordered_list(frontmatter(review,"human_review_items") || []) }}    
          - info: |-
                review #{{iteration}} of `{{ upper(feature_or_fix) + ' ' + parent_dir(spec) }}` -- _while production ready_ -- requires human review ({{length(frontmatter(review, 'human_review_items') || [])}} items):
    
                {{ as_ordered_list(frontmatter(review, 'human_review_items') || []) }}
    - when: "frontmatter(review,'ready') != true"
      then:
          - warn: |-
                {{feature_or_fix}} review {{iteration}} of `{{ parent_dir(spec) }}` in the {{ctx.area}} package area has completed successfully but <i><yellow>not</yellow></i> production ready: <blue>{{link(review)}}</blue>
          - message: |-
                ⚠️  review #{{iteration}} for the _{{feature_or_fix}}_ `{{parent_dir(spec)}}` in the **{{ctx.area_description || ctx.repo}}** _completed_ but is NOT production ready! Findings include:

                {{ as_ordered_list( frontmatter(review,"findings") || [] ) }}

                {{ 
                    frontmatter(review,'human_review') == true
                        ? '- in addition to the code findings blocking readiness there are some rulings that require **human review**:\n' + as_ordered_list( frontmatter(review, 'human_review_items') || [] )
                        : '- no _human review_ is required at this point'
                }}
          - effect: sad-trombone
failure:
    stderr: |-
        {{feature_or_fix}} review {{iteration}} for `{{parent_dir(spec)}}` in the {{ctx.area}} package area failed to complete!
    message: |-
        💥 {{feature_or_fix}} review #{{iteration}} for `{{parent_dir(spec)}}` in **{{ ctx.area || ctx.repo }}** failed to complete ({{err.msg}})!
    effect: phase-jump-3
finalize:
    - when: "human_in_the_loop && !err"
      message: |-
        - the first review after the implementation of `{{spec_name}}` was conducted and there are decisions that require your attention:

        {{human_in_the_loop}}
      info: |-
          - the first review after the implementation of {{ link(spec) }} was conducted and there are decisions that require your attention:
    
          {{human_in_the_loop}}
      stop: null
    - when: "refreshed && !err"
      message: |-
        - we have finished the review/fix cycle for `{{spec_name}}` but we are in a multi-spec pipeline
        - the next spec which will be implemented is `{{parent_dir(next_spec)}}`
        - the spec and plan have been refreshed:

        {{refreshed}}

        > **Note:** no human review was needed for this refresh
      info: |-
        - we have finished the review/fix cycle for `{{link(spec)}}` but we are in a multi-spec pipeline
        - the next spec which will be implemented is `{{link(next_spec)}}`
        - the spec and plan have been refreshed:

        {{refreshed}}

        > **Note:** no human review was needed for this refresh
    - when: "refresh_review && !err"
      message: |-
        - we have finished the review/fix cycle for `{{spec_name}}` but we are in a multi-spec pipeline
        - the next spec which will be implemented is `{{parent_dir(next_spec)}}`
        - the spec and plan have decisions that require human intervention:
  
        {{refresh_review}}
      info: |-
        - we have finished the review/fix cycle for `{{link(spec)}}` but we are in a multi-spec pipeline
        - the next spec which will be implemented is `{{link(next_spec)}}`
        - the spec and plan have been refreshed:
  
        {{refreshed}}     
      stop: null
    - when: "no_refresh && !err"
      message: |-
        - we have finished the review/fix cycle for `{{spec_name}}` and are in a multi-spec pipeline
        - the next _spec_ and _plan_ -- {{parent_dir(next_spec)}} -- was evaluated but did not need to be refreshed to be ready for implementation
      info: |-
          - we have finished the review/fix cycle for `{{spec_name}}` and are in a multi-spec pipeline
          - the next _spec_ and _plan_ -- {{parent_dir(next_spec)}} -- was evaluated but did not need to be refreshed to be ready for implementation
---
# Review of {{title_case(without_date(parent_dir(spec)))}}

> - {{capitalize(feature_or_fix)}}: `{{parent_dir(spec)}}`
> - Review File (_output_): `@{{review}}`
> - Review Iteration: #{{iteration}}

::file ../_senior-reviewer.md

## Context

You are performing a review of the functionality defined by the `{{spec_name}}` spec:

::block when="previous_review"
- This is the #{{iteration}} iteration of the review/fix cycle for this specification
- The first thing you will need to check is whether the _findings_ in the review's `## Unblocked Findings` section were all implemented
    - this is the most obvious thing that the last implementation was supposed to have addressed

::end-block


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

## Task

::block when="iterations == 1"
### Choose Course of Action

You are in the first review cycle after the implementation. During the implementation the agent was told to avoid human-in-the-loop review cycles
intermixed with the implementation where ever possible. As a result, the very first thing you are asked to do is:

- have two sub-agent review:
    - the implementation-log 
    - the spec file
- both subagents should be instructed to:
    - find any decisions which were made by the agent during the implementations
    - find any rules or decisions that the agent flagged as needing to be made
    - find any work that was deferred
- based on what you hear back you need to choose one of the following courses of action:
    1. **Human review:**
        - There are impactful/meaningful design decisions or rules that need to be made 
        - so we will report on this and stop the review cycle to allow for a human-in-the-loop review
        - Process:
            - Report back to caller using the **Writing Clearly** guidelines below
            - the reporting should include rich context, give examples where possible
            - use Mermaid code blocks for any diagrams you feel would help
            - Set the following properties on the review file's Frontmatter:
                - Synthesize your key points and set the `human_in_the_loop` Frontmatter property on this review file ({{review}}):
                    - this property will be an array of points
                    - you may use Markdown syntax to help your report be more easily digested
                    - that includes using Mermaid code blocks -- where appropriate -- for diagrams
                - set `reviewed_by` property to "{{ctx.agent}}/{{ctx.model}}" 
                - set `created` property to "{{ctx.now}}"
                - set `spec` property to "{{ parent_dir(spec) }}/{{ basename(spec) }}"
                - set `implemented` property to `false`
                - set `description` property to "A **{{feature_or_fix}}** review of `{{ parent_dir(spec) }}/{{ basename(spec) }}`"
                ::block when="next_spec"
                - set `next_spec` property to "{{next_spec}}"
                - set `next_plan` property to "{{next_plan}}"
                ::end-block

        - You are done when:
            - you have summarized -- in prose -- what needs to be reviewed back to the caller
            - and documented those same idea/points in the `human_in_the_loop` frontmatter property of the {{review}} file
            - all other required Frontmatter metadata is set on the review file

        > **Note:** the exercise of comparing the implementation to the specification is **not** done during this review when you choose this course of action; the focus is exclusively to document what we need from the caller.
    2. **Summary decisions and gather findings:**
        - Small decisions were made by the agent but nothing that really warrants a human review
        - so we will report these decisions to the caller for informational purposes, but
        - set `human_in_the_loop` property on the review file to `null` 
        - the review process will continue in a normal fashion
::end-block

## Spec Review

Read both the specification document and then perform a review on the implementation:

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality the spec asks for which is light on test coverage
- are there any changes which would make the code more ergonomic, more performant, or both?

## What blocks readiness

A review's job is to find the defects that matter, not to find every defect
it can construct. A blocking finding costs a full fix cycle and adds code; a
missed contrived edge case costs little in a repository with no external
users.

Every finding cites its **authority**, in its first paragraph:

- **spec**: the acceptance criterion, decision, or sentence of the spec (or
  design) it violates, quoted or numbered; or
- **regression**: behavior that worked on the base branch and no longer does,
  reached by a **realistic input**: one an existing call site, template, or
  fixture produces, or one a user would plausibly type.

A finding with neither is an **observation**. A behavior the spec does not
require is not made a requirement by a `docs/` page, a CommonMark or other
external standard, or a reviewer's own expectation of how the code ought to
behave.

| Priority | Meaning | Blocks `ready` |
|---|---|---|
| high | has authority and a realistic input gives a wrong result, or an acceptance criterion is unmet or untested at the level it needs | yes |
| medium | has authority, but only an unusual input reaches it | no |
| low | polish, naming, ergonomics | no |

List observations under `## Observations`, one short paragraph each, after the
findings. Defects in code this spec did not add or change are always
observations, unless this spec's change made them reachable. An observation
is information for the spec's author, who either schedules a fix or lets it
go; it never asks this cycle to repair it.

When a fix for a finding would be large next to the harm it prevents (a new
escaping or protection layer, new syntax in generated output, a matrix of
synthetic payloads), say so and recommend documenting the limitation instead.

## Sweep the class before you write

A review that reports one instance of a defect the code has in several places
is an incomplete review, and it costs a full review/fix cycle for every
sibling it leaves out. For example: five consecutive reviews each report that
one more parser coerces a missing, null, or wrong-type field to a permissive
default. One review that swept every parser for that class would have
replaced all five.

The sweep is bounded by the spec: siblings are sites this spec added or
changed, and the call sites that reach them. A sibling outside that boundary
is an observation.

For every high or medium finding:

- classify it in one sentence that names the defect class, not the file
- enumerate every sibling site within the boundary: each parser, reader,
  detector, or projection that handles the same kind of input or makes the
  same kind of decision
- check each one, using the same reproduction you used for the first (copy a
  real fixture, change one field, run the shipped CLI or public API)
- report the class as **one finding** with an instance table: site, shape
  tested, observed result, expected result; include the sites that were clean

Do not stop at the first high finding. Finish the sweep, then write.

::block when="previous_review"
### Recurrence

Compare each finding in this review with the findings of every earlier review
in this directory. If a finding belongs to the same class as an earlier one:

- say so under a `## Recurrence` heading, naming the earlier review and finding
- state which sibling sites that fix should have swept and did not
- sweep them all now, within the spec boundary, so this review carries the
  complete list
- set the frontmatter property `recurrence` to `true`; the review loop does not
  stop on it, so the next repair cycle implements exactly the list this review
  carries

A class that keeps recurring because each repair adds new machinery, which the
next review then finds a new edge in, is a sign the requirement has no
boundary. Say so, and mark the finding medium unless an acceptance criterion
names the failing case.
::end-block

::file "../_input-robustness.md"

::file ../_writing-clearly.md 

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

A feature MAY be marked production-ready only when each user-observable requirement of the spec has at minimum
the level of verification appropriate for it. Reviewers MUST list any requirement whose strongest
test is at the wrong level under "Findings" with severity at least "high".

::file "../_test-tiers.md"

## Closure

### Review Frontmatter

- Save your review suggestions/findings to "@{{review}}"
- Save the following frontmatter properties to the review file (@{{review}}):
    - set `$schema` to "feature-review.yaml"
    ::file "../_ready.md"
    - set `reviewed_by` property to "{{ctx.agent}}/{{ctx.model}}" 
    - set `recurrence` to `true` when any finding repeats the class of a finding in an earlier review (see "Recurrence" above), otherwise `false`
    - set `created` property to "{{ctx.now}}"
    - set `spec` property to "{{ parent_dir(spec) }}/{{ basename(spec) }}"
    - set `implemented` property to `false`
    - set `description` property to "A **{{feature_or_fix}}** review of `{{ parent_dir(spec) }}/{{ basename(spec) }}`"
    - set the `{{feature_or_fix}}` property to "{{ parent_dir(review) }}/{{ basename(review) }}"
    ::block when="iteration > 1"
    - set the `previous` property to "{{parent_dir(previous_review)}}/{{basename(previous_review)}}"
    ::end-block
::block when="iteration >  1"
- Now set the frontmatter properties of the _previous review_ located at @{{previous_review}}:
    - set the `next` property on the _previous review_ to "{{parent_dir(review)}}/{{basename(review)}}"
    - set the `implemented` property to `true`
::end-block

### Spec Frontmatter

The spec file which underpins the requirements for this feature review needs to have it's frontmatter properties updated too:

- the spec file is located at: {{spec}}
- Set the `review_iterations` Frontmatter property to '{{iteration}}'
- if you set the review's `ready` property to `true` then set the spec file's `completed` property to `true`


::block when="iteration != 1"
> **Note:** this is _not_ the first review we've done on this functionality but the prior review's suggestions have now all been implemented (or at least the developer has claimed that they are).
::end-block
::block when="iteration == 1"

### Human in the Loop?

This is the first review after the implementation completed:

- during this implementation the agent was strongly encouraged to complete the work including making small design decisions where there was a gap
- this means that there may now be several "decisions" that the agent made without a review or things that were deferred because a decision was needed. If either condition exists then we will stop now to get feedback:
    - report all decisions/rules/deferred scope that needs to be reviewed
    - take the time to write this clearly, giving examples where possible
    - avoid jargon and do not assume that the caller knows this repo
    - YOU MUST set the `human_in_the_loop` to `true` on the review file ({{review}})
- if no review is required by the caller then you must set `human-in-the-loop` to `false` on the review file ({{review}})

::end-block

::block when="iteration == 1 && !is_empty('next')"

### Spec and Plan Refresh

We are in a multi-plan based implementation:

- the next spec is '{{next}}', it has a plan located at '{{next_plan}}'
- both the **spec** and the **plan** were written _before_ the current spec was implemented
- we must now refresh the next spec and plan based on:
    - what we've learned during the implementation of the current phase
    - 

::end-block

### Summarize

- Summarize to the caller what was found and be sure to mention whether the review deemed the {{feature_or_fix}} to be **production ready** or not.


**IMPORTANT:**

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
