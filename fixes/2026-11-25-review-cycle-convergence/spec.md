---
kind: fix
name: review-cycle-convergence
created: 2026-11-25
status: draft-spec
area: repo
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-expression-and-mutation-schemas
reviewed: false
implemented: false
needs-rulings: true
---

# Make the review/fix cycle converge

## Problem

`just fix <spec>` alternates a review prompt and a repair prompt until a review
has no findings. For `2026-11-22-expression-and-mutation-schemas` that took 13
reviews in about 23 hours and has not finished. The feature the spec describes
was essentially complete by review 5. The remaining eight reviews repaired a
different part of Darkmatter that the spec never mentions.

The repairs were not poor. Every review confirmed the previous findings were
fixed, with one partial fix and two regressions in 13 rounds. The cycle fails to
end because of what the prompts tell the reviewer to report and the implementer
to repair.

## What happened

Each review labels every finding with its authority: `spec` (a sentence of the
spec is violated), `regression` (something that worked before no longer does),
or `defect` (code the reviewer read gives a wrong result).

| Reviews | What the findings were about | Findings citing the spec |
|---|---|---|
| 1–3 | Callable catalog parsing, where diagnostics point, host name collisions | 5 |
| 4–5 | A missing schema file, two documentation examples, and a lookup bug in `md validate refs` | 1 |
| 6–9 | Which schema folders a document may search: outside Git, in a second checkout, in a nested checkout, through library entry points | 0 |
| 10–13 | How `md validate refs` and `md graph` treat `::file` and similar inclusions: malformed targets, line numbers, target rules | 0 |

The turn came in review 5. The reviewer ran `md validate refs` on its own
previous review file and the command failed to find `$schema:
feature-review.yaml` by bare name. That bug existed before the spec and is
unrelated to it. It was filed as a high finding, repaired, and every later
finding is a neighbor of that repair or of the repair after it.

Other findings unrelated to the spec also used cycles: four about the Windows
run time of tests the spec did not change, and three about the review
pipeline's own schema file.

Since the last implementation phase closed, the branch gained roughly 18,000
lines against roughly 16,000 for the implementation itself. Both figures
include planning records and some unrelated work on the branch, but the largest
files are all in reference validation, for example a 1,983-line test file and
about 1,100 changed lines in `darkmatter/lib/src/markdown/reference/graph.rs`.

## Why the prompts produce this

1. **Any defect the reviewer reads blocks readiness.**
   [feature-review.md](../../prompts/_reviews/feature-review.md) accepts a
   defect "whether or not this spec added or changed it" and says it "blocks
   `ready`, and is repaired in the next cycle".
2. **The repair prompt forbids setting anything aside.**
   [implement-suggestions.md](../../prompts/_implement/implement-suggestions.md)
   says to fix every finding "whether or not it sits in code this spec added or
   changed" and "never create a backlog, follow-up, or `_unscheduled` spec".
3. **Repairs enlarge what the next review reads.** The repair prompt bounds a
   change by the defect class, "not the sentence in the review". A repair to
   unrelated code adds new unrelated code, which the next review reads and
   finds an edge in.
4. **Noticing the pattern changes nothing.** The review prompt says a class
   that keeps recurring "is a sign the requirement has no boundary" and tells
   the reviewer to mark the finding medium. Medium still blocks. Review 8 said
   in its own words that the requirement had no "clear stopping point", and
   the cycle continued for five more reviews.
5. **The cycle limit applies to one run.** `just fix` allows 5 cycles per
   invocation in [review.just](../../just/review.just). Starting it again
   resets the count and shows nothing about how earlier rounds went.

## Intended outcome

A review/fix cycle ends when the spec is implemented and what it changed
works. A defect found elsewhere along the way is written down where it will be
worked on, and does not hold the spec open. When the same kind of defect keeps
returning, the cycle stops and asks the author instead of sweeping wider.

## Requirements

### 1. Give every spec a recorded boundary

- When implementation of a spec starts, record the commit it started from on
  the spec as `implementation-base`.
- A finding is **in scope** when it has `spec` or `regression` authority, or
  when it is a `defect` in lines that differ from `implementation-base`, or
  reached only through them.
- Any other defect is **adjacent**. The reviewer decides this from the diff and
  states the reason in one sentence.
- A spec without `implementation-base` (work already in flight) is reviewed
  under the old rules. Existing specs, plans, logs, and reviews are not
  rewritten.

### 2. Adjacent defects do not block readiness

This requirement depends on Decision 1 below. As recommended:

- The review reports an adjacent defect as a defect report (where it is, the
  input that reaches it, the wrong result) under `## Adjacent Defects`, and in
  a frontmatter list `adjacent-findings`. It gets no class sweep.
- `ready` is `false` exactly when the review has an in-scope finding of any
  priority. Adjacent findings never make it `false`.
- The first review that reports one creates a dated fix in the owning package
  area, `fixes/{date}-{spec-name}-adjacent/spec.md`, with `status: draft-spec`
  and `related:` naming the reviewed spec. Later reviews add to that file.
- The repair prompt does not repair adjacent findings. Its rules against
  deferring work apply to in-scope findings only.
