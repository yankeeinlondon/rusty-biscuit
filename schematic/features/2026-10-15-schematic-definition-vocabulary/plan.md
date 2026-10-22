---
kind: plan
total_phases: 6
created: 2026-10-11
phase: 2
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/contracts.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/inventory.md
skills_files_updated_during_phase_1: []
packages:
  - schematic-define
  - schematic-definitions
  - schematic-gen
  - schematic-oauth
source_files_during_phase_2:
  - schematic/define/src/auth.rs
  - schematic/define/src/builders.rs
  - schematic/define/src/components.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/metadata.rs
  - schematic/define/src/models/aliases.rs
  - schematic/define/src/models/enums.rs
  - schematic/define/src/models/mod.rs
  - schematic/define/src/models/structs.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/parameters.rs
  - schematic/define/src/openapi/export/security.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/parameters.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/params/param_def.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/request.rs
  - schematic/define/src/serde_presence.rs
  - schematic/define/src/types.rs
  - schematic/define/src/websocket/mod.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/composition.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/define/tests/vocabulary.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/gen/src/asyncapi_import.rs
  - schematic/gen/src/codegen/api_struct/constructors.rs
  - schematic/gen/src/codegen/api_struct/fields.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/api_struct/variant.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/auth.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/output/assemble/lib_rs.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/ws_codegen/client.rs
  - schematic/gen/src/ws_codegen/docs.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/oauth/src/manager/authorization_code.rs
  - schematic/oauth/src/manager/mod.rs
  - schematic/oauth/src/manager/revocation.rs
  - schematic/oauth/tests/wiremock_flows.rs
docs_updated_during_phase_2:
  - schematic/docs/definition-builders.md
  - schematic/docs/definition-snapshots.md
  - schematic/docs/generation/validation.md
  - schematic/docs/headers/README.md
  - schematic/docs/pagination.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/schematic-define/SKILL.md
---

# Schematic definition vocabulary implementation plan

## Work and successful completion

Extend the definition builders supplied by `2026-10-14-schematic-definition-api` with typed paths, endpoint authentication, deprecation, descriptive rate-limit/retry metadata, structural composition, and self-contained definition bundles. Replace positional Basic environment configuration and flat OAuth2 configuration. Finally, preserve required query parameters, enums, and wire serialization in generated Rust, excluding pager-adopted parameters, and migrate affected callers in the same change.

Completion means R1–R14 are implemented and verified through definition validation, generated source, compiled fixtures, actual mock-server requests, and exports. Every declared feature reaches each selected output that supports it; otherwise the output emits the existing D10 diagnostic with API, endpoint, feature, and target. Unsupported output features do not prevent supported targets from generating. Shipped builders keep their call signatures, definition authoring changes stay within R12, Basic output remains unchanged, and every other generated difference has a specific explanation. All shipped bundles load and export without the Rust registry. Schematic and affected dependents build and pass on macOS, Linux, native Windows, and WSL2.

**Planning baseline:** source inspection on 2026-10-11 found the legacy `RestApi::env_username`, unit `AuthStrategy::Basic`, flat `OAuth2Config`, and a `QueryParamInfo` that drops required/style/explode metadata. The prerequisite specification is marked unimplemented and requires refresh. Consequently, this is a dependency-ordered implementation plan, conditional on Phase 1's prerequisite refresh, authorizations, and credential-source ruling. It does not claim those decisions or future builder APIs already exist. Leave the supplied specification unchanged; record implementation departures in `implementation-log.md`.

Waves are numbered uniquely across the plan. Tasks within one wave may run concurrently only with disjoint file ownership and satisfied prerequisites. The wave coordinator owns shared-module integration and checkpoint validation. Every later wave waits for the preceding checkpoint. Single-task waves are sequential integration steps. Each task starts with a checkbox and has a short name; phase completion also requires its validation checkpoint. Maintain the implementation log as decisions occur, including the last completed phase, rather than reconstructing it at the end.

## Phase 1: Resolve prerequisites and establish evidence

### Necessary Rules

