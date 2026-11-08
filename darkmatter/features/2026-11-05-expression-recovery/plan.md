---
kind: plan
created: 2026-11-07
total_phases: 4
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Expression Recovery Implementation Plan

> **Outdated (2026-11-08): re-plan before executing.** This plan predates the
> author's 2026-11-08 rulings and adjustments in the spec and the sequencing
> after 2026-11-05-dynamic-schema-types and 2026-11-05-tabular-input-options. Do not execute it as written;
> re-plan once the earlier features have landed.

## Summary and Definition of Done

Implement `try(expr, fallback)` as a cataloged lazy expression that returns the protected value unchanged and evaluates the fallback only after a typed, recoverable read failure. Preserve unguarded failures, passive checking of both arguments, discovery deferral, request permissions, diagnostics, read accounting, and memo invalidation. No language grammar or dependency changes are required.

Completion means:

- Every existing expression surface supports the same recovery boundary, including lifecycle function actions with template operands.
- Missing/unreadable data, allowed transport failures, unsupported formats, malformed data, and data-contract mismatches recover. Invalid requests/contracts, policy refusals, programming errors, providers, resource limits, and nested composition failures propagate.
- Success includes all falsy values. Evaluation is lazy, selected operands execute once per evaluation, and recovery neither resets budgets nor rolls back effects.
- Reference checking and approval discovery recover genuine failures but retain deferred expressions for subsequent permitted evaluation. Independent authored references remain validated.
- Recovered causes produce no diagnostic. Propagated errors retain their codes, messages, authored positions, callback paths, and reference details. Read sets and source/contract memoization remain accurate.
- Static analysis unions the successful branch types, preserves null, and retains typed-array items beside an authored `[]` only with a proof that the other branch accepts emptiness. DMLS recognizes the function without evaluating inputs.
- Focused public-boundary tests, the catalog corpus, and affected package L1/lint checks pass. Generated function documentation, topic pages, and skill guidance describe the implemented behavior.

Execution proceeds through every phase without a human gate. Waves are globally numbered. Tasks within a wave may use parallel agents with the file ownership stated below; waves run in order. Every session and delegated task is non-interactive. Stay in this worktree, do not commit, do not run `cargo fmt`, and leave the feature active for the author's subsequent review.

## Phase 1: Preserve Typed Failure Provenance

### Necessary Rules

1. **Use the bounded read-failure design.** Generic handler errors and every `ExpressionError::Provider` variant remain nonrecoverable. The spec explicitly authorizes its recommended boundary in “Open questions”; no required product ruling remains. Catalog `fallible` metadata never grants recovery. There is no performance spike: the spec explicitly excludes one and the evaluator can borrow its existing lookup and budget.

2. **Register one built-in, without aliases or new syntax.** Add `try` to `darkmatter/schemas/partials/functions.yaml` beside the logical functions with `parameters(expr: unknown | null, fallback: unknown | null)`, `returns(unknown | null)`, `category(Logical)`, `fallible`, and `example(try(null, 7) => null)`. Use the existing case-insensitive registry and lazy execution mode. Do not declare absence handling, numeric recovery, or narrowing metadata. Evidence: `functions/mod.rs::LAZY_BINDINGS`, `call_binding/registration.rs::EvaluationMode`, and the catalog's `and`/`or` declarations already support value-only lazy calls. `host/mod.rs::HostFunctionsBuilder` uses the built-in registry for collision rejection; adding the registration must also reserve `try` for host catalogs.

3. **Distinguish recovery from Boolean short-circuiting.** Replace internal `functions::ShortCircuit` with `LazyKind::{All, Any, Recover}` and map canonical registrations explicitly: `and → All`, `or → Any`, `try → Recover`. Update all four production consumers: `expression/mod.rs::evaluate_lazy_function`, `expression/scope/facts.rs::assume`, `lifecycle/executor.rs::call_with_operand_values`, and `functions/mod.rs::lazy_function`. Remove the current catch-all mapping to `OnFalsy`; it would silently implement `try` as conjunction. Preserve the direct value-call rejection in `functions/mod.rs::call_function`; supplied values cannot represent earlier expression failures.

