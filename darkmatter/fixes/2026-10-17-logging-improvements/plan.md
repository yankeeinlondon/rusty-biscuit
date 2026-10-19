---
total_phases: 6
created: 2026-10-17
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/dmls/src/clock.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/src/log_capture.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/fixtures/triggers/feature-review-incident.yaml
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
source_files_during_phase_2:
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/clock.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/src/workspace/mod.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/capabilities.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/wiki.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_5:
    - darkmatter/dmls/src/full_index.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/mod.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/full_index.rs
    - darkmatter/dmls/tests/l1/startup_log_lines.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_6:
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/edge.rs
    - darkmatter/dmls/src/graph/node.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/definition.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/src/providers/rename.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
docs_updated_during_phase_6:
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/how-to-use.md
    - darkmatter/docs/topics/dmls.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/schema-activation.md
    - darkmatter/dmls/README.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/dmls/docs/hover.md
docs_created_during_phase_6:
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
source_code:
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/dmls/src/clock.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/src/log_capture.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/fixtures/triggers/feature-review-incident.yaml
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/mod.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/capabilities.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/wiki.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/src/full_index.rs
    - darkmatter/dmls/tests/l1/full_index.rs
    - darkmatter/dmls/tests/l1/startup_log_lines.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/graph/edge.rs
    - darkmatter/dmls/src/graph/node.rs
    - darkmatter/dmls/src/providers/definition.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/src/providers/rename.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
documentation:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/lsp/how-to-use.md
    - darkmatter/docs/topics/dmls.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/schema-activation.md
    - darkmatter/dmls/README.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/hover.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
completed_phase: 6
implemented: true
---

# Plan: DMLS Startup Schema Registry and Logging

## Summary and Definition of Done

The spec makes nine changes across the `darkmatter` library and `dmls`:

- **Library (R1, R2):** the trigger envelope classifier stops treating
  unparseable, unclaimed YAML as fatal, and a claimed-but-unparseable envelope
  gets a dedicated `SchemaError` variant.
- **DMLS caching (R3, R4):** a startup schema registry replaces per-request
  trigger rescans, and registry state changes are logged once.
- **DMLS startup (R5, R6, R7):** a flat file list replaces the startup parse of
  every Markdown file. Referenced targets are parsed on demand. The full index
  is built lazily for references, rename, and workspace symbols.
- **Logging (R8, R9):** new startup and index log lines, and slow-request
  timing.
- **Docs:** two new pages (`configuration.md`, `indexing.md`) plus a sweep.

