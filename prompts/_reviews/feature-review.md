---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification file providing the basis for this review's findings
    design: file(match(**/*design*.md)) -> the design file (_optional_) that compliments the spec
    iteration: number -> the review's iteration number
    review: file -> the review file which will be created based on this prompt's execution
    spec_name: string -> the name of the spec (which is really just the spec's folder name)
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
    
    2. **Next-spec refresh** (_in a multi-spec pipeline_)

        In the first review -- but _only_ when a `next_spec` is passed and the review did not stop for a blocking decision -- the next spec and plan are
        refreshed against what this spec actually built, and the outcome is recorded as `next_spec_refresh` on the review file.
    
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
        next_spec
            ? replace(next_spec, "spec","plan")
            : null
    }}


feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
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
    # The review file is read here, after the agent wrote it; nothing upstream
    # sets these values. Only a blocking decision stops the pipeline.
    - when: "!err && file_exists(review) && length(frontmatter(review, 'human_in_the_loop') || []) > 0"
      message: |-
        - the first review after the implementation of `{{spec_name}}` found decisions that block the pipeline:

        {{ as_ordered_list(frontmatter(review, 'human_in_the_loop')) }}
      info: |-
          - the first review after the implementation of {{ link(spec) }} found decisions that block the pipeline:

          {{ as_ordered_list(frontmatter(review, 'human_in_the_loop')) }}
      stop: null
    - when: "!err && next_spec && file_exists(review) && frontmatter(review, 'next_spec_refresh') == 'refreshed'"
      message: |-
        - the next spec in the pipeline, `{{parent_dir(next_spec)}}`, and its plan were refreshed:

        {{ frontmatter(review, 'next_spec_refresh_summary') }}
      info: |-
        - the next spec in the pipeline, {{link(next_spec)}}, and its plan were refreshed:

        {{ frontmatter(review, 'next_spec_refresh_summary') }}
    - when: "!err && next_spec && file_exists(review) && frontmatter(review, 'next_spec_refresh') == 'not-needed'"
      message: |-
        - the next spec in the pipeline, `{{parent_dir(next_spec)}}`, was checked and needed no refresh
      info: |-
        - the next spec in the pipeline, {{link(next_spec)}}, was checked and needed no refresh
    - when: "!err && next_spec && file_exists(review) && frontmatter(review, 'next_spec_refresh') == 'blocked'"
      message: |-
        - refreshing the next spec in the pipeline, `{{parent_dir(next_spec)}}`, hit a blocking decision:

        {{ frontmatter(review, 'next_spec_refresh_summary') }}
      info: |-
        - refreshing the next spec in the pipeline, {{link(next_spec)}}, hit a blocking decision:

        {{ frontmatter(review, 'next_spec_refresh_summary') }}
      stop: null
---
# Review of {{title_case(without_date(parent_dir(spec)))}}

> - {{capitalize(feature_or_fix)}}: `{{parent_dir(spec)}}`
> - Review File (_output_): `@{{review}}`
> - Review Iteration: #{{iteration}}

::file ../_senior-reviewer.md

## Context

You are performing a review of the functionality defined by the `{{spec_name}}` spec:

::block when="spec"
- **Specification:** "@{{spec}}"
::end-block
::block when="design"
- **Technical Design:** "@{{design}}"
::end-block

::block when="previous_review"
### Previous Finding Verification

This is iteration #{{iteration}} of the review/fix cycle. Read @{{previous_review}}
and its implementation log (the repair's `### Finding Ledger`) before reviewing
new work. Completion flags and log summaries are claims to verify against the
implementation, not evidence.

Write a `## Previous Finding Verification` table with these columns: finding ID
and title, required instances, outcome, verification evidence, and what remains.
Include every finding from the previous review and any still-open finding from
earlier reviews. Observations are not findings and are not carried.

- Use outcomes **verified fixed**, **partially fixed**, **regressed**, **still
  blocked**, **newly unblocked**, or **unresolved**. A finding is verified fixed
  when every required instance is checked; the ledger's child rows are that list.
- Do not reopen accepted decisions. For a blocked finding, cite the decision or
  changed prerequisite that unblocked it; newly unblocked work that is already
  repaired can be verified fixed.
- Carry every remaining defect into `## Unblocked Findings` or `## Blocked
  Findings` with its original ID and what remains. Its priority still decides
  whether it blocks `ready` (see "What blocks readiness"): an open high finding
  keeps `ready: false`; an open medium or low one does not, and a pending human
  review alone does not.
::end-block

## Task

::block when="iteration == 1"
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
- classify each decision with "Rulings, Defaults, and Pre-authorizations" below; the pipeline runs unattended, so **course 2 is the default** and course 1 is rare
    - a decision that has a recommended option, a planning default, or one option clearly consistent with the spec is **not** a reason to stop: adopt the recommendation (or accept what the implementer built when it follows one) and record it under `## Decisions adopted for author review` in the review body, with the alternatives and why
    - an implementer adopting a recommendation that the spec said "requires author resolution" is a decision to record, not a reason to stop
    - deferred work the spec allows to be deferred is recorded, not escalated
- based on what you hear back you need to choose one of the following courses of action:
    1. **Human review** (_only for blocking decisions_):
        - at least one decision is **blocking**: no recommendation or default exists, no option is clearly consistent with the spec, and a wrong choice would be costly to undo once the next spec builds on it
        - only the blocking decisions go into `human_in_the_loop`; record every other decision as adopted
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
    2. **Summary decisions and gather findings** (_the default_):
        - decisions were made or adopted, but none is blocking; this holds even when they are numerous or consequential
        - record them under `## Decisions adopted for author review` so the author can revisit any of them later; a reversal becomes an ordinary fix
        - set `human_in_the_loop` property on the review file to `null` 
        - the review process will continue in a normal fashion, including the next spec's refresh
::end-block

::file "../_rulings.md"

## Spec Review

Read the specification and any supplied design document, then review the implementation:

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

- give it a stable ID, `R{{iteration}}-F1`, `R{{iteration}}-F2`, …, in its heading
  and its frontmatter `findings` entry. A carried finding keeps its original ID;
  a newly found sibling of an earlier finding gets a new ID that names the earlier
  one. For a legacy review without IDs, use its filename plus the finding title
- classify it in one sentence that names the defect class, not the file
- enumerate every sibling site within the boundary: each parser, reader,
  detector, or projection that handles the same kind of input or makes the
  same kind of decision
- check each one, using the same reproduction you used for the first (copy a
  real fixture, change one field, run the shipped CLI or public API)
- report the class as **one finding** with an instance table: child ID
  (`R{{iteration}}-F1.1`, …), site, shape tested, observed result, expected
  result. The repair's finding ledger uses these IDs row for row
- include the sites that were clean. A site is clean when its reproduction
  gives the expected result, or a stated contract or invariant in the code rules
  the defect out; agreement between two code paths alone does not show it
- when the defect is in a value a later decision reads (an identity, scope,
  category, or count), follow that value to the decisions within the boundary
  that accept, reject, or classify with it. Each one is a site; the reader
  alone is not the sweep

Low findings get an ID too, without a sweep.

Do not stop at the first high finding. Finish the sweep, then write. Every
blocking finding goes in the titled findings and the frontmatter `findings`
list, including one discovered in a robustness or verification section, so the
repair inventory is complete.

::block when="previous_review"
### Recurrence

Compare each finding in this review with the findings of every earlier review
in this directory. If a finding belongs to the same class as an earlier one:

- say so under a `## Recurrence` heading, naming the earlier review and finding
- state whether it is an unchanged instance, a regression the repair caused, or
  a sibling the earlier sweep missed, and which sites that fix should have
  swept and did not
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
    - leave its `implemented` and `implemented_by` unchanged: they record the
      implementer's claim, and this review's verification table records your
      own assessment
::end-block

### Spec Frontmatter

The spec file which underpins the requirements for this feature review needs to have it's frontmatter properties updated too:

- the spec file is located at: {{spec}}
- Set the `review_iterations` Frontmatter property to '{{iteration}}'
- if you set the review's `ready` property to `true` then set the spec file's `completed` property to `true`


::block when="iteration == 1"

### Human in the Loop?

This is the first review after the implementation completed:

- during this implementation the agent was strongly encouraged to complete the work including making small design decisions where there was a gap
- this means that there may now be several "decisions" that the agent made without a review or things that were deferred because a decision was needed
    - record every one of them under `## Decisions adopted for author review`, clearly and with examples, avoiding jargon and without assuming the caller knows this repo
    - stop for feedback **only** when at least one is blocking (see "Choose Course of Action"); then put just the blocking decisions in `human_in_the_loop`
- otherwise set `human_in_the_loop` to `null` on the review file ({{review}}) and continue

::end-block

::block when="iteration == 1 && !is_empty(next_spec)"

### Spec and Plan Refresh

We are in a multi-plan based implementation:

- the next spec is '{{next_spec}}', it has a plan located at '{{next_plan}}'
- both the **spec** and the **plan** were written _before_ the current spec was implemented
- we must now refresh the next spec and plan based on:
    - what was learned and built while implementing this spec (its implementation log, the landed code, and this review's findings)
    - every decision recorded in this review, adopted or ruled
- skip this section only when you chose **Human review** above; the next spec then stays `refresh_required: true` until the blocking decisions are ruled

Refresh the next spec ({{next_spec}}):

- check its "Refresh notes" (or similar) section and every assumption it makes about this spec: symbol names, file paths, contracts, serialized formats, counts, version constants, and builder or runtime surfaces; verify each against the landed code, not against this spec's text
- correct stale facts in place and add a dated reader note summarizing what changed and why; keep decided requirements unless the landed code forces a change, and say so when it does
- resolve conflicts under "Rulings, Defaults, and Pre-authorizations": adopt a recommendation or clearly consistent option and record it; add an Open Question only for a blocking one
- set its `refresh_required` frontmatter property to `false`

Refresh the next plan ({{next_plan}}):

- update rules and tasks that cite stale names, files, counts, or prerequisites; a prerequisite gate that the landed code now satisfies must say so, listing the actual symbols and locations, and must not wait on rulings already recorded
- if its implementation log already exists from a run that stopped at a prerequisite gate, set that log's `human_review` to `false`, remove its `human_review_items` and `message_to_agent`, append a dated note explaining the refresh, and set the plan's `phase` back to the first phase that did no real work

Record the outcome on this review file ({{review}}):

- `next_spec_refresh`: `refreshed`, `not-needed`, or `blocked` (only when a blocking decision stops the refresh)
- `next_spec_refresh_summary`: a short Markdown list of what changed, or of the blocking decision

::end-block

### Summarize

- Summarize to the caller whether the review deemed the {{feature_or_fix}} to be **production ready**, the new findings by ID, and (after the first review) the outcome of every previous finding, distinguishing a partial repair from a newly found sibling.


**IMPORTANT:**

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
