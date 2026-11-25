---
$schema:
    spec: file(required; match(**/*spec*.md)) -> the specification file defining the target functionality
    design: file(match(**/*design*.md)) -> optionally, a design file which compliments the spec file with more details on the technical design
    review: file -> the review file who's findings/suggestions we will implementing; the latest review beside the spec
    iteration: number(required;default(1)) -> what _iteration_ of the review/implement cycle that we are on
    retry: string -> if you're retrying this prompt after a failure then pass in what you know about prior progress
description: |-
    Implements the findings in a review which was conducted to determine the drift between
    what was actually implemented versus the specification that was being targetted in that
    implementation.
review: "{{ file_exists(dirname(spec) + '/review-1.md') ? find_last_index(dirname(spec) + '/review-1.md') : null }}"
iteration: "{{ review ? file_index(review) : 1 }}"
# is the review "production ready"?
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"
design: "{{ file_exists(dirname(review) + '/design.md') ? dirname(review) + '/design.md' : null }}"
yolo: true
feature-or-fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"

initialize:
    - when: "!review"
      then:
          - error: "`{{parent_dir(spec)}}` has no review to repair: no `review-1.md` exists beside the spec"
    - when: "review && file_exists(review) && !is_empty(frontmatter(review, 'human-in-the-loop')) && frontmatter(review, 'human-resolved') != true"
      then:
          - warn: "{{basename(review)}} is waiting on a human: run `just fix {{spec}}` in a terminal to record the rulings before repairing it"
          - error: "{{basename(review)}} is waiting on a human"

start:
    message: "🏃 implementing _findings_ from review #{{ file_index(review) }} in `{{ parent_dir(review) }}` (_using_ {{ctx.agent}}/{{ctx.model}} _in_ {{ctx.area || ctx.repo}})"
success:
    - when: "ready == true && frontmatter(review, 'implemented') != true"
      then:
          - message: "review #{{ file_index(review) }} of `{{ parent_dir(review) }}` is production ready; no repair was needed"
    - when: "ready != true && frontmatter(review, 'implemented') == true"
      then:
          - message: "✅  implemented all review findings from review **#{{ file_index(review) }}** of `{{ parent_dir(review) }}`; ready for independent verification (area: **{{ctx.area || ctx.repo }}**, wt: **{{ ctx.worktree }}**, branch: **{{ctx.branch}}**)"
          - say: "the review findings in {{ without_date(parent_dir(review)) }} were implemented and are ready for review"
          - effect: bong
    - when: "ready != true && frontmatter(review, 'implemented') != true"
      then:
          - warn: "review #{{ file_index(review) }} repair of `{{ parent_dir(review) }}` ended with incomplete findings; see the finding ledger in {{review}}"
failure:
    message: "💥 implementation of the review #{{iteration}} suggestions from **{{ parent_dir(review) }}** failed to complete ({{err.msg}})!"
    effect: sad-trombone
---
::file "../_no_formatting.md"

::file "../_frontmatter_edits.md"

# Implement Review Suggestions for {{title_case(without_date(parent_dir(spec)))}}

> - **{{capitalize(feature-or-fix)}}:** `{{parent_dir(review)}}`
> - **Iteration:** {{iteration}}

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

::block when="!ready"
### Record the repair in the review

The review file ({{review}}) is the only record of this repair. Do not write to
`implementation-log.md` or `log.md`, and do not read them: the plan's
implementation is recorded there, and everything this repair needs is in the
review and the spec.

- add an H2 section `## Repair` at the end of the review, after any
  `## Human rulings`, and timestamp it with `> **started at:** {{ctx.now}}`
- everything you record goes in that section; leave the reviewer's own text
  above it unchanged
::block when="retry"
- a `## Repair` section already exists from an attempt that failed before it
  finished; keep it and continue in it. What is known about that attempt:

    {{retry}}

- verify its repairs before changing them and record their evidence as
  `already fixed`; an earlier completion claim is insufficient
::end-block

