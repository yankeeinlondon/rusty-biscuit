---
total_phases: 8
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Schematic definition API implementation plan

## Work and Successful Completion

Replace REST definition literals and hand-typed schema references with required-fact builders and a typed, module-scoped definition context. Capture the existing definitions and generated artifacts before changing them; introduce the context and builders additively; update the import emitter; convert OpenAI once and then the remaining APIs in verified batches; remove obsolete interfaces only after their consumers have migrated. Preserve WebSocket definitions and generated request defaults. Finish by regenerating artifacts, verifying dependents, and updating the authoring documentation.

Success means all R1–R30 have observable evidence: all shipped REST endpoints use builders, registries come from the same context assembly as definitions, generated Rust paths compile, invalid references and contradictory declarations fail with context, containers preserve their wire shape without becoming components, pagination resolutions and caller controls survive, and all nine query-bearing paths are migrated. Historical differences are individually explained; final committed Rust, OpenAPI, and Postman artifacts have no drift exemptions. The implementation log records decisions and evidence as they occur. The terminal state is **implementation complete, ready for review**, with platform evidence and any remaining limitations stated accurately.

This is a conditional implementation plan. On 2026-10-11, the generation-integrity, runtime-envelope, and pagination specifications are marked `draft-spec` and `implemented: false`; the definition-API specification has `refresh_required: true`. The current code does not supply the reviewed runtime and pagination interfaces assumed below. Phase 1 must reconcile those outcomes before its baseline is captured. This plan does not implement the predecessor features, revise the specification snapshot, or treat their proposed interfaces as existing code.

Waves are numbered globally. Each wave is an execution step: finish its tasks and checkpoint before starting the next. “Parallel” means the listed tasks may be assigned to separate subagents with disjoint file ownership; the coordinator integrates shared changes and runs the checkpoint. All agents operate non-interactively, without commits, `cargo fmt`, or focus-stealing terminal/browser windows. Single-task waves and per-API migration batches are sequential.

Every phase gate runs `just test` and `just lint` from `schematic/`, with the definition/artifact comparisons available at that point. Use nextest recipes for L1, `just doctest` explicitly for documentation tests, and `just test-l2` only for real-terminal coverage. The current `test` recipe does not run doctests. Load the testing skills and Test Toolkit before authoring tests; use runtime fixture paths and CI-indexable repository reads. Do not use the existing `check-drift` or `test-e2e` recipes unchanged: they still invoke `cargo test`; migrate any required runner to nextest.

## Phase 1 — Establish Contracts and Baselines

### Necessary Rules

These rulings adopt the specification's defaults where available. They are planning decisions to record in `implementation-log.md`, not claims that the design has already been implemented.

