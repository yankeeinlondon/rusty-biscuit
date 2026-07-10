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
reviewed and **accepted** (preferred variants) with Ken on 2026-07-06. The
R-1 … R-8 research (see the final section) has since completed; every AD
survived, and each carries a **Research outcome** note where the evidence
sharpened it. Research-driven detail has been folded into the sections
below.

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
- **Source maps** wrap rust-analyzer's **`line-index` crate** (R-2
  recommendation: maintained, SIMD-optimized, UTF-8/16/32 projection,
  rejects mid-codepoint offsets) rather than a bespoke line index. The
  DMLS wrapper owns: the negotiated `PositionEncoding`, CRLF/lone-CR
  policy (Markdown needs lone-CR handling `line-index` alone doesn't
  give), strict `byte_to_lsp`/`lsp_to_byte` conversion APIs, version
  stamping, and sub-document projection. Raw `line-index` types never
  leak past `source_map`.
- **Position encoding negotiation** (R-2/R-7): support UTF-8 and UTF-16
  (UTF-32 only if free). VS Code and Zed advertise UTF-16 only — always
  answer UTF-16 there; Helix (`utf-8` first) and modern Neovim get UTF-8.
  Never emit positions (including diagnostics) before `initialize`
  completes — Helix cannot decode them until encoding is known.
- **Sub-document projection** uses a region descriptor keeping three
  ranges distinct — opening delimiter line, content, closing delimiter
  line — and composes byte offsets (never line/column addition, which
  breaks on CRLF and astral characters). YAML-parser diagnostics arrive
  frontmatter-content-relative; project by adding the content base byte
  offset, then convert through the host source map. The same shape serves
  future code-fence virtual documents (info-string diagnostics belong to
  the opening fence line, body diagnostics to the body region).
- The R-2 test checklist (negotiation cases, CRLF/lone-CR/mixed endings,
  BOM, BMP multibyte, astral, combining sequences, delimiter off-by-ones,
  empty frontmatter, missing closing fence) is encoded as the source-map
  unit-test matrix before any provider ships.

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
parser; R-3 validated it hands-on). Construction path:

1. A new library API (`extract_frontmatter_block`, R-4 item 2) yields the
   YAML text plus delimiter/body spans and `yaml_base_line`.
2. `rlsp-yaml-parser` in lossless mode parses the YAML.
3. The first `Document<Span>` lowers into an arena of `FrontmatterNode`s:
   mapping entries in declaration order with key span, value span, node
   kind, scalar style, comments, anchor/tag/alias metadata, plus a
   duplicate-key side table keyed by normalized path.
4. Lookup indexes: dotted path (`style.page.margin`), JSON Pointer
   (`/style/page/margin`), and authored-entry lookup for duplicates —
   including nearest-ancestor fallback for paths that don't exist (R-5
   mapping rules).
5. Frontmatter-relative spans project through the source map into document
   LSP ranges.

Malformed-YAML policy (R-3 finding): `rlsp-yaml-parser` returns positioned
syntax errors but **no partial AST on hard errors**. Mid-keystroke, a
dangling `key:` still parses (empty scalar, zero-width value span) — the
truly broken states are indent/flow errors. v1 policy: keep the **previous
good `FrontmatterAst`** for the document (clearly version-stamped stale)
for completion/hover continuity, and emit parse diagnostics from the
current failed parse. A local recovery wrapper is the first escalation if
mid-error completion inside broken YAML becomes a hard requirement; the
bespoke saphyr-parser loader remains the crate-replacement fallback.

Semantic validation stays in `darkmatter::markdown::schemas`: DMLS assembles
the *effective schema* (base + extensions + document `$schema`, compose
precedence), runs library validation, and maps resulting error paths onto
`FrontmatterAst` ranges. Merge/alias-derived values (Claudine's
`&lifecycle-event` anchors) range at their **authored** source; semantic
expansion gets an explicit `SemanticOrigin` layer later if needed rather
than mutating the source AST.

### DSL overlay

