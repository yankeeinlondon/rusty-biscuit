---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification file providing the basis for this review's findings
    design: file(match(**/*design*.md)) -> the design file (_optional_) that compliments the spec
    iteration: number -> the review's iteration number
    review: file -> the review file which will be created based on this prompt's execution
    spec_name: string -> the name of the spec (which is really just the spec's folder name)
    in_loop: boolean -> set by the `review-loop` sequence; when true this review opts out if the previous review already ended the loop
    next_spec: file(match(**/*spec*.md)) -> if the current spec is part of a multi-spec pipeline then this points to the NEXT spec; by default it is resolved from the `next_spec` property in the current spec's frontmatter (a `{date}-{name}` directory name, found beside the current spec), and a caller may pass a path to override it
    next_plan: file(match(**/*plan*.md)) -> the plan that sits beside `next_spec` (it may not exist yet)
description: |-
    Reviews a _feature specification_ to make sure that the specification has been fully implemented. This prompt is also aware of the likelihood of more than one review being necessary and therefore names the reviews `review-{iteration}.md` in the same folder where the feature was specified.

    > **Note:** the caller _can_ pass in the **iteration** number but it should be detected automatically.

    There are two special cases in the review cycle that can be activated:

    1. **First Review**

        When we are in the first review cycle, the very first thing we will consider is whether there is a genuine need for a human-in-the-loop review. The test is explicit (see "Choose Course of Action") and defaults to _no_, because a stop costs the pipeline time.

        - if there is then this review is repurposed to collect the author's rulings: it sets `human_in_the_loop` in the review file, marks the spec `human-in-the-loop`, and **halts the review loop** (the lifecycle raises an error, because a bare `stop` would let the sequence carry on). The next run refuses to start until the human has recorded rulings and set `human_resolved: true`; the rulings then carry the same authority as the spec
        - if there is not then this review will be conducted like a normal review that compares the implementation against what was specified

    2. **Last Review** (_in a multi-spec pipeline_)

        In the last review cycle (aka, when `ready` has been set to `true`) -- but _only_ if this spec is part of a multi-spec pipeline -- we will
        look at whether the next spec and plan need to be refreshed based on the current spec's findings and decisions.

        - the pipeline is declared by the `next_spec` property in each spec's frontmatter: the `{date}-{name}` directory of the next spec, looked up beside the current one. The last spec has none, so it is not in pipeline mode. A named successor that cannot be found fails the run before the agent launches
        - the reviewer records the outcome in the review file as `refresh`: `refreshed` and `not_needed` let the pipeline continue; `needs_human` halts it

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
# The spec names its successor by `{date}-{name}` directory (the repo rule for
# cross-references: a path through a lifecycle directory goes stale when a spec
# is closed). The successor is looked up beside this spec, then in `_completed`
# (and from `_completed` back to its active sibling). `dirname` is not nested
# because a second call re-anchors a relative path on this prompt's directory,
# and a repo-wide glob works but costs seconds on every composition.
next_name: "{{ file_exists(spec) ? frontmatter(spec, 'next_spec') : null }}"
next_spec: |-
    {{
        !next_name
            ? null
            : contains(next_name, '/')
                ? next_name
                : file_exists(replace_last(dir, spec_name, next_name) + '/spec.md')
                    ? replace_last(dir, spec_name, next_name) + '/spec.md'
                    : file_exists(replace_last(dir, spec_name, '_completed/' + next_name) + '/spec.md')
                        ? replace_last(dir, spec_name, '_completed/' + next_name) + '/spec.md'
                        : contains(dir, '_completed/' + spec_name) && file_exists(replace_last(dir, '_completed/' + spec_name, next_name) + '/spec.md')
                            ? replace_last(dir, '_completed/' + spec_name, next_name) + '/spec.md'
                            : null
    }}
next_plan: "{{ next_spec ? replace_last(next_spec, 'spec.md', 'plan.md') : null }}"

feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
initialize:
    # A spec that names a successor nobody can find would silently leave
    # pipeline mode off; fail before the agent launches instead.
    - when: "next_name && !next_spec"
      then:
          - error: "`{{spec_name}}` names `{{next_name}}` as its next spec, but no `spec.md` was found for it beside this spec or in `_completed`"
    # Inside the review-loop sequence the step list is static, so a review
    # whose predecessor already closed the loop opts out here and launches
    # nothing. A direct `claudine compose` never sets `in_loop`.
    # A review that stopped for a human never assessed readiness, so its
    # `ready` cannot end the loop even if a reviewer wrote `true`.
    - when: "in_loop && previous_review && file_exists(previous_review) && frontmatter(previous_review, 'ready') == true && is_empty(frontmatter(previous_review, 'human_in_the_loop'))"
      then:
          - message: "review #{{iteration}} skipped: {{basename(previous_review)}} already marked `{{parent_dir(spec)}}` production ready"
          - skip
    # A review that stopped for a human is not finished until the human has
    # recorded rulings in it. Running again before then would review an
    # implementation whose open questions are still unanswered.
    - when: "previous_review && file_exists(previous_review) && !is_empty(frontmatter(previous_review, 'human_in_the_loop')) && frontmatter(previous_review, 'human_resolved') != true"
      then:
          - warn: "{{basename(previous_review)}} is waiting on a human: record your rulings under its `## Human rulings` heading, set `human_resolved: true` in its frontmatter, then run the review again"
          - error: "{{basename(previous_review)}} is waiting on a human"
start:
    message: "🏃‍♂️ starting review #{{iteration}} of `{{parent_dir(spec)}}` (_{{feature_or_fix}} in the **{{ctx.area || ctx.repo}}**_)"
success:
    # The reviewer stopped for a human. `finalize` halts the loop; here we
    # only say why, so the stop is not read as a failed review.
    - when: "file_exists(review) && !is_empty(frontmatter(review, 'human_in_the_loop'))"
      then:
          - warn: |-
                review #{{iteration}} of `{{parent_dir(spec)}}` in the {{ctx.area}} package area stopped for a human: <blue>{{link(review)}}</blue>
          - message: |-
                🛑  review #{{iteration}} of `{{parent_dir(spec)}}` in **{{ctx.area || ctx.repo}}** stopped for a human. Items needing your decision:

                {{ as_ordered_list(frontmatter(review, 'human_in_the_loop') || []) }}
          - stop
    - when: "file_exists(review) && frontmatter(review,'ready') == true"
      then:
          - success: "{{feature_or_fix}} review {{iteration}} of `{{ parent_dir(spec) }}` in **{{ctx.area}}** finished and deemed code to be **production ready**"
          - message: |-
                ✅  review #{{iteration}} of `{{parent_dir(spec)}}` in **{{ctx.area || ctx.repo}}** completed successfully (_**production ready**_)
          - effect: small-group-cheer
    - when: "file_exists(review) && frontmatter(review,'ready') == true && frontmatter(review,'human_review') == true"
      then:
          - message: |-
                🤷  {{feature_or_fix}} review of `{{ parent_dir(spec) }}` -- while production ready -- requires human review. Review items include:

                {{ as_ordered_list(frontmatter(review,"human_review_items") || []) }}
          - info: |-
                review #{{iteration}} of `{{ upper(feature_or_fix) + ' ' + parent_dir(spec) }}` -- _while production ready_ -- requires human review ({{length(frontmatter(review, 'human_review_items') || [])}} items):

                {{ as_ordered_list(frontmatter(review, 'human_review_items') || []) }}
    - when: "file_exists(review) && frontmatter(review,'ready') == false"
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
    # A deliberate halt for a human is raised by `finalize`; do not report
    # it as a crashed review.
    - when: "file_exists(review) && (!is_empty(frontmatter(review, 'human_in_the_loop')) || frontmatter(review, 'refresh') == 'needs_human')"
      then:
          - stop
    - stderr: |-
          {{feature_or_fix}} review {{iteration}} for `{{parent_dir(spec)}}` in the {{ctx.area}} package area failed to complete!
    - message: |-
          💥 {{feature_or_fix}} review #{{iteration}} for `{{parent_dir(spec)}}` in **{{ ctx.area || ctx.repo }}** failed to complete ({{err.msg}})!
    - effect: phase-jump-3
finalize:
    # `stop` ends only this lifecycle event; the review-loop sequence carries
    # on to the next step. Only a failing step halts a `fail_fast`
    # sequence, so every "a human must look" outcome raises `error`.
    - when: "!err && (!file_exists(review) || (!has_key(frontmatter(review), 'ready') && is_empty(frontmatter(review, 'human_in_the_loop'))))"
      then:
          - warn: "the reviewer recorded neither `ready` nor `human_in_the_loop` in {{review}}"
          - error: "review #{{iteration}} of `{{spec_name}}` left no outcome in {{review}}"
    - when: "!err && !is_empty(frontmatter(review, 'human_in_the_loop'))"
      then:
          - info: |-
                - the first review after the implementation of {{ link(spec) }} was conducted and there are decisions that require your attention:

                {{ as_ordered_list(frontmatter(review, 'human_in_the_loop') || []) }}

                Record your rulings in {{ link(review) }} under `## Human rulings`, set `human_resolved: true` in its frontmatter, and run the review again.
          - message: |-
                - the first review after the implementation of `{{spec_name}}` was conducted and there are decisions that require your attention:

                {{ as_ordered_list(frontmatter(review, 'human_in_the_loop') || []) }}
          - error: "review #{{iteration}} of `{{spec_name}}` needs a human before the review loop can continue: see {{review}}"
    - when: "!err && next_spec && frontmatter(review, 'ready') == true && is_empty(frontmatter(review, 'refresh'))"
      then:
          - warn: "{{review}} is marked ready but records no `refresh` outcome for the next spec, `{{parent_dir(next_spec)}}`"
          - error: "review #{{iteration}} of `{{spec_name}}` is ready but the refresh of `{{parent_dir(next_spec)}}` was not recorded in {{review}}"
    - when: "!err && next_spec && frontmatter(review, 'refresh') == 'refreshed'"
      then:
          - info: |-
                - we have finished the review/fix cycle for {{link(spec)}} but we are in a multi-spec pipeline
                - the next spec which will be implemented is {{link(next_spec)}}
                - the spec and plan have been refreshed:

                {{ as_ordered_list(frontmatter(review, 'refresh_changes') || []) }}

                > **Note:** no human review was needed for this refresh
          - message: |-
                - we have finished the review/fix cycle for `{{spec_name}}` but we are in a multi-spec pipeline
                - the next spec which will be implemented is `{{parent_dir(next_spec)}}`
                - the spec and plan have been refreshed:

                {{ as_ordered_list(frontmatter(review, 'refresh_changes') || []) }}

                > **Note:** no human review was needed for this refresh
    - when: "!err && next_spec && frontmatter(review, 'refresh') == 'needs_human'"
      then:
          - info: |-
                - we have finished the review/fix cycle for {{link(spec)}} but we are in a multi-spec pipeline
                - the next spec which will be implemented is {{link(next_spec)}}
                - refreshing it needs decisions that only a human can make:

                {{ as_ordered_list(frontmatter(review, 'refresh_human_items') || []) }}

                Nothing contested was changed. Record your rulings in {{ link(review) }} under `## Human rulings`, then refresh `{{parent_dir(next_spec)}}` before implementing it.
          - message: |-
                - we have finished the review/fix cycle for `{{spec_name}}` but we are in a multi-spec pipeline
                - the next spec which will be implemented is `{{parent_dir(next_spec)}}`
                - refreshing it needs decisions that only a human can make:

                {{ as_ordered_list(frontmatter(review, 'refresh_human_items') || []) }}
          - error: "the refresh of `{{parent_dir(next_spec)}}` after `{{spec_name}}` needs a human: see {{review}}"
    - when: "!err && next_spec && frontmatter(review, 'refresh') == 'not_needed'"
      then:
          - info: |-
                - we have finished the review/fix cycle for `{{spec_name}}` and are in a multi-spec pipeline
                - the next _spec_ and _plan_ -- {{parent_dir(next_spec)}} -- was evaluated but did not need to be refreshed to be ready for implementation
          - message: |-
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
- The previous review is "@{{previous_review}}"
- The first thing you will need to check is whether the _findings_ in that review (its `findings` frontmatter property and the findings in its body) were all implemented
    - this is the most obvious thing that the last implementation was supposed to have addressed
- If that review stopped for a human (it has a `human_in_the_loop` property), the human recorded **rulings** under its `## Human rulings` heading
    - a ruling is a decision of the author: it has the same authority as the specification and may be cited as the authority for a finding
    - where a ruling and the specification disagree, the ruling wins; say so in the review
    - **every ruling that requires a change is a finding until the change is done.** Check each one against the code, tests, docs, and skills. One not fully carried out is a **high** finding whose authority is the ruling (cite it as "{{basename(previous_review)}}, item N"), listed in `findings` like any other high finding, so the repair step implements it. A ruling that only confirms existing behavior needs no finding

::end-block


::block when="spec"
- **Specification:** "@{{spec}}"
::end-block
::block when="design"
- **Technical Design:** "@{{design}}"
::end-block

::block when="spec && design"
Read both the specification and the design document, then review the implementation.

::end-block
::block when="spec && !design"
Read the specification document, then review the implementation.

::end-block

## Task

::block when="iteration == 1"
### Choose Course of Action

This is the first review after the implementation. The implementing agent ran unattended: it was told to avoid stopping for a human and to finish the work, taking small design decisions itself and recording them. Stopping for a human costs the pipeline time, so you stop **only for a genuine need**, and you decide that **before** you compare the implementation with the specification.

#### Step 1: gather what the implementer decided

Have two sub-agents read, in parallel:

- the implementation log (`{{dir}}/implementation-log.md`)
- the specification, including its `## Human review agenda`, its `message_to_agent` frontmatter property, and every item marked `Proposed` or `Default if not ruled before planning`

Each reports:

- every decision the implementer made where the spec did not dictate the answer, including every default it adopted
- every rule or decision the implementer flagged as needing the author
- every piece of work it deferred or left undone, and why
- anything it could not verify because only a human can (real accounts, devices, credentials, hosts)

Give them the exact paths. They should return lists, not prose summaries.

#### Step 2: classify each item

An item **needs a human** only when at least one of these is true:

1. **Hard to reverse:** it fixes a public API, an error or wire format, a persisted format, or a name that dependents or later specs will build on, and changing it later means rework across crates or specs.
2. **Contradicts a decision:** it departs from something the spec marks Decided, or from an author ruling, instead of choosing among options the spec left open.
3. **Changes a later spec:** it moves scope, a dependency, or an assumption of a later spec in this pipeline in a way only the author can accept.
4. **Safety or trust:** it sets a default that affects credentials, network reach, data loss, or security beyond what the spec states.
5. **No safe default:** either answer is defensible, the wrong one is costly, and the spec supplies no default.
6. **Human-only verification:** the spec requires evidence that only a human can produce.

An item **does not need a human** when it is any of:

- a default the spec itself supplied and the implementer adopted as written
- a choice that is cheap to change later: names inside one crate, test layout, internal structure
- polish or deferred work that does not change a later spec

List these for information; never stop for them. Two guards against over-asking:

- an item whose answer would be "go with the default" is not a question; record it as informational
- do not ask the human to reaffirm a decision the author already made; ask only when something changed the premise of the decision, and then quote the decision's text and say what changed

#### Step 3: choose one course of action

1. **Human review:** at least one item meets the test above.
    - Stop the review cycle and report to the caller, using the **Writing Clearly** guidelines below. Give rich context and examples; use Mermaid code blocks for any diagram that helps.
    - Write at most **7** items, ranked by how much rework a wrong default would cause. If more meet the test, keep the 7 with the widest blast radius and name the rest in the body of the review.
    - Each item must say, in plain language that assumes no knowledge of this repository:
        - what it is
        - which test above it meets
        - the options, each with pros and cons
        - your recommendation and why
        - what happens if the human answers "go with your recommendation"
        - what is blocked or put at risk if it stays undecided
    - Set these properties in the frontmatter of the review file ({{review}}):
        - `human_in_the_loop`: an array of those items; use YAML's `|-` for any item longer than a sentence, and Markdown (including Mermaid) inside the items
        - `human_resolved`: `false` (the human sets it to `true` after recording rulings; the next review will not start until then)
        - `ready`: `false`, and `findings`: `[]`. Readiness was not assessed, because the implementation was never compared with the specification. The readiness rules under "Closure" do not apply to this review: an empty finding list here means "not reviewed", not "nothing wrong". A `ready: true` on this review would make the loop skip the next review and the repair of the rulings.
        - `reviewed_by`: "{{ctx.agent}}/{{ctx.model}}"
        - `created`: "{{ctx.now}}"
        - `spec`: "{{ parent_dir(spec) }}/{{ basename(spec) }}"
        - `implemented`: `false`
        - `description`: "A **{{feature_or_fix}}** review of `{{ parent_dir(spec) }}/{{ basename(spec) }}`"
        ::block when="next_spec"
        - `next_spec`: "{{next_spec}}"
        - `next_plan`: "{{next_plan}}"
        ::end-block
    - End the body of the review file with a `## Human rulings` heading, then one `### Item N` subheading per item containing the line `Ruling: (pending)`. That is where the human answers.
    - Update the specification's frontmatter ({{spec}}): set `review_iterations` to `{{iteration}}` and `status` to `human-in-the-loop`. Without this the next run would think it is still the first review and stop again.
    - You are done when you have summarized in prose what needs review, documented the same points in `human_in_the_loop`, set the other properties above, and updated the spec. The review loop halts by itself when `human_in_the_loop` is set; do not try to stop it.

    > **Note:** the comparison of the implementation to the specification is **not** done in this review when you choose this course; the focus is exclusively to document what we need from the caller.
2. **Summary decisions and gather findings:** no item meets the test.
    - Report the decisions the implementer made to the caller for information, and record them in the body of the review under `## Implementation decisions (informational)`.
    - Set `human_in_the_loop` in the review's frontmatter to `null`.
    - Continue with the review in the normal way.
::end-block

## Spec Review

Compare the implementation with the specification:

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

::file "../_input-robustness.md"

::file ../_writing-clearly.md 

::file "../_test-rigor.md"

::file "../_test-tiers.md"

## Closure

### Review Frontmatter

- Save your review suggestions/findings to "@{{review}}"
- Save the following frontmatter properties to the review file (@{{review}}):
    - set `$schema` to "feature-review.yaml"
    ::file "../_ready.md"
    - set `reviewed_by` property to "{{ctx.agent}}/{{ctx.model}}" 
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

The spec file which underpins the requirements for this feature review needs to have its frontmatter properties updated too:

- the spec file is located at: {{spec}}
- set the `review_iterations` Frontmatter property to '{{iteration}}'
- set the `status` property:
    - `review-findings` when this review is not ready
    - `implemented` when this review is ready
- if you set the review's `ready` property to `true` then also set the spec file's `completed` property to `true`

::block when="iteration != 1"
> **Note:** this is _not_ the first review we've done on this functionality but the prior review's suggestions have now all been implemented (or at least the developer has claimed that they are).
::end-block

::block when="next_spec"

### Spec and Plan Refresh

This spec is one stage of a multi-spec pipeline. Both the next spec and its plan were written **before** this spec was implemented, so what the implementation, this review cycle, and any human rulings taught us may change them.

- if you set `ready` to `true`: the review/fix cycle for `{{spec_name}}` is finishing, so do the refresh below
- if you did not: skip this section and set no `refresh` property

**Documents**

- the next spec: "@{{next_spec}}"
::block when="file_exists(next_plan)"
- the next plan: "@{{next_plan}}"
::end-block
::block when="!file_exists(next_plan)"
- the next plan (`{{next_plan}}`) does not exist yet; refresh the spec only and say so in the refresh log
::end-block

**Read first:** this spec's implementation log; every review in this directory, including any `## Human rulings`; any scope this spec deferred; the decisions of the spec named in the next spec's `parent` property; and the next spec's own `## Refresh notes`, which lists the outcomes of earlier specs that may change it.

**You may change without asking** (a factual refresh):

- counts, names, paths, and symbols: update them to what exists now
- statements marked unverified or assumed: replace them with what was verified
- a name or signature this spec's implementation introduced: adopt it
- a dependency on something this spec created: add it
- scope that this spec's own text assigns to the next spec: move it in
- a planning default that the implementation proved unworkable: change it and say why

Keep the edits minimal. Do not rewrite requirements for style, and do not implement anything.

**Do not change; a human must decide** (`needs_human`):

- anything the next spec marks Decided or that records an author ruling
- scope that no earlier text assigns to the next spec, added or removed
- the order or dependencies of the pipeline
- anything that would contradict a human ruling

For these, make **no** edit to the contested text. Describe the proposed change instead, to the standard of the first-review items (plain language, options with pros and cons, a recommendation).

**Record**

- append a dated entry to a `## Refresh log` section at the end of the next spec (create the section if it is missing): the date, the source (this spec's review {{iteration}} or its implementation log), and one line per change saying what changed and why; apply the same rules to the next plan when it exists
- in the next spec's frontmatter:
    - set `refresh_required` to `false` (leave it `true` when the outcome is `needs_human`)
    - set `message_to_agent` to a short plain-text note of what the next implementer must know; the implement prompt shows it to the agent at the start
- in the review file ({{review}}) set:
    - `refresh`: `refreshed` (you changed the next spec or plan, nothing contested), `needs_human` (something contested remains), or `not_needed` (you evaluated both and nothing needed to change)
    - `refresh_changes`: for `refreshed` and `needs_human`, the plain-language list of what you changed
    - `refresh_human_items`: for `needs_human`, the contested items

::end-block

### Summarize

- Summarize to the caller what was found and be sure to mention whether the review deemed the {{feature_or_fix}} to be **production ready** or not.


**IMPORTANT:**

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
