---
kind: feature
name: symbol-improvements
date: 2026-10-03
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-03
review_iterations: 0
$schema:
  status: |-
    enum(
        draft-spec,
        finalized-spec,
        planned,
        implemented,
        review-findings,
        human-in-the-loop,
        completed,
        on-hold,
        abandoned
    ) -> an indicator of progress for this specification
  reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-visibility-and-kind
related:
  - 2026-10-03-symbol-filters-and-reports
---

# Fast, deterministic symbol reports on v2 records

## Summary

`hug functions`, `hug types`, `hug symbols` and `hug classes` answer "what is
declared in this code?". They work, but they:

- do much more work than the question needs
- guess labels by re-reading source text instead of using record metadata
- emit JSON far larger than the selection
- render the same declarations through separate code paths

This feature is the **performance and foundation** half of the symbol-report
work. It makes the CLI consume the v2 `SymbolRecord` directly, asks the library
for only the passes and kinds a command needs, and makes output deterministic
and sized to the selection. Richer metadata, new filters and new report formats
build on this foundation in `2026-10-03-symbol-filters-and-reports`.

| Area | Today (2026-10-03) | After this feature |
|---|---|---|
| Performance | Every command runs the full lint and semantic analysis. A warm cache saves only 40%. Files are processed one at a time. | Symbol commands skip diagnostics, a cache hit skips parsing, and files are processed in parallel. |
| Metadata | The CLI guesses labels by re-reading source lines. `source.signature_text` is empty. | Records carry the declaring keyword and a render-ready signature header. |
| Query | Each command filters v1 summaries its own way. | A library `SymbolQuery` selects kinds and required capabilities, with explicit completeness. |
| Reporting | Flat list sorted by name across files. `--json` is 28 MB for 1,236 functions. Cache stats on every warm run. | Versioned JSON sized to the selection, file/source default order, quiet stderr, one renderer. |

Filtering is unchanged except that each command's fixed kind set is pushed
into extraction. `--kind`, `--scope`, visibility, container, test, docs and
signature-shape filters, hiding locals in `hug symbols`, and the outline,
table, JSON Lines, count and stats outputs arrive with
`2026-10-03-symbol-filters-and-reports`.

**Depends on `2026-10-03-visibility-and-kind`.** That fix corrects kind,
visibility, `static` and container data for its accepted extraction set across
all 16 languages, and introduces `source.declaration_text`. It retains some
grammar limits and defers some constructs; this feature operates on that
accepted extraction set and does not widen it.

## Baseline

Draft-author measurements from 2026-10-03 on an Apple M4 Max (16 cores),
release build, run in `tree-hugger/` (66 files):

| Command | Result |
|---|---|
| `hug functions --no-cache` | 1.07 s |
| `hug imports --no-cache` | 1.06 s |
| `hug lint --no-cache` | 1.07 s |
| `hug functions`, cache warm | 0.64 s |
| `hug --json functions` | 28.7 MB: 1,236 selected functions, plus all 8,585 full v2 records |
| `hug --json symbols` | 36.8 MB |
| `hug symbols` | 8,593 lines; about 6,000 are `let`, `parameter` or `variable` |

`functions`, `imports` and `lint` cost the same because they do the same work;
see “Diagnostics run only when the command shows them.”

## Background: what one `hug functions` run does today

```mermaid
flowchart TD
    F[collect files] --> L{{"for each file, sequentially"}}
    L --> P["TreeFile::with_language<br/>read + parse (always)"]
    P --> K{cache key}
    K -->|hit| H["hydrate_diagnostics<br/>full lint + semantic run"]
    K -->|miss| A["symbol_index_v2"]
    A --> A1["symbol_records → locals query #1"]
    A --> A2["exported_symbols → locals query #2"]
    A --> A3["try_diagnostics → lint, semantic,<br/>locals query #3, references"]
    A --> A4["bind: name lookup, O(refs × symbols)"]
    H --> S["summarize_file: convert all records to v1, clone"]
    A --> S
    S --> R["render: re-read each source file<br/>to guess the label"]
```

Every box under "for each file" runs for every symbol command, whether or not
the command shows diagnostics, exports or references.

The implementation contracts used in this review are:

- In the tree-hugger package, [analyze_file](../../lib/src/analysis/mod.rs)
  builds the per-file index and fills relations. Its pass dependencies must
  remain explicit when diagnostics become optional.
