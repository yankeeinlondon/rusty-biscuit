---
title: "Spike S1 — DMLS index store measurements (macOS)"
created: 2026-10-17
status: complete
scope: macOS part only (the Windows sharing-violation check is not covered here)
decides: Open Questions 1–4 of 2026-10-17-dmls-index-store
---

# Spike S1: DMLS index store measurements (macOS)

## Purpose

Measure the proposed SQLite fact store against this repository so that the
open questions of `2026-10-17-dmls-index-store` can be answered with numbers:
the numeric memory and latency targets (Q1), batch size and busy timeout (Q2),
whether the joined-path candidate key is needed (Q3), and whether a full
heading scan is fast enough for workspace symbols (Q4). The Windows half of S1
(deleting or sweeping an open store) is not part of this report.

## Method

**Code.** This is a throwaway binary outside the tracked tree:
`/private/tmp/claude-501/-Volumes-coding-wt-rusty-biscuit-feat-dmls-indexing/b7f35e9a-0c36-4dee-9d50-bb27db8217de/scratchpad/spike-s1/proj/src/main.rs`
(`Cargo.toml` next to it). It has path dependencies on `darkmatter/dmls`,
`darkmatter/lib`, and `biscuit-hash`, plus `rusqlite` 0.31 (`bundled`). It
uses the real dmls pieces: `discover_workspace`, `index_document`,
`WorkspaceGraph::build_with_roots` (wiki roots = `[repo root]`),
`RepositoryContexts` built from `RequestSnapshot::from_process`, and
`dmls::bench_index`. It is built in `--release`.

**Store layout.** The layout is the R1 facts plus candidate keys:
`files(path UNIQUE, hash)`, `headings`, `links` (raw target and span; the
`LinkTarget` is recomputed with `classify_link_target` on load), `wiki`,
`transclusions`, and `cand(file_id, form, path_key, name_key, suffix_key)`,
with an index on each key column and on `file_id`. The connection settings are
WAL, `synchronous=NORMAL`, and `busy_timeout=5000`. Each batch is one
`BEGIN IMMEDIATE` transaction. Each row is upserted by path and skipped when
the stored hash matches.

**Measurement.** Wall time comes from `Instant`. Peak RSS comes from
`getrusage` `ru_maxrss`. Rust heap figures come from a counting
`#[global_allocator]` that tracks live and peak bytes. That allocator does
**not** see C allocations (onig, pcre2, sqlite) or memory the system allocator
keeps after a free.

**Corpus.** The worktree at `6e9c25b85` plus the uncommitted spec edits: 5,680
Markdown files, 88.2 MiB.

**Host.** Apple M4 Max, 16 cores, 128 GiB. **The host was heavily loaded the
whole time**: load average 30–190, mostly from another worktree's parallel
`claudine` test run. Every CPU-bound absolute figure is inflated. The
reference `dmls --bench-index` took 42.7 s wall here, against 4.05 s in the
logging-improvements spec. Compare figures within a table. Ratios (small graph
vs. full graph, single vs. pair) are more reliable than absolute values.

**Cold disk cache.** Not measured. It needs the user to run `! sudo purge`
between runs, and the spike does not run `sudo`. All revalidation figures are
**warm**.

## Results

### 1. Store fill

The table covers all 5,680 files, one process, with parsing done between
batches in the same way a real fill interleaves them.

| Batch | Transactions | Time inside transactions (all rows) | Lock wait | DB after close | WAL before checkpoint | Process total (parse-dominated) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 50 | 114 | 339 ms | 2 ms | 12.2 MiB | 4.3 MiB | 40.7 s |
| 200 | 29 | 267 ms | 1 ms | 12.2 MiB | 4.4 MiB | 39.8 s |
| 1000 | 6 | 316 ms | 0 ms | 12.2 MiB | 5.5 MiB | 41.8 s |

When everything is parsed first and the write phase is timed alone, it takes
225, 373, and 205 ms for batches of 50, 200, and 1000. A repeat batch-200 fill
spent 599 ms in transactions. **Writing the whole store costs 0.2–0.6 s.
Parsing costs 36–47 s on this loaded host** (about 4 s unloaded, per the
reference). Batch size does not measurably change the write cost.

