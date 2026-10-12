---
total_phases: 7
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
source_files_during_phase_1:
  - schematic/gen/src/cargo_gen.rs
  - schematic/gen/tests/common/mod.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/justfile
  - .config/nextest.toml
  - .github/workflows/ci.yml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_1:
  - schematic/README.md
  - .github/ci/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - schematic/skill/SKILL.md
  - .claude/skills/rust-devops/ci-cd.md
packages:
  - schematic-gen
  - test-toolkit
---

# Schematic generation integrity implementation plan

## Phase 1: Establish the baseline and compile guard

### Necessary Rules

This plan implements `2026-10-11-schematic-generation-integrity`. Success means safe whole-group partial generation, independent validation and publication for Rust/OpenAPI/Postman, unique OpenAPI operation ids, complete request-side transport, and an offline test that compiles freshly generated code on every normal L1 run. Every R1–R35 requirement must have observable evidence; missing evidence remains explicitly incomplete. The terminal state is **implementation complete, ready for review**.

The work proceeds through baseline capture, validation, OpenAPI repair, safe publication, query separation, request-side support, and final verification. Establish the compile guard before changing generation behavior. Capture actual pre-change request lines before changing URL assembly. Keep supported baseline fixtures green; introduce unsupported fixtures alongside their implementation.

Adopt the specification's planning defaults below unless an author ruling is already recorded. Record the actual choices in `implementation-log.md` as they occur, including deviations, dependencies, outside-area edits, and the complete human review agenda. The defaults do not substitute for execution authorization.

| Default | Adopted ruling |
| --- | --- |
| 1–2 | Public `ValidationDiagnostic { severity, api, endpoint, feature, output, message }`, public severity enum, `Display` and `Serialize`. Render `severity=<severity> api=<api> endpoint=<endpoint> feature=<feature> output=<output>: <message>`; API/group diagnostics use `endpoint=<none>` and identify every contributing API. Print diagnostics on stderr; warnings alone succeed; no new CLI flag. |
| 3 | Inventory comes from provably owned module files/directories matched to current definitions. Parse existing `lib.rs` to confirm modules and public exports. Reject unknown owned modules, missing members, conflicting layouts, inconsistent exports, or retained exports that differ from current definitions. Preserve unowned content outside planned paths and retained WebSocket inventory/exports. |
| 4 | One compatibility function compares runtime-bearing shared files byte-for-byte. A difference with any unregenerated retained module requires full generation before mutation. Export-list changes use inventory checks. Include WebSocket shared support; introduce no speculative persisted marker. |
| 5 | Prepare all content and mutations first. Publish group modules, shared runtime files, `prelude.rs`, then `lib.rs` last. Track manifests, layout conversions, writes, and deletions; report completed and pending mutations on failure. No multi-file transactional guarantee. |
| 6 | Reject raw duplicate endpoint ids within one API. Across group members, compare actual emitted names with suffixes and namespaces; supported Ollama and EMQX overlap remains valid. |
| 7 | Merge compatible operations before allocating ids. Stable registry API order, document path order, and a fixed method order determine the first owner. Reserve every original id; later collisions use PascalCase API-name prefixes and numeric suffixes. Rewrite affected in-document links and scan final grouped and ungrouped documents. |
| 8 | Generate an isolated temporary crate using the manifest builder and explicit local dependency paths, serialized safely for TOML on Windows. Copy the relevant existing lockfile into temporary storage and let Cargo resolve the temporary package entry with `cargo generate-lockfile --offline`; check with `cargo check --offline --locked`. No online fallback. Clear inherited archive-only target/runner settings and use a reusable child target distinct from the outer build, respecting configured target storage. Serialize child builds. |
| 9 | Remove ignored compile/clippy duplicates if the new guard supersedes them; retain only distinct coverage. Adjust `test-e2e` and its callers accordingly, using nextest for remaining runtime tests. |
| 10 | Request runtime tests live in the freshly generated fixture crate, reuse the compile helper, and run through nextest against a local mock server. Production definitions receive no fixture endpoints. |
| 11–12 | Text has `body: String`; binary has `body: Vec<u8>`; both convert to `RequestBody::Raw`. Keep `RequestParts` as a tuple, adding a fifth `Vec<(String, String)>` query element. Headers/cookies remain in its headers element. |
| 13–14 | Optional header/cookie fields reuse `QueryParamType` and `with_*` builders; defer required/style/explode/enum fidelity. Cookies use declaration-ordered `name=value` pairs joined by `; `, preserve legal ASCII values, and percent-encode disallowed UTF-8 bytes with uppercase hex and `%20`. Validate names. Header/cookie arrays use comma-separated values; query arrays use repeated keys. Reject unsupported nested arrays. |
| 15–16 | Temporary generator-local parameter-key environment resolution reads `env_auth` only when effective policy enables fallback. Explicit `with_api_key_param(value)` wins and stores its key outside `Debug`. Leave `EnvMapping`'s shape unchanged; update `legacy_env_mapping_for`'s comment to explain resolution. Log both temporary surfaces for review and replacement by the runtime/envelope follow-up. |
| 17 | Two ordinary `slow_` L1 guard tests: supported fixture generation/compile/runtime coverage, and all 18 real APIs. Add unsupported fixtures progressively; include a negative control through the same helper. |
| 18 | Regenerate `checks/cargo-machete.txt` only if an existing regeneration recipe is found and the conditional edit is authorized; otherwise preserve it and log its stale reference. Initial recipe searches found no obvious match; confirm during implementation. |
| 19 | Current behavior documentation goes under `schematic/docs/generation/`: partial generation/validation and request-side behavior/query separation. Examples and a generation-flow Mermaid diagram must serve a developer new to the repository. Docs never name or link a feature/fix snapshot. |
| 20 | Use the specified kebab-case `Feature` ids: `text-request-body`, `binary-request-body`, `api-key-param`, `header-parameter`, `cookie-parameter`, `duplicate-endpoint-id`, `duplicate-method-path`, `invalid-type-syntax`, `duplicate-operation-id`, `query-in-path`. Keep `query-in-path` warning severity in one place. |
| 21 | New L1 modules: `fresh_output_compile.rs`, `partial_generation.rs`, `request_side_features.rs`; register each in `gen/tests/l1/main.rs` because `autotests = false`. Cargo-invoking tests use `slow_`; fixture helpers follow the rust-testing layout rules. |
| 22 | Partial OpenAPI/Postman generation writes only expanded selected groups, under current resolved module filenames. Do not unify the divergent module resolvers here; log any discrepancy such as `PagerDutyApi`. |