1. **Prerequisite refresh — required before implementation.** Verify the landed/reviewed outcomes of generation integrity, runtime envelope, pagination, and especially `2026-10-14-schematic-definition-api`. Bind this plan to the actual builder signatures, typed context and structural descriptor, diagnostics, isolated compile harness, JSON snapshots, schema identities, pager adoption/default data, and `fetch_page()` surface. Confirm `EndpointSpec`, retained `auth_policy`, removed `oauth_scopes`, and existing path-template validation. Do not rebuild or re-test those earlier features as new scope here. If they have not landed, log the missing prerequisite and stop dependent implementation.
2. **Replacement credentials — defaulted.** Adopt the specification's recommendation for independent endpoint credential sources and log it as an adopted default; the author reviews it afterwards. Adopted default: preserve `.auth(policy)` and add a policy-plus-optional-`EnvMapping` form; a present endpoint mapping replaces the API mapping entirely, otherwise compatible API mapping is reused. Select credentials only for the chosen method, preserve explicit credential versus environment fallback precedence, and fail before send for missing/incompatible sources. Never reinterpret bearer bytes as Basic/query/cookie credentials. PD4 is adopted on this basis.
3. **Defaults — adopt and log PD1–PD10.** Use the specification's metadata shapes, `ParamDef` for typed paths, additive `deprecated: false`, same-context composition registration, embedded Basic environment lists, version-1 JSON bundles, query serialization rules, per-grant OAuth2 payloads, and unconditional query migration. PD4 follows rule 2. Preserve Required and Supported PKCE semantics. No compatibility generator mode, runtime retries, convenience client methods, optional-auth variant, general auth redesign, importer rewrite, or later-spec fidelity work.
4. **Adoptions — planning default.** Adopt GitHub's named rate-limit headers, epoch-second reset, and optional Retry-After header only. Add no provider-wide retry statuses, attempt counts, or backoff assumptions. Bitbucket/EMQX authoring changes are permitted only if the landed Basic builder cannot preserve its call form; flag that conflict with R10 rather than silently redesigning builders. All other new vocabulary is fixture-only. Retain unchanged OpenAI authoring sources.
5. **Authorization — granted.** Starting the run confirms the specification's pre-authorizations: generated artifact updates, breaking OAuth2 consumers, necessary dependent migrations (including the mechanical Bitbucket/EMQX Basic call and test edits Phase 1 found), R12 adoption sites, and fixture/baseline additions. Nothing beyond them is authorized. No commits, pushes, CI changes, live provider calls, or lifecycle moves; new dependencies require an unavoidable concrete need and dependency-document updates.
6. **Input contracts — planning defaults.** Bundle loading supports JSON only; YAML/TOML loaders are not added. Older definitions may omit newly additive vocabulary with specified defaults, but explicit null is rejected unless a field's documented nullable contract permits it. Present empty collections are validated as empty collections, never converted into absence. Reject duplicate JSON object keys throughout the loaded document before lossy `serde_json::Value` map construction. Preserve arbitrary JSON examples/defaults and schema data rather than imposing these rules on the contents of user payloads. Expand the matrix below against the landed serialized types before loader work.
7. **Spikes — none scheduled.** The specification explicitly identifies correctness work and no measured performance uncertainty. Do not add a benchmark or repeat an experiment later. Inspect prerequisite contracts and prove them with retained regression fixtures. If inspection exposes a genuinely unanswered feasibility question, log the exact question and proposed one-time spike for an author ruling; any wider measurement is separate proposed scope.

### Wave 1: Preparation

- [x] **Refresh contracts**
  - Record each prerequisite outcome and actual symbol/file mapping in `implementation-log.md`; identify absent prerequisites and authorizations. Reconcile proposed names without rewriting the specification.
  - Record the credential-source ruling or its blocked status. Inventory supported output diagnostics and establish a requirement-to-evidence ledger for R1–R14.
  - Complete the input matrix for all load-bearing inherited `RestApi`, endpoint, parameter, auth/mapping, schema, and model-catalog fields reachable through the bundle. State absence/null/empty behavior individually; do not infer safety from serde derives.
  - Own the log and contract ledger. This task may run alongside the following read-only inventory, but mutation waits for the checkpoint.

