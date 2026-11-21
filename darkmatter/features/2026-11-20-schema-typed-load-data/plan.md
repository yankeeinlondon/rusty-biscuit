---
kind: plan
created: 2026-11-20
status: planned
total_phases: 5
phase: 5
agent: codex/gpt-6.2-sol
yolo: true
source_files_during_phase_1:
- simplified-schema/lib/src/expression/ast.rs
- simplified-schema/lib/src/expression/lexer.rs
- simplified-schema/lib/src/expression/mod.rs
- simplified-schema/lib/src/expression/parser.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/tests/l1/expression_syntax.rs
- darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
- darkmatter/lib/src/markdown/compose/remote_fetch.rs
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/lint.rs
- darkmatter/lib/src/markdown/compose/expression/member_check.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/prepared.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/remote.rs
- darkmatter/lib/src/markdown/schemas/slots/admission.rs
- darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
- darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
- darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs
- darkmatter/lib/tests/l1/semantic_results_never_persist.rs
- darkmatter/dmls/src/overlay/expressions.rs
- claudine/lib/src/composition/lifecycle/prewarm/classify.rs
- claudine/lib/src/dispatch/matcher.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/darkmatter/remote-cache.md
packages:
- simplified-schema
- darkmatter
- darkmatter-lsp
- claudine
- darkmatter-cli
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/expression/spread.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
- darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/policy.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/schemas/slots/admission.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
- darkmatter/lib/tests/l1/object_spread.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/diagnostics/call_check.rs
- darkmatter/dmls/tests/l1/object_spread_editor.rs
- darkmatter/dmls/tests/l1/main.rs
- claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
- claudine/lib/src/dispatch/matcher/tests.rs
docs_updated_during_phase_2:
- darkmatter/docs/inline/call-check.md
- darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/schema/translate.rs
- darkmatter/cli/schemas/output/clean.yaml
- darkmatter/cli/schemas/output/code-block.yaml
- darkmatter/cli/schemas/output/compose.yaml
- darkmatter/cli/schemas/output/delta.yaml
- darkmatter/cli/schemas/output/edit.yaml
- darkmatter/cli/schemas/output/get.yaml
- darkmatter/cli/schemas/output/graph.yaml
- darkmatter/cli/schemas/output/hash.yaml
- darkmatter/cli/schemas/output/init.yaml
- darkmatter/cli/schemas/output/list-themes.yaml
- darkmatter/cli/schemas/output/render.yaml
- darkmatter/cli/schemas/output/rm.yaml
- darkmatter/cli/schemas/output/schema-about.yaml
- darkmatter/cli/schemas/output/schema-detect.yaml
- darkmatter/cli/schemas/output/schema-triggers.yaml
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/schemas/output/set.yaml
- darkmatter/cli/schemas/output/toc.yaml
- darkmatter/cli/schemas/output/validate-refs.yaml
- darkmatter/cli/src/commands/frontmatter.rs
- darkmatter/cli/tests/l1/load_data.rs
- darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
- darkmatter/dmls/tests/l1/option_arguments.rs
- darkmatter/features/2026-11-20-schema-typed-load-data/implementation-log.md
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
- darkmatter/lib/src/markdown/compose/expression/data/contract.rs
- darkmatter/lib/src/markdown/compose/expression/data/error.rs
- darkmatter/lib/src/markdown/compose/expression/data/load.rs
- darkmatter/lib/src/markdown/compose/expression/data/locate.rs
- darkmatter/lib/src/markdown/compose/expression/data/options.rs
- darkmatter/lib/src/markdown/compose/expression/data/parse.rs
- darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/interpolation/mod.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/value_origin.rs
- darkmatter/lib/src/markdown/errors/blocks.rs
- darkmatter/lib/src/markdown/mod.rs
- darkmatter/lib/src/markdown/types.rs
- darkmatter/lib/tests/l1/diagnostic_source_provenance.rs
- darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
- darkmatter/lib/tests/l1/load_data_execution.rs
- darkmatter/lib/tests/l1/load_data_options.rs
- darkmatter/lib/tests/l1/load_data_options_passive.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/schemas/partials/functions.yaml
- darkmatter/schemas/partials/lifecycle.yaml
- darkmatter/lib/src/markdown/compose/expression/trace.rs
- darkmatter/lib/tests/l1/load_data_destination.rs
- darkmatter/lib/tests/l1/load_data_uncontracted.rs
docs_updated_during_phase_3:
- darkmatter/docs/functions/load-data.md
- darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/expressions.md
source_files_during_phase_4:
- darkmatter/dmls/Cargo.toml
- darkmatter/dmls/src/clock.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/mod.rs
- darkmatter/dmls/src/diagnostics/observed.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/properties.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/router.rs
- darkmatter/dmls/src/slot_jobs.rs
- darkmatter/dmls/tests/common/mod.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/typed_load_data_editor.rs
- darkmatter/lib/Cargo.toml
- darkmatter/lib/src/config/permissions.rs
- darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
- darkmatter/lib/src/markdown/compose/cache/store.rs
- darkmatter/lib/src/markdown/compose/cache_test_hooks.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/data/load.rs
- darkmatter/lib/src/markdown/compose/expression/editor_reads.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/read_gate.rs
- darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/remote_fetch.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/observed.rs
- darkmatter/lib/src/markdown/schemas/slots/admission.rs
- darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
- darkmatter/lib/src/markdown/schemas/slots/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/observe.rs
- darkmatter/lib/src/markdown/schemas/slots/observe_tests.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/tests/l1/editor_observation.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/observed_validation.rs
- darkmatter/lib/tests/l1/semantic_results_never_persist.rs
- simplified-schema/lib/src/file_match.rs
- simplified-schema/lib/src/format.rs
- simplified-schema/lib/src/validate.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/dmls.md
---