4. **Expose one recovery classifier.** Add `ExpressionError::is_recoverable_read_failure() -> bool` in `expression/error.rs`. Callback wrappers recurse to their cause; all unspecified variants return false. Recover `DataError::{UnsupportedFormat, Malformed, ContractMismatch}` and typed remote transport failures. A `FileReferenceDiagnostic` is recoverable only for `NotFound`/`FoundElsewhere` with no underlying cause, or with a retained `FileReferenceError::Io` cause. Every other typed file-reference cause, including missing home/environment/repository state and tree escapes, propagates. The label `NotFound` alone is insufficient: `FileRefFailure::classify` assigns it to context and boundary failures too. Keep the existing fatality predicates independent; recovery is an explicit evaluation boundary, not a weakening of full-document failure policy.

5. **Retain contract-import identity.** Add `DataError::ContractImport(FileReferenceDiagnostic)` and construct it in `data/contract.rs::import_failure` for unresolved, absent, and unreadable imports. Keep source reads as `DataError::FileReference`. `From<Arc<DataError>> for ExpressionError` continues flattening source file failures but keeps `ContractImport` in `ExpressionError::Data`. Its classifier result is always false, its `code()` is `None`, and its displayed message is identical to the former file-reference failure. Evidence: imports and sources currently share `DataError::FileReference`, and `From` erases which operation failed. Both contract and source memo results must retain the distinction.

6. **Provide a diagnostic view without erasing the cause.** Add `ExpressionError::file_reference_diagnostic() -> Option<&FileReferenceDiagnostic>` handling callbacks, direct file errors, and `DataError::ContractImport`. Use it in `compose/report_diagnostics.rs`, `markdown/errors/blocks.rs::interpolation_block`, and `claudine/lib/src/composition/error/render/mod.rs` file-reference classification/detail branches. Extend `data_records` to project contract-import reference details. Evidence: those renderers currently match only the direct expression variant; retaining imports inside `Data` would otherwise change their diagnostic family or presentation. Do not unwrap the import into a recoverable expression error for rendering.

7. **Type remote failures at their producers.** Introduce `RemoteFetchFailure` in a new `compose/remote_fetch/error.rs`, with variants `Policy { message }`, `InvalidReference { message }`, `Transport { message }`, and `Infrastructure { message }`, all cloneable and displaying their existing message verbatim. Map `biscuit_file::FetchError::{RequestFailed, HttpError, BodyReadFailed, InvalidHeader}` and response UTF-8 failures to `Transport`; `PolicyDenied` and `RedirectBlocked` to `Policy`; `UnsupportedScheme` to `InvalidReference`; `ClientBuild` and runtime/client initialization failures to `Infrastructure`. Unknown future causes are nonrecoverable. Redirect rejection is a policy refusal, not an unavailable source. Evidence: `biscuit-file/lib/src/file_reference/error.rs::FetchError` already supplies these typed causes, but `cache/remote_cache.rs::fetch_with_cache` and `remote_fetch.rs::FetchSlot::Failed` currently turn them into strings.

8. **Preserve the remote public string API.** Change the private cache-fetch result and failed slots to retain `RemoteFetchFailure`; add crate-private `RemoteFetchRuntime::get_content_typed` and implement existing public `get_content` as its string-rendering adapter. Existing transclusion and preflight callers at `transclusion/engine.rs` and `preflight/collect.rs::fetch_remote_child_body` keep their string interface. Route `expression/resolve_ctx.rs::fetch_remote_text` through the typed getter, validate the URL, and call `check_allowed` before reading a ready or failed slot. Keep `Ok(None)` exclusively for an absent remote runtime; a missing registered slot is infrastructure failure. Evidence: the existing fast path returns cached content before checking policy, and its `Failed(String)` conflates transport with runtime initialization.