- [x] **Capture baseline**
  - Regenerate Rust, OpenAPI, and Postman into isolated temporary directories using the landed generator and prove byte equality with committed outputs before modifying code. Record exact command forms, tree identity, registry/API counts, and results; resolve existing drift first.
  - Preserve 4a's original bare-definition snapshots. Prepare a narrow comparison normalization for newly defaulted fields, removed unused `env_username`, and equivalent Basic representations. Keep a deliberate description mutation that must fail the comparison. New bundle fixtures are separate artifacts.
  - Inventory every affected generated request and caller: constructors, literals, setters, enum assignments, `Default::default()`, and `..Default::default()`. Include pager ownership when identifying required fields and list repository call sites, not just totals.
  - Verify Basic/OAuth2 users and dependent package names from manifests: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`, and `schematic-oauth`; add actual direct reverse dependencies discovered by inventory. Use `sniff` and its skill for repository/host discovery.

**Checkpoint:** the post-prerequisite baseline is reproducible; callers and package names are listed; snapshot mutation fails; actual APIs and input outcomes are recorded. Missing prerequisites block dependent work. Record available build environments using the `os` skill; no OS evidence is claimed from this planning pass.

## Phase 2: Implement validated definition vocabulary

### Wave 2: Independent type modules

- [x] **Define metadata**
  - Add PD1 rate-limit/retry types and serde contracts in a dedicated definition module. Validate legal HTTP header names, reset units, status bounds 100–599, ordered inclusive ranges, attempts including the initial attempt, positive delays/base, and cap at least base.
  - Test boundary failures and complete value preservation. Endpoint overrides replace the entire declaration; absence inherits. No runtime readers or enforcement.

- [x] **Reshape OAuth2**
  - Replace the flat configuration with the three PD9 per-grant payloads, retaining explicit `grant_type` spellings, optional revocation metadata, scopes, client authentication, and grant accessor behavior.
  - Introduce the authorization-code PKCE type without `NotUsed`. Validate nonempty well-formed URLs, required grant URLs, contradictory non-null fields, and malformed serialized values. Explicitly define treatment of null grant-inapplicable fields in the input ledger.
  - Add compile-fail proof for unrepresentable combinations and loader-facing fixture cases. Own `define/src/oauth.rs`; minimum consumer compilation repairs occur in Wave 3, and complete behavior validation follows in Phase 3.

- [x] **Bridge composition**
  - Extend, never replace, 4a's descriptor carrier if necessary. Add same-context `one_of`, `any_of`, and `all_of` registration and the catalog's serde support. Coordinate context file ownership before concurrent execution.
  - Reject unregistered/foreign-context members, empty lists, and schema-name conflicts. Make explicit structure authoritative and diagnose conflicting registry schemas.
  - Create distinguishable oneOf/anyOf serde fixtures and a struct satisfying allOf; prove serialization/deserialization semantics without assuming an untagged enum guarantees oneOf exclusivity. Log any descriptor extension.

### Wave 3: Builder integration

- [x] **Integrate builders**
  - Integrate new fields/reexports/prelude and optional builder methods in `define/src/types.rs`, parameter types, and the landed builder/context modules. A single owner edits shared files and makes the minimum mechanical Basic/OAuth2 consumer and fixture adaptations needed to keep all affected packages compiling before this phase closes. Full consumer behavior and export changes follow in Phase 3.
  - Typed paths reuse `ParamDef`: force required/Simple through builders; reject deserialized required=false, non-Simple style, Array/Json, duplicates, empty/duplicate enum values, and sanitized-name collisions. Validate declared names exactly against normalized placeholders; no declarations preserves legacy untyped paths. Repeated placeholders produce one field in first-template-occurrence order.
  - Add endpoint/parameter deprecation, metadata inheritance/replacement, and the author-approved endpoint auth representation. Preserve all prior builder signatures and additive absent-field defaults.
  - Reshape Basic to carry complete ordered username/password `EnvList`s, remove stored `env_username`, and replace positional password reads. Keep `AuthMethod::Basic`/`EnvAuthStrategy::Basic` source-neutral, explicit mapping/policy precedence, and shared WebSocket compatibility.
  - Validate adopted required query parameters against spec 3's actual strategy defaults (type-valid and nonempty where applicable); error names API, endpoint, strategy, and parameter. Keep adoption data usable by Phase 5.

**Checkpoint:** new builder forms, validation failures, metadata bounds, composition semantics, Basic precedence/fallbacks, and OAuth2 payload contracts have focused L1 evidence. Definition, generator, OAuth2, and shared WebSocket consumers compile after Wave 3's mechanical repairs. Run affected L1 checks and lint, re-run the baseline comparison, and explain each difference. No shipped authoring rewrite is accepted.

## Phase 3: Connect runtime, generators, and exporters

### Wave 4: Consumers with separate ownership

- [ ] **Resolve authentication**
  - Own generated runtime/auth assembly and request dispatch. Resolve effective endpoint policy before environment lookup/token acquisition; alternatives are alternatives, and replacement replaces API policy.
  - Public requests suppress all known API/endpoint credential locations, including supported programmatic injections and conflicting raw headers. Preserve ordinary headers/cookies and shared client state for the following authenticated call. Retain prior secret/redirect protections.
  - Mock generated clients for Authorization/custom-header/query/cookie auth, injected credentials, absent environment credentials on public calls, and authenticated calls afterward. Test distinct replacement sources and incompatible/missing sources under the recorded ruling. An unsupported representation emits its target diagnostic rather than relocating credentials.
  - Update shared Basic and OAuth2 generated credential consumers. Coordinate with OAuth and exporter owners rather than editing their files concurrently.

- [ ] **Migrate OAuth consumers**
  - Own `schematic/oauth/src/types.rs`, `manager/*`, and OAuth tests. Replace field access with grant-aware operations; migrate serialized fixtures explicitly.
  - Verify all grants, both PKCE modes, default scopes, client-authentication methods, revocation, existing refresh flows, and `oauth/tests/wiremock_flows.rs`. No shipped OAuth2 definition adoption.

- [ ] **Connect exports**
  - Own `define/src/openapi/export/*`, extension models, and `gen/src/export/*`. Emit typed path types/descriptions/required flags, endpoint/parameter deprecation, endpoint security, and all metadata fields.
  - For shared paths, emit a deterministic common parameter declaration and operation overrides for differing `(name, in)` metadata. Normalize `{+name}` for OpenAPI without changing Rust reserved expansion.
  - Register override-only security schemes with deterministic collision checks; export public `security: []` and Postman `noauth`. Never export environment variable names as credential values.
  - Export composition references with conflict checks. Preserve existing concrete Postman examples and emit the composition-specific unsupported diagnostic; invent no example payloads.

- [ ] **Connect Rust output**
  - Own request-struct path generation and module docs, enum machinery, and remaining OAuth2 generator consumers such as shared assembly. Agree file boundaries with the runtime owner first.
  - Emit typed path fields/docs and template-ordered constructors; stringify scalar/enum wire values before existing path encoding; reject non-finite numbers before send. Preserve legacy untyped constructors. Build enum naming/wire access once for path and later query use, including suffix/grouped-module collisions.
  - Typed paths remove `Default`; optional legacy requests retain existing behavior. Emit deprecation as documentation only, not Rust attributes. Show every metadata field in docs without adding runtime behavior.

### Wave 5: Integrate consumers

- [ ] **Prove outputs**
  - Integrate all generator changes and compile generated fixture clients through spec 1's isolated harness. Exercise integer/boolean/number/enum paths, repeated/reserved placeholders, differing GET/DELETE metadata, duplicate declarations, and field/module collisions.
  - Add GitHub's restricted R12 metadata adoption and output assertions. Check that runtime source does not read metadata types and review the runtime diff for retry/sleep/enforcement additions.
  - Assert Basic output and environment docs remain byte-identical for Bitbucket/EMQX; retain ordered fallback tests and explicit mapping/policy precedence checks.
  - Regenerate all outputs into temporary directories. Log each API/file difference and cause, and maintain the narrow legitimate-change allow-list.

**Checkpoint:** definition, generator, OAuth2, and generated fixture code compile; local mock requests prove authentication isolation; Rust/OpenAPI/Postman evidence covers supported features and D10 gaps. Run Schematic L1 checks and lint for integrated changes.

## Phase 4: Load self-contained definition bundles

### Wave 6: Bundle implementation

- [ ] **Implement bundles**
  - Implement PD7 version-1 bundle construction from the context's canonical identities and Rust bindings. Include the full schema-reference closure: nested models, containers, aliases, arbitrary JSON, recursive references, and optional structural catalog.
  - Normalize local references to the bundle namespace and terminate cycles with a visited set. Reject unresolved/external references without fetching files or URLs; do not treat Rust paths or `Vec<Model>` spellings as component names.
  - Canonicalize map keys recursively while preserving meaningful array order. Preserve literals and environment names for equality without reading/resolving environment values. Review literals before committing generated fixtures.

  - After construction/schema contract is stable, implement the public JSON loading and isolated schema-export path. Strictly validate format version, field shapes, nested reference resolution, binding/catalog consistency, duplicate keys, and trailing content using the input matrix.
  - Errors for dangling references identify API, endpoint, and missing name. Loading never consults the Rust registry, executes Rust bindings, or compiles an isolated client.
  - Construction precedes loading within this single task; do not delegate loader work against a guessed bundle representation.

### Wave 7: Bundle evidence

- [ ] **Verify isolation**
  - Generate real-tool bundle fixtures from typed fixture APIs and all shipped REST definitions; assert equal round trips and byte-equal canonical serialization using the refreshed API count rather than hard-coding 18.
  - Run one JSON matrix-walking test with a positive unedited control and exactly one edit per cell. Assert the public load-and-export result or diagnostic, including nested fields; a parser `Ok`/`Err` assertion alone is insufficient.
  - Prove loaded bundles export without invoking registry registration, including recursive/container/alias/JSON/composition schemas. Remove one referenced entry to prove a named load error. Exercise unsupported versions and recursive map determinism.
  - Preserve bare-definition snapshots and run their narrow normalization/mutation regression independently of new bundle baselines.

**Checkpoint:** every shipped bundle loads in isolation and exports correctly; deterministic equality, all matrix cells, and malformed-reference diagnostics pass. No credentials were resolved or silently redacted. Regeneration differences remain explained.

## Phase 5: Preserve query fidelity and migrate callers

### Wave 8: Query implementation

- [ ] **Generate query contracts**
  - Retain required/style/explode/enum and pager-adoption metadata through request analysis. Update all field, constructor, setter, enum dispatch, serialization, and documentation paths together.
  - Non-adopted required fields are `T` and constructor-required even with authored defaults; optional fields are `Option<T>`. Constructor order is template-ordered paths, declaration-ordered required queries, then body; query-only endpoints get constructors.
  - Remove `Default` for typed paths or required non-pager query fields; keep it for all-optional requests. Adopted fields remain removed and never become constructor arguments; `fetch_page()` is the one-page path.
  - Reuse collision-checked enum/wire machinery for scalar and array enum values. Support only PD8 combinations; emit Rust-target diagnostics for DeepObject/Simple query styles, nested arrays, object-valued JSON, and unsupported explode combinations.
  - Encode elements once before joining: Form commas, space `%20`, pipe `%7C`; escaped literal delimiters stay escaped. `None` omits; an empty array emits `name=`. Preserve Unicode/reserved values and scalar spellings.

### Wave 9: Caller migration and fidelity evidence

- [ ] **Migrate dependents**
  - Partition the Phase 1 call-site inventory by package for concurrent subagent ownership after generated signatures stabilize. This is a fan-out task within the wave; no two owners edit the same crate.
  - Migrate only forced constructor/literal/setter/enum/Default uses, then build and run each affected package's canonical `just test {pkg}`. Include existing unaffected consumers as required by the refreshed dependent inventory.
  - Record every changed caller and the reason. A high call-site count does not authorize a compatibility mode. Any concrete integration blocker is logged as unfinished required work.

- [ ] **Verify query wire**
  - Compile fixtures and use generated clients against local mocks to assert exact queries for every PD8 combination, scalar/array enum spellings, delimiters inside values, Unicode, encoded-once values, absence versus empty arrays, and required fields.
  - Add compile-fail checks for missing required arguments and `Default` on typed-path/required-query requests; a positive all-optional fixture must compile with `Default`.
  - Use one paginated fixture with adopted and non-adopted required queries: only the latter is a constructor argument; first and subsequent pages carry valid adopted values. Reject a missing strategy default and assert OpenAI-style adopted fields are absent from `new()`.
  - Run the preserved builder-call samples and authoring diff check; only logged R12 adoptions/mechanical assertion repairs may change definition sources. Regenerate and explain every query-driven output change.

**Checkpoint:** the single generated API has correct constructors, enums, Default behavior, and observed wire encoding; pager exemption and defaults are proven; all inventoried callers compile and pass. No obsolete compatibility branch or unreviewed definition rewrite remains.

## Phase 6: Finish documentation and validate scope

### Wave 10: Documentation and evidence audit

- [ ] **Update documentation**
  - Update `schematic/define/README.md`, relevant generator/schema/definitions READMEs, headers/OAuth2/Basic pages, `schematic/docs/io/openapi-extensions.md`, and topic pages for the vocabulary and bundle contracts. Give compact examples, including constructor order, public raw-header suppression, metadata-only behavior, isolated bundle limitations, and pager rules; use Mermaid where flow is easier to follow visually.
  - Update `.claude/skills/schematic-define/SKILL.md`, affected Schematic skill guidance, and `.claude/commands/create-schematic-api.md`. Remove drifted positional Basic/flat OAuth examples and obsolete testing commands in the touched workflow guidance.
  - Review changed-symbol rustdoc/comments, preserving useful contracts and removing stale narration. Docs state current behavior without naming/linking feature/fix directories. Update dependency docs only if dependencies actually changed.

- [ ] **Audit coverage**
  - Close the R1–R14 evidence ledger; list tests, outputs, callers, matrix coverage, adoptions, descriptor adjustments, and explained artifacts. Review unsupported-output diagnostics and confirm supported targets still generate.
  - Audit load-bearing parse code for defaults/null conflation, `filter_map(..as_str())`, `unwrap_or_default()`, and swallowed `.ok()` errors. Legitimate additive defaults must match explicit matrix contracts and have public-result evidence.
  - Check no positional password lookup, unauthorized stored fields, flat OAuth2 consumers, runtime metadata readers, unexplained output differences, or extra authoring edits remain. Preserve mutation-negative snapshot proof.

### Wave 11: Final validation

- [ ] **Validate platforms**
  - From `schematic/`, run `just test`, `just lint`, `just doctest`, `just test-schema`, and `just lint-schema`. Run isolated fixture compile/compile-fail coverage through the landed harness and nextest-compatible recipe/filter. Some existing convenience recipes still invoke `cargo test`; do not use those consolidated-suite forms.
  - Run `just test {pkg}` from the root for each inventoried dependent and appropriate lint/build checks, including example/bench targets where the change affects them. Use scoped checks rather than a workspace-wide sweep.
  - Reuse qualifying exact-environment passing evidence; otherwise test changed Schematic packages and affected dependents on macOS, Linux, native Windows, and WSL2 via the existing build-host workflow. Read `os` build-host instructions before remote commands; all execution is non-interactive. Archive-safe fixtures resolve repository paths at runtime using test toolkit/harness helpers.
  - Scope OS checks to the same affected behavior; a cross-compile proves compilation only and WSL2 cannot prove native Windows. Record actual commands, hosts, results, and gaps. If a required environment cannot run, report completion incomplete with the concrete blocker.
  - New behavior needs no terminal/browser focus. If existing L2 checks are affected, use only `just test-l2` and background harness spawning. No new CI cells/configuration or live provider validation is required.

- [ ] **Prepare review**
  - Perform final temporary regeneration and verify committed artifacts match the intended output. The allow-list contains only legitimate documented output changes, with no unexplained exceptions; Basic remains unchanged.
  - Confirm each phase checkbox/checkpoint and log outcome. Include every item from the specification's human review agenda, especially metadata keep/drop recommendation, credential ruling, caller inventory, serialization isolation, pager exemptions, defaults, deviations, and OS evidence.
  - End at **implementation complete, ready for review** only when all required behavior and validation are complete. Do not commit, move the feature, run `just complete`, or mark blocked requirements complete. Subsequent specification refresh is review-cycle work for the author.

**Checkpoint:** required scoped gates pass, artifact/snapshot checks are narrow and effective, docs reflect final behavior, and all four environments have qualifying evidence. Unresolved prerequisite, author ruling, authorization, behavior, or environment gaps are explicitly reported as incomplete.

### Input robustness matrix

This matrix applies to the new JSON bundle reader and changed vocabulary deserializers used by it, including nested OAuth2 payloads. It does not impose a new YAML/TOML reader or cover OpenAPI import, Postman/OpenAPI writers, or generated query renderers. Phase 1 must expand inherited fields against the landed types before the loader is implemented; the following tables specify the new contracts rather than pretending the future nested schema is already known.

Each column is an individual load-bearing field. A row's **E** means the public bundle load/export fails with an identifying diagnostic and produces no successful export. **V** means preserve the explicit value and prove its effect through exported output or retained public bundle content. **I** means absent endpoint value inherits API metadata/policy. **N** means no optional declaration. **F** means false. **Z** means an explicitly empty collection is accepted only when its references/invariants are satisfied, and remains empty. **G** means grant-specific behavior as stated below. **NA** means that element shape is inapplicable to that field; wrong object members are covered by their own nested columns, never silently dropped. Duplicate keys and trailing garbage always fail even in optional nested objects.

Bundle JSON:

| Shape | `format_version` | `api` | `bindings` | `schemas` | `models` |
|---|---|---|---|---|---|
| Positive control | V: 1 | V | V | V | V |
| Absent | E | E | E | E | N |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E |
| Wrong one element | NA | nested E | E | E for non-schema entry | nested E |
| Wrong every element | NA | nested E | E | E for non-schema entries | nested E |
| Empty | E | E | Z: no required bindings | Z: no references | E for `{}`; catalog fields govern empty catalog |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

Binding JSON (proposed names bound to actual fields in Phase 1):

| Shape | `schema_identity` | `rust_type_path` |
|---|---|---|
| Positive control | V: resolving identity | V: registered path |
| Absent | E | E |
| Explicit null | E | E |
| Wrong whole type | E | E |
| Wrong one element | NA | NA |
| Wrong every element | NA | NA |
| Empty | E | E |
| Duplicate key | E | E |
| Trailing/invalid content | E | E |

Additive definition JSON (test API-level and endpoint-level fields independently, even when their outcomes match):

| Shape | API `rate_limit` | API `retry_guidance` | endpoint `rate_limit` | endpoint `retry_guidance` | endpoint `auth` | endpoint `deprecated` | parameter `deprecated` | params `path` |
|---|---|---|---|---|---|---|---|---|
| Positive control | V | V | V | V | V | V: true | V: true | V |
| Absent | N | N | I | I | I | F | F | Z: legacy untyped paths |
| Explicit null | E | E | E | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E | E | E | E |
| Wrong one element | nested E | nested E | nested E | nested E | nested E | NA | NA | E |
| Wrong every element | nested E | nested E | nested E | nested E | nested E | NA | NA | E |
| Empty | E | E | E | E | E | E | E | Z: legacy untyped paths |
| Duplicate key | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E |

Absence for `path` is explicitly defined as the empty declaration set for backward compatibility (R1/R10); it does not represent a populated typed path list. Older baseline snapshots exercise absent additive fields; unedited generated bundles exercise their present forms. Optional serialized fields use omission for no declaration; a loader may not conflate null with omission through `Option<T>`.

Rate-limit JSON:

| Shape | `limit_header` | `remaining_header` | `reset_header` | `retry_after_header` | `reset_unit` |
|---|---|---|---|---|---|
| Positive control | V | V | V | V | V |
| Absent | E | E | E | N | E |
| Explicit null | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E |
| Wrong one element | NA | NA | NA | NA | NA |
| Wrong every element | NA | NA | NA | NA | NA |
| Empty | E | E | E | E | E |
| Duplicate key | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E |

Retry guidance JSON:

| Shape | `retry_on` | `honor_retry_after` | `max_attempts` | `backoff` | selector `Code` | range `start` | range `end` | fixed delay | exponential `base_ms` | exponential `cap_ms` |
|---|---|---|---|---|---|---|---|---|---|---|
| Positive control | V | V | V | V | V | V | V | V | V | V |
| Absent | E | E | E | E | E | E | E | E | E | E |
| Explicit null | E | E | E | E | E | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E | E | E | E | E | E |
| Wrong one element | E | NA | NA | nested E | NA | NA | NA | NA | NA | NA |
| Wrong every element | E | NA | NA | nested E | NA | NA | NA | NA | NA | NA |
| Empty | Z: no statuses selected | E | E | E; explicit None tag is valid | E | E | E | E | E | E |
| Duplicate key | E | E | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E | E | E |

Basic environment JSON:

| Shape | `username_env` | `password_env` |
|---|---|---|
| Positive control | V: ordered fallback | V: ordered fallback |
| Absent | E | E |
| Explicit null | E | E |
| Wrong whole type | E | E |
| Wrong one element | E | E |
| Wrong every element | E | E |
| Empty | Z: no username fallback | Z: no password fallback |
| Duplicate key | E | E |
| Trailing/invalid content | E | E |

Empty Basic lists disable that environment source rather than supplying an empty credential; an explicit mapping/programmatic source may still supply credentials. Credential resolution must diagnose missing required sources before sending. Bind the table to `EnvList`'s actual serialized representation after refresh, covering both wrapper and nested list when applicable.

OAuth2 JSON (repeat each applicable column for every grant fixture):

| Shape | `grant_type` | `token_url` | `authorization_url` | `device_authorization_url` | `revocation_url` | `default_scopes` | `pkce` | `client_auth` |
|---|---|---|---|---|---|---|---|---|
| Positive control | V | V | G | G | V | V | G | V |
| Absent | E | E | G | G | N | E | G | E |
| Explicit null | E | E | G | G | E | E | G | E |
| Wrong whole type | E | E | E | E | E | E | E | E |
| Wrong one element | NA | NA | NA | NA | NA | E | NA | NA |
| Wrong every element | NA | NA | NA | NA | NA | E | NA | NA |
| Empty | E | E | E | E | E | Z: no scopes | E | E |
| Duplicate key | E | E | E | E | E | E | E | E |
| Trailing/invalid content | E | E | E | E | E | E | E | E |

For authorization-code, authorization URL and PKCE are required; absence/null fails, and PKCE accepts Required or Supported only. For device code, device authorization URL is required; absence/null fails. For other grants, grant-inapplicable fields may be absent or null (PD9's non-null contradiction rule), but any non-null value is an error and never activates another grant. Client credentials requires neither authorization URL nor device URL. Preserve shared fields and explicit enum tags. The empty-value row means a present empty string/list/object of the expected kind; grant-inapplicable empty strings are non-null contradictions.

Schema entries are OpenAPI-3.0-settings schema objects, not necessarily nonempty objects: `{}` is a valid arbitrary-JSON schema, while a missing referenced component is an error. Do not reject valid empty JSON payloads/defaults/examples or treat their null values as missing fields. Phase 1 must enumerate schema keywords and catalog/type-reference fields the new loader actually interprets (especially `$ref`, composition member lists, names, and binding identities), with separate columns and appropriate schema-specific outcomes. Composition lists must be nonempty and contain valid registered/resolving members; no bad element is filtered out. Endpoint credential mapping columns are added from the adopted PD4 default (rule 2) once its serialized shape exists.

Use fixtures emitted by the real bundle tool/context, with multiple controls where different grants/backoff/composition variants require them. One test per supported input format walks the entire completed ledger, mutating one field per cell and proving the public consequence; nested cases belong to the same matrix walk. Include invalid JSON syntax, valid-document-plus-garbage, duplicates at every interpreted object level, wrong scalar/element shapes, unsupported versions, and missing references. For any review finding, record the outcomes of the rest of its row and column before declaring the issue resolved.
