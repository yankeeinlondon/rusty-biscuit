---
spec: 2026-11-01-map-and-filter-expression
plan: plan.md
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 produced design contracts only; no source code or prototype was written.
    Read the Current Handoff below before Phase 2. The author requested alignment
    with the directive form on 2026-10-27. The earlier prerequisite blocker,
    identifier-only callback, stack-only Locals chain, array-only inputs,
    unknown-only typing, and shadow advisory are superseded. Use the updated
    spec and unstarted plan. Product rulings, including budgets, are settled;
    diagnostic API and Rust option names follow existing conventions;
    do not treat the historical human-review proposals as current gates.
---

# Implementation Log for 2026-11-01-map-and-filter-expression

## Current Handoff — 2026-10-27

The author requested reconciliation with `2026-11-01-map-and-filter-directive`
after reviewing both designs. This update changes the spec, unstarted plan,
and implementation contracts only. No callback runtime, parser, or editor code
has been implemented. Phase 1 remains complete as design work; Phases 2–5 are
unstarted.

The shared schema/signature infrastructure and type-aware evaluation are now
implemented, as the plan's earlier reconciliation already records. The
historical missing-prerequisite gate no longer applies. Ordinary local
shadowing is allowed without an advisory. All product rulings are settled;
diagnostic identifiers and Rust option names remain implementation details,
not unanswered shadowing, prerequisite-order, or budget approvals.

### Reconciled behavior

- Both `=>` and `->`, one identifier or shallow destructuring pattern, and
  object-expression shorthand use shared expression grammar.
- Arrays, schema-typed tuples, and insertion-ordered `{key,value}` object
  entries are accepted. Both functions return arrays. Object filtering retains
  entries, and expression mapping does not rebind its result to the input pattern.
- All callback bindings live in `local`; bare names prefer the nearest local,
  `local.name` addresses it explicitly, and `fm.name` bypasses shadowing. Null
  bindings still shadow. Namespace/global names are protected by the binding view.
- Preserve known item/field/map-result types and filter item types through the
  existing TypeScope and narrowing engine. Unknown remains a fallback, not a
  mandatory type for every local.
- Expression callbacks remain required and iterable inputs may be arbitrary
  expressions. Directive-only render options and source-declaration restrictions
  do not transfer to expression functions.
- Callbacks run sequentially under the enclosing request's authority and share
  visited-item accounting with directives and nested composition. No fresh
  unlimited budget is installed for an inner expression call.

### Author ruling: file predicate narrowing

Both `file_exists` and `is_file_type` narrow a checked value to `file` on true.
The difference is the runtime existence check performed by `file_exists`.
Update its current string-only narrowing registration through the shared fact
mechanism, not a directive-specific special case. A false existence result
cannot exclude `file`; a valid reference may be missing. Facts remain attached
to checked values, not unrelated reevaluations of the same call text. This is
an implementation requirement; no predicate code changed in this design update.
The author explicitly ruled that schema-authored narrowing metadata is not a
prerequisite. Use the existing Rust registration API; the later metadata feature
will migrate its authority without blocking map/filter.

### Author ruling: dynamic child discovery

Extend existing composition preflight to enumerate typed collections and
evaluate discovery-supported target-producing expressions. Inspect candidate
children before collection conditions, selection, or stopping exclude them,
including recursively through explicit local parameters. This can perform
reads for skipped items, but validation and DMLS remain passive and discovery
does not execute shell commands or render repeated bodies.

Preserve rejection of dependencies on pending shell output or other execution
results discovery cannot faithfully produce, including indirect dependencies
through locals. Retained graph metadata distinguishes authored sites and
iteration scopes. Newly encountered runtime targets fail without late approval;
retaining the graph does not imply arbitrary callback-result caching. The
literal-target-only restriction proposed in review was not adopted. This is a
design ruling only; preflight code has not changed.

### Author ruling: expansion budgets

The author accepted 100,000 visits and 16 MiB (16,777,216 bytes) of cumulative
UTF-8 collection expansion per root request. Share visits across expression and
directive calls and nested composition, including rejected items. Standalone
expression sessions use the same default visit limit with one shared counter.
Expression arrays are values, not rendered bytes.

Charge nested output again when copied into an enclosing expansion and cached
fragments whenever inserted into an expansion. Ordinary pipeline passes,
buffer reallocations, unrelated source text, and preflight copies add no charge.
Fail before an append exceeds the allowance; exactly reaching it succeeds.
Expose positive library controls and `--max-iteration-visits` /
`--max-expansion-bytes`; reject zero, negative, and overflowing values, with no
unlimited setting. Include limits in compose identity. Preflight does not spend
the execution allowance. No benchmark gate is required to adopt these defaults.