- In the tree-hugger package, [IdentityFacet, SymbolRef and ProvenanceFacet](../../lib/src/shared/schema_v2/facets.rs)
  hold names, relation targets and extraction metadata.
- In the tree-hugger package, [PassFingerprint](../../lib/src/cache/mod.rs)
  describes analysis inputs but does not currently include the source hash or
  requested passes. The file cache key must include those separately.
- In the tree-hugger-cli package, [PreludeFilter and prelude resolution](../../cli/src/prelude.rs)
  include public re-export aliases and environment-provided names. Moving to
  records must preserve that behavior.

These links point to the current package source and documentation. The
[symbol schema contract](../../docs/symbol-schema-design.md) requires a major
version change for renamed fields or changed semantics.

### Intentional compatibility changes

The smaller JSON shape, file/source default sort order, container-field rename,
and removal of timestamps are intentional changes. Update the schema
documentation and release notes with before/after examples. Keep the existing
`SymbolRecord` model and Rust type names, but advance the serialized
record/index schema from 3.0 to 4.0 when the breaking field changes land; 3.0
is taken by `2026-10-03-visibility-and-kind`. The CLI report envelope carries
its own `report_schema_version: 1` and `schema_version: {major: 4, minor: 0}`
for its embedded records. Later additive record changes, including those in
`2026-10-03-symbol-filters-and-reports`, follow the existing minor-version rule.

The container-field rename moves the current container text from
`identity.module_path` into `identity.container_name`, alongside the fix's
`container_kind`. It lands here, without populating `qualified_name`, so every
breaking record change shares the 4.0 bump.

Invalidate incompatible cache entries; do not migrate stale records or silently
label the new shape as 3.0. Keep the v1 library API and its adapters working
with the renamed container field. No compatibility flag or deprecation period
is required for this early-stage repository.

---

Section numbers keep their values from before this spec was split on
2026-10-03, so gaps (B2–B7, C, D2–D6, D9) belong to
`2026-10-03-symbol-filters-and-reports` or an unscheduled feature.

## A. Performance

### A1. Diagnostics run only when the command shows them

The tree-hugger package's [parse_pass](../../lib/src/analysis/mod.rs)
currently runs diagnostics even for symbol listings. Split that work into
`AnalysisPass::Diagnostics`; ordinary symbol queries do not request it.

Preserve `TreeFile::symbol_index_v2()` as the complete-index convenience API,
including diagnostics. The CLI uses the new query API for selective work.
This avoids silently changing the existing library method's contract while
still removing unnecessary work from listings. `lint` requests diagnostics,
including its existing syntax, experimental-semantics and external-adapter
behavior. A complete cached symbol result requires no parsing or analysis;
a partial cache hit may run the missing passes.

Pass planning removes duplicates and orders prerequisites before consumers:
parse before bind, bind before semantic analysis, and parse before docs.
Diagnostics request their own dependencies without making every listing run
semantic lint. `completed_passes` records only work actually performed;
omitting diagnostics is not evidence that a file is clean.

### A2. A complete cache hit doesn't parse

1. Read the bytes once and hash them through `biscuit-hash`.
2. Look up the cache using the file identity, content hash and analysis inputs.
3. Parse those same bytes only when the requested result is missing.

The library exposes `SourceFile::read(path)` and `.parse()`; `SourceFile`
owns the bytes, detected or explicitly forced language, and source identity.
Its `.query(&query)` uses the same planning path as queries on an already
parsed `TreeFile`. Never reread the file between hashing and parsing: a file
edited in that interval must not cache new records under the old hash.
Existing constructors remain convenience wrappers. Reading, decoding and
language errors retain the existing error contract.

### A3. The locals query runs once per file

`symbol_records()` and `exported_symbols()` each call `symbol_nodes()`, and the
semantic lint calls it again. Within a per-file analysis, capture definitions
once and derive exports, locals and records from that result. Separate
reference or comment queries remain allowed when requested; share definition
captures with native semantic diagnostics rather than re-extracting them.

### A4. Metadata is extracted after duplicates are removed

`symbol_nodes` runs `extract_signature`, `extract_type_metadata` and
`extract_doc_comment` for every capture, then throws duplicates away. The
change removes duplicates on the captured name span and declaration identity
first, choosing the same refined kind regardless of capture order, then
extracts metadata once per surviving symbol. Distinct declarations sharing a
span or name, such as a C typedef and its struct tag, must remain distinct;
regression fixtures establish which captures describe one declaration.

### A5. Command kind sets are pushed into extraction

Each declaration command has a fixed kind set:

