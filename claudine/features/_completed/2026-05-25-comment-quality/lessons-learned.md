# Lessons Learned — Comment Quality

**Feature:** `2026-05-25-comment-quality`
**Review cycles:** 5 (4 written reviews + final approval)
**Authored:** 2026-05-26

A retrospective on what the review iterations actually caught, what kept
recurring, and the process changes that would have collapsed the cycle
count.

## The four findings that drove iteration

| # | Iteration | Severity | Category |
|---|-----------|----------|----------|
| 1 | review-1 | High | Out-of-scope behavior change (prompt rendering) |
| 2 | review-1 | High | Heuristic missed common Rust function shapes |
| 3 | review-1 | High | No fixture tests for the heuristic |
| 4 | review-1 | Med  | `docs/comment-quality.md` missing before/after pairs for positive criteria |
| 5 | review-2 | High | Checker baseline still not clean in target scope |
| 6 | review-2 | High | Reviewer misattributed remote-signal changes (rejected) |
| 7 | review-3 | High | Prompt-rendering changes re-appeared in change set |
| 8 | review-4 | High | Canonical HOW-narration left intact in `stream/reporting.rs` |
| 9 | review-4 | Med  | 13 pre-existing broken intra-doc link warnings |
| 10 | review-4 | Low  | Anti-pattern 7 used a sketch, not a real file path |

The same root causes drove most of these. They are worth naming.

## Lesson 1 — A "comments only" feature is harder to keep clean than it sounds

The single most expensive recurring finding (reviews 1 and 3) was
out-of-scope rendering changes inside `prompt_reporting/`. The cleanup
touched the same files that visually present prompts, and a refactor
commit (`44a038fc3`) bundled rendering tweaks with docstring cleanup.
The reviewer caught it in review-1, the implementer addressed
*specific* glyphs, the commit was partially restored in a later
iteration, and review-3 caught it again.

**What worked:** finally reverting the rendering portion of the
offending commit while *keeping* the docstring simplification, and
proving the comments-only acceptance contract by inspection of the
final diff.

**Lesson:** when a spec promises "no behavior change," the
implementer's diff must be filterable to comments + whitespace + allow
attributes. If you cannot produce that filtered diff trivially,
something behavioral got in. A `git diff -G '^\s*///\?'` or equivalent
content-only filter should be part of the implementer's self-review
before requesting review.

## Lesson 2 — Eat your own dogfood, then eat it again

Review-4's marquee finding was that `stream/reporting.rs` still
contained the canonical HOW-narration anti-pattern — and that file is
the exact "Before" example cited in `docs/comment-quality.md` for
anti-pattern 1. The cleanup removed *other* anti-patterns from the
same file (section-marker `//` comments) but missed the one the rubric
explicitly names.

**Lesson:** for any rubric-driven cleanup, every file path cited in
the rubric documentation must be a verified "After" example by the
end of the feature. Citing a file in the rubric is a public commitment
that the file is clean. A pre-flight check: `grep` the rubric for
file paths, then re-read each one against the rubric before declaring
the cleanup pass complete.

## Lesson 3 — A heuristic script needs fixture tests *and* must pass against its own target scope

Review-1 found that `check-comments.sh` parsed only single-shape
function signatures (missing multi-line signatures and single-line
bodies) and had no automated tests. Review-2 then found that even
after the parser was strengthened, the script still emitted three
findings inside the spec's required cleanup scope
(`claudine/lib/src` + `claudine/cli/src`).

A tool that flags problems is only useful if (a) it covers the shapes
that exist in the codebase and (b) its output against the target
scope is the trusted signal that the cleanup is done.

**Lesson:** for any new lint/heuristic tool, the implementer must
satisfy two acceptance gates in this order:

1. Level 1 fixture tests covering the language constructs the tool
   claims to support — not "spot checks against the current tree,"
   which only prove the tree is currently clean.
2. The tool must exit clean against the scope it was created to
   police, or each remaining finding must be documented as an
   accepted exception in the spec.

## Lesson 4 — Shared development branches confuse reviewers

Review-2 included a high-severity finding about
`remote-signal/daemon/src/sync.rs` and `session_log.rs` changes. Those
changes belonged to a parallel feature
(`2026-05-24-remote-signal`) that happened to share the `claudine`
working branch. The implementer correctly rejected the finding, but
that rejection cost a review cycle and required citing commit subjects
to show the unrelated lineage.