Additional rulings and boundaries:

- The specification is marked `draft-spec`. This planning request authorizes writing this plan, not implementing it. Before an unattended implementation, establish the author's confirmation of pre-authorization items 1–8 below. `yolo: "true"` is execution metadata, not permission. Item 9 is already confirmed by D57. If confirmation is absent, leave dependent actions pending and record that exact boundary; do not infer approval from elapsed time.
- Restate the requested grants: (1) regenerate schema/OpenAPI/Postman; (2) delete only the stale `schematic/gen/schematic/` tree; (3) conditionally regenerate `checks/cargo-machete.txt`; (4) conditionally add missing mock-server/serial-test dev-dependencies and update dependency docs; (5) create temporary crates and Cargo build outputs outside the checkout; (6) change generated `RequestParts`/`into_parts`; (7) remove/rename ignored e2e tests and adjust their recipe; (8) conditionally make only a mapping-default-permitted additive change to `schematic-define`; (9, confirmed) run the fresh-output tests before CI archiving and exclude them explicitly from archive execution and expected listings.
- Editing dependent crates, endpoint literals, adding selector syntax, creating `schematic-runtime`, widening CI scope, or committing is outside the implementation authorization. No commit, push, lifecycle move, or `just complete` is part of this plan. A dependent break is logged as a contradiction of the premise; do not repair its source.
- Per-output independence overrides a global abort: an unsafe or invalid Rust output must remain untouched while independently valid requested OpenAPI/Postman outputs can publish. Process all outputs and aggregate their results into a nonzero exit if any failed.
- R33's live count is nine distinct endpoints, with diagnostics for each requested output. Tests must distinguish nine unique endpoint warnings from the larger number of lines in a multi-output invocation.
- Ordinary cookies retain declaration order; replacing a name retains its position, and a new credential cookie is appended. API static cookies are combined with endpoint static cookies, then set parameters, then the credential; later stages own matching case-sensitive names. Reject unparseable static Cookie content before sending rather than silently dropping it. Log this merge-order elaboration for review.
- R24 requires effective-policy handling, not a new general credential-precedence architecture. Preserve unrelated authentication behavior and defer broader runtime/redaction work. When multiple explicit policy methods interact, derive behavior from `effective_auth_policy` and record any ambiguity rather than adding credential rules silently.
- Reuse `artifact_drift` for OpenAPI/Postman byte comparisons; it already runs unconditionally and uses runtime-remapped paths. Add the missing Rust-source comparison. Existing `just check-drift` uses a stale ignored-test invocation; use the actual nextest module invocation for acceptance and update the relevant recipe/docs if needed, without formatting unrelated files.
- Every phase closes with `just test` and `just lint` in `schematic/`. Do not run `cargo fmt`. Inspect lint recipes before running them; any formatter invocation needs explicit authorization and must not be hidden inside the checkpoint. Update relevant comments and current docs alongside behavior changes, not solely at closure.

