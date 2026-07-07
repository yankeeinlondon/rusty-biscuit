---
status: draft
related:
  - ./spec.md
  - ../../dmls/design/design-strategy.md
  - ../../design/caching-strategy.md
---

# DMLS Architecture Design

**Status:** Draft for review. Companion to [spec.md](./spec.md), which defines
scope and the v1 contract. This document defines the internal architecture and
records the key architectural decisions (AD-1 … AD-7) with variants,
trade-offs, and a stated preference for each. All seven decisions were
reviewed and **accepted** (preferred variants) with Ken on 2026-07-06.

## Design Goals

1. **Prove the baseline fast.** Layer 0–2 (Markdown + wiki-links + schema
   frontmatter) must be reachable without inventing new infrastructure — reuse
   IWES's graph model and Darkmatter's semantic APIs.
2. **Be a platform, not a feature pile.** The seams that v1 leaves behind
   (provider registry, graph edge kinds, source maps, extension baselines)
   must be the same seams the full Darkmatter DSL, embedded languages, and
   compose preview plug into later.
3. **Keystroke-latency correctness first, throughput second.** Correct source
   maps and invalidation before persistent caches or incremental sync.
4. **Cross-platform from day one.** macOS, Windows, Linux; no
   platform-conditional feature gaps in core behavior.

## System Overview

```text
+---------------------------------------------------------------+
| dmls binary                                                   |
|   lsp-server stdio loop · lsp-types · capability negotiation  |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
| Router                                                        |
|   dispatch · cancellation · progress · client quirk profiles  |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
| WorkspaceState                                                |
|   open docs (full text + version) · disk snapshot · config    |
|   source maps · diagnostics cache · client profile            |
+---------+----------------------------------+------------------+
          |                                  |
+---------v-----------------+   +------------v------------------+
| Markdown Graph Substrate  |   | Darkmatter Semantic Overlay   |
| (IWES-derived)            |   |  FrontmatterAst + schema      |
|  docs · headings · links  |   |  directives · interpolation   |
|  wiki-links · anchors     |   |  transclusion edges · shell   |
|  backlink reverse index   |   |  policy verdicts · extensions |
+---------+-----------------+   +------------+------------------+
          |                                  |
+---------v----------------------------------v------------------+
| Provider Registry                                             |
|   per-capability ordered chains with deterministic merge      |
+---------------------------------------------------------------+
          |
+---------v-----------------------------------------------------+
| Semantic authorities (external to dmls)                       |
|   darkmatter (schemas, expressions, grammar, cleanup, style)  |
|   biscuit-file (FileReference resolution)                     |
+---------------------------------------------------------------+
```

Two indexes, one graph: the Markdown substrate and the Darkmatter overlay
write typed nodes/edges into a **single** workspace graph. Providers never
consult two graphs; they query one graph with edge-kind filters.

## Crate and Module Layout

```text
darkmatter/dmls/
  Cargo.toml
  src/
    main.rs            # stdio entry, arg parsing, logging setup
    lib.rs
    router.rs          # request/notification dispatch, cancellation
    capabilities.rs    # ServerCapabilities construction, client profiles
    workspace/         # WorkspaceState, document store, disk snapshot, watch
    graph/             # node/edge model, indexes, reverse index, invalidation
    overlay/           # Darkmatter semantic overlay
      frontmatter.rs   #   FrontmatterAst facade
      directives.rs    #   directive/block parsing (source geometry)
      expressions.rs   #   interpolation/condition span mapping
      schema.rs        #   effective-schema assembly, extension baselines
    providers/         # one module per LSP capability
    diagnostics/       # taxonomy, codes, scheduler, publisher
    source_map/        # byte ↔ UTF-16, frontmatter-relative projection
    config/            # DmlsConfig, discovery, reload
  tests/               # L1 unit + L2 in-memory LSP session tests
```

The Zed extension lives in a separate small public repo, `zed-dmls`
(**AD-7**): a `cdylib`-to-WASM crate following the proven IWE pattern that
locates or downloads the native `dmls` binary. It contains no language logic
and does not affect server architecture.

