---
kind: implementation-log
created: 2026-10-26
feature: 2026-10-25-pipeline-work-reduction
scope: phase 1 spike, Claudine half (C1-C8, D3, X2; anchors A4, A5, A6)
---

# Spike data: Claudine

This is the Claudine half of the phase 1 measurement spike. The Darkmatter and
DMLS half is in `spike-data-darkmatter.md`; the two are merged into
`spike-report.md`.

**Timings are provisional (loaded host).** Every wall time and every
time-percentage share below was taken while three agents were compiling and
testing on the same host (load average 21-80 on 16 cores; each run's load is
recorded in [Load averages](#load-averages)). Work counts, allocation counts,
bytes allocated, and peak live bytes do not depend on load and are
authoritative. Times from the allocation harness also include the counting
allocator's atomics, so they run higher than the Criterion times; where both
exist, both are given.

## Environment

| | |
| - | - |
| Host | Apple M4 Max, 16 cores, 128 GiB, macOS 27.2 (26B5091g) |
| Toolchain | rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1 |
| Profile | `bench` for the bench targets; `release` (`cargo test --release`) for the crate-private scenarios; the two share the `release` settings (no workspace override) |
| Commit | `c582c8bc04f6a8dd418c2b38fb3a7822f843bd75` plus the uncommitted counter call sites and spike files in [Files](#files) |
| Thresholds applied | the spec's: clause 1 share >= 5% of wall time or allocated bytes; clause 2 faster-than-useful-work growth reaching 10 ms or 10 MB at a plausible size (<= 10x the largest real instance); clause 3 peak live bytes growing with an uncapped count |
| Criterion baseline | not saved (`pipeline-work-pre` is to be saved on the quiet-host pass) |

Plausible-size references in this repository: the largest frontmatter is
174 KB (`claudine/docs/research/agent-cli/kilo.md`); real sequences
(`claudine/docs/research/_provider-tasks.yaml`) have about 10-15 steps and no
group; `claudine/schemas/` holds 11 files; the checkout tracks 17,604 files
(6,201 Markdown).

## How each share was measured

Unless a row says otherwise, an item's share is **count x isolated per-unit
cost / anchor total**: the item's work counter inside the anchor, times the
bytes (and time) of one unit measured alone at the anchor's shape. Each
anchor below states which parts it contains, and where the item's work does
not occur inside the in-process anchor (C6, C8, D3), the count is an
estimate and is labeled so.

## Anchor totals

| Anchor | What ran | Bytes allocated | Peak live | Time, provisional (loaded host) | Work counters |
| ------ | -------- | --------------- | --------- | ------------------------------- | ------------- |
| **A4** preflight part | `build_preflight_graph` of 50 steps: 49 prompt tasks, each its own document with 100 KB frontmatter and literal `agent: codex`, plus one 200-leg group (`max_parallel: 4`) | 115,863,131 | 22,533,094 | 170.9 ms (alloc harness); 48.9 ms (Criterion) | `PredictionStateBuild:98`, `dm.LifecycleConfigParse:2`; no `TriggerDiscovery` |
| **A4** group step | the 200-leg group executed through the task executor with fake shells, after 49 committed 10 KB outputs, 100 KB of runtime writes, 100 KB sequence frontmatter (seed allocation of 608,494 B subtracted) | 727,563,433 | 189,970,145 | 80.7 ms (alloc harness) | `RuntimeSnapshot:615`, `RuntimeStateBranch:200`, `LiveFrontmatterCopy:200` |
| **A4** total | both parts | **843,426,564** | 189,970,145 | **251.6 ms** (alloc harness) | |
| **A5** event dispatch | 100 events; each renders a template of 10 `hook.*` leaves (`interpolate`) and parses and evaluates a condition of 5 `loop.*` scalars; 1 MB `tool_response`, 100 KB previous loop result | **1,550,136,916** | 1,476,239 | **534.0 ms** (alloc harness); 154.3 ms (Criterion) | `HookPayloadMaterialize:1000`, `LoopAmbientMaterialize:500`, `dm.ExpressionParse:1100` |
| **A6** shell preparation | `build_preflight_graph` of 100 shell steps, each `echo step-N {{ arg_0 }} ... {{ arg_9 }}`, every `arg_i` a `$schema`-typed string | **92,119,166** | 2,130,385 | 55.8 ms (alloc harness, second run); 266.3 ms (first run, heavier load); 39.2 ms (Criterion) | **`dm.ExpressionParse:109000`**, `dm.EffectivePreparation:1`, `dm.LifecycleConfigParse:1` |
| A6 at 10 steps (scaling check) | the same at 10 steps | 3,966,943 | 309,506 | 2.1 ms (alloc harness) | `dm.ExpressionParse:1900` |

A4 is in-process and covers preflight plus the group's execution. It runs no
prompt launch (no provider), so per-launch work (D3's pre-validation, C8's
document entries, C6's pre-warm submission) is outside it and is estimated.

**For D8 (Darkmatter agent):** A6 parses expressions **109,000** times for
1,000 holes (100 steps x 10). At 10 steps it is 1,900. Both fit
`10 * S^2 + 90 * S` exactly, so preparation reparses every step's holes once
per step: the count is **quadratic in the step count**, not linear in holes.
Its share of A6 is D8's per-parse cost x 109,000 / A6's totals above.

## Items

### C1: group allocates private state for every leg up front

Scenario: a one-step sequence whose step is a group of N shell legs at
`max_parallel: 4`, 100 KB of runtime writes as the base state, run through
the task executor (crate-private, `task/tests/pipeline_spike.rs`). The sequence
document's frontmatter is the live seed; the group is authored inline in it.

| Legs | Frontmatter payload | `RuntimeStateBranch` / `LiveFrontmatterCopy` | Full snapshots | Allocations | Bytes allocated | **Peak live** | Time, provisional |
| ---- | ------------------- | -------------------------------------------- | -------------- | ----------- | --------------- | ------------- | ----------------- |
| 10 | 0 | 10 / 10 | 45 | 9,086 | 10,523,219 | 3,732,621 | 0.67 ms |
| 10 | 100 KB | 10 / 10 | 45 | 9,122 | 14,312,019 | 5,677,787 | 0.73 ms |
| 100 | 0 | 100 / 100 | 315 | 162,929 | 78,030,943 | 27,580,080 | 18.4 ms |
| 100 | 100 KB | 100 / 100 | 315 | 163,145 | 100,251,743 | 38,686,242 | 16.5 ms |
| 1,000 | 0 | 1,000 / 1,000 | 3,015 | 12,393,068 | 1,559,149,669 | **669,456,882** | 1,507 ms |
| 1,000 | 100 KB | 1,000 / 1,000 | 3,015 | 12,395,084 | 1,765,690,469 | **772,671,294** | 1,510 ms |

Per-unit, measured alone: one runtime branch (100 KB base) 102,982 B; one
live-frontmatter copy 7,589 / 48,089 / 454,889 B at 10 / 100 / 1,000 legs
with no payload (109,989 / 150,489 / 557,289 B with 100 KB).

- **Scaling: quadratic for an inline group.** Peak grows with legs
  *declared*, not the cap of 4, and each leg's live-frontmatter copy itself
  grows with the leg count, because the copied frontmatter holds the
  sequence's own `sequence:` list, which contains the group. At 1,000 legs,
  1,000 copies of 455 KB account for 455 MB of the 669 MB peak.
- **A4 share:** 200 x (608,270 B branch + 195,689 B live copy) = 160.8 MB of
  843.4 MB = **19.1% of bytes**; 85% of the group step's 190 MB peak (leg
  copies created at admission would keep about four, roughly 3.2 MB).
  Time 200 x (10.5 + 15.3) us = 5.2 ms = 2.1%, provisional (loaded host).
- **Clauses:** 1 (bytes) and 3 (peak grows with an uncapped count).
- **Recommendation: implement the candidate** (create the leg's runtime and
  live cell when the leg is admitted). It also bounds the quadratic part,
  since at most `max_parallel` live copies exist at once.

### C2: dotted `hook.<path>` materializes the whole payload

Scenario: `EventMetaExpressionLookup::get` (public).

| `tool_response` | `hook.tool_name`: bytes / allocs / time | `hook.tool_input.command`: bytes | Unprefixed `tool_name`: bytes / time |
| --------------- | ---------------------------------------- | -------------------------------- | ------------------------------------ |
| 10 KB | 19,625 / 101 / 7.6 us | 19,631 | 220 / 0.04 us |
| 100 KB | 148,955 / 551 / 37.8 us | 148,961 | 220 / 0.04 us |
| 1 MB | 1,476,743 / 5,171 / 339.5 us (Criterion 142.9 us) | 1,476,749 | 220 / 0.04 us |

Every dotted read records `HookPayloadMaterialize:1`. Peak live per read is
about the bytes allocated (1,475,223 B at 1 MB).

- **Scaling:** per read, linear in the payload size while the useful work
  (one leaf) is constant; per event, proportional to the number of leaf reads.
- **A5 share:** 1,000 materializations x 1,476,743 B = 1,476.7 MB of
  1,550.1 MB = **95.3% of bytes**; 1,000 x 339.5 us = 339.5 ms of 534.0 ms =
  **63.6% of time** (Criterion cross-check: 142.9 ms of 154.3 ms = 92.6%),
  provisional (loaded host).
- **Clauses:** 1 and 2.
- **Recommendation: implement the candidate** (closed per-member resolver).

### C3: full `RuntimeState` snapshot for single-field reads

Scenario: one `snapshot()` read for its mutations, and a nested sequence
container of three shell children, with N committed 10 KB outputs and 1 KB
of writes (crate-private).

| Committed outputs | One snapshot: bytes / time | Nested sequence of 3: full snapshots / bytes / peak / time |
| ----------------- | -------------------------- | ---------------------------------------------------------- |
| 10 x 10 KB | 104,726 / 2.3 us | 35 / 1,658,968 / 434,898 / 0.46 ms |
| 100 x 10 KB | 1,032,806 / 22.2 us | 35 / 9,096,568 / 4,147,218 / 0.61 ms |
| 1,000 x 10 KB | 10,313,606 / 250.5 us | 35 / 83,472,568 / 41,270,418 / 4.0 ms |

- **Scaling:** the snapshot count per container is constant (35); each
  snapshot copies the whole committed history, so bytes grow linearly with
  history while the useful work (three children) is constant.
- **A4 share:** the group step takes 615 full snapshots (three per leg plus
  15), each about 608,270 B at A4's shape (49 outputs plus 100 KB writes):
  374.1 MB = **44.4% of A4 bytes**. A mutations-only copy is 102,982 B, so a
  narrow projection would avoid up to 615 x 505,288 B = 310.8 MB = 36.8%.
  This is an upper bound: some sites read `outputs`, not `mutations`. Time
  615 x 10.6 us = 6.5 ms = 2.6%, provisional (loaded host).
- Most of A4's snapshots are task- and leg-level sites that read one field
  (`task/mod.rs`: `values.extend(runtime.snapshot().mutations)` and siblings;
  `group.rs` leg folds), not container events. They are the same mechanism
  the finding names.
- **Clauses:** 1 (bytes). Clause 2's 10 MB is reached only at 1,000 outputs
  (83 MB); at the plausible 100 outputs it is 9.1 MB.
- **Recommendation: implement the candidate** (narrow owned projections under
  one lock), covering the task- and leg-level single-field sites as well as
  the container-event ones.

### C4: `loop.<member>` clones the previous result

Scenario: `LoopExpressionLookup::get` (public).

| `last_output` | `loop.count`: bytes / allocs / time | `loop.is_last`: bytes |
| ------------- | ----------------------------------- | --------------------- |
| 10 KB | 15,795 / 62 / 3.3 us (Criterion 1.40 us) | 15,795 |
| 100 KB | 145,125 / 512 / 26.7 us | 145,125 |
| 1 MB | 1,472,913 / 5,132 / 284.7 us (Criterion 152.7 us) | 1,472,913 |

Every scalar read records `LoopAmbientMaterialize:1`.

- **Scaling:** per scalar read, linear in the previous result's size; the
  useful work is constant.
- **A5 share** (A5 uses a 100 KB previous result): 500 x 145,125 B =
  72.6 MB = **4.68% of bytes**; 500 x 26.7 us = 13.4 ms = 2.5% of time,
  provisional (loaded host). With a 1 MB previous result the same anchor
  would allocate 736 MB in these reads.
- **Clauses:** 2. A loop of 100 passes reading 5 scalars over a 1 MB group
  result (within 10x of the 174 KB largest real document) allocates 736 MB
  and spends about 142 ms in these reads. Clause 1 is not met (4.68% < 5%).
- **Recommendation: implement the candidate** (direct member projection). It
  also removes the two materializations each C5 condition pass makes.

### C5: loop conditions reparsed every pass

Scenario: `evaluate_condition` over N passes of an `until` condition reading
two `loop.*` scalars (public).

| Passes | `dm.ExpressionParse` | `LoopAmbientMaterialize` | Bytes | Time, provisional |
| ------ | -------------------- | ------------------------ | ----- | ----------------- |
| 10 | 10 | 20 | 74,006 | 53 us |
| 100 | 100 | 200 | 738,116 | 0.52 ms |
| 1,000 | 1,000 | 2,000 | 7,379,216 | 8.8 ms |

One parse alone: 1,689 B, 34 allocations, 1.96 us (Criterion 0.99 us). One
whole condition evaluation: 7,379 B, Criterion 2.35 us.

- **Scaling:** constant per pass (one parse), linear in passes like the
  useful work.
- **Anchor share** (one pass of a trivial shell task): one `sh -c true`
  spawn took 32.1 ms (provisional, loaded host), so the parse is about
  **0.006% of a pass's time**. The pass's in-process bytes outside the
  condition were not measured (the spawn itself allocates 278 B in-process),
  so the byte share is not stated.
- **Clauses:** none. 1,000 passes reparse for 2 ms and 1.7 MB.
- **Recommendation: close (measured, no change).** The larger per-pass cost
  is C4's two `loop` object materializations, which C4's change removes.

### C6: pre-warm rebuilds state per phrase

Scenario: `classify` over a batch of phrases, as `PrewarmHandle::queue` does
(crate-private). "literal" phrases are plain text; "fm-read" phrases read
`{{ title }}`.

| Frontmatter | Phrases | `PrewarmStateBuild` | Bytes, literal / fm-read | Peak | Time, literal, provisional |
| ----------- | ------- | ------------------- | ------------------------ | ---- | -------------------------- |
| 10 KB | 2 | 2 | 106,604 / 112,066 | 46,559 | 16 us |
| 10 KB | 20 | 20 | 1,064,136 / 1,118,786 | 46,559 | 0.16 ms |
| 10 KB | 200 | 200 | 10,640,176 / 11,187,246 | 46,559 | 1.47 ms |
| 200 KB | 2 | 2 | 1,515,204 / 1,520,666 | 719,703 | 0.12 ms |
| 200 KB | 20 | 20 | 15,150,136 / 15,204,786 | 719,703 | 1.22 ms |
| 200 KB | 200 | 200 | 151,500,176 / 152,047,246 | 719,703 | 12.4 ms |

A literal phrase builds the same state as one that reads frontmatter (53 KB
per phrase at 10 KB, 757 KB at 200 KB). Peak stays flat: each state is
dropped before the next.

- **Scaling:** batch-proportional: phrases x frontmatter size.
- **A4 share, estimated:** A4's in-process run submits no pre-warm. With one
  phrase per prompt document, 49 phrases at about 387 KB each (interpolated
  to 100 KB frontmatter) = 19.0 MB = 2.2% of bytes; about 1.6 ms = 0.65% of
  time, provisional (loaded host).
- **Clauses:** 2, marginally, on bytes only: 20 phrases over a 200 KB
  frontmatter allocate 15.2 MB (time 1.2 ms). Real prompt documents carry a
  few phrases, so 20 is near the top of plausible.
- **Recommendation: implement the candidate** (literal fast path, shared
  state built lazily), as a low-priority item. The author may reasonably
  close it given that clause 2 is met only by bytes and only near the top of
  the plausible range.

### C7: provider prediction clones state for literal hints

Scenario: `predict` for one task alone (crate-private), and
`build_preflight_graph` over N prompt tasks sharing one 100 KB-frontmatter
document with `agent: codex` (public).

| Case | `PredictionStateBuild` | Bytes | Time, provisional |
| ---- | ---------------------- | ----- | ----------------- |
| one `predict`, literal `agent` | 2 | 231,521 | 22.7 us |
| one `predict`, hintless | 1 | 115,830 | 12.5 us |
| the caller's frontmatter copy before `predict` | - | 115,080 | 7.2 us |
| preflight, 10 tasks | 20 | 5,825,309 | 3.4 ms |
| preflight, 100 tasks | 200 | 63,037,228 | 26.6 ms (Criterion 14.8 ms) |
| preflight, 1,000 tasks | 2,000 | 2,329,999,505 | 2,345 ms |

- **Scaling:** per task, linear in the document's frontmatter size while the
  useful work (read one literal) is constant; batch-proportional in tasks.
  (The whole preflight grows faster than that; see
  [Surprises](#surprises).)
- **A4 share:** 49 x 231,521 B = 11.3 MB = **1.35% of bytes** (2.0% with the
  caller's copy); 49 x 22.7 us = 1.1 ms = 0.44% of time, provisional (loaded
  host). Of the preflight part alone it is 9.8% of bytes.
- **Clauses:** 2. At 100 tasks with 100 KB frontmatter, prediction copies
  23.2 MB (34.7 MB with the caller's copy).
- **Recommendation: implement the candidate** (classify hints before
  building state), and remove the caller's copy in
  `preflight/mod.rs` (`frontmatter: Map` before `providers::predict`) in the
  same change, since the classification needs only the two hint values.

### C8: document entry copies environment maps it replaces

Scenario: `InvocationContext::enter_document` (public), each entry with a
fresh `DocumentEnvironment` of V variables.

| Variables | Entries | `EnvironmentMapCopy` | Bytes | Bytes per entry | Time per entry, provisional |
| --------- | ------- | -------------------- | ----- | --------------- | --------------------------- |
| 50 | 10 / 100 / 1,000 | 40 / 400 / 4,000 | 657,286 / 6,570,916 / 65,707,216 | 65,707 | 34-103 us (Criterion n/a) |
| 500 | 10 / 100 / 1,000 | 40 / 400 / 4,000 | 4,219,286 / 42,190,916 / 421,907,216 | 421,907 | 231-450 us (Criterion 106 us) |

One map clone alone: 5,940 B (50 variables), 77,180 B (500). An entry costs
about 11 (50 variables) or 5.5 (500) map-equivalents; it also rebuilds the
child-process environment, which is needed work. The two copies of the
launch map that `with_env` discards are the avoidable part.

- **Scaling:** linear in entries x variables; per entry constant, like the
  useful work (capturing the document's environment).
- **A4 share, estimated:** A4 runs no entry in-process. A real launch enters
  about 249 documents (49 prompt tasks, 200 legs): 16.4 MB in total (1.9%),
  of which the avoidable copies are at least 2 x 5,940 B x 249 = 3.0 MB
  (0.35%); avoidable time about 1.2 ms (0.46%), provisional (loaded host).
- **Clauses:** none.
- **Recommendation: close (measured, no change).**

### D3: optional scrub and launch check each rediscover triggers

Scenario: `pre_validate_schema` (public) on a document matching one of N disk
triggers in `SCHEMAS_DIR`, with the embedded Claudine bundle; and one
discovery alone (`DarkmatterSchemas::new(..).with_embedded_trigger_bundle(..).with_trigger_discovery()`).

| Schema-root files | Pre-validation: `TriggerDiscovery` / `EffectivePreparation` | Pre-validation bytes / time | One discovery bytes / time |
| ----------------- | ----------------------------------------------------------- | --------------------------- | -------------------------- |
| 1 | 2 / 2 | 333,557 / 0.32 ms | 158,154 / 0.15 ms |
| 10 | 2 / 2 | 1,936,255 / 1.10 ms (Criterion 0.92 ms) | 959,503 / 0.51 ms (Criterion 0.41 ms) |
| 100 | 2 / 2 | 17,955,571 / 9.87 ms | 8,969,161 / 4.50 ms |

The first pre-validation in a process allocates 253 MB (2,229,787
allocations; the embedded bundle's preparation); the rows are warm.

- **Scaling:** constant count (two discoveries per pre-validation); each
  discovery linear in schema-root files. The repeated discovery is about half
  of a warm pre-validation's bytes and time.
- **A4 share, estimated:** A4's preflight runs no discovery (counter 0);
  pre-validation happens at each prompt launch. With 49 launches at the
  realistic 10 files: 49 x 959,503 B = 47.0 MB of (843.4 MB + 49 x 1.94 MB) =
  **5.0% of bytes**; 49 x 0.51 ms = 25.1 ms of 305.6 ms = 8.2% of time,
  provisional (loaded host). The composite omits each launch's composition
  and provider start, so the true share of a real A4 run is lower.
- **Clauses:** 1, at the threshold, on an estimate that overstates the share.
- **Recommendation: implement the candidate** (one document-scoped
  preparation sharing the registry through `with_trigger_registry`), at low
  priority. If the author wants clause 1 shown on a measured rather than
  estimated anchor, close it.

### X2: one directory walk per file alternative in completion (direct metric only)

Scenario: `file_candidate_paths` (crate-private to `claudine-cli`) over a
tree of 12,000 files (20 directories x 500 Markdown + 100 JSON), K
alternatives, one walk each. Filesystem-dependent: macOS APFS, warm cache.

| Alternatives | Walks | Files found | Time, provisional (loaded host) |
| ------------ | ----- | ----------- | ------------------------------- |
| 1 | 1 | 10,000 | 372 ms |
| 2 | 2 | 10,000 | 642 ms |
| 4 | 4 | 10,000 | 1,333 ms |
| 8 | 8 | 10,000 | 3,214 ms |

- **Scaling:** linear in alternatives, about 300-400 ms per extra walk at
  this size (the checkout itself tracks 17,604 files).
- **Threshold:** crossed by a wide margin (one extra alternative is ~270 ms).
- **Recommendation: write the candidate design.** One alternative already
  costs 372 ms, so the single walk's own cost (including the per-file
  canonicalization in the de-duplication) may be a larger lever than sharing
  walks; the design should measure both. This claim depends on the
  filesystem and is macOS-only evidence.

## Summary

| ID | Direct metric, largest size | Scaling | Anchor share (bytes / time, time provisional) | Clause | Recommendation |
| -- | --------------------------- | ------- | --------------------------------------------- | ------ | -------------- |
| C1 | peak 669 MB at 1,000 legs, cap 4 | quadratic (inline group) | A4: 19.1% / 2.1% | 1, 3 | candidate |
| C2 | 1.48 MB per leaf read at 1 MB | linear in payload per read | A5: 95.3% / 63.6% | 1, 2 | candidate |
| C3 | 10.3 MB per snapshot at 1,000 outputs | linear in history | A4: 44.4% (36.8% avoidable) / 2.6% | 1 | candidate |
| C4 | 1.47 MB per scalar read at 1 MB | linear in result per read | A5: 4.68% / 2.5% | 2 | candidate |
| C5 | 1 parse per pass (1.7 KB, ~1-2 us) | constant per pass | trivial-shell pass: ~0.006% time | none | close |
| C6 | 151.5 MB per 200 phrases at 200 KB | batch-proportional | A4 (est.): 2.2% / 0.65% | 2 (marginal) | candidate, low priority |
| C7 | 231.5 KB per task at 100 KB | batch-proportional | A4: 1.35% / 0.44% | 2 | candidate, plus the caller's copy |
| C8 | 4 map copies, 421.9 KB per entry at 500 vars | linear, like useful work | A4 (est.): 0.35% avoidable / 0.46% | none | close |
| D3 | 2 discoveries per pre-validation; 0.96 MB each at 10 files | constant count | A4 (est.): 5.0% / 8.2% | 1 (at threshold, estimated) | candidate, low priority |
| X2 | +~300-400 ms per extra alternative, 12k files | linear in alternatives | - | threshold crossed | write candidate design |

## Surprises

1. **A sequence document's frontmatter contains its own `sequence:` list,
   and several per-step or per-leg operations copy or process that whole
   frontmatter, so they are quadratic in the sequence's size.** C1's live
   copies (455 KB each at 1,000 legs); A6's parses (`10 * S^2 + 90 * S`);
   and preflight with 1,000 prompt tasks allocating 2.33 GB against 63 MB at
   100 (37x for 10x), of which prediction is only 231 MB. None of the review's
   findings names this; it would need its own finding.
2. In A4's group step, full runtime snapshots (C3) cost more than the up-front
   leg state (C1): 374 MB against 161 MB.
3. Each loop-condition pass materializes the whole `loop` object twice (C4
   inside C5), which outweighs the reparse C5 names.
4. The first pre-validation in a process allocates 253 MB, about 760x a warm
   one at the same size (333,557 B).

## Limits

- **Every time is provisional (loaded host).** Shares by time use the
  allocation harness's per-unit and total times so numerator and denominator
  come from the same harness; Criterion values are cross-checks.
- **A4 is a library-side proxy**, not `claudine sequence --dry-run`: the
  preflight and the group step run in-process (the group with fake shells,
  through the task tests' fixture). Prompt launches are not run, so C6, C8,
  and D3 shares are estimates built from per-unit costs and assumed counts
  (one phrase and one pre-validation per prompt document; 249 document
  entries).
- **Per-unit x count shares** assume every counted unit costs what it costs
  alone at the anchor's shape. For C3 the 608,270 B per snapshot is the
  outer runtime's size; leg buffers start from the same base, so this is
  close, but not measured per site. The avoidable 36.8% is an upper bound.
- **C6** at 100 KB frontmatter is interpolated linearly between 10 KB and
  200 KB.
- **C5's anchor** is one process spawn, not a loop of a shell task through
  the executor; its byte share is not stated.
- **A5** is synthetic: hook dispatch and loop lookups do not share one
  context in Claudine. The 100 KB previous result is my choice; the spec fixes
  only the 1 MB `tool_response`.
- **X2** is filesystem- and OS-dependent and was measured once, on macOS.
- Cold (first-run) numbers equal warm ones except where stated (D3; C5's
  first run of 13 MB).

## Load averages

`sysctl -n vm.loadavg`, immediately before and after each timing run:

| Run | Before | After |
| --- | ------ | ----- |
| `pipeline_work_alloc`, all scenarios (C2, C4, C5, C7, C8, A4 preflight, A5, A6 first run) | `{ 57.93 51.06 35.55 }` | `{ 63.70 52.69 36.48 }` |
| `pipeline_work_alloc -- D3`, `-- A6` (D3 rows, A6 second run and 10-step row) | `{ 79.71 90.51 63.59 }` | `{ 79.71 90.51 63.59 }` |
| crate-private scenarios (C1, C3, C6, C7 per-task, A4 group step) | `{ 57.43 84.39 62.16 }` | `{ 38.92 68.04 59.12 }` |
| Criterion `pipeline_work` | `{ 35.08 48.29 52.28 }` | `{ 28.11 45.10 50.96 }` |
| X2 | `{ 21.38 42.47 49.88 }` | `{ 69.48 51.77 51.60 }` |

## Reproduce

Each target runs on its own; a trailing filter narrows it. Record
`sysctl -n vm.loadavg` before and after.

```sh
# Allocation and work-count rows, public scenarios (C2 C4 C5 C7 C8 D3 A4 A5 A6)
cargo bench -p claudine --features work-counters --bench pipeline_work_alloc
cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A5

# Criterion timings, system allocator, no counters compiled in
cargo bench -p claudine --bench pipeline_work
cargo bench -p claudine --bench pipeline_work -- A5_event_dispatch

# Crate-private scenarios (C1, C3, C6, C7 per task, A4 group step)
cargo test -p claudine --lib --release --features work-counters -- \
    --ignored --nocapture --test-threads=1 pipeline_spike

# X2 (claudine-cli)
cargo test -p claudine-cli --release --bin claudine -- \
    --ignored --nocapture --test-threads=1 x2_spike
```

`--test-threads=1` is required: the counters and the allocator are
process-wide.

## Files

Counter call sites (the only production-code change; each compiles to nothing
without `work-counters`):

- `claudine/lib/src/work_counters.rs`: variants `RuntimeStateBranch`,
  `LiveFrontmatterCopy`, `HookPayloadMaterialize`, `LoopAmbientMaterialize`,
  `PrewarmStateBuild`, `PredictionStateBuild`, `EnvironmentMapCopy` (and `ALL`)
- `claudine/lib/src/composition/runtime_state.rs`: `RuntimeSnapshot` in `snapshot()`
- `claudine/lib/src/composition/sequence/task/group.rs`: `RuntimeStateBranch`,
  `LiveFrontmatterCopy` in `run_parallel`
- `claudine/lib/src/dispatch/expression.rs`: `HookPayloadMaterialize` in `hook_payload_object`
- `claudine/lib/src/composition/looping/expression.rs`: `LoopAmbientMaterialize` in `LoopAmbient::to_value`
- `claudine/lib/src/composition/lifecycle/prewarm/classify.rs`: `PrewarmStateBuild` in `resolve`
- `claudine/lib/src/composition/sequence/preflight/providers.rs`: `PredictionStateBuild` in `predict` and `predict_key`
- `claudine/lib/src/invocation_context.rs`: `EnvironmentMapCopy` (4) in `enter_document`

Spike tooling (test or bench only):

- `claudine/lib/benches/pipeline_work_alloc.rs`, `claudine/lib/benches/pipeline_work.rs`,
  registered in `claudine/lib/Cargo.toml`
- `claudine/lib/src/pipeline_spike.rs` (helpers), `claudine/lib/src/pipeline_spike/tests.rs` (C6, C7 scenarios), and the counting allocator in
  `claudine/lib/src/lib.rs`, both `cfg(all(test, feature = "work-counters"))`
- `claudine/lib/src/composition/sequence/task/tests/pipeline_spike.rs`, included from
  `task/tests.rs` under `cfg(feature = "work-counters")`
- `claudine/cli/src/completion/schema_completion/tests/x2_spike.rs`, included
  from `schema_completion/tests.rs`