| Command | Kinds |
|---|---|
| `functions` | Function, Method, Constructor |
| `types` | Type, Class, Interface, Enum, Trait, TypeAlias |
| `classes` | Class and Type as containers, plus the members it displays today |
| `symbols` | all kinds |

When the command's kind set excludes a kind, the library skips metadata
extraction, and when safe the reference-binding work, for that kind. Keep
lightweight identity, span and container data for excluded declarations: they
can still shadow a selected symbol or own it. Selecting only methods must not
change reference resolution or lose their containers. The library takes this
as the `SymbolQuery` described under “A `SymbolQuery` API.” A user-supplied
`--kind` that narrows these sets arrives with
`2026-10-03-symbol-filters-and-reports` and uses the same mechanism.

### A6. Files are analyzed in parallel

`rayon` is already a dependency (`god_files` uses it). Per-file analysis
becomes `par_iter`, and results are collected back into file order, so output
is byte-identical to a sequential run. Each worker owns its parser and
per-file state; hash maps never determine output order. Use a bounded number
of outstanding file results and serialize them in the requested order. Cache
writes must be atomic under concurrent workers/processes, and failures are
reported in file order.

### A7. One persistent CLI cache, with complete keys

Remove the CLI's unconditional in-memory cache layers from the ordinary
single-visit scan, keeping their public library APIs. Prelude analysis can
reuse a request-local map of results because it actually revisits files.

Use the tree-hugger package's [PassFingerprint](../../lib/src/cache/mod.rs)
for pass inputs, alongside normalized file identity, source hash, forced
language, canonical requested passes and extraction selection. The
fingerprint includes schema, grammar version/identity, actual effective query
text (including inherited queries and overlays), relevant configuration, and
extractor implementation revision. Rules and external-tool versions affect
only passes that use them. Any later input that changes records, such as the
naming context `2026-10-03-symbol-filters-and-reports` introduces for
qualified names, joins the key in the change that introduces it.

**Reader's note:** hashing query text cannot detect a change to Rust extraction
code. Keep `EXTRACTOR_REVISION` as one fingerprint input until a reproducible
build identity replaces it; a computed query digest alone cannot replace it.
Use `biscuit-hash` for all digests and an unambiguous canonical encoding.

Cache analysis before presentation filtering. A kind-restricted extraction
has its own cache selection key and never satisfies a broader request unless
its recorded coverage proves completeness. Name, export and output-format
filters do not invalidate complete extraction data. A full-index request
cannot reuse an entry missing relations, docs or diagnostics. Corrupt or
incompatible entries are misses, not empty results; `--no-cache` bypasses reads
and writes. Cache failures remain recoverable and quiet unless verbose output
is requested.

### A8. The bind pass indexes candidate names

`find_reference_target` scans every symbol for each reference. The change:

- A `HashMap<&str, Vec<usize>>` built once per file replaces the full scan.
  Bucket lookup is expected constant time; choosing among same-named candidates
  still costs work proportional to that bucket. No new dependency is needed.
- `push_unique_ref` uses a membership set per owner while preserving ordered
  vectors. Compare resolved references by ID; compare unresolved references by
  name and qualifier. Two missing IDs do not make two references equal.

Indexing only removes the repeated full-file scan; it does not change which
candidate is chosen. Name-only binding remains the resolution rule. Scope-aware
resolution is the unscheduled `scoped-references` feature.

**`is_recursive` is honest.** Name-only binding cannot prove that a call
targets the enclosing function, nor that none does. Whenever semantic analysis
runs, `semantics.is_recursive` reports `Unknown`, never a guessed `Yes` or
`No`, until scoped resolution exists. No analysis requested remains `None`.

### A9. Output is deterministic and streamed

- `created_at_epoch_ms` and `updated_at_epoch_ms` are both removed from record
  provenance; extractor identity, version and pass remain. Two runs over the
  same tree then produce identical output and identical cache entries.
- JSON is written with `serde_json::to_writer` to a locked, buffered stdout,
  instead of building one large string. This streams serialization; default
  file ordering still requires bounded buffering of parallel results.
- Pretty output goes through one locked `BufWriter`, not a `println!` per line.
- `SOURCE_LINE_CACHE`, the global mutex-guarded copy of every source file kept
  for label guessing, is deleted because records now carry declaration keywords
  (see “The library reports the declaring keyword”).