9. **Preserve expression transport provenance.** Add `ExpressionError::RemoteTransport { function: String, message: String }` for the typed transport branch of `functions/fs.rs::remote_text_error`. Keep the former `Other` display framing and generic evaluation diagnostic code. Other remote failure kinds remain nonrecoverable, with denied hosts retaining the existing `FileReference(RemoteDenied)` projection. `RemoteTextError` must carry the typed failure rather than a catch-all string. Shared readers using `remote_text_error` receive this transport classification; do not classify their generic parsing/handler errors. The fetch slot memo retains the original typed failure, never a fallback result.

10. **Keep cache and pass semantics.** Preserve existing source, binding, and contract keys in `data/runtime.rs`; preserve command/lifecycle invalidation and read-set retention. The remote runtime remains the transport-result memo, so do not add a second transport cache to `DataRuntime`. `RemoteNotEnabled` propagates out of `try` to the existing discovery deferral logic and is never treated as a missing source. Evidence: `DataError::is_deferral`, `DataMemo::load_source`, `data/load.rs::load_data_fn`, and `reference/graph.rs::prepare_content` already separate restricted preparation from composition.

11. **Use ordinary scopes for both branches.** Check arity before evaluation; evaluate argument 0 once; return its bound successful value, including null/falsy values; evaluate argument 1 only after the classifier accepts argument 0's error. Add crate-private `FunctionRegistration::bind_lazy_argument(index, value) -> Result<Value, CallBindingError>` and `check_lazy_return(&Value) -> Result<(), CallBindingError>` in `call_binding/bind.rs`, using the sole compiled signature and existing `bind_argument`/compiled return checker. Factor the shared indexed binding body used by `bind_callback_value` into a private helper. Recovery binds the successful protected value at index 0 or the selected fallback at index 1 and checks its return. It does not call `LazyCall::finish`, fabricate an unevaluated argument, or bind a fallback at index 0: `finish` requires both required parameters, and `bind_next` indexes by the number of successful earlier bindings. Leave the incremental `and`/`or` path intact. An error from argument 1 escapes this invocation. Use the same lookup, observer, lexical bindings, context, and iteration budget. Neither branch receives facts from the other. Evidence: existing `evaluate_lazy_function` binds operands individually but returns Boolean results and accumulates Boolean guard facts, neither of which applies to recovery.

12. **Handle the separate lifecycle lazy path.** Extend `lifecycle/executor.rs::call_with_operand_values` with `LazyKind::Recover`, preserving `evaluate_operand` for template operands. Only `LifecycleExprError::Evaluate` delegates to the expression recovery classifier; `Compose`, `Context`, and `Prose` propagate. A nested composition failure remains excluded even when it contains a lower-level read error. Direct non-template function actions continue through `invoke_expression_function → eval_expr`. Evidence: template operands take a separate lazy loop here, while `evaluate_operand_in → evaluate_expr_in` and Claudine's executor use the shared Darkmatter evaluator. Add host tests rather than a second Claudine recovery implementation.

13. **Infer types with a local empty-array exception.** In `call_check.rs`, add `try_type` identified through the canonical registry, analogous to `is_load_data`/`load_data_type`. For exactly two value arguments infer both in the incoming `TypeScope` and join with `Shape::union`, preserving null and unknown. An empty array literal, including parentheses around it, adds no alternative only when a concrete alternative of the other shape accepts `[]`. Prove acceptance with `simplified_schema::completion::alternative_accepts_in` and `scope.definitions()`; do not use `ScopedType::admits`, which deliberately accepts unresolved references permissively. Unknown shapes and unproved recursive references do not establish the exception. Test both argument orders, empty-admitting tuples, fixed nonzero tuples, and constrained arrays. Evidence: `Shape` retains `PropertyAtom` constraints, `ScopedType::admits` treats unresolved references optimistically, and the spec requires a proof of emptiness rather than a change to general array inference.

14. **Remain passive and conservative.** `scope/facts.rs::assume` returns the unchanged incoming scope for recovery. `absence.rs::short_circuit_polarity` remains limited to `and`/`or`; `try` handles no undeclared reads. Existing `call_check` traversal, `static_variable_reads`, `static_access_reads`, context-requirement discovery, and preflight dependency/approval collection continue visiting both operands. DMLS completion/hover use the shared registry and inferred result. Do not add input reads, execute a fallback in an editor, or exempt a referenced file globally. Evidence: those walkers traverse ordinary function arguments already, while `scope/facts.rs` currently interprets every lazy registration as conjunction/disjunction.