- Findings about the cycle's own prompts and schemas, and about the run time of
  tests the spec did not change, are adjacent by this rule. No special case is
  needed.

### 3. A recurring class changes the repair, then stops the cycle

The review prompt already compares each finding with earlier reviews and names
the class. Change what follows from that. The thresholds depend on Decision 2.

- **Second occurrence of a class.** The review states the one invariant the
  class violates and names the single function that should own the decision.
  The repair moves the decision to that owner and removes the per-site copies.
  The next review verifies the owner and searches for remaining copies. It does
  not invent new probe inputs for the class.
- **Third occurrence.** The review writes a `human-in-the-loop` item under the
  existing "changes scope" test, and the finding is blocked on it. The item
  gives the invariant, the history of the class, and three options: redesign
  within this spec, move the class to its own spec, or accept the current
  behavior. This replaces "mark the finding medium".

### 4. `just fix` shows the trend and counts every round

- Each review records its findings' authority and scope in frontmatter so the
  loop can read them without parsing prose.
- On start, and after each review, `just fix` prints one line per review:
  iteration, in-scope findings by authority, adjacent findings, and whether a
  class recurred.
- The cycle limit counts the spec's `review-iterations`, not the cycles of the
  current run (Decision 3). Past the limit the recipe stops and prints the
  trend, and continues only with an explicit override.
- When a round ends without a ready review, the existing `cycle-verdict`
  decision uses the trend: two consecutive reviews with no `spec` finding give
  `needs-human`, never `extend`.

### 5. Keep the record consistent

- Update `schemas/feature-review.yaml`, `schemas/review-fix-cycle.yaml`, and
  the sequence form of the cycle under `prompts/_fix/` to match.
- Update the sentence in `CLAUDE.md` under "Features and Fixes" that says an
  implementation "fixes what it finds rather than filing it", so it describes
  the adjacent-defect rule.
- Update the repo docs and skills that describe the review/fix cycle.

## Decisions for the author

### Decision 1: what happens to a defect outside the spec's boundary

Today the cycle repairs every defect the reviewer finds, wherever it is. That
rule was written on purpose, to stop agents from deferring real work. It is
also what kept this cycle open for eight extra reviews.

- **A. File it in a dated fix and do not block (recommended).** The review
  writes `darkmatter/fixes/2026-11-25-expression-and-mutation-schemas-adjacent/spec.md`
  listing the lookup bug, and marks the feature ready once its own findings are
  fixed. Cost: adjacent defects wait for a separate cycle, and a reviewer could
  misjudge an in-scope defect as adjacent. The one-sentence reason in the
  review makes that checkable. Recommended because it is the only option that
  stops unrelated repairs from adding code for the next review to read.
- **B. Repair the reported instance only.** The lookup bug is fixed where it
  was reported, with no class sweep and no recurrence tracking. Cost: the next
  review can still read the repair and report its neighbor; the chain is
  slower but not cut.
- **C. Keep today's rule.** Cost: the behavior described above.

### Decision 2: when a recurring class stops the cycle

- **A. Redesign on the second occurrence, stop on the third (recommended).**
  The implementer gets one attempt at a single-owner repair before the author
  is interrupted. In the cycle above this would have stopped at review 7.
- **B. Stop on the second occurrence.** Fewer wasted rounds, more
  interruptions; many classes are legitimately fixed by one wider sweep.
- **C. Never stop, only require the redesign.** No interruptions, and no
  protection when the redesign does not hold.

### Decision 3: what the cycle limit counts

- **A. All review iterations of the spec, with an explicit override
  (recommended).** `just fix spec.md` at iteration 6 prints the trend and exits;
  `just fix spec.md --extend` runs another round. Cost: one extra flag when a
  long cycle is justified.
- **B. Cycles of the current run, as today, plus the trend printout.** Nothing
  stops a rerun, so the limit stays advisory.

## Acceptance checks

- With `claudine` replaced by a stub that writes fixture review files, `just
  fix` exits ready when the latest review has only adjacent findings, stays in
  the cycle for an in-scope low finding, prints the trend, and refuses to pass
  the limit without the override.
- The review and repair prompts compose with `md compose --deny all` for a
  first review, a later review, and a review with `recurrence: true`. The
  composed repair prompt for a review with adjacent findings does not ask for
  them to be repaired.
- `schemas/feature-review.yaml` accepts a review with `adjacent-findings` and
  recorded authority, and still accepts the 13 existing reviews of
  `2026-11-22-expression-and-mutation-schemas` unchanged.
- The implementation log classifies the findings of those 13 reviews under the
  new rules, as a table. The expected result: the lookup bug from review 5 is
  adjacent, no in-scope finding exists after review 5, and the recurrence rule
  would have raised the diagnostic-location class (reviews 1, 2, and 3) for a
  single-owner repair at review 2.
- A spec without `implementation-base` runs under the old rules.

## Out of scope

- Finishing or closing `2026-11-22-expression-and-mutation-schemas`. Its
  in-flight repair of review 13 is handled on its own.
- Changing which agent or model reviews or repairs.
- Review prompts other than the feature review (`performance-review.md`,
  `cross-platform.md`, and the rest of `prompts/_reviews/`).
- Whether low-priority in-scope findings should block readiness.