A stdout write error stops work; a broken pipe exits quietly without a panic.
Read/parse failures go to stderr and produce a nonzero status. For JSON,
collect analysis errors before writing the single document so failures cannot
silently produce a success report.

### A10. Prelude resolution reuses analysis without losing exports

The tree-hugger-cli package's [prelude resolver](../../cli/src/prelude.rs)
currently parses both preludes and target files independently. Discover its
required files before scheduling work and reuse complete per-file indexes.
An explicitly narrowed scan may exclude a prelude target; analyze that target
on demand without emitting it as an extra selected file.

Preserve alias names and declaration locations, unresolved public re-exports,
and `PRELUDE` environment names. Synthetic export records are marked synthetic
and do not borrow the target declaration's ID while claiming a different
name/location. Prelude membership is determined before ordinary symbol filters;
lookup cannot use a kind-restricted index that omitted the target.

### Performance targets

These are goals for a quick before/after sample on the baseline host using
the same release build settings and command inputs. Ratios are not guarantees
across hosts. Behavioral work counters establish the removed work; timing is
supporting evidence, not a statistical or CI gate. A missed goal is recorded
with the remaining cost rather than triggering a larger measurement program:

| Measurement | Target |
|---|---|
| `hug functions --no-cache` on `tree-hugger/` | ≤ 40% of baseline |
| `hug functions` warm | ≤ 15% of baseline |
| `hug lint --no-cache` | no slower than baseline |
| Peak RSS, `hug symbols` on the whole repo | no higher than baseline |

The targets measure the fixed command kind sets. Filters such as `--kind`
beyond those sets arrive with `2026-10-03-symbol-filters-and-reports` and are
not part of this sample.

---

## B. Record foundation

### B1. The library reports the declaring keyword

`SymbolRecord.identity` in the tree-hugger package gains
`keyword: Option<String>`, taken from the declaration's syntax node, never
derived from `declaration_text` or other source text. Examples: `struct`,
`union`, `enum`, `type`, `trait`, `impl`, `mod`, `const`, `static`, `let`,
`var`, `val`, `class`, `interface`, `protocol`, `object`, `record`, `def`,
`func`, `sub`, `function`. A construct with no declaring keyword, such as a
Java method, reports `None`. The renderer prints the keyword, falling back to
the kind name.

This replaces `symbol_presentation` and `binding_token_for_symbol` in
`cli/src/main.rs`. Those match source text (`trimmed.contains(" let ")`,
`trimmed.contains('=')`) and print labels like `=`, `for` and `:=`. They also
misfire on multi-line declarations and on comments that contain a keyword.

### B8. Render-ready signature text

Records no longer carry timestamps, as specified under “Output is deterministic
and streamed.” Fill `source.signature_text` with the declaration header as
written, for display. Scripts then receive an exact, language-native
signature, and `2026-10-03-symbol-filters-and-reports` renders it under
`--signatures full`.

Preserve internal whitespace and line breaks; strip only outer whitespace.
Declaration-only signatures stop at the terminator; expression bodies stop at
their grammar-defined boundary. The header excludes initializer expressions,
in-node annotations and decorators, doc comments, and a trailing `=` or `=>`.
Missing grammar support returns `None` and a compact-rendering fallback, never
the full body.

#### Three fields, three contracts

`declaration_text` (from `2026-10-03-visibility-and-kind`) and `signature_text`
are both kept, with distinct contracts:

- **`declaration_text`** is evidence. It runs from the start of the
  declaration node to its body, including initializers and the annotations
  the grammar parses inside the node, so it keeps syntax a visibility level or
  kind cannot carry.
- **`signature_text`** is the display header. It is the part of the same
  declaration a reader needs to recognize the symbol.
- **`keyword`** comes from the syntax node, not from either text.

Both texts are filled from one declaration node in one extraction pass, and
`signature_text` is always a contiguous sub-range of `declaration_text`.

| Declaration | `declaration_text` | `signature_text` | `keyword` |
|---|---|---|---|
| `pub(in crate::a) fn f() { … }` | `pub(in crate::a) fn f()` | `pub(in crate::a) fn f()` | `fn` |
| `pub const TABLE: [u8; 256] = [0, 1, …];` | `pub const TABLE: [u8; 256] = [0, 1, …];` | `pub const TABLE: [u8; 256]` | `const` |
| `public private(set) var count = 0` | `public private(set) var count = 0` | `public private(set) var count` | `var` |
| `@Override public void run() { … }` | `@Override public void run()` | `public void run()` | `None` (renders `method`) |
| `private[Shape] def secret() = 2` | `private[Shape] def secret() =` | `private[Shape] def secret()` | `def` |