15. **Keep pending policy failures fatal.** Do not clear `NestedComposeSlot` blocked state or change `pipeline::{blocking_guard, stop_if_blocked}`, `BeforeEval`, or `ReportWatch` stopping behavior. Existing warnings survive recovery and remain subject to `--deny`. A `try` result cannot authorize shell commands or remote hosts. Cover this through normal composition and CLI paths rather than introducing a new diagnostic or CLI switch.

16. **Limit input-format work.** This feature changes an expression text grammar's evaluation and error transport, not data-format/configuration loading rules. The Input Robustness Matrix is therefore excluded. Existing `load_data_execution` format tests remain the loader coverage; add recovery controls and negative cases around their public results without multiplying every data field by every format. Do not change YAML/JSON/TOML parsing, frontmatter deserialization, or cache-manifest syntax.

17. **Use hermetic L1 evidence.** Tests use private temporary directories, existing request-support helpers, mock HTTP servers, `function_test_hooks`, `CliProcessFixture`, and the Test Toolkit. New consolidated test modules must be registered in each crate's `tests/l1/main.rs`. Do not use real terminals, browser windows, live providers, shell-specific commands, permission-dependent skips, or new dependencies. For deterministic unreadable-source coverage, a unit test beside `data/runtime.rs` seeds its existing private source memo with a real resolved identity and a typed permission-denied `Io`, then invokes public document composition and asserts its output/read set; pair it with the existing real filesystem reader tests and public classifier tests. This avoids permission-bit behavior changing under root or Windows while proving the recovery path's observable result.

### Wave 1

- [ ] **Type read failures**
  - Own the expression error model, `data/error.rs`, `data/contract.rs`, new `remote_fetch/error.rs`, `cache/remote_cache.rs`, `remote_fetch.rs`, `expression/resolve_ctx.rs`, and `functions/fs.rs`; apply Rules 4–10.
  - Keep policy checks ahead of memo/cache access and preserve cache freshness/stale-response rules and cache warnings. Change no hashing algorithms or manifest fields.
  - Add unit coverage of every existing recovery category, including callback-wrapped causes; test source versus contract-import misses/I/O, every retained context/tree error, remote UTF-8, HTTP failure, redirect refusal, and infrastructure failure.
  - Risk: typed transport changes span three adapters. Assert identical displayed text at each string compatibility boundary and keep deferrals out of failed memo slots.

### Wave 2

- [ ] **Preserve error projection**
  - Depends on Wave 1. Own the diagnostic accessor and its use in `compose/report_diagnostics.rs`, `markdown/errors/blocks.rs`, and Claudine's `composition/error/render/mod.rs`.
  - Extend existing `load_data_diagnostics` and Claudine error-render tests to compare unguarded missing-contract diagnostics before/after the representation change: code, message, source location, reference, typed resolution failure, and callback position.
  - Checkpoint: typed provenance survives both memo reuse and diagnostic rendering; unguarded errors retain their public behavior. Run the focused affected error/data test filters before Phase 2.

## Phase 2: Implement Lazy Recovery and Passive Types

### Wave 3

- [ ] **Add lazy recovery**
  - Depends on Phase 1. Own the catalog, `functions/mod.rs`, `expression/mod.rs`, `call_binding/bind.rs`, and `lifecycle/executor.rs`; apply Rules 2–3 and 11–12.
  - Register `try`, replace the Boolean-only discriminator, preserve direct-value rejection, and implement selected-operand binding and return validation in both runtime paths.
  - Add evaluator tests for every falsy success, one execution per selected operand, no fallback effects on success, nested recovery, failed fallback propagation, shared exhausted budgets, whole-map versus per-item recovery, and case-insensitive lookup. Assert the original callback position for propagated errors.
  - Own `function_dispatch.rs`, `function_catalog_parity.rs`, and registry tests needed for the new catalog entry and value-only call rejection. Put the new form in the executable catalog corpus, not only a parser test.