| Item | Adopted rule or prerequisite |
| --- | --- |
| Prerequisite refresh | Read review 1 outcomes of `2026-10-11-schematic-generation-integrity`, `2026-10-12-schematic-runtime-envelope`, and `2026-10-13-schematic-pagination`. Record actual validator, diagnostic, compile-check, runtime, pagination, adoption, overlay, and frozen-test interfaces. If those outcomes are unavailable, mark dependent work not executable; do not fabricate substitute runtime or pagination features. |
| PD1 | Containers recurse to element/value registrations. Maps accept String and Serde-supported integer keys; reject unsupported keys with API, endpoint, and type context. String-like newtypes require an explicit supported-key contract. Option affects wire nullability and the Rust wrapper, while remaining transparent only for component registration. |
| PD2 | Aggregate structured `DefinitionError::Collision { schema_name, first, second }` errors for each module group/API; fully qualified Rust type names are diagnostic text only. Render one line per collision using the specification's message unless the predecessor established a diagnostic format. |
| PD3–PD5 | Use the predecessor's request-id field, secret/redaction fields, CrossOrigin type, and exact-origin validator. Lowercase and validate one API request-id header. Parameter `.secret()` sets the existing field. Add header sugar only where its storage exists. Endpoint cross-origin replaces the API policy wholesale; runtime callers may only tighten it. |
| PD6 | Keep `EndpointParams::Default`, preferring `EndpointParams::new()` in examples. Audit all other public definition defaults and record an exact allow-list with reasons. Endpoint, RestApi, and their builders have no public Default; generated request types retain theirs. |
| PD7–PD8 | Endpoint constructor requires id, method, and path; typestate requires description and one success shape. API constructor requires context, name, and base URL; typestate requires description and an explicit authentication choice. Adopt Design-table names, cover every authored HTTP verb and request form, and yield pagination names to the predecessor. Reserve `.path_param`, `.public`, `.auth`, `.deprecated`, `.rate_limit`, and the later response vocabulary names. |
| R12 proposal | Adopt `#[non_exhaustive]` on Endpoint and RestApi after every external consumer and emitter has migrated. Do not introduce it in the additive builder phase. Any use of the spec's source-scan-only fallback must be justified by concrete churn evidence and flagged for review; it weakens the construction guarantee. |
| PD17 | Keep auth_policy and its multi-method semantics; remove oauth_scopes only after repeating the shipped-use and reader scan. An unexpected live use requires a logged resolution, not silent data loss. |
| PD19 | Convert OpenAI through the emitter once in Phase 5. Convert the remaining APIs smallest first in Phase 6, one API per batch. Shared-module pairs integrate serially through their common context. Run R10/R24 after every batch; fix failures before continuing. |
| PD20 | Capture canonical JSON under `gen/tests/fixtures/definition-baseline/<api>.json`, sorting object keys and preserving endpoint, array, and registry entry order. Measure actual aggregate size. Above 5 MB, use biscuit-hash for API metadata, each endpoint, and each ordered registry entry including schema content; record canonicalization and algorithm. No performance spike is needed for this specified size decision. |
| PD21 | After migration, remove Schema.module_path from the REST path if no semantic reader remains; otherwise wire it from the context and test that reader. Emitter copying is not a reader. Preserve shared WebSocket behavior. |
| Fallible assembly | Adopt `define_x_api(&mut DefinitionContext) -> Result<RestApi, DefinitionError>`. One module-group assembly returns definitions and their registry together; lookups propagate errors distinctly from unknown names, rather than returning empty data or unwrapping. Registry adapters stay in schematic-definitions; schematic-define remains provider-neutral. |
| Naming and identity | Never derive a generated Rust path by parsing `std::any::type_name`. Ordinary names require a proven public identifier; exceptions use compile-checked `rust_path!` and optional schema_name. Prefer a typed macro token proving the authored path names T; support generic type paths only where the spike proves their stable implementation. register_as targets non-container leaves. |
| Wrapper storage | Reserve `structure: Option<TypeRef>` with no public setter; all authored registrations leave it None. Store the wrapper/reference information needed by validation and lowering separately, so R3/R7 work without reading the reserved carrier. Do not use the carrier as a second schema source. |
| Ownership limits | No deriving proc-macro, callback descriptors, global registry, shared vendor presets, convenience methods, GraphQL, response-v2 behavior, or vocabulary additions. Fixed query inputs and narrow wire/Rust-name separation are authorized here; general query fidelity and self-contained schema bundles remain later work. No CI cells or scope changes are added. |

### Wave 1 — Refresh and Inventory (Parallel)

- [ ] **Refresh Interfaces**
  - Record predecessor review outcomes and the actual API names in the implementation log, including EndpointSpec, CrossOrigin, pagination resolution/adoption, the generic overlay hook, and isolated compile checking.
  - Map the six R27 frozen tests to their actual names and locations; preserve the no-GraphQL half of the runtime surface test.
  - Own the contract/log document; no source changes. Exit requires a completed prerequisite interface map or an explicit dependency hold.
- [ ] **Inventory Consumers**
  - Use sniff for repository/dependency discovery and text searches for concrete call sites. Record all REST API names, endpoint counts and ordered ids, registry entries/order, container components, defaulted fields, Basic/OAuth2 users, and five WebSocket definitions.
  - Recount the expected approximately 519 endpoints, 265 OpenAI endpoints, 36 container registrations, 35 legacy pagination declarations, and nine query-bearing paths; treat observed inventory as authoritative.
  - Inventory GitLab repository-file request users before its field location changes, all Default implementations, oauth_scopes/auth_policy/module_path readers, external literals, registry lookups, and generated-schema dependents. Own an inventory artifact, not the shared log file.

Checkpoint: predecessor contracts and inventory agree; actual counts and consumers replace estimates throughout the execution checklist. Missing prerequisites stop baseline capture and dependent implementation, while preserving the inventory for later use.