## Runtime Model

See **AD-3**. Proposed shape:

- The `lsp-server` main loop owns protocol I/O and dispatches on one thread.
- Cheap, read-only requests (hover, definition, completion against warm
  indexes) are answered synchronously on the loop thread.
- Indexing and diagnostics run on a small worker pool (crossbeam channels,
  `std::thread`), producing immutable index snapshots the loop thread swaps
  in atomically. No tokio; no async runtime.
- Diagnostics are debounced per document (proposed: ~200 ms after last edit)
  and superseded jobs are cancelled by generation counter.

## Document and Text Model

- **Full document sync** (spec commitment). Open documents are `String`s
  keyed by URI with the client's version number. The client buffer is
  authoritative over disk.
- Every parse product (graph nodes, overlay nodes, diagnostics) is stamped
  with `(uri, version)`; providers reject stale lookups.
- **Source maps** are built per document version: a line index storing byte
  offsets plus per-line UTF-16 unit counts, giving O(log n) byte ↔ LSP
  position conversion. Frontmatter-relative and (future) fence-relative
  ranges project through the same API (`base_line` + offset composition —
  same discipline as `extract_frontmatter_text`).
- CRLF, lone CR, multibyte, and astral-plane characters are covered by a
  dedicated test matrix before any provider ships.

## Workspace Graph

Node kinds (substrate): document, section/heading, paragraph, list/item,
table, code fence, quote, link, link-definition.

Node kinds (overlay): frontmatter block, frontmatter key path, schema
declaration, style declaration, directive, directive argument, interpolation
expression, condition expression, transclusion target, shell directive,
disclosure block, page block, fence info string.

Edge kinds:

| Edge | Sources |
|---|---|
| `references` | Markdown links, reference links, wiki-links, anchors |
| `includes` | IWES structural inclusion links |
| `transcludes` | `::file`, `::code`, `::toc-linking`, `::file-links`, prologue, epilogue |
| `uses_schema` | `$schema`, baseline/extension schemas |
| `uses_file` | frontmatter `file(...)`, images, style assets, directive paths |
| `uses_variable` | interpolation/condition → frontmatter key, `ctx.*`, `env.*` |
| `defines_anchor` | headings, custom IDs |
| `defines_symbol` | headings, frontmatter keys, directives, schema keys |

A single reverse index over all edge kinds powers references, backlinks,
rename impact analysis, and invalidation fan-out.

### Invalidation

- Each indexed file carries an xxHash content hash (`biscuit-hash`;
  Markdown-aware split hashing via Darkmatter where frontmatter/body
  distinction matters). Watch events and `didChange` compare hashes before
  re-indexing.
- Re-index of file F invalidates: F's own diagnostics, plus dependents
  reachable over `transcludes` and `uses_file` edges (compositional), plus
  link-target validity of documents with `references` edges into F (cheap
  revalidation, not full re-index).
- v1 keeps the whole graph in memory and rebuilds the workspace index at
  startup with `window/workDoneProgress` reporting. See **AD-2** for the
  persistence stance and its relationship to the compose caching strategy.

## Darkmatter Semantic Overlay

### FrontmatterAst

A DMLS-owned facade over a position-aware YAML parse (**AD-4** picks the
parser). Contract:

- delimiter range; key-path ranges; scalar/sequence/mapping value ranges
- dotted-path and JSON-Pointer lookup → source range
- frontmatter-relative → document-range projection via the source map
- original text slices for hover/code actions
- graceful degradation on malformed YAML (best-effort partial tree + parse
  diagnostics), because the user is mid-keystroke most of the time

Semantic validation stays in `darkmatter::markdown::schemas`: DMLS assembles
the *effective schema* (base + extensions + document `$schema`, compose
precedence), runs library validation, and maps resulting error paths onto
`FrontmatterAst` ranges.

### DSL overlay

Directive, interpolation, and condition parsing must be span-faithful to the
compose pipeline. **AD-6** decides where that parsing lives (library vs
DMLS-local). Whatever the home, the overlay only does *static* analysis:
token-resolution ladders are explained, never executed.