- [ ] **Infer recovery types**
  - Concurrent with lazy recovery; own `call_check.rs`, `scope/facts.rs`, `load_data_passive.rs`, and type-scope tests. Use the agreed `LazyKind` names; the runtime task owns its definition and exports.
  - Implement Rule 13 and make `Recover` introduce no narrowing. Keep every argument visible to passive checking and read/context inventory; add tests proving an invalid unreachable fallback rejects before an instrumented handler executes.
  - Cover nullable and different-type unions, computed/imported unknown contracts, parenthesized empty literals in both positions, constrained arrays rejecting emptiness, nonempty tuples, objects, and no guard-fact leakage. Preserve existing `and`/`or` and `||` tests unchanged.
  - Checkpoint after both tasks: build the library and run focused runtime, call-check, catalog, static-read, and lifecycle operand tests. Both new runtime dispatch branches and passive matches must compile before Phase 3.

## Phase 3: Prove Consumers and Command Behavior

### Wave 4

- [ ] **Verify composition surfaces**
  - Depends on Phase 2. Own new `darkmatter/lib/tests/l1/expression_recovery.rs`, its module registration, and extensions to `load_data_runtime.rs` and `load_data_execution.rs`.
  - Through public composition, cover body and whole-value frontmatter interpolation, `::block`, transclusion `when=` and expression targets, inline map/filter callbacks, collection-region source/condition expressions, and shell-template expressions. Test collection callback arguments and captured lexical locals in the fallback as well. Assert values and retained source locations through each normal entry point. Keep currently disabled features disabled.
  - Prove valid data succeeds; missing, unreadable, malformed, unsupported, and mismatched data recover; invalid/imported contracts and malformed/context/tree references propagate. Include a valid protected value with an invalid unreachable call in its fallback to prove the document-wide passive rejection.
  - Add remote mock cases: allowed failed request, invalid UTF-8, denied host with a seeded cache and zero requests, and reference-check deferral followed by successful permitted loading. Exercise approval discovery with a recoverable local failure and with a deferred protected read; conservative fallback approval requirements remain visible.
  - Assert unresolved misses add no fabricated path, resolved failed sources and denied URLs remain in the read set, executed fallbacks add reads, and unexecuted fallbacks do not. Two fallbacks over one failed memo return different values; a later unguarded call still fails. Prove command and nonempty lifecycle invalidation allow a regenerated file to load.
  - Cover independent authored references to the same missing file, inserted-expression text staying data, retained undeclared-property warnings, blocking warnings, and pending nested `as_markdown` stops. Reuse the existing portable command fixture from `load_data_runtime.rs` for invalidation, rather than shell-dependent `echo` commands.

- [ ] **Verify editor analysis**
  - Concurrent; own DMLS tests and `lib/tests/l1/host_functions.rs`. Add tests in existing `dmls/tests/l1/expression_call_diagnostics.rs` and `expression_scopes.rs`, using current provider test helpers.
  - Assert completion lists `try`, hover shows its signature, wrong arity/callback syntax/unknown nested calls report shared diagnostics, and a callback over `try(load_data(..., "{ a: string }[]"), [])` completes `.a` with its declared type.
  - Exercise passive analysis with nonexistent source and contract paths and unchanged effect counters; assert both arguments' variable/context reads remain in static inventory. Prove host catalogs cannot register `try` or a conflicting case variant.
  - Production DMLS behavior remains catalog/analysis-driven; do not introduce a DMLS-specific recovery engine or input loader.

- [ ] **Verify lifecycle hosts**
  - Concurrent; own lifecycle recovery tests under `darkmatter/lib/src/lifecycle/` and `claudine/lib/src/composition/lifecycle/executor/tests/`, including each local module registration.
  - Exercise md lifecycle guards, action expressions, and a `try` function action whose selected fallback is a template operand; demonstrate value preservation rather than Boolean conversion. Cover lazy unused template operands and the ordinary evaluator path.
  - Exercise Claudine's delegated executor with a request resolution context: optional data fallback, nonrecoverable provider/binding/contract failures, unchanged lifecycle shell failures, and memo invalidation after an event. Use existing host/test handlers instead of launching providers.
  - `LifecycleExprError::Compose` remains nonrecoverable; test a nested composition error explicitly so a broad source-chain search cannot accidentally widen recovery.

