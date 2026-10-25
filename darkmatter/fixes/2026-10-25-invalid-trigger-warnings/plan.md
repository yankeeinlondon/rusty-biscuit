---
total_phases: 6
created: 2026-10-25
phase: 1
agent: "opencode/zai-coding-plan/glm-5.3"
yolo: "true"
human_review: false
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/cli/tests/l1/schema_trigger_rename.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
---

# Implementation Plan — Warn and skip invalid schema triggers

Derived from the reviewed specification in `2026-10-25-invalid-trigger-warnings`
(status `finalized-spec`, reviewed, no open questions, `needs_rulings: false`).
Every file reference below is grounded in the current tree.

## Work Summary and Success Criteria

Today one invalid automatically discovered schema trigger aborts the whole
discovery scan — `scan_with_source` returns `Err(SchemaError::TriggerLoad)` and
every host fails: `md compose` fails with a schema-preparation error, `md clean`
and `md schema validate`/`triggers` exit nonzero, DMLS serves a last-good (or
no) registry, and Claudine renders a misleading fatal
`CompositionError: schema load failed` that blames the prompt's `$schema` even
when the prompt authored none. The incident fixture is a `schemas/memory.yaml`
with unsupported envelope keys (`name`, `description`) and a null `match:`.

This plan converts per-trigger envelope and payload failures into typed,
source-identified warnings (`dm.schema.invalid_trigger`, severity warning) at
the automatic discovery boundary in the darkmatter library, skips only the
offending trigger (filename shadowing unchanged), installs the valid subset,
and delivers the warnings through every existing consumer: ordinary `md`
output on stderr, `md compose --output report-json`, `md schema validate
--format json`, the `md schema triggers` trace, `md clean` (stderr plus an
additive top-level `warnings` array in the version-1 `--json` envelope), DMLS
per-scope warning diagnostics on trigger files with valid-subset registry
install and repair-driven refresh, and Claudine prompt preparation and launch
(delivered once per invocation, never as a fatal schema-load error). The code
joins composition's existing `--deny` policy so
`md compose --deny dm.schema.invalid_trigger` and `--deny all` can make it
blocking. Explicit document `$schema` load failures, validation failures under
valid schemas, baseline behavior, infrastructure errors (unreadable roots,
unreadable enumerated files, case-fold collisions, invalid embedded bundle
structure), and the strict passive parsers are all unchanged.

The work splits into six phases: (1) the darkmatter library recovery core —
typed finding record, per-trigger recovery in discovery, payload
validator-construction gate with failure observations, registry/report
exposure, clean-context and compose integration; (2) `md` CLI delivery across
compose, validate, triggers, and clean; (3) DMLS valid-subset install, warning
diagnostics, watch coverage, and editor behavior; (4) Claudine carry-through
across preliminary checks, launch, retries, and CLI delivery; (5)
documentation and skills; (6) end-to-end acceptance verification. Phases 2, 3,
and 4 depend only on Phase 1 and can proceed concurrently (Waves 6–7).

Successful completion looks like:

- Spec acceptance 1–10 all hold, demonstrated by tests: the `memory.yaml`
  fixture composes successfully through the normal CLI path with the warning on
  stderr and the composed document on stdout; a valid trigger beside an invalid
  one still applies (both filename orders); every classified envelope/payload
  failure shape skips exactly its trigger; all-invalid discovery continues with
  baseline and authored schema and still returns its warnings; infrastructure
  failures remain errors; passive parser direct calls still return their typed
  errors.
- One warning per skipped trigger per top-level invocation, in discovery
  order, deduplicated by full source identity (normalized disk identity or
  bundle/document pair) — across repeated schema passes, preflight,
  transclusion, Claudine retries, and multi-file validation — and reported
  again on a new invocation, including one reusing cached discovery. Findings
  are retained before independent schema/validation/composition failures and
  delivered with them.
- `--deny dm.schema.invalid_trigger` and `--deny all` stop composition through
  the existing policy path before the next effect, keeping warning severity and
  the trigger's own source; `--disable schema-triggers` performs no discovery
  and emits no warning; an authored `$schema` referencing the ignored trigger
  file still fails.