### Extension baselines

`DmlsConfig` maps extension name → SimplifiedSchema path + activation globs.
At document index time, matching globs contribute baselines merged beneath
the document's `$schema` (identical precedence to `md compose`). Claudine is
purely data: `claudine.yaml` + globs. The seam is the same one future
extensions (or per-project schemas) use.

## Provider Registry

See **AD-5**. Proposed shape: for each capability, a static, ordered `Vec` of
providers (substrate provider first, overlay providers after) with a
capability-specific merge policy:

| Capability | Merge policy |
|---|---|
| completion | union, overlay items ranked first on tie, dedup by label+kind |
| hover | first non-empty wins, overlay may *prepend* to substrate preview |
| definition | union of locations; overlay may claim exclusive when it resolved a Darkmatter-only construct |
| references | union, dedup by location |
| diagnostics | union (namespaced sources prevent collisions) |
| document links | union, dedup by range |
| code actions | union, stable kind ordering |
| folding/symbols | union, containment-sorted |

Provider errors degrade to empty results + a logged warning; a panicking
provider must not take down the request loop (catch-unwind at the chain
boundary).

## Diagnostics Pipeline

- Push diagnostics (v1), namespaced sources: `darkmatter.markdown`,
  `darkmatter.links`, `darkmatter.frontmatter`, `darkmatter.schema`,
  `darkmatter.compose`, `darkmatter.style`, `darkmatter.security`.
- Stable diagnostic codes from day one (they become part of the user-facing
  contract; renaming codes later breaks user suppression config).
- Scheduling tiers:
  1. **Immediate** (on didChange, debounced): syntax, frontmatter parse,
     directive well-formedness — single-document, no graph needed.
  2. **Graph** (after re-index): link/anchor validity, transclusion cycles,
     schema validation, interpolation variable resolution.
  3. **Workspace** (on demand / on save): cross-file sweeps.
- A generation counter per document discards stale publishes.

## Caching and Indexing Strategy

The existing [compose caching strategy](../../design/caching-strategy.md)
(SurrealDB resource graph, PENDING/DONE/DIRTY states, remote TTLs) is designed
for *compose-time* work: LLM calls, image variants, remote fetches. DMLS
deliberately does **not** sit on that store for its hot path:

- LSP queries are keystroke-latency; an embedded DB round-trip per hover is
  the wrong shape. The graph must be native in-memory structures.
- DMLS's passive-analysis safety rule excludes exactly the expensive resource
  kinds (remote, LLM) that the compose cache exists to amortize.

What DMLS *keeps* from that strategy: xxHash content-hash identity,
`(resource_hash, content_hash)` keying for any future persisted artifacts,
and dependency-edge-driven dirty propagation. If post-v1 measurements show
cold-start indexing is too slow on large workspaces, the warm-start cache
(**AD-2**, variant C) reuses those conventions. When compose preview lands
post-v1, *that* command may consult the compose cache — through Darkmatter
library APIs, not directly.

## Configuration

- Sources and precedence: editor `workspace/configuration` >
  repo config file > built-in defaults. Frontmatter never configures the
  server (it configures the document).
- Repo config file: proposed `.dmls.toml` at workspace root (name/location is
  spec Open Question 1; alternatives: `dmls.toml`, a `[dmls]` table in an
  existing Darkmatter config if one emerges).
- Reload on `workspace/didChangeConfiguration` and config-file watch events;
  schema-extension changes invalidate only frontmatter/schema indexes.

## Client Capability Profiles

A `ClientProfile` computed at initialize gates: snippet support,
`codeAction/resolve`, resource operations in `WorkspaceEdit`,
`workDoneProgress`, position-encoding negotiation, and known quirks (keyed by
client name/version, e.g. the Helix selection-range quirk inherited from
IWES). Providers consult the profile instead of sniffing capabilities ad hoc.

## Testing Strategy

