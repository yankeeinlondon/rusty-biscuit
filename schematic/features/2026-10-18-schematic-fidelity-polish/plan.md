---
total_phases: 13
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Schematic fidelity and polish implementation plan

## Work summary and success criteria

Make the external API artifacts faithful to the definitions: recover OpenAPI
schema constraints, import the supported Schematic metadata, produce valid
Postman examples, and export reversible AsyncAPI 3.0 documents. Consolidate API
registration and module naming before generating module features. Compile the
documentation, verify upstream hygiene, and assemble the closing evidence.

Success means R1–R58 have evidence or an explicitly reasoned deviation in
`implementation-log.md`; fresh artifacts are deterministic and drift-free;
default and selected-feature clients compile; documentation examples compile;
and the eight dependent packages build and pass tests without source or manifest
changes. The evidence must distinguish actual passes, unexercised environments,
accepted gaps, and open prerequisites. The terminal state is **implementation
complete, ready for review**, with whole-work closure separately evaluated.

This is a **provisional execution plan**. On 2026-10-11, all seven prerequisite
specifications (1, 2, 3, 4a, 4b, 5, 6) say `implemented: false`; their feature
directories contain no implementation logs. Their reviewed specifications are
design input, not proof that their contracts exist. Planning can finish now;
implementation cannot pass Phase 1 until the dependency gate and contract refresh
are satisfied. Do not implement their work inside this feature or silently adapt
this plan to the legacy runtime.

Each phase below has a completion checkpoint. Within a phase, a **wave** is an
execution step: tasks in the same wave may run concurrently only when their
write ownership is disjoint. Wave numbers are unique across the whole plan;
waves and phases execute in order. A single-task wave is sequential. An
integrator owns shared manifests, justfiles, test roots, regeneration, and
committed artifacts; subagents propose edits there rather than racing writes.
Every subagent brief must state that the entire session tree is non-interactive.
This plan describes delegation opportunities; creating the plan requires no
subagents.

All code phases update relevant comments, topic pages, README sections, and skills
alongside behavior (R48, R49); Phase 12 audits those updates rather than deferring
them. Every completed phase runs `just test` and `just lint` from `schematic/`.
When generation changes, regenerate through the recipes and run
`just lint-schema`; when artifact output changes, run its L1 drift coverage.
Record commands, environment, results, changed files, and decisions in the log.
Use focused tests during development and the complete phase checkpoint once
integrated; do not repeat unchanged passing checks without a new reason.

## Phase 1: Establish contracts and baselines

### Necessary Rules

The following are adopted defaults, assumptions, and execution gates for review,
not new requests for interactive feedback.

