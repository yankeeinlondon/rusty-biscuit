---
kind: fix
created: 2026-10-26
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
human_review: false
related:
    - 2026-10-25-pipeline-work-reduction
    - 2026-10-24-sequence-containers
---

# Sequence work grows with the square of the sequence

A sequence document's frontmatter contains its own `sequence:` list. Several
per-step and per-leg operations copy or reprocess the **whole** frontmatter.
So each step's cost grows with the number of steps, and the sequence's total
cost grows with its square. Small sequences hide it; large ones hit hundreds
of megabytes to gigabytes.

## Measured

These come from the `2026-10-25-pipeline-work-reduction` spike. Counts and
bytes are exact and reproducible. Times were taken on a loaded host and are
omitted.

| Path | 10 | 100 | 1,000 | Growth |
| ---- | -: | --: | ----: | ------ |
| Group with N inline legs (`max_parallel: 4`): one leg's live-frontmatter copy | 7.6 KB | 48 KB | **455 KB** | per copy ∝ N, so total ∝ N² |
| … peak live bytes of the group step | 3.7 MB | 27.6 MB | **669 MB** | about N² (455 MB of it is the leg copies) |
| Preflight of N prompt tasks: bytes allocated | — | 63 MB | **2.33 GB** | 37 times for 10 times the tasks |
| Preflight of S shell steps with 10 holes each: `ExpressionParse` count | 1,900 | 109,000 | — | exactly `10·S² + 90·S` |

Reproduce:
- `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A6`
  for the shell rows;
- `cargo test -p claudine --lib --release --features work-counters -- --ignored --nocapture --test-threads=1 pipeline_spike`
  for the group and prompt rows.

## Known and suspected sites

| Site | Status |
| ---- | ------ |
| `composition/sequence/task/group.rs` (about line 468): every declared leg gets `Mutex::new(seed.clone())` of the live frontmatter, and that frontmatter includes the `sequence:` list holding the group itself | **confirmed** by counter and allocation |
| Prompt-task preflight: provider prediction (`preflight/providers.rs`) accounts for 231 MB of the 2.33 GB; the rest is not yet attributed | **partially attributed** |
| Shell-step preflight: the `S²` term means each step's analysis visits every step's holes, most likely by walking the whole document frontmatter (with its `sequence:` list) for shell roots, guards, or types | **hypothesis** |

## Step 1: attribute before changing code

The plan's first task attributes the two unexplained paths before designing
the fix. Use the method that located the DMLS hover cost:
1. Run the spike scenario at the large size in a release build.
2. Take a CPU sample (`sample <pid> 15` on macOS) and demangle it with
   `rustfilt`.
3. Read the call tree under the preflight entry point.
4. Confirm each candidate site with a work counter (the `work-counters`
   feature already exists in `claudine/lib`).

Record every site found in the implementation log, with its count at 10 and
100 steps. **A site is in scope when its cost per step grows with the
sequence's size.**

## Required behavior

- **Per-step and per-leg work is independent of how many other steps the
  sequence has.** Preparing, preflighting, or running step *k* never copies or
  re-analyzes the other steps' definitions. The authoring document's
  `sequence:` list is read once per preparation (or once per reread, when the
  file changes) and shared.
- **A leg's private frontmatter view is not a full copy of a document that
  contains the leg.** It must still:
  - start from the group-start state;
  - keep its own writes private;
  - never let a sibling's write become visible;
  - follow the existing rule that a member's `set_frontmatter` reaches disk
    and the next step rereads the file.

  The plan chooses the mechanism: share the immutable seed and keep each
  leg's writes as a delta, or exclude the container's own definition from the
  copied view. This change also provides most of Track 5's C1 saving. C1
  (creating leg state only when a leg is admitted) is still done there, and
  bounds the remaining copies to `max_parallel`.
- **Shell-step analysis parses each placeholder of step *k* a constant number
  of times**, independent of S. This is the quadratic term only; reducing the
  constant factor of 3 parses per hole is Track 5's D8, decided after this
  fix.
- **Unchanged:**
  - every preflight result and approval inventory;
  - the shell-approval decision and its freshness rules;
  - every runtime write and its visibility;
  - retries, rereads, and the `seq` and `group` namespaces;
  - error text and timing.

## Out of scope

- Track 5's other runtime items: C2–C4, C6, and C7.
- Prompt launch cost beyond preflight.

## Implementation surfaces

- `claudine/lib/src/composition/sequence/task/group.rs`
- `claudine/lib/src/composition/sequence/preflight/` (`mod.rs`,
  `providers.rs`, and whatever step 1 identifies)
- The scope stack and live-frontmatter cells used by containers
- The spike scenarios in `claudine/lib/benches/pipeline_work_alloc.rs` and
  `claudine/lib/src/**/pipeline_spike*`, for re-measurement

## Documentation

- `claudine/docs/topics/` pages on sequence containers and preflight: state
  that a leg's view is private and how it is formed, if they describe the
  copy today. No user-visible behavior changes.
- The `claudine` skill, if it describes per-leg frontmatter copies.

## Acceptance and verification

1. **Work-count regressions (L1, `work-counters` feature):**
   - shell preflight's `ExpressionParse` count is linear in S. At 10 and 100
     steps with 10 holes, the count per step is the same;
   - a group of 10 and of 100 inline legs creates per-leg frontmatter views
     whose size does not grow with the leg count. Assert it with the
     `LiveFrontmatterCopy` counter plus a bytes probe in an allocation bench
     row, not an exact-bytes assertion;
   - prompt-task preflight bytes per task are the same at 100 and 1,000 tasks,
     within 10% (reported by the bench; the L1 test asserts the counted work).
2. **Re-measured:** the spike's A4 and A6 rows and C1's table are re-run, and
   the before/after values are recorded in this fix's implementation log and
   in `spike-report.md`.
3. **Behavior unchanged:** existing sequence, group, preflight, approval, and
   retry suites pass unchanged, including the sibling-isolation and
   atomic-group tests.
4. `just test`, `just test-l2`, and `just lint` pass in `claudine/`. No test
   exceeds five seconds.