# Implement Schema-Typed `load_data`

## Summary and Definition of Done

Implement [the functional specification](spec.md) across Simplified Schema, Darkmatter, the `md` CLI, DMLS, and Claudine's expression consumers. Authors can load data without duplicating the destination declaration, choose all three tabular header modes, and spread objects in expressions. Normal composition validates its actual result. DMLS reports destination errors only from current, approved observations and gives completion and hover the same observed values. A direct load preserves its original data-file coordinates alongside the authored expression.

Completion requires the worked example, loader robustness matrix, schema and location parity cases, editor refresh and authorization cases, and shared parser corpus to pass through their normal entry points. Publish the updated caller skill and regenerate the catalog-derived outputs. Record implementation and test evidence after Phase 5. Tasks in a numbered wave run concurrently on disjoint files. Finish the wave before continuing to the next task or phase. Run every remaining phase in the session's worktree without interactive input.

Phase 1 is complete in commits `2b782c3d5` and `d87d0d654`. The execution cursor remains at Phase 2 because `prompts/_implement/implement-plan.md` reads `phase` as the next phase to run. The linked [implementation log](implementation-log.md) records the Phase 1 gates. The remaining phases begin with the AST, parser, and cache-only runtime already present.

## Phase 1 — Define shared syntax and transport

### Necessary Rules