| Decision | Rule and observable consequence |
| --- | --- |
| Prerequisites | Require implementations and review-1 outcomes for `2026-10-11-schematic-generation-integrity`, `2026-10-12-schematic-runtime-envelope`, `2026-10-13-schematic-pagination`, `2026-10-14-schematic-definition-api`, `2026-10-15-schematic-definition-vocabulary`, `2026-10-16-schematic-response-model-graphql`, and `2026-10-17-schematic-websocket`. Refresh contracts from their logs before any behavior change. If unavailable, record the blocker and stop implementation with a failure status. Do not fabricate a refresh or edit their specs. |
| Execution authority | This request authorizes writing the plan only. Before later unattended implementation, verify the spec's pre-authorizations have been confirmed in that session. `yolo` does not authorize commits, provider calls, lifecycle moves, dependent edits, or CI changes. Missing required authority is a recorded execution blocker, not an interactive prompt. |
| P10-a and document identity | Export genuine AsyncAPI 3.0 from the client application's perspective, with a genuine 3.0 import path and existing 2.x import compatibility. One document per WebSocket API; a unique module uses `{module}.json`, a shared module uses `{module}--{canonical-selection-name}.json`. Validate selection names and reject case-insensitive collisions. Use `schematic/asyncapi/`. Do not emit 2.x structures under a 3.0 version. |
| AsyncAPI subset | Preserve info, applied servers/security, endpoint channels/parameters, send/receive operations, message/schema components, auth policy, selectors, per-direction fallback, role, and heartbeat hints. Exclude traits, correlation hints, and bindings beyond WebSocket query/headers. Reject unsupported versions and diagnose remote references without fetching. |
| AsyncAPI metadata | Adopt message `x-schematic-selectors: [{field, value}]`, message `x-schematic-fallback: true`, channel `x-schematic-role` using the upstream role's serialized values, and channel `x-schematic-heartbeat` using the upstream serde representation. Read hints back as well as exporting them. Confirm exact selector value types and hint placement after spec 6; log any adaptation. Role does not reverse client-perspective operation directions. |
| P10-b, P10-h, P10-i | Document-level `x-schematic.version` is integer major 1; absence means 1. Unsupported or malformed versions disable all Schematic metadata, including selectors, with WARN. Unknown fields produce INFO; malformed known fields produce WARN independently. Standard OpenAPI wins conflicts with location-bearing WARN. Strict rejects WARN/ERROR and permits INFO. Reuse `OpenApiDiagnostic`; no new diagnostic format. |
| Extension subset | The contract table below is provisional until upstream refresh; exact new field spellings cannot honestly be finalized from today's legacy structs. Freeze an actual field-by-field standard/extension mapping in Phase 1 before implementing Phase 4. Remove `oauth_scopes`; keep only bearer header override and API-key value prefix as auth extension details, at their API or override declaration level. Delete the unused schema/auth extension structs only after preserving those details. |
| P10-c | Choose validated candidates in order: first example, default, const/single-value enum, first enum value, constrained typed placeholder. Emit required properties and properties with explicit example/default. Try composition branches in declaration order against the full schema; combine allOf. At depth 3 omit optional recursion or use permitted null; diagnose required unbounded recursion. No randomness, clock, general regex solver, or satisfiability engine. Failed candidates never become invalid examples. |
| GraphQL | Postman emits one POST per operation with authored query, operationName, and variables generated from a registered schema or `{}` when no schema exists. OpenAPI skips each operation with a named diagnostic; GraphQL is outside its round-trip subset. |
| P10-d | One feature per resolved Rust module group, default enables every module. WebSocket-only dependencies are optional. Full inventory drives manifests even for partial runs. No `all-apis` feature unless defaults prove insufficient. |
| P10-e and P10-f | A plain registration function in `schematic-definitions` binds canonical selection names, aliases, identity, protocol, module group, definition and registry/catalog constructors. Derive all consumers. One resolver, owned there, preserves the Rust assembler's existing suffix inference behavior; external artifact stems follow it. Generated clients acquire no definitions build dependency merely for enumeration. |
| P10-g | No endpoint wrapper methods. The author already supplied the compile-cost evidence (about 7 seconds per 550 wrappers); record the accepted gap and do not benchmark or revisit it. Documentation uses typed `client.send(Request::new(..))` after verifying the spec 2 API. |
| P10-j | Optional environment export switch; `{stem}.postman_environment.json` beside the collection. Saved responses stay inside collections. All collection variables are declared even without environments. Credentials are empty and never read from host variables. |
| P10-k | Reuse `jsonschema = { version = "0.55", default-features = false }`, already present in `schematic-gen` dev-dependencies and Cargo.lock. If definitions tests need it, use the same locked version as a dev-dependency. Validate against original registered schemas, with local resolution only. Inspect its supported dialect before relying on it. |
| P10-l | Use every committed OpenAPI artifact (historically 15) plus targeted fixtures; derive current counts rather than hard-code them. Compare semantic values against upstream JSON definition snapshots; list unsupported differences rather than weakening equality. |
| Upstream-only repairs | Verify stale-tree absence, vendor constructor/shared preset absence, and the `?` path hard error. Failure is an upstream blocker; this feature does not delete the stale tree, move presets, or migrate endpoint literals. R43's unchanged-output claim applies to preset verification, not to the independent bridge/header improvements. |
| Validation boundaries | No CI workflow/scope change, raw `cargo test`, direct nextest invocation, `cargo fmt`, commits, provider requests, or dependent edits. Use repository recipes and the test toolkit. Load the `os` skill before resolver/path work and before claiming any target environment is unavailable. |

The extension mapping to freeze includes all of the following. Standard fields
remain authoritative even when extensions preserve additional semantics.

| Semantic field | Mapping to verify after prerequisite refresh |
| --- | --- |
| module_path, request_suffix, env_mapping, API/endpoint fixed headers | Existing document/operation metadata; standard ordinary headers become parameters too. |
| request details | Extension only for details standard requestBody cannot express, including multipart fidelity. |
| response shapes, status/error table, request-id, streaming framing/end markers | Standard responses/content first; extension retains runtime-only semantics, including Bytes framing and end-marker keep. |
| rate-limit, endpoint cross_origin, secret markers | Preserve actual upstream metadata at its declaration level. |
| deprecated, auth override, scopes | Standard deprecated/security/securitySchemes for expressible meaning; no duplicate auth method/location/name/scopes in extensions. |
| bearer header override, API-key value prefix | Only extra auth fields in versioned document/operation metadata; never credentials. |
| x-schematic-selector | Preserve selector-merged operation metadata under the document version gate; keep same-address operations and content types reversible. |
| oauth_scopes, obsolete extension structs | Delete removed scopes copy; delete SchematicSchemaExtension and AuthExtension after mapping replacement details. |

### Wave 1: Dependency gate

- [ ] **Confirm prerequisites**
  - Read prerequisite implementation logs and review outcomes; verify typed send,
    serializable definitions/snapshots, response v2/GraphQL, WebSocket selectors,
    roles/hints, partial generation, staged import outputs, and isolated compile
    checks exist. Resolve discrepancies in this plan and log them, leaving the
    specification as its historical snapshot.
  - Check existing user changes before writing. Verify implementation authority
    and its boundaries from session evidence. Create `implementation-log.md`
    with spec/plan identifiers, implementer, packages, per-phase source/doc/skill
    files, decisions, and review agenda entries.

### Wave 2: Independent baseline investigations