- [ ] **Verify compose command**
  - Concurrent; own new `darkmatter/cli/tests/l1/expression_recovery.rs` and its `tests/l1/main.rs` registration. Use `CliProcessFixture` and the existing native sentinel executable.
  - Run normal `md compose` with reference checking enabled for a valid source, missing/malformed/mismatched/unsupported data, missing/invalid contract import, and an independent broken authored reference. Assert exit status, composed value, and whether the later sentinel action ran.
  - Assert formatted success has no recovered-cause stderr message, the data envelope has no absorbed diagnostic, and read sets match execution. Prove failed fallbacks report their own normal location and callback path without a second error for the protected cause. Include `--deny` on an unrelated warning and assert no later command launches.
  - Use in-process public APIs for the broader error matrix; subprocess tests here prove the reference-check/compose/reporting wiring rather than repeating every type and format combination.

Checkpoint: all four tasks finish and their focused tests pass before documentation generation and full-area validation. Shared runtime changes discovered by a test failure are fixed by the owning Phase 2 task, then only the affected focused tests repeat.

## Phase 4: Document and Validate the Finished Contract

### Wave 5

- [ ] **Publish recovery guidance**
  - Depends on Phase 3. Update `darkmatter/docs/topics/darkmatter-expressions.md`, `topics/expression-engine.md`, `topics/loading-data.md`, `inline/call-check.md`, and `errors/composition-problems.md` with the selected-operand rules, bounded error list, passive checking, deferral, warnings, memo/read-set behavior, and empty-array typing exception.
  - Give compact examples for optional data, falsy success, failed fallback, and whole-map versus per-item recovery. Include a Mermaid flow from protected evaluation to success/recoverable/fatal outcomes and fallback evaluation. State current behavior directly; docs must not reference a feature directory or specification snapshot.
  - Update the README's existing loading-data overview with a short `try(load_data(...), [])` example/link. Update `.claude/skills/darkmatter/expressions.md`, `compose.md`, and `remote-cache.md` to explain explicit recovery and typed transport while keeping unguarded failures fatal.
  - Run `just regen-expr-doc` from `darkmatter` to regenerate the expression table and function pages, including `docs/functions/try.md`. Review the generated diff for catalog/doc agreement. This is the existing finite generator, not an agent-driven docs recipe.
  - Review comments on changed symbols for drift, especially Boolean-only lazy-function descriptions and data errors described as always fatal. Keep the distinction between unguarded fatality and explicit recovery. Record the bounded design and representation changes in an implementation log beside this plan; leave the spec snapshot unchanged.

### Wave 6

- [ ] **Run final checks**
  - Depends on documentation completion. Run `just test` and `just lint` from `darkmatter`, then `just test-library` and `just lint` from `claudine`. The Darkmatter recipes cover the library, CLI, and DMLS; Claudine library tests cover the changed host/error projection. These are nextest/Clippy recipes; do not substitute `cargo test` or invoke `cargo fmt`.
  - Run `cargo check --color=never -p darkmatter --examples` to compile the documentation generator and other library examples. No simplified-schema source changes, new dependencies, CI matrix changes, benchmark run, prompt-corpus sweep, L2, or browser test run is part of this feature.
  - Review the final diff against the definition of done: every recoverable family has a positive control and a public recovery result; every excluded family has a propagating result; all named surfaces have coverage; no generic-error catch or diagnostic-message classification was introduced.
  - Search the changed code for `RemoteTextError::Failed`, string-only failed slots, broad `ExpressionError::Other` recovery, and a catch-all lazy-kind mapping. Remove obsolete paths. Confirm error rendering retains imported-contract provenance and no fallback result entered a source memo.
  - Record executed commands and results, acceptance-criterion coverage, and any implementation departures in the implementation log. Fix failing required checks before reporting completion. The terminal state is “implementation complete, ready for review”; do not move the feature into `_completed` or run `just complete`.
