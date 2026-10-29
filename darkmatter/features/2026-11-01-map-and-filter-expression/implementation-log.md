---
spec: 2026-11-01-map-and-filter-expression
plan: plan.md
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
human_review: false
message_to_agent: |-
  Phase 3 is implemented: map/filter execute serially using retained lexical
  scopes and the shared binder. EvaluationLookup::iteration_budget forwards
  session/request counters; pipeline-installed contexts share runtime.iteration.
  Discovery contexts remain noncharging. Expression arrays charge visits only.
  ExpressionError::Callback retains typed nested sources and original positions;
  without_callbacks and callback_path expose cause/path, and MarkdownError::iteration_failure
  prevents child interpolation exhaustion from being tolerated or hidden by as_markdown.
  DMLS now provides callback-local completion, owner/type hover and semantic tokens;
  catalog examples execute and the generated table is refreshed. Phase 4 owns the
  remaining compose/lifecycle acceptance matrix. Preserve all prior uncommitted
  Phase 2 changes in this checkout; nothing was staged, formatted, or committed.
decisions_for_review:
  - |-
    Calls containing callbacks use a distinct call node with value-or-callback
    arguments. Existing value-only calls keep their current argument shape.
    This preserves compatibility for ordinary callers while making callbacks
    impossible to store as values. A single call variant would simplify some
    traversals but require changing every existing caller and constructor; choosing
    it later would require migrating those constructors and the two call visitors.
  - |-
    Directive work already supplied the shared callback parser, retained scopes,
    iterable typing, budgets, and file-predicate narrowing. This phase extends
    those implementations instead of duplicating them. Expression execution stays
    in the next phase as planned, so registered callback calls currently fail
    explicitly if execution is attempted. Moving execution into this phase would
    require completing the runtime and budget tests scheduled for the next phase.
  - |-
    The root test wrapper failed before running a feature-enabled single-package
    selection. I used the existing per-package test recipe with explicit features
    to run the same L1 coverage, keeping shared runner changes outside this
    expression implementation. Fixing the wrapper here would broaden the change
    into build infrastructure. If review chooses that scope, fix its argument
    forwarding and rerun the wrapper; no expression code needs changing.
source_files_during_phase_2:
  - claudine/lib/src/composition/lifecycle/prewarm/classify.rs
  - claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
  - claudine/lib/src/dispatch/matcher.rs
  - claudine/lib/src/dispatch/matcher/tests.rs
  - claudine/lib/tests/l1/claudine_schemas.rs
  - darkmatter/cli/tests/l1/compose_call_warnings.rs
  - darkmatter/dmls/src/diagnostics/call_check.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
  - darkmatter/lib/src/lifecycle/schema_projection.rs
  - darkmatter/lib/src/lifecycle/signatures.rs
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
  - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
  - darkmatter/lib/src/markdown/compose/expression/call_check.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/lint.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/prepared.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/lexical.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/tests/l1/function_binding_registry.rs
  - darkmatter/lib/tests/l1/function_call_binding.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
  - darkmatter/schemas/partials/functions.yaml
docs_updated_during_phase_2:
  - darkmatter/docs/iterables.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/features/2026-11-01-map-and-filter-expression/implementation-log.md
  - darkmatter/features/2026-11-01-map-and-filter-expression/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/collections.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/semantic_tokens.rs
  - darkmatter/dmls/tests/l1/collection_editor.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/binding.rs
  - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/nested.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/site_scope.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/tests/l1/collection_execution.rs
  - darkmatter/lib/tests/l1/function_call_binding.rs
  - darkmatter/schemas/partials/functions.yaml
docs_updated_during_phase_3:
  - darkmatter/README.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/features/2026-11-01-map-and-filter-expression/plan.md
  - darkmatter/features/2026-11-01-map-and-filter-expression/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/expressions.md
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

## Phase 2

### Requirement-to-test map (before implementation)

- Callback grammar boundary (F1, L1): shared parser tests for both arrows, shallow patterns, nested calls, ternaries, comma boundaries, misplaced callbacks, and incomplete input; erased/spanned AST parity and formatting round trips.
- Passive lexical boundary (F1, L1): public static read inventories and call checks cover bare/explicit local reads, explicit frontmatter, nested shadowing, null bindings, projected array/object/tuple types, invalid patterns, and restoration. Existing retained-frame and provenance tests cover the shared directive substrate.
- Signature/catalog boundary (F2, L1): shipped catalog registration and public preparation checks cover callback positions, invalid counts/types, forbidden host names, dead-body category 5 errors, and passive provider behavior.
- Composition/CLI boundary (F2, L1): normal composition and md invocation must reject an invalid callback body before execution, including empty input. DMLS shares the same located incompatible-call projection.
- File predicates already register true-implies file in this checkout; verify existing narrowing tests and add filtered-expression projection coverage. Text grammar changes do not use the configuration-input matrix; no persistence is added.

