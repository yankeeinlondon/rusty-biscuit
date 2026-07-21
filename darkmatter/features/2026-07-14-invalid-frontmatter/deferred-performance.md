---
feature: 2026-07-14-invalid-frontmatter
kind: deferred-performance
---

# Deferred Performance Measurements — Invalid Frontmatter

## Phase 7 benchmark comparison

- **Maps back to:** the review finding *"High — Safety, corpus, performance, and platform acceptance
  evidence is incomplete"* in
  [`review-1.md`](./review-1.md), which states: *"There is likewise no recorded Phase 7 benchmark
  comparison for the no-frontmatter and clean-frontmatter hot paths."*
- **Deferred during:** implementation cycle 1 (`2026-07-18T18:33:32-07:00` → `2026-07-18T20:42:31-07:00`),
  logged in [`log.md`](./log.md).

### What was required

The spec's [Performance](./spec.md) section sets a **relative no-regression** posture, explicitly
deferring any hard per-document millisecond budget. Acceptance is therefore *no measurable
regression* on the two common cases:

1. documents with **no frontmatter**, and
2. documents whose frontmatter is **already clean**.

### Why it was deferred

The host CPU load did not permit a legitimate measurement. Load averages recorded across the task:

| Point in task | 1-min | 5-min | 15-min |
|---|---|---|---|
| Start | 87.07 | 128.63 | 117.97 |
| Peak (mid-task) | 168.92 | — | — |
| Finish | 13.59 | 52.84 | 72.71 |

Multiple agents ran concurrently throughout. A cross-run comparison under this variance would have
measured scheduler contention, not the change. **No number was reported rather than an untrusted
one.**

### What is needed to close it

- A **main-vs-branch drift bracket** on a quiet host: measure `main`, measure the branch, then
  re-measure `main`. If the two `main` measurements do not agree within noise, the host was not quiet
  enough and the comparison is void.
- Both hot paths measured separately — the no-frontmatter path and the already-clean-frontmatter path
  — since the spec's zero-cost guarantee applies specifically to the former.
- Both affected package areas: `biscuit-file` and `darkmatter`.

### Substitute evidence that did land

`darkmatter/lib/tests/clean_counters.rs` (8 tests) expresses the spec's performance requirements as
**counter invariants** rather than timings:

- no frontmatter → zero schema resolution and zero trigger-discovery work;
- trigger discovery and the built validator cached per `clean` invocation;
- safety-gate reparse only on candidate edits — an already-clean document parses once;
- analysis cost does not scale with candidate count.

These hold identically on a loaded host, so they are the durable guarantee. They do **not**, however,
discharge the relative no-regression acceptance, which still requires the timed comparison above.

## Review implementation follow-up — 2026-07-20

- **Maps back to:** the same review finding, *"High — Safety, corpus, performance, and platform
  acceptance evidence is incomplete"*, in [`review-1.md`](./review-1.md).
- **Evaluated during:** review implementation cycle 1, started at
  `2026-07-20T23:38:36-07:00`.

### Benchmark vehicle correction

The audit found that `clean_hot_paths/full_pipeline` still timed only
`Markdown::try_from_content → cleanup → as_string`. The shipped feature now performs raw
frontmatter extraction, YAML analysis, schema analysis, and raw-preserving assembly around that
legacy sequence, so the unchanged harness could not observe the feature's added work.

`darkmatter/lib/benches/clean_hot_paths.rs` now includes those actual feature stages in the two
`full_pipeline` cases. The no-frontmatter fixture takes the real extraction-and-bypass branch. The
already-clean-frontmatter fixture performs one source-first YAML analysis, resolves and applies the
default schema context, cleans the parsed Markdown, and assembles the original frontmatter block
without reserialization. The existing `phase1-before` Criterion baseline therefore remains the
pre-feature comparator while the candidate now measures the shipped path.

### Why the timed comparison remains deferred

No timing was reported in this follow-up. A legitimate result still requires the quiet-host
main/branch/main drift bracket described above. This session used a shared dirty worktree with
concurrent agent activity, so it could neither produce an isolated `main` build nor establish a
quiet, repeatable scheduler baseline. Running Criterion once here would create a precise-looking but
untrustworthy number.

The corrected harness passed `cargo check -p darkmatter --bench clean_hot_paths --color never`, and
the load-independent counter suites passed. These verify that the comparison vehicle compiles and
that the structural cost invariants hold; they do not replace the deferred timing measurement.