- **L1:** source-map matrix; FrontmatterAst range fixtures; graph edge
  extraction per surface; provider merge policies; diagnostic code stability;
  schema-error → range mapping.
- **L2:** in-memory LSP sessions via `lsp-server::Connection::memory()` —
  full initialize → edit → diagnostics → request → shutdown conversations
  against fixture workspaces (plain Markdown, wiki-link vault, Darkmatter
  docs, Claudine prompts). A dedicated L2 test asserts passive requests spawn
  no child processes and open no sockets.
- **Compose-parity goldens:** DSL overlay parses are compared against compose
  pipeline behavior on shared fixtures to catch drift.
- Cross-platform path fixtures (Windows separators/drive letters, case
  collisions, symlinks) run in CI on all three OSes.

---

## Architectural Decisions

Each decision lists variants with trade-offs and a preference. All were
**accepted as preferred** on 2026-07-06.

### AD-1: IWES integration mode — `accepted` (B)

How DMLS acquires the IWES graph substrate and router shape.

**A. Depend on published `liwe` + write our own thin router.**
Use `liwe` as a crates.io dependency for the graph/arena/key-index; write the
`lsp-server` router ourselves modeled on `iwes`.

- ✅ Upstream bug fixes and improvements for free; least code owned.
- ✅ Clean provenance/licensing story.
- ❌ `liwe`'s public API is shaped for `iwes`'s concrete needs; Darkmatter
  overlay nodes/edges may not fit its arena and index types without fighting
  the API.
- ❌ Upstream API churn becomes our churn; we can't add edge kinds upstream
  quickly.

**B. Vendor/adapt a minimal IWES-derived subset (graph concepts + router
shape) into `dmls`, no `liwe` dependency.**
Port the ideas — arena graph, key index, router structure, position fixes —
as DMLS-owned code sized to our needs.

- ✅ Full freedom to make the graph natively multi-edge-kind (the central
  architecture bet) instead of overlaying onto someone else's model.
- ✅ No upstream coupling on the hot path; Darkmatter library remains the only
  hard semantic dependency.
- ❌ We own more code; upstream fixes must be hand-ported.
- ❌ Divergence makes later upstreaming of extension seams harder.

**C. Hybrid: depend on `liwe` for parsing/graph where its API fits, vendor
only the router; decide per-module after a spike.**

- ✅ Defers the irreversible part of the decision until evidence exists.
- ❌ Risk of a long-lived half-and-half state with two graph vocabularies.

**Preference: B**, validated by a short spike that first *attempts* A's
`liwe` dependency. The design-strategy analysis already found `iwes`'s server
concrete rather than extension-oriented, and the single-graph-with-edge-kinds
bet is the architecture's core — owning that data structure is strategic, not
incidental. The spike exists to cheaply confirm `liwe`'s API really doesn't
fit before we commit to porting (research topic R-1).

### AD-2: Index persistence — `accepted` (A, with C as escape hatch)

**A. In-memory only; full workspace re-index at startup.**

- ✅ Simplest correct thing; no schema-migration or staleness class of bugs.
- ✅ Startup index of a few thousand Markdown files is plausibly seconds
  (needs benchmark, R-6); progress-reported.
- ❌ Cold start cost paid every session; large vaults may exceed budget.

**B. Persistent sidecar index (SQLite or SurrealDB per compose caching
strategy).**

- ✅ Warm starts; enables future workspace-scale features (persistent
  backlink DB).
- ❌ Schema migration across dmls versions; stale-index correctness bugs are
  the worst LSP bugs; DB round-trips tempt the hot path.
- ❌ Heavy for v1 with zero measured need.

**C. In-memory hot path + optional warm-start snapshot cache
(content-hash-keyed, xxHash conventions from the compose caching strategy).**

- ✅ Keeps hot path native; cache is advisory (corrupt/missing → rebuild).
- ❌ Still a serialization format to maintain; only worth it if A measures
  slow.

**Preference: A for v1, with C as the designed escape hatch** (the
content-hash identity is built in from the start, so C is additive). B is
rejected for the LSP hot path outright; the compose cache remains a
compose-side concern.