The existing directive implementation supplies shared patterns, both arrow tokens, lexical runtime/type frames, iterable projection, and budgets. This phase extends those contracts rather than rebuilding them. Runtime callback execution and editor UI enhancements remain Phase 3 work.

### Integration findings

The first focused run found that literal arrays/objects were assigned only a broad
container type before iterable projection. This hid invalid object patterns and
shadowed-local argument incompatibilities. Composite literals now use the existing
recursive member/element projection; scalar literals retain their precise types.
Each failing test is rerun alone before the combined suite. A fixture compilation
error from the added pattern span was corrected before test execution.

The existing `file_exists` and `is_file_type` registrations already use true-implies
`file` facts, including a non-excluding false side. No predicate behavior rewrite
was necessary. Claudine has two exhaustive AST consumers; both now recognize
callback calls, and callback dependency reads use the shared inventory.

### Completed grammar checkpoint

Both arrows, shared shallow patterns, nested callback call arguments, body operators,
spanned/erased parity, and span-preserving display are implemented. The new parser
cases passed in the initial scoped run; invalid placement and incomplete syntax
retain `dm.expression.invalid_callback`. All three initially failing analysis tests
now pass individually. The result test accepts numeric literal spelling across
integer/float representations while retaining the precise public projected type.

The first tier-coverage invocation could not list a test binary during integration
compilation; rerun it after the source and feature builds settle. No test was marked
ignored or moved to another tier. The linker reports its existing large unwind-table
warning for the library test binary; it does not prevent execution.

### Passive scope checkpoint

The combined focused run exercised 449 parser/callback tests across the library,
CLI, and DMLS: 448 passed. The remaining test incorrectly expected md’s permissive
unknown-verb mode to reject a callback verb; it now uses the normal lifecycle
parser with its existing strict unknown-verb policy, and is being rerun alone.
The CLI report-json empty-input rejection and DMLS located callback diagnostics
passed. Passive local binding, nested restoration, free-read inventory, declared
array/tuple/object projection, and file-filter result typing passed.

### Signature checkpoint

The corrected shipped-catalog/lifecycle-exclusion test passed alone through strict
lifecycle parsing. All Phase 2 checklist items are now complete. Extra coverage
asserts that registered `seq` and unavailable `loop` cannot be bound as parameters,
and a failing provider fake proves DMLS never executes either populated or null
callback bodies. Claudine’s matcher compilation and speech prewarm classifier
have targeted callback cases. Runtime evaluation remains Phase 3.

The required scope command selected `claudine`, `darkmatter`, `darkmatter-cli`,
`dmls`, and suite-input consumer `test-toolkit`, with `change_class: package`.
Neither test nor lint gates is skipped. The edits introduce no OS-dependent path
or platform code, so no optional Windows cross-check is warranted.

### Full-gate correction

The first `just test` attempt stopped after 1,346 passes at the new protected-name
fixture: its unavailable-global reason used spaces, which `BindingView` correctly
rejects. The fixture now uses the valid `host.loop` reason. This was a test setup
error, not a production defect or load-sensitive failure. Rerun the single failing
test before the gate; no cases are dropped or retiered.

The corrected protected-name test passed alone (0.111 s). Lint runs before the
final full-suite rerun so any lint corrections can be covered by that run.

Lint found a redundant conversion closure and a conversion impl placed after its
test module. Both were corrected manually. A rerun was accidentally started from
the repository root; that workspace lint and its child processes were terminated
(exit 143), and the required Darkmatter-only lint was restarted. No formatting
command was run.

The next full gate reached 8,190 passes before the registry ledger guard correctly
reported that `map` and `filter` lacked value-handler rows. That ledger now checks
the two callback functions through their passive call contracts and retains every
existing handler row. Its exact inventory comparison still rejects an untested
registration. The corrected guard passed alone (0.066 s). No production behavior
changed for this correction. The generated function table also matches the new
catalog descriptors.

### Scope and completion regression follow-through

The full rerun reached 12,953 passes before the DMLS completion golden reported
two intended additions in the lifecycle guard expression surface: `filter(iterable,
callback)` and `map(iterable, callback)`, with no removed items. The exact golden
test also failed alone; update only that artifact, then read it back through the
same normal test without the update environment.

Review found an absence-analysis defect in the new callback walk: it discarded
enclosing guard facts for free captures, and its local guards could be mistaken
for document guards. The regression inputs are `present ? map([], p =>
present.name) : []`, `map([null], p => p ? fm.p : null)`, and its `local.p`
variant. The read inventory now enters the existing lexical type-frame model
while retaining enclosing facts, keeping `fm.p` separate from the local.

The scoped absence regression passed (0.065 s). The DMLS golden update passed,
and a separate invocation without `DMLS_UPDATE_GOLDEN` read the artifact back
and passed (0.468 s). Only the two expected completion entries changed.

### Downstream scope