After the suffix column was added, the final layout holds 12.4 MiB in these
rows: 5,680 files, 75,401 headings, 21,894 links, 6 wiki links, 212
transclusions, and 7,890 candidate keys. By `dbstat`, the largest objects are
`headings` (6.4 MB), `links` (1.5 MB), `headings_file` (0.9 MB), and the
`files` path index (0.9 MB).

A cold fill (batch 200) peaked at **552 MiB RSS** with a Rust heap peak of 84
MiB. A warm re-run of the same fill (every row skipped, but every file still
re-parsed) peaked at 528 MiB RSS.

### 2. Memory split of the full index

| Quantity | Value | Method |
| --- | ---: | --- |
| Peak RSS, `bench_index`-equivalent (16 parse threads) | 591–680 MiB | `ru_maxrss` |
| `BTreeMap<PathBuf, DocumentIndex>` for all files, retained | **24.8 MiB** | live heap delta |
| R1 facts subset only (what `MemoryFacts` would hold) | **16.1 MiB** | live heap delta |
| `WorkspaceGraph` + context cache, retained | **71.7 MiB** | live heap delta |
| Transient heap peak during parsing (16 / 4 / 1 threads) | +155 / +108 / +94 MiB | counting allocator |
| Transient heap peak during graph build | +107 MiB | counting allocator |
| Peak RSS (16 / 4 / 1 threads) | 680 / 644 / 573 MiB | `ru_maxrss` |

In a sequential per-file pass, RSS climbs in steps at large files and never
comes back down. One example is
`tree-hugger/features/_completed/2026-10-08-scoped-references/log.md` (795
KiB): parsing it takes 2.9 s, its Rust heap peak is 78 MiB, and it adds 158
MiB of RSS. RSS reaches about 420 MiB while live Rust heap stays near 25 MiB.

**Finding.** Of the 680 MB, only about **97 MiB is retained data** (25 MiB of
`DocumentIndex` plus 72 MiB of graph). The other ~85% is the high-water mark
of **parse transients**. That is memory the allocator keeps, or C allocations
the counter cannot see; this spike does not attribute it further. It is not a
"`DocumentIndex` vs. graph" split.

### 3. Candidate fan-out

Four key variants were tested:

- **(a) name:** every link yields only the file-name key.
- **(b) joined:** the spec's R2 as written. Relative links (including bare
  `a/b.md`) yield only the joined path key. Wiki links and `/`, `~`, `^`, `@`,
  `$`, scheme, and directory forms yield the file-name key.
- **(c) joined + bare-name:** like (b), but a *bare* relative form (no `./` or
  `../` prefix) also yields the file-name key.
- **(d) joined + suffix:** like (b), but a bare form also yields its
  normalized text as a suffix key, matched against every path suffix of the
  target.

The true referrers come from the full graph: distinct source documents with a
`References` or `Transcludes` edge into the target's root or headings. These
are the only edge kinds that the references, rename, wiki, and DSL providers
query.

**Over-approximation check.** Over all 3,017 true (referrer, target) pairs
across 1,483 referenced targets:

| Variant | Missed true referrers | Candidates per referenced target: mean / p50 / p95 / max |
| --- | ---: | --- |
| true referrers | — | 2.0 / 1 / 5 / 26 |
| (a) name | **0** | 51.0 / 4 / 332 / 332 |
| (b) joined (R2 as written) | **18** (0.6%) | 3.1 / 2 / 11 / 29 |
| (c) joined + bare-name | **0** | 22.4 / 3 / 139 / 146 |
| (d) joined + suffix | **0** | 21.1 / 3 / 136 / 143 |

All 18 misses under (b) are bare paths written relative to the
**repository root** rather than the document's directory. For example,
`[codex](claudine/docs/research/non-interactive-sessions/codex.md)` in
`claudine/docs/topics/stream-parsing.md`, and `::file
claudine/docs/shared-resources/agent-skills/cli-report.md`. A bare reference
plans more than one candidate (the document directory, then the repository
root, and so on; see `FileReference::candidate_plan_with_order`). Which
candidates it plans depends on context, so a stored joined key cannot cover
them.

**Representative targets.** "true" is the number of true referrers; the other
columns count candidates. All four variants were measured against the store
in SQL; the timings column shows the SQL time for (a) / (c).