Directive, interpolation, and condition parsing must be span-faithful to the
compose pipeline. **AD-6** decides where that parsing lives (library vs
DMLS-local). Whatever the home, the overlay only does *static* analysis:
token-resolution ladders are explained, never executed.

### Required `darkmatter` library additions (R-4 + R-5)

The R-4 inventory found real parsers for every DSL surface, but almost none
expose spans publicly (`LanguageGrammar` and `markdown::reference` are the
two that are ready). The v1 library workstream, in priority order — all
**additive**, leaving compose's semantic structs and pipeline untouched:

1. Shared span vocabulary: `SourceSpan`, `Spanned<T>`, line/column helpers.
2. `extract_frontmatter_block` — public delimiter/body/base-line extraction
   (today only `Frontmatter::raw_source()` is public, and it re-joins lines).
3. Span-aware expression parsing: `parse_spanned` / `parse_condition_spanned`
   / `lex_spanned` alongside the existing `Expr` (which today loses all token
   positions; `Parser.position` is a token count, not a byte offset).
4. Span-aware, public directive scanning across `::file`, `::code`,
   `::toc-linking`, `::file-links`, `::shell`, page blocks, shell blocks
   (currently crate-private), and disclosure — unified
   `scan_darkmatter_directives` / `scan_darkmatter_blocks` returning keyword,
   target, and option spans.
5. Public read-only parse products for frontmatter `$()` (AST shape, branch
   and suffix spans — no execution surface).
6. Nested YAML position mapping wired into schema and style diagnostics
   (today `build_position_map` covers top-level keys only, and
   `StyleWarning::source_span` is always `None`), plus the richer
   `ValidationProblem` shape from R-5: a real problem-code enum
   (missing/type/constraint/unknown-key/file-reference instead of the
   current 3-way collapse into `Invalid`), `offending_property` for
   unknown-key cases, schema-origin metadata surviving baseline merge, and
   compose-parity pending-`$(...)` results exposed as data.
7. Export the heading-slug generator (`generate_heading_slug`) and a
   span-carrying `extract_headings` — DMLS must not reimplement the slug
   authority.
8. `LanguageGrammar`: no change needed.

Riskiest items are directive-scanner unification and anything touching the
expression AST or frontmatter normalization — those land as parallel APIs
with compose-parity goldens, never as rewrites.

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
  contract; renaming codes later breaks user suppression config). The
  frontmatter/schema taxonomy is fixed by R-5 (`dm.frontmatter.yaml_parse`,
  `dm.schema.invalid_schema_shape`, `dm.schema.prepare`,
  `dm.schema.type_mismatch`, `dm.schema.constraint`,
  `dm.schema.missing_required`, `dm.schema.unknown_key`,
  `dm.schema.deprecated_key`, `dm.schema.invalid_file_reference`,
  `dm.schema.pending_shell_value`) and the wiki-link taxonomy by R-8
  (`wiki.unresolved-target`, `wiki.ambiguous-target`,
  `wiki.ambiguous-after-rename`, `wiki.heading-missing-in-target`,
  `wiki.empty-target`, `wiki.empty-heading`, `wiki.unsupported-syntax`,
  `wiki.portability-collision`, `wiki.invalid-percent-escape`).
- Ranging convention (matches yaml-language-server/taplo practice): the
  validator identifies the semantic failing node, but final ranges always
  come from the concrete syntax tree (`FrontmatterAst`), never from message
  parsing or rendered line maps. Missing-required-key diagnostics range the
  **parent mapping** (a real visible range, not zero-width — many clients
  render zero-width poorly); unknown-key diagnostics range the offending
  key node; `relatedInformation` points at the schema source.
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

## Performance Budgets (R-6)

Measured context: this monorepo holds 4,052 Markdown files / 38.9 MiB
(avg ~10 KiB, p99 ~65 KiB) — materially heavier per file than IWE's
published benchmark corpus (~700-byte synthetic docs, 5k in 128 ms on an
M3 Pro), so IWE's numbers set the *shape*, not the budget. Key v1 budgets
(release build, reference laptop; full table in R-6):