Both map/filter specs are finalized for planning; implementation is unstarted.
Historical Phase 1 proposals below remain history, not unresolved gates.

### Replacement contract: callback patterns

The original `Callback { param: String, ... }` sketch is superseded:

```text
BindingPattern = Identifier(name) | Object(fields)
Callback = { pattern: BindingPattern, body: Expr }
CallArgument = Value(Expr) | Callback(Callback)
```

Use spanned counterparts for the pattern, each field, arrow, body, and callback.
Object fields bind their same-named property initially; defaults, aliases, rest,
and nested destructuring are not required. A pattern is one input parameter,
regardless of the number of names it binds. Both arrows lower to the same
semantics. Formatter normalization is allowed if both input spellings round-trip
semantically. Directive callback slots consume the same callback representation;
no second pattern parser and no callback-valued JSON representation.

### Replacement contract: retainable lexical frames

The historical stack-linked `Locals<'a>` sketch is not the sole scope contract.
Use an immutable frame with a parent handle and multiple bindings, so an object
pattern can bind all fields atomically after shape validation. Frames must be
retainable through owned/shared handles when a directive carries its context
into later pipeline stages; expression evaluation borrows a view of the same
model for immediate callback execution. Do not introduce a second resolver or
copy local values into frontmatter to extend their lifetime.

Runtime bindings retain values, original source-item location, and per-value
provenance, including caller-file origins. Passive bindings retain projected
types and source spans without needing evaluated values. Both views share
binding identities, explicit/implicit lookup, and shadowing rules. A guarded
`local.x` and bare `x` name the same local subject; `fm.x` is a separate subject.
The nearest binding wins even when null. Restoring the parent after success or
failure must also restore its refinement facts.

The former occurrence sketch `/files/{index}` is insufficient for all new
inputs: object entry `value` must retain the original property origin, while
`key` is the entry's key string. Destructured fields retain their respective
origins; projection must not substitute a synthetic local path for the source
path. Use the existing value-origin/file-argument machinery for these mappings.

Before freezing APIs, prove both immediate expression evaluation and retained
frame use after callback exit, including nested shadow restoration, local type
projection, null shadowing, and file-origin preservation. This extends Phase 2
coverage rather than requiring another open-ended design spike.

### Dispatch, errors, and operational integration

The registry-owned callback evaluation mode remains appropriate. Bind iterable
inputs against the widened array/object/null signature and adapt them through
the shared collection entry representation. Keep scalar-kind errors in the
shared binder. The historical `ExpressionError::ArgType` proposal was already
superseded by `ExpressionError::Call` and must not be revived.

Extend callback error context from an index-only wrapper to a source-item
location capable of an original array index or object key. Preserve nested
wrappers and underlying typed cause, source span, diagnostic code, and fatality.
Error accessors must remain transparent through callback context.

Budget defaults, standalone session controls, and generated-output accounting
must be finalized with the directive work before the affected runtime path.
Iteration counts are shared across callbacks/directives/transcluded documents;
expression arrays are values rather than rendered document bytes. Do not charge
the same rendered bytes again just because they passed through an expression.

### Verification of this reconciliation

Documentation/design changes only. Check the spec, unstarted plan, and handoff
for contradictory active instructions and run `git diff --check`. No production
tests are warranted until implementation changes. The current acceptance matrix
has twelve criteria, including patterns, retained scopes, known-type propagation,
object entries, and shared budgets. Historical Phase 1 test results below are
not evidence that callbacks are implemented.

## Historical Phase 1 Record

The following records the original planning session under its former identifier.
Its proposals and prerequisite observations are historical; the current handoff
and revised spec/plan supersede the conflicting contracts.


# Implementation Log for 2026-10-20-map-and-filter (5 phases)

## Phase 1

Phase 1 is a rulings-and-design phase. It changed no source code, test, or
catalog artifact; it produced the dispositions and contracts below. No
prototype code was written, so there is nothing to remove or carry forward.

### Headline: the hard prerequisite is not implemented