Sniff’s package dependency inventory and an audit of expression AST consumers
identified Claudine’s matcher/prewarm walkers and lifecycle schema inventories as
the changed API consumers. Its large library and CLI suites use focused matcher,
prewarm, schema, host-binding, and composition-presence tests. Messenger’s large
library suite uses the research corpus/validation binaries, which exercise passive
schema and composition loading. Small dependent packages (under 200 source test
functions) receive their full L1 runs: biscuit-icon-cli, biscuit-speaks-cli,
claudine-contract, messenger-cli, and playa-cli. Runs are serial.

The other large direct dependents were inspected: biscuit-terminal-cli and sniff-cli
use Markdown rendering, research uses Link rendering, and claudine-gen uses document
and schema loading rather than the changed expression AST/checker. Their relevant
unchanged interfaces have no tests exercising these changed symbols, so their full
suites are not run. No monorepo-wide test gate is used.

### Retained analysis-frame span contract

A complete 13,559-test area run passed (10 pre-existing ignored tests). Final audit
then found that passive frames retained types/identities but declaration spans
remained only on the callback AST. `TypedLocal` now optionally owns its declaration
span, `ItemFrame::enter_spanned` attaches each pattern field’s span, and
`LexicalTypes::span_of_name` uses nearest-binding lookup. The new lifetime test
drops the AST before querying the retained frame and checks that an unlocated
shadow cannot inherit an outer declaration’s span. Existing unspanned callers
retain their API. The final gates will cover this addition.

### Pre-existing ignored tests

No ignore or tier exclusions were added. The canonical gate skips these existing tests:

- `darkmatter/lib/src/layout/page/tests.rs::f35_6_rhythm_raw_samples` — measurement harness; opt in with DM_PERF_RAW_DIR.
- `darkmatter/lib/src/markdown/render_tree/build_context.rs::f35_7_link_policy_raw_samples` — measurement harness; opt in with DM_PERF_RAW_DIR.
- `darkmatter/lib/src/markdown/render_tree/code_renderer.rs::f23_code_surface_raw_samples` — measurement harness; run explicitly with DM_PERF_RAW_DIR set.
- `darkmatter/lib/src/markdown/compose/shell_expansion/alias.rs::resolve_alias_ll` — requires an `ll` alias in the user's login-shell configuration.
- `darkmatter/dmls/src/diagnostics/frontmatter.rs::a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias` — alias analysis is not yet ported onto ScalarProjection.
- `darkmatter/dmls/tests/l1/lsp_session.rs::undeclared_properties_in_block_and_multi_line_expression_values_warn_at_their_own_ranges` — alias and block-scalar projection are not yet ported onto ScalarProjection.
- `darkmatter/dmls/tests/l1/lsp_session.rs::tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored` — alias and block-scalar projection are not yet ported onto ScalarProjection.
- `darkmatter/dmls/tests/l1/lsp_session.rs::an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote` — alias and block-scalar projection are not yet ported onto ScalarProjection.
- `darkmatter/dmls/tests/l1/lsp_session.rs::a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix` — alias and block-scalar projection are not yet ported onto ScalarProjection.
- `darkmatter/dmls/tests/l1/lsp_session.rs::aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved` — alias and block-scalar projection are not yet ported onto ScalarProjection.

### Final area gates

- `cd darkmatter && just test`: **13,560 passed**, 10 existing ignored tests,
  135.623 s execution. The new lifetime test passed alone first (0.028 s).
- `cd darkmatter && just lint`: **passed** for darkmatter, darkmatter-cli, dmls,
  zed-dmls-cli, and the Zed WASM extension (`wasm32-wasip2`).
- All new and changed callback tests stayed below the 5 s L1 budget.
- No source formatting, staging, commits, or spec lifecycle moves were performed.

The downstream runs passed 901 Claudine tests, 9 Claudine CLI tests, and
64 Messenger tests. The root `just test biscuit-icon-cli` wrapper failed before
execution because its single-package feature spec (`--features image`) was split
into package names. Use the existing canonical per-package `just _test` helper
with explicit features for the remaining runs; no test failed and no shared CI
or just-runner code is changed as part of this expression feature.

### Null-item pattern boundary

The requirement audit identified a shared projector edge case: an empty entry
shape and a null-only item both had no non-null alternatives, so an object pattern
over `[null]` was treated like an empty source. Add the exact negative input
`map([null], {key} => key)` beside the controls `map(null, {key} => key)`,
`map([], {key} => key)`, and `map([{key: null}], {key} => key)`. Only the null
item must fail the object pattern; a present null field remains valid data.
The signature checklist item is reopened until this regression passes.

The null-item regression failed before the fix with the original exact input and
passed after it (0.050 s). The shared projector now treats only an empty, non-null
entry shape as having no items; a null-only item reaches the existing not-an-object
error. The null-source, empty-source, and present-null-field controls pass. The
signature checklist item is checked again. Shared iterable and expression docs
state this distinction.

### Downstream results