| Operation | Budget |
|---|---|
| Cold-start index, 1k files | p50 ≤ 500 ms, p95 ≤ 1 s |
| Cold-start index, this repo (~4k files) | p50 ≤ 2 s, p95 ≤ 4 s |
| Cold-start index, 5k vault / 20k vault | p50 ≤ 2.5 s / ≤ 10 s |
| First progress notification | ≤ 250 ms |
| Single-document re-index (≤10 KiB) | p95 ≤ 25 ms |
| Diagnostics debounce | 200 ms default (adaptive 150–300) |
| Hover / completion / definition (warm) | p95 ≤ 10 / 15 / 15 ms |
| References on 20k graph | p95 ≤ 50 ms |
| Memory | target ≤ 20 MiB per 1k repo-like files |

Measurement infrastructure ships **with the first indexer slice**, in this
order: (1) `dmls --bench-index <dir> --json` reporting per-stage timings and
peak RSS; (2) `tracing` spans (`discover`, `read`, `hash`, `parse_markdown`,
`frontmatter`, `directives`, `graph_build`, `reverse_index`, `diagnostics`,
`snapshot_swap`) backing both the bench mode and debug logs; (3) a
deterministic synthetic-corpus generator (tiers: `tiny-100`, `small-1k`,
`repo-rusty-biscuit`, `vault-5k`, `dense-5k`, `large-20k`,
`pathological-1k`). Criterion microbenches and an LSP replay harness come
after.

The AD-2 escape hatches now have objective activation criteria (R-6):
build the warm-start cache only when at least two hold — e.g. `vault-5k`
cold start p95 > 5 s, repo cold start p95 > 4 s, I/O stages > 50% of cold
start, warm-start would skip ≥ 80% of files by hash, or one OS is > 2×
slower than the fastest. Build incremental text sync only when profiling
shows full-text replacement/source-map rebuild is the bottleneck (re-index
p95 over budget, or ≥ 250 KiB files stalling under typing). Neither is
built merely because it is architecturally available.

## Configuration

- Sources and precedence: editor `workspace/configuration` >
  repo config file > built-in defaults. Frontmatter never configures the
  server (it configures the document).
- Repo config file: **`.dmls.toml`** at the workspace root (decided; R-7's
  editor snippets already use it as a root marker alongside `.git`).
- Reload on `workspace/didChangeConfiguration` and config-file watch events;
  schema-extension changes invalidate only frontmatter/schema indexes.

## Client Capability Profiles

A `ClientProfile` computed at initialize gates optional behavior; providers
consult the profile instead of sniffing capabilities ad hoc. R-7 pinned the
current per-editor reality (VS Code 1.127, Zed 1.9.0, Neovim 0.10–0.12,
Helix 25.07); the load-bearing gates:

- **Position encoding:** UTF-16 for VS Code and Zed (they advertise nothing
  else); UTF-8 for Helix and modern Neovim. Never publish positions before
  `initialize` completes.
- **Resource operations:** all four support create/rename/delete in
  `WorkspaceEdit`; `ChangeAnnotation`s only for VS Code and Neovim — for
  Zed/Helix, code-action titles must carry the explanation.
- **File operations:** `workspace/willRenameFiles` works on VS Code, Zed,
  and Helix — **not** Neovim (capability off). Keep rename available as a
  code action/command for clients without file-operation notifications.
- **File watching:** client-side watchers everywhere, but Neovim's watching
  is limited/disabled on Linux — DMLS keeps a server-side rescan/hash
  fallback.
- **Resolve support:** `codeAction/resolve` and `completionItem/resolve`
  everywhere, but Zed deliberately does not resolve `textEdit` on
  completion items — never defer `textEdit` to resolve.
- **Structure features:** Helix has no LSP folding, selection ranges, or
  linked editing (disable there); Neovim folding is `lineFoldingOnly`.
- **Hover content:** text-first Markdown everywhere; images/HTML tables are
  a VS Code-only enhancement, never load-bearing.
- **Named quirks:** `helix_one_char_selection_is_empty` (the IWES quirk) as
  an isolated profile flag, applied only to selection-sensitive code
  actions.