- DMLS installs the current valid subset (a formerly valid trigger saved in an
  invalid state loses its properties from validation/completion/hover while a
  valid neighbor's changes apply), publishes one warning diagnostic per trigger
  source (including outside-workspace files and prepared scopes with no open
  document), clears it only when no prepared scope still reports it, stays
  quiet on unchanged refreshes, and refreshes on envelope/payload repair,
  deletion, or creation of a missing payload or its parent folder.
- Claudine prepares and launches prompts with a fixture provider despite
  invalid triggers, delivering the warning once alongside existing host
  warnings; its preliminary schema checks (including ones that return no
  effective schema or reject the prompt) carry the findings; nothing renders
  them as a `CompositionError` or blames the prompt.
- `just test` and `just lint` pass in the `darkmatter/` and `claudine/`
  package areas; all new tests are L1 (no terminal or browser windows, no
  focus gain) and safe on macOS, Linux, native Windows, and WSL2; the passive
  shipped-schema corpus check passes; the planned marker is removed from user
  documentation and the topic pages, CLI references, DMLS docs, Claudine docs,
  and skills describe the landed behavior.

## Phase 1 — Darkmatter library: recovery at the discovery boundary

Convert per-trigger failures into typed findings in
`lib/src/markdown/schemas/triggers/` (discovery, assemble), expose them through
`DarkmatterSchemas`, `ValidationReport`, the clean schema context, and
composition, and register the diagnostic code with the policy catalog. Waves
1–5 below are sequential; each builds on the one before.

### Decisions

The spec's Design Decisions are settled and are implemented as written. The
decisions below close the remaining implementation gaps. Each is the rule to
follow, not a request for confirmation.

- **Finding record and its carrier**
  - **decision:** A new public typed record (working name `InvalidTriggerFinding`)
    holds `{ source: TriggerSource, cause: SchemaError, cause_path:
    Option<PathBuf>, position: Option<(line, column)> }` — the typed trigger
    source (disk path or embedded bundle/document), the first underlying typed
    cause, an optional related payload/import source, and the error position
    when the cause is `TriggerEnvelopeYaml`. It lives on
    `TriggerRegistry.findings` in discovery order, is exposed by
    `DarkmatterSchemas::discovery_findings()`, and is carried by
    `ValidationReport` (a dedicated `trigger_findings` collection populated
    even on the no-effective-schema paths) rather than being forced into the
    `SchemaAdvisory` enum. Projection into `ComposeWarning` follows the
    `from_schema_advisory` pattern (`compose/context/report.rs:711`).
  - **alternatives considered:** extend `SchemaAdvisory` with an
    `InvalidTrigger` kind (blocked: its `Eq`/`Hash`/`Ord` by `(kind, path)`
    cannot hold a typed `SchemaError` cause or an embedded identity); log a
    string; return findings only through `EffectiveSchema`.
  - **why:** the registry owns discovery state; `ValidationReport` needs the
    findings on paths where no `EffectiveSchema` exists (`mod.rs:661`,
    `mod.rs:807-813`, `mod.rs:849-855`); the advisory set's dedup semantics do
    not fit a cause-carrying record.
  - **downsides:** one more collection to thread through
    `ValidationReport`/compose projections; the advisory JSON shape and the
    finding shape must be mapped at the CLI.
- **Error taxonomy at the boundary (spec-settled, restated as rule)**
  - **decision:** Recoverable (finding + skip, scan continues): envelope
    classification errors (malformed claimed YAML, unknown keys, missing/null
    `match`, invalid condition grammar, vacuous conditions, null/non-string
    `kind`, renamed spellings), payload resolution/parsing/compilation/import/
    merge-compatibility failures including missing files and cycles, and
    invalid discovered embedded triggers. Hard errors (unchanged): root
    enumeration and unreadable-root failures, reading an enumerated file
    before classification, case-fold collisions, invalid embedded bundle
    structure, explicit baseline failures, and authored document `$schema`
    failures. Passive parsers stay strict; recovery exists only at the
    discovery boundary.
  - **alternatives considered:** relax the parsers; recover only in Claudine;
    recover only envelope errors.
  - **why:** spec "Recover at the automatic discovery boundary"; shared
    recovery gives every host the same valid registry without hiding broken
    roots or weakening explicit enforcement.
  - **downsides:** callers must distinguish findings from infrastructure
    errors — the `Result` type keeps doing that (errors stay `Err`).
- **Validator-construction gate**
  - **decision:** Every candidate payload passes validator construction
    through the shared validator cache (`SHARED_VALIDATOR_CACHE` /
    `validator_for_shared`, `schemas/validate.rs:252-260`) during payload
    resolution, before the trigger installs — for disk and embedded payloads,
    matched or not. A payload that passes `validate_simple_object_schema`'s
    shape gate (including raw JSON Schema without `$defs`) but fails to
    compile becomes a finding via a new dedicated `SchemaError` variant
    (working name `TriggerPayloadCompile { path, source }`) so the warning says
    "failed to compile", not "not mergeable". The cache prevents recompiling
    accepted payloads on every document pass.
  - **alternatives considered:** gate at final merged-schema compilation
    (rejected by spec: a later layer can hide the bad property); compile
    matched triggers only (rejected: registries must never hold known-bad
    payloads); reuse `TriggerPayloadNotMergeable` for compile failures
    (inaccurate message).
  - **why:** spec "Check every candidate's payload through validator
    construction before installing it"; the existing cache satisfies "do not
    compile every accepted payload again on every document pass".
  - **downsides:** cold-load work per payload; acceptable per spec (no timing
    target, cache reuse required).
- **Failure observations for refresh**
  - **decision:** Payload resolution surfaces its partial observations when it
    fails: attempted lookup locations (the existing `searched` vocabulary) and
    successfully read dependency files are returned alongside the error (e.g.
    the resolution error carries them, or resolution reports an observation
    bag out-of-band). `TriggerRegistry::dependencies()` includes each skipped
    envelope (an existing file, hash-tracked) plus observed read dependencies;
    `searched_locations()` includes probed locations from failed resolutions
    (missing locations stay lookup observations, never existing-file
    dependencies). The payload memo (`file_payload_memo`) continues to cache
    successes only; failures and their findings are recomputed per scan —
    display dedup happens per invocation at delivery, never by a memo or a
    process-global "already displayed" bit.
  - **alternatives considered:** watch only accepted triggers; approximate
    failure coverage with folder-level watches only; memoize failures.
  - **why:** spec "Observe failed loads so repairs can recover" — repairing an
    envelope, repairing a payload/import, or creating a missing file or its
    parent folder must refresh through existing cache checks and file events
    (DMLS `scope_affected`/`Dependencies.searched`/`absent_bases` already
    consume exactly these inputs).
  - **downsides:** resolution must thread an observation collector through
    error paths; a recursive-search creation still needs the existing rescan
    limits (unchanged).
- **Diagnostic code, severity, identity, and policy registration**
  - **decision:** Code `dm.schema.invalid_trigger`, severity warning, source
    family `darkmatter.schema`, constant placed with the existing
    `dm.schema.*` constants (`schemas/mod.rs:1742-1753`) and registered in
    `COMPOSE_DIAGNOSTIC_CODES` (`compose/policy.rs:51-70`). Dedup identity is
    the full trigger source: the existing normalized disk identity for files,
    the `(bundle, document)` pair for embedded triggers — never a filename
    alone. `ComposeWarning`'s `WarningIdentity` subject uses the path for disk
    sources and a non-path identity variant for embedded sources (extend
    `WarningSubject` if needed; an embedded identity must never become a local
    file link, watch, or URI).
  - **alternatives considered:** reuse `dm.schema.prepare` (an error code);
    dedup by payload; string-log without a code.
  - **why:** spec "Keep a typed warning with the trigger's identity";
    composition's policy catalog governs every coded warning and powers
    `--deny` with zero new policy machinery.
  - **downsides:** one more code in the catalog and help lists (derived
    automatically from `deny_codes()`).
- **Embedded trigger claiming and identity**
  - **decision:** In `append_embedded_bundle`, the first document classified
    as a trigger or invalid trigger reserves its name before payload loading,
    even when it is skipped; a later bundle never stands in for it. Invalid
    embedded triggers produce findings keyed by bundle name + document name,
    displayed as `embedded:{bundle}/{name}` (the existing
    `TriggerSource::Embedded` display). Ordinary embedded import documents and
    `EmbeddedBundle` structure errors keep their current behavior. The
    embedded memo still caches successes only.
  - **alternatives considered:** claim names only for successfully loaded
    triggers (rejected: shadowing must not activate a fallback); memoize
    failures.
  - **why:** spec "Between embedded bundles, the first document classified as
    a trigger or invalid trigger reserves its name before payload loading,
    even when it is skipped."
  - **downsides:** a broken nearer embedded trigger suppresses a valid
    same-named one until repaired (same trade-off as disk shadowing).
- **CLI stream and report policy**
  - **decision:** Ordinary CLI output writes discovery warnings to stderr via
    the established components (`warning_rows`/`Status` for compose,
    `report_section`/`Prose` + `UnorderedList` for clean) and keeps stdout for
    normal output; structured report modes (`report-json`, validate `--format
    json`, `clean --json`) carry warnings in their structures with no repeated
    stderr row. In `md schema validate` pretty mode, discovery findings render
    on stderr (a new, small renderer); per-file advisory bullets keep their
    existing behavior. `md schema triggers` lists skipped triggers with source
    and reason inline in its stdout trace (inspection is its normal output).
    `md clean --json` adds a top-level `warnings` array with rows shaped like
    validate's warning rows (`{source, code, path, message}`, `path` null for
    embedded), keeping the version-1 envelope and `diagnostics` untouched.
  - **alternatives considered:** warnings on stdout in ordinary mode; repeat
    warnings on stderr in report mode; extend `diagnostics` spans to describe
    triggers (rejected by spec: document-span entries cannot describe a
    trigger source).
  - **why:** spec "Preserve report streams and explicit warning policy" and
    the per-command delivery rules; consistency with every existing warning
    family.
  - **downsides:** validate pretty gains a second warning surface (stdout
    bullets for file advisories, stderr for discovery findings).