- `cargo nextest run -p claudine --features work-counters -E '(test(claudine_schemas) | test(collection_host_compat) | test(type_scope_ownership_guard) | test(dispatch::matcher) | test(composition::lifecycle::prewarm::classify) | test(host_catalog)) & !test(/(^|::)(level2_|level3_|browser_|real_|slow_)/)' --color=never` — exit 0; Summary [  45.167s] 901 tests run: 901 passed (1 slow), 5564 skipped.
- `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(compose_presence_parity) | test(compose_schema_cli::compose_schema_grammar_error_reports_invalid_schema_without_launching_provider)' --color=never` — exit 0; Summary [   1.485s] 9 tests run: 9 passed, 4924 skipped.
- `cargo nextest run -p messenger --features research -E 'binary(research_corpus) | binary(research_validation)' --color=never` — exit 0; Summary [   3.589s] 64 tests run: 64 passed, 0 skipped.
- `just _test biscuit-icon-cli --features image` — exit 0; Summary [   0.414s] 99 tests run: 99 passed, 0 skipped.
- `just _test biscuit-speaks-cli` — exit 0; Summary [   0.950s] 107 tests run: 107 passed, 1 skipped.
- `just _test claudine-contract` — exit 0; Summary [   0.075s] 48 tests run: 48 passed, 0 skipped.
- `just _test messenger-cli` — exit 0; Summary [   3.395s] 133 tests run: 133 passed, 0 skipped.
- `just _test playa-cli` — exit 0; Summary [   0.264s] 25 tests run: 25 passed, 0 skipped.
- `cargo clippy -p claudine --all-targets -- -D warnings` — exit 0; Clippy passed.

Biscuit Speaks CLI’s one exclusion is the pre-existing real-provider test
`real_cli_speaks_with_kokoro_provider`, outside L1. Other downstream skipped counts
come from the explicit focused filters above. All selected downstream tests and
Claudine Clippy passed. A second mistargeted workspace lint was stopped immediately
(exit 143); the recorded Darkmatter lint is the correctly scoped run.

### Requirement-to-test evidence

- Grammar and span boundary (F1/L1): parser tests
  `call_arguments_preserve_callback_spans_and_erasure`,
  `nested_callbacks_and_commas_have_one_grammar`,
  `callbacks_cannot_escape_argument_positions`, and
  `incomplete_callback_reports_the_shared_parser_location`.
- Lexical/type boundary (F1/L1): `callback_read_inventory_keeps_only_free_captures`,
  `callback_nested_scope_restores_outer_binding_and_separates_frontmatter_facts`,
  `callback_declared_arrays_tuples_and_objects_keep_known_types`,
  `callback_result_projection_preserves_members_nulls_and_filter_items`,
  `callback_patterns_protect_registered_loop_and_seq_globals`, and
  `callback_type_frames_retain_binding_spans_after_the_ast_is_dropped`. Existing
  retained runtime-frame, null-shadowing, provenance, and file-predicate tests
  passed in the full gate.
- Signature/pattern boundary (F1/L1):
  `callback_calls_check_dead_bodies_and_shadow_document_types` and
  `callback_signatures_and_patterns_reject_invalid_calls`, including the exact
  null-item regression and absent/null/empty/present-field controls.
- Shipped catalog and registration boundary (F2/L1):
  `callback_mode_accepts_inline_parameters_but_never_stored_values`,
  `callback_mode_rejects_callbacks_nested_in_stored_parameters`,
  `shipped_collection_callback_signatures_are_not_lifecycle_verbs`, and the
  registry’s exact inventory guard.
- Composition/CLI boundary (F2/L1):
  `compose_call_warnings::an_invalid_call_in_an_empty_callback_rejects_before_execution`
  spawns the normal md binary against the shipped catalog, checks the incompatible
  inner call diagnostic, and requires a null report document.
- Editor/provider boundary (F2/L1):
  `callbacks_reject_protected_patterns_and_nested_calls_without_execution`,
  `callback_analysis_never_runs_provider_handlers`, and the updated completion
  response contract. A failing provider fake and entry counter prove passivity.
- Shared consumers (F2/L1): the downstream commands above exercise Claudine’s
  normal schema/preparation and matcher/prewarm paths, Messenger’s corpus loader,
  and the small dependent CLIs. New tests are in existing declared targets and
  live L1 modules; the DMLS effects-instrumentation feature is enabled by CI.

No file-format loader, configuration field validator, or persistence mechanism was
added, so the configuration robustness matrix and persistent-value round trip do
not apply. The expression grammar and shipped catalog have normal-invocation
coverage; the completion golden was explicitly updated and read back.

### Closing verification

After the null-item fix, `cd darkmatter && just test` passed **13,560 tests** in
81.722 s, with the same 10 existing ignored tests. `cd darkmatter && just lint`
also passed, including Zed WASM. Tier coverage passed with **zero stranded tests**;
the null boundary adds matrix rows to an existing L1 test and changes no target
or tier names. All selected downstream tests and Claudine Clippy passed.