Syntax the grammar places outside the declaration node, such as a Rust `#[…]`
attribute or a Python decorator, appears in neither text.

#### Default reports carry the header, not the evidence

`signature_text` is part of default `--json` records. `declaration_text` is a
source blob like body and AST text: it is omitted from default `--json`
records and present in `--json-full`. `--json-full` means complete analysis,
not an automatic copy of all source.

### B9. A `SymbolQuery` API

The library gains one entry point for "give me these symbols", used by the CLI
and available to other consumers:

```rust
let query = SymbolQuery::new()
    .kinds([SymbolKindV2::Function, SymbolKindV2::Method])
    .capabilities([Capability::Declarations, Capability::Containment]);
let result = SourceFile::read(path)?.query(&query)?;
```

The new API is in the tree-hugger package beside [TreeFile](../../lib/src/file/tree_file.rs),
the existing parsed-file entry point. Both source snapshots and parsed files
accept the same query. Library defaults select all kinds. Queries specify
required capabilities separately from selection: declarations, containment,
references, semantics, structured docs and diagnostics. Selection cannot
silently remove prerequisite data. Return records plus file/language identity
and completed capabilities, so an empty result and an uncomputed facet are
distinguishable.

This feature defines only kind selection and capability selection.
`2026-10-03-symbol-filters-and-reports` adds scope, visibility, test, docs and
metric predicates and capabilities to the same type.

Required work is explicit:

| Request | Required analysis |
|---|---|
| Basic functions/types/symbols | declarations and cheap metadata (keyword, signature text); no references or diagnostics |
| Classes | declarations plus containment, including excluded parent identities |
| Complete index / `--json-full` | all index capabilities, including diagnostics |

Keep `completed_capabilities` in query results/reports for work such as
containment that is narrower than a complete pass. A cache hit uses stored
capability coverage; an empty facet is not proof that it was computed.

File discovery, Git selection and output ordering remain CLI responsibilities.
The library owns kind selection once; the CLI owns a separate presentation
step for grouping classes. Imports remain import records, not synthetic
declarations forced into `SymbolQuery`.

---

## D. Reporting

### D1. JSON sized to the selection

- `--json` emits `{ report_schema_version, schema_version, root_dir, files:
  [{ file, language, completed_passes, completed_capabilities, symbols: [SymbolRecord] }] }`,
  containing only selected records. Class reports additionally contain
  `members`, keyed by selected container ID, with the member records the
  command displays; their `symbols` array contains the selected containers.
- Remove the old v1 symbols payload and unconditional `symbol_indexes` block.
  Empty selections use `files: []`; omit file entries with no matches.
- `--json-full` emits the same versioned envelope with complete per-file
  `indexes` instead of selected `files`. File selectors still apply; symbol
  selectors are rejected rather than implying a complete index is filtered.
  Request every index capability, including diagnostics.
- Default records omit unrequested relations/semantics and large
  source/body/AST blobs, including `declaration_text`. `completed_passes` and
  query result capabilities make omitted work explicit. A reference may name an
  excluded symbol; selection does not rewrite it or add that symbol to
  `symbols`.
- All machine-readable paths are portable scan-root-relative paths, with one
  `root_dir` anchor in JSON; the scan root is the invocation's resolved root.
  Preserve library-native source paths internally and project paths without
  altering record IDs.

Target: `hug --json functions` on the baseline `tree-hugger/` input is under
3 MB; measure serialized bytes, retaining required identity and source spans.
`imports` and `lint` retain their own current report shapes in this feature;
the new declaration envelope is not a global change to `--json`.

### D7. A useful default order

| Order | Today | After |
|---|---|---|
| default | by name across all files | by file, then source position (`--sort name` keeps the old order) |
| `--sort kind` | alphabetical by the kind's name (`--sort-by-kind`) | Module, Namespace, Type, Class, Interface, Trait, Enum, TypeAlias, Function, Method, Constructor, Macro, Constant, Field, Property, EnumVariant, Variable, Parameter, Unknown |

Tie-breakers are portable file path, start byte, end byte, kind and ID; name
sorting first uses the written case-sensitive name. Sort relation targets
deterministically too. Existing sorting/grouping flags (`--sort-by-kind`,
`--sort-by-module` and the grouping options) map to these rules where
equivalent; conflicting explicit orders are argument errors. Byte identity
applies to the same input/root/options/build, not different absolute roots or
terminal widths.