- **DMLS registry replacement**
  - **decision:** A scan that succeeds with findings installs its valid
    subset; the whole-scope `TriggerFailed`/last-good retention is removed for
    individually invalid triggers (infrastructure `RootFailed` keeps failing
    closed). Per-scope findings are retained independently; a source's warning
    diagnostic is published once and cleared only when no prepared scope still
    reports it. Direct authoring checks of an open trigger file stay strict
    (`SuggestionState::TriggerError`) and never fail document preparation.
  - **alternatives considered:** keep the previous whole registry; keep only
    the broken trigger's previous version; drop every trigger on one failure
    (all rejected by spec).
  - **why:** spec "Replace DMLS's previous registry with current valid
    triggers"; the CLI and editor must agree that "ignored" means ignored.
  - **downsides:** a temporarily broken saved trigger loses editor assistance
    until repaired; diagnostic ownership must handle shared sources across
    scopes (the design above does).
- **Claudine carry, not catch**
  - **decision:** `load_effective_schema` returns findings alongside
    `Option<EffectiveSchema>` (outcome struct or equivalent) so no caller can
    mistake them for load failures; the `schema_error` closure and
    `translate.rs` stay untouched for real errors. Every preliminary and
    launch path retains findings — including `drop_invalid_optionals_with_origin`'s
    early return and checks that yield no effective schema or reject the
    prompt — and delivery happens once per top-level invocation (compose
    routes via `PreparedComposition.warnings` + `emit_compose_warnings`; the
    wrap launch runner renders them alongside existing host warnings), deduped
    by trigger source identity. No Claudine-only catch-and-ignore path.
  - **alternatives considered:** swallow findings in Claudine; print during
    loading; per-retry repeated display.
  - **why:** spec implementation-surfaces table and "Deliver findings even
    without a successful effective schema".
  - **downsides:** Claudine's schema-loading signature and warning plumbing
    widen; the runner gains its first compose-warning rendering.
