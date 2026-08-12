# Run record — REJECTED (retained)

Checkpoint: `f-refgraph-setup-fix`. Captured 2026-07-16. **Rejected** at the
review-3 closeout (2026-07-17). The run of record for this checkpoint is
[`run-20260717T033745`](../run-20260717T033745/summary.md).

This run is **retained, not deleted**, per the standing rule that raw vectors and
harnesses are never deleted. It is recorded here so its number cannot be quoted
without its defects.

## Its claim

`compose_trivial`, `after vs base` = **+4.91 %** (+0.543 ms), against a 1.13 %
identical-code drift floor. The run of record measured **+0.76 %** for the same
quantity — a 5× disagreement, which is what made this checkpoint's disposition a
review-3 blocker.

Both figures were independently recomputed from the 96 retained observations per
arm at closeout and reproduce exactly. This run's number is **real**; it is its
*interpretability* that fails.

## Why it is rejected

Rejected for **structural defects that make its number uninterpretable** — not
for being unfavourable:

1. **No control arms.** It measured `compose_trivial` only. The committed harness
   emits six cases; this directory holds eight files, all `compose_trivial`. It is
   therefore **not reproducible** by `refgraph-setup-fix.sh` as committed, and it
   is *structurally incapable* of distinguishing a compose regression from a
   host-wide shift — the exact question at issue.
2. **Its files did not come from a live run in this directory.** All eight JSONs
   and `load.log` share one mtime, `20:39:04` — materialised in a single second,
   *after* the accepted run finished at `20:38:39`. It is a retained copy of an
   earlier ad-hoc invocation, not a harness product.
3. **5 seconds of load monitoring.** `load.log` holds a **single** sample
   (`20:36:44`, load 5.61) — and that timestamp precedes its own file mtime by
   2m20s, consistent with (2). What the host did during the capture is
   essentially unrecorded.
4. **Its A/A drift floor is 3× wider** (1.13 % vs 0.36 %) — the run resolved the
   code less well.

Additionally, its margin to the 5 % gate is **0.09 pp** — an order of magnitude
below its own 1.13 % drift floor. Even taken at face value, it does not resolve
the gate it appears to nearly fail.

## Recomputation

```bash
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun refgraph-setup-fix-report.ts raw/f-refgraph-setup-fix/run-20260717T033000-trivial-conservative
```

## Raw results

Retained beside this file: `compose_trivial.r1..r8.json` and `load.log`.

## Note on the pair

The two runs' `base` arms are the **same audit-commit binary**, same host, same
fixture bytes, ~1 minute apart. They measured **11.049 ms** (this run) and
**11.957 ms** (the run of record) — **+8.2 % drift on identical code, larger than
the 5 % threshold being adjudicated.** Neither run can settle the gate; the
checkpoint needs one quiet-host bracketed re-run. See
[`results.md`](../../../../results.md) → *Reference-graph setup remediation*.
