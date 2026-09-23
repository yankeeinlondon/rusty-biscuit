# Finding 35.3 — `Arc<str>` fetched response bodies: retained cost model

Rebuilt for review-2's finding *"Several benchmark dispositions still lack
retained raw samples"*. The original record
(`run-20260716T160000/f35_3-copy-cost-model.txt`) carried medians and prose only;
its harness (`f35_3_profile` in `remote_fetch.rs`) was **deleted after capture**,
so nothing could be recomputed. This run retains the per-observation vectors.

**The disposition is unchanged (NO-WIN -> REVERTED), but three of the original's
numbers do not reproduce. See *Corrections* below.**

## Run record

| Field | Value |
|---|---|
| Run id | `run-20260716T182500-f35-3` |
| Baseline commit | `a80e032c3` (nothing shipped for 35.3 — see *Why there is no candidate in the tree*) |
| Candidate commit | `a80e032c3` (same; this is a cost model, not an A/B of shipped code) |
| Harness | `darkmatter/lib/benches/phase11_evidence.rs` -> `bench_f35_3_copy_model` — **RETAINED** |
| Command | `cargo bench -p darkmatter --bench phase11_evidence` |
| Profile | release (Criterion default bench profile) |
| Host | macOS Darwin 25.5.0, arm64, Apple M4 Max |
| Host load (1m) | 14.95 before -> 16.92 after (shared host; two concurrent agent builds) |
| TTY mode | piped (non-interactive) |
| Warm-up | 3.0 s per case (Criterion default) |
| Samples | 200 per case |
| Statistic | median + bootstrap 95 % CI of the mean (via `recompute.ts`) |
| Tools | rustc 1.96.0, cargo 1.96.0, criterion 0.5.1, bun 1.3.3 |
| Fixture | `benchmarks/fixtures/remote_heavy.md` — 79 028 bytes, xxHash64 `0dc952a78995bde7` (manifest identity, unchanged) |

### Predeclared threshold (carried verbatim from the original record)

> Target operation: resolving a remote `::file` / `::code` / expression URL
> through `RemoteFetchRuntime` (register + fetch + point-of-use `get_content`).
> Minimum repeatable win: >= 5% on the target operation.
> Maximum permitted control regression: 0% (no path may get slower).

### Recomputation

```
bun recompute.ts raw/f35-residuals/run-20260716T182500-f35-3
```

Every statistic below is reproduced by that command from the four committed
`*-sample.json` vectors beside this file.

## Why there is no candidate in the tree

35.3 was implemented and then **reverted**, so there is no shipped candidate to
call. What the disposition actually rests on is a *cost model*: whether swapping
`FetchSlot::Ready`'s `String` for an `Arc<str>` can pay for itself at all. The
model is pure `std` — a body `String`, an `Arc<str>` conversion, and the two read
shapes — so it is reproducible without re-applying the reverted change. The
revert is left in place, as required.

The mechanical fact the model turns on is unchanged and was re-read from the
code: `FetchSlot::Ready` is populated by **moving** `RemoteFetchOutcome.body` (a
`String` from `String::from_utf8`). `Arc<str>` cannot reuse that allocation (its
refcount header is inline), so storing one adds a **full body copy per URL** that
the pre-change code never paid, while the public owned `get_content` facade must
still hand out a `String`.

## Results (medians, recomputed from the retained vectors)

| Case | Median | 95 % CI (mean) | Role |
|---|---|---|---|
| `store_string_to_arc` | **791.37 ns** | [799.31, 808.02] ns | NEW cost, once per URL |
| `read_string_clone` | **795.39 ns** | [797.33, 804.08] ns | baseline read, per consumer |
| `read_arc_to_string` | **798.75 ns** | [808.05, 815.05] ns | candidate read, owned facade |
| `read_arc_clone` | **3.67 ns** | [3.68, 3.71] ns | candidate read, `&str`-only consumer |

### Net per typical document (one URL, one consumer)

| Call site | Baseline | Candidate | Net |
|---|---|---|---|
| `::file` / preflight / expression (needs owned `String`) | 795.39 ns | 791.37 + 798.75 = 1590.12 ns | **+794.73 ns worse** |
| `::code` (needs `&str` only) | 795.39 ns | 791.37 + 3.67 = 795.04 ns | **-0.35 ns — break-even** |

Call-site audit (re-read from the code, unchanged from the original record): of
the five consumers, exactly one (`engine.rs` `::code` ->
`wrap_in_code_block(&body, ..)`) is `&str`-only; the other four
(`engine.rs` `::file` -> `Markdown::try_from_content(body)`, `preflight/collect.rs`,
and `resolve_ctx.rs` x2) need an owned `String` and therefore regress.

## Corrections to the original record (numbers that do NOT reproduce)

1. **Store is not 1.75x a clone; it is the same cost.** The original reported
   store 1.167 us vs `String::clone` 0.667 us. Re-measured, they are
   indistinguishable (791.37 ns vs 795.39 ns). This is the mechanically expected
   result — both are one allocation plus one 79 KB memcpy — so the original's
   1.75x ratio was host noise, not a property of the code.
2. **`::code` is break-even at one consumer, not `+0.50 us` worse.** It follows
   from (1): break-even is at `store / clone` = 791.37 / 795.39 ~= **1.0**
   consumer, not the ~1.75 the original derived. A document transcluding one
   remote URL as code **twice** would make `::code` a small win, not merely
   break even.
3. **`Arc::clone` is 3.67 ns, not "0.000 us".** The original's zero was a
   reporting-resolution artifact.

The original's headline conclusion — *net pessimization* — therefore survives
only for the four owned-facade call sites, and is **withdrawn for `::code`**,
which is break-even.

## Disposition: NO-WIN -> REVERTED (unchanged), on corrected reasoning

The revert still stands, but the honest reason is narrower than the original's:

- **Four of five call sites regress unconditionally** by ~795 ns per document
  (the owned facade must copy out of the `Arc`, and the store already copied in).
  The predeclared budget is **0 % control regression** — this alone rejects it.
- **The win is unreachable by construction.** For the copy budget to reach the
  predeclared 5 % floor of the target operation, the *entire* register + fetch +
  `get_content` operation would have to cost less than
  `795.39 ns / 0.05` ~= **15.9 us**. The original measured that operation at
  534.5 us on **loopback** — the most favorable case that exists, and ~34x above
  that break-even; a real network fetch is 10-100x slower still. The no-win
  conclusion is robust for any fetch cost above ~16 us, so it does not depend on
  re-measuring the fetch, which this run did **not** re-measure.

Per the Benchmark & Evidence Contract ("Findings with no repeatable improvement
outside noise close through a recorded no-win disposition and removal of
unnecessary code"), the `Arc<str>` slot, the crate-internal shared accessor, and
the `::code` call-site change remain reverted. `remote_fetch.rs` and the `::code`
path are byte-identical to their pre-Phase-10 state.

## Not re-measured in this run

- **Result 2's loopback fetch cost (534.5 us).** Not reproduced here; the
  no-win argument above is deliberately restated so it does not depend on it.
  Treat the original's `0.125 %` share as unverified — the corrected copy cost
  would make it ~=`0.149 %` on the same 534.5 us figure.

## Cross-platform

OS-identical, and moot — no code shipped. The inspected path contains no `cfg`,
no filesystem branch, and no OS-divergent runtime behavior: it is slot storage
plus an in-memory copy.