1. **Represent object entries in source order.** Use `ObjectEntry::{Field { key, value }, Spread { operand }}` and `SpannedObjectEntry` with the same variants, a spanned key, and the operand span on its `SpannedExpr`. Add `Token::Ellipsis` for three adjacent dots and accept it only at the start of an object entry. Give both entry enums a borrowed `value()` accessor for walkers. Keep the explicit-key set in `parse_object_literal` across spreads. Erase spans through `SpannedExpr::erase`, and let `expression::scan` continue to use the parser as its closing-brace authority. Reject spread in `simplified/grammar.rs`'s literal-example extraction, including nested spreads. The existing `ObjectLiteral` variants in `expression/ast.rs` preserve written order, and the grammar's literal extractor currently reads those entries directly.
2. **Use ordered spread inference with open objects.** Evaluate each operand once, merging fields by insertion into the result map. Null contributes nothing. Use the operand span for passive and runtime type errors. Add a span-bearing `CallProblem::InvalidSpreadOperand` and a new `dm.expression.invalid_spread_operand` code in both the passive check and a typed `ExpressionError::InvalidSpreadOperand` projection in `report_diagnostics.rs`. Preserve the erased evaluator's public `Result<Value, ExpressionError>` entry point. Use the tracked evaluator's occurrence-to-span mapping for the runtime error, including operands inside repeated callbacks. Match an erased node to its spanned counterpart by structural traversal order, keeping each occurrence distinct. The passive visitor checks each spread in its lexical scope, including unselected branches. Reuse the authored interpolation offset mapping in `frontmatter_interpolation.rs::locate_frontmatter_span` and the normal body and lifecycle expression projections. Model known object keys with the existing `SchemaShape` properties and open keys with `PatternKey::CatchAll` with an unknown value definition for an unresolved spread. A spread arm that always writes a field replaces its previous type. An optional field, nullable arm, or open key widens the previous type with the possible incoming type. A later explicit field sets that key exactly. Reject a proven non-object, non-null operand in the passive pass and check unresolved cases on evaluation. For passive option inspection, fold literal object and null spreads in order. An opaque spread invalidates previously known overlapping keys and leaves later written keys inspectable. Make `ArgumentType::PartialObject` accept a known written option alongside opaque keys even when it has no named computed fields, so the static check still catches a proven invalid `header` after `...opts`. Unconditionally visit overwritten spread operands in call, read, and scope walkers. The existing `expression_type`, `partial_object`, and member projections already track fields and branch types, and `evaluate_expr` already owns expression evaluation order.
3. **Retain normal binding and stage order.** Declare `as?: type-definition(container) | null` and `header: boolean | string[]` in `schemas/partials/functions.yaml`. `load_data_fn` returns null for a bound null file before inspecting options or preparing a contract. For other files, use `Option<&PreparedContract>` in planning and binding, and prepare only a present non-null carrier. Test the binder's accepted arity and optional-slot layout in `call_binding/bind.rs`; both omitted `as` and a bound null use the same empty contract key. Keep the authored index of `options` at 2 and the existing argument-refinement route `data/load.rs::refine_binding_error` for dynamic bind failures. Preserve existing contract import, union, and failure handling. Keep `plan_read` and `judge_absent` as the common runtime request check. The passive `load_data` checks still run over unselected branches and use the binder's normalized constants. This follows `data/load.rs`, `data/contract.rs`, and `call_check.rs`, which currently separate binding, contract resolution, format planning, and loading.
4. **Model the three header modes explicitly.** Use `HeaderMode::{FirstRecord, None, Names(Arc<[String]>)}` in `data/options.rs`, with the outer `Option` reserved for unspecified or non-tabular header state. Keep the existing argument coercions, including string-form booleans, and validate names after binding without trimming. The ordered name list is object-row mode with no skipped record. Use `DelimitedData::headers` as record zero in both no-skip modes, even though the reader calls it a header. `string[]` count mismatches use the record start and header normalization errors use the first offending original field offset. Share name validation and a single row compatibility function between passive literal checks and `plan_read`. A known conflict for `None` or `Names` is at argument 1, while invalid options are at argument 2. For an unresolved import, defer compatibility until runtime. Use the original parsed record width and positions. Check the first-record name count before building rows, and keep `biscuit-file/lib/src/delimited.rs::read_delimited` in charge of raw quote and width errors before binding. Use its existing scan and records for both extensions. With explicit names, a first-record width mismatch points to the first record start and includes both counts. Later ragged records use the existing delimiter parser's error. The tabular parser in `data/parse.rs` already owns record width and offsets. `data/load.rs::table_rows` owns logical rows, and `data/locate.rs::table_offset` currently looks up authored header names.
5. **Normalize in binding and preserve the parse memo.** Keep `SourceKey` keyed on source identity, format, and delimiter. Key the bind memo by that source entry, optional contract identity, and effective header mode including the ordered names. The selected row mode is determined by these inputs. Arrays and objects pass through, a parsed null becomes an empty array, and a scalar becomes a single-element array before optional contract conversion. Keep `parse_local_document` returning the raw tree for its local-data callers. Extract `to_camel_case(&str) -> String` from `functions/strings.rs::camel_case` and use it for header-derived object keys. Preserve the row view's column index for explicit and normalized names. In object mode, reject empty normalized names and collisions at the first offending header field even with zero data rows. In positional mode, skip first-record header names without checking them. This keeps the existing `DataRuntime` raw/parse/bind split and shares the same case conversion the author invokes in expressions.
6. **Separate provisional and observed types.** Give unavailable uncontracted loads an open object or unknown-item array union and add null only as required by the file argument. Narrow a proven named tabular mode to an array of open objects, and a proven uncontracted `header: false` mode to `unknown[][]`. A literal null file is always typed null. Keep the existing literal and computed contract typing for contracted calls. Unknown keys produce no missing-member finding. Store realized values as `Arc<Value>` in the job snapshot and project them on demand through `TypeScope` and `scope/{shape,iterable,projection}.rs`. Key facts by the document-relative authored expression span and site, including callback occurrences. Read a known object key or array index directly from the observation. Make a known absent key definite only for a current, fully observed object. Keep the declaration from the ordinary schema lookup alongside the observed representation for hover. An executed headerless uncontracted call marks its containing property observation ineligible for a destination verdict and remains `unknown[][]` at the call site, including through callbacks. A null-file short circuit or an unexecuted branch produces no such mark. Prevent the headerless exemption from becoming a missing-member finding by retaining provisional element types through dependent observations. `scope/iterable.rs` and `scope/projection.rs` already own collection and member projection. The spec exempts headerless cell inference while retaining full compose validation.
7. **Carry provenance from the returning expression.** Add a tracked evaluation entry point with an opt-in result trace. Record the actual load's source descriptor at dispatch, keyed by the executing AST occurrence, and forward the descriptor through parentheses and the selected ternary arm. Treat other expression operators, member/index selection, callback results, and object construction as derived. Use occurrence identity during evaluation and project it to the authored range at the interpolation boundary. Never match provenance by JSON equality or evaluate a branch a second time. Return an internal `EvaluationTrace` containing the returned load descriptor, executed headerless marker, and optional observed call facts. Build the load descriptor from the successful `BindEntry<LoadedData>` and its `SourceEntry<LoadedSource>`. Carry `DataSourceIdentity`, format, shared original text, shared tabular row view with a name-to-column mapping, and the scalar-wrap flag. Track only origin in ordinary compose and collect call facts only for the worker. Use the same descriptor for explicit-contract failures and subsequent destination problems. Strip the property prefix from the typed problem path before running the locator, and use the loaded value to distinguish actual keys and indexes. A bare load or a load in the selected ternary branch retains its source. A selected fallback from `try` has the expression location. Use one `SourceLocator` per reported source group and map normalized index 0 to the scalar root. Reuse the existing missing-member positioning convention. Defaults and derived values get only the authored location. `evaluate_expr` already owns ternary selection, `FrontmatterProvenance` owns original authored values, and `report_diagnostics.rs::failure_diagnostics` is where destination failures currently omit `DataLocation`.
8. **Use the slot worker for observation.** Extend `overlay/mod.rs`'s slot demand and `slot_jobs.rs::{JobInputs, SlotJob, SlotResult, SlotView, SlotSnapshots}` to include whole top-level expression sites that syntactically contain an omitted, null, or computed `as`. Include calls in callbacks and spread operands. Pass the same captured buffers, config, registered-host inventory, registry, and context as request-side `schema::assemble`. Return its `SchemaBundle` from the worker assembly instead of discarding it. Capture the same `Option<Arc<RegisteredHosts>>` passed as `registered_hosts` to request-side assembly in `JobInputs` and forward it through `SlotJob` and `resolve_slot_templates`. Carry a separate slot-demand flag and property-site list, so an ordinary slot refresh keeps its own session outcome and grants. Mark a property eligible for scheduling when it contains a call with a missing second argument, a literal null, or a second argument which cannot be established non-null statically. Keep ordinary passive call checks for all arities. Visit other whole-value properties on demand as dependencies under the same worker policy. A job with independently demanded slots assembles in `SlotUse::Resolve` for that demand, then hands the resulting schema to the property pass. A property-only job assembles in `SlotUse::Pins` with the current session, or `SlotUse::None` when no pins are installed. Scan sites with `overlay/expressions.rs::frontmatter_interpolations` and the shared whole-value scanner. Evaluate in stable document order, using memoized dependency-first resolution for other eligible whole-value properties. Bare names see known literal and completed properties. `fm` sees the authored seed, matching `FrontmatterLookup`, while reads of seed paths with pending syntax defer. Treat a statically absent root as unavailable for this pass. Keep absent members of known values subject to ordinary evaluation rules. A cycle, pending dependency, or runtime-only root yields pending. Do not substitute null for an unknown editor binding. Reuse slot admission and its callback scope, expanding binding admission only for known editor frontmatter. Keep its function classifications and defer unsupported operations. Reserved host event values, `ctx` groups requiring new runtime observations, and runtime-only roots are pending. Editor-known frontmatter comes from the captured document. Use `SlotEvaluator` admission for pure functions and apply the editor read restrictions to registrations before dispatch. Use a nonrecoverable typed pending outcome for refused or unavailable reads so `try` recovers a permitted, proven miss without turning an unapproved read into a known fallback. `evaluate_recovery` in `expression/mod.rs` already distinguishes recoverable reads from other failures. Retain normal ternary, short circuit, and `try` execution. The slot evaluator currently installs `ShortcutLookup` over null and the slot job already scopes reads to captured buffers.
9. **Authorize before local probes and remote cache access.** In the editor evaluator, route content and existence resolution through `FileReference::resolve_detailed_with_presence` using the document-bound resolution context, a nonprompting slot grant check as admission callback, and the job's `SourceBuffers::get(path).is_some()` as presence callback. Derive the context with `simplified_schema::file_context::document_file_context` and reuse the buffered resolver pattern in `simplified-schema/lib/src/resolve/select.rs`. Keep actual probed candidates, including higher-priority misses ahead of a match. A refused candidate aborts resolution as pending. Only a clean admitted miss that passes `judge_absent` qualifies for the load warning. Use `DetailedOutcome::Failed(ResolutionFailure::NoMatch)` for clean misses, `DetailedOutcome::Matched` for success, and treat I/O, unavailable context, and gate refusal as pending in DMLS. Record `Missing` and `NonFile` dispositions before a winning candidate as refresh dependencies. Keep admitted probe dependencies from a refused search through the admission callback, following `resolve/select.rs::resolve_admitted`. Treat a file disappearing between a successful probe and a read as an unavailable observation. `ExpressionError::is_recoverable_read_failure` can recover ordinary loader I/O classified as `NotFound`, so convert editor-unavailable I/O to pending before returning to the evaluator. Check the resolved identity again before opening bytes or serving a memo. Use a lexical precheck before physical identity resolution to reject candidates with no potentially applicable file-read rule or exact in-process grant and to enforce lexical exclusions. Then apply the existing canonical policy and exact-grant checks before the resolver's existence/type probe. The lexical precheck grants nothing. `SlotGrants::decide_without_asking` canonicalizes targets and `PermissionPolicy::file_read` checks both lexical and physical exclusions. Keep those symlink and repository-containment protections ahead of content and existence answers. Test a lexical denial, a physical exclusion reached through an otherwise allowed link, and an approved missing path. Defer recursive references and listing functions on this editor surface because resolving those references authorizes a search root and does not admit each existence probe. Apply the same editor-local guard to transitive contract imports and template-driven reads. Use the existing checked selection and scoped `resolve::buffers` hooks. Keep URL imports unsupported because `resolve/select.rs::select` returns `SelectionFailure::Remote` before selecting a source. A nested template in a computed local contract receives the property worker's guarded reads and cache-only runtime. Route the input-reading builtins through their registration's read classification. Defer a builtin in editor observation until its local and remote paths go through these guards, with `load_data` and `file_exists` explicitly enabled. Ordinary compose keeps its current resolution path. `functions/fs.rs::file_exists_fn` currently resolves local candidates without asking the slot gate, and `load_data_fn` checks content only after resolving a winner.
10. **Use a read-only transport cache path.** Use the landed `RemoteFetchRuntime::{cache_only,cached_text}` and `remote_cache.rs::lookup_fresh`. Remove the temporary dead-code expectations when the caller lands. `register_and_fetch` leaves a cache-only URL unregistered, so the editor-aware `ResolutionContext::fetch_remote_text` must use `cached_text` directly for both `load_data` and remote `file_exists`. The latter returns true on a fresh hit and pending on a cache miss. Record cache expiry from any successful executed remote read, including a read used only as a condition. The lookup checks the slot gate, captured network settings and exact grants (via `SlotGrants::fetch_policy`), HTTP(S), and runtime policy before touching the store. Open the unnamespaced transport store at `FileStore::at(FileStore::resolve_cache_root(context.base_dir(), None))`, the store filled by `md compose --cache-root <base-dir>`. Use that default for the worker and a temporary root in tests. A usable entry requires the current layout/version, a decodable body, and a future `expires_at` without `no-cache` or `no-store`. Return body and expiry together. A denied URL, invalid permission block, malformed cache, unavailable entry, or expired entry is pending. Use this mode for property reads and their transitive contract/schema reads. Keep independently demanded schema-slot resolution on its existing path and discard invalid property observations on revocation even when the slot snapshot remains usable. Use already resolved slot pins for the destination schema, and defer its verdict while those pins are unavailable. A property-only demand must never initiate slot resolution or fetch a slot's inputs. Never register a fetch task, refresh, serve stale fallback, purge, or write a cache entry from this path. `ResolutionContext::fetch_remote_text` currently registers a fetch on a miss, and `SlotGrants::remote_runtime(None)` currently constructs a runtime without a store.
11. **Validate a private effective instance.** Add `EffectiveSchema::validate_observed_frontmatter` for the worker, reusing the existing conversion, pending-value, default, and instance-validation paths. Pass the authored seed and a map of concrete replacements, mark all replacement leaves as data, and retain pending treatment only for still-authored syntax. Validate the complete candidate so root unions and sibling constraints see the same context as compose. Keep findings rooted in directly declared observed properties via `declares_top_level_property`. Suppress a root-union property verdict while any relevant root input is unresolved, and honor the existing validator's union verdict on a fully known candidate. For an unresolved effective declaration, keep observations available for inference and defer destination findings. A concrete observation can supply inference even when the effective schema lacks a declaration for that key. For guarded file checks, add an observation-specific validation policy without changing the ordinary validator cache key. Use a scoped, panic-safe policy for the call covering `coerce.rs`, eager format resolution in `format.rs`, the `file_match.rs` keyword, and re-projection through `validate.rs::darkmatter_file_reference_outcome`. Keep lazy syntax-only `file-reference` checks enabled. Inspect the effective lowered declaration (following resolved `$ref` nodes) to identify potentially file-dependent top-level properties before validation. Mark a plain mapping property unverified when its declaration can call a file handler. Defer the root for unions and any root-level constraint that can connect one affected property to another. This conservative projection assigns suppression by path because `register_darkmatter_formats_with` receives the string value without the instance path. Core conversion, defaults, and validation still judge unaffected properties against one full candidate. Treat eager formats and file-match keywords as unverified in that scope and skip their I/O on validation and re-projection. Preserve lazy syntax-only judgments. Filter reports by decoded instance path, including root-level unknown-key information from the validator. Keep the guard scoped across conversion and diagnostic re-projection. The existing `validate_data` path provides the concrete-value semantics and `NestedTemplates` provides authored pending handling. Filtering file errors after an ordinary validation would already have probed the filesystem.
12. **Install and expire one coherent view.** Put observations, findings, relevant reads, and the minimum successful remote expiry in the existing job result. Only the latest sequence/base key can install. Reject a result already expired at install. Withhold all observations and their projected diagnostics as soon as the view is stale. Use the existing watch/dirty/debounce path for local changes, unsaved buffers, config, grants, and schema invalidation. Add a dispatch-loop deadline for the earliest installed remote expiry and enqueue the affected document for the same refresh path. Use a crossbeam deadline in the router loop, rescheduling it after install or invalidation. Capture wall time and monotonic time together at dispatch and derive the monotonic deadline from the remote wall-clock expiry. Expose the paired samples through `dmls/src/clock.rs` so tests advance both from one manual clock. Check the deadline on access as well as at the timer, and recheck wall-clock freshness at install. Capture the router's input generation at dispatch and increment it on open-buffer edits, disk watch events, and config, grant, or schema invalidation. Discard and requeue in-flight property results from an older generation, including results with a new dependency the previous snapshot did not watch. Capture local file stat facts at read/probe time and compare again before installation to catch changes prior to watch registration. Check open-buffer changes against the captured buffer identity or version. Keep the existing stat-rescan fallback for unwatched clients. `WatchedRead::now` currently samples after the job and cannot by itself rule out a read-to-install race. Deduplicate malformed and contract diagnostics against passive diagnostics by code and authored range. Filter ordinary schema findings caused by the still-authored template placeholder at any eligible observed, pending, or stale whole-property site. Keep unrelated document/schema findings and static call diagnostics. Publish a destination verdict for that site only from the current observation pass. Keep ordinary static call checking active while the observation is pending. Emit an executed, unrecovered, provably missing local load as a warning at the authored site using `dm.expression.evaluation_failure` and the existing file-reference details. Keep typed call-level failures with their own data locations. `SlotSnapshots::{install,invalidate_paths,recheck_stats}` and `router.rs::{dispatch_slot_jobs,sync_slot_watches}` already own freshness and watch delivery.
13. **Bound editor work with existing limits.** Share one `DataRuntime` and one property-pass budget per job. Use `DMLS_SLOT_LIMITS` (25,000 visits and 4 MiB) for evaluation, buffer/body acquisition, fact capture and location work. Count unique source bytes before parsing. For approved local files and transitive schema reads, enforce the remaining byte limit on buffer length or a bounded stream read with a one-byte overflow check. Apply the same cap to cache manifest/blob reads before decode and digest checks by extending `FileStore`'s read path for the cache-only lookup. Leave its existing normal-transport callers on the ordinary read path. Check cancellation before reads, evaluation steps and location work. Stop the pass on exhaustion, discarding incomplete observations and retaining only fully computed results with current dependencies. Use observed shapes for property references and evaluated call sites in `TypeScope`. Keep up to eight shared value alternatives per call occurrence, deduplicated within the same work budget. Treat overflow at that occurrence as provisional. Cap hover at eight entries per container, depth three, and 120 characters per string; include omitted counts and keep the declared type visible. Apply the limits before formatting. `SlotEvaluator::evaluate` currently opens a budget per slot, and the worker already has sticky budget and cancellation checks. The specification asks for bounded work and reuse. Deterministic budget, memo and cancellation assertions answer that requirement, so schedule no spike or wider benchmark.
14. **Migrate affected callers and outputs by ownership.** Keep the shipped catalog fixture keys `name,active,stars`, `host,port`, and `name;port`; their names already match the new conversion and the TSV example is positional. `functions.yaml` is the shipped tabular call inventory. Claudine's actual `load_data` examples and runtime callers use YAML, while its sequence example exercises interpolation closing. Update test expectations in Darkmatter's `load_data_execution.rs` for empty YAML and the nullable bracket contract, and catalog signature expectations in `expression/catalog/tests.rs`. Update the published CLI output schema descriptions for the new applicability of the existing field. The copies in `darkmatter/cli/schemas/output/{clean,code-block,compose,delta,edit,get,graph,hash,init,list-themes,render,rm,schema-about,schema-detect,schema-triggers,schema-validate,set,toc,validate-refs}.yaml` all document the common diagnostic record. Update the legends in DMLS autocomplete, hover, and diagnostics guides, which carry the original signature and option set. Keep the prior JSON and JSON5 duplicate-key behavior, which `data/parse.rs::parse_local_document` documents as last-wins. Rebuild the shipped artifacts in dependency order with root `just shipped-schemas` because the checked-in catalog section and Claudine's dependent artifact are generated over the changed definitions.