| Target | true | (a) name | (b) joined | (c) joined + bare-name | (d) joined + suffix | SQL ms (a) / (c) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| most-referenced `spec.md`: `fixes/_completed/2026-09-11-cicd-cleanup/spec.md` | 12 | 332 | 15 | 138 | 135 | 0.58 / 0.16 |
| most-referenced `README.md`: `messenger/cli/README.md` | 8 | 91 | 18 | 37 | 23 | 0.20 / 0.06 |
| most-referenced `SKILL.md`: `.claude/skills/rust-testing/SKILL.md` | 23 | 70 | 23 | 46 | 46 | 0.09 / 0.05 |
| root `README.md` | 0 | 91 | 10 | 29 | 11 | 0.08 / 0.04 |
| mid-traffic: `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/results.md` | 4 | 25 | 4 | 10 | 10 | 0.03 / 0.02 |

Every candidate query takes under 1.5 ms.

Also counted: 18 `uses_file`/`uses_schema` edges resolve to another indexed
`.md` file. No current provider queries those edge kinds, so leaving them out
of the store (R1) loses nothing today.

### 4. Small graph vs. full graph

Each run is a fresh process, so its peak RSS is attributable. Each run opens
the store, queries candidates, loads facts from SQLite, and calls
`build_with_roots` over the candidates plus the target with a new
`RepositoryContexts`. "Rebuild" is a second build in the same process, where
the context cache is warm, as it would be in a long-running server. The
process RSS before any work was 10.8 MiB.

| Target | Variant | Candidates | Facts load | First build | Rebuild (warm ctx) | Total | Peak RSS | Heap peak | Referrers equal full graph |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `spec.md` (cicd-cleanup) | (a) | 333 | 6.6 ms | 153 ms | 51 ms | 160 ms | 38.7 MiB | 14.3 MiB | yes (12) |
| | (c) | 139 | 3.1 ms | 99 ms | 26 ms | 103 ms | 29.3 MiB | 5.6 MiB | yes |
| `messenger/cli/README.md` | (a) | 92 | 2.4 ms | 101 ms | 33 ms | 104 ms | 27.9 MiB | 4.8 MiB | yes (8) |
| | (c) | 38 | 1.2 ms | 71 ms | 12 ms | 73 ms | 24.2 MiB | 2.6 MiB | yes |
| `rust-testing/SKILL.md` | (a) | 71 | 1.4 ms | 75 ms | 15 ms | 77 ms | 24.4 MiB | 2.6 MiB | yes (23) |
| | (c) | 47 | 0.8 ms | 62 ms | 5 ms | 63 ms | 22.6 MiB | 1.9 MiB | yes |
| root `README.md` | (a) | 91 | 2.6 ms | 130 ms | 32 ms | 134 ms | 27.8 MiB | 4.7 MiB | yes (0) |
| | (c) | 30 | 1.3 ms | 99 ms | 12 ms | 101 ms | 23.8 MiB | 2.5 MiB | yes |
| mid-traffic `results.md` | (a) | 26 | 1.3 ms | 94 ms | 8 ms | 96 ms | 23.1 MiB | 2.3 MiB | yes (4) |
| | (c) | 11 | 0.7 ms | 96 ms | 9 ms | 97 ms | 22.1 MiB | 2.0 MiB | yes |

The full graph build on the same host took 494–1,893 ms (`bench_index`
reports 608 ms for build plus 20 ms for the reverse index). It keeps 72 MiB
resident and needs every file parsed first. A first build in a process has a
floor of about 50–90 ms whatever the candidate count, most of it
context-cache construction. With warm contexts, a small build takes 5–51 ms.

Variant (b) also produced the correct answer for these five targets, but
only by luck: none of them is the target of a missed bare referrer (see 3).

### 5. Revalidation (warm only)

| Pass over the 5,680 listed files (88.2 MiB) | Time |
| --- | ---: |
| discovery walk (the R5 file list; exists before first need) | 186–297 ms |
| load every stored `(path, hash)` | 1.2–1.6 ms (13 ms once) |
| read + xxHash, sequential | 108–151 ms |
| read + xxHash, 16 threads | **48–53 ms** |
| metadata only (size + mtime), sequential | 8–11 ms |
| metadata only, 16 threads | **4–5 ms** |