### Wave 2 — Resolve Design Risks (Parallel, Once Only)

- [ ] **Registration Spike**
  - Build one small throwaway stable-Rust prototype covering a nominal type, serde-renamed nominal type, alias/re-export, foreign type, primitives, Value, Vec, Option, HashMap, and nested containers. Prove private descriptors, ordered idempotent registration, collision aggregation, destination-context validation, and fully qualified emitted paths.
  - Resolve dispatch without overlapping blanket `T: JsonSchema` and container implementations. Candidate: a local registration trait with nominal implementations emitted by a declarative helper/import emitter, and disjoint implementations for containers/primitives/Value. JsonSchema supplies schema names; the nominal implementation supplies the Rust identifier. Do not introduce a second schema registration list merely to implement this trait. If a simpler schemars-supported dispatcher meets every contract, prefer it and document how it distinguishes nominal wrappers from containers.
  - Prove rust_path spelling/resolution and T identity, including a negative mismatched-T case; select the exact registration bounds and descriptor wrapper metadata. Reject designs relying on type_name parsing, specialization, or a new deriving proc-macro. Record chosen public signatures before Phase 2.
- [ ] **Reproduction Proof**
  - On one host, run the unchanged OpenAI import with identical recipe options and pinned inputs into a temporary destination; compare mod.rs and types.rs byte for byte. Regenerate Rust/OpenAPI/Postman into temporary trees and compare their complete file sets with committed artifacts.
  - Classify differences as tool/version/options, nondeterminism, stale artifacts, or legitimate authored changes. Define a repeatable generic transform for legitimate import differences and prove reproduction before migration; never overwrite evidence to make the test green.
  - Preserve the hand-written overlay. This one-time reproduction investigation is not repeated later as a spike; later re-import and drift checks are regression validation.

Checkpoint: stable registration design compiles, required negative cases fail as expected, and import/artifact reproduction is proven or its concrete blocker is logged. No timing benchmark, OS measurement sweep, or broader importer rewrite is scheduled.

### Wave 3 — Capture Safety Nets (Sequential)

- [ ] **Capture Baselines**
  - Add plain serde support first to RestApi, Endpoint, EndpointParams, ParamDef, QueryParamType, ParamStyle, and necessary inner runtime/pagination types (R17). Round-trip each type without changing authored definition behavior.
  - Capture unchanged definition JSON plus ordered registry schemas, and resolved pagination for every list endpoint and the 35 declarations. Include GET Vec recognition, POST arrays requiring explicit list authorship, aliases, and adopted caller controls.
  - Add a reusable non-mutating `just migration-check` recipe/helper: regenerate all three outputs into temporary directories, compare file sets/bytes, and compare historical definitions as JSON values before any changed-model deserialization. Accept only named API/endpoint/field or registry-entry differences with causes; never broad response/schema ignores.
  - Extend the normal L1 artifact drift suite to Rust clients, with final committed artifacts as its reference and no permanent allow-list. Add a deliberately changed-description control proving snapshot comparison detects behavioral drift. Measure fixture size and apply PD20 only if needed.
  - Initialize implementation-log frontmatter following the specified worktree example, listing spec/plan identifiers, implementer, touched packages, and per-phase source/doc/skill files. Log all rules, spikes, counts, and reproduction evidence as they occur.

Phase gate: serde round-trips, snapshot mutation control, original pagination resolutions, and reproduced artifact comparisons pass; `just test`, `just lint`, and `just doctest` pass. No shipped definition has been rewritten.

## Phase 2 — Implement Typed Registration

### Wave 4 — Context and Descriptor Core (Sequential)

- [ ] **Build Context**
  - Implement the spike-selected dispatcher, DefinitionContext, private TypeDesc, fallible registration, rust_path!, and DefinitionError (R1–R5, R7). One module group owns insertion order and collisions; repeated canonical registration does not reorder entries.
  - Use existing OpenAPI-compatible schema settings. Preserve explicit registrations of types not mentioned by endpoints and existing nested-def extraction; omit only container components and primitives/free-form JSON components.
  - Implement register_as and schema_name without merging distinct conflicting types. Test HuggingFace Commit/CommitInfo and Ollama progress alias behavior in small fixtures without converting those APIs yet.
  - Derive descriptor serde, reserve optional structural data, and validate every reference against the destination registry, Rust path, and wrapper shape. Matching imported/deserialized data may pass; foreign, forged, and unsupported structural values must not bypass validation.