15. **Check a value once at its normal schema stage.** Pass the tracked output of whole-value interpolation to both `run_with_registry` calls in `pipeline/mod.rs` and the existing caller-owned post-shell path through `validate_typed_shell_values`. Preserve the documented two-pass interpolation order, pending-value exemptions, and caller-owned verdict. The normal post-shell branch already reassembles the schema for the completed instance and covers pass two. Test a property first completed in pass two and a caller-owned post-shell invocation to pin ownership. Use the existing preparation path for slot state. Use the decoded property key from the existing source mapper for quoted YAML keys and UTF-8 before the interpolation. Attach authored expression metadata only when a realized expression authored at that site supplied the failing property's value.

### Wave 1

- [x] **Add spread syntax**
  - Change `simplified-schema/lib/src/expression/{lexer,parser,ast,mod}.rs` and the literal extraction in `simplified/grammar.rs` using Rules 1–2. Update structured AST fixtures in the parser and `tests/l1/expression_syntax.rs`. Assert source ranges, order, explicit duplicates across a spread, shorthand, nesting with adjacent interpolation closers, and rejection of array spread and spread in catalog examples.
- [x] **Add cache lookup**
  - In `darkmatter/lib/src/markdown/compose/{cache/remote_cache.rs,remote_fetch.rs}`, add the read-only cache lookup and typed expiry per Rule 10. Share cache decode/freshness code with the normal cache path and keep its existing purge behavior on normal fetches. Cover fresh, stale, denied host, invalid policy, no-store, no-cache, corrupt body and cache miss with a seeded store and deterministic clock. Assert zero transport requests and zero cache writes.