### Spikes and input scope

No separate performance spike is needed. The specification rules it out: measure elapsed time for one ordinary local fresh-output run on one host, recording cache state and fixture/full-output durations. The synthetic trait timing is not an estimate of this build. Do not schedule repeated timings or a cross-OS benchmark campaign. Baseline request capture and live-count traversal below are one-time evidence tasks, not recurring spikes.

The Input Robustness Matrix is **not applicable to the adopted design**: API definitions are typed Rust data; retained inventory parses generated Rust source; OpenAPI/Postman scans inspect generated output documents; the manifest builder is a writer. Cargo owns manifest/lockfile parsing, and this plan copies lockfiles without introducing a custom loader. If implementation instead adds a manifest, lockfile, or configuration reader, make its matrix a prerequisite before that reader lands: one column per load-bearing field per supported format, positive real-tool fixture control, and absent/null/wrong-whole-type/mixed-element/all-wrong-element/empty/duplicate/trailing-invalid outcomes asserted through the public decision. Do not substitute parser-return assertions or silently default invalid inputs. Review `serde(default)`, `Option<T>`, `filter_map(as_str)`, `unwrap_or_default()`, and `.ok()` on that reader. Log this design departure.

### Execution conventions

Waves are globally numbered. Each wave is a dependency barrier and a concurrent implementation step: all its tasks finish before the next wave. Single-task waves are sequential. Parallelizable waves name file ownership; one integrator owns shared registration, manifests, snapshots, recipes, and generated artifacts. Subagents, if used during implementation, receive narrow briefs stating that the entire session tree is non-interactive. This planning run does not dispatch implementation agents.

Load the Schematic, rust-testing, rust-devops, and OS skills for their respective contracts. Use sniff and its skill for package/host discovery. Use toolkit-remapped repository paths, ephemeral ports, and fixture-owned processes; no live provider traffic. Use nextest, not `cargo test`, for unit/runtime execution. Compile subprocesses sharing the child target run sequentially, with runner-visible coordination across nextest processes. Preserve terminal/browser focus during existing L2 checks.

### Wave 1 — Baseline evidence (parallelizable)

- [x] **Capture wire baseline** — Task 1.1; owns isolated request fixtures and baseline evidence.
  - Before generator edits, capture actual local-server request lines from the current generated client for representative existing query endpoints. Cover ordered scalar/array parameters, spaces, `+`, `%`, Unicode, empty values, repeated keys, literal/base URL queries, and a legacy embedded-query template. Preserve `%20`, escaping, and ordering as golden bytes; do not derive expectations from the replacement encoder.
  - Keep representative provider requests local using custom base URLs and controlled credentials. Record fixture provenance and which existing query/path tests cover the remaining endpoints. Do not alter endpoint literals.
- [x] **Record scope baseline** — Task 1.2; owns `implementation-log.md` and read-only registry traversal.
  - Record author-confirmation state, all adopted defaults, existing worktree changes, relevant recipe behavior, and current package test/lint failures before attributing them to this feature.
  - Traverse the live registry once: endpoint/API/group totals, text/binary/multipart bodies, parameter-key auth, header/cookie parameters, and nine embedded-query endpoints. Compare with 18 APIs, about 519 endpoints/265 OpenAI, zero unused features, and about 29 multipart; reconcile “six POST empty bodies” only if the data supports it (R32).
  - Repeat the repo-wide `RequestParts`/`into_parts` consumer search outside generator/schema; discover actual dependent package names with sniff. Inspect build-job publication, archive filters/expected listings, completion accounting, and the stale-tree references. Do not change CI here.

### Wave 2 — Offline safety net (sequential)