- **No performance spike**
  - **decision:** None. The spec states no performance study is required and
    mandates validator-cache reuse; the only new cold-load cost (validator
    construction per payload) is bounded by that cache and covered by a
    cache-reuse test rather than a measurement.
  - **alternatives considered:** a cold-load timing spike on one host.
  - **why:** the spec's own figures and design answer the question a spike
    would ask.
  - **downsides:** no timing baseline is produced (none requested).
- **Input robustness**
  - **decision:** No new parser or field validation is added, so no new
    robustness matrix is written: the existing envelope classifier's shape
    handling (absent/null/wrong-type `kind`, unknown keys, malformed claimed
    YAML, vacuous `match`) is reused verbatim and stays strict. The default
    outcome for every classified error shape is one located finding (trigger
    source + reason) and a skip; the only exceptions are proven once each in
    tests: `Ok(None)` shapes stay silently ignored (including malformed
    unclaimed YAML), root/file-read/case-fold/bundle-structure failures stay
    hard errors, and shadowed files stay unread and warning-free. The new
    `warnings` array in `md clean --json` is additive with no new parsed
    fields anywhere, so no serde smell surface is added (the grep check in
    Phase 6 still runs).
  - **alternatives considered:** a full per-field matrix for the envelope
    (already owned and tested by the classifier).
  - **why:** the matrix section targets new config/file-format readers; this
    fix changes ownership of already-validated outcomes, not validation.
  - **downsides:** none; classifier tests must stay green unchanged.

### Wave 1 — Typed finding record and code (foundation)

