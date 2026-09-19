# Run record — reference-graph setup remediation (run of record)

Checkpoint: `f-refgraph-setup-fix`. Captured 2026-07-16. **Accepted** as the run
of record for this checkpoint at the review-3 closeout (2026-07-17); the sibling
[`run-20260717T033000-trivial-conservative`](../run-20260717T033000-trivial-conservative/rejection.md)
is rejected on provenance and retained.

Disposition, rationale, and the checkpoint's threshold verdict live in
[`results.md`](../../../../results.md) → *Reference-graph setup remediation*.
This file is the AD-A run record: pins, commands, environment, samples,
dispersion, thresholds, and raw-result locations.

## Verdict

**Threshold NOT ESTABLISHED — neither pass nor fail.** The gated quantity
(`compose_trivial`, `after vs base`) measures **+0.76 %** (+0.091 ms), under the
5 % gate — but a pass is not claimable from this run. The `base` arm is the same
audit-commit binary in this run and its sibling, same host, same fixture bytes,
~1 minute apart, and it measured **11.049 ms** vs **11.957 ms** — **+8.2 % drift
on identical code, larger than the 5 % threshold being adjudicated.** The host
could not resolve the gate.

The blocker is a **quiet host**, not an owner ruling. See `results.md` for the
predeclared admissibility criteria the required re-run must meet.

## Pins

| Arm | Commit | Meaning |
|---|---|---|
| `base` | `51c1f16e10ffe825b56987573ba4eabc659c768e` | audit commit — the gate's baseline (`spec.md` frontmatter `audit_commit`) |
| `before` | `e15b1cc22b113a9b24058207d760cd879fa62eb6` | integrated head carrying the regression (parent of the fix) |
| `after` | `92a3d502eb65c30205a9a255dd13dd8dc6d0aabf` | `before` + *perf(darkmatter): cache baseline schema canonical JSON for compose options* |

`after` is sampled twice per round (`after_A` / `after_B`) — same binary, so their
delta is the identical-code drift floor.

**Provenance caveat.** `92a3d502e` is timestamped `2026-07-16 21:10:34 -0700`, but
this run executed at `20:37:45 -0700` — 33 minutes earlier. The `after` binary was
built from the working tree while the fix was uncommitted, and committed after. The
table records the commit that tree became; the `after` pin is therefore
**reconstructed, not observed**. The required re-run must build all three pins from
committed SHAs.

## Build

Detached worktrees with an isolated `CARGO_TARGET_DIR` per pin, so no pin reads
another's incremental artifacts:

```bash
for pin in base:51c1f16e1 before:e15b1cc22 after:92a3d502e; do
  name="${pin%%:*}"; sha="${pin##*:}"
  git worktree add --detach "/tmp/dmbench/src-$name" "$sha"
  CARGO_TARGET_DIR="/tmp/dmbench/target-$name" \
    cargo build --release -p darkmatter-cli \
    --manifest-path "/tmp/dmbench/src-$name/Cargo.toml"
done
```

The harness expects `/tmp/dmbench/target-{base,before,after}/release/md` and
hard-fails if any is missing (`refgraph-setup-fix.sh:31-33`). Those worktrees are
scratch and not retained; rebuild from the SHAs above.

## Capture

```bash
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
./refgraph-setup-fix.sh raw/f-refgraph-setup-fix/run-20260717T033745 8
```

## Recomputation

Every statistic below is reproduced from the retained raw vectors by:

```bash
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun refgraph-setup-fix-report.ts raw/f-refgraph-setup-fix/run-20260717T033745
```

## Fixtures

Committed bytes under `benchmarks/fixtures/`; identities frozen in
`manifest.yaml` and re-verified by
`benchmark_fixtures.rs :: benchmark_manifest_matches_recorded_identities` — that
test is the authority; identities are not recomputed ad hoc.

`compose_trivial`: 241 bytes, `darkmatter_hash`
`10f054ee903d73ec-489140f252295fb7`, `xxhash64` `26e8dcccefde48cd`.

All three pins read identical fixture bytes; only the binary differs.

## Method and environment

- `hyperfine`, `--shell=none`, `--style basic`, `NO_COLOR=1`
- warm-up 3; 12 runs per arm per round; 8 rounds → **96 observations per arm per case**
- every round samples all four arms per case, so a load excursion shifts arms
  together rather than landing inside one
- statistic = mean of pooled observations; dispersion = sample stddev
- non-TTY (piped), so terminal detection short-circuits
- **Host:** Apple M4 Max (`Mac16,5`), macOS 26.5.2, Darwin 25.5.0 arm64
- **Toolchain:** `stable`; profile `--release`
- **Capture window:** `20:37:45`–`20:38:39` local (~54 s)
- **Load during capture:** 1-min **5.42–7.16** (`load.log`, 5-s samples,
  continuous across the capture) — *not* a quiet host

## Raw results

Retained beside this file: `<case>.r<round>.json` (hyperfine JSON, 4 arms × 12
runs each) for all six cases, plus `load.log`.

## Results (n=96/arm)

Times in ms. `drift floor` = `(after_B − after_A)/after_A`, the same binary
against itself.

| case | `base` | `before` | vs base | `after_A` | `after_B` | **drift (A/A)** | **after vs base** |
|---|---:|---:|---:|---:|---:|---:|---:|
| `compose_trivial` | 11.957 | 15.132 | **+26.6 %** | 12.069 | 12.026 | **−0.36 %** | **+0.76 %** (+0.091 ms) |
| `compose_schema_transclusion` | 17.568 | 21.580 | **+22.8 %** | 16.959 | 17.049 | +0.53 % | −3.21 % |
| `compose_interpolation_heavy` | 15.907 | 18.601 | **+16.9 %** | 15.775 | 15.883 | +0.68 % | −0.49 % |
| `compose_transclusion_heavy` | 55.874 | 63.055 | **+12.9 %** | 47.199 | 47.149 | −0.11 % | −15.57 % † |
| `render_basic` *(control)* | 5.793 | 5.896 | +1.79 % | 5.756 | 5.939 | **+3.18 %** | +0.95 % |
| `help` *(control)* | 5.134 | 5.124 | −0.19 % | 5.114 | 5.168 | **+1.04 %** | +0.14 % |

† `compose_transclusion_heavy`'s `base` arm carries a large round-8 excursion
(78.54 ms vs ~52 ms round-typical). Excluding round 8, `after vs base` is
**−10.4 %**, not −15.57 %. Reported as computed with the caveat attached rather
than dropping the round silently; the retained vectors let anyone check it.

## What this run does establish

The remediation works. `before vs base` is **+12.9 % to +26.6 %** across the four
compose cases while both controls stay flat (`help` −0.19 %, `render_basic`
+1.79 %); after the fix every compose case falls to **−15.6 % … +0.76 %**. That
contrast is 10–20× any drift floor present and is compose-specific by
construction. **`92a3d502e` removes ~25 percentage points of the Command-Setup
regression.** Only the residual's exact value is unresolved.

## Why this run is the run of record

Accepted on **provenance, not on its more favourable number**:

1. **It is a live product of the committed harness.** Per-file mtimes are
   staggered `20:37:46 → 20:38:39`, matching its 11 continuous `load.log` samples
   — the signature of a real sequential run.
2. **It has the control arms.** All six cases, including `render_basic` and
   `help`. `help` runs no compose code at all, which is what makes a
   compose-specific claim falsifiable.
3. **Its load is monitored end-to-end** (5-s samples across the whole capture).

Its own resolving power is nevertheless insufficient for the 5 % gate — see
*Verdict* above. Accepting it as the run of record is not the same as passing.