## Phase 2 — Complete expression support

### Wave 2

- [x] **Integrate spread evaluation**
  - Replace the Phase 1 generic error in `expression/mod.rs` and interim `expression_type`, `partial_object`, and `literal_value` behavior in `expression/call_check.rs`. Add the library error and projection to `expression/error.rs` and `report_diagnostics.rs`. Extend the existing scope projections in `expression/scope/{iterable,projection}.rs`. Keep the landed `entry.value()` walkers in `expression/{absence,lint,member_check,prepared}.rs`, `compose/{remote.rs,preflight/collect.rs}`, and `schemas/slots/{admission.rs,evaluator.rs,grants/targets.rs}`. Update the interim error assertions in `lib/tests/l1/more_is_more_literals_and_indexes.rs`; the opt-in `lib/tests/prompts/dasherized_identifier_corpus.rs` already accepts the new tree. Add evaluator and passive coverage for order, single evaluation, multiple spreads, null, dynamic wrong types, and member typing after unknown spread and a subsequent explicit write. Prove the preflight and slot scans discover an input read in an overwritten operand.
- [x] **Adapt editor traversal**
  - Retain operand traversal in the three `SpannedExprKind::ObjectLiteral` consumers in `darkmatter/dmls/src/overlay/expressions.rs` and exercise the explicit field-key spans already in `SpannedObjectEntry`. Use the existing key span and operand ranges in provider coverage. Exercise cursor scope, hover, and completion inside spread operands and nested callbacks through DMLS providers.