> **Important:**
> 
> - always write idiomatic Markdown (CommonMark + GFM)
> - if there is a good reason to create an illustration, represent the diagram with a [Mermaid](https://mermaid.js.org/intro/) code block in your Markdown
> - use indentation of 8 spaces
> - be sure to use the Markdown H2/H3 headings recommended so that the document has the right structure
::end-block

::block when="ready"
The review marked the {{feature-or-fix}} **production ready**: a ready review has no findings, so there is nothing to repair. Explain the current state of this review/implement cycle and exit.
::end-block


::block when="!ready"

### Inventory before implementation

Read the entire review, including frontmatter findings, body findings, instance
tables, required repairs, recurrence, robustness recommendations, and blockers.
Set the review's `implemented` flag to `false` before starting repairs so a stale
claim cannot make an incomplete retry look complete.
Create a `### Finding Ledger` in the `## Repair` section before editing code:

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
  **unresolved**, or **disputed**. `clean` is for checked sibling sites,
  not a finding that requires repair. Record evidence for fixed/already-fixed/clean
  rows and a concrete reason for every open row.
- A parent finding is fixed only when every required child is verified and no
  discovered defect remains open. Use already fixed only if no repair was needed
  and all required checks passed. Any partial repair remains unresolved unless
  the remaining work is explicitly blocked. Count parent findings
  separately from instance rows.

The ledger is the whole record of the work. Update a row when its outcome is
known, and put what you discovered or decided in that row's evidence column.
Do not keep a timestamped progress narrative beside it, and do not restate a
row's outcome anywhere else in the section.

> **Note:**
>
> - a subagent returns the disposition and evidence of each row it worked on to
>   you, the orchestrator; only you write to the review file

### Fix the class, not the instance

A review finding is almost always one instance of a defect class. When one
instance is fixed and its siblings are left, the next review finds the next
sibling and the cycle repeats. For example: a review reports that one parser
reads a missing field as an empty list; the fix corrects that parser and notes
that a sibling parser does the same "for the reviewer"; the next review's only
finding is that sibling. Each such deferral costs a full review/fix cycle.

For every finding, before any code changes:

- name the defect class in the finding's parent ledger row, in one sentence that does not mention the
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
fix it in this cycle, or record it as blocked, unresolved, or disputed
in the ledger with the reason. These outcomes do not count
as fixed.

### Dispute a finding that has no authority

A high finding must cite its authority: a spec acceptance criterion, decision,
or sentence it violates, or a regression from the base branch reached by an
input a real call site, template, fixture, or user produces. When a finding
cites neither, or its cited authority does not say what the finding claims,
do not implement it. Record it in the ledger as
**disputed**, with one sentence naming the authority it lacks; the next review
must cite that authority or drop the finding.

When the review's suggested repair is large next to the harm it prevents (a
new escaping or protection layer, new syntax in generated output, or code that
exists only to satisfy synthetic payloads), build the smallest repair that
removes the harm instead, and record the choice in the ledger. Each such layer
is new surface the next review can find an edge in, but the harm itself is
never left in place.

Fix every finding in this cycle, high, medium, and low, whatever the size of
the repair, and whether or not it sits in code this spec added or changed:
the cycle is complete only when every finding is fixed. Before repairing a
finding the review did not reproduce, reproduce it with a test first. Never
create a backlog, follow-up, or `_unscheduled` spec for open work.

This cycle runs non-interactively, so never stop to ask. When a repair needs a
decision the spec does not settle, such as two reasonable fixes with different
user-visible results, make it: choose the option that best fits the spec, the
area's `docs/`, and existing behavior, and implement it with tests. Record the
decision, the alternatives, and why in the ledger row. Only when the decision
passes the readiness rules' **worth the author's time** test (it is hard to
reverse, departs from a decision, changes scope, touches safety or trust, or
you genuinely doubt it) also add a one-line summary to the review's
`human-review-items` and set its `human-review` to `true`, keeping that list at
3 items or fewer; a choice you would simply recommend keeping stays in the
ledger only. When the spec pre-authorizes
bounded implementer autonomy and the choice meets its conditions, also amend
the spec as that decision directs.

