---
$schema:
    spec: file(required; match(**/*spec*.md)) -> the specification file defining the target functionality
    design: file(match(**/*design*.md)) -> optionally, a design file which compliments the spec file with more details on the technical design
    review: file -> the review file who's findings/suggestions we will implementing
    iteration: number(required;default(1)) -> what _iteration_ of the review/implement cycle that we are on
    log: file -> the log file we're writing to while working on the implementation
    retry: string -> if you're retrying this prompt after a failure then pass in what you know about prior progress
    in_loop: boolean -> set by the `review-loop` sequence; when true this step opts out once the review it would implement has ended the loop
description: |-
    Implements the findings in a review which was conducted to determine the drift between
    what was actually implemented versus the specification that was being targetted in that
    implementation.
iteration: "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') || 1 : 1 }}"
review: "{{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}"
log: {{ dirname(spec) + '/log.md' }}
# is the review "production ready"?
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"

design: "{{ file_exists(dirname(review) + '/design.md') ? dirname(review) + '/design.md' : null }}"
yolo: true
feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"

initialize:
    # Inside the review-loop sequence the step list is static; a repair
    # whose review is ready opts out here instead of launching an agent to
    # write one log line.
    - when: "in_loop && ready == true"
      then:
          - message: "repair of review #{{iteration}} skipped: `{{parent_dir(spec)}}` is production ready"
          - skip
    - ensure_file: '{{log}}'

start:
    message: "🏃 implementing _findings_ from review #{{ file_index(review) }} in `{{ parent_dir(review) }}` (_using_ {{ctx.agent}}/{{ctx.model}} _in_ {{ctx.area || ctx.repo}})"
success:
    - when: "ready == true"
      then:
          - message: "review #{{ file_index(review) }} of `{{ parent_dir(review) }}` is already production ready; no repair was needed"
    - when: "ready != true && frontmatter(review, 'implemented') == true"
      then:
          - message: "✅  implemented all review findings from review **#{{ file_index(review) }}** of `{{ parent_dir(review) }}`; ready for independent verification (area: **{{ctx.area || ctx.repo }}**, wt: **{{ ctx.worktree }}**, branch: **{{ctx.branch}}**)"
          - say: "the review findings in {{ without_date(parent_dir(review)) }} were implemented and are ready for review"
          - effect: bong
    - when: "ready != true && frontmatter(review, 'implemented') != true"
      then:
          - warn: "review #{{ file_index(review) }} repair of `{{ parent_dir(review) }}` ended with incomplete findings; see the disposition ledger in {{log}}"
failure:
    message: "💥 implementation of the review #{{iteration}} suggestions from **{{ parent_dir(review) }}** failed to complete ({{err.msg}})!"
    effect: sad-trombone
---
::file "../_no_formatting.md"

::file "../_frontmatter_edits.md"

# Implement Review Suggestions for {{title_case(without_date(parent_dir(spec)))}}

> - **{{capitalize(feature_or_fix)}}:** `{{parent_dir(review)}}`
> - **Iteration:** {{iteration}}
> - **Log File:** {{log}}

## Key Documents

- **Review:** @{{review}}
- **Iteration:** {{file_index(review)}}
- **Specification:** {{contains(spec, ctx.area) ? '@' + spec : '@' + ctx.area + '/' + spec }}
::block when="design"
- **Design:** @{{design}}
::end-block

::block when="spec"
> **Note:**
>
> The review who's suggestions you are tasked with implementing was based
> on the detected _delta_ between the specification file above and the
> actual implementation source code.
::block when="design"
> 
> The design document above was created as a complimentary document to the
> specification file.
::end-block
::end-block

## Skill Selection

- use the '{{ctx.area}}' agent skill
- use the 'rust' skill when writing code
- use the 'rust-testing' skill when writing or debugging tests

## Task

::block when="!ready"
The review file above has completed in the {{ctx.area}} package area with a number of findings/suggestions for implementation.
::end-block

### Start with Logging

In order to track progress of this task we are using the '{{log}}' as a log file:

- you should immediately create a section for your part of this log file:
    - add an H2 header section called `## Implementation of Review Findings #{{iteration}}`
    - then timestamp it with: `> **started at:** {{ctx.now}}`
- write progress as Markdown unordered list items; use a table for the finding ledger
    - if you have structured sub-items for something then you can and should use a nested unordered list to represent this
- now start this unordered list by mentioning some important metadata for this implementation:
    - `- this implementation is attempting to implement _all_ of the review findings found in '{{ review }}'`
    - `- this is iteration {{iteration}} of the review-to-implement cycle `
    ::block when="retry"
    - `- we are _retrying_ iteration #{{iteration}} because in our last attempt the agent failed before achieving it's objectives`
        - `- the only context we have about where we got to in this previous attempt is:\n\t{{retry}}`
        - `- this does mean that some of the findings from the review are already likely implemented (but not all)`
        - verify prior repairs before changing them and record their evidence as `already fixed`; an earlier completion claim is insufficient
    ::end-block