### Wave 5 — Adapter and Boundary Coverage (Parallel)

- [ ] **Adapt Registry**
  - In schematic-definitions, adapt ordered neutral registrations to SchemaRegistry without a reverse crate dependency. Add a module-group assembly seam alongside current literal-based lookup.
  - Test shared Ollama/EMQX groups versus isolated groups, identical registration order, nested schema extraction, and explicit unreachable-from-endpoint registrations. Avoid introducing a second manually maintained registry or API inventory.
- [ ] **Test Registration**
  - In schematic-define, implement compile-fail coverage for string construction, the structural setter, misspelled rust_path, and mismatched typed paths according to the spike ruling.
  - Cover all container/primitive/Value combinations, nullable wire shape, map key rejection, aliases, renamed schema names, collision aggregation, descriptor serde, and matching/forged destination-context data. Execute the input matrix below through the public validation result.

Phase gate: R1–R7 have focused passing coverage; old definitions still generate the original baseline; new typed fixture paths pass the predecessor's isolated generated-crate compile check. Integration does not depend on public string descriptor constructors.

## Phase 3 — Add Required-Fact Builders

### Wave 6 — Builder Implementation (Sequential)

- [ ] **Build Endpoints**
  - Implement constructors for every authored verb and typestate for description/success. Fallible build handles invalid facts and repeated success calls with API/endpoint diagnostics (R8–R10, R13).
  - Add non-exhaustive Success/ErrorSet placeholders and exactly the R13 forms: four success constructors, matching shorthands, ErrorSet::none, and API default_errors. Lower to existing ApiResponse in one place; no definition names that storage type.
  - Cover JSON, multipart, URL-encoded, text, and binary request authoring. Preserve present text/binary runtime gaps and output-specific diagnostics; do not claim new builder syntax fixes body transport.
- [ ] **Build APIs**
  - Add API typestate and coherent auth calls, with no Default on values/builders. Auth choice cannot silently overwrite a prior choice; conflicting explicit env mappings fail in either setter order. Keep explicit AuthPolicy authoritative for supported multi-method cases.
  - Support existing headers, version, params, module_path, request_suffix, and inheritance precedence. Build endpoints before borrowing the context for the API builder; validate references and aggregate collisions at API build.
  - Audit the exact Default allow-list; add EndpointParams::new. Keep external literals temporarily legal until the emitter and providers migrate.

### Wave 7 — Fixture Migration and Compile Contracts (Parallel)

- [ ] **Migrate Fixtures**
  - Convert external literal-based generator tests/helpers to builders in output, api_struct, module_docs, validation, definition_gen, and HTTP-client tests. Split file ownership before delegation; provider definitions remain unchanged.
  - Update the relevant R27 literal-surface test to required-fact coverage; keep pagination-table/legacy-presence tests until actual removal. Migrate crate-internal exporter examples only where needed for the public authoring contract.
- [ ] **Prove Typestate**
  - Add compile-fail cases for Endpoint/RestApi Default, missing endpoint description/success, missing API description/auth, and unavailable non-none ErrorSet construction.
  - Test every accepted success/request form, second-success error, auth order/conflicts, intentional multi-method policy, parameter/header precedence, and literal-equivalent fixture output.

Phase gate: R8–R13 pass on new fixtures, all old providers remain baseline-equivalent, and future-vocabulary reserved names remain unused. Non-exhaustive enforcement is still deferred deliberately.

## Phase 4 — Connect Runtime and Query Forms

### Wave 8 — Definition Forms and Runtime Wiring (Parallel)

- [ ] **Add Definition Forms**
  - Own schematic-define changes for endpoint CrossOrigin replacement, request-id/secret sugar, auth_policy, fixed-value query inputs, pager-managed markers, and narrow wire-name/Rust-name mapping (R14–R16, R29–R30).
  - Store fixed values and adoption markers as serializable definition data. Reject fixed/managed conflicts, unmatched adoption, conflicts with caller/API defaults, and managed required inputs lacking the pagination-authored default. Reuse the resolved-strategy validator and PagerManagedRequiredWithoutDefault diagnostic.
  - Retain a path-query warning for existing providers; prepare the hard-error path but enable it only with completion of all nine migrations in Phase 6.