R-7 also ships working registration snippets (VS Code extension, Zed
extension, Neovim 0.10/0.11+, Helix `languages.toml`) that seed the v1
editor-setup docs, using `.dmls.toml` / `.git` as root markers.

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

**Research outcome (R-1, high confidence): B confirmed; the `liwe` attempt
is a documented negative check only.** Inspection of upstream `0.7.0`
found the decisive mismatches: `RefIndex` holds exactly two edge maps
(inclusion, reference) with no extension point; `GraphNode` is a closed
enum with no payload slot for frontmatter key paths or directive
arguments; parser/config APIs are IWE-product-shaped. Wrapping would force
a parallel overlay index — the two-graph state AD-1 exists to avoid. Port
list: the `Router` loop shape, the `utf16_to_byte_offset`/
`byte_to_utf16_offset` helpers *as tests*, the multibyte/astral test cases
from `iwes` (definition, hover, completion, link), `base_path.rs`'s
Windows/percent-encoding test cases, the `KeyIndex` wiki-basename
algorithm, and the in-memory LSP fixture shape. Estimated DMLS-owned
subset: ~1,400–2,400 LOC plus tests (a literal vendor would start at
4,000–6,000). Licensing: IWE is Apache-2.0 — copied/adapted code keeps
Apache notices and IWE attribution at the module level; concept-only ports
need only a design-note citation.

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

**Research outcome (R-6): A confirmed and now gated by numbers.** IWE
eagerly loads 20k small synthetic docs in 631 ms; Marksman and
markdown-oxide are both eager in-memory models; VS Code considered "a few
hundred milliseconds" of host blocking unacceptable enough to spawn a
separate process. No comparable server publishes usable memory data, so
DMLS measures its own (target ≤ 20 MiB per 1k repo-like files). The escape
hatches got objective activation criteria and the bench harness ships with
the first indexer slice — see Performance Budgets.

**Phase 11 sign-off (2026-07-07): A stands; no cache built.** Release-build
`dmls --bench-index` medians on the dev host (macOS, Apple Silicon): full repo
(3,141 files) ~1.89 s cold (budget p50 ≤ 2 s); `vault-5k` ~0.54 s
(budget p50 ≤ 2.5 s, ~5× under); `dense-5k` ~1.30 s and `pathological-1k`
~0.32 s (stress tiers). Single-document re-index is far inside the 25 ms p95
budget (per-file parse ≈ 0.10 ms even on the dense tier; warm L2 request cycle
is sub-ms). No two AD-2 escape-hatch activation criteria hold, so the v1
in-memory-only model is confirmed and no warm-start on-disk cache is warranted.
Full numbers: `phase11-bench-results.md`.

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

**Research outcome (R-3, hands-on bake-off): B confirmed.**
`rlsp-yaml-parser 0.11.1` (lossless mode) delivered the best geometry fit:
byte spans on every node kind, attached comments preserved, anchors/aliases
kept, no panics on malformed input, ~3.1 ms on a 200-line block (all
candidates are fast enough). Known limits now encoded in the design: no
partial AST on hard syntax errors (see FrontmatterAst malformed-YAML
policy), merge keys parsed as authored entries rather than semantically
replayed (which matches our authored-range diagnostics stance), and a
single-maintainer crate that doesn't accept external contributions — the
facade plus the prototyped ~170-line saphyr-parser fallback (C) is the
insurance. `serde-saphyr` confirmed type-driven only (no arbitrary spanned
tree); it stays a possible validation-side tool.

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

**Research outcome (R-4): C confirmed; the boundary is now concrete.**
Every DSL surface has a real library parser, but only `LanguageGrammar`
and `markdown::reference` are span-ready today; everything else is
classified needs-span-API (several parsers — shell blocks, disclosure,
frontmatter `$()` — are additionally crate-private). The prioritized
additive API list lives in "Required `darkmatter` library additions" and
is a v1 workstream in its own right. The rule held under inventory: no
surface needs a new DMLS-local *grammar*, only public span exposure.

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
earlier `research-areas.md`. **All eight were executed on 2026-07-06 and
their findings are folded into this document and the spec**; the files
below now contain the full research bodies and remain the authoritative
detail behind the summaries above.

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