**Lesson:** reviewers see the full branch diff, not just the diff
*intended* for the feature under review. Two practical mitigations:

- **Reviewers** should constrain `git log`/`git diff` to the feature
  directory or to commits whose conventional-commit scope matches the
  feature, when multiple specs share a branch.
- **Implementers** should call this out in the review prompt:
  "this branch also contains commits for feature X; their scope is
  `refactor(remote-signal…)`." A one-line scope statement at the top
  of the review prompt removes the ambiguity before it costs a cycle.

## Lesson 5 — Unconditional acceptance criteria need to mean what they say

The spec said `cargo doc -p claudine` "produce no warnings about
broken intra-doc links." Review-4 found 13 such warnings — all
pre-existing. The criterion as written demanded they be fixed; the
intent (per the implementer) was "no *new* warnings from this
feature."

**Lesson:** acceptance criteria that compare against the entire
codebase (not just the diff) must be either:

- Phrased as a delta — "no new warnings introduced by this feature,"
- Quantified against a baseline — "warning count not increased from
  $N$,"
- Or scoped to the files the feature touches.

The Phase 4 plan task already used the delta phrasing ("warning count
unchanged from baseline"); the spec's acceptance criteria did not.
Spec and plan should agree on the comparison frame.

## Lesson 6 — Rubric docs must cite real files

Review-4's low-severity finding was that anti-pattern 7 used a
"(sketch)" rather than a real file path. The acceptance criterion was
explicit: "each citing a real file path in the codebase." Sketches
are useful for explanation but they are not calibration — a reviewer
applying the rubric to real code cannot triangulate against a
hypothetical fixture.

**Lesson:** every anti-pattern in a rubric doc must point at a real
"Before" file (preserved historically via the citation, since the
file itself will be cleaned up) and either a real "After" file or a
realistic transformation of the Before. If no real example exists in
the codebase, the rubric should probably not list that anti-pattern.

## Lesson 7 — Reviewer environment limitations affect verification confidence

Every one of reviews 1, 2, and 3 noted that the reviewer attempted
`cargo test` and was blocked by Cargo lock contention or rebuild
times, then stopped the run to avoid hanging the non-interactive
session. The spec's "test pass" acceptance criterion was therefore
not directly verified by the reviewer in three consecutive
iterations.

**Lesson:** for non-interactive review sessions, the implementer
should attach test-run evidence (counts, durations, exit codes) at
the time of requesting review so the reviewer is not blocked by
infrastructure issues outside the feature. Review-4 did this
implicitly by including test counts (`2302 + 978 passing`) in the
verification table — and that review converged.

## Process changes that would have collapsed the cycle count

If these had been part of the workflow from review-1, this feature
likely would have finished in two cycles, not five:

1. **Pre-review checklist for the implementer.** Before requesting
   review:
   - Produce a comments-only diff filter and confirm no source line
     changes leaked in.
   - Run the new heuristic against its own target scope and report
     the count.
   - List test-run evidence (counts, durations) so the reviewer is
     not blocked by Cargo locks.
   - Re-read every file cited in the rubric docs against the rubric.
2. **Branch-scope statement at review time.** When a branch contains
   multiple features, the review request prompt says so and points
   the reviewer at the feature-specific commit range.
3. **Delta-phrased acceptance criteria** for any tool/lint output
   that compares against the entire codebase rather than the diff.
4. **Sketch ban in rubric docs.** Every cited "Before" must be a
   real file path. If no real example exists, drop the anti-pattern.

## What review-driven iteration is good at — and what it isn't

Review cycles caught real defects: out-of-scope behavior changes
twice, heuristic gaps in function-shape parsing, missing fixture
tests, an un-cleaned canonical example, and a pre-existing acceptance
criterion mismatch. None of these were "polish"; each materially
affected whether the feature met its contract.

But four of the five high-severity findings were preventable by the
implementer with a stricter pre-review checklist. Iteration is the
correct safety net for *missed judgement calls*, not for missed
mechanical self-checks. The pre-review checklist above is the
intended cost-reduction for the next feature of this shape.