- [ ] **Wire Runtime Emission**
  - Own generator/runtime changes consuming those agreed fields. Apply endpoint policy to redirects and followed links, and emit request-id/secret markers in the predecessor's runtime form without new redaction logic.
  - Insert fixed query values exactly once through the normal encoder, expose no setters for them, and preserve GitLab's git_ref Rust name for wire name ref. A caller or pagination strategy cannot overwrite fixed values.
  - Represent fixed inputs in OpenAPI/Postman or produce the established output-specific diagnostic with API, endpoint, feature, and target. One target's unsupported feature does not block other outputs.

Checkpoint: integrate the two branches only after agreeing on field signatures. Replace R27's no-endpoint-cross-origin assertion with R14 coverage; retain its no-GraphQL assertion.

### Wave 9 — Verify Public Behavior (Sequential)

- [ ] **Verify Behavior**
  - Use two local origins in the predecessor's L1 harness: an endpoint allow_origins override reaches the second origin; a sibling under same_origin refuses it and the second origin records zero requests for that sibling. Cover invalid exact origins and caller tightening.
  - Assert request-id normalization/header validity and secret-marker emission, without duplicating predecessor redaction tests. Cover Some(AuthPolicy) through generated execution/emission.
  - Assert fixed/default/caller query composition, encoding once, repeated-key conflicts, serialized fixed values/markers, matched pager control, and required-managed rejection through public results. Old providers still match baselines.

Phase gate: R14–R15 and new R29–R30 fixtures pass, output-specific validation remains honest, and full area checks pass without enabling the hard path-query error prematurely.

## Phase 5 — Convert the Import Emitter and OpenAI

### Wave 10 — Repeatable Builder Imports (Sequential)

- [ ] **Convert Emitter**
  - Change definition_gen.rs to emit fallible builders, nominal registration support selected by the spike, explicit typed registrations in preserved order, and the generic overlay call. The importer may use crate-internal construction without public Default.
  - Normalize imported query-bearing paths in the shared import route before builder construction. Constants become stored fixed inputs; supported placeholders become caller query inputs. Reject malformed/ambiguous expressions with endpoint context. Keep the vendored OpenAI source unchanged.
  - Implement a tested recorded transform only where reproduction established its necessity. Keep handwritten overlay files outside the emitter's output ownership; no OpenAI-specific emitter branch.
- [ ] **Convert OpenAI**
  - Regenerate OpenAI's definitions once using the new emitter, preserving the seven beta=true query inputs. Convert its hand-written pagination overlay to endpoint-level assignments; unknown overlay endpoint ids remain errors.
  - Route the OpenAI assembly/registry through its context; explicitly preserve old component names/order except the ruled container removals. Do not schedule another OpenAI definition rewrite in Phase 6.

### Wave 11 — Import Regression Coverage (Sequential)

- [ ] **Prove Reimports**
  - For a small real importer-produced fixture, compile/evaluate emitted builder source and compare its public RestApi/registry result with the in-memory import. Re-import twice to prove determinism and execute JSON/YAML path-normalization matrices.
  - Test a generic configured overlay call and unchanged overlay source; re-import OpenAI into a temporary destination and prove resolved endpoint pagination survives.
  - Run R10/R24 for OpenAI and all nine-path fixture cases; log actual beta query changes per endpoint/output. Check the isolated generated crate. Refresh committed affected artifacts through the generator so ordinary drift tests remain green.

Phase gate: R22 passes, OpenAI's original count/order and pagination resolve identically, all seven constants reach provider URLs once, and artifacts are equal or individually explained against history while matching final committed output exactly.

## Phase 6 — Migrate Remaining APIs and Remove Legacy Surface

### Wave 12 — Verified Provider Batches (Sequential)

