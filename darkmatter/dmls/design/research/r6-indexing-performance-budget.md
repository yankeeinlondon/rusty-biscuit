---
prompt: |-
  DMLS (the Darkmatter Language Server) will hold its workspace graph fully
  in memory and rebuild it at startup (AD-2 in
  @darkmatter/features/2026-07-04-dmls/design.md), with a content-hash-keyed
  warm-start cache as a designed escape hatch. We need evidence-based
  latency budgets before implementation.

  1. Survey published performance characteristics and user-reported numbers
     for comparable Markdown LSPs on large workspaces: marksman,
     markdown-oxide, IWE/iwes, and vscode-markdown-languageservice. What
     startup indexing times and memory footprints do they exhibit on
     1k / 5k / 20k-file vaults? What indexing strategies do they use
     (eager, lazy, persisted)?
  2. Define a benchmark corpus plan for DMLS: what mix of file counts,
     file sizes, link densities, frontmatter complexity, and Darkmatter
     directive usage would be representative? Include this monorepo's own
     Markdown population (measure it: file count and total size of *.md
     under the repo) as one tier, and a synthetic 5k-file vault as another.
  3. Propose concrete budgets with rationale: cold-start full index,
     single-document re-index after a keystroke, diagnostics debounce
     window, hover/completion/definition response from warm indexes, and
     memory ceiling per 1k files.
  4. Recommend measurement infrastructure: criterion benches vs. tracing
     spans vs. a `dmls --bench-index <dir>` mode, and which to build first.
  5. Identify the specific signals that should trigger building the
     warm-start cache (AD-2 variant C) or incremental text sync, so the
     escape hatches have objective activation criteria.

  Deliverables: comparable-server survey table, corpus definition, budget
  table, and measurement-infrastructure recommendation.
last_updated: 2026-07-06
hash: cae8f4eaf3611af8-b22a936fd3f6044c
---
## DMLS Latency Budgets and Benchmark Plan

DMLS v1 should keep AD-2's full in-memory workspace graph and startup rebuild, but it needs explicit performance gates before that becomes architecture by inertia. The public evidence is thin for exact Markdown LSP memory footprints, so the plan below separates measured data, source-level strategy evidence, and proposed DMLS budgets.

### Comparable Server Survey

