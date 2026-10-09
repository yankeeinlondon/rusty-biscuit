# Persistent Symbol Cache

`cache::SymbolCache` (`cache/symbol_cache.rs`) is the only cache `hug` uses
(`--no-cache` skips it); full design and the robustness table are in
[`docs/cache-design.md`](../../../tree-hugger/docs/cache-design.md).

- **One entry per file.** Key = `canonical_digest` (length-prefixed BLAKE3)
  of path (`/` separators), source xxHash, forced language or `detected`,
  and the records `PassFingerprint` (schema, `<crate>+rev.<EXTRACTOR_REVISION>`,
  grammar identity read from the `Language`, effective `locals`/`imports`/
  `references` text). Kinds and capabilities are **not** key inputs; the
  entry records `covered_kinds` (`"all"` or a non-empty ascending list) and
  `completed_capabilities`.
- **Diagnostics carry their own fingerprint inside the entry** (lint +
  comments text, rule metadata, `experimental_semantics`); a mismatch makes
  the diagnostics "not computed", never a key change.
- **Lookup**: `Hit` (no parse) / `WidenedCapabilities { parsed }` (docs,
  semantics-with-references and containment-over-all-kinds run on stored
  records; references and diagnostics re-parse) / `WidenedKinds` (one
  analysis for the union) / `Miss(CacheMiss)`. Every result goes through
  `analysis::coverage::project`, which narrows a wider entry to exactly the
  uncached result; a new pass that changes records must be reflected there
  (`symbols_cache::projected_results_*` fail otherwise).
- **Reader**: `deny_unknown_fields`, no defaults on entry fields, custom
  visitors for the coverage lists; failures are `CacheMiss::Malformed(serde
  message)` / `Mismatched` / `Inconsistent`, never an empty result. The entry
  and **every record** must carry exactly `SchemaVersion::CURRENT` (minor
  included), else `Mismatched("record schema version")`; a nested
  version-bearing value must be checked in `StoredEntry::validate`. Reference
  evidence has its own strict form (`cache/stored_evidence.rs`): the entry
  writes every key (`null` owners/IDs, `[]` candidates/reasons) and
  `deny_unknown_fields` types read them, never the public serde defaults;
  `validate_evidence` (one `StoredEvidence` argument) then checks spans
  against the source (bytes, char boundaries, one-based line/byte column),
  confidence/ID/candidate consistency, generated-ID form (`is_generated_id`,
  64 lowercase hex) for every evidence/summary/record ID, every target,
  candidate, owner and summary ID against a *retained* record of the written
  name (`StoredEvidence::covered_kinds`: an absent record is accepted only
  when the entry omits a kind it could have; owners and `referenced_by` are
  callables, so only an omitted Function/Method/Constructor exempts them;
  an every-kind entry exempts nothing), a `Scoped` no-ID target's name written
  elsewhere (`unbound_names_have_a_binding`: an import or unreported binding
  always is), canonical order, owners against coverage regions, the
  innermost region (`innermost_owners`) and records' names, unique regions
  and a retained callable's own region iff it has a `body_span`, containing
  that body (every language; never compared with `declaration_span`, which
  is only the declarator for C/C++; the region builder in
  `occurrences::resolve` reads the same fact, so an empty or comment-only
  Lua body owns its declaration's region), each non-null
  `target.qualified_name` (the written path, `written_path`) the source text
  ending at the occurrence's end and longer than the name, and agreeing with
  the occurrence's cache-only `path_start_byte` (a required key in
  `StoredOccurrence`, never in the public schema; `null` iff the path is
  `null`, else `end - path.len()`; `ReadOccurrences` carries the starts beside
  the occurrences into `StoredEvidence::path_starts`), in every language and
  whether or not any record is retained: a path or start deleted alone is
  `Inconsistent`, both deleted together is the documented coordinated-forgery
  limit (no source scan can tell a never-written `null` path from a deleted
  one: Rust macro token trees, leading `::`, JS spread), candidates in
  `Resolver::candidate_key` order (name position, then ID; `Resolver::new`
  sorts each `by_name` bucket once, `name_only` keeps the first of adjacent
  equal IDs) among retained records, summaries without
  repeats (`RefSet`'s key) in first-occurrence order
  (`summaries_in_first_occurrence_order`: exact for IDs; import edges must
  match the exact selected import span retained on each scoped occurrence), the
  no-rules languages' exact name lookup and `unsupported_language` reason,
  edges both ways (`references`/`referenced_by`/`dependencies` vs `Scoped`
  occurrences), and the whole `semantics` facet recomputed by
  `analysis::semantic_facets` (the semantic pass's own function; default
  without `semantics`). A test that renames a record or forges a qualified
  name in an entry must update the evidence refs and reference summaries
  too (`cache_upgrade::rename_record`/`set_qualified_name` do), and can only
  rename a record no occurrence writes (`rename_record` asserts it). Extend
  `cli/tests/symbols/cache_robustness.rs` (entry and record columns;
  `evidence_columns`/`INCONSISTENT_EVIDENCE` for evidence) for a
  new load-bearing field, and assert `CacheLookup::Hit` (not just equal
  output) in any warm-run test: a silently undecodable entry still produces
  equal output.
- **Writes** are temp file + rename (`PersistentCache::put`); last rename
  wins.
- **One directory for every scan root**: `hug` opens
  `default_cache("hug")` (`CACHE_PROJECT_ID` in `cli/src/main.rs`), never a
  package-root partition, so a file reached from two scan roots shares one
  entry
  (`cache_lookup::a_run_from_another_scan_root_hits_the_same_entry_with_the_same_ids`).
  Nothing scan-root-dependent may enter a stored record or the key.
- `StoredEntry::validate` refuses (`Inconsistent`) a `metrics` facet or a
  `true` `is_test`/`is_deprecated` whose capability (`metrics`/`flags`) the
  entry does not list, and a record without `metrics` under `metrics`. Flags
  and metrics need the tree, so widening by them parses. Query text is
  injectable for tests (`SymbolCache::with_query_text`).
- `InProcessCache`, `InMemorySymbolCache`, `SymbolSnapshot`,
  `AnalyzerFingerprint` remain as library types but no CLI path uses them.

## Cached import consistency

`StoredImport` and `StoredImportRef` are strict private readers in
`cache/stored_evidence.rs`. Import validation precedes every capability/kind
projection, including entries retaining no occurrences. External IDs and
symbol source spans are never produced. Exact written capture geometry,
name agreement, canonical capture order, and selected-import dependency
provenance are checked without another Tree-sitter parse. Required
`import_inventory` is a BLAKE3 witness of the complete grammar-produced import
list, including nullable metadata and exact name/statement geometry. The sole
entry writer computes it before projection; the reader checks it before every
capability, rejecting missing grouped members, invented roles, and changed
boundaries or metadata. There is no approximate source lexer. Retain independent
geometry/provenance checks alongside the witness; it is consistency evidence,
not authentication against coordinated edits of every witness.
The `symbols_cache_imports` L1 corpus restores a tool-written entry per edit,
asserts independent public facts and actual later hits, and has a registration
guard for all producer/geometry partitions. Preserve genuine no-alias/source/
original/statement controls and opaque/string/non-import syntax warm hits.
The shared `tests/fixtures/scoped_import_lexical.rs` corpus feeds library
`symbols_cache_lexical` and CLI `cache_lexical`; `just test lexical_inventory`
checks every supported language, all regex contexts and comment/raw delimiters,
six cache stages, deleted/invented imports and JSON/text reference directions.
Keep the live-target corpus and consumer registration guards complete.