After the null-item correction, the final focused dependent command
`cargo nextest run -p claudine --features work-counters -E 'test(collection_host_compat) | test(type_scope_ownership_guard)' --color=never`
passed **10 tests** in 0.485 s. The earlier selected downstream suites passed
**1,386 tests** total, and Claudine Clippy passed. There are no unresolved selected
test or lint failures. Source/code comments, canonical docs, the skill topic,
checklist, metadata, and the next-phase handoff are updated. Phase 2 is complete.

The frontmatter `source_files_during_phase_2` is the complete 44-file source and
shipped-artifact inventory. The four documentation updates and the skill update
are listed separately there and on the plan. No new documentation file was created.
## Phase 3

### Requirement-to-test map (before implementation)

- Runtime collection boundary (F1/L1): public `evaluate` regressions for both arrows, null/absent/empty sources, scalar binding rejection, null items/results, insertion order, shallow patterns, truthiness, and nested locals/restoration.
- Evaluation authority boundary (F2/L1): ordered lookup fakes prove source evaluated once, each filter item visited, first-error stop, forwarding of host bindings and absence observations. Controlled temporary files exercise real `find_files`/`frontmatter` with the document context.
- Budget boundary (F2/L1): standalone session sharing, exact and exceeded visit limits, nested expression/directive accounting, and existing byte/cache expansion tests. Expression arrays consume visits but no rendered-byte allowance.
- Error boundary (F1/F2 L1): nested callback wrappers preserve function, original index/key, typed source and fatality; normal compose report verifies located failure.
- Editor boundary (F2/L1): passive provider tests for nested scopes, completion ordering, hover ownership/type and semantic ranges on body/frontmatter surfaces.
- Shipped catalog boundary (F2/L1): real catalog parser and executable examples cover nested/quoted arrows and pattern commas, lifecycle exclusion and registration parity.

Phase 2 changes were already present and uncommitted at session start; Phase 3 preserves them and records only its own edited source paths. No file-format loader or persistence is being added; catalog grammar changes receive shipped-artifact and normal execution coverage.

### Runtime integration checkpoint

Runtime evaluation now binds the source through the registered signature, captures it once, enters retained immutable frames, and evaluates callbacks serially with one budget authority. Null sources skip execution; null items/results remain values. Nested errors retain typed callback wrappers, original positions, authored document locations, and authoring-fatal classification. Absence facts now use lexical subjects inside bodies, preserving explicit frontmatter reads.

The initial seven focused runtime tests passed five cases and exposed two test-expectation mistakes: cleanup joins plain paragraph lines, and thiserror exposes the boxed expression source as a boxed error in the generic chain. The expected composed output and source-chain downcast were corrected. Both failing tests then passed alone before the combined run. Neither was load-sensitive. Real `find_files`/`frontmatter` reads and callback free-capture warnings passed.

Integration also identified a request-fatality gap: nested composition and tolerated transclusions recognized directive budget failures but not callback exhaustion inside interpolation. Added typed iteration-exhaustion traversal and explicit regressions for child transclusion and `as_markdown` fallback. No effect permissions or rollback behavior were added.

### Shared budget requirement-to-test map (before implementation)

- Session/lookup boundary (F1/L1): a session over an existing session/scoped lookup must consume the same visit counter; a failing reservation remains sticky when returning to the outer session. Extend `binding::lexical_tests` with `callback_sessions_and_scopes_share_the_enclosing_budget`.
- Request context boundary (F2/L1): body and frontmatter expression adapters must retain the pipeline counter; discovery adapters must not charge it. Add `callback_context_adapters_share_request_budget_and_discovery_is_free` in the existing options tests with a temp-directory request.
- Parent runtime/compose tests exercise real map/filter evaluation and mixed directives/child requests against this wiring. No configuration or persisted format is changed.

The first targeted compile overlapped the new trait's implementation, so it failed
before tests at missing `EvaluationLookup::iteration_budget`; the added exhaustive
options-classification requirement was corrected. The next compile reached only
an evaluator type mismatch in the parent's in-progress slice (coercion shape versus
lexical scope shape); no test has failed in the completed budget wiring.

The pipeline now attaches its shared counter before retaining nested-composition
options. Both body and frontmatter context adapters reuse it, while discovery
adapters use a noncharging budget. Scoped and site lookups forward the same
allowance; a newly associated session inherits an enclosing lookup/context's
allowance or creates its default standalone budget. Runtime counter state is
excluded from identity; the existing encoded limits remain in graph and cache
identity. Existing expansion-byte reservation and cached-subtree ledgers remain
unchanged.

Targeted verification passed:
`cargo nextest run -p darkmatter -E 'test(callback_sessions_and_scopes_share_the_enclosing_budget) | test(callback_context_adapters_share_request_budget_and_discovery_is_free)' --color=never`
— 2 passed, 10,187 filtered out, 0.051 s test execution. The two new L1 tests
ran from declared existing library targets in 0.023 s and 0.029 s. The linker
repeated the existing large unwind-table warning; compilation and execution passed.