- [x] **Adapt Claudine traversal**
  - Keep the landed `entry.value()` traversal in both walkers in `claudine/lib/src/composition/lifecycle/prewarm/classify.rs` and variable collection in `claudine/lib/src/dispatch/matcher.rs`. Add tests that assert that a call and binding in a spread operand affect classification and discovery even when a later field overwrites the same key.

- [x] **Verify expression integration**
  - After Wave 2, map `CallProblem::InvalidSpreadOperand` in `darkmatter/dmls/src/diagnostics/{call_check,codes}.rs` and assert the authored operand range through a diagnostics request. Run the Simplified Schema package's `just test`, `LANG=C.UTF-8 just test -E 'test(object_spread) | test(design_time_call_check)'` in Darkmatter on Linux, and Claudine's `just test-library`. Search the workspace for `ObjectLiteral` to verify all consumers accept the entry enums and no tuple-destructuring walker remains.

## Phase 3 — Implement optional loads and compose provenance

- [x] **Bind optional loads**
  - Update `functions.yaml`, `call_check.rs`'s literal contract and partial-options checks, and `data/{contract,load,options,runtime,error,locate}.rs` and `functions/strings.rs` for Rules 3–6. Add a one-argument YAML example using `fixture.yaml` and an explicit-name CSV example using `fixture-headerless.csv` with `header: ["label", "port"]`. Pin the port cell as text in the CSV result. Ensure dynamic invalid option errors preserve the same argument index as passive errors through authored-span projection. Update `dmls/tests/l1/option_arguments.rs` for nullable contract arguments and list option completion and diagnostics. Run root `just shipped-schemas` before catalog tests.
