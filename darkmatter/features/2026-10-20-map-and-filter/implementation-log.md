---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/features/2026-10-20-map-and-filter/spec.md"
plan: "darkmatter/features/2026-10-20-map-and-filter/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: true
human_review_items:
    - |-
        **The feature this one is built on has not been implemented yet.**

        Map and filter need a shared way to describe and check function signatures (what arguments a function takes, and their types). That shared system is the job of a separate feature, `2026-10-28-schema-enhancements`. Today it is only a finished spec: none of its 22 plan tasks are done, and the file this feature is supposed to add `map` and `filter` to (`darkmatter/schemas/partials/functions.yaml`) does not exist. The plan forbids building a stand-in. So Phase 2 cannot be finished as written, and someone has to decide what happens next before Phase 2 starts.

        - **Option A: pause this feature and implement schema-enhancements first.** *Pros:* matches the plan exactly, with no temporary code and no second function catalog. *Cons:* schema-enhancements is a large, 6-phase piece of work, so map and filter are delayed by all of it.
        - **Option B: split Phase 2 and continue now with what does not depend on it.** The new `x => ...` syntax, the rules for which names are visible inside a callback, and the editor plumbing do not need the shared signature format. Only signature checking (Phase 2) and adding catalog entries (Phase 3) wait for schema-enhancements. *Pros:* the parser and scope work, which is the riskiest part, starts now and has its own tests. *Cons:* until the catalog work lands, `map`/`filter` cannot be published or checked for wrong argument counts or types, and the plan's phase order has to be redrawn.
        - **Option C: add map and filter to today's catalog (`darkmatter/docs/schemas/expression-functions.yaml`) and move them across later.** *Pros:* fastest route to a working feature. *Cons:* the spec explicitly rules out a temporary catalog form and a parallel checking path, and this one would have to be migrated again.

        **Recommendation: Option B.** It keeps to the spec (no temporary format, no second catalog) while getting the parts with the most design risk built and tested now. Option A is the right choice if schema-enhancements is already next in the queue anyway.
    - |-
        **Please confirm three policy recommendations the spec left open.** Phase 2 builds the warnings and errors, so they need settling before it starts. The details and reasons are in the implementation log under "Rulings and dispositions".

        1. **Warn when a callback's name hides a document property** (for example, `filter(tags, tags => ...)` when the document has a `tags` field). *Recommended:* yes, but only when the hidden property is actually declared in that document. Writing `fm.tags` still reads the document and gets no warning. Reusing an outer callback's name gets no warning either. *Alternative:* no warning at all, which is simpler but lets accidental shadowing go unnoticed.
        2. **Names for the new errors and warnings.** *Recommended:* `dm.expression.misplaced_callback` (a callback written where one is not allowed), `dm.expression.reserved_callback_parameter` (a callback named `ctx`, `fm`, `null`, or a host name such as `seq`), and `dm.expression.callback_shadows_property` (the warning above). An error inside a callback keeps the error code it already had, and its message gains `map(): item 2: ...`. *Alternative:* give wrapped errors a new code of their own. That is not recommended, because existing code that sorts errors by code (file-not-found, fatal versus warning, Claudine's error display) would stop recognizing them.
        3. **Limits on running callbacks.** *Recommended:* run items one at a time, in order, under the same permissions, timeouts, and caches as the surrounding expression. Add no new size or nesting limit, and do not undo side effects from items that ran before a failing item. *Alternative:* add a maximum list size or a time budget. That needs a number to be chosen, and the spec gives no basis for one.
message_to_agent: |-
    Phase 1 produced design contracts only; no code was written. Read the implementation log's "Phase 1" section before you start. It covers the callback AST (`CallArgument::{Value, Callback}`, not an `Expr::Lambda`), the explicit stack-allocated `Locals` environment threaded beside `AbsenceScope` (not a forwarding `EvaluationLookup` wrapper), `EvaluationMode::Callback` registry dispatch, the `ExpressionError::Callback` wrapper with `root_cause()`, and the table of every AST consumer that must become binder-aware.

    Do not start Phase 2 until the human review items are answered. The prerequisite `2026-10-28-schema-enhancements` is unimplemented, so the "Check callback signatures" task cannot be done as written. The parser and passive-scope tasks can proceed only if the author picks the split option. Watch for these traps: the lexer folds `dep.name` into one Variable token, so match locals on the root segment; caller-file provenance is keyed by frontmatter pointer (`caller_file_occurrence`); branch facts about a same-named document property must be dropped inside a callback body; DMLS uses `BindingView::baseline()`, so it cannot see Claudine host globals; `null` is a variable root and must be a forbidden parameter name.

    Phase 1 gates: `affected_scope.py` reported non-empty packages because of uncommitted work this feature did not make (the strict-equality docs and a Claudine schema test), so `just test` and `just lint` were run in darkmatter. See the log for results.
---

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
