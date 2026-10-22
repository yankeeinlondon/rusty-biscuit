---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification file providing the basis for this review's findings
    design: file(match(**/*design*.md)) -> the design file (_optional_) that compliments the spec
    iteration: number -> the review's iteration number
    review: file -> the review file which will be created based on this prompt's execution
    spec_name: string -> the name of the spec (which is really just the spec's folder name)
description: |-
    Reviews a _feature specification_ to make sure that the specification has been fully implemented. This prompt is also aware of the likelihood of more than one review being necessary and therefore names the reviews `review-{iteration}.md` in the same folder where the feature was specified.

    > **Note:** the caller _can_ pass in the **iteration** number but it should be detected automatically.

    **First Review:** in the first review cycle, the very first thing we consider is whether the implementation surfaced a critical decision the author must make.

    - if it did, this review is repurposed to document those decisions; `just fix` then opens an interactive session with the author to settle them
    - if it did not, this review is conducted like a normal review that compares the implementation against what was specified

    Refreshing later specs in a multi-spec pipeline is not done here: the implementation's final phase does it.

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
log: "{{ dirname(spec) + '/implementation-log.md' }}"

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
    - when: "!err && file_exists(review) && length(frontmatter(review, 'human_in_the_loop') || []) > 0"
      then:
          - info: |-
                the first review after the implementation of {{ link(spec) }} found decisions that need the author:

                {{ as_ordered_list(frontmatter(review, 'human_in_the_loop')) }}
          - message: "🤷  the first review of `{{spec_name}}` found **{{ length(frontmatter(review, 'human_in_the_loop')) }}** decision(s) that need the author; `just fix` opens an interactive session to settle them"
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
  Findings` with its original ID and what remains. Any open finding, whatever
  its priority, keeps `ready: false` (see "What blocks readiness"); a pending
  human review alone does not.
::end-block

## Task

::block when="iteration == 1"
### Choose Course of Action

You are in the first review cycle after the implementation. The implementing
agents were told to decide rather than ask, to record every decision (with the
alternatives, why, and the downsides) in the implementation log, and to list a
truly critical decision in the log's `decisions_for_author`. So the very first
thing you do is:

- have two sub-agents review:
    - the implementation log ({{log}}), including its `decisions_for_author` frontmatter
    - the spec file
- both sub-agents should be instructed to:
    - find any decisions which were made by the agent during the implementation
    - find any decision that the agent flagged for the author
    - find any requested scope that was deferred, narrowed, or left "blocked on a ruling"

Deferred scope is never a question for the author: deferring requested scope is
forbidden, so each instance is a **high finding** in this review, and the
repair implements it.

::file ../_decisions.md

Then choose one of these courses of action:

1. **Human review:** at least one decision meets the bar in "When a human is needed" above (an item in `decisions_for_author` is a candidate; check it against the bar rather than passing it through).
    - a decision the agent recorded with its alternatives and reasoning does not qualify just because the author has not seen it, and neither does a recommendation the spec or plan already made
    - Process:
        - Report back to caller using the **Writing Clearly** guidelines below
        - the reporting should include rich context, give examples where possible
        - use Mermaid code blocks for any diagrams you feel would help
        - Set the following properties on the review file's Frontmatter:
            - set `human_in_the_loop` to a list with one entry per decision (a YAML `|-` block each): what the decision is, why it is critical, the options with pros and cons, and your recommendation; Markdown, including Mermaid code blocks, is allowed
            - set `human_resolved` to `false`
            - set `ready` to `false`, and `findings` to the deferred-scope findings (an empty list when there are none)
            - set `reviewed_by` property to "{{ctx.agent}}/{{ctx.model}}" 
            - set `created` property to "{{ctx.now}}"
            - set `spec` property to "{{ parent_dir(spec) }}/{{ basename(spec) }}"
            - set `implemented` property to `false`
            - set `description` property to "A **{{feature_or_fix}}** review of `{{ parent_dir(spec) }}/{{ basename(spec) }}`"

    - You are done when:
        - you have summarized -- in prose -- what needs to be reviewed back to the caller
        - and documented those same decisions in the `human_in_the_loop` frontmatter property of the {{review}} file
        - all other required Frontmatter metadata is set on the review file

    > **Note:** the exercise of comparing the implementation to the specification is **not** done during this review when you choose this course of action; the focus is exclusively to document what we need from the author. The next review does the full comparison after the author's decisions are applied.
2. **Summarize decisions and gather findings:** no decision meets the bar
    - report the agent's decisions to the caller for informational purposes
    - set `human_in_the_loop` property on the review file to `null` 
    - the review process continues in the normal fashion
::end-block

## Spec Review

Read the specification and any supplied design document, then review the implementation:

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality the spec asks for which is light on test coverage
- are there any changes which would make the code more ergonomic, more performant, or both?

## What blocks readiness

A review's job is to find the defects that matter, not to find every defect
it can construct. The review/fix cycle is complete only when every finding,
high, medium, and low, is fixed, so every finding costs a fix cycle and adds
code; a missed contrived edge case costs little in a repository with no
external users.

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
| medium | has authority, but only an unusual input reaches it | yes |
| low | polish, naming, ergonomics | yes |

Priority orders the repair work (high first); it never decides whether the
cycle can end.

List observations under `## Observations`, one short paragraph each, after the
findings. Defects in code this spec did not add or change are always
observations, unless this spec's change made them reachable. Observations do
not block `ready`, but the implementation repairs every observation that
describes a defect in the same cycle. So write each one as a defect report:
where it is, the input that reaches it, and the wrong result. An observation
that describes no defect (a note about future work, a naming remark) says so
in its first sentence.

When a fix for a finding would be large next to the harm it prevents (a new
escaping or protection layer, new syntax in generated output, a matrix of
synthetic payloads), say so and describe the smallest repair that removes the
harm; the implementation fixes it either way.

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

This is the first review after the implementation completed. Make sure
`human_in_the_loop` on the review file ({{review}}) matches the course of action
you chose above: the list of decisions for the author, or `null`.

::end-block

### Summarize

- Summarize to the caller whether the review deemed the {{feature_or_fix}} to be **production ready**, the new findings by ID, and (after the first review) the outcome of every previous finding, distinguishing a partial repair from a newly found sibling.


**IMPORTANT:**

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
