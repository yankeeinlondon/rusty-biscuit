---
kind: implementation-log
created: 2026-10-26
scope: phase 1 spike report for 2026-10-25-pipeline-work-reduction
---

# Spike report: pipeline work reduction

This report decides each Track 5 item from the spike's measurements. The raw
data, methods, and per-item tables are in
[spike-data-darkmatter.md](spike-data-darkmatter.md) and
[spike-data-claudine.md](spike-data-claudine.md). This page states only what
those numbers decide, what the spike found beyond its items, and what
happens next.

## Read this first

- **Counts and bytes are reliable; times are not yet.** Work counters and
  allocation totals do not depend on host load. They were re-run
  independently and matched exactly:
  - A3: 1,207,807,451 bytes per warm edit loop;
  - A6: 1,900 and 109,000 `ExpressionParse` at 10 and 100 steps.

  Every timing was taken with a load average of 35–90 on 16 cores (both
  measurement agents and another worktree were compiling). The same A3 phase
  measured 54.7 ms and 87.6 ms in two runs. All times and time-% shares are
  **provisional**. No `pipeline-work-pre` Criterion baseline was saved.
- **Thresholds applied** are the spec's: share of at least 5% of the anchor's
  time or bytes (clause 1); faster-than-useful-work scaling reaching 10 ms or
  10 MB at a plausible size (clause 2); peak bytes growing with an uncapped
  count (clause 3). Because times are provisional, no verdict below rests on
  a time share alone.
- **Measured at** commit `84c55d16c` plus the uncommitted spike tooling, on
  the macOS development host (16 cores), `bench` profile.

## Three findings outside the item list

These matter more than any single item. Each now has its own dated fix, and
two of them change how Track 5 should proceed.

### 1. DMLS frontmatter typing clones schema shapes per entry (defect)

On the A3 document (23 KB), one hover allocates **145 MB**, and **967 MB** on
a document 10 times larger (peak live 56 MB and 418 MB). One diagnostics pass
allocates 525 MB and 970 MB.

The per-provider breakdown (`A3_PROVIDER_BREAKDOWN=1`, added to the A3
bench) puts the whole hover cost in `FrontmatterProvider::hover`. The other
three providers decline and allocate under 1 MB. A CPU sample of that hover
attributes about 95% of samples to
`expression_values → def_at_path_ctx → ShapeMemo::ensure`. That code
deep-clones `PropertyAtom` trees, and about 30% of samples are the drops of
those clones.

The cause:
- **Hover types every scalar in the document to find the one under the
  cursor, and does it twice:** once via `expression_value_at` and once via
  `stack_expression_hover`. Diagnostics run the same walk.
- **`ShapeMemo` is keyed by the instance path,** so each lifecycle list item
  (`[key, Index(n)]`) builds its own owned shape. `descend` clones the
  selected arm or builds a merged copy.
- **`def_at_path_ctx` clones each leaf's `PropertyDef`.**

The result is cost ≈ entries × schema size, and Claudine's lifecycle schema is
large. → `2026-10-26-dmls-frontmatter-typing-allocation`.

### 2. Sequence work grows with the square of the sequence (defect)

A sequence document's frontmatter contains its own `sequence:` list.
Several per-step and per-leg operations copy or reprocess that whole
frontmatter:

- **Group legs:** each leg's live-frontmatter copy is 455 KB at 1,000 legs
  (7.6 KB at 10). 1,000 of them make 455 MB of C1's 669 MB peak.
- **Prompt tasks:** preflight of 1,000 prompt tasks allocates 2.33 GB,
  against 63 MB at 100 (37 times more for 10 times the tasks).
- **Shell steps:** preflight of S shell steps with 10 holes each parses
  exactly `10·S² + 90·S` expressions (1,900 at 10 steps, 109,000 at 100).

This is the shared cause behind most of C1's peak and most of D8's parse
count. → `2026-10-26-sequence-quadratic-frontmatter`.

### 3. DMLS index store not open: not a defect

All the DMLS servers that were checked, including freshly restarted ones,
had no SQLite store open. That is by design: the store opens on the first
find-references, heading-rename, file-rename, or workspace-symbols request.
Their resident memory (25–186 MB) is therefore not explained by the store in
either direction. It is still unexplained, and the first fix above is the
likely main contributor. Re-check resident memory after that fix lands
before opening anything further.

## Item verdicts

