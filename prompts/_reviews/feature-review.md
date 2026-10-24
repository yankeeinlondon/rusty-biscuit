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
        next_spec
            ? replace(next_spec, "spec","plan")
            : null
    }}


feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
initialize:
    # Reviewing before the author has ruled would review code the rulings are
    # about to change.
    - when: "previous_review && file_exists(previous_review) && !is_empty(frontmatter(previous_review, 'human_in_the_loop')) && frontmatter(previous_review, 'human_resolved') != true"
      then:
          - warn: "{{basename(previous_review)}} is waiting on a human: run `just fix {{spec}}` in a terminal to record the rulings"
          - error: "{{basename(previous_review)}} is waiting on a human"
start:
    message: "🏃‍♂️ starting review #{{iteration}} of `{{parent_dir(spec)}}` (_{{feature_or_fix}} in the **{{ctx.area || ctx.repo}}**_)"
success:
    - when: "!is_empty(frontmatter(review, 'human_in_the_loop'))"
      then:
          - message: |-
                🛑  review #{{iteration}} of `{{parent_dir(spec)}}` in **{{ctx.area || ctx.repo}}** needs your rulings before the cycle can continue:

                {{ as_ordered_list(frontmatter(review, 'human_in_the_loop') || []) }}
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
earlier reviews.

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

If an earlier review in this directory has a `## Human rulings` section, each
ruling there is the author's decision, with the same authority as the spec;
where a ruling and the spec disagree, the ruling wins. A ruling that requires
a change is a **high** finding until the change is done (cite it as
"review-N.md, item M"). Never ask about a ruled item again.
::end-block

## Task

::block when="iteration == 1"
### Gather the implementer's decisions

The implementation ran unattended and was told to decide rather than stop.
Before comparing it with the spec, have two sub-agents read, in parallel, the
implementation log in `{{dir}}` (starting with its `decisions_for_review`
frontmatter list and any "Decided during implementation" heading) and the spec
(including any decision that pre-authorizes bounded implementer autonomy).
Each returns lists, not prose:

- every decision the implementer made where the spec did not dictate the answer
- every decision it flagged for the author
- every piece of work it deferred, and why

Judge each item with "Stopping for a human" below. Record every item that does
not need a human under `## Implementation decisions (informational)` in the
review body, and put a one-line summary of each in `human_review_items` (see
the readiness rules); they never stop the cycle.
::end-block

### Stopping for a human

Every stop halts the pipeline until the author is at a terminal, so the
default is **not to stop**. An item needs a human only when at least one of
these is true **and** you cannot recommend an option that stays within the
spec, its decisions, and the author's rulings:

1. **Hard to reverse:** it fixes a public API, an error or wire format, a
   persisted format, or a name that dependents or later specs build on.
2. **Contradicts a decision:** every way forward departs from something the
   spec decided or the author ruled, and the spec does not pre-authorize the
   departure.
3. **Changes scope:** it adds or removes scope, or moves work to or from
   another spec (for example, a finding that cannot be repaired until another,
   unimplemented spec lands).
4. **Safety or trust:** it sets a default affecting credentials, network
   reach, data loss, or security beyond what the spec states.
5. **No safe default:** the options trade off things the author would weigh,
   the wrong choice is costly, and the spec supplies no default.
6. **Human-only verification:** the spec requires evidence only a human can
   produce.

These never need a human:

- a decision the implementer made that you would recommend anyway, or one the
  spec pre-authorizes and whose conditions it met
- a choice that is cheap to change later (names inside one crate, test
  layout, internal structure, a test's measurement method when the stated
  purpose of the acceptance criterion is kept)
- a decision the author already made; ask again only when new evidence breaks
  its premise, and then quote the decision and say what changed
- anything whose answer would be "go with your recommendation"

A finding whose repair needs a decision that does **not** meet the test is an
ordinary, unblocked finding: state the option you recommend in the finding,
and the repair step implements it. Never mark such a finding blocked and never
write that a decision is "reserved for the author".

When at least one item meets the test:

- still do the full spec review below, so the repair after the rulings can fix
  every finding in the same cycle
- write at most **5** items to `human_in_the_loop`, ranked by the rework a
  wrong choice would cause; each item says, in plain language that assumes no
  knowledge of this repository, what it is, which test it meets, 2–4 options
  with pros and cons, your recommendation and why, and which findings wait on
  it. Use YAML's `|-` for long items; Markdown and Mermaid are allowed
- set `human_resolved` to `false`, and set `blocked` and `has_blocked_findings`
  as the readiness rules describe; a finding is blocked only when it waits on
  one of these items
- end the review body with a `## Human rulings` heading and one
  `### Item N` subheading per item containing `Ruling: (pending)`
- the `just fix` loop collects the rulings interactively and then continues;
  do not try to stop it yourself

When no item meets the test, set `human_in_the_loop` to `null`.

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
  fixture produces, or one a user would plausibly type; or
- **defect**: code you read during the review, whether or not this spec added
  or changed it, that gives a wrong result for a realistic input.

An issue with none of these is not reported. A behavior the spec does not
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

A defect in code this spec did not add or change is a finding like any other:
it gets a priority, blocks `ready`, and is repaired in the next cycle. Write it
as a defect report: where it is, the input that reaches it, and the wrong
result.

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
changed, and the call sites that reach them. A sibling you notice outside that
boundary is reported as a finding of its own.

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
finding goes in the titled findings and the frontmatter `findings`
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
    - set `human_in_the_loop` (and `human_resolved: false`) only as "Stopping for a human" describes; otherwise set `human_in_the_loop` to `null`
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


::block when="iteration == 1 && !is_empty(next_spec)"

### Spec and Plan Refresh

We are in a multi-plan based implementation:

- the next spec is '{{next_spec}}', it has a plan located at '{{next_plan}}'
- both the **spec** and the **plan** were written _before_ the current spec was implemented
- we must now refresh the next spec and plan based on:
    - what we've learned during the implementation of the current phase
    - 

::end-block

### Summarize

- Summarize to the caller whether the review deemed the {{feature_or_fix}} to be **production ready**, the new findings by ID, and (after the first review) the outcome of every previous finding, distinguishing a partial repair from a newly found sibling.


**IMPORTANT:**

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