The revalidation process peaked at 21 MiB RSS with a 2.6 MiB heap peak. Warm,
the full hash pass costs about 45 ms more than a metadata-only walk. Cold:
**not measured; it needs the user to run `! sudo purge` between runs.**

### 6. Heading scan (workspace symbols)

There are 75,401 stored headings. Loading all `(path, level, title, line)`
rows in one query takes **28 ms**. A subsequence scan, including the
document-title pass, takes 10.6–13.2 ms per non-empty query and 0.5 ms for the
empty query:

| Query | Matches |
| --- | ---: |
| `index store` | 50 |
| `rdme` | 6,091 |
| `cand` | 11,729 |
| `zzqxj` | 0 |
| empty | 80,411 |

### 7. Two processes cold-filling one store

Two processes (A and B) were started together on the same new store file.

**Interleaved runs (realistic):** each process parses a batch, then writes it.

| Batch | A: total, written, max lock wait | B: total, written, max lock wait | Single-process total (other run) |
| ---: | --- | --- | ---: |
| 50 | 44.6 s, 5,480, 0.1 ms | 44.8 s, 200, 3.4 ms | 40.7 s |
| 200 | 45.0 s, 0, 0.1 ms | 44.6 s, 5,680, 0.0 ms | 39.8 s |
| 1000 | 36.7 s, 1,680, 62.1 ms | 36.7 s, 4,000, 10.2 ms | 41.8 s |

**Write-only stress:** both processes parse everything first, then start
writing at the same instant through a file barrier.

| Batch | Single write phase | Pair write phase A / B | Max lock wait A / B |
| ---: | ---: | ---: | ---: |
| 50 | 225 ms | 212 / 216 ms | 3.8 / 119.3 ms |
| 200 | 373 ms | 285 / 250 ms | 217.7 / 42.2 ms |
| 1000 | 205 ms | 212 / 193 ms | 196.0 / 0.0 ms |

- **`BEGIN IMMEDIATE` never hit `SQLITE_BUSY`** (busy timeout 5000 ms) in any
  run. The worst wait was 218 ms, on a host at load average about 150.
- Total time for a pair is the same as for one process. Parsing dominates, and
  the idempotent skip lets the later writer skip the rows the other process
  already wrote.
- **Unexpected failure:** `PRAGMA journal_mode=WAL` on a **new** file returned
  `SQLITE_BUSY` ("database is locked") at once, without consulting the busy
  timeout. This happened to the second process in 2 of the first 3 concurrent
  runs and crashed it. After a retry loop (sleep 5 ms) was added, one later
  run needed 1 retry (25 ms). This is a known SQLite behavior: changing the
  journal mode needs an exclusive lock, and that lock does not go through the
  busy handler.

## What it decides