- [x] **Install compile guard** — Task 1.3; depends on Tasks 1.1–1.2.
  - Add a reusable fixture helper and `gen/tests/l1/fresh_output_compile.rs`; register it in `main.rs`. Exercise the public generation entry point into an external `tempfile` crate, using `cargo_gen` for its manifest. Fix manifest path serialization with TOML-safe strings, including Windows separators/quotes, without adding a custom configuration reader.
  - Start the two `slow_` guard tests with supported JSON/form/multipart/query/auth fixtures and all 18 real APIs. Supply fixture types/dependencies inside the temporary crate. Copy lockfiles and resolve/check strictly offline with explicit manifest/output/target paths. Missing Cargo/cache is an explanatory failure, never a silent skip.
  - Prove checkout isolation using the relative file list and bytes of a small copied input tree before/after. Keep targets/output elsewhere; do not hash/scan the monorepo or require a clean worktree. Compare fresh Rust file lists/bytes with committed schema sources and reuse artifact drift tests for the exported documents (R16–R20).
  - Add a deliberate type-error negative control through the same helper; assert compiler failure and useful stderr. Ensure it is selected with the guard tests in CI. Remove superseded ignored compile/clippy tests, repair `test-e2e` callers if needed, and preserve distinct remaining coverage.
  - Establish sequential child-build execution across nextest processes and bounded child cleanup. Fixture runtime execution uses offline nextest; add unsupported cases only when Phase 6 implements them.

### Wave 3 — CI placement (sequential)

- [x] **Place builder checks** — Task 1.4; depends on Task 1.3.
  - Under R35's confirmed authority, add a package-scoped build-owner step before archive production in the current `ci.yml`/producer path. Read the resolved plan; run only for planned schematic-gen work, without creating an environment, cell, gate, or parallel scheduler.
  - Use one explicit selection for the cargo-invoking guard/negative-control/fixture-runtime tests, and its matching exclusion in every archive consumer's executed **and expected** listings, including native Windows and WSL2 paths. Do not exclude every `slow_` test indiscriminately.
  - Persist build-job reports for those tests and integrate the existing completion accounting so exclusion cannot hide an unexecuted test. Load CI ownership rules before touching producer/accounting support files; confine changes to R35's placement contract and log every outside-area edit.
  - Measure one normal local guard run now; record CI duration separately when a run is available. If R35 cannot fit the current report contract within the narrow authority, log the exact mismatch and leave R35 incomplete rather than changing CI policy.

### Validation checkpoint

Run `just test` and `just lint`; show the new non-ignored `slow_` tests in normal L1 selection and absent from sanity. Run relevant existing CI contract tests via their owning canonical recipes. Inspect `just ci-local --plan` and workflow/filter/report diffs without triggering a full hosted run solely to discover semantics. Record actual CI evidence separately from the permitted plan-plus-diff fallback. Phase 1 is complete when baseline goldens/counts exist, fresh supported/full output compiles offline, the negative control fails as expected, and R35's build/archive/report contract is demonstrated.

## Phase 2: Validate each output independently

### Wave 4 — Validation foundation (parallelizable)

- [ ] **Define scoped diagnostics** — Task 2.1; depends on Phase 1; owns a new diagnostics module and its tests.
  - Implement the public reusable diagnostic/severity/feature surface from defaults 1, 2, and 20. Prove construction from outside `validation.rs`, both severities, API/group scope, endpoint omission, rendered format, and serialized severity (R9).
- [ ] **Check emitted identities** — Task 2.2; depends on Phase 1; owns validator helpers and accept/reject tables.
  - Check duplicate raw ids within an API, actual emitted names across shared groups, and one centralized `(method, normalized_path)` uniqueness key. `/items/{id}` and `/items/{name}` collide on the same method; different methods do not. Add no selector model (R12–R13).
  - Parse Rust request/response types with the Rust type grammar, definitions module paths with their supported path grammar, and output filenames as safe single components. Accept generic/qualified types; reject traversal/absolute filenames.
  - Use exactly the emitters' identifier transformations for path/query/header/cookie/body/multipart/helper fields, builders, and methods. Record newly found gaps. Keep valid Ollama/EMQX overlaps and legitimate `/api/api` compositions accepted (R11).
  - Implement warning-only embedded-query detection with one severity constant; exact base/path duplication under the composition rule may warn, never fail by substring. Test nine unique live endpoints for every output (R33).

### Wave 5 — Output orchestration (sequential)

- [ ] **Integrate batch validation** — Task 2.3; depends on Tasks 2.1–2.2; owns validation entry points and CLI integration.
  - Validate the whole batch for Rust, OpenAPI, and Postman independently before writing each root. Keep Rust-only type errors out of export validators. Ensure public generation entry points and the CLI follow the same boundary (R8–R10).
  - Add a mixed valid/invalid batch asserting zero mutation for its failing output; malformed Rust type syntax must fail Rust but permit both export artifacts. Assert every requested output is attempted, successes are named, errors yield final nonzero status, and warnings alone succeed.
  - Update affected validator comments and diagnostic docs with examples and ownership of the public surface.

### Validation checkpoint

Run focused validator/CLI tests, then `just test` and `just lint`. Preserve the Phase 1 compile guard. Evidence must distinguish API-local route/id errors, emitted-name errors, export-only gaps, and warning-only success. Phase 2 closes when no requested output can partially publish a batch rejected by its validator.