> **Important:**
> 
> - always write idiomatic Markdown (CommonMark + GFM)
> - if there is a good reason to create an illustration, represent the diagram with a [Mermaid](https://mermaid.js.org/intro/) code block in your Markdown
> - use indentation of 8 spaces
> - be sure to use the Markdown H2/H3 headings recommended so that the document has the right structure

::block when="ready"
The review was marked as being **production ready** so there is no longer a need to continue the review-to-implement loop.

- add the following entries to the log file ({{log}}):
    - `- the review found in '{{review}}' indicated that the specification is **production ready**!`
    - `- the specification file used to define the functional/non-functional target of all this work can be found at '{{spec}}'`
- there are times in which a review can mark the work as "production ready" but still have some findings that should be considered as follow on work
    - when this is the case we need to document this in the log file:
        - `- while the review found this feature to be production ready, it did have findings worth looking at for follow on work:`
        - the as a nested list add the "name" of each findings
        - `- refer to the review file -- {{review}} -- for more details`

Now explain to the user the current state of this review/implement cycle and exit.
::end-block


::block when="!ready"

### Inventory before implementation

Read the entire review, including frontmatter findings, body findings, instance
tables, required repairs, recurrence, robustness recommendations, and blockers.
Set the review's `implemented` flag to `false` before starting repairs so a stale
claim cannot make an incomplete retry look complete.
Create a `### Finding Ledger` in this iteration's log section before editing code:

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|---|---|---|---|
| R{{iteration}}-F1 | Finding title | pending | |
| R{{iteration}}-F1.1 | Required site/shape and expected behavior | pending | |

- Retain review IDs. For legacy reviews without IDs, use the review filename plus
  finding title and assign numbered child rows locally. Include one parent row
  per finding and child rows for every required instance and consumer. Add rows
  as the sweep discovers more work; never replace the inventory with a summary.
- Reconcile frontmatter and body so nothing disappears. Include blocking issues
  buried in recommendations; mark explicitly nonblocking follow-ups separately.
- Record blockers and whether they have been removed by an accepted decision or
  changed prerequisite. Complete all unblocked work while genuine blockers remain.
- Finish each row as **fixed**, **already fixed**, **clean**, **blocked**,
  **deferred**, **unresolved**, or **disputed**. `clean` is for checked sibling sites,
  not a finding that requires repair. Record evidence for fixed/already-fixed/clean
  rows and a concrete reason for every open row.
- A parent finding is fixed only when every required child is verified and no
  discovered defect remains open. Use already fixed only if no repair was needed
  and all required checks passed. Any partial repair remains unresolved unless
  the remaining work is explicitly blocked or deferred. Count parent findings
  separately from instance rows.

From this point on in the task be sure to log progress on the task:

- indicate when you're _starting_ the work on a review finding: `- starting the work on '{task-name}' at {HH:MM:SS-local-time}`
    - add sub-bullets for things you've discovered, completed, or were blocked on while working on the task
- indicate when you've _completed_ the work on a review finding: `- work completed for '{task-name}' at {HH:MM:SS-local-time}`

> **Note:** 
> 
> - when employing subagents to do work always inform them about the log file and ask them to update as they do their work
> - the only exception to asking the subagents to own the logging is if you are running multiple subagents in parallel:
>    - in this case you should ask the subagents to report back to you as the orchestrator when they have a log item to report
>    - this allows you to report back to the caller in a coherent way by grouping the log messages by the various subagents
>    - Note: this does mean you'll need to accumulate the list of log items of any given subagent until it completes so you can log that subagent's log items as one group.

### Fix the class, not the instance

A review finding is almost always one instance of a defect class. When one
instance is fixed and its siblings are left, the next review finds the next
sibling and the cycle repeats. For example: a review reports that one parser
reads a missing field as an empty list; the fix corrects that parser and notes
that a sibling parser does the same "for the reviewer"; the next review's only
finding is that sibling. Each such deferral costs a full review/fix cycle.

For every finding, before any code changes:

- name the defect class in the log, in one sentence that does not mention the
  file the review named (for example: "a load-bearing field's absent, null,
  wrong-type, and invalid-element shapes are coerced to a permissive default")
- grep for every sibling site: every parser, reader, detector, or projection
  that handles the same kind of input or makes the same kind of decision,
  within the code this spec added or changed and the call sites that reach it
- fix every instance you find in this cycle and add a test case for each,
  whether or not the review named it
- trace changed identity, scope, category, arity, or other decision evidence
  through collection, filtering, summaries, candidate selection, lookup stopping
  conditions, and every downstream consumer; add a ledger row for each affected
  decision instead of stopping after fixing the reader or grouping step
- update the ledger with the checked sites, dispositions, and evidence

A **clean** site requires a reproduction with independently stated expected
results or a precise contract/invariant explanation tied to the code. Verify
both incorrect extra results and missing required results. Calling a fallback
"conservative" does not excuse losing a result the contract requires. Cache and
uncached equivalence checks must also assert the independently expected facts.

The following are not permitted for anything that is itself a defect against
the specification: "flagged for the reviewer", "left unchanged", "deliberately
left tolerant", "out of scope for this finding", "possible follow-up". Either
fix it in this cycle, or record it as blocked, deferred, unresolved, or disputed
in the ledger and closing report with the reason. These outcomes do not count
as fixed.

### Dispute a finding that has no authority

A high finding must cite its authority: a spec acceptance criterion, decision,
or sentence it violates, or a regression from the base branch reached by an
input a real call site, template, fixture, or user produces. When a finding
cites neither, or its cited authority does not say what the finding claims,
do not implement it. Record it in the ledger and closing report as
**disputed**, with one sentence naming the authority it lacks; the next review
must cite that authority or drop the finding.

Also dispute, rather than build, a repair that is large next to the harm it
prevents: a new escaping or protection layer, new syntax in generated output,
or code that exists only to satisfy synthetic payloads. Propose documenting
the limitation instead. Each such layer is new surface the next review can
find an edge in.

Observations and medium or low findings never require a repair; take one only
when it is a small, surgical change inside the spec's code.

::block when="review && file_exists(review) && frontmatter(review, 'recurrence') == true"
**This review reports a recurring finding class.** Its `## Recurrence` section
names the sibling sites an earlier fix missed. That list is the minimum scope
for this cycle, apart from any finding you dispute: fix every site it names,
then run your own sweep for any it missed.
::end-block

The repository's surgical-changes rule bounds each change to the defect class.
It does not bound it to the sentence in the review.

::file "../_input-robustness.md"

::file "../_headless-orchestration.md"


Now your task is to:

1. Act as an orchestrator and iterate over each suggestion (serially)
2. For each suggestion call a subagent to:
    - work from its parent finding and every child ledger row, including the
      recurrence list; return a disposition and evidence for each row,
    - name the defect class and sweep every sibling site (see "Fix the class, not the instance" above),
    - implement the suggestion at every site the sweep found,
    - add and/or update tests to provide full test coverage for the suggestion,
    - run focused regression checks for the repair,
    - tell the subagent to use the 'rust', 'rust-testing', and '{{ctx.area}}' agent skills

    > Note: 
    > 
    > - Run checks in the package areas directly affected by the implementation.
    >   The spec ({{spec}}) helps identify scope; the actual changes determine it.

3. Reconcile every ledger row yourself after all subagents finish. Check their
   evidence and inspect the combined changes; a subagent's success message is
   insufficient. Run `just test` and `just lint` in each affected package area
   after the final change, plus the verification tiers required by the behavior.
   Account for unrelated failures explicitly; never claim checks passed when
   they did not. Continue with remaining findings after a blocked repair.

4. Write the closing log entries to the log file ({{log}}):

    - use `### Completion Report`; state **complete**, **partial**, or **blocked**
      and the elapsed duration. Complete requires every finding repaired or
      independently verified already fixed, or disputed with its missing authority
      documented, and all required checks passing.
      Blocked means all remaining work depends on unavailable prerequisites;
      otherwise an incomplete cycle is partial.
    - count parent findings: `total = fixed + already fixed + blocked + deferred
      + unresolved + disputed`. No parent may remain pending, be counted twice, or be
      reported fixed while any required instance/consumer remains open.
    - summarize verified repairs and list every open finding/child ID, what
      remains, its reason, and the next required action. An evaluated finding
      is not necessarily implemented.

    - if host load prevents a legitimate required performance measurement,
      record the deferral, set `deferred_perf_measurement` to `true` on the log,
      and create or append to `{{ dirname(spec) + '/deferred-performance.md' }}`
      with the measurement details, finding ID, and review file ({{review}})

    - list changed files and final verification commands/results, tied to the
      findings they verify

5. Write completion metadata before exiting, including for partial or blocked work:

    - update the review file's metadata ({{review}}):
        - set the `log` frontmatter to '{{log}}'
        - set `implemented` to `true` only for a complete cycle as defined above;
          otherwise set it to `false` so incomplete work remains selectable
        - set the `implemented_by` frontmatter to `{{ ctx.agent }}/{{ ctx.model }}`
        - preserve the reviewer's readiness judgment, findings, and body; your
          ledger is the repair claim and the next review verifies it independently
    - update the log file's metadata ({{log}}):
        - set the `implementation_{{iteration}}` frontmatter to "{{ ctx.now }}"

Your final response must give the cycle outcome, reconciled finding counts,
verification results, and every remaining finding ID. Use "implementation
complete, ready for review" only for a complete cycle.

::end-block

::file "../_finish-before-exit.md"

::file "../_os.md"

::file "../_test-tiers.md"