- [ ] **Convert Providers**
  - Use Phase 1's smallest-first inventory for every remaining REST API. For each batch, migrate its definition function, ordered typed registrations, lookup assembly, headers/auth/params, and every endpoint in one pass. Neither WebSocket definitions nor type-module stored-form tests are wholesale rewritten.
  - Move pagination exceptions to endpoint builders and author each API's small strategy function in its own module. Convert all 35 Gitea/GitHub/GitLab/Bitbucket declarations; preserve page/per_page setters or the predecessor's equivalent caller controls. No shared presets.
  - Convert Gitea recursive=true and GitLab ref query input within their API batches. Migrate inventoried GitLab call sites in the same batch, preserving git_ref where possible; log unavoidable source changes.
  - Handle aliases/re-exports/foreign types with register_as/rust_path and schema_name only where needed. Preserve explicit registry order and nested components; enumerate dropped container components by API and name.
  - End each batch with original endpoint count/ordered-id assertions, R10 JSON comparison, R19 pagination-resolution comparison, R24 generated-byte/file-set comparison, isolated compile checks, and affected dependent tests. Regenerate affected committed outputs before ordinary drift tests. Fix failures before the next batch and log the last completed API for unattended recovery.

### Wave 13 — Legacy Removal and Enforcement (Sequential)

- [ ] **Remove Legacy Interfaces**
  - After the final provider migration, remove pagination exception tables and their validation, PaginationStyle, PaginationResponse, with_pagination, and the two legacy EndpointParams fields. Retain OpenAI overlay id validation.
  - Remove hand-typed registry filling and string-vs-string completeness checks; replace them with destination-context validation while preserving strict OpenAPI completeness coverage. Module-group assembly returns definitions/registry together for all lookup and generator entry points.
  - Repeat oauth_scopes/auth_policy and Schema.module_path reader scans. Remove oauth_scopes from live definition, exporter extension, emitter, and consumers; retain/test auth_policy. Apply/log PD21 without breaking WebSocket readers.
  - Enable the hard `?` error in shared validation/build/import boundaries only now, after confirming all nine paths are migrated. Assert diagnostics name API, endpoint, and offending path.
- [ ] **Enforce Construction**
  - Apply non_exhaustive to Endpoint/RestApi after a workspace consumer scan confirms migration. Add external literal compile-fail coverage and source scans forbidding Endpoint literals, ApiResponse/ApiRequest/Schema constructors, json_type strings, and string registrations in provider authoring, with only narrow type/test exceptions.
  - Complete R27's six-entry retirement ledger: name each replacement and preserve pagination-resolution coverage, EndpointParams allow-list coverage, and no-GraphQL coverage. Remove tautological tests/tables only with a logged reason or behavioral replacement.
  - Verify removed identifiers have no live consumers across the workspace; distinguish historical documentation from code. Confirm every API exposes its expected count and id list.

Phase gate: R6, R12, R16, R18–R21, R23, R27–R30 pass across the complete provider set. Every generated URL for the nine cases preserves ordinary decoded path/query behavior; GitLab reserved characters and Unicode form one correctly encoded value. No legacy pagination types, query-bearing paths, or external construction literals remain live.

## Phase 7 — Regenerate and Verify Dependents

### Wave 14 — Final Artifact Integration (Sequential)

- [ ] **Regenerate Artifacts**
  - Run the generator for the complete Rust/schema, OpenAPI, and Postman trees; never hand-edit generated source. Review complete file-set and byte differences against the Phase 1 baseline and log each intended difference by API, endpoint/registry entry, file, and cause.
  - Final drift L1 compares against final committed artifacts with no allow-list. Historical definition normalization remains narrow and reviewable; migration baselines remain available through review.
  - Run `just test`, `just lint`, `just doctest`, `just test-schema`, and `just lint-schema` in schematic. Include new schematic-runtime checks using the actual predecessor recipes; current area recipes omit that not-yet-created package.

### Wave 15 — Dependents and Platform Evidence (Parallel with Separate Build Targets)

- [ ] **Verify Dependents**
  - Resolve actual package names from inventory and run root `just test {pkg}` for sniff/lib, biscuit-speaks/lib, model-citizen/lib, homelab/lib, eversolo-integration, arcam-amp-integration, sony-receiver-integration, and unfolded-integration-helper, plus any additional discovered direct consumers. Include schematic-oauth and all five WebSocket definition builds/tests.
  - Compile relevant lib/bin/test targets plus declared examples/benches and changed reverse dependencies. Run corresponding lint recipes. Fix only migration-induced call-site failures and log each touched consumer; an unrelated failure is recorded separately.