- [ ] **Measure fidelity**
  - R5, R6, R14: collect per-API untyped-property counts and paths; distinguish
    legitimate free-form from lost constraints, including typed losses. Inventory
    current artifacts and exact request/response schemas.
  - Export then import empty/absent optional collections through the public
    pipeline for ledger row 14. Existing extension serde tests already cover
    omitted headers, but are not evidence of public import fidelity. Keep the
    reproduction as a regression test whether it fails or not.
  - Ownership: registry baseline helpers and extension reproduction tests only.

- [ ] **Inventory consumers**
  - R33: enumerate the six starting runtime lists in `definitions/src/lib.rs`,
    `definitions/src/registry/lookup.rs`, and `gen/src/pipeline.rs`, plus strict
    completeness/registry test tables; search for additional WebSocket lists.
    Record canonical names, aliases, ordering, registries, protocols, modules,
    current counts, and all consumers.
  - Inspect the eight dependent manifests without edits; record exact package
    IDs, paths, feature usage, and build/test recipes. Record users of artifact
    filenames before resolver-driven renames.
  - Ownership: inventory/log notes, no registration edits yet.

- [ ] **Verify hygiene**
  - R43, R50–R52: verify the stale tree is absent, vendor constructors/shared
    preset layer are absent, and `?` paths are rejected and absent from the
    registered endpoints. Measure emqx combined-doc loss and retain its fixture.
  - R47: locate `2026-10-09-consistent-use-schematic` across sniff lifecycle
    directories and search sniff for `FocusedProviderClient`; record evidence
    without editing sniff. Its incomplete migration does not block this feature.
  - Ownership: hygiene tests and log notes, no upstream repairs.

### Wave 3: Contract freeze and bounded spikes

- [ ] **Freeze contracts**
  - Integrate baseline notes; finalize the extension field mapping above from
    real upstream types. Expand the input matrices below to one column per
    actual load-bearing field, including nested fields newly introduced upstream.
    Specify absence/null/empty outcomes before reader implementation begins.
  - Record all adopted P10-a–P10-l defaults, extension subset exclusions,
    artifact naming, schema dialects, feature groups, and deviations for review 1.

- [ ] **Check validators**
  - One bounded spike on the current host: validate a small registered recursive
    request schema and inspect local-reference/dialect behavior of the existing
    validator. Inspect whether a small vendored AsyncAPI 3.0 schema plus its
    local dependencies can use the same validator offline. No performance sweep.
  - If feasible, use a frozen schema fixture and document its source/version;
    otherwise retain structure/closure validation and explicitly record full
    AsyncAPI schema validation as unverified. A validation gap never excuses an
    invalid 3.0 shape. Run this spike once; later phases add correctness tests.
  - If needed, one small emitted nullable-ref/enum fixture identifies a legal
    OpenAPI 3.0 representation accepting null and preserving non-null constraints.
    Record the representation for Phase 2; this is not a new benchmark.

**Checkpoint:** all prerequisites verified, public reproduction retained,
baseline/log inventory complete, field mapping and matrices frozen, spike
decisions recorded, and baseline `just test`/`just lint` green. An unmet
prerequisite prevents Phase 2; unresolved sniff migration is reported separately.

### Input robustness matrix

Applies to changed OpenAPI and AsyncAPI readers, in **JSON and YAML**, including
extension deserialization. It does not apply to generated Cargo manifests or
Postman writers. Both formats use the same semantic tables; execute a separate
whole-matrix test for each format and each public importer. YAML permits only
one document here. Reject duplicate mapping keys in both formats before a generic
Value conversion can erase them; do not let JSON last-wins hide duplicates.

The table generator used by the implementation must expand nested standard,
schema, security, model, and extension fields actually read by changed code.
The tables below define the known field policies now; the Phase 1 freeze fills
new upstream field spellings, and the matrix task cannot be checked off with
generic field classes standing in for actual columns.

Public-result codes: **E** means import fails with a location-bearing parse or
structural error; **W** means permissive import returns a field-specific WARN
and preserves unrelated valid data, while strict import fails; **I** means a
documented inherited/standard value; **O** means no optional metadata, with the
standard-derived definition retained; **V** means the explicit empty value is
preserved in the resulting definition; **D** means the entire Schematic subset
is disabled with WARN. Every code is asserted through returned definitions,
diagnostics, re-exported meaning, or written output, never a private parser.
Wrong-element rows on scalars mutate the field to an array and exercise its
wrong-whole-field outcome; they are not silently skipped.

Known OpenAPI extension fields (both JSON and YAML):

| Shape | version | module_path | request_suffix | env_mapping | API headers | operation headers | request | response | bearer header override | API-key value prefix | selector metadata |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | major 1 round trip | custom module | custom suffix | mapped auth names | fixed header | overriding header | multipart detail | runtime response detail | custom header | nonempty prefix | merged endpoints recovered |
| Absent | major 1 | O | O | O | O: no extra headers | O: no override | O | O | O | O | O |
| Explicit null | D | W | W | W | W | W | W | W | W | W | W |
| Wrong whole type | D | W | W | W | W | W | W | W | W | W | W |
| One wrong element | D | W | W | W | W: reject whole field | W: reject whole field | W | W | W | W | W: reject whole field |
| All wrong elements | D | W | W | W | W | W | W | W | W | W | W |
| Empty | D | W: empty name | W: empty suffix | V: no mappings | V: no extra headers | V: no override | W | W | W: empty header | V: empty prefix | W: invalid selector |
| Duplicate key | E | E | E | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E | E | E | E |