## Phase 3: Make exported operation ids unique

### Wave 6 — OpenAPI repair (sequential)

- [ ] **Allocate unique operations** — Task 3.1; depends on Phase 2; owns `gen/src/export/openapi.rs` and its merge tests.
  - Preserve compatible same-method/path auth-alternative merging; reject incompatible operations for OpenAPI only. Use normalized route identity without turning cross-API overlap into a Rust-wide error (R13).
  - Track operation ownership through the grouped merge. Reserve all originals before deterministic renaming, including an adversarial original already equal to a proposed prefixed name; allocate numeric suffixes without displacing it (R14).
  - Update owner-scoped in-document link `operationId` references. Do not globally replace an ambiguous raw id in links from different source documents. Final-scan grouped and ungrouped documents, including link targets where affected. Test determinism and unchanged unique per-API ids.
- [ ] **Verify artifact identities** — Task 3.2; depends on Task 3.1; owns uniqueness/artifact checks and generated artifacts.
  - Extend the existing passive artifact corpus coverage to reject duplicate ids in every committed OpenAPI document. Reuse the drift scans rather than duplicating them.
  - Regenerate using the generator and explain the Ollama id changes. Other OpenAPI content changes only where R14 requires; Postman and Rust remain unchanged unless live generator behavior explains them (R15). Do not hand-edit artifacts.
  - Update current export behavior docs and relevant merge comments.

### Validation checkpoint

Run merge/adversarial/link-reference tests and nextest's `artifact_drift::` selection; then `just test` and `just lint`. The full-output compile/drift guard must remain green. Every exported document has unique ids and compatible security alternatives remain preserved.

## Phase 4: Prepare and publish safe partial generation

### Wave 7 — Publication foundations (parallelizable)

- [ ] **Resolve retained inventory** — Task 4.1; depends on Phase 3; owns inventory/group/compatibility helpers under `gen/src/output/`.
  - Expand REST selections to complete current registry groups. Establish owned file/directory inventory plus parsed `lib.rs` modules/exports; validate retained declarations against current group exports. Preserve WebSocket members/support for REST and WebSocket selections.
  - Distinguish absent/empty roots from nonempty unsafe roots. Reject unknown owned modules, missing members, unclassifiable files, layout conflicts, corrupt exports, and enumeration/read errors. Preserve unrelated unowned content; do not follow symlinks or recursive cleanup through them (R1–R5, R7).
  - Centralize shared-runtime byte comparison; unchanged support passes, changed support with retained modules requires full generation. Test full generation after refusal and preserve the extension point for a future persisted marker (R34).
- [ ] **Prepare mutation plans** — Task 4.2; depends on Phase 3; owns a separate preparation/publication helper and portable failure-injection seam.
  - Represent prepared files and ordered mutations sufficiently to report completed/pending writes/deletions, including manifests and layout conversions. Finish validation, assembly, syntax checks, and formatting in memory before publication.
  - Check ownership for every overwrite/delete/shared/manifest target. An unowned occupant fails before mutation; a file/directory conversion containing unowned content remains intact. Prepare replacements before obsolete-path cleanup (R3, R6).
  - Preserve atomic per-file writes, publish `lib.rs` last, and make dry-run preparation use the identical plan/diagnostics while creating no directories. Inject an I/O failure **during publication after preflight**, rather than an invalid planned filename that preflight correctly rejects.

### Wave 8 — Partial generation integration (sequential)

- [ ] **Integrate safe publication** — Task 4.3; depends on Tasks 4.1–4.2; owns `output/mod.rs`, CLI/export integration, manifests, and registration.
  - Connect group expansion, retained inventory, per-output preparation, and publication. Rebuild shared files/prelude/lib exports from retained plus regenerated groups; preserve every unselected module byte-for-byte (R1–R2).
  - Restrict partial cleanup to selected groups; full generation may remove obsolete owned groups. Rust/OpenAPI/Postman roots establish safety independently. Partial artifacts rewrite only selected expanded groups; artifact-only failures do not prevent other valid roots publishing (R3–R8).
  - Include generated `Cargo.toml` and WebSocket shared files inside the preflight/report boundary; no earlier CLI side-write may bypass it. Aggregate final statuses honestly and suppress crate-updated success messages after partial failure (R6, R10).