### AD-3: Concurrency model — `accepted` (B)

**A. Single-threaded synchronous loop (IWES-style).**

- ✅ Simplest; IWES proves it works for Markdown-scale workspaces.
- ❌ A slow schema validation or workspace index blocks hover; debounce and
  cancellation get awkward when everything shares one thread.

**B. Main protocol thread + small worker pool for indexing/diagnostics,
immutable snapshot swap (crossbeam channels, no async runtime).**

- ✅ Keeps request handlers simple and synchronous against a warm snapshot;
  isolates slow work; cancellation = generation counters.
- ✅ No tokio dependency; matches `lsp-server`'s blocking design.
- ❌ Snapshot-swap discipline needed (no partial mutation visible to readers).

**C. Full async (tokio + tower-lsp style).**

- ✅ Natural cancellation futures; ecosystem familiarity.
- ❌ Abandons the IWES-compatible `lsp-server` stack the strategy chose;
  async infects every provider signature; no measured need.

**Preference: B.** It is the smallest step past A that removes A's known
failure mode, and it preserves the chosen protocol stack.

### AD-4: Position-aware YAML parser behind `FrontmatterAst` — `accepted` (B, C fallback)

**A. `serde-saphyr`** (saphyr-parser based; `Spanned<T>` fields, snippet
errors, validation hooks).

- ✅ Actively positioned as the serde_yaml successor; strong diagnostics.
- ❌ Type-driven: spans attach to deserialized structs, but schema-driven
  frontmatter is open-shaped — we need spans for *arbitrary* trees, which
  may mean deserializing to a spanned Value-like tree if it offers one.

**B. `rlsp-yaml-parser`** (YAML 1.2 combinator parser, span-as-data on every
node, comment preservation, built for LSPs).

- ✅ Exactly the geometry-first shape `FrontmatterAst` needs; comments kept.
- ❌ Young/low-adoption crate; maintenance risk; combinator parsing slower
  (fine at frontmatter sizes).

**C. Build on `saphyr-parser` events directly** (own tiny spanned-tree
loader, ~300 lines).

- ✅ No adoption risk beyond the well-maintained saphyr core; tree shape is
  exactly ours.
- ❌ We own YAML edge cases (anchors, merge keys, multiline scalars) that
  library authors already handled.

**Preference: B behind the facade, validated by prototype (R-3), with C as
the fallback** if B's maintenance or correctness disappoints. The facade
makes this swappable; the *decision that matters* is that no provider ever
sees the parser type. A is kept for possible use on the *validation* side if
its spanned deserialization proves useful, but it is not the geometry source.

### AD-5: Provider registry shape — `accepted` (A)

**A. Static ordered chains per capability** (plain structs implementing a
per-capability trait, registered in a `Vec` at startup; merge policy per
capability as tabled above).

- ✅ Deterministic, debuggable, zero dynamic machinery; conflict rules are
  explicit code.
- ❌ Adding a provider means touching registry construction (acceptable —
  providers are compiled in anyway).

**B. Dynamic plugin registry** (trait objects + registration API, extensions
register providers at runtime).

- ✅ Maximum extensibility story.
- ❌ Speculative: v1 has exactly two provider families (substrate, overlay);
  Claudine extends via *data* (schemas), not code. Classic Rule-2 violation.

**C. Monolithic handler per capability** (one function that inlines both
substrate and overlay logic).

- ✅ Fewest abstractions today.
- ❌ Substrate/overlay separation is the seam that keeps Layer 0 shippable
  and testable alone; inlining erases it and makes compose-preview-era
  growth painful.

**Preference: A.** It encodes the one separation we know we need (substrate
vs overlay) without building a plugin system nobody asked for.

### AD-6: Home of position-aware Darkmatter DSL parsing — `accepted` (C)

Interpolation, conditions, and directives already have parsers inside
Darkmatter's compose pipeline, but their span/position surfaces may not be
public or complete.

