---
kind: feature
created: 2026-10-27
status: draft-spec
area: claudine
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    next_spec: string -> the `{date}-{name}` of the spec expected to follow this one; absent when none is planned
human_review: false
reviewed: false
related:
    - 2026-10-25-pipeline-work-reduction
---

# Parse each shell placeholder once while preparing a sequence shell site

## Why this exists

Sequence preflight prepares every `shell:` command before any task starts
(`claudine/lib/src/composition/sequence/preflight/mod.rs`,
`SequencePreflight::prepare_site`). It analyzes each `{{ … }}` placeholder of
the command in five separate passes, and each pass parses the placeholder's
expression from text again:

| Pass | Where the parse happens |
| ---- | ----------------------- |
| template grammar | `parse_authored` (Darkmatter `template/grammar.rs`) projects each hole's untyped type through `project_hole` |
| approval scan | `refuse_unapprovable_reads` → `approval_diagnostics` (`lifecycle/bindings.rs`) → Darkmatter `prepare_value` → `scan_leaf` |
| target identity | `refuse_unapprovable_reads` → `first_target_identity_root` (`sequence/preflight/shape.rs`) |
| type projection | `CommandTemplate::project_types` → `project_hole` (Darkmatter `template/typing.rs`) |
| late-hole marking | `mark_late_holes` → `DeferredShellRoots::late_roots_of` (Darkmatter `template/candidate.rs`) |

The expression text is the same in all five passes and none of them changes
it, so four of the five parses repeat work. This is item D8 of
`2026-10-25-pipeline-work-reduction`. That feature's rule says an item that
meets one of the measurement clauses gets its own dated spec, and D8 met
clause 1 when it was re-measured on 2026-10-27.

## Measured evidence (2026-10-27, macOS development host)

Revision `d7ea96a1f`, with `2026-10-26-sequence-quadratic-frontmatter` and
`2026-10-25-shared-acyclic-definitions` merged.

- **Anchor A6** (sequence preflight of 100 shell steps, each
  `echo step-N {{ arg_0 }} … {{ arg_9 }}`; `claudine/lib/benches/`):
  - `ExpressionParse` 10,000 (100 per step at 10 and at 100 steps, so the
    sequence fix made it linear);
  - 26,709,585 bytes allocated;
  - Criterion median **8.2117 ms** (interval 8.1460–8.2866 ms).
- **Where the 10 parses per placeholder come from.** One run of A6 at 10 and
  100 steps, with a temporary backtrace tally at the parse counter (removed
  afterward), shows exactly one parse per placeholder per step at each of ten
  sites:
  - the five shell-site passes above;
  - two frontmatter late-root scans (`value_roots`, counted as
    `DeferredRootScanParse`);
  - Claudine's removed-read check (`reject_removed_sequence_reads`);
  - two interpolations that evaluate the command text.
- **Unit cost.** One parse costs **132.18 ns** (Criterion
  `pipeline_work/D8/single_parse`, interval 130.89–133.39 ns) and 215 bytes
  (`pipeline_work_alloc`).
- **Share of A6.** Removing four of the five shell-site parses saves
  4 × 1,000 = 4,000 parses: 4,000 × 132.18 ns = 0.529 ms, which is **6.4% of
  A6's time** (clause 1). In bytes it is 4,000 × 215 B = 0.86 MB, or 3.2%.
  - This is count × isolated unit cost, the report's method, not a
    before-and-after differential.
  - The timing pass ran at a 1-minute load average of about 7–8 on 16 cores.
    That is under the bench preflight's limit (half the cores) but above the
    "about 2" the spike report asked for. Both the per-parse and A6 medians
    came from that one pass.
- If only the two repeats that Darkmatter's isolated
  `CandidateScope::build` sequence shows are counted (3 parses per hole
  there), the share is 3.2% of time. The higher figure applies because all
  five passes run inside one `prepare_site` call.
- Clauses 2 and 3 are not met. The repeat grows linearly with placeholders
  (about 1 MB and 1 ms at ten times the largest real sequence), and the
  peak does not grow.

## Behavior to build

Within one shell-site preparation, each distinct placeholder expression is
parsed once, and every analysis pass of that site reads the retained syntax.

- The retained syntax lives only as long as the site's preparation. Nothing
  is kept across sites, steps, preflights, retries, or launches.
- Evaluation stays fresh. Interpolation, presence checks, and every runtime
  fill still evaluate against current values, and file functions run on
  every evaluation.
- Errors keep their timing and content. A placeholder that fails to parse
  is reported by the same pass, with the same error, at the same point in
  preflight as today. A grammar error in the authored command
  (`TemplateGrammarError`) still withdraws the same approvals.
- The frontmatter late-root scans, the removed-read check, and evaluation
  are outside this change. They read the document's frontmatter values, not
  one site's placeholders.
- Darkmatter's public template entry points (`parse_authored`,
  `CommandTemplate::project_types`, `DeferredShellRoots::late_roots_of`)
  keep their behavior. A variant that accepts already-parsed syntax may be
  added beside them.

## Contracts that must not change

- Approval membership and template shapes, including authored-program-hole
  withdrawal and exact/template approval of the first task.
- Late, deferred, and guarded hole timing (`HoleTiming`) and the late roots
  each hole reports.
- `MissingValue` and the unapprovable-read refusals, with their messages and
  sources.

## Acceptance

1. An L1 work-count regression with `claudine`'s `work-counters` feature:
   preflight of the A6 shape (S shell steps of H placeholders) records
   exactly `S × H × 6` `ExpressionParse` (today `S × H × 10`). The shell
   site's share falls from 5 parses per placeholder to 1, and the other five
   sites are unchanged.
2. Behavior tests through sequence preflight cover: a malformed placeholder
   (same error, same pass); an authored program-word hole (approval
   withdrawn as today); late, deferred, and guarded holes (unchanged timing
   and roots); a missing value with and without a presence guard; and a file
   function in a placeholder still running at evaluation.
3. The A6 rows of `pipeline_work_alloc` and the Criterion
   `A6_shell_preparation` case are re-run, and the implementation log
   records them beside the 2026-10-27 figures above.
4. `just test` and `just lint` pass in `claudine/` and `darkmatter/`.
5. The pre-flight checks topic page states that a site's placeholders are
   parsed once per preparation.