Absence of headers is explicitly defined by this extension format as no extra
headers, matching omission by its writer; it is not a fallback for malformed
collections. Reproduce omitted fields before adding serde defaults. Version
controls also cover unsupported integer major, fractional/string/null versions,
unknown keys (INFO), and malformed fields alongside a valid sibling. Add actual
columns for rate-limit, cross_origin, request-id, errors, response framing/keep,
secret markers, and every nested env/header/selector field during the freeze.
Do not treat nullable JSON Schema instance values as null schema objects.

Known OpenAPI standard fields changed by parameter/multipart/auth reconciliation:

| Shape | path-item parameters | operation parameters | parameter name | parameter in | parameter required | parameter $ref | requestBody content | multipart properties | multipart required | encoding contentType | operation security |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | inherited parameter | overriding parameter | retained name | retained location | required flag | local ref resolves | multipart imports | parts retained | required parts | part media type | applied auth |
| Absent | I: none inherited | I: inherit path | E | E | I: false except path must be true | inline parameter | E for present requestBody | I: unconstrained object, diagnose unsupported parts | I: no required parts | I: media default | I: root security |
| Explicit null | E | E | E | E | E | E | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E | E | E | E | E | E | E |
| One wrong element | E | E | E | E | E | E | E | E | E | E | E |
| All wrong elements | E | E | E | E | E | E | E | E | E | E | E |
| Empty | V: no inherited params | V: no own overrides | E | E | E | E | E | V: no parts | V: all optional | E | V: anonymous |
| Duplicate key | E | E | E | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E | E | E | E |

Unresolved local refs produce ERROR diagnostics; remote refs are diagnosed
without fetch. Add concrete nested part type/format/items, encoding, schema,
security scheme and scope columns where the refreshed mapping reads them.
Object-valued parts, unsupported file arrays, and unrepresentable encodings
produce diagnostics, never silently become text. An absent requestBody is
different from a present requestBody with absent content.

Known AsyncAPI standard fields (both formats; apply separate version-specific
columns/fixtures to 2.x url/publish/subscribe and 3.0 host/pathname/operations):

| Shape | asyncapi | info | info.title | servers | channels | operations (3.0) | components.messages | components.schemas | applied security | message payload |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | supported version | identity retained | title retained | handshake URL | endpoint retained | directions retained | message retained | model retained | auth retained | schema retained |
| Absent | E | E | E unless explicit name override | W: no server; require explicit usable base URL | E | W: no mapped operations | O: inline messages only | O: inline schemas only | I: applicable parent security | W: no representable payload |
| Explicit null | E | E | E | E | E | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E | E | E | E | E | E |
| One wrong element | E | E | E | E | E | E | E | E | E | E |
| All wrong elements | E | E | E | E | E | E | E | E | E | E |
| Empty | E | E | E | W: no server | V: zero endpoints with WARN | V: no operations with WARN | V: no named messages | V: no named models | V: anonymous | V only for legal unconstrained schema; otherwise E |
| Duplicate key | E | E | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E | E | E |

AsyncAPI Schematic metadata (both formats):

| Shape | x-schematic-selectors | selector field | selector value | x-schematic-fallback | x-schematic-role | x-schematic-heartbeat |
| --- | --- | --- | --- | --- | --- | --- |
| Control | same frame selects same message | wire field retained | match retained | fallback retained | role retained | hints retained |
| Absent | O: no selectors | W | W | I: false | I: Client default | O |
| Explicit null | W | W | valid only if upstream selector supports null, otherwise W | W | W | W |
| Wrong whole type | W | W | W per upstream selector domain | W | W | W |
| One wrong element | W: reject whole list | W | W per domain | W | W | W |
| All wrong elements | W | W | W per domain | W | W | W |
| Empty | V: no predicates | W | valid only if upstream allows that value | W | W | W unless upstream defines empty hints |
| Duplicate key | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E |