**A. Extend the `darkmatter` library with span-aware public APIs**
(expressions, directives expose typed ASTs with byte ranges; DMLS consumes
them).

- ✅ One grammar, zero drift by construction; compose and LSP literally share
  the parser. Aligns with "library is the semantic authority."
- ✅ Repo is young — refactoring library internals now is cheap (stated
  monorepo priority).
- ❌ Library API surface grows; compose-internal types get publicity
  obligations.

**B. DMLS-local overlay parsers, validated against compose via golden tests.**

- ✅ No library changes; LSP-specific error recovery (mid-keystroke states)
  doesn't contaminate compose parsers.
- ❌ Two grammars that *will* drift; goldens catch drift only where fixtures
  exist. This is the false-diagnostics failure mode research-areas.md warns
  about.

**C. Hybrid: library exposes span-aware parsing for surfaces that already
have real parsers (expressions, schema, directives); DMLS owns only pure
source geometry (frontmatter YAML ranges, block pairing/recovery while
typing).**

- ✅ Grammar authority stays in the library; editor-specific recovery stays
  out of compose's way.
- ❌ The boundary needs discipline to not erode.

**Preference: C**, with the explicit rule: *anything that defines what a
construct means lives in `darkmatter`; anything that only locates text or
tolerates half-typed input may live in DMLS.* R-4 inventories which library
parsers already expose spans and which need API additions.

### AD-7: Zed extension placement — `accepted` (B)

**A. In-monorepo at `darkmatter/dmls/zed/`, workspace-excluded (like
`schematic/schema`), submitted to zed-industries/extensions with a `path`
entry.**

- ✅ One repo to version; extension changes ride server PRs.
- ❌ Registry submission via `path` + submodule of the whole monorepo is
  awkward (the submodule would pull the entire rusty-biscuit repo); license
  file needed at the extension path.

**B. Separate small public repo (`zed-dmls`), the proven IWE pattern.**

- ✅ Matches how every reviewed extension (including IWE's) is published;
  tiny public surface; independent versioning; clean license scope.
- ❌ A second repo to maintain; cross-repo release coordination.

**Preference: B.** The extension is ~200 lines that changes only when release
asset naming changes; registry ergonomics and the monorepo's privacy boundary
outweigh mono-repo convenience. (Development happens against a local dev
extension either way.)

---

## Deferred Design Topics → Research

Authored as inline-compose documents (research prompt in `prompt`
frontmatter) under `darkmatter/dmls/design/research/`, consolidating the
earlier `research-areas.md`. Results feed back into this document and the
spec.

- **[R-1](../../dmls/design/research/r1-iwes-integration-boundary.md)**
  `liwe`/`iwes` API inventory + minimal-subset spike guidance (feeds AD-1).
- **[R-2](../../dmls/design/research/r2-source-maps-position-encoding.md)**
  Position-encoding reality across the four editors; line-index dependency
  recommendation; source-map test matrix.
- **[R-3](../../dmls/design/research/r3-yaml-parser-bakeoff.md)** YAML parser
  hands-on bake-off on real Darkmatter frontmatter fixtures (feeds AD-4).
- **[R-4](../../dmls/design/research/r4-darkmatter-span-surface-inventory.md)**
  Inventory of existing Darkmatter parser span surfaces; library API-addition
  list (feeds AD-6).
- **[R-5](../../dmls/design/research/r5-schema-error-range-mapping.md)**
  SimplifiedSchema validation error shape → diagnostic taxonomy and
  range-mapping rules.
- **[R-6](../../dmls/design/research/r6-indexing-performance-budget.md)**
  Comparable-server performance survey, benchmark corpus, latency budgets
  (feeds AD-2 escape-hatch criteria).
- **[R-7](../../dmls/design/research/r7-editor-capability-matrix.md)** Editor
  capability matrix + per-editor setup snippets (feeds `ClientProfile`).
- **[R-8](../../dmls/design/research/r8-wiki-link-resolution-rules.md)**
  Wiki-link matching, ranking, completion-insertion, and rename-safety rule
  set with cross-platform gotchas.