**Done means** (the spec's acceptance criteria):

- [x] Unrelated malformed YAML in a schema root cannot disable triggers
  (`md compose`, `md schema triggers`, DMLS).
- [x] Repeated ready trigger-registry lookups do no directory scan, file read,
  hash, or payload resolution.
- [x] Startup parses only open documents; unopened Markdown is parsed on first
  need.
- [x] References, rename, and workspace symbols return complete answers from a
  full index built on first use.
- [x] Log lines match R4, R8, and R9.
- [x] Every row of the spec's Tests table passes under `just test` in
  `darkmatter/`, and `just lint` is clean.
- [x] Docs are created and swept per the spec's Documentation section.

Agent terminal state is "implementation complete, ready for review". Do not
move the spec to `_completed` and do not commit unless told to.

## Phase 1: Rulings, Seams, and Library Fix (R1, R2)

Settle open design points, add the test seams every later phase depends on,
and land the library-level fix, which is independent of DMLS.

### Necessary Rules

The spec left these points open. The author confirmed the recommended
option for every ruling on 2026-10-17; they are final and bind all phases.

1. **Registry module placement.** Ruled: a new
   `dmls/src/overlay/registry.rs` owning classification, dependency records,
   and the atomic install; `OverlayCache::trigger_registry` becomes a lookup
   only.
2. **Where "stage timings" live (R9).** Ruled: a per-request
   `RequestTimings` value created at the dispatch boundary and passed through
   the overlay build, instead of global spans. Spans alone cannot be asserted
   in an injected-clock test.
3. **Claimed-envelope fallback scanner (R1).** Ruled: a small
   line-oriented scanner that tracks multiline quoted and block scalars well
   enough to skip lines inside them. It is not a regex. If a line cannot be
   proven top-level, it is unclaimed (`Ok(None)`), per the spec's stated
   limitation.
4. **Non-YAML dependencies in the fallback watch path (R3/R5).** The registry
   records payload, import, and example files "wherever they live". Ruled:
   register watches only on their parent directories (dedup'd), not per
   file, to keep the registration count bounded.
5. **Workspace identifier for log lines (R8).** The spec says "the workspace
   identifier" but does not define it. Ruled: the workspace folder's
   URI as given by the client, falling back to the effective root path.
6. **`failed_triggers` and `failed_scopes` fields.** The R8 table omits them
   but the field notes require them. Ruled: both are always emitted
   (zero when none), and the line text switches to
   `schema registry initialized with failures` when `failed_scopes > 0`.
7. **Cancellation granularity (R7/R9).** Ruled: cancellation is
   cooperative, checked between file parses, so a canceled waiter returns
   promptly without aborting a shared build.
8. **Wider measurement.** Ruled: no multi-OS file-list timing sample
   (Windows walk cost in particular) in this fix, consistent with the spec
   ruling out a new perf spike. Any such sample is a separate change.

### Spikes

None. The spec's own `--bench-index` figures already show Markdown parsing
dominates startup. This is a ruling, not a spike.

### Input Robustness Matrix

R1 changes a file-format reader (the trigger envelope classifier), so the
matrix applies. Load-bearing fields: the `kind` claim and the envelope shape
(`match`, payload). The loader is one YAML parse plus the new fallback scan.

**Default outcome.** A parseable envelope that claims the envelope but has an
invalid shape returns a located `TriggerMatch` error, loads nothing, and
rejects the scan. A claimed envelope with invalid YAML returns the new
variant with line and column. Exceptions are listed below.

**Document shapes (test once; one loader):**

| Shape | Outcome |
| --- | --- |
| unparseable, claim line present | new YAML error variant (R2) |
| unparseable, no claim | `Ok(None)` (exception to default) |
| unparseable, indented or nested `kind:` | `Ok(None)` |
| unparseable, `kind: trigger-schema-extra` | `Ok(None)` |
| unparseable, commented-out claim | `Ok(None)` |
| unparseable, claim text inside a multiline quoted scalar | `Ok(None)` |
| CRLF line endings and initial BOM | same result as LF without BOM |
| duplicate key `kind` | follow the YAML parser's existing outcome; assert, do not change |
| trailing invalid content after a valid document | unparseable path, so the claim rule decides |
| second YAML document | unchanged from today; assert it |
| quoted-key and JSON-style mapping that parses | unchanged (claim read from the parsed mapping) |

**Field shapes:** `kind` is a scalar, so only the absent, explicit-null,
wrong-type, and empty rows apply. On a parseable document, `kind:` null,
`kind: 123`, `kind: ""`, and an absent `kind` all return `Ok(None)` and are
never conflated with a claim. The control case is the unedited valid envelope.

**Tests:** in-process at `parse_trigger_envelope_from_str` and
`triggers::scan`. Spawn `md` only for the end-to-end wiring test (control plus
the incident fixture).

**Code-smell grep before closing:** `#[serde(default)]` on envelope fields,
`unwrap_or_default()` on the YAML parse, and `.ok()` on the YAML parse in
`envelope.rs`. Any hit that turns a parse failure into "absent" is a defect.

### Wave 1 (parallel)

- [x] **Envelope claim fallback.** In
  `lib/src/markdown/schemas/triggers/envelope.rs`, make
  `parse_trigger_envelope_from_str` return `Ok(None)` for unparseable source
  unless the R1 claim rule matches.
  - Normalize CRLF and a leading BOM first.
  - Implement the scanner per ruling 3.
  - Parseable source keeps today's path exactly.
  - Prerequisite: none.
- [x] **New error variant.** Add the `SchemaError` variant for a claimed
  envelope with invalid YAML (message, optional line and column, help text
  about YAML syntax).
  - Discovery wraps it in `TriggerLoad`; embedded bundles keep
    `EmbeddedTriggerLoad`.
  - Handle it explicitly in CLI error codes and help.
  - Keep `TriggerMatch` unchanged for grammar and shape errors.
- [x] **Test seams.** Add the seams later phases need, in DMLS only, with no
  behavior change:
  - a read/hash/parse counter on the file-reading helpers;
  - an injectable monotonic clock trait;
  - a captured-`tracing` subscriber helper.
  - Check the existing test toolkit first and reuse its helpers if present.

### Wave 2 (after Wave 1)

- [x] **Library tests.** Cover every row of the matrix above, plus:
  - discovery with a valid trigger beside an unparseable, unclaimed file
    (the trigger loads);
  - the incident fixture, which is not a trigger;
  - a corpus test that every shipped `schemas/*.yaml` classifies without
    error (extend the existing corpus test if one exists);
  - embedded-wrapper correctness.
- [x] **CLI end-to-end.** In a `git init` fixture with the incident file in a
  schema root, `md compose doc.md` succeeds and `md schema triggers doc.md`
  exits 0. Also assert a claimed-but-unparseable file exits non-zero with the
  YAML message and the YAML help.
- [x] **DMLS authoring path.** In `dmls/src/overlay/mod.rs`, publish the new
  variant's diagnostic at the reported position.
  - Convert the one-based location through the source map into the negotiated
    position encoding, not byte columns.
  - Test non-ASCII text before column 93 so UTF-16 and UTF-8 differ.
  - Test that an unclaimed, unparseable schema-root file gets no trigger
    diagnostic.

### Validation checkpoint 1

- [x] `just test` and `just lint` pass in `darkmatter/`.
- [x] Smell grep done and recorded in the implementation log.
- [x] Rulings 1-8 confirmed by the author (2026-10-17).

## Phase 2: Slow-Request Logging (R9)

Independent of the registry, and it gives later phases timing evidence.
Runs before Phase 3 so the stage timings can be wired as the registry lands.

### Wave 3 (parallel)

- [x] **Dispatch timing.** In `dmls/src/router.rs`, measure each request from
  dequeue to response send with the injected clock.
  - Emit exactly one completion event per terminal path: success, provider
    error, caught panic, cancellation (`cancelled=true`).
  - At or above 500 ms: `warn` `slow request` with `method`, `uri`, `id`,
    `outcome`, `elapsed_ms`.
  - Below 500 ms: `debug` `request complete` with the same fields.
  - Requests with no document omit `uri`.
  - Notifications get `slow notification` / `notification complete`, measured
    through handler completion only.
- [x] **Stage timings.** Add `RequestTimings` (ruling 2) covering overlay
  build, schema assembly, trigger registry, and the provider chain, plus
  flags for "waited for file list" and "waited for full index".
  - Stages are absent when not run.
  - Document nesting; do not sum.
- [x] **No payload logging.** Add a test that no source text, frontmatter
  values, or payloads appear in captured output.

### Wave 4 (after Wave 3)

- [x] **R9 tests.** Using the injected clock, cover below, exactly at, and
  above 500 ms; one event on error and on cancellation; notifications have no
  response timing; URI-less requests omit `uri`.
- [x] **Stdout safety.** Assert logs never enter the stdout protocol stream,
  and `--log-file` routing is unchanged.

### Validation checkpoint 2

- [x] `just test` and `just lint` pass; one completion event per request is
  asserted by test.

## Phase 3: Schema Registry (R3, R4)

The riskiest phase. Builds on rulings 1, 4, 5, and 6 (confirmed).

### Wave 5 (parallel)

- [x] **Root enumeration and classification.** In the new registry module,
  enumerate the immediate regular `.yaml`/`.yml` children of the roots from
  `SchemaRoots::for_document`.
  - Classify each as named schema, trigger schema, or unclassified, per the
    spec.
  - Use the existing resolution contexts and package catalog. Do not walk
    ancestors and do not recapture the environment.
  - Keep symlink rules, case-fold collision errors, filename-byte ordering,
    root dedup, and root-state reporting.
  - Schema discovery is independent of Markdown include/exclude.
  - Claimed trigger failures are recorded as trigger failures; I/O failures
    are never `unclassified`.
- [x] **Per-document registry composition.** Compose the registry for a
  document from the cached classification of its five roots with filename
  shadowing.
  - Unclassified files still shadow later triggers.
  - Resolve payloads and `$path` in the document's context.
  - Cache parsed root contents separately from document-bound registries.
  - Result must equal `triggers::scan`, trigger for trigger and in order.
- [x] **Dependency records.** Record payload, import, and example files, with
  xxHash from `biscuit_hash`, including deleted and missing dependencies,
  failed-load inputs, and searched directories for unresolved names.

### Wave 6 (after Wave 5)

- [x] **Lookup path.** Replace the uncached `scan_triggers` call in
  `OverlayCache::trigger_registry` with a lookup of the ready registry.
  - A ready, unchanged lookup does no scan, read, hash, or payload resolution.
  - Install updates atomically; prepare them outside the provider chain.
  - Fail closed on root I/O errors: no last-good data, no fallthrough to a
    less-local root.
  - Keep per-scope transactional loading and last-good retention. A cold
    failure reports unavailable.
- [x] **Invalidation.** Handle `didChangeWatchedFiles` and YAML `didSave`
  (including for open buffers; the scan reads disk), `rescan_workspace`,
  configuration/context generation changes, and package-manifest changes.
  - Re-read the changed path, compare xxHash, and reclassify only on a
    difference.
  - Creation, deletion, and root-membership changes invalidate lookups
    without a comparable hash.
  - Clear stale diagnostics and refresh affected open documents on recovery.
- [x] **Late contexts.** For a newly opened document outside the prepared
  contexts (including an untitled buffer), prepare its applicable roots before
  answering schema-dependent requests. Never substitute the workspace roots.
- [x] **Registry log transitions (R4).** Implement the three-row table plus
  `trigger registry unavailable`, driven by the same comparison that decides
  diagnostic transitions (one source of truth).

### Wave 7 (after Wave 6)

- [x] **Equality tests.** Per-document registry equals `triggers::scan` for
  documents in a package, a package area, and the tree root, with
  `SCHEMAS_DIR` and a fixture home set.
- [x] **No-I/O test.** Repeated ready lookups hit the read-counting seam zero
  times; other schema assembly keeps its freshness behavior.
- [x] **Invalidation tests.**
  - Watched events for an envelope, an external payload, and a new root file
    update the registry; an unchanged xxHash does not reclassify.
  - Missing payload creation, deletion, external home/environment roots, and
    non-YAML examples recover.
  - Saving an open YAML buffer invalidates; unsaved edits do not.
- [x] **Failure and shadowing tests.**
  - Last-good is retained only where the malformed file is unshadowed.
  - Root I/O errors fail closed.
  - Scoped contexts keep distinct payload and pattern resolution.
- [x] **R4 log tests** with the captured subscriber: repeated identical
  failures give one `warn`, a different error gives a second, recovery gives
  one `info`, and a repeat of the same failure logs at `debug`.

### Validation checkpoint 3

- [x] `just test` and `just lint` pass.
- [x] Re-run the incident reproduction: with the registry up, a mid-edit
  malformed unrelated YAML file produces zero repeated warnings.

## Phase 4: File List and Lazy Parsing (R5, R6)

### Wave 8 (parallel)

- [x] **File-list walk.** In `dmls/src/workspace/discover.rs`, replace the
  parse-everything startup walk with a flat list per effective workspace root.
  - Markdown filtered by `include`/`exclude`; YAML (`*.yaml`, `*.yml`) filtered
    by `exclude` only.
  - Set `require_git(false)` and skip `.git` via `filter_entry`; keep
    `hidden(false)`.
  - Do not follow symlinks; keep canonical-path duplicate reporting (each
    duplicate still logged at `warn`).
  - Read no file contents.
  - Run on a background thread, scheduled after registry preparation.
  - A listing error is logged as incomplete and marks the list incomplete.
- [x] **Lazy target cache.** Add the referenced-target cache keyed by path with
  xxHash.
  - Stale marking keeps the prior hash and parse; a re-read reuses the parse
    when the hash is unchanged.
  - Deletion removes the entry; a failed read never reuses old headings.
  - Open buffers override disk; close restores the disk view or removes an
    unsaved-only entry.
  - Concurrent requests for one target share one parse; stale worker results
    cannot replace a newer buffer version.
  - Parsing is passive: no compose, shell, or remote.

### Wave 9 (after Wave 8)

- [x] **Switch consumers to the file list.** Replace "is this indexed?" with
  "is this in the file list?" for graph edge targets and anchor-completion
  candidates. Path completion in `completion.rs`, `frontmatter.rs`, and
  `wiki.rs` reads the file list.
- [x] **Gate path-existence diagnostics.** Publish broken-link, missing
  `::file`/`::code`/`$schema` target, and wiki-link diagnostics only after the
  file list is ready. Publish syntax and schema findings immediately, then
  refresh when ready. An incomplete list never yields a definitive "missing".
- [x] **On-demand parse features.** Wire missing-anchor diagnostics, heading
  completion, link hover preview, and transclusion cycle detection (visited
  set, followed outward from the open document) to the lazy cache. Targets
  outside the workspace still resolve via `FileReference` in the source
  document's context and get dependency invalidation.
- [x] **Watches and fallback.** Add `**/*.yaml` and `**/*.yml` to workspace
  watch globs even when Markdown `include` is empty. Register watches on the
  actual schema directories and dependency locations (ruling 4) when the
  client supports it. If registration is unsupported or rejected, rescan on
  save. Rebuild the list and watches when discovery configuration or ignore
  files change. No polling.

### Wave 10 (after Wave 9)

- [x] **File-list tests.**
  - Fixture with Markdown, YAML, gitignored `node_modules/`, a symlink, and
    `.git/` containing `.md` files gives the expected list and counts.
  - A non-repository root honors `.gitignore`; a linked-worktree fixture
    (`.git` as a file) still does.
  - Startup with a large fixture parses only open documents (parse counter).
  - A broken-link diagnostic is not published before the list is ready.
- [x] **Lazy-parse tests.**
  - A missing-anchor check parses exactly the target, a second check reuses
    it, and a changed target is re-read.
  - Fallback save and config/ignore changes keep parsing lazy; unchanged
    bytes reuse the parse; open buffers override disk and close restores it.
  - Rejected watch registration recovers via the fallback.

### Validation checkpoint 4

- [x] `just test` and `just lint` pass.
- [x] Same-host sample of startup/listing only, labeled separately from the
  full index (no new numeric gate).

## Phase 5: Full Index on Demand and Startup Logs (R7, R8)

### Wave 11 (parallel)

- [x] **Index state machine.** Implement not-started, building, ready, and
  failed states.
  - Callers share one background build; dispatch stays responsive to edits,
    cancellation, and shutdown.
  - Canceling one waiter cancels only that request.
  - A failed or canceled build never becomes ready; a later request retries.
  - Reuse documents already parsed lazily or open.
  - Do not hold the workspace-index lock across I/O or parsing.
  - Reconcile file-list changes and buffer versions received during the
    build before publishing the snapshot.
  - Any unreadable or unparseable listed file fails the build and returns an
    error to waiters. Never produce partial rename edits.
  - Report `window/workDoneProgress` as startup does today, only when the
    client supports it.
- [x] **Entry points.** Gate find references, heading rename,
  `workspace/willRenameFiles`, and workspace symbols on the build. A rename
  that only checks the local target (prepare) does not build backlinks. Keep
  the interface behind the same feature entry points, so the index-store
  feature can replace the data source.
- [x] **Log lines (R8).** Emit:
  - the registry-ready line, with `roots`, the schema counts,
    `failed_triggers`, `failed_scopes`, and `workspace`;
  - the workspace-files-listed line, plus `workspace rescan complete`;
  - the full-index-built line with its `trigger`.
  - Remove `startup index complete documents=…`.
  - Counts follow the spec's counting rules (distinct files, shared
    directories counted once as tree-wide).

### Wave 12 (after Wave 11)

- [x] **R7 tests.**
  - First references, rename, and workspace-symbols requests each build once;
    rename edits include a never-opened, never-referenced referrer.
  - Simultaneous callers share one build.
  - Cancellation, unreadable inputs, edits and deletes during the build, and
    a workspace file rename cannot yield a partial or stale ready snapshot.
- [x] **R8 tests** with the captured subscriber and known fixture counts,
  including the failure variant of the registry line.
- [x] **Benchmark.** Confirm `dmls --bench-index` still measures the full
  index.

### Validation checkpoint 5

- [x] `just test` and `just lint` pass.
- [x] Every row of the spec's Tests table maps to a passing test (record the
  mapping in the implementation log).