The control fixture must make every field load-bearing: named local references,
path/operation overrides, non-default auth, multipart encodings, overlapping
message payloads, and declared hints. Use frozen real exported artifacts, augmented
once by the tools where necessary; mutate exactly one cell at a time. For each
importer/format, one parameterized test walks the complete matrix plus its control
row. Assert strict/permissive behavior, definition changes, diagnostics, and
persisted output/dry-run absence. Grep changed readers for `#[serde(default)]`,
`Option<T>`, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()`; justify
each load-bearing use. Use presence-aware decoding when absence and null differ.
A review finding on a cell requires reporting the entire row and column state.

## Phase 2: Recover schemas and reference closure

### Wave 4: Bridge implementation

- [ ] **Recover constraints**
  - R1–R4: implement recursive conversion in
    `definitions/src/registry/conversion.rs`: type arrays, nullable primitives,
    refs/enums, const/enum, compositions, and additionalProperties booleans/maps.
    Multiple non-null types use anyOf; preserve sibling constraints beside refs
    and compositions. Unsupported keywords produce schema-path diagnostics.
  - Test actual accepted/rejected values for optional strings/arrays/structs,
    nullable enums, overlapping integer/number unions, tagged enums, flattened
    structs, maps, bounds, formats, defaults, and required properties.

### Wave 5: Independent registry checks

- [ ] **Validate closure**
  - R7, R53: resolve against emitted components including extracted nested
    definitions; walk registered roots and endpoint schemas with visited refs,
    JSON-pointer unescaping, recursive models, and external-ref diagnostics.
    Block generation/export on missing references with parent, path, missing name.
  - Test missing refs, extracted definitions, valid recursion, and inline Vec<T>
    with element ref and no container component. Ownership: registry closure code.

- [ ] **Audit losses**
  - R5, R6: implement recursive source/output constraint comparison; inspect
    properties, items, composition branches, map values and referenced components.
    Catch losses in typed output as well as accidental Any; paired free-form/lost
    fixtures pin classification. Ownership: audit tests/helpers, not conversion.
  - Record before/after counts and every deliberate free-form path; assert zero
    lost constraints and no container component names across all registries.

### Wave 6: Artifact integration

- [ ] **Regenerate schemas**
  - Regenerate OpenAPI artifacts, review only intended type/constraint recovery,
    and run closure/loss/export/drift tests. Confirm no property disappears.
    Regenerate Rust into a comparison tree and assert byte-identical output;
    do not hand-edit generated sources.

**Checkpoint:** R1–R7/R53 evidence exists, all registries pass, lost count is
zero, free-form exceptions are named, artifacts pass drift, and common phase
checks pass. A bridge-only change must not change generated Rust.

## Phase 3: Export required constant headers

### Wave 7: Header mapping

- [ ] **Map headers**
  - R8, R9: emit required ordinary headers with string single-value enum and
    default; merge API/endpoint declarations case-insensitively, endpoint wins.
    Merge compatible explicit parameters and diagnose incompatible collisions.
  - Identify credential headers from effective operation auth, including override.
    Reserved Accept/Content-Type/Authorization use content/security; preserve
    unexpressible fixed details in metadata and diagnose gaps.
  - Test overrides, credential exclusions, reserved names, and collisions; verify
    committed Anthropic anthropic-version. Regenerate OpenAPI and update its docs.

**Checkpoint:** ordinary headers are usable by standard OpenAPI consumers,
no duplicate/reserved/credential parameter is emitted, Rust output is unchanged,
and export/drift plus common phase checks pass.

## Phase 4: Import standard and Schematic metadata

### Wave 8: Standard import mappings

- [ ] **Merge parameters**
  - R16, R17: strict rejects WARN/ERROR only; replace the tautological test with
    INFO-only/WARN-only/ERROR cases and correct ImportOptions docs. Resolve local
    parameter refs before merging path-item/operation parameters by name/location,
    case-insensitive for headers. Ownership: builder, parameter mapping tests.

- [ ] **Import multipart**
  - R18: preserve part names, required/file/text, accepted content types from
    encoding, and representable file arrays. Diagnose unsupported object/encoding
    forms. Round-trip the ElevenLabs AddVoiceSample shape. Ownership: request-body
    mapping and multipart fixture tests; shared import wiring goes to integrator.

### Wave 9: Extension reconciliation

- [ ] **Roundtrip metadata**
  - R10–R15, R54: implement document version gating and independent field decoding;
    preserve standard precedence and location-bearing unknown/malformed/conflict
    diagnostics. Include actual refreshed response/framing/keep and runtime metadata.
  - Retain only the two extra auth details, delete removed oauth_scopes and unused
    extension structs/re-exports, and record the R14 reproduction disposition.
    AuthExtension must not be wired as a complete auth duplicate.
  - Compare export/import/export semantic definitions using upstream snapshots
    and every committed artifact plus fixtures. Compare preserved semantics,
    including merged selectors; explicitly list subset exclusions/differences.

### Wave 10: Reader acceptance

- [ ] **Exercise inputs**
  - Execute the frozen JSON and YAML OpenAPI matrices through public import;
    malformed fields must not discard valid siblings. Cover missing version,
    unsupported/malformed versions, unknown keys, conflicts, and strict behavior.
    Ensure duplicate detection occurs before Value conversion.
  - Update round-trip claims in extensions.rs, export/mod.rs, and topic pages to
    the supported subset; regenerate OpenAPI and verify GraphQL remains excluded
    once Phase 9 installs its named diagnostics.

**Checkpoint:** R10–R18/R54 pass, mappings and subset are logged, public matrices
pass, Rust output remains unchanged, and artifact/common phase checks pass.

## Phase 5: Consolidate API registration

### Wave 11: Inventory implementation

- [ ] **Unify registrations**
  - R33, R34: implement the plain authoritative inventory in definitions; derive
    registry lookup, apis_by_module, generator selection/help/input, REST exports,
    WebSocket packaging/export enumeration, and test consumers. Preserve aliases
    and stable ordering; REST-only consumers filter by protocol.
  - Touch API-level registration/wiring only, not hundreds of endpoint literals.
    A broader required rewrite is a logged blocker under the spec's scope rule.

### Wave 12: Consumer coverage

- [ ] **Prove propagation**
  - R34, R35: inject one local fixture registration and observe it in every
    consumer; include WebSocket and shared-module cases. Derive the strict gate's
    covered set from the inventory; named skips require one reasoned registry.
  - Compare covered/shipped sets, grep away secondary hand-written lists, and
    log exact locations removed. Avoid replacing runtime duplication with a
    parallel test table. Update registration guidance and pipeline Mermaid flow.

**Checkpoint:** adding one fixture entry reaches all consumers; no API silently
misses completeness validation; unchanged registrations yield unchanged artifacts
and Rust output; common phase checks pass.

## Phase 6: Unify module and artifact names

### Wave 13: Resolver migration

- [ ] **Unify resolver**
  - R36, R37: load the os skill, move the existing Rust inference policy to one
    definitions-owned resolver, and remove the divergent implementation. Rust,
    OpenAPI, Postman, registration grouping, and later AsyncAPI naming use it.
  - Test explicit module paths, PagerDutyApi-style inferred names, fallback,
    shared groups, path portability, and case-insensitive stem uniqueness.
    Preserve public Rust module names and aliases.
  - Search old filename consumers; rename/remove superseded committed artifacts
    exactly once and log each. No dependent edits: an incompatible dependent
    requires resolving the naming policy within this feature or recording a blocker.

**Checkpoint:** every writer resolves the same module/stem, real filename
changes are listed, no stale artifact survives, generated Rust is unchanged,
and drift/common phase checks pass. This follows Phase 5 and precedes features.

## Phase 7: Generate module feature flags

### Wave 14: Feature generation

- [ ] **Gate modules**
  - R29, R32: generate one feature per full-inventory module, default all;
    optional WebSocket-only dependencies enabled by their module features.
    Gate module declarations, prelude exports, shared WebSocket helpers, and
    module-dependent docs together. Keep shared runtime dependencies as needed.
  - Verify generated full/partial manifests are equal and partial generation
    preserves module siblings. Shared modules remain a single feature group.

### Wave 15: Feature and dependent evidence

- [ ] **Compile selections**
  - R30: use the upstream isolated-crate mechanism via recipes for no features,
    one REST module, one WebSocket module, a shared group, and a REST/WS pair.
    Positive probes verify enabled APIs; compile-fail/item-absence probes verify
    disabled APIs. Inspect resolved dependency graph: REST-only has no tungstenite.
  - Ownership: isolated feature tests and graph checks; integrator owns manifest
    and generated output. Regenerate clients; run test-schema/lint-schema.

- [ ] **Verify dependents**
  - R31: build/test all eight recorded package IDs via repository recipes, with
    unchanged manifests and sources. Divide independent packages among subagents
    only if build/output isolation is available; otherwise serialize builds.
  - Record feature usage and command results. A dependent needing edits is a
    requirement failure, not permission to change that dependent.

**Checkpoint:** selected builds expose exactly their groups, additive features
work, transport dependencies are correctly gated, full/partial generation agrees,
all dependents pass unchanged, and common phase checks pass.

## Phase 8: Compile honest generated documentation

### Wave 16: Documentation generation

- [ ] **Correct examples**
  - R38–R41, R51: executable Rust fences become no_run; keep prose/shell/JSON
    languages appropriate. Use real typed-send constructors, complete ApiKey
    fields, remove stray ///; emission, and retain every shared-module API's docs.
  - Test freshly generated emqx docs for all members and assert no stray lines
    or unsupported method names. No endpoint wrappers are added.

### Wave 17: Doctest integration

- [ ] **Compile docs**
  - Extend `just doctest` or reuse recipe-driven isolated doctest compilation to
    include the excluded generated crate. Compile default-all and a selected
    module with no defaults; examples must not import disabled modules.
  - Regenerate clients/README, record the D53 accepted gap, and run generated
    doctests, test-schema/lint-schema, and common phase checks.

**Checkpoint:** real Rust examples compile under both feature configurations,
all shared API docs survive, and no missing method, incomplete literal, or ///;
line remains. Generated source changes come only from generator changes.

## Phase 9: Produce runnable Postman artifacts

### Wave 18: Schema example engine

- [ ] **Generate examples**
  - R19: pass the authoritative original schema registry into Postman generation;
    implement P10-c selection, constraints, compositions, local refs, and depth-3
    recursion. Validate candidates against full original schemas before emitting.
  - Test deterministic bytes, invalid authored defaults/examples, unsatisfiable
    branches, oneOf exclusivity, allOf constraints, and required recursion.
    Unsupported cases return endpoint/schema-path diagnostics, never invalid JSON.

### Wave 19: Independent Postman mappings

- [ ] **Map requests**
  - R22–R24/R55: preserve OAuth2 grants/URLs/scopes, Text/Binary mode/content
    types, header parameters, and one encoded Cookie header. Respect constants,
    effective credentials, collisions, and optional parameters' disabled state.
  - Emit typed GraphQL operations as runnable POST bodies with generated variables.
    Add one OpenAPI skip diagnostic per GraphQL operation, no synthetic HTTP paths
    or GraphQL extension. Ownership: request/auth/body mapping and skip diagnostics.

- [ ] **Build environments**
  - R20: build optional environment output, declare every collection placeholder
    regardless of the switch, and separate server/auth variables for shared API
    groups. Include path/query/header/cookie variables; credentials remain empty.
  - Ownership: variable/environment writer module and tests. Integrator owns
    export option/CLI wiring and shared type/test-root edits.

- [ ] **Save responses**
  - R21: use the example engine for each success media type and declared error
    shape. Choose deterministic concrete statuses that respect narrower overrides.
    Empty bodies stay absent, binary examples are documented placeholders, and
    finite streams follow framing/end markers and per-frame validation.
  - Ownership: saved-response builder and tests, not request/auth modules.

### Wave 20: Postman integration

- [ ] **Validate collections**
  - Validate every shipped JSON request and saved JSON response against original
    schemas with local definitions; finite streams validate their frames.
    Every failure is diagnosed and has a named authored example or accepted gap.
  - Exercise variable completeness with/without environments, shared-module auth,
    OAuth2 golden output, cookies/collisions, GraphQL body/URL, and skip diagnostics.
    Log three representative generated examples for author review.
  - Regenerate collections and requested environments, extend drift coverage,
    update Postman/OpenAPI docs, and run common phase checks.

**Checkpoint:** each emitted example validates, output is deterministic, all
variables are declared, OAuth2/content types/parameters survive, GraphQL is
runnable in Postman and explicitly skipped in OpenAPI, and drift passes.

## Phase 10: Export and persist AsyncAPI definitions

### Wave 21: Independent format implementations

- [ ] **Export AsyncAPI**
  - R25/R56–R58: add `gen/src/export/asyncapi/` over confirmed spec 6 types.
    Emit real 3.0 servers with host/pathname, channel addresses/messages,
    separate send/receive operations, and component schemas/security.
  - Preserve client/server/bidirectional directions from the client perspective,
    applied auth, selectors/fallback, role, heartbeat hints, and handshake URLs.
    Validate deterministic naming and local ref closure. Ownership: exporter.

- [ ] **Read AsyncAPI**
  - R26, R27: version-dispatch 2.x and 3.0 readers; retain legacy fixtures,
    reject unsupported versions, resolve local refs, and derive security from
    declarations applied to the endpoint rather than the first component scheme.
  - Read selectors, fallback, role and hints; duplicate fallbacks per direction
    produce a diagnostic. Use the Phase 1 frozen matrix to remove silent malformed
    field fallbacks. Ownership: asyncapi_import.rs and reader-specific helpers.

### Wave 22: Persistence and round trips

- [ ] **Persist imports**
  - R26: use the upstream staged OpenAPI import-output contract for reusable
    WebSocket definition/model outputs; retain in-memory results and diagnostics.
    Read saved outputs back and prove dry-run writes nothing.
  - Validate partial-write/failure behavior inherited from that contract in a
    temporary directory; do not replace it with a report-only or bespoke format.

- [ ] **Verify roundtrips**
  - R27/R56–R58: per shipped API, compare semantic channels, messages/schemas,
    directions, auth/parameters, selectors/fallback, roles, and hints. Include
    overlapping payloads, each role, shared modules, and base-path URL fixtures.
  - Execute JSON/YAML whole-input matrices through public import and persisted
    results. Perform offline schema validation if the Phase 1 decision supports
    it; otherwise mark that guarantee unverified. Ownership: round-trip tests.

### Wave 23: AsyncAPI integration

- [ ] **Publish artifacts**
  - R28: add `just generate-asyncapi`, write one collision-safe document per API
    into schematic/asyncapi, and add L1 drift/closure coverage derived from inventory.
  - Document subset, 2.x import compatibility, 3.0 structure, filename rule,
    extensions ignored by third-party tools, client perspective, and read-back.
    Regenerate and run common phase checks.

**Checkpoint:** all current WebSocket APIs export/re-import, saved output is
reusable, no local ref dangles or file collides, role/selector/hint semantics
survive, matrices and drift pass, and schema-validation status is explicit.

## Phase 11: Doctest the definition README and verify hygiene

### Wave 24: Independent checks

- [ ] **Compile README**
  - R42: include define/README.md as crate docs and rewrite examples to the real
    upstream vocabulary, including pagination, response shapes, and auth policies.
    Network/credential examples use no_run; ignore needs a stated compilation
    obstacle. Ensure a deliberately broken fixture proves the recipe catches drift.
  - Ownership: define README/lib docs. Run `just doctest`, including generated
    docs from Phase 8, and update related README usage.

- [ ] **Confirm invariants**
  - R43/R50/R52: rerun the retained correctness assertions for no vendor-named
    neutral constructors, no shared preset layer, no stale gen/schematic tree,
    and no query delimiter in registered paths, with the upstream hard-error test.
  - Record that this phase moves no presets or endpoint literals and does not
    delete upstream-owned trees. Ownership: verification evidence/tests only.

**Checkpoint:** README examples compile through recipes, verification evidence
is current, upstream-only invariants hold, and common phase checks pass. These
are final correctness checks, not reruns of exploratory spikes.

## Phase 12: Audit documentation and deterministic output

### Wave 25: Independent documentation audits

- [ ] **Audit guidance**
  - R48: audit schematic/README, define/gen/schema READMEs, and docs/io pages:
    openapi-extensions, export-openapi, import-openapi, export-postman,
    crates-openapi, plus AsyncAPI export and feature/inventory guidance.
  - Explain what a newcomer can do, with compact examples and Mermaid diagrams
    for generation and round-trip flows. Topic pages never name/link feature/fix
    snapshots. Ownership: README/topic docs.

- [ ] **Audit contracts**
  - R49: review changed symbols' rustdoc/inline comments against code, record
    drift corrected, and verify subset, strict-mode, auth, feature, and example
    guarantees agree across docs and tests.
  - Update schematic-define skills and any existing create-schematic-api guidance
    to describe one registration and current recipe/API contracts. Update
    dependencies docs for dependency additions/removals; do not clean unrelated
    skill guidance. Ownership: skill/dependency docs and audit notes.

### Wave 26: Determinism checkpoint

- [ ] **Check regeneration**
  - Regenerate Rust/OpenAPI/Postman/environments/AsyncAPI through recipes twice;
    second outputs must be identical with no unintended diff. Review renamed
    files, shared-module siblings, full/partial manifests, and artifact drift.
  - Run common phase checks plus generated-crate checks for any generation edit.
    Scope all checks to Schematic and its dependents, without changing CI policy.

**Checkpoint:** docs and skills accurately describe built behavior, every changed
symbol received a drift pass, and deterministic artifacts have no stale files.

## Phase 13: Assemble closing evidence

### Wave 27: Final acceptance run

- [ ] **Verify acceptance**
  - Run from schematic/: `just test`, `just doctest`, `just lint`,
    `just lint-schema`, and `just test-schema`; include recipe-driven selected
    feature/doc compile checks and all artifact/input-matrix coverage.
    Do not use the current legacy `check-drift` recipe, which invokes cargo test;
    run drift through L1 recipes or a prerequisite-corrected recipe.
  - Confirm current-tree build/test evidence for all eight dependents, using
    `just test {pkg}` and their recorded build recipes from the repo root/areas.
    Reuse unchanged qualifying results from Phase 7; rerun when later generated
    changes invalidate them. Inspect diffs to prove no dependent was edited.
  - Load os skill and collect/reuse qualifying macOS, Linux, native Windows, and
    WSL2 evidence for required checks. Record exactly what ran and any unavailable
    environment with supporting evidence; never call missing evidence a pass.
    Do not create a CI matrix cell/workflow as a substitute.
  - All new tests are L1 hermetic filesystem/subprocess tests; use test toolkit
    fixture paths and declared repository inputs. If an existing L2 test was
    actually changed, run `just test-l2` with focus-preserving harness visibility.

### Wave 28: Ledger and review handoff

- [ ] **Close ledger**
  - R44, R45: create one disposition table with all 22 ledger rows and 14
    recommendations, each linked to actual child implementation evidence or a
    reasoned accepted gap. Verify 36 nonblank entries with a one-shot check;
    do not commit a test depending on lifecycle log paths.
  - Add required extra dispositions: Schema::module_path, warning then hard-error
    migration of nine `?` paths, old pagination types/conversion, vendor strategy
    ownership, oauth_scopes removal, emqx docs, and parent section 6 items 5–8
    where not already represented. Record corrected July figures.
  - Mark the five named accepted gaps separately: optional-auth Ollama variant,
    auth dual-source cleanup, invented HuggingFace stub shapes, orphaned types,
    and typed convenience methods. Retrieve reasons from predecessor evidence;
    absent evidence stays open, never invented.

- [ ] **Report closure**
  - R46, R47: evaluate D20's four parts independently: landing acceptance,
    dependent checks, dispositions, and sniff migration. Refresh sniff symbol
    search/spec status read-only; any reference or incomplete migration makes
    part 4 open. This feature can be review-ready while whole-work closure is open.
  - Complete the spec's 26-item human review agenda as decisions occur and verify
    all entries now have evidence. Include a requirement index for R1–R58,
    adopted defaults, deviations, subset exclusions, validator/gap status, OS
    evidence, renamed artifacts, and pre-authorizations actually used.
  - Report **implementation complete, ready for review** only after required
    checkpoints pass; list remaining accepted gaps/open whole-work items. Never
    move a spec, run just complete, or commit under this plan alone.

**Checkpoint:** all acceptance results are accounted for, every requirement has
a disposition, the ledger has 22 + 14 complete rows, D20 states are honest, and
the author has a concrete review agenda. Missing prerequisite/authorization or
required failed checks are blockers, not accepted implementation completion.