- [x] **Finding record**
    - Add `InvalidTriggerFinding` (fields per the decision above) in the
      schemas module tree with `code()` → `dm.schema.invalid_trigger`,
      `source_family()` → `darkmatter.schema`, a `message()` that names the
      trigger source, the first cause's reason, and the fact that it was
      ignored (modeled on the spec's example text), and identity accessors
      (normalized disk identity or `(bundle, document)`).
    - Place the code constant with the existing `dm.schema.*` constants
      (`schemas/mod.rs:1742-1753`) and add it to `COMPOSE_DIAGNOSTIC_CODES`
      (`compose/policy.rs:51-70`); keep the catalog's uniqueness/prefix tests
      green (`policy.rs:186-194`) and the CLI cross-check
      (`cli policy.rs:607-613`) green.
    - Derive ordering by discovery sequence; provide equality/identity by full
      source identity (not filename) for dedup use by hosts.

### Wave 2 — Discovery recovery (envelopes, disk and embedded)

- [x] **Envelope recovery**
    - In `scan_with_source` (`triggers/discovery.rs:574-657`): an envelope
      classification error for an unshadowed file records a finding
      (`TriggerSource::File`, typed cause, envelope position when the cause is
      `TriggerEnvelopeYaml`), skips that trigger, and continues; the file
      keeps its claimed name (shadowing unchanged — verify `first_by_name`/
      `claimed_names` still reserve it); valid triggers keep exact ordering;
      all-invalid yields `Ok(registry)` with no triggers and the findings.
    - Keep hard errors hard: `source.root_files`/`root_file_text` failures
      (`SchemaError::Io`), `TriggerCaseFoldCollision`, `SchemaRoots::for_document`
      failures.
    - In `append_embedded_bundle` (`triggers/discovery.rs:209-244`) and
      `load_embedded_trigger` (`triggers/discovery.rs:315-364`): an invalid
      embedded trigger (envelope error or payload failure) records a finding
      keyed by bundle + document, is skipped, and still claims its name before
      payload loading; `bundle.check()`/`EmbeddedBundle` structure failures
      remain errors; the memo keeps successes only.
    - Update the discovery-contract module docs (`triggers/discovery.rs:9-35`,
      `scan_with_source` doc, `schemas/mod.rs:509-513`) from "transactional"
      to per-trigger recovery in the same change (comment-drift rule); list
      the findings type and the remaining hard-error taxonomy.
    - Payload resolution failures inside `scan_with_source`'s payload loop and
      `append_embedded_bundle` are converted to findings in this wave only if
      Wave 3's observation plumbing is not yet needed to compile; otherwise
      they land in Wave 3 (keep the tree green per wave).

### Wave 3 — Payload gate and failure observations

- [x] **Payload validator gate**
    - In `resolve_file_payload` / `resolve_embedded_payload`
      (`triggers/assemble.rs:297-339`, `725-787`), after
      `enforce_merge_compatible`, run validator construction through the
      shared validator cache; a compile failure maps to the new
      `TriggerPayloadCompile`-style `SchemaError` variant carrying the trigger
      source and the underlying compile error, which discovery converts to a
      finding with the payload/import path as `cause_path` when available.
    - Cover raw JSON Schema without `$defs` that passes
      `validate_simple_object_schema` (`resolve.rs:3877-3895`) but fails
      compilation; gate regardless of matching or later overrides.
    - Keep cycle detection (`check_payload_cycles`) and merge-compatibility
      errors flowing to findings with the same envelope attribution.
- [x] **Failure observations**
    - Extend payload resolution so a failing resolution still reports probed
      lookup locations (`searched`) and successfully read dependencies; wire
      them into `TriggerRegistry` next to the findings: skipped envelopes and
      observed read dependencies join `dependencies()`, probed locations
      (including missing ones) join `searched_locations()`.
    - Leave `file_payload_memo` semantics for successes untouched
      (fingerprint, LRU, capacity); failures stay uncached.
    - Ensure repairing/deleting an envelope, repairing a payload/import, or
      creating a missing file (or its parent folder) changes the registry
      inputs the existing freshness checks consume (dependencies hashes,
      folder listings, searched candidate kinds) — verified through
      `TriggerRegistry` accessors in tests this wave and through DMLS in
      Phase 3.

### Wave 4 — Registry exposure and clean context (concurrent)

- [x] **Schema-surface exposure**
    - `TriggerRegistry` gains `findings` (ordered) and exposes them;
      `DarkmatterSchemas::with_trigger_discovery` (`schemas/mod.rs:523-527`)
      returns `Err` only for infrastructure errors and installs the registry
      with findings; add `discovery_findings()`.
    - Deliver findings on the no-schema paths: `effective_for_with_override`'s
      `(None, true, None)` branch (`mod.rs:661`), `validate`
      (`mod.rs:800-815`) and `validate_for_phase` (`mod.rs:836-857`) vacuous
      reports, and `PreparedSchemas::effective_for`
      (`compose/schema_validation.rs:203-210`) no longer flatten them away;
      `ValidationReport` carries `trigger_findings` alongside `advisories`.
- [x] **Clean schema context**
    - `CleanSchemaConfig::resolve` (`schemas/clean.rs:153-171`) stops aborting
      on per-trigger failures; `CleanSchemaContext`/`SchemaCleanAnalysis`
      expose the findings (including the `Ok(None)` effective-schema branch at
      `clean.rs:222-224`) so `md clean` and DMLS formatting can consume them;
      `trigger_schemas: false` still performs no discovery and yields no
      findings.

### Wave 5 — Compose integration and library tests (concurrent)

- [x] **Compose integration**
    - `prepare_schemas` (`compose/schema_validation.rs:252-316`) no longer
      fails for per-trigger findings; project each finding into the compose
      report as a coded `ComposeWarning` (stage `schema_validation`, code
      `dm.schema.invalid_trigger`, warning severity, path = trigger file for
      disk sources, non-path `WarningIdentity` subject for embedded) so
      ordinary mode renders it through `warning_rows`
      (`cli compose.rs:884-893`) and `report-json` emits a diagnostic row via
      `warning_diagnostic` with no stderr repetition.
    - Deny policy: with `--deny dm.schema.invalid_trigger`/`--deny all`, the
      existing `stop_if_blocked`/`blocking_guard` machinery
      (`compose/pipeline/mod.rs:740-855`) stops composition before its next
      effect — including during preflight and child composition — keeping the
      warning's severity and the trigger's own source (never a schema-load
      error, never prompt attribution).
    - Retention and dedup: findings are added before any later stage so
      existing carriers (`compose_with_warnings`,
      `take_preflight_failure_warnings`, `earlier_warnings`) deliver them when
      authored-schema resolution, validation, or composition later fails;
      `WarningIdentity` dedup keeps one row per source across repeated schema
      passes and transclusion children.
- [x] **Library recovery tests**
    - Update the transactional tests in `triggers/discovery.rs:995-1053,
      1191-1263` (`malformed_trigger_is_hard_error`,
      `malformed_trigger_atomic_no_registry`, `vacuous_trigger_is_hard_error`,
      `unmatched_trigger_with_missing_payload_is_hard_error`,
      `payload_resolution_failure_is_transactional`) to the recovered
      contract: skip + finding + valid neighbors installed.
    - Add the recovery matrix at library level: the `memory.yaml` incident
      shape (unknown keys + null `match`); malformed claimed YAML; every
      envelope classification error; missing payload files, cycles,
      non-mergeable and non-compiling payloads (including raw JSON Schema
      without `$defs` that a later declaration would override); invalid
      embedded triggers identified by bundle/document; all-invalid discovery
      with no baseline/authored schema returning findings and no effective
      schema; shadowing after a skip across roots and between bundles;
      distinct paths sharing a filename and two triggers sharing a broken
      payload stay distinct; discovery-order emission and first-cause-wins.
    - Assert infrastructure errors still fail scans; assert direct passive
      parser calls (`parse_trigger_envelope_from_str`,
      `resolve_trigger_payload`) still return their typed errors; assert
      skipped-envelope and failure observations appear in
      `dependencies()`/`searched_locations()`; add a validator-cache reuse
      test (payload not recompiled per pass — reuse the memo/tally pattern).
    - Follow the test-input spelling rules and L1 discipline from the
      `rust-testing` skill; isolated repo fixtures with controlled home and
      schema roots; no focus-gaining windows.

### Validation — Phase 1

- `just test` and `just lint` in `darkmatter/` pass; specifically the
  discovery, assemble, advisory, policy-catalog, schema-validation, and clean
  suites are green with the updated contracts.
- A library-level test composes/validates a document in a fixture repository
  containing the incident `schemas/memory.yaml` and observes: success, the
  finding on the result (report/registry), and no trigger installed.

## Phase 2 — `md` CLI warning delivery

Depends only on Phase 1. All four tasks are independent of each other and of
Phases 3–4; they form Wave 6 together with the first tasks of Phases 3 and 4.

### Wave 6 — CLI tasks (concurrent with Phase 3 Wave 6 and Phase 4 Wave 6)

- [ ] **Compose delivery**
    - Verify through the normal invocation path: ordinary compose of a
      `$schema`-less document beside the invalid trigger succeeds, stdout
      carries the composed document, stderr carries exactly one warning row
      naming the trigger and reason; `--output report-json` emits the
      diagnostic row with `dm.schema.invalid_trigger` and writes no warning to
      stderr; warnings are retained and delivered when an independent
      schema/validation/composition failure occurs; `--deny
      dm.schema.invalid_trigger` and `--deny all` stop composition before the
      next effect with the warning's own severity and source;
      `--disable schema-triggers` emits nothing; an authored `$schema`
      referencing the ignored trigger file still fails with the existing
      error.
    - Add/adjust CLI L1 tests (isolated repository fixtures) for each bullet;
      reuse `CliProcessFixture` conventions from the testing skill.
- [ ] **Schema validate**
    - `validate_one` (`cli/src/commands/schema/validate.rs:236-243`) no longer
      maps per-trigger failures to `FileOutcome::SchemaError`; findings ride
      the outcome: `--format json` adds warning rows to the existing
      `warnings` collection (shape `{source, code, path, message}`, `path`
      null for embedded) on the file that discovered them, and failure
      outcomes (validation failures) retain earlier discovery warnings;
      existing outcome fields and exit statuses (0/1/2/3/64) are unchanged —
      exit 2 remains for real schema/baseline load failures only.
    - Pretty mode renders discovery findings on stderr (new small renderer
      using the established components); per-file advisory bullets are
      untouched.
    - Multi-file invocations: a shared trigger warning is attached to the
      first input that discovers it, once per invocation (dedup by source
      identity at the command boundary).
    - Assignment preparation (`validate.rs:245-269`, `assignment.rs`) works
      against the recovered registry, including re-resolution after
      assignments; add a test with an invalid trigger beside a valid matching
      one (both filename orders per spec acceptance 2).
- [ ] **Schema triggers**
    - `run_triggers` (`cli/src/commands/schema/triggers.rs:33`) no longer
      aborts on invalid triggers; the stdout trace gains a skipped-triggers
      section listing each finding's source and reason inline (inspection
      output), with existing sections unchanged; add a test.