- [ ] **Prove partial safety** — Task 4.4; depends on Task 4.3; owns `gen/tests/l1/partial_generation.rs` and partial-generation docs.
  - Generate all APIs into a temp tree, then select one Ollama member; assert both group members regenerate, all retained modules stay byte-identical, all 18 API exports remain, and the resulting tree compiles via the serialized fresh-output helper.
  - Test owned stale deletion alongside hand-written files/bin content, unowned occupied targets including manifests/shared files, corrupt/missing inventory, export mismatch, shared-runtime mismatch, symlink protection, layout conversion conflicts, and empty roots. Refusal preserves relative paths and bytes.
  - Cover group-limited OpenAPI/Postman publication, artifact-only roots, dry runs, repeated successful partial generation, and a portable mid-write failure with completed/pending lists. Retry must safely complete or request full generation without additional mutation; no promise of universal crash recovery.
  - Update `schematic/gen/README.md`, `generate-one` usage/docs, and the partial-generation/validation topic page with examples and the preparation/publication Mermaid flow.

### Validation checkpoint

Run partial-generation/filesystem/CLI tests, then `just test` and `just lint`. Compile the retained-plus-regenerated crate. Verify write/deletion reporting and no-mutation refusals on the actual public entry point, not only helper returns. Preserve original ownership proofs and log any compatibility/inventory concern for review.

## Phase 5: Separate queries without changing the wire

### Wave 9 — Query transport (sequential)

- [ ] **Split query transport** — Task 5.1; depends on Phase 4; owns request-parts templates, `into_parts`, path/query generation, and client URL assembly.
  - Repeat the external-consumer search before changing the public tuple. Add unencoded query pairs as its fifth element; declared arrays remain repeated ordered keys. Substitute path parameters as before, keeping legacy embedded suffixes untouched (R28).
  - Apply existing percent encoding exactly once when appending pairs, preserving `%20`, existing literal/base URL queries, reserved characters, empty values, and order. Do not blindly switch to reqwest form-query encoding (R29).
  - Update every template and request-parts consumer, including helpers/tests. Keep header/cookie values in headers. Do not migrate definitions or implement URL redaction now.

### Wave 10 — Query evidence and regeneration (sequential)

- [ ] **Verify query equivalence** — Task 5.2; depends on Task 5.1; owns query/path/http tests, fixture runtime cases, and regenerated schema.
  - Update `query_params_codegen.rs`, `path_substitution.rs`, and `http_client.rs` for the new shape without weakening existing encoded-wire expectations. Compare raw local-server request lines with Phase 1 goldens and explain coverage of every existing declared-query endpoint.
  - Assert query pairs are unencoded, encoded once only at assembly, and separate from path/headers. Confirm warning-only legacy paths keep their previous escaping/order.
  - Run `just generate`; explain schema diffs as the query split and prior operation-id repair. Update query examples, tuple docs, and downstream follow-up notes in the implementation log.

### Validation checkpoint

Run raw-wire golden tests and fresh full-output compile/drift tests, then `just test`, `just lint`, `just test-schema`, and `just lint-schema`. Previously captured bytes are unchanged. A dependent failure remains a logged blocker; no dependent source is edited.

## Phase 6: Deliver every declared request-side input

### Wave 11 — Independent request features (parallelizable)

- [ ] **Generate raw bodies** — Task 6.1; depends on Phase 5; owns body-field/conversion generation and raw-body fixture cases.
  - Emit text/binary body fields and convert to `RequestBody::Raw` with declared content type. Test empty/nonempty text and non-UTF-8 bytes; verify the existing send branch receives and transmits them exactly (R21–R22).
  - Add fixtures to the compile guard alongside codegen assertions and local mock-server byte/header tests; no production definition edits.
- [ ] **Generate named parameters** — Task 6.2; depends on Phase 5; owns separate header/cookie field and serialization helpers plus parameter fixtures.
  - Emit optional typed fields/builders, preserve exact wire names, and join scalar/array values using the adopted mapping. Validate nested-array shapes, sanitized-field/builder collisions, header names/values, and cookie names with scoped diagnostics.
  - Merge headers case-insensitively: endpoint static headers override API defaults; set parameters override matching endpoint headers; unset parameters preserve them. Reject invalid supplied values before network send without echoing contents (R25).
  - Build one Cookie header from API/endpoint static cookies and supplied parameters, preserving distinct names and case-sensitive replacement. Verify declaration order, Unicode/space/separator encoding, arrays, unset fields, and CR/LF resistance using exact server-observed bytes (R26).
  - Expose only the small merge hooks needed by parameter-key auth; do not introduce a general runtime abstraction.

Tasks 6.1 and 6.2 must keep edits to shared `into_parts.rs`, validator registration, fixture manifests, and test-module registration with the integrator. Separate helpers/case files allow concurrent work without overlapping writes.

