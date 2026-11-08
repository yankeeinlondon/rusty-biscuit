---
kind: plan
created: 2026-11-08
total_phases: 5
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - claudine
---

# Plan: Expression Recovery (`try(expr, fallback)`)

Source: the `2026-11-05-expression-recovery` spec (finalized 2026-11-08). Rulings
ER-1 (recovery stays at the typed read-failure boundary) and ER-2
(`dm.data.unsupported_format` is not recoverable) are in force. This plan
replaces the 2026-11-07 plan. It keeps that plan's code evidence for lazy
registration, `ExpressionError`, and transport-cause preservation, re-checked
against the tree on 2026-11-08. It redoes the error table, the read-set and
memo rules, and the schema-slot work to fit the rulings and the sequencing
after 2026-11-05-dynamic-schema-types and 2026-11-05-tabular-input-options.

Every implementation choice is settled in **Phase 1 → Necessary Rules**
(`E1`…`E24`; the `E` prefix keeps them apart from the spec's ER-1/ER-2 rulings
and from dynamic-schema-types' `N` rules). Later phases cite rules by ID and do
not reopen them. The only exception is the Phase 1 rebase check: an assumption
it disproves revises the named rule there, and the revision is recorded in the
implementation log.

Execution conventions for every phase:

- Run package-area recipes only: `just test` and `just lint` in `darkmatter/`
  (library, CLI, and DMLS), and `just test-library` and `just lint` in
  `claudine/`. Tests run under nextest through those recipes. Never run
  `cargo test` on a consolidated suite.
- Never run `cargo fmt`, never commit, never move the spec to `_completed`, and
  never run `just complete`. The terminal state is "implementation complete,
  ready for review".
- Stay in this worktree and branch. Every session and delegated task is
  non-interactive. No test opens a terminal or browser window, uses a live
  network endpoint, or launches a provider.
- A task that changes a symbol's behavior also fixes that symbol's `///`, `//!`,
  and inline comments in the same change (repository drift rule). Boolean-only
  wording on lazy functions and "data failures always stop composition" wording
  are the known drift sites.
- Write path comparisons with `biscuit_file::canonicalize_simplified` and
  `Path::starts_with`. Load the `os` skill before touching path-comparison code
  or writing an OS-sensitive fixture (the unreadable-source fixture in E22).

## Summary and Definition of Done

### What is being built

`try(expr, fallback)` is a cataloged, lazy, function-shaped special form. It
evaluates `expr` once. If that succeeds, it returns the value unchanged; any
falsy value counts as success. It evaluates `fallback` only when `expr` fails
with a typed, recoverable read failure. Every other failure propagates
untouched. There is no grammar change, no new diagnostic code, no CLI switch,
and no dependency change.

```markdown
{{ as_json(try(load_data("optional.yaml", "string[]"), [])) }}
```

The work has five parts:

1. **Typed provenance** (`darkmatter` lib): one recovery classifier. A typed
   remote transport cause replaces today's string-only fetch failures. Contract
   imports become distinct from source reads. A data failure re-raised by nested
   composition is marked so it cannot pass for a direct read failure.
2. **Lazy recovery runtime**: catalog registration, a third lazy kind next to
   `and`/`or`, and selected-argument evaluation in both evaluator paths (the
   expression evaluator and the lifecycle operand path). Includes the slot
   determinism fact.
3. **Passive analysis**: result typing with the empty-array exception; no
   narrowing; both arguments stay visible to every passive walk.
4. **Consumers**: every expression surface, the reference check, approval
   discovery, schema slots, lifecycle hosts (md and Claudine), DMLS, and the
   `md compose` reporting path.
5. **Docs and skills**: topic pages, generated function pages, and skill rules.

### Definition of Done

Each item is observable and has a named test in the phase that delivers it. The
full mapping is in [Acceptance coverage](#acceptance-coverage).

- [ ] Missing and unreadable sources, allowed transport failures,
  `dm.data.malformed`, and `dm.data.contract_mismatch` recover under `try`.
  Everything else in the spec's error table propagates, including
  `dm.data.unsupported_format` (ER-2), `dm.data.invalid_options`, contract
  imports, policy refusals, and `dm.schema.slot_*` refusals.
- [ ] All six falsy values are successes. The fallback runs only after a
  recoverable failure, and each selected operand runs once per evaluation.
  Recovery resets no budget and rolls back no read.
- [ ] Reference checking and approval discovery recover genuine failures but
  never pick the fallback for a deferred read. A later permitted pass loads the
  real value.
- [ ] A recovered cause produces no diagnostic in the formatted output or the
  data envelope. Propagated errors keep their codes, wording, locations,
  callback paths, and reference fields.
- [ ] The read set and every memo layer hold the source result or failure,
  never the fallback. Memo invalidation still applies.
- [ ] Inside a dynamic schema slot, `try` recovers a missing in-repository
  file without any grant (dynamic-schema-types Q21), and
  approval or deny refusals propagate. The slot's class is derived from both
  arguments.
- [ ] Static typing unions both branches and keeps the typed array's items next
  to an authored `[]` only when the other branch provably accepts `[]`. DMLS
  recognizes `try` without executing anything.
- [ ] `just test` and `just lint` pass in `darkmatter/`; `just test-library` and
  `just lint` pass in `claudine/`; `cargo check -p darkmatter --examples`
  compiles.
- [ ] Topic docs, the generated `try` function page, the README overview, and
  the `darkmatter` skill describe the implemented behavior. No doc names a
  feature or spec.

## Phase 1: Rebase check, decisions, and baseline

Goal: confirm that the two earlier features landed as this plan assumes, fix
every remaining decision, and record a green baseline.

### Assumptions on earlier features

The rebase check task verifies each assumption below against the landed code,
not against either feature's plan. For each one it records in the
implementation log **holds** or **differs**, with the file and line of the
evidence. For every **differs** it also records the revised wording of the
affected `E` rule. If either feature has not landed (its code is absent, or its
implementation log does not say "implementation complete"), stop and report.
Do not implement on top of a partial dependency.

**2026-11-05-dynamic-schema-types** (rule IDs are that plan's):

- **D1 (N17).** `DataRuntime` keeps an observation log stored in the memo entry.
  A failed read keeps its dependency (`hash: None`). `load_data`'s not-found
  probe and every missing contract import are recorded. A memo hit re-records.
  *Used by:* E15 (read-set expectations).
- **D2 (N20).** `SlotClass::{Pure, InputReading, Captured, Volatile, Rejected}`
  is a required fact on every `FunctionBinding` (`functions/mod.rs`), lazy
  bindings included, and `slot_class(&SpannedExpr)` reaches every argument of
  every call through `visit_call_nodes` (`call_check.rs:1476` today). The
  derived `REMOTE_READ_FUNCTIONS` list is built from the `reads` fact. *Used
  by:* E6.
- **D3 (N28).** `ResolutionContext::slot_gate` and `authorize_slot_read` run
  before the target is touched and before the first memo lookup. A
  `GateRefusal` reaches the evaluator as a typed error that is none of
  `ExpressionError::FileReference`, `Data`, or `RemoteRead`. Record its exact
  variant. *Used by:* E3. If the refusal reuses one of those three, E3 gains an
  explicit `false` arm for it before anything else.
- **D4 (N31).** `SessionMode::ReferenceCheck` turns an unapproved slot read into
  a deferral of that property's judgment, driven by the refusal error that
  propagates out of the slot evaluation. *Used by:* E12.
- **D5 (N35).** `DataRuntime::memo_generation()` exists and `invalidate_memo`
  increments it. A pinned snapshot is re-verified after the generation moves.
  *Used by:* E15.
- **D6 (N37).** The slot codes live in `schemas/slots/codes.rs`
  (`SLOT_DIAGNOSTIC_CODES`) with the spellings `dm.schema.slot_approval_required`,
  `slot_denied`, `slot_forbidden_binding`, `slot_forbidden_function`, and
  `slot_empty`. *Used by:* E3, E18.
- **D7 (N14, N29).** Slot admission rejects forbidden bindings and functions in
  every branch before evaluation. Pre-flight target collection counts every
  argument of every call. *Used by:* E12, E18.
- **D8 (N15, N16, N19).** The slot evaluator uses the shared
  `expression::evaluate` with a fresh `IterationBudget` per slot. A failed
  `load_data` inside a slot keeps its `ExpressionError::Data` cause and
  `data_location`. *Used by:* E6.
- **D9 (N9, N21).** `[]` contributes no candidates to an `enum`/`suggest` slot.
  An empty final list is `dm.schema.slot_empty`. A cached slot resolution stores
  failures and observations, so creating a probed file invalidates it. *Used
  by:* E18.
- **D10 (N18).** `load.rs` reads local bytes through
  `simplified_schema::resolve::buffers::read`. A failed read is still
  `DataError::FileReference` with kind `NotFound` and a retained
  `FileReferenceError::Io` cause. *Used by:* E3.
- **D11 (no N rule).** `ExpressionError::RemoteRead` (`expression/error.rs:442`),
  `SchemaFileRead` (`:458`), and `Other` (`:469`) still exist as today. Source
  misses and contract-import misses still share `DataError::FileReference`
  (`data/contract.rs:244`).

**2026-11-05-tabular-input-options** (spec wording; its plan is being re-planned):

- **T1.** `DataError::InvalidOptions` exists, its `code()` is
  `dm.data.invalid_options`, and the code is listed in `DATA_DIAGNOSTIC_CODES`.
  *Used by:* E3.
- **T2.** The memo has three layers (raw source, parsed source, bound result).
  Each layer caches its own failures under one computation per key. The
  generation guard covers every layer. `RemoteNotEnabled` is never cached.
  *Used by:* E15.
- **T3.** A raw-layer local read failure is still `DataError::FileReference`
  (`NotFound` with an `Io` cause). A UTF-8 decode failure is still
  `DataError::Malformed`, raised in the interpretation layer. *Used by:* E3.
- **T4.** `load_data` accepts two or three arguments, and `load_data_type`
  (`call_check.rs:236`) types both arities. *Used by:* E10.
- **T5.** Execution order: bound options are validated first, then the contract
  is prepared, the source resolved and recorded, the effective format chosen
  (`unsupported_format` is raised only when no `format` override is given),
  option applicability and row shape checked, remote policy enforced, and only
  then is anything read, parsed, and bound. *Used by:* E3 and the ER-2 tests.
- **T6.** A headerless object-row contract fails with
  `dm.data.row_shape_conflict`. *Used by:* E3.
- **T7.** `DataRuntime::stats()` keeps `reads` (raw loads), `parses`, `binds`,
  `source_hits`, and `bind_hits` as the spec defines them. *Used by:* E15 tests.
- **T8.** Remote host authorization runs before any cached remote body or
  interpretation is returned. *Used by:* E8.

### Necessary Rules

**Boundary**

- **E1. Bounded recovery (ER-1).** Only the failures in E3 recover. Generic
  handler errors (`ExpressionError::Other`), every `ExpressionError::Provider`
  (`error.rs:302`, including `ProviderFailureKind::Transport` at `:192`),
  arithmetic, nested composition, and host-function failures propagate. Catalog
  `fallible` metadata never grants recovery; it is documentation and signature
  metadata only (`call_binding/signature.rs:190`). No performance spike: the
  spec rules one out, and the success path only adds one match.

- **E2. One registration, no new syntax.** Add `try` to
  `darkmatter/schemas/partials/functions.yaml` under `# Logical`, after `or`
  (`:559-565`). The declaration is
  `parameters(expr: unknown | null, fallback: unknown | null)`,
  `returns(unknown | null)`, `category(Logical)`,
  `example(try(null, 7) => null)` and `example(try(0, 7) => 0)`. The
  description says that the fallback runs only after a recoverable read
  failure and that falsy values are successes. It is not marked `fallible`;
  like `and`/`or`, it only propagates its operands' failures. It declares no
  aliases and no absence or recovery metadata. Register it as
  `FunctionBinding::lazy("try")` in `LAZY_BINDINGS` (`functions/mod.rs:174`).
  The case-insensitive registry and the host collision check
  (`host/mod.rs:238`, which uses the built-in registry) then reserve `try`
  automatically. The lexer has no `try` keyword (`simplified-schema` lexer
  keywords are literals only), so no parser change is needed.

- **E3. One classifier.** Add `ExpressionError::is_recoverable_read_failure(&self)
  -> bool` in `expression/error.rs`, beside `is_authoring_fatal` (`:644`).
  `Callback { source, .. }` delegates to `source`. The match is exhaustive: no
  `_ => …` arm, so a new variant has to be classified on purpose. The only
  `true` cases are:
  - `FileReference(d)` where `d.kind` is `NotFound` or `FoundElsewhere` and
    `d.source` is `None` (a clean miss) or `Some(FileReferenceError::Io { .. })`.
    Every other retained cause is `false`: tree escapes, `CwdOutsideBaseDir`,
    `BaseDirNotRepositoryRoot`, missing home, environment, or repository
    anchors, and an invalid URL. `FileRefFailure::classify` (`error.rs:80-96`)
    labels all of those `NotFound`, so the label alone proves nothing.
    `Malformed`, `RemoteNotEnabled`, and `RemoteDenied` are `false`.
  - `Data(e)` where `e` is `DataError::Malformed` or `DataError::ContractMismatch`,
    or `DataError::FileReference(d)` under the `FileReference` rule above.
  - `RemoteRead { kind: RemoteReadKind::Transport, .. }` (E7).

  Everything else is `false`. That includes `UnsupportedFormat` (ER-2),
  `InvalidOptions` (T1), `NotAContainer`, `RowShapeConflict`,
  `ThisImportRejected`, `ContractSchema`, `ContractImport` (E4), `NestedData`
  (E5), `SchemaFileRead`, `GlobReference`, the slot gate refusal (D3), `Call`,
  `CallCheck`, `Binding`, `Parse`, `UnknownFunction`, `InvalidCallback`,
  `IterationLimit`, `Arithmetic`, `Provider`, the four missing-context variants,
  `ContractViolation`, `ReservedRootPathUnknown`, `MalformedLiteralToken`, and
  `Other`. Keep `is_authoring_fatal` and `is_missing_runtime_context`
  unchanged: recovery is an explicit boundary, not a weaker fatality policy.

- **E4. Contract imports keep their identity.** Add
  `DataError::ContractImport(FileReferenceDiagnostic)` in `data/error.rs` and
  construct it in `data/contract.rs::import_failure` (`:244-249`) in place of
  `DataError::FileReference`. Its `Display` is the current file-reference
  message, unchanged; `code()` and `written_contract()` return `None`; it is
  never `is_deferral()`. `From<Arc<DataError>> for ExpressionError`
  (`data/error.rs:216-225`) keeps flattening `FileReference` and keeps
  `ContractImport` inside `ExpressionError::Data`. The memo already stores
  `Arc<DataError>`, so the distinction survives memo reuse in every layer.

- **E5. Nested data failures are marked.** `nested.rs:303-312` re-raises a
  child composition's data failure as `ExpressionError::Data`. Under E3 that
  would make `try(as_markdown("child.md"), "")` recover a malformed file inside
  the child, which the spec forbids ("nested composition failures propagate").
  Add `ExpressionError::NestedData(Arc<DataError>)`, displayed exactly like
  `Data` and authoring-fatal like `Data`. Raise it at `nested.rs:311`. Add
  `ExpressionError::data_failure(&self) -> Option<&Arc<DataError>>` (returns
  `Data` and `NestedData`, after callbacks) and use it at the three projection
  sites: `report_diagnostics.rs:348`, `nested.rs:375`, and the `is_authoring_fatal`
  arm (`error.rs:656`). The `dm.data.*` records at the `as_markdown` call stay
  byte-identical.

- **E6. A diagnostic view that keeps the cause.** Add
  `ExpressionError::file_reference_diagnostic(&self) -> Option<&FileReferenceDiagnostic>`.
  It covers callbacks, `FileReference`, and `Data`/`NestedData` holding
  `DataError::FileReference` or `DataError::ContractImport`. Use it in every
  renderer that matches the direct variant today, so imports keep their
  presentation:
  - `report_diagnostics.rs:341-350` (`with_expression_cause`) and
    `data_records` (`:394-399`);
  - `markdown/errors/blocks.rs:320` (`interpolation_block`);
  - Claudine's `composition/error/render/mod.rs:243` (`compose_failed_code`),
    `:523`, and `:542` (detail).

  Never unwrap an import into an `ExpressionError::FileReference` to render it;
  that would make it recoverable.

**Remote transport**

- **E7. Type remote failures where they happen.** Add
  `compose/remote_fetch/error.rs` (declared from `remote_fetch.rs`) with
  `pub(crate) struct RemoteFetchFailure { kind: RemoteReadKind, message: String }`.
  `Display` is `message` verbatim. Define
  `pub enum RemoteReadKind { Transport, Policy, InvalidReference, Infrastructure }`
  in `expression/error.rs`; it is the public classification vocabulary.
  Map the causes as follows:
  - `biscuit_file::FetchError::{RequestFailed, HttpError, BodyReadFailed,
    InvalidHeader}` (`biscuit-file/lib/src/file_reference/error.rs:186-210`) and
    the three response UTF-8 failures (`cache/remote_cache.rs:416`, `:481`,
    `:523`) → `Transport`;
  - `PolicyDenied` and `RedirectBlocked` → `Policy`;
  - `UnsupportedScheme` and an unparsable URL (`resolve_ctx.rs:319`) →
    `InvalidReference`;
  - `ClientBuild`, the runtime and client initialization failures
    (`remote_fetch.rs:584`, `:610`), and an unregistered slot after
    registration (`resolve_ctx.rs:328`) → `Infrastructure`.

  `fetch_with_cache` (`remote_cache.rs:403-410`) returns
  `Result<_, RemoteFetchFailure>`. `FetchSlot::Failed` (`remote_fetch.rs:44`)
  holds a `RemoteFetchFailure`. Add crate-private
  `RemoteFetchRuntime::get_content_typed`, and keep the public
  `get_content(&Url) -> Result<Option<String>, String>` (`:666`) as its
  rendering adapter, so transclusion and preflight callers are untouched.
  Change hashing, manifest fields, and freshness rules not at all.

- **E8. Expression transport provenance.** `RemoteTextError::Failed(String)`
  (`resolve_ctx.rs:339-344`) becomes `Failed(RemoteFetchFailure)`, and
  `ResolutionContext::fetch_remote_text` (`:315-334`) uses the typed getter.
  The policy-derived `Denied` arm (`:331`) stays and is checked first, so a
  denied host is never typed `Transport`. `ExpressionError::RemoteRead`
  (`error.rs:442`) gains `kind: RemoteReadKind`. Its `Display`, its non-fatal
  grading, and its report projection (`report_diagnostics.rs:344`) are
  unchanged. `functions/fs.rs::remote_text_error` (`:82-93`) fills `kind` from
  the failure. A redirect refusal stays `RemoteRead { kind: Policy }`; a denied
  host stays `FileReference(RemoteDenied)`. `Ok(None)` keeps meaning only
  "no remote runtime" (→ `RemoteNotEnabled`, a deferral). The fetch slot keeps
  the original typed failure; nothing stores a fallback there.

**Evaluation**

- **E9. A third lazy kind.** Replace `functions::ShortCircuit::{OnFalsy,
  OnTruthy}` (`functions/mod.rs:167-180`) with `LazyKind::{All, Any, Recover}`,
  mapped explicitly by canonical name: `and → All`, `or → Any`,
  `try → Recover`. An unmapped lazy name is a `debug_assert!` failure and maps
  to nothing, never to a default. Today's catch-all (`_ => OnFalsy`, `:179`)
  would silently run `try` as a conjunction. `lazy_function` (`:784`) returns
  `(registration, LazyKind)`. Update every consumer:
  `expression/mod.rs:916` and `:976-1003`; `scope/facts.rs:20`, `:46-51`; and
  `lifecycle/executor.rs:1338-1348`. The direct value-call rejection in
  `call_entry` (`functions/mod.rs:877-885`) stays and covers `try`: supplied
  values cannot represent an earlier failure.

- **E10. Selected-argument evaluation.** Add `evaluate_recovery` next to
  `evaluate_lazy_function` in `expression/mod.rs`:
  1. `registration.check_arity(args.len())` before evaluating anything.
  2. Evaluate `args[0]` once with `argument(0)`, the same lookup, observer,
     lexical scope, context, and `budget`.
  3. On `Ok(v)`, bind it with `registration.bind_callback_value(0, v)`
     (`call_binding/bind.rs:315`, already an indexed single-overload binder),
     check the result with a new
     `FunctionRegistration::check_lazy_return(&Value)` (wrapping
     `CompiledSignature::check_return`, `signature.rs:211`), and return it.
  4. On `Err(e)` with `e.is_recoverable_read_failure()`, evaluate `args[1]` the
     same way, bind it at index 1, check its return, and return it. An error
     from `args[1]` escapes unchanged.
  5. On any other `Err(e)`, return `e` without touching `args[1]`.

  Do not use `LazyCall::bind_next`/`finish` (`bind.rs:526`, `:562`): `finish`
  requires both parameters bound, and `bind_next` indexes by the count of
  earlier bindings. Do not accumulate guard facts: neither branch learns
  anything from the other. On the success path, never clone the request state,
  touch the memo, or read the fallback. Leave the `and`/`or` loop unchanged.

- **E11. The lifecycle operand path.** `lifecycle/executor.rs::call_with_operand_values`
  (`:1335-1360`) runs its own lazy loop over `evaluate_operand` (`:465`), which
  interpolates template operands. Add the `Recover` arm there with E10's steps.
  Only `LifecycleExprError::Evaluate(e)` with `e.is_recoverable_read_failure()`
  selects the fallback. `Compose` (a template operand's subtree composition),
  `Context`, and `Prose` (`lifecycle/error.rs:629-650`) propagate. The result
  is the selected value, never a Boolean. Claudine needs no second
  implementation: its executor hands Darkmatter's `StackExecutor` its inputs
  (`claudine/lib/src/composition/lifecycle/executor.rs:20`, `:453-462`), and its
  conditions go through the shared evaluator.

- **E12. Discovery and deferral.** `RemoteNotEnabled`, the slot gate's
  refusal (D3), and the reference-check slot deferral (D4) are nonrecoverable
  under E3, so they leave `try` as the same error. The existing rules then
  apply: `DataError::is_deferral` (`data/error.rs:184`), never caching a
  deferral (`data/runtime.rs:299-305`), the reference check's restricted
  preparation (`reference/graph.rs:1154`), and `SessionMode::ReferenceCheck`.
  The fallback is never chosen for a deferred read, and no pass freezes a
  discovery-time value as the final one. Static dependency and approval
  collection (preflight, N29 slot targets, `static_variable_reads`, context
  requirements) keep visiting both arguments; this is conservative and needs
  no code change. Pending nested-composition blocks (`pipeline::blocking_guard`,
  `stop_if_blocked`), shell approvals, and the request's `FetchPolicy` are not
  touched.

**Passive analysis**

- **E13. Result typing.** In `call_check.rs`, add `try_type` beside
  `load_data_type` (`:236`), dispatched from `expression_type`'s
  `FunctionCall` arm (`:178-180`) and identified through
  `functions::inspect_registration` like `is_load_data` (`:222`). For exactly
  two arguments, type both in the incoming `TypeScope` and join them with
  `ScopeShape::union` (`scope/shape.rs:296`). `null` and `unknown` survive.
  **Empty-array exception:** when one argument, after unwrapping `Paren`, is the
  literal `[]`, it contributes no alternative if and only if some arm of the
  other branch's shape provably accepts `[]`. Prove this with
  `simplified_schema::completion::alternative_accepts_in(atom, &json!([]),
  scope.definitions())` (`simplified-schema/lib/src/completion.rs:252`;
  `TypeScope::definitions`, `scope/mod.rs:207`). Do not use
  `ScopedType::admits` (`scope/shape.rs:108`), which accepts any recursive
  alternative. An `Unknown` shape absorbs the union anyway. A fixed nonzero
  tuple, a `min(1)` array, an object, and a recursive reference that cannot be
  proven all keep the ordinary union. The exception applies to `try` only, in
  both argument orders; general empty-array inference does not change.

- **E14. No facts, no absence role.** `scope/facts.rs::assume` returns the
  incoming scope unchanged for `LazyKind::Recover`. `absence.rs::short_circuit_polarity`
  (`:153-161`) stays limited to `and`/`or`, so `try` is not an absence guard and
  undeclared-property warnings inside it remain. `call_check` traversal,
  `static_variable_reads`, `static_access_reads`, `visit_call_nodes`, and
  context-requirement discovery already visit every function argument; add no
  `try`-specific skip.

**Memo, read set, reporting**

- **E15. Memo and read set are the loader's.** `try` reads and writes no memo
  and no read set. It adds no memo key, no read-set entry, and no
  observation. Each `load_data` call inside it follows T2/T5/D1 as if
  unguarded: failures are cached per layer under their own keys, resolved
  sources are recorded before the read, and the generation counter (D5)
  governs invalidation. The fallback's value is never stored anywhere a later
  call can find it. A read-set test compares the guarded call's read set with
  the same unguarded call's, not with a fixed list, so D1's probe recording
  is respected without hard-coding it. A second test asserts that no entry is
  a canonicalized form of a path that does not exist.

- **E16. Reporting.** A recovered cause is dropped by `try` before any stage
  sees it, so nothing reaches `ComposeReport` warnings, `report_diagnostics`,
  or the data envelope. Warnings already pushed by the protected expression
  stay pushed and still obey `--deny`. A failed fallback's error is the
  fallback's own, with its own authored span and callback path. Add no
  diagnostic code and no CLI switch.

**Schema slots**

- **E17. Slot class.** Register `try` with `SlotClass::Pure` and `reads: false`
  (D2): it has no effect of its own. The `slot_class` walk reaches both
  arguments, so a slot's class is the join over both. This is conservative: a
  volatile fallback makes the slot volatile even on runs where it is not
  selected. If D2 shows that the walk does not visit lazy-call arguments, fix
  the walk (not the registration) in the file D2 names. Only the selected
  argument's reads become observations (E15 plus D1).

- **E18. Slot behavior is the gate's.** Inside a slot, `try` sees the gate's
  refusal as a nonrecoverable error (E3, D3), so an unapproved or denied read
  propagates as `dm.schema.slot_approval_required` / `dm.schema.slot_denied`.
  Admission rejects a forbidden binding or function in either argument before
  evaluation (D7). A granted read that then fails as a miss recovers. A
  fallback `[]` contributes nothing, and an empty final list is still
  `dm.schema.slot_empty` (D9). A nonexistent file inside the defining
  repository is judged as an existence probe (dynamic-schema-types Q21, N24
  "Absent targets"): no grant is needed and the read fails with the typed
  not-found error, which `try` recovers. A missing file outside the repository
  still needs a grant, so without one it propagates `slot_approval_required`.
  Add Q21 to the Phase 1 rebase check (D-list).

**Scope limits and tests**

- **E19. Recoverable function families.** The classifier works on the typed
  error, not on the function name. So the file-reference rule in E3 applies to
  every read-side function that reports through `FileReferenceDiagnostic`
  (`frontmatter`, `markdown_title`, …), and the transport rule applies to every
  function that reports through `remote_text_error`. That is the typed
  read-failure boundary (ER-1). `SchemaFileRead` (`validate_schema`'s schema or
  import) is a contract-side failure and propagates, matching E4.

- **E20. No input-format work.** Loader formats, options, and parsing are owned
  by the earlier features. This plan adds recovery controls and negative cases
  around their public results; it does not multiply formats by failure kinds
  and does not change YAML/JSON/TOML/CSV parsing, frontmatter, or cache
  manifests. The Input Robustness Matrix does not apply: no new loader is
  introduced.

- **E21. Hermetic L1 only.** Use private temp directories,
  `request_support::{request, request_at, context_at}`, `wiremock::MockServer`
  (already used by `lib/tests/l1/load_data_runtime.rs:36-37`),
  `function_test_hooks` for instrumented functions, `CliProcessFixture` for the
  binary, and the dynamic-slot fixture helpers (D-check confirms their landed
  path). No new dependencies, no live network, no shell-specific commands, no
  permission-dependent skip. Each test stays under 5 s.

- **E22. Portable "unreadable source".** Make the unreadable source a
  **directory** named `data.yaml`. Resolution finds it, the extension selects a
  format, and the read fails with an OS I/O error on every OS (EISDIR on Unix,
  access denied on Windows), with no permission bits involved. If the rebase
  check finds that resolution rejects directories before the read, fall back to
  a unit test beside `data/runtime.rs` that seeds the raw layer with a typed
  `Io` failure for a real resolved identity and composes through the public
  API. Record which approach was used.

- **E23. Test placement.** New L1 modules, each registered once in its crate's
  `tests/l1/main.rs` by the Phase 4 scaffolding task:
  `darkmatter/lib/tests/l1/expression_recovery.rs` (surfaces and evaluation),
  `expression_recovery_data.rs` (loader, memo, remote, read set), and
  `expression_recovery_slots.rs` (schema slots), plus
  `darkmatter/cli/tests/l1/expression_recovery.rs`. Unit tests stay beside
  their code. DMLS, host-function, lifecycle, and Claudine tests extend the
  existing files named in Phase 4.

- **E24. Docs describe current behavior.** Topic pages state the rules
  directly and never name a feature, spec, or `{date}-{name}`. A generated page
  comes only from `just regen-expr-doc`. The README stays an overview.

### Wave 1

- [ ] **Rebase check and implementation log**
  - Create `implementation-log.md` beside this plan with the headings Rebase
    check, Baseline, Departures, Decisions applied, and one heading per phase.
  - Verify D1–D11 and T1–T8 against the code (not plans), recording holds or
    differs with `path:line`. For each difference, write the revised `E` rule
    text in the log. Edit this plan only where a rule's evidence is now wrong,
    and leave a dated line under the rule.
  - Re-check the line evidence cited in E3–E14 (files may have moved during
    the two earlier features) and correct citations in the log.
  - Try the E22 directory fixture once in a scratch test under the scratchpad
    directory (not in the repo), and record the error type it produces.
  - Confirm that Claudine is touched only by E6 (renderer) and by tests (E11).
    `rg "ShortCircuit|lazy_function|ExpressionError::(FileReference|Data|RemoteRead)" claudine`
    must show no other match site. Record the result.

### Wave 2

- [ ] **Baseline**
  - Run `just test` then `just lint` in `darkmatter/`, and `just test-library`
    then `just lint` in `claudine/`. Record pass/fail counts and any failure
    already present in the log, so later red results can be attributed.

### Validation checkpoint (end of Phase 1)

- [ ] Every assumption has a recorded verdict, every difference has a revised
  rule, and the baseline is recorded. A failing baseline test that this
  feature's files touch is investigated before Phase 2 starts.

## Phase 2: Typed failure provenance

Goal: every failure `try` must judge is typed, and unguarded behavior is
byte-identical.

### Wave 3 (serial: the shared contract)

- [ ] **Error model and classifier**
  - Own `expression/error.rs`, `data/error.rs`, `data/contract.rs`, `nested.rs`,
    and the new `compose/remote_fetch/error.rs` (plus its `mod` line in
    `remote_fetch.rs`). Apply E3, E4, E5, the `RemoteReadKind`/`RemoteFetchFailure`
    types and the `RemoteRead.kind` field from E7–E8, and the two accessors from
    E5–E6.
  - To keep the crate compiling, set `kind: RemoteReadKind::Infrastructure` at
    the single construction site (`functions/fs.rs:91`). The Wave 4 remote
    task replaces it.
  - Unit tests in `error.rs`: one `is_recoverable_read_failure` case per
    variant of `ExpressionError` and of `DataError` (exhaustive table test),
    each true case wrapped in `Callback` twice, every `FileReferenceError` cause
    under `NotFound`, `ContractImport` false while an equivalent source miss is
    true, `NestedData(Malformed)` false while `Data(Malformed)` is true.
  - Unit tests in `data/error.rs`: `ContractImport` display equals the former
    file-reference message, `code()` is `None`, and it is not a deferral.

### Wave 4 (parallel, disjoint files)

- [ ] **Remote producers**
  - Own `cache/remote_cache.rs`, `remote_fetch.rs` (except the `mod error;`
    line), `expression/resolve_ctx.rs`, and `functions/fs.rs`. Apply E7–E8.
  - Tests beside the code: each `FetchError` maps to its kind; the response
    UTF-8 failure is `Transport`; runtime and client init are `Infrastructure`;
    `get_content` strings are byte-identical to today's for every kind (assert
    against the existing tests at `remote_fetch.rs:759-1775`, unchanged); a
    redirect to a denied host is `Policy`; a denied host never reaches
    `Transport`.
- [ ] **Diagnostic projection**
  - Own `compose/report_diagnostics.rs`, `markdown/errors/blocks.rs`, and
    `claudine/lib/src/composition/error/render/mod.rs`. Apply E6 and the
    `data_failure` accessor from E5.
  - Extend `lib/tests/l1/load_data_diagnostics.rs` with before/after equality
    for an unguarded missing contract import and an unreadable import: code,
    message, `source`, span, `reference_details`, resolution failure, and
    callback path. Do the same for a malformed `load_data` inside an
    `as_markdown` child (now `NestedData`). In
    `claudine/lib/src/composition/error/tests.rs`, assert that an import
    failure still renders `composition.invalid_file_reference` with its file
    detail.

### Validation checkpoint (end of Phase 2)

- [ ] `just test` and `just lint` in `darkmatter/`; `just test-library` in
  `claudine/`. No existing assertion changed except constructors that gained
  `kind`. Typed provenance survives memo reuse (a second unguarded call fails
  with the same variant).

## Phase 3: Lazy recovery and passive types

Goal: `try` evaluates on every runtime path and types correctly, with no
surface wiring yet.

### Wave 5 (parallel, disjoint files)

- [ ] **Runtime recovery**
  - Own `schemas/partials/functions.yaml`, `functions/mod.rs`,
    `expression/mod.rs`, `call_binding/bind.rs`, `scope/facts.rs`, and
    `lifecycle/executor.rs` (with its `executor/tests.rs`). Apply E2, E9, E10,
    E11, E14 (the `facts.rs` arm), and E17 (the `SlotClass` fact on the `try`
    binding).
  - Unit tests in `expression/mod.rs`'s test module, using an instrumented
    `function_test_hooks` function as the fallback:
    `fallback_not_evaluated_after_success` (rstest over `null`, `false`, `0`,
    `""`, `[]`, `{}`, and a non-falsy value), `protected_runs_once`,
    `fallback_runs_once_after_recoverable_failure`,
    `nonrecoverable_failure_skips_fallback`,
    `fallback_error_escapes_its_own_try`,
    `enclosing_try_catches_recoverable_fallback_failure`,
    `enclosing_try_rethrows_nonrecoverable_fallback_failure`,
    `try_shares_the_iteration_budget` (an exhausted budget in the protected
    operand is not reset by the fallback), `try_lookup_is_case_insensitive`,
    and `wrong_arity_fails_before_evaluation`.
  - `functions/mod.rs` tests: `LazyKind` mapping for all three names,
    `direct_value_call_of_try_is_rejected`, and the existing
    `lazy_function(...).is_some()` invariant (`:2072`).
  - Lifecycle tests in `lifecycle/executor/tests.rs`:
    `try_action_returns_the_selected_value_not_a_boolean`,
    `try_template_operand_fallback_is_interpolated`,
    `unused_template_operand_is_not_interpolated`, and
    `template_operand_compose_failure_is_not_recovered` (E11).
- [ ] **Passive types**
  - Own `call_check.rs` and its unit tests, plus `lib/tests/l1/load_data_passive.rs`
    and `lib/tests/l1/design_time_call_check.rs`. Apply E13. Agree on the
    `LazyKind` name with the runtime task. This task does not edit
    `functions/mod.rs`.
  - Tests: `try_type_unions_both_branches`, `try_type_keeps_null`,
    `try_type_with_different_fallback_type`,
    `try_type_unknown_contract_stays_unknown`,
    `empty_fallback_keeps_typed_array_items` (both argument orders and a
    parenthesized `([])`), `empty_fallback_kept_for_min_one_array`,
    `empty_fallback_kept_for_fixed_tuple`, `empty_fallback_kept_for_object`,
    `empty_fallback_kept_for_unprovable_recursive_reference`,
    `empty_admitting_tuple_drops_empty_fallback`,
    `try_type_over_three_argument_load_data` (T4), and
    `try_adds_no_guard_facts` (`try(x, null) && x.y` is checked as if no guard
    existed). In `design_time_call_check.rs`:
    `invalid_call_in_unreachable_fallback_rejects` and
    `invalid_call_in_protected_operand_rejects`, both proven before an
    instrumented handler is entered.

### Validation checkpoint (end of Phase 3)

- [ ] `just test` and `just lint` in `darkmatter/`. The existing `and`, `or`,
  `||`, and ternary tests pass unmodified. The catalog parity test
  (`lib/tests/l1/function_catalog_parity.rs`) accepts the new entry, and its
  examples evaluate.

## Phase 4: Consumers and command behavior

Goal: prove the contract through every normal entry point.

### Wave 6 (serial)

- [ ] **Test scaffolding (E23)**
  - Create the three library modules and the CLI module, each with a `//!`
    summary and no tests, and register them in `lib/tests/l1/main.rs` and
    `cli/tests/l1/main.rs`. This is the only edit to those `main.rs` files.

### Wave 7 (parallel, disjoint files)

- [ ] **Composition surfaces** (`lib/tests/l1/expression_recovery.rs`)
  - `try_recovers_on_every_expression_surface`: rstest cases for body
    interpolation, whole-value frontmatter interpolation (passes 1 and 2), a
    conditional page block, a transclusion `when=` condition and an expression
    target, an inline `map`/`filter` callback, a `::map`/`::filter` region
    source and condition, and a shell-template expression. Each case covers
    one recovered miss and one propagated `unsupported_format`, and asserts
    the retained source location.
  - `try_is_not_enabled_where_expressions_are_disabled`: a surface with
    expressions off keeps them off.
  - `whole_map_recovery_replaces_the_result`, `per_item_recovery_continues`,
    `propagated_callback_error_keeps_item_position`,
    `fallback_sees_callback_locals_and_captures`.
  - `inserted_fallback_text_is_not_rescanned` (a fallback string holding
    `{{ x }}` is output literally).
  - `parse_error_inside_try_is_fatal`, `runtime_binding_failure_propagates`
    (a contract bound to a non-string `as`), `arithmetic_failure_propagates`,
    `iteration_limit_propagates`, `missing_request_context_is_fatal`,
    `malformed_reference_is_fatal`, `tree_escape_is_fatal`,
    `glob_reference_failure_is_fatal`, and `provider_failure_is_fatal` (a test
    hook returning `ExpressionError::Provider`).
  - `nested_composition_data_failure_is_fatal` (E5) and
    `pending_nested_blocking_diagnostic_stays_fatal` (an `as_markdown` child
    with a blocking warning).
  - `unrelated_warning_survives_recovery` and
    `undeclared_property_warning_inside_try_remains`.
  - `independent_reference_to_missing_file_still_fails`: the same file read
    unguarded elsewhere in the document still stops it.
- [ ] **Data, memo, remote, read set** (`lib/tests/l1/expression_recovery_data.rs`)
  - Positive controls and recoveries: `valid_source_is_returned`,
    `missing_source_recovers`, `unreadable_source_recovers` (E22),
    `malformed_source_recovers` (YAML, plus invalid UTF-8),
    `contract_mismatch_recovers`, and `headerless_malformed_record_recovers` (T6
    table: ragged row under `header: false`).
  - ER-2 and request errors: `unsupported_format_is_not_recovered`,
    `format_option_makes_unsupported_extension_readable` (the same file read
    with `{format: "json"}` recovers its miss),
    `computed_invalid_options_are_not_recovered` (T1),
    `headerless_object_rows_are_not_recovered` (T6),
    `missing_contract_import_is_not_recovered`,
    `invalid_contract_import_is_not_recovered`,
    `not_a_container_is_not_recovered`, and `this_import_is_not_recovered`.
  - Memo (E15, T2, T7): `second_unguarded_call_fails_from_memo` (`reads`
    unchanged), `different_fallbacks_stay_distinct`,
    `fallback_never_enters_any_memo_layer` (`source_hits`/`bind_hits` show the
    failure reused, never the fallback),
    `object_mode_failure_does_not_poison_positional_mode`,
    `pipeline_passes_share_the_data_memo` (reference check plus composition:
    `reads == 1`), and `executed_command_invalidates_recovered_failure` (a
    portable command fixture from `load_data_runtime.rs` creates the file;
    the later call loads the data, D5).
  - Read set (E15): `guarded_read_set_equals_unguarded`,
    `resolved_failed_source_stays_in_read_set`,
    `denied_url_stays_in_read_set`, `unresolved_miss_gets_no_canonical_entry`,
    `executed_fallback_reads_are_recorded`, and
    `unexecuted_fallback_adds_no_reads`.
  - Remote (wiremock): `allowed_transport_failure_recovers` (HTTP 500, invalid
    UTF-8 body, connection refused), `denied_host_is_fatal_with_warm_cache_and_memo`
    (zero requests at the mock), `redirect_to_denied_host_is_fatal`, and
    `reference_check_deferral_loads_remote_data_at_composition` (`[]` is never
    the final value).
  - Approval discovery: `approval_discovery_recovers_local_miss_without_freezing`
    and `approval_discovery_keeps_fallback_requirements_visible` (a shell
    template in the fallback is still in the approval batch).
- [ ] **Schema slots** (`lib/tests/l1/expression_recovery_slots.rs`)
  - Use the landed dynamic-slot fixtures (miniature repo, grant store under a
    temp home). Apply E17–E18.
  - `slot_try_recovers_missing_in_repo_file`: no grant, an
    `enum(a, {{ try(load_data("missing.yaml", "string[]"), []) }})` slot whose
    target is absent inside the fixture repository, resolving to `[a]` (Q21).
  - `slot_try_missing_outside_repo_needs_grant`: the same slot naming an absent
    file outside the repository propagates `dm.schema.slot_approval_required`.
  - `slot_try_propagates_approval_required` (no grant) and
    `slot_try_propagates_denied` (a persisted deny), each with its
    `dm.schema.slot_*` code at the slot's position and the fallback never run.
  - `slot_try_forbidden_binding_in_fallback_rejected` and
    `slot_try_forbidden_function_in_fallback_rejected` (admission, before
    evaluation).
  - `slot_try_empty_fallback_contributes_nothing` and
    `slot_try_only_fallback_empty_list_is_slot_empty`.
  - `slot_try_records_only_selected_reads`,
    `slot_try_class_is_join_of_arguments` (pure/pure → `Pure`,
    reading/`[]` → `InputReading`, pure/volatile → `Volatile`, through the
    landed `slot_class`), and `slot_try_preflight_collects_both_targets`.
  - `slot_try_reference_check_defers_instead_of_fallback` (D4) and
    `slot_try_resolution_cache_recovers_after_file_created` (D9).
- [ ] **Editor and host functions**
  - Own `dmls/tests/l1/expression_call_diagnostics.rs`,
    `dmls/tests/l1/expression_scopes.rs`, `dmls/tests/l1/value_hovers.rs`, and
    `lib/tests/l1/host_functions.rs`.
  - `try_is_offered_in_completion`, `try_hover_shows_signature`,
    `try_wrong_arity_is_diagnosed`, `try_callback_syntax_is_diagnosed`,
    `unknown_function_in_try_fallback_is_diagnosed`, and
    `callback_over_try_completes_item_members` (`.a` typed `string` for
    `map(try(load_data("x.yaml", "{ a: string }[]"), []), item => item.…)`).
  - `try_analysis_reads_no_inputs` (nonexistent source and contract paths, the
    effect and read counters unchanged) and
    `both_try_arguments_appear_in_static_reads`.
  - `host_catalog_rejects_try_collision` and
    `host_catalog_rejects_try_case_variant`.
- [ ] **Lifecycle hosts**
  - Own the md lifecycle L1 cases (`lib/tests/l1/load_data_runtime.rs` holds the
    `md_host` fixtures; add a `mod lifecycle_recovery` block at its end) and
    `claudine/lib/src/composition/lifecycle/executor/tests/` (a new
    `recovery.rs`, registered in that directory's module file).
  - md: `lifecycle_guard_try_recovers_optional_data` and
    `lifecycle_side_effect_invalidates_recovered_failure` (the next event loads
    the regenerated file).
  - Claudine: `claudine_lifecycle_try_recovers_optional_data`,
    `claudine_lifecycle_try_keeps_binding_failure_fatal`,
    `claudine_lifecycle_try_keeps_contract_failure_fatal`,
    `claudine_when_condition_try_recovers`, and
    `claudine_lifecycle_shell_failure_is_outside_try`.
- [ ] **Compose command** (`cli/tests/l1/expression_recovery.rs`)
  - Through `CliProcessFixture` and normal `md compose` with the reference check
    on: `compose_recovers_optional_data` (valid, missing, unreadable,
    malformed, and mismatched cases of the spec example; exit 0; stdout
    exact), and `compose_keeps_invalid_requests_fatal` (unsupported extension,
    computed invalid options, missing import, invalid import; exit 1; the
    later sentinel action does not run).
  - `recovered_cause_emits_no_diagnostic` (formatted stderr is empty, and
    `--format json` has no diagnostic for the absorbed cause, conforming to the
    envelope schema).
  - `unrelated_warning_blocks_under_deny` (no later command launches).
  - `fallback_failure_reports_its_own_location` (one error, the fallback's
    span and callback path; the protected cause is absent).

### Validation checkpoint (end of Phase 4)

- [ ] `just test` in `darkmatter/` and `just test-library` in `claudine/`. A
  shared-runtime defect found here is fixed by the file's Phase 2 or 3 owner
  rules, and only the affected focused tests repeat before the checkpoint.

## Phase 5: Docs, skills, and final validation

### Wave 8 (parallel, disjoint files)

- [ ] **Darkmatter docs**
  - Add `ExpressionFunctionPage { function: "try", path: "docs/functions/try.md", see_also: … }`
    to `EXPRESSION_FUNCTION_PAGES` (`catalog/mod.rs:667`), linking to
    `../topics/darkmatter-expressions.md` and `../topics/loading-data.md`. Add
    `try_page_states_lazy_semantics` to `catalog/tests.rs` next to the
    `load_data` page test (`:588`). Run `just regen-expr-doc` from
    `darkmatter/`, and review the generated diff of
    `docs/topics/darkmatter-expressions.md` and `docs/functions/try.md`.
  - Update `docs/topics/darkmatter-expressions.md` (a recovery section:
    the four runtime steps, the recoverable list and the propagating list from
    E3, falsy success, failed fallback, and whole-map versus per-item
    examples), `topics/expression-engine.md` (lazy kinds, selected-argument
    binding), `topics/loading-data.md` (replace or extend the optional-file
    pattern at `:296` with `try`; the unsupported-format rule and the `format`
    escape; memo and read set under recovery), `topics/caching.md` (the memo
    stores results and failures, never a fallback), `inline/call-check.md`
    (both arguments checked; typing and the empty-array exception),
    `errors/composition-problems.md` (explicit recovery against unguarded
    stops; recovered causes are not reported), `inline/preflight-checks.md`
    (deferral never picks the fallback), and
    `topics/schemas/schema-constraints.md` (try in a slot, gate refusals
    propagate, `[]` contributes nothing).
  - Include one Mermaid flowchart (protected → success / recoverable →
    fallback / other → propagate) on `darkmatter-expressions.md`. The audience
    is a developer new to the repo: lead with the task ("read an optional data
    file"), and give one compact example per rule. Name no feature or spec
    (E24).
  - `darkmatter/README.md`: one line plus a link in its expressions overview,
    only if its summary now reads as "data failures always stop".
  - Claudine docs: if `claudine/docs/topics/flow-control/lifecycle.md` lists
    lazy functions or says data failures always stop, add `try` with a link to
    the Darkmatter topic. Otherwise leave it untouched, and record that in the
    log.
- [ ] **Skills**
  - `.claude/skills/darkmatter/expressions.md`: the classifier, the
    recoverable list, ER-2, "never recover by function name or message", and
    `LazyKind`. `compose.md`: surfaces and discovery deferral. `requests.md`
    ("Data runtime and read set"): `try` never writes a memo or read set, plus
    `ContractImport` and `NestedData`. `remote-cache.md`: `RemoteFetchFailure`
    kinds and the string adapter. `lifecycle.md`: the `Recover` arm in
    `call_with_operand_values`. Keep each skill entry compact and link to the
    topic page.

### Wave 9 (serial: final validation)

- [ ] **Final checks**
  - Run `just test` and `just lint` in `darkmatter/`, `just test-library` and
    `just lint` in `claudine/`, and `cargo check --color=never -p darkmatter --examples`
    (the doc generator). Do not run L2, browser, real, or prompt tiers, a
    benchmark, or a workspace-wide gate.
  - Review the diff against [Acceptance coverage](#acceptance-coverage): every
    named test exists and passes.
  - Search the changed code for leftovers. `rg "RemoteTextError::Failed\(String|Failed\(String\)|ShortCircuit|_ => .*OnFalsy"`
    must be empty. `rg "is_recoverable_read_failure"` must show the only
    callers: the evaluator, the lifecycle executor, and tests. No recovery may
    match on `Other`, on a function name, or on a message.
  - Comment drift pass over every changed symbol (lazy-function docs,
    `load_data_fn`'s error docs, `DataError::FileReference`'s "or a contract
    import" doc at `data/error.rs:68-70`, and `RemoteRead`).
  - Complete the implementation log: commands and results, the acceptance
    table with test names, every departure from this plan, and the rebase
    verdicts. Leave the spec unchanged. Report "implementation complete, ready
    for review".

## Acceptance coverage

Each spec acceptance criterion and the tests that prove it (file prefixes:
`lib:` = `darkmatter/lib/tests/l1/`, `cli:` = `darkmatter/cli/tests/l1/`).

| Spec criterion | Tests |
| --- | --- |
| Example through `md compose`: data for valid; `[]` for missing, unreadable, malformed, mismatched; unsupported extension, invalid options, missing/invalid import not recoverable (ER-2) | `cli:expression_recovery::{compose_recovers_optional_data, compose_keeps_invalid_requests_fatal}`; `lib:expression_recovery_data::{unsupported_format_is_not_recovered, format_option_makes_unsupported_extension_readable, computed_invalid_options_are_not_recovered, missing_contract_import_is_not_recovered, invalid_contract_import_is_not_recovered}` |
| `try` in a schema slot: missing file recovers; unapproved or denied read propagates `dm.schema.slot_*` | `lib:expression_recovery_slots::{slot_try_recovers_missing_in_repo_file, slot_try_missing_outside_repo_needs_grant, slot_try_propagates_approval_required, slot_try_propagates_denied}` |
| Fallback not evaluated after success (each falsy value); each operand once; passes share the memo | `expression/mod.rs` unit `fallback_not_evaluated_after_success`, `protected_runs_once`, `fallback_runs_once_after_recoverable_failure`; `lib:expression_recovery_data::pipeline_passes_share_the_data_memo` |
| Failed fallback propagates; enclosing `try` catches only recoverable; whole-map vs per-item; item positions | unit `fallback_error_escapes_its_own_try`, `enclosing_try_catches_recoverable_fallback_failure`, `enclosing_try_rethrows_nonrecoverable_fallback_failure`; `lib:expression_recovery::{whole_map_recovery_replaces_the_result, per_item_recovery_continues, propagated_callback_error_keeps_item_position}` |
| Parse errors, bad calls in either argument, binding failures, arithmetic, iteration limits stop; budget not reset | `lib:expression_recovery::{parse_error_inside_try_is_fatal, runtime_binding_failure_propagates, arithmetic_failure_propagates, iteration_limit_propagates}`; `lib:design_time_call_check::{invalid_call_in_unreachable_fallback_rejects, invalid_call_in_protected_operand_rejects}`; unit `try_shares_the_iteration_budget` |
| Denied host: no request, fatal with cache; allowed transport failure recovers; reference-check deferral loads remote later | `lib:expression_recovery_data::{denied_host_is_fatal_with_warm_cache_and_memo, allowed_transport_failure_recovers, redirect_to_denied_host_is_fatal, reference_check_deferral_loads_remote_data_at_composition}` |
| Nested blocking diagnostic, missing context, malformed reference, path-boundary refusal stay fatal | `lib:expression_recovery::{pending_nested_blocking_diagnostic_stays_fatal, nested_composition_data_failure_is_fatal, missing_request_context_is_fatal, malformed_reference_is_fatal, tree_escape_is_fatal}` |
| Every surface; DMLS reads nothing; host collisions rejected | `lib:expression_recovery::try_recovers_on_every_expression_surface`; lifecycle and Claudine tests in Phase 4; `slot_try_*`; `dmls:try_analysis_reads_no_inputs`; `lib:host_functions::{host_catalog_rejects_try_collision, host_catalog_rejects_try_case_variant}` |
| No diagnostic for a recovered cause (text and envelope); unrelated warnings and `--deny` hold; propagated codes/locations kept | `cli:expression_recovery::{recovered_cause_emits_no_diagnostic, unrelated_warning_blocks_under_deny, fallback_failure_reports_its_own_location}`; `lib:expression_recovery::unrelated_warning_survives_recovery`; Phase 2 projection equality tests |
| Read set; memo reuse; distinct fallbacks; command regeneration invalidates | `lib:expression_recovery_data::{resolved_failed_source_stays_in_read_set, unresolved_miss_gets_no_canonical_entry, guarded_read_set_equals_unguarded, second_unguarded_call_fails_from_memo, different_fallbacks_stay_distinct, fallback_never_enters_any_memo_layer, executed_command_invalidates_recovered_failure}` |
| Static typing: `{ a: string }[]` kept with `[]`, DMLS member completion, nullable, different type, unknown contract, non-empty constraint | Phase 3 passive-type tests; `dmls:callback_over_try_completes_item_members` |
| Catalog corpus and normal `md compose` cover the form; generated pages agree | `lib:function_catalog_parity`; `catalog/tests.rs::try_page_states_lazy_semantics`; `cli:expression_recovery::compose_recovers_optional_data` |

## Review decisions (2026-11-08)

The author accepted this plan's resolutions of the spec ambiguities it
reported: a missing file's probe path stays in the read set (dynamic-schema-types
Q18; the spec's "no fabricated entry" refers to an unresolvable reference); a
data failure raised inside a nested `as_markdown` carries a marker and
propagates; "data source" covers every read function with a typed
missing-file or transport failure; failures in lifecycle template operands
propagate. The slot behavior for an absent file follows dynamic-schema-types
Q21 (E18).