- [ ] **Verify Platforms**
  - Load the os skill before choosing remote procedures or reporting a platform unavailable. Reuse qualifying evidence per required cell; otherwise run required checks on macOS, Linux, native Windows, and WSL2 using documented non-interactive host access and isolated target directories.
  - Exercise generated paths, compile-fail fixtures, temporary artifact paths, query encoding, and affected dependents on each required environment. Record exact commands, features, results, hosts, and missing evidence; never report one host's pass as four-platform coverage.
  - Use only documented unattended credential procedures. If a host is inaccessible, record the blocker and evidence still required; do not add CI cells or silently weaken acceptance. Run existing relevant L2 suites only via just test-l2 with background/no-focus harness visibility.

Phase gate: generated-schema and all affected consumer checks pass; the evidence ledger distinguishes passing environments from unverified ones. Full four-environment evidence is required for an unqualified completion claim.

## Phase 8 — Update Authoring Guidance and Close Evidence

### Wave 16 — Documentation and Closure (Parallel)

- [ ] **Update Authoring Guidance**
  - Update schematic-define README/rustdoc, area and relevant package READMEs, definition/registration and IO topic pages, schematic-define/Schematic skills, and `.claude/commands/create-schematic-api.md` to teach builders, explicit context assembly, typed aliases, auth coherence, pagination, and fixed query inputs (R26).
  - Provide compact compiling examples for ordinary types, alias registration, containers, and one API definition; explain required facts and errors. Use a Mermaid flow for context → endpoint builds → API validation → registry/output assembly. Current-behavior docs never name feature/fix directories.
  - Review docs/comments on every behavior-changed symbol and remove drift. Update dependency docs only for actual crate additions/removals. Search for stale literal/string/pagination authoring patterns, permitting only explicit historical mentions.
- [ ] **Close Evidence Ledger**
  - Map every R1–R30 requirement to concrete tests, scan results, artifacts, and platform evidence. Record every PD default, non-exhaustive outcome, dispatcher choice/bounds, structural carrier, fallible signature, Default allow-list, field disposition, normalization, retired test, dependent edit, and unexplained failure as it occurs.
  - Complete all 16 review-agenda entries, including accepted-gap candidates (optional auth, wider auth dual-source cleanup, HuggingFace stub shapes, orphaned types) and the already accepted convenience-method omission. No gap is silently claimed fixed.
  - Document the implemented R13 method table, reserved names, final descriptor/serde/adoption contracts, and facts needed to refresh `2026-10-15-schematic-definition-vocabulary` and `2026-10-16-schematic-response-model-graphql` after author review 1. Do not implement their features or refresh them prematurely.

Final checkpoint: run focused documentation/doctest and changed-test checks, then the required area/dependent gates for any new executable changes. Review requirement coverage and historical differences; no broad test rerun without a new change or unresolved concern. Report **implementation complete, ready for review** only when evidence supports it, explicitly identifying justified exceptions or unverified environments. Do not commit, move the feature, or run `just complete`.

### Input Robustness Matrix

This applies to the descriptor deserialization boundary and the changed OpenAPI import normalization inputs. Plain output snapshots, serializers, and builder fields are not configuration inputs; do not impose a new format policy on every definition field or build the later self-contained bundle. Phase 1 binds the descriptor's final field spellings and importer-supported formats; add a column for every additional load-bearing field the selected implementation actually reads. Tables below define the new boundary contracts rather than changing unrelated importer behavior.

Descriptor JSON is read into data and then validated through the public destination-context/API build result. The fixture is a real context-produced descriptor referencing a registered nominal type, with separate context-produced primitive and wrapper fixtures for their positive cases. Reserved structural data round-trips internally but cannot become authored structural registration in this feature.