### Wave 12 — Authentication and integration (sequential)

- [ ] **Inject parameter credentials** — Task 6.3; depends on Tasks 6.1–6.2; owns client auth helpers/builders and their fixture cases.
  - Resolve `effective_auth_policy`, including explicit methods/fallback overrides. Add temporary explicit-key storage/setter outside `Debug`; explicit keys win, first nonempty enabled env fallback wins, and disabled fallback stays disabled. Align `headers_satisfy_fallback`/missing-auth handling (R23–R24).
  - Merge query keys into the separate query collection and cookie keys into the existing single Cookie header. The resolved credential replaces every matching query/cookie name case-sensitively and preserves unrelated parameters. Location-specific encoding occurs once.
  - Test explicit/env/empty-then-nonempty/missing credentials, disabled fallback, policy overrides, same-name conflicts, and header/query/cookie sentinel Debug safety for clients and request structs (R27). Prefer isolated child environments; if mutating process environment, use `EnvGuard` and appropriate serial coordination, accounting for nextest process boundaries.
  - Update `legacy_env_mapping_for`'s comment; leave `EnvMapping` fields unchanged. Validate invalid HTTP header values without credential-bearing diagnostics. Record temporary surface choices for review 1.
- [ ] **Complete fixture coverage** — Task 6.4; depends on Task 6.3; integrator owns helper orchestration, manifests, shared files, and documentation.
  - Merge feature work and run the freshly generated fixture crate through nextest with wiremock on port 0. Cover JSON, multipart, URL-encoded, text, binary, supported Bearer/header-key/Basic/OAuth, query-key/cookie-key auth, and query/header/cookie parameters (R19, R21–R29).
  - Keep exactly the two principal `slow_` guard tests and the negative control, unless distinct coverage justifies another test. Child compilations/runtime tests stay inside the R35 builder selection; cheap codegen assertions stay on archive consumers. Update both executed/expected exclusions if names change.
  - Reuse fixture-manifest wiremock/serial_test dependencies; add a dependency only if actually absent and authorized. Regenerate through `just generate` and explain any changed real output despite zero-count fixture-only features.
  - Update request-side topic documentation/READMEs with body, parameter, cookie serialization, auth fallback, and Debug examples; mark the credential setter/env scan temporary. Review changed symbol docs/comments and log drift corrections.

### Validation checkpoint

Run generated fixture wire tests, two offline guard tests, and the negative control; then `just test`, `just lint`, `just test-schema`, and `just lint-schema`. Assert bytes/headers/URLs received by the local server, not merely source strings. Prove each invalid-value case fails before send, and no sentinel key appears in scoped Debug output. Review R35 selections again after fixture expansion.

## Phase 7: Remove stale output and close verification

### Wave 13 — Final maintenance (parallelizable)

- [ ] **Remove stale output** — Task 7.1; depends on Phase 6 and confirmed deletion authority; owns only the stale tree and authorized reference maintenance.
  - Confirm the five tracked residue files under `schematic/gen/schematic/` are neither workspace members nor test/CI inputs, then remove that tree. Verify absence on disk and deletion entries in the diff; unstaged `git ls-files` is not an absence test (R30).
  - Search maintained references, excluding snapshot specs/plans. Confirm whether `checks/cargo-machete.txt` has a regeneration recipe; regenerate only under the conditional grant, otherwise preserve the line and record a finding.
- [ ] **Audit behavior docs** — Task 7.2; depends on Phase 6; owns current README/docs/skills audit, with log integration reserved to the integrator.
  - Finish the drift pass across `schematic/README.md`, `schematic/gen/README.md`, `schematic/docs/generation/`, dependency docs, `schematic/skill/`, and applicable `.claude/skills/schematic-define/` entries. Keep examples aligned with implemented behavior, and docs independent of feature lifecycle paths.
  - Check every behavior-changed symbol's comments; remove/fix drift assuming code is correct. Record choices, departures, unmigrated query warnings, compatibility limitations, and temporary auth/query tuple surfaces for follow-up refresh.

### Wave 14 — Final regeneration (sequential)

- [ ] **Regenerate final artifacts** — Task 7.3; depends on Tasks 7.1–7.2; sole owner of generated outputs.
  - Run `just generate` for full schema/OpenAPI/Postman output. Inspect file lists and content diffs; explain each change by R14, R28, or a live supported feature. Confirm no generated file was hand-edited and no dependent/endpoint literals changed (R15, R20, R31).
  - Run `just test`, `just lint`, `just test-schema`, `just lint-schema`, the nextest `artifact_drift::` tests, and existing `just test-l2` with no focus gain. Use the actual working drift invocation rather than a stale ignored-test recipe.

