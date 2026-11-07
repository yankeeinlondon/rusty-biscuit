---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
kind: feature
created: 2026-11-07
status: draft-spec
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
clarified: false
implemented: false
review_iterations: 0
packages:
    - darkmatter
    - darkmatter-lsp
related:
    - 2026-11-04-loading-data
---

# Expression Recovery

Add `try(expr, fallback)`: evaluate `expr` and return its value, or evaluate
`fallback` when `expr` encounters a recoverable read failure. This gives
optional data sources one consistent recovery mechanism without adding a
separate `try_` function for each reader.

This remains a draft. The inline review on 2026-11-07 settles the syntax,
evaluation rules, initial error boundary, and reporting below. The breadth
of recovery for other function failures remains in [Open questions](#open-questions).

## Motivation and example

2026-11-04-loading-data deliberately left general recovery to a separate
feature. An existence check handles an absent file, but cannot handle a
present file that contains malformed data or violates its expected shape.

```markdown
{{ as_json(try(load_data("optional.yaml", "string[]"), [])) }}
```

This produces the loaded array when valid and `[]` when the file is missing,
unreadable, malformed, or contains values that cannot satisfy `string[]`.
The explicit `as_json` makes the array's displayed representation clear;
`try` itself returns a value, not rendered Markdown.

## Current contracts

The following are the relevant implementation boundaries as of 2026-11-07.
Links name the owning package so implementers can find the shared behavior.

- In the **darkmatter** library, the evaluator in
  [expression/mod.rs](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs)
  propagates a failed left operand of `||`. That operator chooses a fallback
  only for a successfully evaluated falsy value. Ordinary function arguments
  are evaluated before their handler runs, but `and` and `or` already use
  lazy argument evaluation; callbacks also defer their bodies.
- The **darkmatter** library's
  [ExpressionError](../../../darkmatter/lib/src/markdown/compose/expression/error.rs)
  preserves structured causes and callback wrappers. Full-document
  interpolation failures stop composition regardless of `fail_fast`.
  Some individual functions already provide special recovery, including
  `try_frontmatter` and numeric conversion fallbacks. There is no general
  expression-level error boundary today.
- The **darkmatter** library's
  [DataError](../../../darkmatter/lib/src/markdown/compose/expression/data/error.rs)
  distinguishes bad data from an invalid contract. All currently stop an
  unguarded `load_data` call, including during the reference check that
  `md compose` runs before commands. Remote reads in that check are deferred
  because that pass has no network permission; they are not failed fetches.
- The **darkmatter** library's
  [function registration](../../../darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs)
  already distinguishes eager, lazy, and callback execution. Catalog
  `fallible` metadata alone does not classify a failure as recoverable.
- The **simplified-schema** library owns the shared expression
  [syntax tree](../../../simplified-schema/lib/src/expression/ast.rs) and parser;
  Darkmatter owns evaluation, call binding, and result-type inference.
  DMLS (the **darkmatter-lsp** package) consumes the passive analysis without
  executing expressions or reading expression inputs.

## Scope and decisions

### Syntax and evaluation

Use a function-shaped special form, `try(expr, fallback)`, with exactly two
value-expression arguments and no callback syntax or implicit error variable.
Register it through the existing catalog and function lookup rules. Keep the
existing function-call syntax tree; no new operator, precedence rule, or
reserved lexer keyword is needed. Extend the existing lazy evaluation path
so the evaluator, rather than an ordinary eager handler, owns recovery.
Direct value-only call APIs cannot reconstruct lazy expressions: they must
reject such an invocation as a call contract error, as appropriate for their
existing lazy-function API, rather than claim to have caught an earlier error.

Runtime behavior is:

1. Evaluate `expr` once in the current expression scope and request context.
2. On success, return its value unchanged. `null`, `false`, `0`, `""`, `[]`,
   and `{}` are successes; none selects the fallback.
3. On a recoverable failure, evaluate `fallback` once in the same lexical
   scope. Return its value or propagate its error. This invocation of `try`
   does not catch a failure from its own fallback; an enclosing `try` may.
4. On any other failure, propagate it immediately without evaluating the
   fallback.

Both arguments remain visible to parsing, the passive call check, editor
validation, static variable-read inventory, and context-requirement discovery.
A provably invalid call in an unexecuted fallback still rejects the document
before evaluation. These inspections perform no expression I/O. Runtime reads
and effects, by contrast, occur only in the selected arguments.

Recovery does not retry or undo work: reads, observations, earlier callback
visits, and spent iteration allowances remain. For example,
`try(map(items, item => load_data(item, "string[]")), [])` replaces the entire
map result when an item fails; `map(items, item => try(load_data(item,
"string[]"), []))` replaces only that item's value and continues. The same
iteration budget is used by the protected expression and the fallback.
Inserted text remains data and is never rescanned as authored expressions.

### Which errors can be recovered

Introduce one typed recovery classifier in the **darkmatter** library beside
[ExpressionError](../../../darkmatter/lib/src/markdown/compose/expression/error.rs),
used by every evaluation surface. A callback wrapper delegates classification
to its cause while preserving the wrapper for any propagated diagnostic.
Unknown or newly introduced error variants default to nonrecoverable.

The initial boundary is deliberately explicit:

| Failure | Behavior inside `try` |
| --- | --- |
| A well-formed data-source reference names a missing file, or an ordinary local read fails | Recover |
| An allowed remote read fails during transport, including an unusable UTF-8 response | Recover |
| `dm.data.unsupported_format`, `dm.data.malformed`, or `dm.data.contract_mismatch` | Recover |
| A malformed file reference, invalid `as` type, missing or invalid imported contract, `dm.data.not_a_container`, `dm.data.row_shape_conflict`, or `dm.data.this_import_rejected` | Propagate: the requested operation is invalid |
| Parse errors, unknown functions, invalid callbacks, passive call-check rejections, or runtime argument/return binding failures | Propagate |
| Policy refusal, including denied remote hosts and warnings made blocking with `--deny` | Propagate |
| A read unavailable only because the current discovery/reference pass cannot perform it | Defer under that pass's existing rules; do not select the fallback |
| Iteration or expansion limits, nested-composition depth limits, missing request context, unavailable host bindings, or internal contract failures | Propagate |
| Arithmetic failures, provider failures, generic unclassified handler failures, or nested composition failures | Propagate in the initial design; see Open questions for broader function coverage |

A data-source I/O failure is recoverable; a failure loading its type contract
is not. Preserve that distinction as structured provenance through memoized
errors. Do not infer it from a function name or rendered message. Similarly,
the existing file-failure label `NotFound` also covers context and tree-root
errors: inspect the retained typed cause rather than treating that label as
proof of a missing file. Path-boundary refusals remain nonrecoverable.

**Reader's note.** The original proposal caught every fallible function's
runtime error, including all `dm.data.*` failures. That would hide bad
contracts as though the input data were merely unavailable. Some remote
failures currently share a generic error variant with programming errors.
Implementation must preserve a typed transport cause through the shared
remote-read adapter before allowing transport recovery; catching that generic
variant is unsafe. Preserve existing diagnostic codes, wording, source
locations, and reference fields when projecting these newly typed causes.
This is an intentional, local extension to the current rule that data failures
always stop composition: only an explicit `try` absorbs the listed failures.
Unguarded failures and all existing fatality checks keep their contracts.

### Surfaces, reference checks, and effects

Use the same evaluator behavior in body and frontmatter interpolation,
conditional page blocks, transclusion `when=` conditions and target
expressions, inline callbacks, collection-region expressions, shell-template
expressions, and lifecycle expressions. Lifecycle shell failures themselves
are outside the expression boundary. Do not enable an expression feature on
a surface where that feature is currently disabled.

The reference check and shell-approval discovery must evaluate the whole
`try` expression under their existing execution restrictions. A genuine
recoverable read failure chooses the fallback and does not stop that pass.
A deferred value does not: retain the expression for a later permitted pass,
and conservatively inspect both arguments for static dependencies and approval
requirements. Never freeze a discovery-time fallback as the final value or
fetch a URL simply to decide whether a branch will run.

A separate authored reference elsewhere in the document still receives its
normal validation. `try` does not exempt a file globally from reference
checking. It also cannot clear a pending blocking diagnostic from nested
composition, approve a command, or loosen the request's fetch policy.

### Reporting, read set, and memoized results

A recovered cause is handled: emit no error or advisory warning for that
cause, and no new recovery diagnostic code. Accordingly, the compose data
envelope contains no diagnostic for the absorbed failure. Existing warnings
produced before the failure remain visible and obey `--deny`; `try` is not
an absence guard and does not suppress undeclared-property warnings.
If the fallback also fails, report its normal structured error at its
proven authored location, retaining callback provenance. Do not attach an
invented location or emit the handled cause as a second error.

The read set follows its existing rules, including failure paths. Keep a
resolved unreadable/malformed source and a resolved denied URL in it; a
missing reference with no resolved identity has no path to record. Do not
invent a canonical path for an absent file. Reads attempted by the fallback
are recorded too; an unexecuted fallback adds no runtime reads.

Cache the source result or failure under the existing request memo keys,
never the fallback value in place of that result. A later unguarded call
still fails, and another `try` can supply a different fallback. Existing
memo invalidation after executed commands and nonempty lifecycle events
still applies. A read deferred for lack of permission in the current pass
is not memoized as a failure. Policy checks still precede memo/cache use.

### Static types and editor behavior

In the **darkmatter** library,
[expression_type](../../../darkmatter/lib/src/markdown/compose/expression/call_check.rs)
computes the union of the protected expression's successful type and the
fallback's type through the shared shape-union operation. Successful `null`
remains in the union. Neither branch gains truthiness or presence facts from
a failure, and `try` declares no predicate narrowing rule. Computed or
unresolved contracts retain their existing unknown-type behavior.

**Reader's note.** An empty array literal currently infers an array with an
unknown item type. Simply joining it with a typed array would lose the item
information promised by the original acceptance criterion. For this special
form, an authored empty-array argument adds no alternative when the other
branch is proven to admit the empty array. Thus
`try(load_data("optional.yaml", "{ a: string }[]"), [])` retains that array's
item type for callback completion. Apply this only when the other type
actually accepts `[]`; do not drop it for a nonempty-constrained array,
a fixed-length tuple, or an object. Otherwise preserve the ordinary union.
This does not change empty-array inference throughout the language.

DMLS completion, hover, arity diagnostics, callback item typing, and unknown
function checking must recognize `try` through the shared catalog and analysis.
No editor feature executes either argument. Host function registration must
apply the existing built-in collision checks to the new name.

## Non-goals

- Changing `||`, ternaries, numeric conversion fallbacks, or `try_frontmatter`.
- Error objects, callback fallbacks, implicit error bindings, retries, or
  transactional rollback.
- Catching parse errors, programming errors, policy refusals, or resource limits.
- Adding a new CLI recovery switch: the authored expression is the opt-in.
- Expanding remote permissions, reference resolution rules, or execution
  privileges during passive analysis and preflight discovery.

## Open questions

### How broad should recovery be beyond structured read failures?

The catalog contains fallible functions whose generic errors do not reliably
distinguish an unavailable input from misuse or a broken implementation.
Provider calls also have an explicit fatal-failure contract today. The author
should decide whether this feature must extend to those functions immediately.

1. **Keep the typed read-failure boundary above (recommended).** Pros: meets
   the optional-data goal, has a precise safety boundary, and leaves existing
   provider and programming-error contracts intact. Cons: `try` cannot yet
   absorb every operational failure from every fallible function. This is the
   recommendation because broad recovery without trustworthy causes can make
   a broken request appear successful.
2. **Classify all operational function failures in this feature.** Pros:
   gives a broader, consistent recovery API immediately. Cons: requires an
   inventory and typed-error migration of built-ins and host functions,
   explicit changes to provider fatality contracts, and more compatibility
   tests. Policy, binding, and resource failures must remain excluded.
3. **Catch generic errors from every function marked `fallible`.** Pros:
   smaller initial implementation and broad apparent coverage. Cons: the
   marker does not distinguish operational failure from misuse; it can hide
   programming and policy errors unless all producers are audited first.
   Not recommended.

Implementation may use the recommended bounded design; it must not silently
widen recovery to generic or provider failures. Any author decision to broaden
it must update the error table and affected contracts before implementation.

## Implementation phases

### Phase 1: Establish the recovery boundary

Add the typed classifier and preserve transport and contract-import provenance
through the shared read adapters and memo. Add the catalog registration and
lazy evaluator behavior. Keep existing source spans and error projection.

### Phase 2: Apply the contract across consumers

Make reference/preflight deferral, static result types, and DMLS behavior
agree with the runtime contract. Check lifecycle consumers, including the
Claudine host, for assumptions about lazy calls and fatal error propagation.
A parser change in simplified-schema is not required by this design.

### Phase 3: Verify and document

Add focused unit and command-path coverage below. Update the Darkmatter
expression and loading-data topics, call-check and composition-problem pages,
and generated function documentation. Update the Darkmatter skill's expression
rules so it explains explicit recovery alongside unguarded fatal failures.
The README needs only a brief example or overview link if its behavior summary
would otherwise become misleading. No dependency changes are planned.

## Acceptance criteria and validation

Use local temporary fixtures and mock remote responses; tests require no live
network, terminal windows, or browser focus. Run the package-area `just test`
and `just lint` recipes for the affected Rust packages during implementation.
This inline spec review itself does not change executable code.

- Through `md compose`'s normal reference-check and composition path, the
  example above produces the data for a valid file and `[]` for missing,
  unreadable, malformed, and contract-mismatching data. An unsupported data
  extension is recoverable. A missing or invalid contract import is not.
- An instrumented fallback is not evaluated after success, including each
  falsy value. Each selected operand executes once per actual evaluation;
  multiple pipeline passes still share the existing data memo.
- A failed fallback propagates; an enclosing `try` can catch it only if its
  cause is recoverable. Callback tests distinguish whole-map recovery from
  per-item recovery and preserve original item positions on propagated errors.
- Parse errors, a bad call in either argument (including an unreachable
  fallback), runtime binding failures, arithmetic errors, and iteration limits
  stop the document. The fallback cannot reset an exhausted budget.
- A denied host issues no request and remains fatal even with a cached source.
  An allowed mocked transport failure recovers. A reference-check deferral
  later loads valid remote data instead of permanently selecting `[]`.
- A pending nested-composition blocking diagnostic, missing request context,
  malformed reference, or path-boundary refusal remains fatal under `try`.
- Every expression surface listed above has recovery coverage through its
  normal entry point; passive DMLS analysis performs no input reads. Existing
  host catalog loading still rejects collisions with built-ins.
- A recovered failure emits no diagnostic for that cause in formatted output
  or the data envelope. Unrelated warnings remain, and `--deny` still stops
  the command. Propagated errors retain existing codes and structured locations.
- A resolved failed source stays in `read_set`; an unresolved missing source
  does not gain a fabricated entry. A second unguarded call fails from the
  memo, different fallbacks remain distinct, and an executed command that
  regenerates the file invalidates the old failure under the existing rules.
- Static typing preserves the `{ a: string }[]` item shape with `[]` as the
  fallback, including DMLS member completion in a callback. Tests also cover
  nullable results, a different fallback type, unknown contracts, and an array
  constraint that forbids emptiness so the empty-array exception cannot erase
  a real alternative.
- A catalog corpus check and a normal `md compose` invocation cover the new
  form. Generated function pages and the topic pages state the same lazy
  semantics and recoverable-error list.

No performance spike is required: this adds a branch and typed error
classification around an existing evaluation. The success path must not read
or evaluate the fallback, clone the entire request state, or clear the memo.