### D8. Quiet by default

The cache-stats line printed to stderr on every warm run moves behind a new
`--verbose` flag.

### D10. One renderer

In the tree-hugger-cli package, [render_symbols and render_symbol_line](../../cli/src/main.rs)
format the same declarations through separate code paths. Replace them with
one `biscuit-terminal` component, fed from records (keyword, signature,
visibility, container), so visibility and signatures agree in grouped and flat
output. This feature keeps today's visible layout, compact signatures, OSC 8
hyperlinks and `--plain` behavior. Width wrapping, `--format table`,
signature modes and the outline view build on this component in
`2026-10-03-symbol-filters-and-reports`.

---

## Sequencing

```mermaid
flowchart LR
    FIX["2026-10-03-visibility-and-kind"] --> V2["Render records directly:<br/>keyword, signature text; version JSON 4.0"]
    V2 --> PERF["Separate diagnostics;<br/>reuse source and captures"]
    PERF --> Q["SymbolQuery planning<br/>and complete cache keys"]
    Q --> OUT["Parallel deterministic output,<br/>prelude reuse, one renderer"]
    OUT -.-> F2["2026-10-03-symbol-filters-and-reports"]
```

Each phase leaves `hug` working, and updates relevant docs/tests with its code.
Phase 1 consumes records directly and removes diagnostics from listing work.
Preserve v1 library adapters until the separate author decision on removing
them. Introduce the shared query planner before kind-restricted cache entries
or parallel prelude reuse, so those optimizations have one completeness
contract. `2026-10-03-symbol-filters-and-reports` starts after this feature
lands; it extends `SymbolQuery`, the record facets and the renderer defined
here. Final acceptance includes all resolved requirements; open author
decisions must be settled or explicitly deferred in the implementation log
before their features ship.

## Decisions

### Resolved

- **The CLI consumes records directly.** This preserves metadata and removes
  conversion/cloning work; the v1 public library API remains supported here.
- **Queries declare completeness.** Restricted extraction and missing passes
  cannot masquerade as a complete cache hit.
- **Deterministic records have no wall-clock timestamps.** Breaking record
  changes are versioned; old cache entries are rejected.
- **No candidate-bucket dependency.** Use standard collections for the bind
  index.
- **Parallel output preserves order.** Buffering is bounded in file order;
  global name sorting may need to retain all selected records.

Decided by the author on 2026-10-03:

- **The original feature is split in two and trimmed.** This spec keeps
  performance and the record/query/report foundation; metadata, filters and
  reports move to `2026-10-03-symbol-filters-and-reports`, and five
  lower-priority items become unscheduled features (`scoped-references`,
  `changed-files-filter`, `cyclomatic-complexity`, `extra-report-formats`,
  `stats-package-grouping`). One spec was too large to implement and review as
  a single change.
- **`keyword` lands here.** Deleting `SOURCE_LINE_CACHE` is justified only if
  records already carry the keyword the renderer used to guess.
- **Reference leftovers stay, honestly labeled.** The name→candidates index
  and `push_unique_ref` set remove measured cost without changing results;
  `is_recursive` reports `Unknown` rather than a guess until scoped resolution
  exists, because name-only binding cannot prove either answer.
- **Library matrices are exhaustive; CLI goldens are representative.** Each
  extraction fact is asserted for all 16 languages in library tests, while CLI
  output uses one golden per output format over the 16-language `multi/`
  tree. This matches the prerequisite fix's rule and keeps CLI goldens
  maintainable.
- **`declaration_text` and `signature_text` both stay, with written
  contracts.** One is evidence, the other is display; filling both from one
  node in one pass keeps them consistent and cheap.
- **`declaration_text` is omitted from default JSON.** It is a source blob like
  body text; `--json-full` carries it.

### Open Questions

#### Removing v1 from the library

This is a separate follow-up decision, not work authorized by this feature.
Moving the CLI does not prove that other packages have no v1 callers.

- **Remove after an audit:** one public model and less adapter maintenance;
  requires migrating repository callers and changes the public API.
- **Keep a convenience adapter:** simpler entry point for existing callers;
  carries continuing maintenance and risks losing new metadata.
- **Recommendation:** audit then remove in a follow-up. Complete the CLI move
  first and record actual remaining callers before estimating removal work.

#### Annotations interleaved with modifiers