- [x] **Project compose locations**
  - Add the traced result described in Rules 6–7 to evaluation, `frontmatter_interpolation.rs`, `value_origin.rs::FrontmatterProvenance`, and both validation passes in `pipeline/mod.rs`. Attach the authored span for every failing realized whole expression and the optional data-location descriptor for direct loads. Carry metadata in an `expression_origins` field on `MarkdownError::SchemaValidationFailed`, keyed by the decoded top-level property and shared by every problem under it, preserving schema codes and defaults' origin. Update `schema_validation.rs` to build the expression metadata for instance failures and initialize it empty for preparation failures. Update `report_diagnostics.rs::failure_diagnostics` and `MarkdownError::status_block` in `markdown/types.rs`. Extend `markdown/errors/blocks.rs::schema_validation_failed_block` using its existing `Prose` links to show both locations. Use the same projection helper for structured and human-readable diagnostics. Use the existing CLI diagnostic record serialization and refresh the output schema descriptions from Rule 14. Compose uses its existing validation stage, source text and memo. No extra read or shape derivation enters the compose path.
- [x] **Run loader matrix**
  - Use `lib/tests/l1/{load_data_execution,load_data_headerless,load_data_options_passive,load_data_diagnostics,expression_recovery_data}.rs` and `cli/tests/l1/{load_data,compose_data_output}.rs`. Use one-edit fixture mutations with a passing control, checking public values/errors, source locations, stage ordering, and read/parse/bind counts. The matrix below defines the required cases. From `darkmatter/`, run `LANG=C.UTF-8 just test -E 'test(load_data) | test(compose_data_output) | test(expression_recovery_data) | test(option_arguments)'` on Linux. Assert `DataRuntime::stats()` reports one read, one parse, and one bind for omitted and explicit-null contracts over the same source and options. Two explicit name lists produce two binds over that same parse.

### Input Robustness Matrix

Invalid shapes produce a located error, run no subsequent processing for the failed request, and publish no composed value. Passive invalid authored calls prevent execution even in an unselected branch. Runtime-invalid file content necessarily incurs the attempted read. Assert that options errors precede the data read, and that malformed content and shape conflicts remain non-recoverable through `try`.

- **File and contract.** Test required `file` absent, wrong type, empty string, and local path control. Literal and computed null file arguments return null without reads or type imports. Test `as` absent and null sharing a bind entry, empty/malformed/scalar definitions, bad whole carrier, and mixed and all-invalid carrier-list items using the existing contract validator tests. Keep controls for literal, computed and imported contracts, including an array contract that accepts a wrapped scalar and a tuple that fails with original-root provenance.
- **Options.** Test omitted, null, and empty options as defaults, reject the wrong whole shape and unknown keys. For `header` explicitly test absent, null, both booleans, nonempty string-array control, wrong whole type, mixed and all-invalid elements, empty array, empty name, duplicate name, and names preserved verbatim after ordinary binding. Use an object element for the unconvertible header element and separately pin accepted binding conversions. `format` and `delimiter` accept absent or null as unspecified. Reuse their existing public validator coverage for whole-field types and empty strings. Exercise tabular applicability once per option, including null header on a tree format, TSV tab only, and both explicit header/contract incompatibilities before the read.
- **Delimited loader.** Use the shared CSV/TSV parser once for ragged later rows, invalid quotes/trailing text, quoted delimiters and newlines, and empty input. Cover each delimiter's ordinary control plus aliases supplied via format override. Exercise every header mode with and without a compatible contract, including positional header skipping with unusable names, normalized empty names and collisions, explicit-name first-record width mismatch with both counts and coordinates, and headerless no-record input judged against a supplied contract. Test separate memo results for different explicit lists and source coordinates for both normalized and explicit keys.
- **Tree loaders.** Use existing public compose fixtures as controls, including object and array roots. Pin YAML duplicate-key rejection, second-document rejection, malformed trailing syntax, bare `k:` preserved as null, scalar wrapping and empty YAML to `[]`. JSON accepts the last duplicate key, rejects trailing content and an empty file, and returns `[]` for root null. JSON5 accepts the last duplicate key plus comments and trailing commas, rejects trailing non-document content and empty input. JSON Lines shares JSON value parsing, skips blank lines and reports malformed trailing content at the record. Pin `[]` for an empty stream through the same parser. TOML rejects duplicate keys and trailing invalid syntax and preserves an empty table. Pin scalar wrapping with a number, a boolean, and an empty string. Keep invalid UTF-8 byte offset behavior through one shared decoder case.
- **CLI wiring.** Reuse the existing CLI controls and located malformed input cases per loader. Add one explicit-name width error and one destination-schema failure with an authored primary span and original data coordinates. Verify output status and absence of a composed value or a later command's marker. Keep the field permutations in-process. Audit changed loaders and binders for load-bearing `#[serde(default)]`, absence/null conflation via `Option<T>`, and silent drops via `filter_map`, `unwrap_or_default()`, or `.ok()`. A matrix review finding reports every other shape in that validator or loader's row and column.

## Phase 4 — Observe and validate in DMLS

### Wave 3