Source files changed by this budget slice:
- `darkmatter/lib/src/markdown/compose/expression/binding.rs`
- `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`
- `darkmatter/lib/src/markdown/compose/context/options.rs`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs`
- `darkmatter/lib/src/markdown/compose/site_scope.rs`

No formatting, staging, commits, persistent-format changes, or platform-specific
code changes were made. Full area gates belong to the parent after integration.


### Catalog requirement-to-test map (before implementation)

- Signature/example parser boundary (F1/L1): `callback_example_arrows_preserve_nested_calls_patterns_and_quoted_text` loads catalog signatures through the public catalog loader, checks exact authored calls/results, and includes malformed callback/output cases. Exact regression: `example(filter([1, 2, 3], n => n > 1) => [2, 3])`, plus `->`, pattern commas, nested callbacks and quoted arrows.
- Shipped artifact/runtime boundary (F2/L1): existing `every_executable_example_evaluates_to_its_declared_result` executes all real embedded catalog examples through parse/evaluate. New examples cover both arrows, mixed tuple-shaped arrays, ordered object entries/destructuring, null/empty, nested callbacks, null results, truthiness, and scalar rejection. Existing registry parity and lifecycle exclusion tests prove wiring.
- Generated documentation boundary (F2/L1): regenerate the canonical table and verify `narrative_doc_function_table_matches_catalog`.

No file loader or field validator changes: this is expression/schema text grammar, so the configuration robustness matrix does not apply. No persistence is added.

### Catalog implementation checkpoint

The first regression attempt exposed a fixture mistake (`parameters()` is forbidden; a zero-argument signature uses `parameters(void)`). After correcting the fixture, the exact filter example failed because the example grammar only accepted value-only `FunctionCall` nodes. Extending that existing check to `CallbackCall` fixed it; the balanced/quoted separator scanner already handles nested callback arrows and needed no replacement. The regression then passed alone (1 test, 0.028 s).

The shipped catalog now includes 18 additional executable examples. No second function catalog or lifecycle list is introduced; existing callback-exclusion and registry parity tests remain the authority. Literal mixed arrays show tuple-shaped values; schema-declared tuple coverage belongs to evaluator/type tests.

The first documentation-generator invocation collided with in-progress evaluator edits and failed compilation (missing iteration-budget forwarding and an incomplete CallbackCall match). Rerun after runtime integration; these are not catalog defects.

A second generator compile attempt found an in-progress error-renderer call (`interpolation_block(term, ...)` supplied an undeclared extra argument). The runtime owner was notified. No catalog artifact or example parser failed in that attempt.

### Catalog completion evidence

- `cargo run -p darkmatter --example expression_doc_generator -- --write` passed after runtime integration. This updates only the generated table region. Its existing `narrative_doc_function_table_matches_catalog` test subsequently read the generated artifact and passed.
- `cargo nextest run -p darkmatter --lib --test l1 -E 'test(markdown::compose::expression::catalog::tests) | test(shipped_collection_callback_signatures_are_not_lifecycle_verbs) | test(the_generated_verbs_are_value_accepting_catalog_functions_and_side_effects_but_set) | test(function_binding_registry)' --color=never` passed: **26 tests**, 0.147 s execution, 10,163 filtered out. This includes all executable examples, malformed example cases, generated table matching, callback signature/lifecycle exclusion, and generated-verb parity. All new tests are L1 in existing compiled targets. The registry filter matched no separate ledger test; the parent should include the registry's exact ledger test in its area gate.
- The generator temporarily reported a dead-code warning for `DeclaredTypes::variable_subject`; runtime/editor integration owns the method's consumption. No formatting, staging, or commits were performed.

Catalog checklist marked complete immediately after the integrated scoped tests passed. Changed files in this slice: `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`, `darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs`, `darkmatter/schemas/partials/functions.yaml`, generated table in `darkmatter/docs/topics/darkmatter-expressions.md`, and the one catalog checkbox in the plan. No new docs or skills files.


### Editor requirement-to-test map (before implementation)

- F1/L1 cursor typing: overlay tests cover nested parameter reuse, outer restoration, iterable arguments outside bindings, explicit local/fm roots, both arrows, declaration hover, and incomplete-body completion.
- F2/L1 protocol: collection_editor's normal in-process LSP session covers body/frontmatter completion and hover, projected callback diagnostics, semantic binding/arrow ranges, escaped scalar and multiline inputs.
- F2/L1 passivity: existing recorded-handler tripwire gains inline map/filter bodies and empty input; all providers must retain zero handler entries and zero network/effect construction.
- No file-format loader or persisted value changes; robustness matrix and persistence round trip do not apply.

### Editor integration checks

The first focused `test(callback)` run executed 12 tests: 10 passed, including
escaped and multiline frontmatter diagnostic projections and the provider
passivity tripwire. Two new hover assertions expected broad `number`/`string`
types, but the shared projector correctly retained literal types. The assertions
are corrected to preserve that precision, then each failing test is rerun alone
before the combined slice. Earlier compile attempts stopped on in-progress runtime
integration errors outside the editor files; those were reported and corrected
by the runtime owner. No full-area gate or formatting command was run.

### Editor completion evidence

The final scoped command passed all **12 tests**, 1,562 excluded by the explicit
filter, in 0.349 s:

`cargo nextest run -p dmls --features effects-instrumentation -E 'test(callback)' --color=never`

New exact tests:
- `overlay::expressions::tests::callback_cursor_scopes_keep_nested_ownership_and_restore_outer_bindings`
- `overlay::expressions::tests::callback_completion_recovers_incomplete_bodies_and_prioritizes_locals`
- `collection_editor::callback_expression_protocol_scopes_hover_completion_and_tokens`
- `collection_editor::callback_frontmatter_diagnostic_projects_escaped_scalar`
- `collection_editor::callback_frontmatter_diagnostic_projects_multiline_scalar`

The existing `providers::collections::tests::collection_analysis_runs_no_callback_and_no_effect`
now exercises populated `map` and null `filter` expressions across all providers;
its recorded handler, effect-engine, network, and shell sentinel assertions pass.
Both initially failing tests passed alone before the combined rerun. No unresolved
failures, ignored tests, tier changes, persistence, or platform-specific code were
introduced. `tests/l1/main.rs` already declares `mod collection_editor`; unit tests
compile through the normal library target. All new tests are under 0.25 s.

Completion uses LSP `sortText` only when lexical candidates exist, preserving the
nearest-binding order even when clients normally sort labels. Hover retains the
shared projector's literal types, rather than widening them to primitive types.
Callback tokens use the existing frozen variable/operator legend; both arrow
spellings and every object-pattern field retain parser-authored byte ranges.
No product decision departed from the plan. Parent owns full-area gates and
canonical documentation updates.

Editor source files changed in Phase 3:
- darkmatter/dmls/src/overlay/expressions.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/providers/semantic_tokens.rs
- darkmatter/dmls/src/providers/collections.rs
- darkmatter/dmls/tests/l1/collection_editor.rs

Plan checklist updated after focused verification. No frontmatter key changed.

`cargo clippy -p dmls --lib --color=never -- -D warnings` passed in 2.62 s
after manually collapsing one nested conditional it flagged. This was a
control-flow spelling correction with unchanged behavior; the phase's full
area gate covers the final source. The scoped test run above remains the
behavioral verification. No editor command remains running.


### Runtime verification and documentation checkpoint

The integrated focused command `cargo nextest run -p darkmatter -E 'test(callback_runtime) | test(iteration) | test(collection_blocks::budget) | test(an_exactly_full_visit_allowance)' --color=never` passed 38 tests. It covers exact/exceeded budgets, standalone session sharing, mixed expression/directive visits, nested child and fallback fatality, existing byte and cache charging, null and scalar cases, nested pattern scopes and source-order fakes, typed error paths, and controlled file reads. New tests run below 0.1 s.

Additional targeted tests pin recoverable call-binding severity through callback wrappers and schema-declared tuple values with native and quoted YAML numbers. The tuple fixture initially used an unsupported trailing `(required)` after `tuple(...)`; removing that fixture-only syntax fixed the single test, which passed alone (0.060 s). The call-binding severity test passed (0.053 s).

All Phase 3 checklist items are complete after scoped verification. Canonical expressions documentation now describes execution, limits, errors, and editor behavior; the README links to it. The darkmatter expressions skill records runtime dispatch, shared counters, typed wrappers, and cursor scopes. No new documentation file, dependency, persisted format, or OS-specific implementation was introduced.

### Exact new runtime regression inventory

All compile in the existing `l1` target and run in L1:

- `function_call_binding::callback_runtime_preserves_order_nulls_patterns_and_nested_scopes`
- `function_call_binding::callback_runtime_binds_scalars_and_preserves_nested_failure_causes`
- `function_call_binding::callback_runtime_evaluates_source_once_and_stops_at_first_error`
- `function_call_binding::callback_runtime_session_budget_is_shared_and_arrays_do_not_charge_bytes`
- `function_call_binding::callback_runtime_call_binding_severity_survives_the_wrapper`
- `collection_execution::callback_runtime_composition_shares_expression_and_directive_visits`
- `collection_execution::callback_runtime_real_file_reads_use_the_enclosing_document_context`
- `collection_execution::callback_runtime_composition_preserves_free_capture_absence_observations`
- `collection_execution::callback_runtime_child_exhaustion_cannot_be_hidden_by_a_fallback`
- `collection_execution::callback_runtime_nested_failure_reports_every_original_position`
- `collection_execution::callback_runtime_schema_tuples_and_yaml_representations_keep_their_values`

The final required scope command (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`) selected `claudine`, `darkmatter`, `darkmatter-cli`, `dmls`, and `test-toolkit`, with `change_class: package`. Both full area gates are required and are being run. The selection includes prior Phase 2 uncommitted files; the Phase 3 metadata lists only this phase's 26 source/artifact paths. No cross-OS run is needed for this phase's platform-independent evaluator and passive editor changes; existing context/path APIs remain authoritative.