`signature_text` excludes in-node annotations and must be a contiguous
sub-range of `declaration_text`. Leading annotations (`@Override public void
run()`) satisfy both. An annotation written after a modifier, such as Java
`public @Nullable String name()` or a C# attribute inside a modifier list,
cannot be removed without breaking contiguity.

- **Keep interleaved annotations:** `signature_text` starts after the leading
  annotations only; contiguity holds, and the exclusion rule becomes "leading
  in-node annotations".
- **Drop contiguity for these cases:** removes every annotation; the
  sub-range test becomes a subsequence test.
- **Recommendation:** none recorded; this case surfaced while splitting the
  spec and needs an author decision before signature extraction is planned.

---

## Testing

**Library extraction facts are tested in every supported language.**
"Supported" means every value of `ProgrammingLanguage::all()`. Today that is
16 languages:

- Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala
- PHP, Perl, Lua, Bash, Zsh

A capability that a language can't express is asserted as not applicable
(`None` or empty). Separately label unsupported extraction/grammar support;
those constructs are not falsely classified as absent from the language. Each
row identifies representative positive and negative cases, rather than
pretending every feature is applicable everywhere.

**CLI output is tested representatively**, as in the prerequisite fix:
exhaustive language/value combinations belong in library tests; CLI tests
prove wiring and output shape.

All tests named here are L1 unless marked otherwise. Run them with `just test`
in `tree-hugger/`, using nextest through that recipe; use the monorepo Test
Toolkit for isolated CLI/cache fixtures. Repository fixture reads use
runtime harness roots and literal joined paths that the test-input index can
resolve. No implementation test should read this dated spec as its contract.

### Shared fixtures

- **`lib/tests/fixtures/symbols/<language>.<ext>`**, introduced by the fix.
  This feature extends each one, where the language has the construct, with:
  - a declaration with a large or multi-line initializer
  - a multi-line declaration header
  - an annotated or decorated declaration whose annotation is inside the
    declaration node
  - a comment containing a declaration keyword
- **A `lib/tests/fixtures/symbols/multi/` tree** containing all 16 languages,
  for multi-file, parallel, ordering and CLI golden tests.

### Coverage guard

The fix's language-coverage test (iterate `ProgrammingLanguage::all()`, fail
on a missing fixture or matrix row) is extended to check every matrix below.
A new language can't be added without a row in each.

### Per-language library matrices

Each matrix is a table-driven test with one row per language, so a failure
names its language.

| Capability | Asserted per language |
|---|---|
| Keyword | every declaration in the fixture reports the expected keyword from its syntax node; `None` only where the language has no keyword for that construct; a keyword inside a comment never produces a label |
| Signature text | expected `signature_text` per declaration; it is a contiguous sub-range of `declaration_text`; it excludes initializers, in-node annotations/decorators, doc comments and trailing `=`/`=>`; missing grammar support is `None`, never the body |
| Cache correctness | a warm-cache result equals a `--no-cache` result |

### CLI output goldens

One golden per output format this feature produces, built from `hug symbols`
over the 16-language `multi/` tree:

- `cli/tests/fixtures/golden/symbols.plain` (terminal text under `--plain`)
- `cli/tests/fixtures/golden/symbols.json` (`--json`)

They are compared after `strip_ansi_codes` and path redaction (the existing
`corpus` helpers), so they hold on every OS. Other commands are covered by
structural assertions: `functions` and `types` select exactly their kind sets,
and a `classes --json` report carries containers in `symbols` and displayed
members in `members`.

### Focused cases

- **JSON envelope.** `--json` parses, holds only selected records, carries
  `report_schema_version`, `schema_version` 4.0, `root_dir`, per-file
  language, `completed_passes` and `completed_capabilities`, and round-trips
  through serde. It omits `declaration_text`; `--json-full` includes it and
  complete indexes, and rejects symbol selectors. An empty selection is
  `files: []`.
- **Performance counters.** A symbol command over `multi/` records zero
  diagnostics-pass executions. This is observed through a pass counter exposed
  for tests, not through timing. Propagate counters into parallel workers.
  A complete warm hit additionally records zero parser constructions, and a
  cold analysis records one definition-capture extraction per file.
  `lint` and `TreeFile::symbol_index_v2()` still run diagnostics.
- **Parallel equals sequential.** Over `multi/`, the parallel run's `--json`
  output is byte-identical to a run with a single rayon thread.
- **Two runs are identical.** Back-to-back runs, cold and then warm, produce
  byte-identical `--json`.