- [x] **Secure editor reads**
  - Wire Rules 8–10 and 13 into `schemas/slots/{admission,evaluator,session}.rs`, `compose/expression/{resolve_ctx.rs,functions/fs.rs,data/load.rs}`, and Simplified Schema's `resolve/{buffers,select}.rs`. This task returns evaluated values, executed call metadata, and read observations. It owns the bounded read hooks and consumption of the Phase 1 cache-only transport. Use a worker-only guarded lookup with structured pending, missing and failed outcomes. Include named contract preparation, path refusals, buffer presence, and read logging on failure. Share the memo over the full property pass. Verify a refused read gets no resolver existence/type probe or source bytes, a denied URL cannot consult the cache, and a missing local source differs from unreadable and unauthorized sources.
- [x] **Validate observed values**
  - Implement Rule 11 in `markdown/schemas/mod.rs` and Simplified Schema's `{coerce,format,validate,file_match}.rs`. Exercise it with supplied candidate values independently of the worker. Integrate it with the worker after Wave 3. Preserve original type and data location on the immutable observation and reuse the traced evaluator and source locator. Test enum membership, numeric string conversion, a missing field supplied by a default, closed-object rejection, imported types and trigger layering, root union pass/failure and unresolved arms, file-dependent deferral, and template-like returned strings. Assert the validator works from a copy and does not mutate any editor document or stored value.

### After Wave 3

- [x] **Install editor results**
  - Extend the existing DMLS slot job and router paths per Rule 12. Drive completion and hover in `overlay/expressions.rs` and `providers/{completion,hover}.rs` through installed facts, including alias/frontmatter member lookup and callback scopes. Keep provisional shapes on stale or pending results. Project schema diagnostics in `diagnostics/frontmatter.rs` and expression diagnostics in `diagnostics/call_check.rs` from the same view. Use LSP `relatedInformation` for a proven local data-file range (using the captured source text for UTF-16 conversion) and `data.data_location` for the full structured location. Preserve existing default and schema origin information.
- [x] **Prove editor parity**
  - Add in-process DMLS protocol coverage beside `dmls/tests/l1/schema_slots_editor.rs`, reusing the worker hooks from `slot_jobs.rs`. Check the worked example valid, then all three destination failures from bad age on line 4. Assert source location for direct and parenthesized/conditional loads and expression-only location for the mapped/spread row. Run each property separately through compose. Cover guarded and recovered missing inputs, a visible unrecovered warning, creation of a missing dependency, unsaved dependent buffers, no-watcher refresh, approval/config/schema invalidation, delayed superseded jobs, and remote freshness/expiry without a fetch. Include a rejected effectful function, an unavailable runtime-only binding, and a cycle. Verify denied or pending reads produce no property verdict or invented missing-member error. Use `just test -E 'test(schema_slots_editor) | test(typed_load_data_editor)'` in Darkmatter with the new cases registered as `dmls/tests/l1/typed_load_data_editor.rs`. Assert bounded hover, shared parse counts, prompt-free budget exhaustion, and `unknown[][]` with no editor cell verdict for an executed uncontracted headerless load whose bad cell fails in compose.

## Phase 5 — Publish and verify

### Wave 4

- [ ] **Update user guides**
  - Update `darkmatter/README.md`, `docs/topics/{loading-data,darkmatter-expressions,caching}.md`, `docs/inline/interpolation.md`, `docs/lsp/features.md`, `docs/topics/schemas/schema-slots.md`, `docs/cli/data-contracts.md`, and `docs/errors/composition-problems.md`, plus `dmls/docs/{autocomplete,hover,diagnostics}.md`. Include a before-and-after migration from `First Name` to `firstName` in both contract and member access, an explicit-name example, the conversion stages, authorization and cached-remote setup, refresh and provisional editor behavior, and the diagnostic data contract. Include one Mermaid flow diagram for editor demand, evaluation, installation, and invalidation. Update the repo skills' Darkmatter expression, request and DMLS topics to explain the shared loader and observation paths.
- [ ] **Document spread syntax**
  - Update `simplified-schema/docs/topics/expression-syntax.md` and its repo skill expression guidance. Include precedence, explicit duplicates, null, and the effect of unknown keys on inference.
- [ ] **Update caller skill**
  - Update `darkmatter/docs/use-skills/use-darkmatter/authoring.md` and use the canonical loading guide for behavior details. Use escaped or fenced expression examples as required by skill publication.

- [ ] **Regenerate published outputs**
  - After Wave 4, run `just regen-expr-doc` and `just publish-skills` from `darkmatter/`. Inspect the generated `docs/functions/load-data.md` and the published skill for the optional signature, preserved interpolation examples, and migration guidance.
- [ ] **Run acceptance suites**
  - Run `just test` and `just lint` in Simplified Schema and Darkmatter and `just test-library` and `just lint` in Claudine. Use `LANG=C.UTF-8` for Darkmatter's Linux run, as Phase 1 verified that the terminal horizontal-rule fixture otherwise uses the ASCII fallback under `LANG=C`. Include the passive `shipped_corpus` and `shipped_schema_corpus`, the `shipped_corpus_invocation` consumer, artifact freshness, and the L1 loader, CLI and DMLS entry-point cases. Run `BISCUIT_L1_INCLUDE_SLOW=1 just test -E 'test(shipped_corpus)'` in Simplified Schema and `LANG=C.UTF-8 BISCUIT_L1_INCLUDE_SLOW=1 just test -E 'test(shipped_schema_corpus) | test(shipped_corpus_invocation)'` in Darkmatter on Linux to cover the named slow tests. Keep fixtures temporary, transport local or mocked, and editor tests headless. Run `git diff --check`. Record commands, results, normalized-header and diagnostic examples, and any spec departure in `implementation-log.md` beside this plan for review after this phase.