`2026-10-28-schema-enhancements` (in the Claudine area's features directory)
is `status: finalized-spec`, `implemented` unset; its `plan.md` has 0 of 22
tasks checked and it has no implementation log. Evidence gathered on
2026-10-20 at `HEAD` `455eb05e4`:

- `darkmatter/schemas/partials/functions.yaml` does not exist. The only
  `functions.yaml` is the review **draft**
  `claudine/docs/schemas/partials/functions.yaml` (836 lines, header: "Draft
  function contracts for review … These declarations target the shared
  argument-conversion layer, not today's runtime").
- The embedded runtime catalog is still
  `darkmatter/docs/schemas/expression-functions.yaml`, loaded by
  `expression/catalog/{mod,parser}.rs` via `include_str!`, with the old
  `verification: executable | display-only` example fields.
- No `function(...)` SimplifiedSchema type, no shared signature checker, no
  `example(call => result)` parser exists in code.
- The prerequisite spec currently says (line 234): "An executable registration
  whose parameter or return requires a function value fails catalog
  registration". This feature must amend exactly that restriction.

Per the plan's Wave 1 rule ("If the shared work is unavailable, report the
dependency and stop dependent implementation rather than building a shim"),
**Phase 2's "Check callback signatures" task and Phase 3's "Publish catalog
entries" task are blocked** until the prerequisite lands. Phase 2's parser and
passive-scope tasks do not need the shared signature format and could proceed
independently if the author allows a split (see `human_review_items`).

### Rulings and dispositions

The author was not available in this non-interactive session. Every row below
is therefore an agent **recommendation** unless marked otherwise; none is an
author ruling. `yolo: true` does not resolve them (plan, Phase 1).

| Ruling | Disposition | Status |
| --- | --- | --- |
| Shared signature prerequisite | Prerequisite absent (evidence above). Do not invent a temporary signature format. Block the signature-checking and catalog-publication tasks on it. | **Blocked** — needs author decision on ordering |
| Catalog ownership | Authoritative source after the prerequisite is `darkmatter/schemas/partials/functions.yaml` as the spec says; today it is `darkmatter/docs/schemas/expression-functions.yaml`. The prerequisite (not this feature) owns the move/conversion and the consumer list (`catalog/parser.rs` `include_str!`, `catalog/mod.rs` parity test, DMLS function hover/completion, Claudine draft). Until then, do not add `map`/`filter` to the old catalog (that would be the second catalog copy the plan forbids). | Recommendation; depends on prerequisite |
| Shadow advisory | Keep it, narrowly: warn only when a callback parameter name equals a **declared** document property (frontmatter key or schema property) at that site. Explicit `fm.name` is unchanged and earns nothing. Reusing an enclosing callback's parameter earns nothing (spec: "introduces no additional prohibition or advisory"). | Recommendation — author to confirm (spec Open Question 1) |
| Diagnostic identities | See "Diagnostic identity proposal" below. | Recommendation — author to confirm (spec Open Question 2) |
| Operational policy | Sequential per-item evaluation, in order, inside the enclosing expression's one evaluation scope (`begin_expression_scope` is **not** reopened per item, so `current` memoization, permissions, remote policy, caches, and timeouts are the enclosing expression's). No new list-size, nesting, or iteration limit. No new effect permission. Effects completed before a failing item are not rolled back. No performance spike: the spec gives no target size or known cost. | Recommendation — author to confirm (spec Open Question 3) |
| Scope and syntax | Callback-only argument representation (contract below); `serde_json::Value` unchanged; binding classification goes through `BindingView` (including host globals) with no hardcoded Claudine names; callbacks are parsed and passively checked on empty/null input; identifier and condition-mode operator rules unchanged. | Recommendation, consistent with the spec's agreed scope semantics |

#### Diagnostic identity proposal

| Situation | Code | Severity | Raised by | Span |
| --- | --- | --- | --- | --- |
| `=>` outside a call-argument slot (top level, array/object element, operator operand, ternary branch, parenthesized value) | `dm.expression.misplaced_callback` | error (authoring-fatal) | parser | whole callback, `param` start to body end |
| Callback in a call-argument slot the catalog does not declare as a function parameter (e.g. `length(x => x)`, named or value-only position) | `dm.expression.misplaced_callback` (same identity) | error | passive validation (`validate_expression`), catalog-driven | whole callback |
| Callback with zero or several parameters (`() => 1`, `(a, b) => 1`) | `dm.expression.parse_failure` (existing), message names the single-parameter rule | error | parser | the parameter list |
| Parameter named after a reserved namespace, the `null` root, or a global the site's `BindingView` declares (available **or** unavailable) | `dm.expression.reserved_callback_parameter` | error (authoring-fatal) | passive validation using the site's view | parameter name |
| Parameter hides a declared document property | `dm.expression.callback_shadows_property` | warning, deduplicated by the existing warning identity (code + subject = parameter name + site) | lint pass | parameter name |
| Arity / non-callback argument / wrong callback shape for `map`/`filter` | the prerequisite's shared signature-check codes | error | shared checker | the argument |
| Error inside a callback body at runtime | the **inner cause's** code (unchanged), with wrapper context in the message | inner cause's severity/fatality | evaluator | inner cause's span |
| Genuinely undeclared document property read inside a body | `dm.expression.undeclared_property` (existing, unchanged) | warning | absence observer | the read |

Rationale for "inner cause's code": consumers already classify by variant and
code (file-reference failure classes, authoring-fatal status, Claudine error
rendering). Giving a wrapped failure a new code would silently reclassify it.

### Design spike findings (read-only inspection)

Inspected `expression/{ast,parser,lexer,mod,prepared,binding,absence,lint,error}.rs`,
`expression/scope/`, `expression/functions/mod.rs` (registry/dispatch), and the
DMLS overlay. Facts that shape the contracts:

1. **The lexer folds dotted names into one token.** `dep.name` lexes as one
   `Token::Variable("dep.name")`, producing `Expr::Variable("dep.name")`, not
   `MemberAccess`. Local resolution must therefore match on the **root
   segment** (`absence::root_of`) and project the remaining segments onto the
   local value. `dep[0]` is `Index { base: Variable("dep") }` and needs no
   special handling.
2. **`=` alone is a lex error today** (`lexer.rs` "Expected '=' after '='"),
   so `=>` can be claimed as a new `Token::Arrow` without changing any
   currently valid input. `>` remains a comparison token.
3. **Ordinary dispatch is eager.** `evaluate_function` evaluates every
   argument before `functions::dispatch{,_fs}`; only `and`/`or` are
   special-cased through `LAZY_BINDINGS` (`EvaluationMode::Lazy`,
   `handler: None`) and `lazy_arity_eligibility`. The registry already has the
   seam a callback binding needs.
4. **Every lookup path funnels through Darkmatter's `evaluate_expr`.** There
   are 21 non-test `EvaluationLookup` implementors (Darkmatter and Claudine);
   Claudine's lifecycle `invoke_expression_function` builds an
   `Expr::FunctionCall` and evaluates it through Darkmatter. A local binding
   implemented inside `evaluate_expr` reaches every surface without touching
   those implementors.
5. **Caller-file provenance is keyed on the argument's frontmatter pointer.**
   `caller_file_occurrence(Expr::Variable(p))` returns `/p`. For a callback
   local this would wrongly name a document property (`frontmatter(dep)` would
   look up provenance for a property called `dep`).
6. **Branch facts are keyed by expression.** `scope.facts().assume(expr, …)`
   records facts about variable paths. A fact recorded outside a callback
   about document property `tags` must not apply to a local named `tags`
   inside the body.
7. **Errors are flat and consumers match variants directly.**
   `ExpressionError` is `Clone`, flat by design, with no "root cause"
   accessor. `ExpressionError::FileReference(..)` is matched directly in
   `markdown/errors/blocks.rs`, `functions/{mod,repository}.rs`, and
   `claudine/lib/src/composition/error/render/mod.rs`.
8. **`ParseError` is message + position.** There is no kind or end offset, so
   a dedicated misplaced-callback identity with a span needs a typed kind.
9. **DMLS classifies against `BindingView::baseline()`** (no host globals).
   It can reject namespace and `null` parameter names, but not Claudine host
   globals, unless it is given a host view.
10. **`null` is a variable root** (`NULL_ROOT`), not a literal token; a
    parameter named `null` must be forbidden alongside the namespaces.

### Contract: callback node

- Lexer: `=` immediately followed by `>` is `Token::Arrow`. Nothing else
  changes.
- AST: function-call arguments become a callback-aware argument type, in both
  forms, so a callback is unrepresentable as a value:

  ```rust
  // span-erased
  Expr::FunctionCall { name: String, args: Vec<CallArgument> }
  enum CallArgument { Value(Expr), Callback(Callback) }
  struct Callback { param: String, body: Box<Expr> }

  // spanned (erase() maps 1:1)
  SpannedExprKind::FunctionCall { name: String, args: Vec<SpannedCallArgument> }
  enum SpannedCallArgument { Value(SpannedExpr), Callback(SpannedCallback) }
  struct SpannedCallback { param: Spanned<String>, arrow: SourceSpan, body: Box<SpannedExpr>, span: SourceSpan }
  ```

  Chosen over an `Expr::Lambda` variant because the spec's core invariant is
  "a lambda is an argument, never a value"; with a variant, programmatic
  constructors (Claudine builds `Expr::FunctionCall` directly) could place one
  anywhere. Both choices force every exhaustive consumer to change at compile
  time; this one also makes misplacement unrepresentable. Cost: every
  `args` iteration changes (list under "AST consumer ownership").
- Parser: inside `parse_function_call`, an argument that is
  `IDENT Arrow …` is parsed as a callback whose body is `parse_expression()`
  (lowest precedence: `x => a ? b : c` is `x => (a ? b : c)`; the body ends
  at the call's `,` or `)`). Anywhere else, encountering `Arrow` is a parse
  error with kind `MisplacedCallback` and the callback's span. `(a, b) =>` and
  `() =>` in argument position are parse failures naming the one-parameter
  rule. The parser stays catalog-independent; catalog placement is a passive
  validation check (next bullet). Both modes (interpolation and condition)
  use the same rule.
- Catalog-driven placement: `validate_expression` rejects a callback in a
  slot the catalog does not declare as a function parameter, with the same
  `misplaced_callback` identity. This needs the prerequisite's function
  parameter types; until then only parser placement is checkable.
- `Display` renders `param => body` so formatting and `lint` rewrites
  round-trip.
- Incomplete editor input: `map(xs, x =>` with no body is an ordinary
  "expected expression" parse error at end of input; DMLS completion inside a
  callback body uses the same recovery path it uses for any incomplete call
  argument. No second grammar.

### Contract: lexical scope

- Runtime: an explicit, stack-allocated local environment is threaded through
  `evaluate_expr` alongside `AbsenceScope` (preferably as a field of it, since
  `AbsenceScope` already reaches every recursive call):

  ```rust
  struct Locals<'a> { name: &'a str, value: &'a Value, occurrence: Option<&'a str>, parent: Option<&'a Locals<'a>> }
  ```

  `Expr::Variable(path)` checks `Locals` for `root_of(path)` **before**
  `lookup.resolve`; on a hit it projects the remaining dotted segments with the
  existing member semantics and never calls the observer (so a local is never
  an undeclared property). Explicit `fm.*`, namespaces, and globals never
  reach a local because their roots are forbidden as parameter names.
  Restoration on callback exit **or failure** is structural: the frame lives
  on the Rust stack for one item's evaluation.
- Rejected alternative: a forwarding `EvaluationLookup` wrapper. It would
  have to forward ten trait methods and must deliberately *not* forward
  `begin_expression_scope`; any future trait method with a default would
  silently drop its override. The explicit environment has no such drift.
- Provenance: a callback item bound from an iterable whose own argument has a
  caller-file occurrence (`/files`) carries `/files/{index}`, mirroring the
  existing `Index` logic in `evaluate_caller_file_argument`; otherwise
  `None`. `caller_file_occurrence` returns the local's occurrence, never
  `/{param}`.
- Facts: entering a callback body drops branch facts whose root is the
  parameter name, so outer facts about a same-named document property do not
  narrow the local.
- Passive analysis: one binder-aware read walker in `absence.rs`
  (`static_variable_reads` extended with a bound-name set) is the single
  source of "which reads are free". Roots bound by an enclosing callback are
  excluded from document-root collection, undeclared-property checks, and
  shell-blocking dependency analysis, but `ctx.*`/namespace/global reads in
  bodies are still collected for context-requirement discovery. Consumers
  that currently re-walk the tree (list below) should call it rather than
  each reimplement binder handling.
- Types: a local has design-time type `unknown`; no item or result type is
  inferred.

### Contract: callback dispatch and error wrapper

- Registry: add `EvaluationMode::Callback` beside `Lazy`. `map` and `filter`
  register with it; `evaluate_function` routes `Callback` bindings to a
  callback evaluator before the eager path, using the registry for name,
  alias, and arity, exactly as `and`/`or` do.
- Evaluation: evaluate the iterable argument once, in the enclosing scope and
  locals; `null` → `[]`; array → iterate; any other type →
  `ExpressionError::ArgType { function, index: 0, expected: "array", actual_type }`.
  Evaluate the body once per item in order. `filter` keeps the original item
  when `is_truthy(result)`, never short-circuits.
- Wrapper: a new variant, not a message rewrite:

  ```rust
  #[error("{function}(): item {index}: {source}")]
  Callback { function: &'static str, index: usize, source: Box<ExpressionError> }
  ```

  plus `ExpressionError::root_cause(&self) -> &ExpressionError` (follows
  `Callback` to the innermost error). `is_authoring_fatal`,
  `is_missing_runtime_context`, `glob_hint`, and diagnostic-code selection
  delegate to `root_cause()`. Consumers that match `FileReference(..)`
  directly (`errors/blocks.rs`, `claudine/.../error/render/mod.rs`, and the
  `functions` sites that construct rather than match can stay) must match on
  `root_cause()`. Nested calls produce nested wrappers, e.g.
  `map(): item 1: filter(): item 0: frontmatter(): …`. The inner error's
  location (`relocate` in `prepared.rs`) is preserved because the wrapper adds
  no span of its own.

### AST consumer ownership

Compile-forced by the `CallArgument` change. "Binder" means the consumer must
exclude callback-bound roots (preferably by calling the shared walker).

| Consumer | Kind | Owner |
| --- | --- | --- |
| `expression/{ast,lexer,parser}.rs` | grammar, `erase`, `Display` | Phase 2 parser task |
| `expression/{absence,prepared,lint,binding}.rs`, `expression/scope/facts.rs` | passive walk; binder | Phase 2 passive-scope task |
| `compose/preflight/collect.rs` (`collect_expression_roots`, `visit_calls`) | context requirements; binder | Phase 2 passive-scope task |
| `compose/frontmatter_interpolation.rs` (`collect_roots`, shell-blocking roots, `static_truthiness`) | binder | Phase 2 passive-scope task |
| `compose/shell_expansion/template/candidate.rs` (`read_roots`), `compose/remote.rs` (`collect_expression_urls`), `errors/blocks.rs` (`collect_frontmatter_roots`) | binder | Phase 2 passive-scope task |
| `compose/interpolation/evaluator.rs` (`walk_context_variables`, `presentation_value`) | binder | Phase 2 passive-scope task |
| `expression/mod.rs` (`evaluate_expr`, `evaluate_function`), `expression/error.rs`, `functions/mod.rs` | runtime | Phase 3 runtime task |
| `dmls/src/overlay/expressions.rs` (`children`, `expression_at`, `function_call_at`, `root_identifier`, `references_key`, `completion_candidates`), `dmls/src/providers/dsl.rs` | editor | Phase 3 editor task (Phase 2 makes it compile) |
| Claudine: `lifecycle/prewarm/classify.rs` (`frontmatter_reads`, `contains_call`), `looping/config.rs` (`collect_identifiers`), `sequence/preflight/shape.rs`, `dispatch/matcher.rs`, `lifecycle/bindings{,/rename}.rs`, `lifecycle/executor.rs` | binder / construction | Phase 2 makes them compile and binder-correct; Phase 4 lifecycle task tests them |

The `claudine/features/2026-10-28-schema-enhancements/spikes/coercion-baseline`
snapshot also matches `Expr::FunctionCall`; it has no Cargo manifest and is
not a workspace member, so it is out of scope.

### Verification

No source, test, or catalog file changed in this phase, so no test was
added and there is no requirement-to-test mapping yet. That mapping is due in
Phase 4.

- Gate-skip check (from the repository root):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  returned `packages: [claudine, test-toolkit, claudine-cli, darkmatter,
  darkmatter-cli, dmls]`, `change_class: package`. That comes from uncommitted
  files this feature did not touch: the strict-equality docs and
  `claudine/lib/src/composition/schema/tests.rs`, which were already in the
  working tree when the phase started. The gates were therefore run.
- `just test` (darkmatter), first run: 10674 passed, 1 failed, 10 skipped,
  and the run was cancelled with about 114 tests unrun. The failure was
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (`searched_candidates.rs:169`, watch registration count 3 instead of 2).
  Rerun alone it passed 3 times out of 3, so it is recorded as
  **load-sensitive**, and this phase made no code change that could affect it.
- `just test` (darkmatter), full rerun: **10799 passed, 10 skipped, 0 failed**.
- `just lint` (darkmatter): **passed**, including the zed-dmls wasm32-wasip2
  check.
- Dependent packages (Claudine): not run. This phase changed no code, so no
  symbol a dependent uses changed.
- Cross-OS check: not run. There is no code change and no OS risk.
- Darkmatter skill: not updated. The contracts above are proposals pending
  author confirmation; the skill is updated when the behavior lands.
