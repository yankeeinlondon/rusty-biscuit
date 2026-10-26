---
kind: implementation-log
created: 2026-10-26
feature: 2026-10-25-pipeline-work-reduction
scope: phase 1 spike, Darkmatter and DMLS half (D1, D2, D4-D13, X1, X3, X4; anchors A1, A2, A3)
---

# Spike data: Darkmatter and DMLS

This is the Darkmatter/DMLS half of the phase 1 measurement spike. The
Claudine half (C1-C8, D3, A4-A6) is recorded separately, and the two are
merged into `spike-report.md`.

**Timings are provisional.** Every wall time and every time-percentage
share below was taken on a heavily loaded host (load average 28-90 on 16
cores; three agents compiling and testing at once). They are labeled
"provisional (loaded host)" wherever they appear. Work counts, allocation
counts, bytes allocated, and peak live bytes do not depend on load and are
authoritative. A serial timing pass on a quiet host will replace the times;
the commands are in [Reproduce](#reproduce).

## Environment

| | |
| - | - |
| Host | Apple M4 Max, 16 cores, 128 GiB, macOS 27.2 (26B5091g) |
| Toolchain | rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1 (pinned by `rust-toolchain.toml`) |
| Profile | `bench` (Cargo's default, inheriting `release`; no workspace override) |
| Commit | `c582c8bc04f6a8dd418c2b38fb3a7822f843bd75` plus the uncommitted counter call sites and benches listed in [Files](#files) (measurement began on `e2c67b091`; the two commits between committed the shared tooling this spike uses, and `84c55d16c` also picked up this spike's `PositionMapBuild` call site in `schemas/mod.rs`) |
| Load average, alloc run (`pipeline_work_alloc`) | before `{ 31.98 65.08 58.22 }`, after `{ 27.64 62.50 57.42 }` |
| Load average, A3 run (`pipeline_work_a3`) | before `{ 79.71 90.51 63.59 }`, after `{ 56.92 83.84 62.10 }` |
| Load average, Criterion run (`pipeline_work`) | see [Criterion timings](#criterion-timings-provisional-loaded-host) |

## Method

- **Counts** come from `darkmatter::work_counters` (process-wide, compiled
  only with the test-only `work-counters` feature) and, for A3, also from
  `dmls::work_counters`.
- **Allocations, bytes, and peak live bytes** come from
  `test_toolkit::alloc_count::CountingAllocator`, declared only in the
  `pipeline_work_alloc` and `pipeline_work_a3` bench binaries. Each case is
  counted on one run. A warm case is counted after one unmeasured run; a cold
  case runs `reset_process_caches_for_bench()` first, outside the measurement.
- **Time** in the item tables is the median of five runs inside the counting
  binaries, so it includes the counting overhead (provisional, loaded host).
  Criterion medians with the system allocator are listed separately.
- **Share of an anchor** uses one of three methods, named in each row:
  - *differential*: the same operation with and without the repeated work,
    both through public APIs (D1, D2, D4, D9);
  - *count x unit cost*: the item's counter inside the anchor times its
    isolated per-unit cost. Where a per-unit cost cannot be isolated, it is
    fitted from the three synthetic sizes (time = a*n + b*count) or bounded
    from above by attributing the whole isolated run to the counted unit;
    such a bound is labeled *upper bound*;
  - *direct*: the anchor's own phase, measured separately (X1).

### Thresholds applied

The spec's proposal, unchanged: (1) share at least 5% of wall time or bytes
on the anchor; (2) growth faster than the useful work reaching at least
10 ms or 10 MB at a plausible size (no more than 10x the largest real
instance); (3) peak live bytes growing with an uncapped count.

"Largest real instance" for the editor and lifecycle items is A3's document
(`prompts/_implement/implement-plan.md`, 143 frontmatter lines); its 10x
scale-up (`a3x10`, every lifecycle stack and the loop gate repeated ten times)
is the plausible ceiling used for clause 2.

## Anchor totals

| Anchor | Case | Allocations | Bytes allocated | Peak live | Time, provisional (loaded host) |
| ------ | ---- | ----------: | --------------: | --------: | ------------------------------: |
| A1 corpus validate | 1,000 docs, warm | 1,083,804 | 87,858,668 | 55,211 | 25.1 ms |
| A1 | 1,000 docs, cold | 1,084,714 | 87,961,182 | 94,568 | 26.0 ms |
| A1 + runtime baseline (the `md schema validate` default) | warm | 1,159,813 | 92,912,756 | 57,115 | 30.8 ms |
| A1 + runtime baseline | cold | 1,967,083 | 202,628,926 | 22,957,873 | 59.0 ms |
| A1 + `effective_for` presence check (what `md schema validate` runs, D2) | warm | 1,909,500 | 157,117,226 | 33,769 | 40.9 ms |
| A1 variant: 3-item eager file list, prepare + validate + normalize (D1) | warm | 3,337,000 | 345,202,560 | 28,956 | 330.2 ms |
| A2 Claudine bundle: `no_trigger_match` | warm | 19,516 | 3,130,659 | 538,677 | 1.32 ms |
| A2 `no_trigger_match` | cold | 2,284,196 | 333,754,341 | 161,184,669 | 118.4 ms |
| A2 `document_events` | warm | 21,495 | 3,192,599 | 538,677 | 1.44 ms |
| A2 `document_events` | cold | 2,944,197 | 415,865,122 | 212,021,771 | 153.7 ms |
| A2 `nested_sequence_group` | warm | 22,090 | 3,218,165 | 538,677 | 1.40 ms |
| A2 `nested_sequence_group` | cold | 3,473,384 | 493,749,290 | 274,584,434 | 183.1 ms |
| A2 `implement_plan` (the A3 document) | warm | 40,880 | 4,621,686 | 538,677 | 2.67 ms |
| A2 `implement_plan` | cold | 4,512,290 | 810,380,271 | 295,794,838 | 245.7 ms |
| A3 DMLS edit loop, `a3x1` | warm | 6,021,888 | 1,207,807,451 | 56,171,594 | 317.0 ms |
| A3 `a3x1` | cold | 8,600,473 | 1,729,019,852 | 120,686,502 | 874.5 ms |
| A3 `a3x10` | warm | 17,063,227 | 3,305,397,053 | 417,946,731 | 832.9 ms |
| A3 `a3x10` | cold | 19,641,812 | 3,826,609,454 | 482,461,639 | 2,401.9 ms |

A3 by phase (warm; bytes allocated / time provisional (loaded host)):

| Phase | `a3x1` | `a3x10` | Darkmatter work counted (`a3x1` / `a3x10`) |
| ----- | -----: | ------: | ----------------------------------------- |
| overlay | 364.7 MB / 87.6 ms | 373.7 MB / 88.4 ms | `EffectivePreparation` 1 / 1 |
| diagnostics | 525.5 MB / 142.9 ms | 970.2 MB / 255.0 ms | `ExpressionParse` 587 / 4,412; `LifecycleSourceMapBuild` 2 / 2; `ScopeJoinAncestorVisit` 417 / 20,154; `OperandSiteComparison` 1,107 / 97,506; `OperandMarksComputation` 516 / 4,593; `CoercionFailureBuild` 22 / 22; `AlternativeMatcherPrepare` 6 / 6; `LifecycleConfigParse` 1 / 1 |
| completion | 17.9 MB / 4.4 ms | 17.9 MB / 4.6 ms | none |
| hover (verb `ensure_file`) | 155.0 MB / 40.7 ms | 976.9 MB / 243.4 ms | `AlternativeMatcherPrepare` 6 / 6 |
| hover (expression `file_exists`) | 144.8 MB / 36.8 ms | 966.7 MB / 237.8 ms | none |

DMLS's own counters (reads, hashes, parses, enumerations, registry scans)
were zero in every warm and cold loop: the server-lifetime state absorbs
them after the first loop.

## Items

Each table gives the direct metric at each size, allocations/bytes/peak, and
time (provisional, loaded host). Counts and bytes are exact.

### D1: eager-file rewrite recompiles union-arm validators per item (medium)

Direct metric: `RewriteArmCompile` (each also a `ValidatorCompile`) per rewrite.

| Case | Compiles | Allocations | Bytes | Peak live | Time (provisional) |
| ---- | -------: | ----------: | ----: | --------: | -----------------: |
| isolated, 10 items | 20 | 5,577 | 538,864 | 20,401 | 0.48 ms |
| isolated, 100 items | 200 | 55,707 | 5,385,274 | 100,411 | 5.0 ms |
| isolated, 1,000 items | 2,000 | 557,007 | 53,849,374 | 900,511 | 49.9 ms |
| A1 variant, normalize only (1,000 docs x 3 items) | 6,000 | 1,689,000 | 162,444,780 | 14,671 | 146.5 ms |
| A1 variant, prepare + validate + normalize | 6,000 | 3,337,000 | 345,202,560 | 28,956 | 330.2 ms |

- **Scaling:** batch-proportional, two compilations per item (one per
  eligible arm); about 27 KB and 25 us per compilation.
- **Share of anchor (A1 variant), differential:** normalization is **47.1% of
  bytes** and **44.4% of time (provisional)**; its 6,000 compilations are
  essentially all of it.
- **Clause met:** 1 (share), and 2 (10 MB / 10 ms reached at 1,000 items).
- **Recommendation: implement, with one addition for the author.** The
  candidate (a memo inside one rewrite invocation) cuts the A1 variant from 6
  to 2 compilations per document, since each document is its own invocation.
  Getting the arm validator from `ValidatorCache` (design rule 1, "get every
  validator from `ValidatorCache`") would make it 2 per process, and the cache
  key already carries `base_dir` and the request context. That reuses a
  compiled validator, not a verdict, so it keeps D1's invariant ("nothing is
  reused across values or requests") as it concerns verdicts; the author
  should confirm that reading before phase 2.

### D2: `md schema validate` prepares the effective schema twice

Reproduces `cli/src/commands/schema/validate.rs::validate_one` after
assignments: `api.validate(&md)` then `api.effective_for(&md)?.is_none()`.
The A1 corpus is outside a repository, so `validate_one` would run no trigger
discovery either. Direct metric: `EffectivePreparation` per file (1 is
needed, 2 is what runs).

| Case | Preparations | Allocations | Bytes | Time (provisional) |
| ---- | -----------: | ----------: | ----: | -----------------: |
| validate only, 10 / 100 / 1,000 files | 10 / 100 / 1,000 | 10,817 / 108,170 / 1,083,500 | 875,527 / 8,757,466 / 87,837,226 | 0.25 / 2.43 / 24.9 ms |
| validate + presence, 10 / 100 / 1,000 | 20 / 200 / 2,000 | 19,077 / 190,770 / 1,909,500 | 1,568,327 / 15,685,466 / 157,117,226 | 0.41 / 4.22 / 40.9 ms |
| baseline, validate only, 1,000 | 1,000 | 1,159,500 | 92,887,226 | 28.4 ms |
| baseline, validate + presence, 1,000 | 2,000 | 2,009,500 | 165,815,226 | 47.2 ms |

- **Scaling:** linear (one extra preparation per file, about 69 KB and 16 us).
- **Share of anchor (A1 as `md schema validate` runs it), differential:**
  **44.1% of bytes, 39.1% of time (provisional)**; with the runtime baseline
  44.0% / 39.9%.
- **Clause met:** 1.
- **Recommendation: implement the candidate** (report plus a preparation
  summary from the same `Option<EffectiveSchema>`).

### D4: source positions built before knowing a schema applies

`validate` (positions first) against a control that prepares the schema and
validates without positions. Direct metric: `PositionMapBuild`.

| Corpus (1,000 docs) | Builds | Bytes with / without positions | Time with / without (provisional) | Positions' share (bytes / time) |
| ------------------- | -----: | ------------------------------: | --------------------------------: | ------------------------------: |
| A1 (inline schema, 90% valid) | 1,000 | 87,837,226 / 72,866,949 | 25.0 / 18.2 ms | 17.0% / 27.4% |
| schema-free, no baseline | 1,000 | 6,444,188 / 1,428,911 | 2.88 / 0.39 ms | 77.8% / 86.4% |
| schema-free, runtime baseline (CLI default) | 1,000 | 34,398,188 / 29,382,911 | 12.3 / 9.5 ms | 14.6% / 22.5% |

- **Scaling:** linear; about 15 KB and 6.9 us per A1 document, 5 KB per
  schema-free document.
- **Share of anchor:** differential, above. The A1 difference also includes
  locating the 100 invalid documents' problems (300 key comparisons), which
  is needed work and negligible.
- **Clause met:** 1.
- **Recommendation: implement a different option, the lazy position
  provider (the review's second solution).** The candidate (build positions
  only in the `Some(effective)` branch) saves only the schema-free,
  no-baseline row. With the runtime baseline, which `md schema validate`,
  compose, and DMLS apply by default, every document has `Some(effective)`,
  and in A1 every document has a schema, so the candidate saves nothing in
  either. Positions are needed only when a problem exists, and 90% of A1
  documents are valid.

### D5: scope join rebuilds ancestor sets

The passive lifecycle call check (`lifecycle_call_checks`, Claudine's
registry) over one `start` stack of N conditionals, each guarding its own
optional property. Direct metric: `ScopeJoinAncestorVisit` (links visited
building address sets and walking back to the shared history).

| Conditionals | Visits | Allocations | Bytes | Peak live | Time (provisional) |
| -----------: | -----: | ----------: | ----: | --------: | -----------------: |
| 10 | 192 | 1,698 | 145,572 | 32,589 | 0.046 ms |
| 100 | 15,447 | 17,517 | 1,825,168 | 293,894 | 0.83 ms |
| 1,000 | 1,504,497 | 184,680 | 58,716,764 | 3,045,203 | 48.0 ms |

- **Scaling:** quadratic in the conditionals of one stack. Fitted unit cost
  (100 and 1,000): 30.0 bytes and 29.4 ns per visit (provisional), against
  13.6 KB and 3.7 us per conditional of useful work.
- **Share of anchor (A3), count x unit cost:** `a3x1`: 417 visits, about
  12.5 KB and 12 us, **0.001% of bytes, 0.004% of time**; `a3x10`: 20,154
  visits, about 0.6 MB and 0.59 ms, **0.02% / 0.07%**.
- **Clause met:** none. Clause 2's growth holds, but 10 ms needs about 600
  conditionals in one stack; `a3x10` reaches 0.6 ms.
- **Recommendation: close, measured no change.**

### D6: diagnostic positions rescan the prefix

`one_based_position` for N findings in the last thousand lines of a
10,000-line (551 KB) source. Direct metric: `PositionPrefixScanBytes`.

| Findings | Bytes scanned | Allocations | Time (provisional) |
| -------: | ------------: | ----------: | -----------------: |
| 10 | 5,481,070 | 0 | 0.93 ms |
| 100 | 55,071,700 | 0 | 9.2 ms |
| 1,000 | 550,978,000 | 0 | 92.5 ms |

- **Scaling:** findings x prefix length (quadratic in file size when findings
  grow with it); about 0.17 ns per byte; no allocation.
- **Share of anchor (A3):** **0%**. DMLS never calls `one_based_position`
  (A3 counts zero bytes scanned); it ranges its diagnostics through its own
  source map. The callers are composition's call precheck and located
  lifecycle findings, on the `md compose` path, which no anchor in this half
  covers.
- **Clause met:** none. At the plausible ceiling (10x the largest real
  document, about 230 KB) 100 late findings scan about 23 MB, about 4 ms.
- **Recommendation: close, measured no change.**

### D7: operand-site discovery scans linearly and recomputes markers

A Claudine document with N `info` actions in `start`, validated warm against
the frozen bundle. Direct metrics: `OperandSiteComparison`,
`OperandMarksComputation`, and (added to bound the latter)
`OperandMarksNodeVisit`.

| Actions | Site comparisons | Marks computations | Marks node visits | Allocations | Bytes | Time (provisional) |
| ------: | ---------------: | -----------------: | ----------------: | ----------: | ----: | -----------------: |
| 10 | 100 | 108 | 233 | 21,041 | 3,185,285 | 1.36 ms |
| 100 | 10,000 | 1,008 | 2,213 | 33,210 | 3,586,411 | 1.87 ms |
| 1,000 | 1,000,000 | 10,008 | 22,013 | 155,628 | 7,301,983 | 9.9 ms |

- **Scaling:** site comparisons quadratic (N^2); marks linear (about 10 per
  action, about 2.2 node visits each). Comparisons allocate nothing. Fitted
  time (provisional): 2.6 ns per comparison, 5.9 us per action of useful
  validation.
- **Share of anchor, count x unit cost:** A2 `implement_plan` warm: 1,107
  comparisons (about 3 us, **0.1%** of 2.67 ms), 516 marks computations over
  1,046 node visits (at a generous 100 ns per visit, 0.1 ms, **at most 3.9%**;
  marks allocate only a short verb string and form list). A3 `a3x10`: 97,506
  comparisons, about 0.25 ms, **0.03%** of time, 0% of bytes.
- **Clause met:** none. 10 ms of comparisons needs about 2,000 action sites
  in one document; `a3x10` has about 440.
- **Recommendation: close, measured no change.**

### D8: placeholders reparsed across analysis passes

Measured in isolation through the template API: `parse_authored`,
`project_types`, and `DeferredShellRoots::late_roots_of` per placeholder,
which is `CandidateScope::build`'s public sequence (its guard lookup, for a
late placeholder only, is not reached). Direct metric: `ExpressionParse` per
command.

| Holes | Parses | Allocations | Bytes | Peak live | Time (provisional) |
| ----: | -----: | ----------: | ----: | --------: | -----------------: |
| 1 | 3 | 33 | 3,434 | 2,002 | 1.3 us |
| 10 | 30 | 253 | 21,899 | 9,772 | 8.0 us |
| 50 | 150 | 1,217 | 103,883 | 44,044 | 38.3 us |
| one parse of `fm.a1` | 1 | 5 | 215 | 210 | 0.17 us |

- **Scaling:** linear, three parses per hole where one is needed. The two
  redundant parses cost about 430 bytes and 0.33 us per hole (provisional);
  at 50 holes that is 21.5 KB of 104 KB (21%) and 17 us of 38 us (44%) of
  the template analysis.
- **Share of anchor (A6):** recorded by the Claudine half, which counts
  `ExpressionParse` in A6; multiply its redundant-parse count by the per-parse
  cost above (215 bytes, 0.17 us provisional).
- **Clause met:** pending A6's share. Clause 2 is not met: A6's 1,000 holes
  make about 2,000 redundant parses, about 0.4 MB and 0.33 ms.
- **Recommendation:** close unless A6 shows at least 5%.

### D9: lifecycle grammar clones excluded subtrees

`parse_lifecycle_config_with` under Claudine's host profile (registry, verbs,
and effects from the frozen bundle's lifecycle artifact). A stack of plain
`info` actions placed under the loop's `gate` against the same stack under
`start`; and one guarded compact item (`when` beside a verb carrying the
payload) against the same item unguarded. Direct metric: bytes copied per
parse, differential.

| Payload | Gate stack bytes (gate / start / extra) | Guarded item bytes (guarded / unguarded / extra) | Extra time, gate / guarded (provisional) |
| ------: | --------------------------------------: | -----------------------------------------------: | ---------------------------------------: |
| 1 KB | 26,281 / 12,935 / 13,346 | 4,314 / 2,248 / 2,066 | 2.8 / 0.5 us |
| 10 KB | 254,251 / 130,565 / 123,686 | 22,314 / 11,248 / 11,066 | 23.1 / 0.3 us |
| 100 KB | 2,536,651 / 1,309,565 / 1,227,086 | 202,314 / 101,248 / 101,066 | 237.6 / 3.6 us |

- **Scaling:** linear in the excluded subtree: a gate stack is copied about
  twice (as the review said), so parsing allocates about 1.9x what the same
  stack costs under `start`; a guarded item's payload is copied once.
- **Share of anchor (A3):** one lifecycle parse per edit loop
  (`LifecycleConfigParse` 1). The whole parse of the A3 frontmatter is
  170,545 bytes and 70 us (`a3x1`), 1,625,064 bytes and 0.67 ms (`a3x10`), so
  the copies are an upper bound of **0.014% of bytes and 0.02% of time**
  (`a3x1`), **0.05% / 0.08%** (`a3x10`). The A3 document uses `when`/`then`
  bundles, never the compact guarded form, and its gate holds about 1.3 KB.
  A4 is the Claudine half's.
- **Clause met:** none. 10 MB would need a gate of about 800 KB.
- **Recommendation: close, measured no change** (unless A4 shows otherwise).

### D10: `LifecycleSourceMap` built separately per validation pass

Direct metric: `LifecycleSourceMapBuild` per preparation, and one build's
cost via `LifecycleSourceMap::from_frontmatter_in` (Claudine's registry).

| Case | Builds | Allocations | Bytes | Peak live | Time (provisional) |
| ---- | -----: | ----------: | ----: | --------: | -----------------: |
| one build, `a3x1` frontmatter | 1 | 2,063 | 128,200 | 41,309 | 47 us |
| one build, `a3x10` frontmatter | 1 | 19,079 | 1,213,931 | 361,972 | 437 us |
| A3 edit loop (diagnostics), `a3x1` / `a3x10` | 2 / 2 | | | | |

- **Scaling:** constant count (2 per preparation, 1 needed); each build is
  linear in the lifecycle's size.
- **Share of anchor (A3), count x unit cost** (the one removable build):
  `a3x1` **0.011% of bytes, 0.015% of time**; `a3x10` **0.04% / 0.05%**.
  A4 is the Claudine half's.
- **Clause met:** none.
- **Recommendation: close, measured no change.**

### D11: rejected-candidate diagnostics built on successful binds

`FunctionRegistration::bind_call` on a function whose last overload
(`unknown`) accepts an object argument the earlier overloads reject. Direct
metrics: `CandidateFailureBuild`, `CoercionFailureBuild`, bytes per
successful bind.

| Rejected overloads | 1 KB object | 10 KB | 100 KB |
| -----------------: | ----------: | ----: | -----: |
| 1 | 7,587 B, 57 allocs, 1.0 us | 47,087 B, 6.6 us | 602,087 B, 68 us |
| 4 | 26,459 B, 3.8 us | 184,459 B, 28 us | 2,404,459 B, 269 us |
| 8 | 52,242 B, 7.8 us | 368,242 B, 58 us | 4,808,242 B, 536 us |

Each bind builds exactly one `CandidateFailure` and one `ValueFailure` per
rejected overload; peak live equals bytes allocated (everything is held until
selection ends). Times provisional.

- **Scaling:** proportional to object size x rejected overloads: about
  600 KB per rejection of a 100 KB (authored size) object, where the failure
  clones the received value.
- **Share of anchor (A2), count x unit cost, upper bound:** A2 binds no call
  (`CandidateFailureBuild` 0). `implement_plan` builds 22 conversion failures
  per validation; at the 1 KB case's whole-bind cost (7.6 KB, 1.0 us, far
  above a string value's failure) that is at most **3.6% of bytes and 0.8% of
  time** of the warm validation (4.62 MB, 2.67 ms). A3: 22 per loop, under
  0.02%.
- **Clause met:** none. 10 MB needs more than 8 rejected overloads with a
  100 KB argument.
- **Recommendation: close, measured no change.**

### D12: named-type memo clears completely at 256 entries

An edit-and-reuse loop: each run edits one imported file (new bytes, so a new
revision) and resolves a schema importing every file. Direct metric:
`NamedTypeTableParse` per edit.

| Imported files | Parses per edit | Allocations | Bytes | Peak live | Time (provisional) |
| -------------: | --------------: | ----------: | ----: | --------: | -----------------: |
| 10 | 1 (the edited file) | 2,461 | 419,260 | 80,718 | 0.44 ms |
| 300 | 300 (the memo empties on every insert) | 113,632 | 53,423,433 | 810,698 | 15.7 ms |

- **Scaling:** constant below 256 live revisions; batch-proportional above.
  A long session that edits one schema also adds one revision per edit, so
  every 256th edit empties the memo and reparses every hot table once.
- **Share of anchor (A3):** 0 parses in the warm edit loop.
- **Clause met:** none at a plausible size (the Claudine bundle imports 9
  files, 10x is 90, under 256).
- **Recommendation: implement (rule 1, regardless of numbers):** latest
  revision per path. These rows are the before-numbers phase 2 must not
  regress.

### D13: alphanumeric annotation checks compile validators directly

Resolving a schema with N imports of one `alphanumeric(suggest(ab, abc))`
type, cold. Direct metric: `AlphanumericCheckCompile` per resolution.

| Imports | Check compiles | All compiles | Allocations | Bytes | Time (provisional) |
| ------: | -------------: | -----------: | ----------: | ----: | -----------------: |
| 1 | 2 | 3 | 1,634 | 326,042 | 0.11 ms |
| 10 | 11 | 12 | 5,688 | 801,158 | 0.29 ms |
| 100 | 101 | 102 | 46,194 | 5,548,692 | 2.15 ms |

- **Scaling:** linear, one compilation per occurrence (plus one for the
  definition); about 52 KB and 20 us each.
- **Share of anchor (A2):** 0 (the Claudine bundle declares no suggested
  `alphanumeric`); A3: 0.
- **Clause met:** none.
- **Recommendation: implement (rule 1, regardless of numbers):** the
  `ValidatorCache` structural slot. Expected after: 1 check compile at every
  size.

### X1: cached alternative matchers for editor hover

Direct metric, measured inside A3: `AlternativeMatcherPrepare` = 6 per verb
hover (and 6 per diagnostics publish), at both `a3x1` and `a3x10`.

- **Cost, direct, upper bound:** the verb hover minus the expression hover
  (which prepares none): 10.2 MB and 3.9 ms (provisional) per hover at
  `a3x1`; 10.2 MB and 5.6 ms at `a3x10`. That is 0.85% of A3's bytes and 1.2%
  of its time (`a3x1`).
- **Recommendation: stay deferred (close).** The bound crosses 10 MB per
  hover only because it contains everything else that differs between the
  two hovers; it does not grow with the document.

### X3: memo of completed recursive conversion targets

Direct metric: `RecursiveTargetTranslate` for one cold
`shape_from_json_schema` of the frozen recursive lifecycle fixture (one
`$defs` entry, wired to seven event keys): **7 translations of 1 target**;
329,581 allocations, 65.5 MB, peak 15.9 MB, 14.5 ms (provisional; includes
two validator compiles). Neither A2 nor A3 translates a recursive target (the
frozen Claudine bundle's lifecycle is the unrolled one).

- **Recommendation: stay deferred (close).** At most 6 of 7 translations are
  removable, on a cold first translation of a schema revision only, on a path
  no anchor reaches.

### X4: key-to-child index for locating many problems in wide mappings

Direct metric: `PositionKeyComparison` for one document whose 1,000
top-level properties are all invalid: **501,500 comparisons** (N^2/2);
190,286 allocations, 23.4 MB (from the 6,000 conversion failures, not the
comparisons, which allocate nothing), 18.7 ms (provisional). At the D7 rate
of a few ns per comparison the scan is about 1.3 ms of it. A1 makes 300
comparisons per 1,000 documents.

- **Recommendation: stay deferred (close).**

## Summary

| Item | Scaling | Share of its anchor (bytes / time, time provisional) | Clause | Recommendation |
| ---- | ------- | --------------------------------------------------- | ------ | -------------- |
| D1 | batch-proportional | A1 variant 47.1% / 44.4% | 1, 2 | implement; add `ValidatorCache` (rule 1) for the author to confirm |
| D2 | linear | A1 (CLI sequence) 44.1% / 39.1% | 1 | implement candidate |
| D4 | linear | A1 17.0% / 27.4%; baseline schema-free 14.6% / 22.5% | 1 | implement the lazy position provider instead of the candidate |
| D5 | quadratic | A3x10 0.02% / 0.07% | none | close |
| D6 | findings x prefix | A3 0% / 0% | none | close |
| D7 | quadratic (sites), linear (marks) | A2 0.1% (sites), at most 3.9% (marks) | none | close |
| D8 | linear | A6: Claudine half | pending | close unless A6 is at least 5% |
| D9 | linear | A3x10 at most 0.05% / 0.08% | none | close |
| D10 | constant (2 builds) | A3x10 0.04% / 0.05% | none | close |
| D11 | size x rejections | A2 at most 3.6% / 0.8% | none | close |
| D12 | constant, then batch-proportional past 256 | A3 0% | none | implement (rule 1) |
| D13 | linear | A2 0% | none | implement (rule 1) |
| X1 | constant per hover | A3 at most 0.85% / 1.2% | none | stay deferred |
| X3 | 7 of 1 target | no anchor reaches it | none | stay deferred |
| X4 | quadratic | A1 negligible | none | stay deferred |

## Surprises

- **DMLS's own request cost dwarfs every item.** One warm A3 edit loop at
  `a3x1` allocates 1.21 GB; each hover alone allocates about 145 MB (peak
  56 MB) with no counted Darkmatter work, and at `a3x10` about 967 MB (peak
  418 MB). Diagnostics allocate 525 MB / 970 MB. None of the D or X items
  explains it. A request-scoped DMLS observation is out of this spec's scope
  ("needs its own spec backed by spike numbers"); these numbers are that
  backing.
- **A3's diagnostics parse 587 expressions for a document with about 90**
  (4,412 at `a3x10`). It is the editor-side analogue of D8, but at about
  0.17 us per parse it is 0.03-0.09% of the loop.
- **D4's candidate fixes the case that matters least.** The runtime
  baseline makes every document's effective schema `Some`.
- **D1's anchor share comes from per-document invocations.** The candidate
  memo alone leaves 2 compilations per document.
- **D7's quadratic site scan is real (1,000,000 comparisons at 1,000
  actions) but cheap** (about 2.6 ms).

## Limits

- **All times are provisional (loaded host)**, and the item-table times
  include the counting allocator's and counters' overhead. Shares built from
  times inherit both. Byte shares are exact.
- **D2** reproduces `validate_one`'s library calls rather than running the
  CLI, and skips assignments and trigger discovery (the A1 corpus is outside
  a repository).
- **D4** is differential: the control also skips problem location for the
  100 invalid documents (300 key comparisons; negligible).
- **D5, D7, D10, D11** shares are count x unit cost. D5's and D7's unit costs
  are fitted from three sizes; D7's marks and D11's failures are upper
  bounds (a generous 100 ns per marks node visit; the 1 KB whole-bind cost
  per conversion failure).
- **D6** has no anchor in this half: DMLS does not reach `one_based_position`.
- **D8** reproduces `CandidateScope::build`'s public pieces, not the
  crate-private function, and omits the guard lookup for late placeholders.
- **D9** measures copies differentially (gate against start, guarded against
  unguarded); its A3 share is bounded by the whole parse.
- **X1** is bounded by the difference of two different hovers.
- **A3** uses the frozen bundle plus `globals.lifecycle.json` copied from
  `claudine/schemas/` (its `globals.yaml` is identical, so the artifact is
  fresh) so the document gets Claudine's events, as it does in the
  repository. The DMLS L1 Claudine fixture omits that artifact.
- The A3 document is `prompts/_implement/implement-plan.md` (repository
  root); `claudine/prompts/` holds no lifecycle document.

## Criterion timings, provisional (loaded host)

`cargo bench -p darkmatter --bench pipeline_work` (system allocator, no
counters, `sample_size(10)`, 2 s measurement). Load average before
`{ 28.19 51.85 53.85 }`, after `{ 43.83 45.49 49.83 }`. Medians:

| Case | Median | Case | Median |
| ---- | -----: | ---- | -----: |
| A1/corpus_validate/1000-warm | 28.29 ms | D6/late_findings/10 | 930.8 us |
| A1/corpus_validate/1000-cold | 25.08 ms | D6/late_findings/100 | 9.438 ms |
| A1/corpus_validate_baseline/1000-warm | 31.48 ms | D6/late_findings/1000 | 95.12 ms |
| A1/corpus_validate_baseline/1000-cold | 63.14 ms | D7/start_actions/10 | 1.371 ms |
| A2/no_trigger_match-warm | 1.426 ms | D7/start_actions/100 | 2.521 ms |
| A2/no_trigger_match-cold | 123.7 ms | D7/start_actions/1000 | 11.59 ms |
| A2/document_events-warm | 1.545 ms | D8/template_holes/1 | 967.8 ns |
| A2/document_events-cold | 158.4 ms | D8/template_holes/10 | 16.42 us |
| A2/nested_sequence_group-warm | 1.646 ms | D8/template_holes/50 | 56.36 us |
| A2/nested_sequence_group-cold | 196.3 ms | D8/single_parse/1 | 183.1 ns |
| A2/implement_plan-warm | 2.781 ms | D9/gate_stack/1KB, 10KB, 100KB | 8.361 us, 79.99 us, 2.870 ms |
| A2/implement_plan-cold | 265.0 ms | D9/start_stack_control/1KB, 10KB, 100KB | 4.777 us, 94.56 us, 2.198 ms |
| D1/rewrite_isolated/10 | 547.5 us | D9/guarded_item/1KB, 10KB, 100KB | 1.491 us, 9.361 us, 124.5 us |
| D1/rewrite_isolated/100 | 5.567 ms | D9/unguarded_item_control/1KB, 10KB, 100KB | 1.246 us, 14.63 us, 146.8 us |
| D1/rewrite_isolated/1000 | 55.89 ms | D9/a3_parse/a3x1, a3x10 | 201.9 us, 943.3 us |
| D1/a1_variant_prepare_validate_normalize | 321.5 ms | D10/source_map_build/a3x1, a3x10 | 110.4 us, 579.9 us |
| D1/a1_variant_normalize_only | 144.8 ms | D11/1rej-1KB, 10KB, 100KB | 1.719 us, 7.778 us, 67.90 us |
| D2/validate_only/10, 100, 1000 | 244.8 us, 2.370 ms, 23.95 ms | D11/4rej-1KB, 10KB, 100KB | 3.896 us, 25.36 us, 450.3 us |
| D2/validate_then_presence/10, 100, 1000 | 596.7 us, 7.383 ms, 125.8 ms | D11/8rej-1KB, 10KB, 100KB | 10.29 us, 67.41 us, 1.222 ms |
| D2/baseline_validate_only/10, 100, 1000 | 714.1 us, 7.989 ms, 84.82 ms | D12/edit_and_reuse/10 | 1.258 ms |
| D2/baseline_validate_then_presence/10, 100, 1000 | 1.566 ms, 9.255 ms, 77.40 ms | D12/edit_and_reuse/300 | 33.58 ms |
| D4/a1_validate | 63.91 ms | D13/suggested_imports/1, 10, 100 | 173.7 us, 316.0 us, 2.198 ms |
| D4/a1_no_positions_control | 49.59 ms | X3/recursive_translation | 13.33 ms |
| D4/schema_free_validate | 4.339 ms | X4/wide_invalid_mapping/1000 | 18.21 ms |
| D4/schema_free_no_positions_control | 358.5 us | | |
| D4/schema_free_baseline_validate | 12.65 ms | | |
| D4/schema_free_baseline_no_positions_control | 9.301 ms | | |
| D5/conditionals/10, 100, 1000 | 43.74 us, 777.1 us, 46.34 ms | | |

These are visibly disturbed by load. `D2/baseline_validate_then_presence/1000`
(77.4 ms) is faster than `D2/baseline_validate_only/1000` (84.8 ms), which
does strictly less work; `D4/a1_validate` (63.9 ms) is more than twice the
identical `A1/corpus_validate/1000-warm` (28.3 ms); and
`D9/start_stack_control/10KB` is slower than `D9/gate_stack/10KB`. The item
tables therefore take their times from the counting binary's same-process
medians, which at least ran each pair back to back. No Criterion baseline was
saved; `pipeline-work-pre` should be saved by the quiet-host pass.

## Reproduce

Each target runs on its own:

```bash
# counts, allocations, peak, and median time for every item and A1/A2
cargo bench -p darkmatter --features work-counters --bench pipeline_work_alloc
# ... or a subset, by substring:
cargo bench -p darkmatter --features work-counters --bench pipeline_work_alloc -- D5 A2/claudine_bundle/implement

# Criterion timings (system allocator, no counters) for the same cases
cargo bench -p darkmatter --bench pipeline_work
# ... or one case:
cargo bench -p darkmatter --bench pipeline_work -- pipeline_work/D2/
# the baseline the spec names, for phase 2's comparison
cargo bench -p darkmatter --bench pipeline_work -- --save-baseline pipeline-work-pre

# A3: the DMLS edit loop, by phase, a3x1 and a3x10, warm and cold
cargo bench -p dmls --bench pipeline_work_a3
```

Record `sysctl -n vm.loadavg` before and after each run.

## Verification

- `cargo check -p darkmatter -p darkmatter-cli -p dmls` (no features)
  passes: every call site compiles to nothing without `work-counters`.
- `just lint` in `darkmatter/` passes, and
  `cargo clippy -p darkmatter --features work-counters --benches -- -D warnings`
  covers the feature-gated alloc bench.
- `just test` in `darkmatter/`: 12,533 passed, 10 skipped. Two earlier runs
  at load averages of 50-143 each failed one timing-sensitive test
  (`compose_lifecycle::ctrl_c_runs_cancelled_then_finalize_and_exits_130`,
  `registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`);
  both passed on every isolated rerun and in the full third run.

## Files

Counter call sites (production code; each a `crate::note_work!` that
compiles to nothing without `work-counters`):

- `darkmatter/lib/src/markdown/schemas/validate.rs` (`build_validator_with`: `ValidatorCompile`)
- `darkmatter/lib/src/markdown/schemas/rewrite.rs` (`rewrite_property_union`: `RewriteArmCompile`)
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs` (`check_alphanumeric_values`: `AlphanumericCheckCompile`)
- `darkmatter/lib/src/markdown/schemas/mod.rs` (`positions_for`: `PositionMapBuild`)
- `darkmatter/lib/src/lifecycle/source_map.rs` (`from_frontmatter_named`: `LifecycleSourceMapBuild`)
- `darkmatter/lib/src/lifecycle/parse.rs` (`parse_lifecycle_config_with`: `LifecycleConfigParse`)
- `darkmatter/lib/src/markdown/compose/expression/scope/mod.rs` (`shared_ancestor`, `steps_since_shared`, `link_addresses`: `ScopeJoinAncestorVisit`)
- `darkmatter/lib/src/markdown/schemas/bound_operands.rs` (`Walker::walk`, `marks`, `collect_marks`: `OperandSiteComparison`, `OperandMarksComputation`, `OperandMarksNodeVisit`)
- `darkmatter/lib/src/markdown/schemas/resolve.rs` (`load_named_types` miss: `NamedTypeTableParse`)
- `darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs` (`candidate`: `CandidateFailureBuild`)
- `darkmatter/lib/src/markdown/coercion/engine.rs` (`fail`: `CoercionFailureBuild`)
- `darkmatter/lib/src/markdown/compose/expression/diagnostics.rs` (`one_based_position`: `PositionPrefixScanBytes`)
- `darkmatter/lib/src/markdown/schemas/completion.rs` (`alternative_accepts_in`: `AlternativeMatcherPrepare`)
- `darkmatter/lib/src/markdown/schemas/json_shape.rs` (`Translator::reference`: `RecursiveTargetTranslate`)
- `darkmatter/lib/src/markdown/schemas/positions.rs` (`locate_in`: `PositionKeyComparison`)

Tooling:

- `darkmatter/lib/src/work_counters.rs`: the new `Work` variants and `WorkSnapshot::plus`
- `darkmatter/lib/benches/pipeline_work_support/mod.rs`, `pipeline_work.rs`, `pipeline_work_alloc.rs`
- `darkmatter/lib/benches/fixtures/pipeline-work/` (frozen A3 document, lifecycle artifact, README)
- `darkmatter/dmls/benches/pipeline_work_a3.rs`
- `darkmatter/lib/Cargo.toml`, `darkmatter/dmls/Cargo.toml` (`[[bench]]` entries)