- **Cache correctness.** Editing a language's query text in a test-local copy
  invalidates the entry. Also vary extractor revision, language override,
  query kinds, schema and requested capabilities. Warm narrow then broad
  queries (and the reverse) must match uncached results. Simulate corrupt and
  concurrent writes without treating either as empty success.
- **Binding.** The candidate index selects the same targets as the previous
  full scan; `push_unique_ref` keeps distinct unresolved references with
  missing IDs; `is_recursive` is `Unknown` whenever semantics run and `None`
  when not requested.
- **Prelude behavior survives.** Cover aliases, unresolved re-exports,
  environment names and targets outside the selected scan, with no duplicate
  parse.
- **Order.** Default file/source order, `--sort name`, `--sort kind`, mapped
  legacy flags and conflicting-order argument errors.
- **Output failures are observable.** Cover broken pipes, file errors and
  deterministic parallel error reporting without real terminal focus.
- **Quiet stderr.** A warm run prints no cache stats without `--verbose`.

### Performance sample

Use one host and a quick release-build before/after sample of the existing
baseline commands and inputs. Record elapsed time, serialized bytes and the
already requested peak RSS comparison in the implementation log; record the
build and cache state. The baseline already establishes that listings spend
lint-like time, so no separate exploratory spike is needed to prove that cost.
Do not expand measurements to every language, additional hosts, repetitions,
statistical thresholds or new benchmark corpora. The existing Criterion suite
may remain available through `just bench`; it is not a mandatory new measurement
program or CI gate. The all-language fixture tree proves correctness, not a
performance target.

### Host coverage

Path rendering is OS-sensitive. Its tests run on the environments CI schedules
for the area, and follow the `os` skill's guidance for path comparison.
Packages remain required to work on macOS, Linux, native Windows and WSL2;
this feature adds no CI environments or gates, and needs no real terminal or
focus changes.

---

## Documentation to update in the same change

- **`tree-hugger/docs/symbol-kinds-and-visibility.md`** (created by the fix):
  extend it with keywords and the `declaration_text` / `signature_text`
  contracts, including the comparison table.
- **A new `tree-hugger/docs/symbol-reports.md`.** It covers, for a reader new
  to the repository:
  - every symbol command and its fixed kind set
  - the JSON envelope, with a short sample, and what `--json-full` adds
  - default and explicit ordering
  - a Mermaid diagram of the query → passes → render flow
- **`tree-hugger/docs/cache-design.md`:** the single cache layer and the
  computed fingerprint.
- **`tree-hugger/README.md`, `cli` help text and the shell completions:**
  `--sort`, `--verbose`, and the changed JSON shape and default order.
- **`.claude/skills/tree-hugger/SKILL.md` and `api-reference.md`:**
  - `SourceFile` and `SymbolQuery`
  - `keyword` and `signature_text`
  - the removal of `InMemorySymbolCache` and `InProcessCache` from the CLI
    path
- **`tree-hugger/docs/symbol-schema-design.md`:** record/index version 4.0,
  removed timestamps, container rename, `keyword`, the two text contracts,
  `is_recursive` reporting `Unknown`, explicit pass dependencies and retained
  v1 adapters.
- **`docs/dependencies.md` and area dependency docs:** update only if dependency
  declarations change; no new dependency is planned.

## Out of scope

- Correct kinds, visibility, `static` and containers. That is
  `2026-10-03-visibility-and-kind`.
- Scope classification, modifiers and attributes, generics and inheritance,
  qualified names, metrics, every new filter (`--kind`, `--scope`,
  visibility, `--in`, smart case, tests/docs/deprecation, signature shape,
  `--file`/`--name`), JSON Lines, outline, wrapped rendering, `--format
  table`, `--signatures`, `--count` and `--stats`. That is
  `2026-10-03-symbol-filters-and-reports`.
- Scope-aware reference resolution, occurrence records and `hug refs`; see the
  unscheduled `scoped-references` feature.
- `--changed[=<ref>]`; see the unscheduled `changed-files-filter` feature.
- Cyclomatic complexity; see the unscheduled `cyclomatic-complexity` feature.
- TSV, Markdown and tags output; see the unscheduled `extra-report-formats`
  feature.
- Package grouping in `--stats`; see the unscheduled `stats-package-grouping`
  feature.
- Cross-file or project-wide reference resolution; the resolver work in
  `lib/src/resolver/` covers the project level.
- Removing v1 from the library's public API; see its follow-up Open Question.
- Lint, diagnostics and god-files output.
- Language-server or editor integrations.