### Wave 15 — Evidence and review handoff (parallelizable)

- [ ] **Verify dependent packages** — Task 7.4; depends on Task 7.3; owns local dependent build/test evidence.
  - Using actual discovered package names, run root `just test {pkg}` for sniff, biscuit-speaks, model-citizen, and the five homelab crates, including their library/CLI packages as applicable. Record build and test results individually; no dependent-source edits.
  - Stop the affected dependent verification branch and log any break contradicting the no-consumer premise. Continue independent verification, but leave R31 incomplete until resolved within authorization.
- [ ] **Verify platform evidence** — Task 7.5; depends on Task 7.3; owns OS/CI report evidence.
  - Load OS/build-host instructions and discover configured hosts through sniff. Reuse qualifying evidence per package/environment/tier; run missing required tests on available macOS/Linux/native-Windows/WSL2 hosts using approved non-interactive procedures. Record compile and behavioral evidence separately.
  - For archive-only WSL2/native-Windows reproduction, explicitly verify the guard exclusion/expected listings and run the offline fixture/full compile guard on a toolchain-bearing host for that environment when available. Linux builder execution proves placement for WSL archives; it must not be reported as a native WSL compile run.
  - Confirm build reports precede archiving, guard tests never execute on hosted archive consumers, and completion records account for the remaining archive tests. Use CI reports if available, otherwise the R35 plan/workflow-diff fallback and clearly state what is unproven. Do not create a hosted run or add labels merely to discover behavior.
  - Record host availability and unmet criteria; do not reduce the four-platform contract, treat cross-compilation as runtime proof, install toolchains on archive consumers, or change CI event scheduling.

Avoid competing child builds during Tasks 7.4–7.5: run independent package/host checks concurrently only when their Cargo target storage is separate; otherwise serialize shared-target builds. This is an execution constraint, not a new test tier.

### Wave 16 — Acceptance reconciliation (sequential)

- [ ] **Reconcile review evidence** — Task 7.6; depends on Tasks 7.4–7.5.
  - Finish `implementation-log.md` with all 22 defaults actually used, all non-prescribed choices/deviations, feature counts/reconciliation, guard/CI timing, dependencies/outside-area files, platform results, dependent results, generated diffs, comment drift, and every specification human-review agenda item.
  - Map R1–R35 to test names/results or recorded measurements using the ledger below. Identify missing grants, failing requirements, unavailable evidence, and any archive/report mismatch explicitly; a green local tree alone is insufficient.
  - Report implementation complete, ready for review only when all implementation tasks are done, distinguishing unresolved verification from passed evidence. Leave lifecycle location unchanged and do not commit.

### Acceptance ledger

| Requirements | Primary observable evidence |
| --- | --- |
| R1–R2 | Task 4.4 retained-tree/group/export assertions and offline compile of partial output. |
| R3–R7 | Tasks 4.1–4.4 ownership, inventory, compatibility, mutation-report, dry-run, and root-specific safety tests. |
| R8–R10 | Task 2.3 whole-batch/no-mutation/mixed-output CLI tests; Task 2.1 public diagnostic tests. |
| R11–R13 | Task 2.2 accept/reject/name/normalized-route tables; Task 3.1 compatible/incompatible export overlaps. |
| R14–R15 | Tasks 3.1–3.2 deterministic allocation/link tests, committed uniqueness scan, generated Ollama repair. |
| R16–R18 | Task 1.3 offline/isolation/normal-selection/negative-control results. |
| R19–R20 | Task 6.4 complete fixture guard; Tasks 1.3/7.3 full Rust and existing export drift comparisons. |
| R21–R27 | Tasks 6.1–6.4 fresh generated-client local-server bytes, auth/merge/error behavior, and Debug sentinels. |
| R28–R29 | Tasks 1.1/5.1–5.2 pre-change goldens, separated parts, and unchanged server-observed URL bytes. |
| R30–R31 | Tasks 7.1/7.3–7.5 stale-tree diff, generator-only regeneration, dependent and platform evidence. |
| R32 | Task 1.2 registry measurements and reconciliation recorded in the log. |
| R33–R34 | Tasks 2.2/4.1 nine distinct warning endpoints, shared severity, compatibility refusal/full recovery tests. |
| R35 | Tasks 1.4/6.4/7.5 builder reports, ordering, matching archive execution/expected exclusions, and completion evidence. |

Final checkpoint: all phase checkpoints and the acceptance ledger are reconciled, current documentation describes the shipped behavior, and the review log gives the author a concrete record of every adopted decision and remaining evidence gap.