| Shape (JSON) | Component/schema name | Generated Rust path | Wrapper/reference shape metadata | Reserved structure |
| --- | --- | --- | --- | --- |
| Control | Matching registered nominal name builds | Matching registered public path builds | Matching wrapper and leaf registration builds | Explicit null builds as the authored None value |
| Absent | Reject missing required identity | Reject missing path | Reject missing required wire-shape metadata | Accept as reserved None, separately asserted from explicit null |
| Explicit null | Reject for nominal reference; native non-component case uses an explicit discriminator | Reject | Reject | Accept reserved None by this format's documented optional-field contract |
| Wrong whole type | Reject | Reject | Reject | Reject |
| One wrong element | Scalar: an array value is rejected | Scalar: an array value is rejected | Reject mixed malformed nested shape | Reject malformed nested TypeRef data |
| Every wrong element | Scalar: reject array value | Scalar: reject array value | Reject wholly malformed shape | Reject wholly malformed structural data |
| Empty | Reject empty nominal name | Reject empty path | Reject invalid empty wire shape | Reject empty object/array lacking a valid variant |
| Duplicate key | Reject | Reject | Reject | Reject |
| Trailing/invalid content | Reject whole input | Reject whole input | Reject whole input | Reject whole input |

Use separate non-component discriminator columns if the spike's descriptor representation requires them. Missing versus null may share a documented outcome only for the reserved optional structure; neither may silently clear a required identity. A syntactically valid forged name, path, shape, or Some(structure) that does not match the destination context yields an API/endpoint/type validation error. Matching deserialized references remain accepted. Private fields and successful serde parsing alone are not proof.

For imported query-bearing paths, the edited field is the path-map key under an existing operations object; existing method/operation facts stay fixed. Parameters below refer only to declarations the normalizer consumes to reconcile query inputs; a format permitting omitted parameters defines that absence explicitly. Run the table independently for JSON and YAML through the public import result, not a parsing helper.

| Shape (JSON and YAML, separate runs) | paths object/path key | parameters collection (if consumed) | Each consumed parameter name | Each consumed parameter location |
| --- | --- | --- | --- | --- |
| Control | Fixture imports with expected normalized path and fixed/caller query values | Valid declarations preserve query controls | Matching wire name appears in public definition/request | query location matches public request |
| Absent | Missing paths rejects/no usable API; deleting the selected key removes that endpoint, never invents it | Explicitly means no declared parameters; unambiguous path constants still normalize | Reject missing name | Reject missing location |
| Explicit null | Reject null paths/operation value | Reject; do not read as omitted declarations | Reject | Reject |
| Wrong whole type | Reject non-map paths or non-string key | Reject non-array value | Reject | Reject |
| One wrong element | Reject malformed entry among valid paths | Reject malformed entry among valid parameters | Reject malformed name among valid declarations | Reject malformed location among valid declarations |
| Every wrong element | Reject all-malformed entries | Reject all-malformed entries | Reject all malformed names | Reject all malformed locations |
| Empty | No usable API is rejected at definition validation; empty path key rejected | Empty array is explicit zero declarations | Reject empty name | Reject empty location |
| Duplicate key | Reject duplicate authored path keys where forbidden | Reject duplicate collection key | Reject duplicate field key; contradictory repeated wire declarations fail validation | Reject duplicate field key |
| Trailing/invalid content | Reject whole document | Reject whole document | Reject whole document | Reject whole document |

Keep legal repeated parameters in distinct locations distinct; this table does not forbid valid OpenAPI behavior. If reconciliation reads another field such as a fixed default, schema type, or Rust-name extension, Phase 1 must add its complete column and outcomes before implementation. Do not accept null via Option or silently filter malformed declarations. Import normalization also covers malformed query expressions, unsupported placeholders, duplicate fixed keys, and fixed/caller collisions with endpoint-specific errors. If enforcing these affected-input contracts exposes an existing parser defect, fix it at the shared read boundary or log a concrete unmet requirement; do not treat a one-off negative case as complete coverage.

- [ ] **Verify Input Matrices**
  - Phase 2 owns the descriptor JSON matrix; Phase 5 owns the JSON and YAML import matrices. One test per format walks its entire applicable matrix from a real-tool fixture, making one edit per cell and asserting the public definition/API validation outcome. The unedited control proves the fields affect output; assert endpoint presence, schema/path/shape, and emitted query values rather than just serde success.
  - Preserve duplicate keys and trailing content using textual mutations; a parsed JSON Value cannot represent duplicate keys. Use fixture variants for legal non-component/wrapper states. Unknown malformed data must not turn into empty successful output.
  - Audit affected read paths for `#[serde(default)]`, Option where absence/null differ, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` swallowing load-bearing errors. Any review finding reports every other cell in its row and column. Matrix evidence is required at the owning phase's gate, not deferred to final review.