- [ ] **md clean**
    - Ordinary mode: the discovery warning is written to stderr via the
      existing `report_section` helpers (`cli/src/commands/clean.rs:312-348`);
      cleaning succeeds beside an invalid trigger; findings never affect the
      exit code.
    - `--json`: add the top-level `warnings` array to `CleanJsonReport`
      (`cli/src/commands/clean/json.rs:14-24`) with rows shaped like
      validate's warning rows; version stays 1; `diagnostics`, `applied`,
      `changed` and all other fields unchanged; stdout carries the envelope
      with clean stderr (the JSON path currently returns before
      `report_findings` — keep it that way so report mode never repeats to
      stderr).
    - `--disable schema-triggers` (existing `no_schema_triggers` wiring,
      `clean.rs:302`) emits no warning; add tests for all three bullets.

### Validation — Phase 2

- `just test` and `just lint` in `darkmatter/` pass with the new CLI tests;
  `md compose`, `md schema validate`, `md schema triggers`, and `md clean`
  each demonstrate warn-and-skip through their normal invocation paths in L1
  tests with isolated fixtures.

## Phase 3 — DMLS: valid subsets, warning diagnostics, refresh

Depends only on Phase 1. Wave 6 installs the recovered registry; Wave 7
builds on it and is concurrent with Phase 4's Wave 7.

### Wave 6 — Registry install

- [ ] **Valid-subset install**
    - `Preparation::run`/`TriggerRegistries::install`
      (`dmls/src/overlay/registry.rs:424-469`, `581-638`): a scan that
      succeeds with findings installs the valid subset (registry +
      findings) — the whole-scope `TriggerFailed`/last-good retention is
      removed for individually invalid triggers; infrastructure
      (`RootFailed`) behavior is unchanged; first-load all-invalid discovery
      installs an empty-but-ready registry with findings (no previous
      registry exists to retain).
    - Scope outcomes carry the findings; `lookup` serves the installed
      registry so validation, completion, and hover use the current valid
      triggers (a formerly valid trigger saved invalid loses its properties).
    - Dependencies and watches: skipped envelope files become hash-tracked
      dependencies; failure observations (searched candidates, absent bases)
      flow into `Dependencies` (`registry.rs:298-359`) so
      `invalidate_paths`/`scope_affected` (`registry.rs:713-726`,
      `842-866`) and `schema_watch_registration`
      (`dmls/src/workspace/watch.rs:144-169`) cover repair of envelopes,
      payloads, imports, and creation of missing files or parent folders;
      existing recursive-search refresh limits and rescan behavior are
      preserved; `RootFileClass::FailedTrigger` classification stays for
      authoring surfaces.

### Wave 7 — Diagnostics, editor behavior, tests (concurrent with Phase 4 Wave 7)