| Server                            | Published / user-reported startup indexing time                                                                                                                                                                                        | Published / user-reported memory footprint                                                                                                   | Indexing strategy                                                                                                                                                                                                                                                                 | Notes for DMLS                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
|-----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------:|---------------------------------------------------------------------------------------------------------------------------------------------:|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| IWE / `iwes`                      | Published Criterion load benchmark on Apple M3 Pro: 5k docs in **128 ms**, 10k in **285 ms**, 20k in **631 ms**. No 1k row published; a linear interpolation from the 5k row is about **25 ms**, but that is not a measured number.    | No explicit memory footprint found.                                                                                                          | Eager load at startup for LSP/MCP, then reused in memory. The CLI rebuilds per invocation. IWE documents this as directory walk, read, parse, and in-memory graph build; LSP pays this once at startup.                                                                           | Best hard evidence found. Its synthetic docs are small, about 700 bytes each, so DMLS must not copy these numbers directly. Source: [IWE benchmark](https://iwe.md/docs/architecture/benchmark/) reports load/query split and LSP startup amortization, corpus sizes, and load numbers.                                                                                                                                                                                                                                                                                                 |
| Marksman                          | No 1k / 5k / 20k startup benchmark found. Its checked-in microbench covers reference resolution over 10 / 50 / 250 synthetic docs, not startup.                                                                                        | One user issue reports a second Marksman process “keep[s] on consuming more and more RAM,” but the report has no machine-readable MB values. | Eager workspace load from disk into in-memory `Folder` / `Doc` / lookup / connection structures; incremental document updates are supported. Source inspection shows recursive file enumeration, Markdown filtering, parse/index per doc, and `withDoc` incremental update paths. | Treat as architecture evidence, not budget evidence. It validates the in-memory graph shape, but does not give usable large-vault latency or memory budgets. Sources: [features](https://github.com/artempyanykh/marksman/blob/main/docs/features.md), [RAM issue #406](https://github.com/artempyanykh/marksman/issues/406), [Folder.fs](https://github.com/artempyanykh/marksman/blob/main/Marksman/Folder.fs), [Doc.fs](https://github.com/artempyanykh/marksman/blob/main/Marksman/Doc.fs), [Benchmarks](https://github.com/artempyanykh/marksman/blob/main/Benchmarks/Program.fs). |
| markdown-oxide                    | No public 1k / 5k / 20k startup benchmark found.                                                                                                                                                                                       | No explicit memory footprint found.                                                                                                          | Eager in-memory vault model. Source describes `Vault` as the in-memory representation of Obsidian vault files, with update paths for changed files.                                                                                                                               | Good comparable feature set for PKM links, tags, backlinks, headings, and Obsidian syntax. Public docs emphasize performance but do not publish large-vault numbers. Sources: [repo](https://github.com/Feel-ix-343/markdown-oxide), [docs](https://oxide.md/), [vault source](https://github.com/Feel-ix-343/markdown-oxide/blob/main/src/vault/mod.rs).                                                                                                                                                                                                                               |
| `vscode-markdown-languageservice` | No 1k / 5k / 20k startup indexing table found. VS Code's migration blog cites “a few hundred milliseconds” of extension-host blocking on a large Markdown workspace as a motivation for moving Markdown tooling to a separate process. | No explicit memory footprint found.                                                                                                          | Mixed lazy/stateful caches. Document diagnostics can be stateless, but the service exposes a stateful diagnostics manager for repeated calls and workspace changes. It also creates workspace link and table-of-contents caches.                                                  | Important because it proves VS Code considered a few hundred milliseconds of blocking too high in the editor host. DMLS should keep expensive startup/index work off the protocol loop and expose progress. Sources: [VS Code blog](https://code.visualstudio.com/blogs/2022/08/16/markdown-language-server), [language service source](https://github.com/microsoft/vscode-markdown-languageservice/blob/main/src/index.ts), [TOC cache source](https://github.com/microsoft/vscode-markdown-languageservice/blob/main/src/tableOfContents.ts).                                        |

Evidence quality:

| Claim                                                                                                                        | Confidence                                     |
|------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------|
| IWE can eagerly load a small synthetic 20k-file Markdown graph in under 1s on an Apple M3 Pro.                               | High, published benchmark.                     |
| Marksman and markdown-oxide use in-memory workspace/vault representations with eager startup load and per-file update paths. | Medium-high, source-level evidence.            |
| Comparable Markdown LSP memory per 1k files is publicly known.                                                               | Low; no reliable published MB/file data found. |
| TypeScript Markdown tooling considers a few hundred ms of editor-host blocking unacceptable.                                 | High, explicit VS Code blog statement.         |

### DMLS Benchmark Corpus Plan

DMLS needs both real-corpus and synthetic-corpus tiers. Synthetic-only would understate Darkmatter costs because Darkmatter documents contain frontmatter, directive markers, links, and repo-style docs with longer bodies than IWE's 700-byte synthetic files.

Measured local tier, excluding `target/`:

| Corpus                            | Files | Total size                  | Avg file     | p50         | p90          | p95          | p99          | Max           | Notes                                                                                                                                                                                                                                                                 |
|-----------------------------------|------:|----------------------------:|-------------:|------------:|-------------:|-------------:|-------------:|--------------:|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `rusty-biscuit` worktree Markdown | 4,052 | 40,791,085 bytes, 38.90 MiB | 10,082 bytes | 5,584 bytes | 24,542 bytes | 34,343 bytes | 64,928 bytes | 140,251 bytes | 1,704 files begin with YAML frontmatter. Approximate marker counts from source text: 390 files with Darkmatter-like directive markers, 2,367 directive occurrences, 91 files with wiki-links, 234 wiki-links, 1,841 files with Markdown links, 21,676 Markdown links. |

Benchmark tiers:

| Tier                 | Purpose                       | Shape                                                                                                                                                          |
|----------------------|-------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `tiny-100`           | Fast development sanity check | 100 files, 1-5 KiB each, sparse links, 20% frontmatter, no directives.                                                                                         |
| `small-1k`           | Normal docs workspace         | 1,000 files, 5-10 MiB total, 2-6 links/file, 40% frontmatter, 5% directives.                                                                                   |
| `repo-rusty-biscuit` | Real DMLS/Darkmatter workload | The measured monorepo Markdown population above. Run from the repo root and from `darkmatter/` to catch root-discovery differences.                            |
| `vault-5k`           | Required synthetic vault tier | 5,000 files, deterministic seed, about 50 MiB total, avg 8-12 KiB/file, 3-10 headings/file, 5-12 Markdown links/file, 1-4 wiki-links/file, 40-50% frontmatter. |
| `dense-5k`           | Link graph stress             | 5,000 files, same byte size as `vault-5k`, but 25-50 links/file, hub files with 500-1,000 inbound links, repeated heading slugs, ambiguous file stems.         |
| `large-20k`          | AD-2 scale gate               | 20,000 files, 150-250 MiB total, mixed sizes with p99 >= 100 KiB, sparse-to-medium links.                                                                      |
| `pathological-1k`    | Tail-latency guard            | 1,000 files with 20 very large files, malformed frontmatter, duplicate headings, broken links, deeply nested lists, CRLF, Unicode and astral-plane text.       |

Synthetic file mix for `vault-5k`:

| Dimension             | Distribution                                                                                                                                                    |
|-----------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| File size             | 60% 4-8 KiB, 30% 8-24 KiB, 8% 24-64 KiB, 2% 64-150 KiB.                                                                                                         |
| Headings              | 1 H1, 2-8 H2/H3 headings, duplicate heading slug in 10% of files.                                                                                               |
| Links                 | 5-12 Markdown links/file, 1-4 wiki-links/file, 5% broken links, 5% fragment links, 2% ambiguous file-stem links.                                                |
| Frontmatter           | 45% of files. Of those: 60% flat scalar maps, 25% nested maps/lists, 10% schema/style fields, 5% malformed or incomplete YAML.                                  |
| Darkmatter directives | 10% of files. Include `::file`, `::code`, `::toc-linking`, `::file-links`, `::block`, `::shell` as passive/static syntax only. Do not execute shell directives. |
| External file refs    | 5% of files include images or local file references for path-resolution diagnostics.                                                                            |
| Newlines / encoding   | 80% LF, 15% CRLF, 5% mixed Unicode-heavy content.                                                                                                               |

### Proposed DMLS Budgets

These are startup and hot-path budgets for a release build on a modern developer laptop. They should be measured on macOS first, then repeated on Linux and Windows CI or a dedicated benchmark host before claiming cross-platform parity.

| Operation                                                    | Budget                                                                                                       | Rationale                                                                                                                                                                                         |
|--------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------:|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Cold-start full index, `small-1k`                            | p50 \<= 500 ms, p95 \<= 1.0 s                                                                                  | Slower than IWE's extrapolated small-doc load, leaving room for Darkmatter frontmatter, directives, source maps, and richer graph edges.                                                          |
| Cold-start full index, `repo-rusty-biscuit`                  | p50 \<= 2.0 s, p95 \<= 4.0 s                                                                                   | Current repo is 4,052 Markdown files / 38.90 MiB, materially heavier than IWE's 5k synthetic corpus.                                                                                              |
| Cold-start full index, `vault-5k`                            | p50 \<= 2.5 s, p95 \<= 5.0 s                                                                                   | User-visible but acceptable with work-done progress. If exceeded, warm-start cache becomes a serious candidate.                                                                                   |
| Cold-start full index, `large-20k`                           | p50 \<= 10 s, p95 \<= 20 s                                                                                     | This is the upper bound for AD-2 without persisted cache. It is intentionally much looser than IWE's 631 ms because DMLS documents are larger and semantically richer.                            |
| First progress notification                                  | \<= 250 ms                                                                                                    | Large workspaces must not appear hung.                                                                                                                                                            |
| Single-document re-index after keystroke, avg file \<= 10 KiB | p95 \<= 25 ms                                                                                                 | Keeps the worker path under an interactive frame budget and leaves room for debounce/scheduling.                                                                                                  |
| Single-document re-index, p99 file \<= 75 KiB                 | p95 \<= 75 ms                                                                                                 | Matches this repo's file-size tail.                                                                                                                                                               |
| Single-document re-index, pathological 150-250 KiB file      | p95 \<= 150 ms                                                                                                | This may miss frame budget but should remain below human-visible stall thresholds when run off the protocol loop.                                                                                 |
| Diagnostics debounce window                                  | Default 200 ms; adaptive 150-300 ms; heavy workspace diagnostics >= 500 ms                                   | VS Code's “few hundred milliseconds” blocking anecdote argues against synchronous diagnostic churn. 200 ms balances typing cadence with fast feedback.                                            |
| Hover from warm index                                        | p95 \<= 10 ms, p99 \<= 25 ms                                                                                   | Must feel instantaneous and run safely on the LSP loop only if bounded.                                                                                                                           |
| Completion from warm index                                   | p95 \<= 15 ms, p99 \<= 40 ms                                                                                   | Completion tolerates slightly more work for ranking/filtering, but must not scan all document text.                                                                                               |
| Definition from warm index                                   | p95 \<= 15 ms, p99 \<= 40 ms                                                                                   | Should be graph/index lookup plus source-map projection.                                                                                                                                          |
| References/backlinks from warm index                         | p95 \<= 50 ms on 20k sparse/medium graph; p99 \<= 100 ms on dense graph                                        | Workspace fan-out is acceptable, but must remain under perceptual latency.                                                                                                                        |
| Memory ceiling                                               | Target \<= 20 MiB per 1k repo-like files; hard ceiling \<= 32 MiB per 1k files, excluding editor/client memory | This allows raw text/source maps plus graph overhead while keeping 20k files below about 640 MiB hard ceiling. Because comparable public memory data is missing, DMLS must measure this directly. |

### Measurement Infrastructure

Build measurement in this order:

| Priority | Infrastructure                      | Build first?                        | Why                                                                                                                                                                                                                                                  |
|---------:|-------------------------------------|-------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1        | `dmls --bench-index <dir> --json`   | Yes                                 | This directly measures the product path: discovery, file reads, parse, graph build, source maps, diagnostics precompute if enabled, peak RSS, and final graph counts. It works on real repos and synthetic corpora and can be run in CI or manually. |
| 2        | `tracing` spans around index stages | Yes, same first slice               | The bench mode should emit structured timings from spans: `discover`, `read`, `hash`, `parse_markdown`, `frontmatter`, `directives`, `graph_build`, `reverse_index`, `diagnostics`, `snapshot_swap`. This makes regressions actionable.              |
| 3        | Synthetic corpus generator          | Yes, paired with bench mode         | Deterministic corpora make budget failures reproducible. Reuse the IWE-style seeded generator pattern, but generate Darkmatter-specific frontmatter and directives.                                                                                  |
| 4        | Criterion microbenches              | After bench mode                    | Useful for parser/source-map/link-extraction internals, but insufficient for AD-2 because startup cost is system-shaped: filesystem walk, allocation, graph construction, and RSS.                                                                   |
| 5        | LSP replay harness                  | After basic provider implementation | Measures hover/completion/definition latency through JSON-RPC and cancellation behavior. Not needed before the indexer exists.                                                                                                                       |

`dmls --bench-index` output should include at least:

```json
{
  "root": "...",
  "files": 5000,
  "bytes": 52428800,
  "elapsed_ms": 2410,
  "peak_rss_bytes": 123456789,
  "stages": {
    "discover_ms": 120,
    "read_ms": 380,
    "hash_ms": 55,
    "parse_ms": 1100,
    "frontmatter_ms": 250,
    "directives_ms": 90,
    "graph_ms": 310,
    "diagnostics_ms": 105
  },
  "graph": {
    "nodes": 123456,
    "edges": 456789,
    "documents": 5000,
    "links": 60000,
    "frontmatter_blocks": 2250,
    "directives": 800
  }
}
```

Use `tracing` for all timings even when not benchmarking. The same spans should back `--bench-index`, debug logs, and future startup progress messages.

### Escape-Hatch Activation Criteria

Build the warm-start cache, AD-2 variant C, when any two of these hold on release builds:

| Signal                                     | Threshold                                                                                                             |
|--------------------------------------------|----------------------------------------------------------------------------------------------------------------------:|
| `vault-5k` cold-start full index           | p95 > 5.0 s on reference hardware                                                                                     |
| `repo-rusty-biscuit` cold-start full index | p95 > 4.0 s                                                                                                           |
| `large-20k` cold-start full index          | p95 > 20.0 s                                                                                                          |
| Startup user experience                    | first useful hover/definition unavailable for > 5 s on 5k files despite progress reporting                            |
| File I/O dominates startup                 | read/discover/hash stages are > 50% of cold-start time and unchanged-file reuse would skip most of it                 |
| Repeated sessions                          | warm-start prototype would skip >= 80% of files by content hash in normal restart workflows                           |
| Windows/macOS/Linux variance               | one supported OS is > 2x slower than the fastest OS for the same corpus after obvious filesystem issues are addressed |

Build incremental text sync when either of these holds:

| Signal                | Threshold                                                                                                                                                            |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------:|
| Full-sync update cost | p95 single-document re-index after keystroke > 25 ms for avg files or > 75 ms for p99 repo files, and profiling shows text replacement/source-map rebuild dominates. |
| Large open files      | Files >= 250 KiB produce p95 update latency > 150 ms under full sync in normal typing traces.                                                                        |
| LSP payload overhead  | Full document sync sends > 1 MiB/s sustained during typing in large files or causes visible client/server CPU churn.                                                 |
| Diagnostics staleness | Debounced diagnostics routinely publish > 750 ms after last edit because full reparse work backs up the worker queue.                                                |

Do not build the warm-start cache just because it is architecturally available. Do not build incremental text sync until full-sync measurements show that text transport or full text replacement is the bottleneck. The first milestone is therefore `dmls --bench-index`, tracing spans, and deterministic corpora; cache and incremental sync decisions should be made from those numbers.