| Open question | Recommendation | Evidence |
| --- | --- | --- |
| **Q3: joined-path key** | **Keep it, but change R2.** An explicitly relative form (`./`, `../`) yields only the joined key. A **bare** form (`a/b.md`, `spec.md`) yields the joined key **and** the file-name key. Wiki and root-prefixed forms stay as written. Do not add a suffix key: it is barely narrower than the file-name key (p95 136 vs. 139). | R2 as written misses 18 of 3,017 true referrers (0.6%), all bare root-relative paths. That breaks the "never miss" rule. Name-only is safe but reaches 332 candidates for every `spec.md` target (p95 332). Variant (c) is safe (0 misses) with p95 139 and max 146. |
| **Q2: batch size** | **Keep 200.** Batch size is not a performance lever. | All rows are written in 0.2–0.6 s against seconds of parsing. 50, 200, and 1000 cost the same. Larger batches only lengthen the other process's longest wait (up to 196 ms at 1000 on a loaded host), and 200 gives finer progress steps. |
| **Q2: busy timeout** | **Keep 5000 ms**, which leaves more than 20 times the worst wait. **Add:** retry `journal_mode=WAL` on open (bounded, about 2 s, short sleeps), or create a new store under a temporary name and rename it into place. Treat a busy WAL switch as "busy", never as corruption. | No `BEGIN IMMEDIATE` waited longer than 218 ms. The WAL pragma failed immediately in 2 of 3 first concurrent creations. |
| **Q4: heading scan** | **A full scan is enough;** no narrowing step. Optionally keep the loaded rows for the session so that later queries cost only the scan. | 75,401 headings: 28 ms to load and 11–13 ms per query. The R3 estimate of "milliseconds" holds, at tens of milliseconds. |
| **Q1: numeric targets** | **Warm restart, first references request** on this repository: ≤ **500 ms** from request to answer, not counting the file-list walk the dependency already does, and ≤ **1 s** including it. **Peak memory** added by that request: ≤ **64 MiB**. **Cold fill (no store):** set no bound below today's full build (about 4 s wall and 550–680 MiB peak RSS). The store does not make a first fill cheaper, and acceptance 11 should say it applies to warm restarts only. | Under heavy load the warm path measured about 50 ms (revalidate hashes) + 2 ms (load hashes) + under 1 ms (candidates) + 60–130 ms (first small build, variant c) ≈ 0.25 s, plus 190–300 ms for discovery. Peak process RSS was 22–29 MiB with a 2–6 MiB heap; revalidation peaked at 21 MiB. A cold fill peaked at 552 MiB because it parses everything. |
| **Decision C over A** (spec "Decisions" table) | **C still wins, but the stated reason is wrong. Return to the author.** | Option A would keep about **97 MiB** resident (72 MiB graph + 25 MiB `DocumentIndex`, or 16 MiB of R1 facts), not 680 MB. The 680 MB is the high-water mark of parsing, which neither A nor C avoids on a cold fill and both avoid on a warm restart. C beats A by needing about 5 MiB and about 0.1 s per request, where A needs a full build of 0.5–1.9 s and about 72 MiB resident. The table should give that reason. |
| R4 "about 360 ms" for read + hash | Correct the figure to about 50 ms in parallel (110–150 ms sequential), warm. Discovery (190–300 ms) is the file list, not part of revalidation. | Section 5 |
| No-mtime decision; R6 pre-rename pass in fallback mode | Both are confirmed as cheap when warm. A full re-hash costs about 45 ms more than a metadata walk. | Section 5. Cold is not measured. |

## Caveats

- **Loaded host.** Load average was 30–190 throughout. CPU-bound absolute
  times (parse, fill, graph builds) are inflated, roughly ten times for parsing
  against the reference 4.05 s. Ratios and I/O-bound figures are more
  trustworthy. Re-measure on a quiet host before writing the targets into
  acceptance criteria.
- **Warm cache only.** No cold-disk figures; they need `! sudo purge` between
  runs, run by the user.
- **RSS attribution is approximate.** The counting allocator sees only the
  Rust heap. The gap of about 450–580 MiB between the Rust heap peak and RSS
  is parse-time high-water, either allocator retention or C allocations (onig
  and pcre2 are in the dependency tree). The spike does not separate the two.
- **Heuristic candidate keys.** Bare means "no `./` or `../` prefix and no
  special prefix". `/`, `~`, `^`, `@`, `$`, scheme, and directory forms
  fell back to the file-name key. Percent-decoding and `<…>` stripping are
  minimal. "0 misses" holds for this corpus and for the edge kinds the
  providers query today. It is not a proof; the R2 over-approximation test is.
- **Small-graph runs use stored facts only.** They exclude open buffers, an
  external wiki universe (the `build_with_roots` change is not built), and
  rename edit construction. The referrer comparison counts distinct source
  documents, not individual edits.
- **Parse cost.** Parse cost is skewed toward a few files: the slowest single
  file takes 2.9 s, and the 50 slowest files account for 16% of parse time.
  Nothing in the store changes that cost for a cold fill.
- **Concurrent edits.** Revalidation reported 1–4 files changed because
  other sessions edited files during the spike. This does not affect the
  timings.
- **Moving source tree.** The spike compiled against the working tree of
  `darkmatter/dmls`. During the spike another session modified that tree
  (logging-improvements work in `graph/substrate.rs`, `workspace/startup.rs`,
  `lib.rs`, and others). Each figure reflects the tree as it was at that
  build, not exactly commit `6e9c25b85`.
- **Row equality.** Concurrent fills were not checked for identical final rows
  (acceptance 9). Only the absence of errors and the written and skipped
  counts were checked.