### Downstream verification scope

Sniff's complete package dependency inventory (`sniff repo package-dependencies --json`) identifies the direct consumers. Claudine's large library and CLI suites receive focused matcher, prewarm, schema, host-binding, and composition tests that use the shared expression evaluator and binding model. Messenger's optional research corpus and validation binaries exercise schema/composition loading. Small dependent CLIs receive their complete L1 runs: biscuit-icon-cli, biscuit-speaks-cli, messenger-cli, and playa-cli; claudine-contract's small contract suite also runs. Every package runs serially after the Darkmatter gate.

The remaining large direct consumers were inspected: biscuit-terminal-cli and sniff-cli render Markdown; research renders links; claudine-gen loads documents/schemas; darkmatter-fuzz is a fuzz target. They have no tests exercising the changed evaluator or callback editor APIs, so broad unrelated suites are not run. Darkmatter CLI and DMLS are covered by the full area gate. No workspace-wide gate or cross-OS execution is required.

### Full area gates

- `cd darkmatter && just lint` — passed for the library, CLI, DMLS, zed-dmls-cli, and Zed WASM extension (`wasm32-wasip2`).
- `cd darkmatter && just test` — **13,579 passed**, 10 skipped, 89.332 s execution. No failed tests or rerun of the full gate was needed.
- The 10 exclusions are the pre-existing ignored measurements, shell-alias, and scalar-alias tests listed individually under Phase 2 above; this phase added no ignore, tier marker, timeout override, or exclusion.
- All new tests are in existing declared targets and live L1 modules. New runtime, catalog, and editor tests remain well below the 5 s L1 budget.
- `git diff --check` — passed. No formatting, staging, commits, or lifecycle moves were performed.