- [ ] **Warning diagnostics**
    - Publish each finding as a warning diagnostic on the disk trigger file
      (severity Warning, code `dm.schema.invalid_trigger`, source
      `darkmatter.schema`): file-level range with the established fallback
      (envelope position when the cause provides one), cause locations as
      related information when available; extend the existing
      transitions/publication path (`record_failure`/`take_transitions`/
      `publish_trigger_transitions`, `registry.rs:666-751`,
      `router.rs:1086-1110`) from one record per scope to per-finding records.
    - Scope independence: keep each prepared scope's findings independently;
      publish each source warning once; a healthy scope must not clear a
      warning another scope still reports; clear a source's diagnostic when it
      is repaired, deleted, or no longer contributes a finding to any prepared
      scope; repeated unchanged refreshes repeat neither logs nor
      publications.
    - Embedded findings render as text identities only — never a file URI,
      link, or watch; publish for prepared scopes with no open document
      (workspace probes) and for trigger files outside the workspace; protocol
      stdout stays clean (LSP notifications only).
- [ ] **Editor features and formatting**
    - Verify validation, completion, and hover reflect the installed valid
      subset and change with valid neighbors while an invalid trigger is
      skipped (mostly inherited from install; add explicit coverage).
    - Formatting (`dmls/src/providers/formatting.rs:42-64`): formats despite
      skipped triggers, applies its edits, and adds no warning message of its
      own (the registry publishes the warning on the trigger file); direct
      authoring checks of an open trigger stay strict and never fail document
      preparation.