| ID | Finding | Measured (largest size; bytes exact) | Clause | Verdict |
| -- | ------- | ------------------------------------ | ------ | ------- |
| **C1** | group allocates leg state up front | peak 669 MB at 1,000 legs, cap 4; A4 19.1% of bytes | 1, 3 | **implement**, after finding 2: leg state at admission bounds the copies to `max_parallel`, and finding 2 shrinks each copy |
| **C2** | `hook.<path>` materializes the whole payload | 1.48 MB per leaf read at 1 MB; A5 95.3% of bytes | 1, 2 | **implement** |
| **C3** | full `RuntimeState` snapshots | 10.3 MB per snapshot at 1,000 outputs; A4 44.4% of bytes (36.8% avoidable) | 1 | **implement**. Most snapshots come from task- and leg-level `snapshot().mutations` reads, not only container events; the plan covers both |
| **C4** | `loop.<member>` clones the previous result | 1.47 MB per scalar read at 1 MB; A5 4.7% | 2 | **implement**. Each loop pass materializes `loop` twice, which outweighs C5 |
| C5 | loop conditions reparsed | 1 parse per pass, about 1.7 KB | none | close |
| C6 | pre-warm rebuilds state per phrase | 151.5 MB per 200 phrases at 200 KB; A4 about 2.2% (estimate) | 2 (marginal) | implement, low priority; re-measure after finding 2 |
| **C7** | prediction clones state for literal hints | 231.5 KB per task at 100 KB; A4 1.35% | 2 | **implement**, including the caller's extra frontmatter copy |
| C8 | environment-map copies | 4 copies, 422 KB per entry at 500 variables; 0.35% avoidable | none | close |
| **D1** | eager-file rewrite recompiles arm validators | A1 variant 47.1% of bytes; batch-proportional | 1, 2 | **implement**. The per-rewrite memo leaves 2 compiles per document; also taking the arm validator from `ValidatorCache` needs the author's reading of "nothing reused across values or requests" (the validator is shared, every value is still judged afresh) |
| **D2** | `md schema validate` prepares twice | second preparation 44.1% of the CLI sequence's bytes | 1 | **implement** the candidate |
| **D3** | scrub and launch check rediscover triggers | 2 discoveries per pre-validation, 0.96 MB each; A4 5.0% (estimate) | 1 (at threshold) | implement, low priority |
| **D4** | positions built before a schema applies | A1 17.0% of bytes; schema-free baseline 14.6% | 1 | **implement the lazy position provider**, not the candidate: with the runtime baseline every document has a schema, so the candidate saves nothing on the default path |
| D5 | scope join rebuilds ancestor sets | A3x10 0.02% | none | close |
| D6 | diagnostic positions rescan the prefix | A3 0%: DMLS never calls `one_based_position` | none | close |
| D7 | operand-site discovery | A2 0.1% (sites), at most 3.9% (marks) | none | close |
| D8 | placeholders reparsed | 3 parses per hole instead of 1; 215 B per parse | deferred | **decide after finding 2.** In A6 the 109,000 parses are about 23 MB of 92 MB (25%), but almost all of that count is finding 2's quadratic term, not D8's factor of 3. Re-measure A6 once finding 2 is fixed |
| D9 | lifecycle grammar clones subtrees | A3x10 at most 0.05% | none | close |
| D10 | `LifecycleSourceMap` built per pass | A3x10 0.04% | none | close |
| D11 | failure records built on success | A2 at most 3.6% | none | close |
| D12 | named-type memo clears at 256 | A3 0 | rule 1 | **implement** (decided by rule) |
| D13 | alphanumeric checks compile directly | A2 0 | rule 1 | **implement** (decided by rule) |
| P1 | Content Policy grammar in editor regexes | not a performance item | — | **implement** the parity matrix |
| X1 | cached alternative matchers | A3 at most 0.85% | none | stays deferred |
| X2 | one walk per file alternative (claudine-cli completion) | about 270–400 ms more per extra alternative over 12,000 files (macOS, one run) | 2 | **write a candidate design.** Confirm the timing on a quiet host first |
| X3 | recursive conversion memo | no anchor reaches it | none | stays deferred |
| X4 | problem-locating index | A1 negligible | none | stays deferred |

**Totals:** 9 to implement (C1–C4, C7, D1, D2, D4, plus D12, D13 and P1 by
rule), 2 low-priority (C6, D3), 1 pending (D8), 1 design (X2), 9 closed (C5,
C8, D5, D6, D7, D9, D10, D11, plus finding 3), 3 still deferred (X1, X3, X4).

## Author decisions needed

1. **Thresholds.** Keep the spec's 5% / 10 ms / 10 MB, or adjust them. C4 (4.7%)
   and D3 (5.0%, estimated) sit on the line.
2. **D1 with `ValidatorCache`:** does sharing a compiled validator across
   documents satisfy "nothing reused across values or requests"?
3. **Order.** The two defect fixes go first. C1 and D8 wait for the
   sequence fix, and every A3 share is re-measured after the DMLS fix,
   because hover and diagnostics dominate A3's totals today.

## Next steps

1. Fix `2026-10-26-dmls-frontmatter-typing-allocation` and
   `2026-10-26-sequence-quadratic-frontmatter`.
2. Re-run the alloc benches (`pipeline_work_alloc` in both crates,
   `pipeline_work_a3`) and update this report's A3, A4, and A6 rows. Counts
   need no quiet host.
3. On a quiet host (load average below about 2, no other builds, including
   other worktrees), run the timing benches once, check Criterion's spread
   (more than about 5% median absolute deviation means repeat), and save
   `pipeline-work-pre`.
4. Update the Track 5 spec with these verdicts and write the phase 2 plan.

## Tooling added by the spike

- `test_toolkit::alloc_count`: a counting global allocator for bench binaries.
- `darkmatter::work_counters` and `claudine::work_counters`, behind each
  package's test-only `work-counters` feature. Call sites use the crate-root
  `note_work!` macro, which compiles to nothing without the feature. There
  are 23 production call sites (15 in Darkmatter, 8 in Claudine).
- Benches:
  - `darkmatter/lib/benches/pipeline_work{,_alloc}.rs`;
  - `claudine/lib/benches/pipeline_work{,_alloc}.rs`;
  - `darkmatter/dmls/benches/pipeline_work_a3.rs`, with an
    `A3_PROVIDER_BREAKDOWN=1` switch that prints each hover provider's cost.
- Claudine crate-private scenarios: `cargo test -p claudine --lib --release
  --features work-counters -- --ignored --nocapture --test-threads=1
  pipeline_spike`.