`just check-tier-coverage darkmatter` passed: one area listed, zero stranded tests. All three delegated slices finished before the full area gates; no subagent remains active.

### Serial downstream results

- `cargo nextest run -p claudine --features work-counters -E '(test(claudine_schemas) | test(collection_host_compat) | test(type_scope_ownership_guard) | test(dispatch::matcher) | test(composition::lifecycle::prewarm::classify) | test(host_catalog)) & !test(/(^|::)(level2_|level3_|browser_|real_|slow_)/)' --color=never` — exit 0; Summary [  48.485s] 901 tests run: 901 passed (1 slow), 5564 skipped.
- `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(compose_presence_parity) | test(compose_schema_cli::compose_schema_grammar_error_reports_invalid_schema_without_launching_provider)' --color=never` — exit 0; Summary [   1.580s] 9 tests run: 9 passed, 4924 skipped.
- `cargo nextest run -p messenger --features research -E 'binary(research_corpus) | binary(research_validation)' --color=never` — exit 0; Summary [   3.897s] 64 tests run: 64 passed, 0 skipped.
- `just _test biscuit-icon-cli --features image` — exit 0; Summary [   0.439s] 99 tests run: 99 passed, 0 skipped.
- `just _test biscuit-speaks-cli` — exit 0; Summary [   1.048s] 107 tests run: 107 passed, 1 skipped.
- `just _test claudine-contract` — exit 0; Summary [   0.082s] 48 tests run: 48 passed, 0 skipped.
- `just _test messenger-cli` — exit 0; Summary [   3.473s] 133 tests run: 133 passed, 0 skipped.
- `just _test playa-cli` — exit 0; Summary [   0.268s] 25 tests run: 25 passed, 0 skipped.

All **1,386 selected downstream tests passed**. Filtered-out counts reflect the named focused selections, not new exclusions. Biscuit Speaks CLI's one skip is its pre-existing `real_cli_speaks_with_kokoro_provider`, outside L1. Claudine's existing `claudine_schemas::every_file_loads_and_every_export_resolves` passed in 6.491 s under the suite load; an isolated rerun follows to check its timing without competing tests. No downstream source changed in this phase.

### Closing verification

The isolated downstream command `cargo nextest run -p claudine --features work-counters -E 'test(claudine_schemas::every_file_loads_and_every_export_resolves)' --color=never` passed its one test in **3.126 s** (3.140 s total execution). Its suite timing is recorded as load-sensitive; no source change, exclusion, timeout override, or broad rerun was necessary.

Phase 3 is **implementation complete, ready for review**. Its 19 new tests, 18 executable catalog examples, full area gates (13,579 passing tests and lint), zero-stranded tier audit, and 1,386 selected downstream tests all passed; the isolated timing rerun also passed. There are no unresolved selected test or lint failures. Existing skipped tests are identified above and under Phase 2. The request-to-test mapping, exact commands, 26-file source inventory, documentation/skill inventory, and next-phase handoff are retained in this log and its frontmatter. No product ruling departed from the plan, so no new review decision was needed.

All delegated work and commands finished. All Phase 3 checkboxes are checked; Phase 4/5 work remains scheduled in the plan. No formatting, staging, commits, new worktree, branch change, or spec lifecycle move was performed. The spec required no frontmatter update in this phase. Frontmatter readback and `git diff --check` pass.