- [ ] **DMLS test suite**
    - Update the last-good/transactional registry tests
      (`registry.rs:902-1681`; e.g.
      `a_malformed_nearer_candidate_keeps_last_good_and_its_watch_until_repaired`,
      `trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope`
      in `tests/l1/lsp_session.rs:574-661`) to the valid-subset contract.
    - Add per spec acceptance 8: formerly valid trigger saved invalid while a
      valid neighbor changes (properties drop, neighbor's changes appear);
      first-load all-invalid; multiple simultaneous warnings; one source
      shared by scopes published once and cleared only when no scope reports
      it; prepared scopes without open documents; repair/deletion clearing;
      unchanged refresh quiet; strict diagnostics preserved while directly
      editing the trigger. Use the existing harness
      (`tests/common/mod.rs` — `LspWorkspace`, diagnostics waits, registry
      gates); no focus-gaining windows.

### Validation — Phase 3

- `just test` and `just lint` in `darkmatter/` (dmls included) pass; the LSP
  session tests demonstrate install-of-valid-subset, publish-once/clear-on-
  repair warning diagnostics, quiet repeated refreshes, and formatting
  passthrough.

## Phase 4 — Claudine: carry findings through preparation and launch

Depends only on Phase 1. Wave 6 is the library carry; Wave 7 (concurrent with
Phase 3 Wave 7) is delivery and tests.

### Wave 6 — Library carry

- [ ] **Findings through schema loading**
    - `load_effective_schema` (`claudine/lib/src/composition/schema/mod.rs:
      553-581`) returns the findings alongside `Option<EffectiveSchema>`
      (outcome struct or equivalent) — the `schema_error` closure and
      `schema_error_to_composition_error` (`translate.rs:4-44`) are untouched
      and never see recoverable findings; embedded-bundle registration
      (`bundle.rs`) keeps working with invalid embedded triggers reported as
      findings (no disk fallback; shadowing preserved).
    - Retain findings through every consumer: `pre_validate_verdict`
      (`mod.rs:803-955`, including the no-effective-schema and
      prompt-rejection outcomes), `drop_invalid_optionals_with_origin`
      (`mod.rs:1052-1178` — the `Ok(None) | Err(_) => return` early exit must
      still carry findings), `resolve_launch_schema`
      (`prepare.rs:511-532`), the drop-and-retry legs
      (`translate.rs:123-230`), interactive collection re-runs, and
      status-report reloads.
    - Accumulate across repeated preparation inside one top-level invocation
      (dedup by full source identity, first observed cause wins, discovery
      order); no process-global display state.

### Wave 7 — Delivery and tests (concurrent with Phase 3 Wave 7)

- [ ] **Warning delivery**
    - Compose/inline-compose/sequence routes: findings flow into
      `PreparedComposition.warnings` (`types.rs:876-878`) and render through
      `emit_compose_warnings` (`cli/src/commands/compose/loop_run.rs:33-52`).
    - Wrap launch runner (`cli/src/commands/wrap/composition/runner.rs`):
      render the schema findings once per invocation alongside existing host
      warnings (existing `log::warn`/warning helpers), including across
      retries, resumes, and harness-loop re-reads; never as a
      `CompositionError`, never attributed to the prompt, never repeated per
      retry.
    - The existing `CompositionError::SchemaLoad` rendering
      (`error/render/schema.rs:15-33`) remains exactly as-is for real
      schema-load failures.
- [ ] **Claudine test suite**
    - Flip `malformed_disk_trigger_fails_without_embedded_fallback` and the
      import-failure tests
      (`claudine/lib/tests/l1/claudine_schemas_embedded.rs:870-916`) to
      warn-and-skip with shadowing still preventing embedded fallback.
    - Add: prompt preparation/launch with a fixture provider succeeds beside
      the incident trigger with the warning delivered once; preliminary
      checks that return no effective schema still deliver findings; findings
      retained before an independent schema/composition failure; repeated
      preparation displays once; an authored `$schema` referencing the ignored
      trigger file still fails; no test launches a real provider.

### Validation — Phase 4

- `just test` and `just lint` in `claudine/` pass; the fixture-provider
  launch test demonstrates the incident resolved end to end through
  Claudine's normal path.

## Phase 5 — Documentation, skills, and drift

Wave 8. The two tasks are independent and concurrent. Documentation is the
current record: write the landed behavior in full, remove the planned marker,
and keep READMEs untouched unless their overview promises the old behavior
(verify both areas' READMEs; command references stay in `docs/`).

### Wave 8 — Docs and skills (concurrent)

- [ ] **Darkmatter documentation**
    - `docs/topics/schemas/schema-triggers.md`: replace the "Invalid
      discovered triggers (planned)" section (lines 14-27) with the current
      behavior — recovery, warning identity and streams, shadowing, embedded
      triggers, `--deny`, and repair refresh; update the "malformed discovered
      payload remains an error" sentence (line 85-86) to the recovered
      contract.
    - `docs/topics/schemas/definition.md`: update the discovery-contract
      passage around line 4110 ("Discovery is transactional") and the
      "Repository Trigger Schemas" section; `docs/topics/schemas/
      schema-targeting.md`: any failure-behavior claims.
    - `docs/cli/policy.md` (deny-codes list, ~94-98) and
      `docs/cli/report-json.md` (codes section, ~171-195): add
      `dm.schema.invalid_trigger` and the multi-file/first-input delivery
      rule where those pages list codes; `docs/cli/clean.md`: document the
      `warnings` array in the `--json` envelope.
    - DMLS docs: `docs/lsp/indexing.md` and `darkmatter/dmls/docs/
      diagnostics.md` — valid-subset install, warning diagnostics with
      publish-once/clear semantics, refresh-on-repair, and the unchanged
      strict authoring checks.
    - Skills: `.opencode/skill/darkmatter/schema.md` (trigger recovery,
      findings, hosts) and any trigger bullet in the darkmatter skill's
      SKILL.md; keep the skill's authority links pointing at the updated
      topic pages.
- [ ] **Claudine documentation and skills**
    - `claudine/docs/topics/schemas.md`: rewrite the "a malformed disk trigger
      fails the launch visibly" sentence (~1003) and the discovery-flow
      section (~938-1009) for warn-and-skip; keep "$schema pointing at a
      trigger remains an error".
    - `claudine/docs/topics/composition.md` (prepare-time warnings ~811-816;
      trigger-schema rules ~984-1062), `execution-flow.md` (~245-246, ~256,
      inline leg ~513-516), and `frontmatter-properties.md` (~10, ~22):
      describe findings carried through preliminary checks and launch,
      delivered once per invocation.
    - Skills: `.opencode/skill/claudine/SKILL.md` (~94-95, ~390) and
      `.opencode/skill/claudine/cli-commands.md` (~80) — the schema-bundle
      row's "an invalid one fails the launch" becomes warn-and-skip with
      valid-subset behavior.

### Validation — Phase 5

- Every behavior change from Phases 1–4 is described in the area `docs/`; no
  `docs/` page still describes whole-scan failure or last-good registries for
  individually invalid triggers; the planned marker is gone; `docs/
  dependencies.md` files need no changes (no crates added or removed).

## Phase 6 — End-to-end acceptance verification

Wave 9. Walks the spec's acceptance list against the finished tree.

### Wave 9 — Acceptance walkthrough

- [ ] **Acceptance checklist**
    - Walk spec acceptance items 1–10 in order with isolated repository
      fixtures (controlled home/schema roots; never the author's incident
      file; fixture provider only): the incident reproduction through the
      normal CLI path; invalid-beside-valid in both filename orders with
      validation still enforced; the full failure-shape matrix with passive
      parser strictness; all-invalid, matcher-only, ignored ordinary YAML,
      and unchanged shadowing across roots and bundles; cache/refresh,
      repair, deletion, and distinctness cases; host coverage (`md schema
      validate`, `md schema triggers`, Claudine preparation/launch, DMLS,
      assignment preparation, existing composition-consuming commands,
      multi-file reports, cached-discovery re-invocation); explicit `$schema`
      enforcement and the passive shipped-schema corpus check; the DMLS
      editor scenarios; `--deny`, `--disable`, `md clean`, and DMLS
      formatting rows.
    - Where an item is already covered by a Phase 1–5 test, cite it; add the
      missing ones rather than re-testing covered behavior.
- [ ] **Final gates and hygiene**
    - `just test` and `just lint` in `darkmatter/` and `claudine/`; confirm
      all new tests are L1 with no terminal/browser windows or focus gain and
      no permission-bit-dependent additions outside the existing unix-gated
      patterns (Windows/WSL2 safety); confirm `docs/dependencies.md`
      untouched; grep the code smells from the input-robustness rule over the
      diff (no new `#[serde(default)]`, `Option<T>` absence conflation,
      `filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on
      load-bearing parsing — expected clean since no new parsing was added);
      verify every behavior-changing edit carried its doc/comment update
      (comment-drift rule) and that no README overview still promises the old
      behavior.

### Validation — Phase 6

- All acceptance items pass or cite a passing test; both package areas are
  green on `just test` and `just lint`; the plan is fully checked off and the
  fix is ready for review (the author, not the agent, moves it to
  `_completed`).