## Phase 6: Documentation and Skill Sweep

Starts as soon as behavior stabilizes; drafting can overlap Phase 5.

### Wave 13 (parallel)

- [x] **New page `lsp/configuration.md`** under `darkmatter/docs/lsp/`: every
  `.dmls.toml` key and default, precedence (`.dmls.toml`, editor settings,
  defaults), and the `[workspace]` section. Absorb the configuration content
  from `topics/schemas/dmls-schema-support.md` and link to it from there.
  Written for a newcomer, with an example per rule.
- [x] **New page `lsp/indexing.md`**: the startup order and Mermaid diagram,
  registry and scopes, file list and ignore rules, on-demand parsing, the
  full-index features, and the registry-transition, startup, and slow-request
  log lines (including the counting rules and stage nesting).
- [x] **Sweep of existing pages.** Correct statements this fix makes untrue:
  - `lsp/architecture.md` (resolve table, paragraph below it, invalidation
    table, watch globs);
  - `topics/dmls.md`, `topics/schemas/definition.md` (R1 claim rule with
    example, R2 error), `topics/schemas/dmls-schema-support.md`,
    `topics/schemas/schema-activation.md`;
  - `lsp/how-to-use.md`, `lsp/features.md`;
  - `darkmatter/dmls/docs/` pages and `dmls/README.md` (link to
    `configuration.md`, one line on logs linking `indexing.md`);
  - `docs/research/` only where it states current behavior.
  - A `docs/` page never links to or names this fix.
- [x] **Source docs and skill.** Update `dmls/src/workspace/discover.rs`
  module docs and `.claude/skills/darkmatter/` (`dmls.md`, `schema.md`).
  Review `///` and `//` comments in every symbol changed in Phases 1-5, and
  report any comment/code drift found and how it was resolved.

### Wave 14 (after Wave 13)

- [x] **Link and kind check.** Verify links resolve (`md` link tooling) and
  that any kinded docs satisfy `schemas/kind.yaml`.
- [x] **Final gate.** Run `just test` and `just lint` in `darkmatter/`, and
  `just test-l2` only if any terminal-facing behavior changed. Use
  Sniff/GitNexus to find consumers of changed public library types (Claudine
  composes through the trigger path) and run their targeted tests.
- [x] **Implementation log.** Record departures from the spec, the smell-grep
  result, the spec-row-to-test mapping, and any rulings the author changed.
  Leave the spec as written. Do not move it to `_completed`.

### Validation checkpoint 6 (final)

- [x] All "Done means" items at the top are checked.
- [x] Hand off as "implementation complete, ready for review".