A finding is **blocked** only when it waits on an item in the review's
`human-in-the-loop` that has no ruling yet, or on a prerequisite this cycle
cannot produce (an unavailable host or account). A review that calls a
finding blocked, or a decision "the author's", without such an item does not
block it: the review asked for no ruling, so you make the decision as above
and repair the finding. Recording the same finding as blocked in two
consecutive cycles stops the loop and costs the author a cycle; do not do it
for anything you could decide.

::block when="review && file_exists(review) && !is_empty(frontmatter(review, 'human-in-the-loop'))"
**The author has ruled on this review's `human-in-the-loop` items** under its
`## Human rulings` heading. A ruling has the same authority as the spec and
wins where they disagree. Before any code change, add each ruling that
requires a change to the ledger as a **high** row titled `carry out ruling on
item N: {short description}`, and carry it out like any other finding: code,
tests, `docs/`, skills, and, where the ruling changes a decision or an
acceptance criterion, the spec (mark the decision amended by the ruling, with
its date). Rulings cannot be disputed. A ruling that cannot be carried out as
written, or conflicts with another, is **unresolved** with the reason; do not
guess. A ruling that confirms existing behavior adds no row. Findings the
rulings unblock are repaired in this same cycle.
::end-block

::block when="review && file_exists(review) && frontmatter(review, 'recurrence') == true"
**This review reports a recurring finding class.** Its `## Recurrence` section
names the sibling sites an earlier fix missed. That list is the minimum scope
for this cycle, apart from any finding you dispute: fix every site it names,
then run your own sweep for any it missed.

Before editing, identify where the decisions diverge and whether they can
reuse one authoritative rule. Show the smallest failing example and a control
whose result must differ. After editing, run the earlier cases from the same
defect class alongside the new probes. Record which rule the decisions now
share, or explain why their answers legitimately differ.
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
   they did not. A slow test counts as a failure (see "A Slow Test Is a
   Failing Test"). Continue with remaining findings after a blocked repair.

4. Close the `## Repair` section with `### Outcome`. Keep it short; the ledger
   already holds the detail, and Git holds the changed files:

    - state **complete**, **partial**, or **blocked**, and the elapsed
      duration. Complete requires every finding repaired or independently
      verified already fixed, or disputed with its missing authority
      documented, and all required checks passing. Blocked means all remaining
      work depends on unavailable prerequisites; otherwise an incomplete cycle
      is partial.
    - count parent findings: `total = fixed + already fixed + blocked
      + unresolved + disputed`. No parent may remain pending, be counted twice, or be
      reported fixed while any required instance/consumer remains open.
    - list every open finding/child ID with its next required action; an
      evaluated finding is not necessarily implemented. Do not repeat rows that
      are closed.
    - list the final verification commands and their results

    - if host load prevents a legitimate required performance measurement,
      record the deferral, set `deferred-perf-measurement` to `true` on the
      review, and create or append to `{{ dirname(spec) + '/deferred-performance.md' }}`
      with the measurement details, finding ID, and review file ({{review}})

5. Write completion metadata to the review's frontmatter ({{review}}) before
   exiting, including for partial or blocked work:

    - on each entry of `findings`, set `result` to the parent finding's
      disposition: `fixed`, `already-fixed`, `blocked`, `unresolved`, or
      `disputed`. Change nothing else in an entry. An entry written as plain
      text (a review from before the structured form) is left as it is; its
      result is in the ledger only
    - set `implemented` to `true` only for a complete cycle as defined above;
      otherwise set it to `false` so incomplete work remains selectable
    - set `implemented-by` to `{{ ctx.agent }}/{{ ctx.model }}`
    - set `repaired-on` to "{{ ctx.now }}"
    - preserve the reviewer's readiness judgment, the rest of its frontmatter,
      and its body above `## Repair`; your ledger is the repair claim and the
      next review verifies it independently

Your final response must give the cycle outcome, reconciled finding counts,
verification results, and every remaining finding ID. Use "implementation
complete, ready for review" only for a complete cycle.

::end-block

::file "../_finish-before-exit.md"

::file "../_os.md"

::file "../_test-tiers.md"

::file "../_slow-tests-fail.md"
