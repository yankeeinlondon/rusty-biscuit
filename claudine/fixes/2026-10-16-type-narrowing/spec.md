---
kind: fix
created: 2026-10-16
status: draft-spec
clarified: true
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-20
review_iterations: 0
implemented: false
completed: false
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
area: darkmatter
human_review: false
depends-on:
    - 2026-10-23-shell-template-approval
related:
    - 2026-10-25-loops-and-flow-control
    - 2026-10-26-task-inputs-and-providers
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-28-schema-enhancements
    - 2026-10-31-type-aware-evaluation
---

# One type scope: guard narrowing applied everywhere

A guard such as `when: "ctx.dirty_files && commit_message"` proves that
`commit_message` is present inside the branch it protects, but shell preflight
ignores it:

```yaml
success:
    - when: "ctx.dirty_files && commit_message"
      then:
          - shell: git commit -m "{{commit_message}}"
```

```text
lifecycle shell command at success[4].then[2].shell.command failed pre-flight
resolution: shell command reads commit_message, which is missing; a shell
command never runs with a missing value
```

When the guard selects this branch, `commit_message` is present, yet preflight
refuses the document before evaluating the guard.
The cause is not one missing check but five independent ones. Each consumer
decides for itself whether a value is present or what type it has, and the
shell check added by `2026-10-23-shell-template-approval` was written without
a guard rule. A sixth, the flow-sensitive narrowing of
`2026-10-31-type-aware-evaluation`, is planned.

This fix replaces them with **one type scope**: at every site where an
expression is checked, the type of each property is its declared type refined
by every guard that encloses the site. Every consumer asks the scope for a
type; presence is a consequence of that type, never a separate question. The
fix lands on the pipeline before `2026-10-27-darkmatter-lifecycle-ownership`,
so that spec moves one walk, already carrying narrowed types, into Darkmatter.

**Review note (2026-10-20).** This remains a draft; the edits below correct
static narrowing against the existing evaluator and approval contracts. They
preserve equality, runtime value checks, and saved approval syntax. In
particular, a guard supplies conditional evidence, not a replacement for
checking the value immediately before a command runs. The interaction between
`file_exists` facts and later argument conversion remains an Open Question.

## Today: five places decide presence or type

| Site | Decides by | Honors guards |
|---|---|---|
| `expression/absence.rs` `AbsenceScope` | recognizes intentional absence handling in fallbacks, ternaries, and absence predicates | within one expression only; warning policy is distinct from presence proof |
| `compose/directive_targets.rs` `narrowed_expression_paths` | `::block when=` around a `::file`/`::code`/`::url` target | body blocks only |
| `shell_expansion/template/candidate.rs` | `read.is_null()` → `MissingValue` | no |
| `claudine` `composition/preflight.rs` | `read.is_null()` → `MissingValue` | no |
| `claudine` `composition/sequence/preflight/mod.rs` | `read.is_null()` → `MissingValue` | no |

The type side is split the same way: `schemas/simplified/subtype.rs`
`is_subtype` serves shell templates only, and
`2026-10-28-schema-enhancements` defines its own argument-binding comparison.

## Decided

Decided 2026-10-16 by the owner.

### 1. The type scope

`TypeScope` is a proposed immutable, Darkmatter-owned scope mapping resolved
bindings and property paths to their narrowed type at one site. It starts from
the site's existing effective-schema projection and is extended on entering a
guarded region. Optional properties include `null` under the existing shared
schema presence rules, even when their underlying atom is `string`. An
undeclared property or unsupported projection is `unknown`, with no presence
guarantee. Known context-descriptor presence and existing concrete-value
transclusion rules remain available through this shared layer.

- **One static question.** Consumers ask `scope.type_of(path)`. A value is
  *statically proven present* when that type excludes `null`. This is conditional
  evidence at that site; runtime shell validation still rejects an actual null.
- **Shell fallback stays local.** The shell-template-only fallback to `string`
  describes scalar conversion for approval, not a declared type or proof of
  presence. Apply it after scope lookup. Preserve the original validator and
  distinguish declared types, guard refinements, and fallback conversion; an
  undeclared name must not become a declared string in function checks or DMLS.
- **One producer.** Guard facts come only from
  `TypeScope::assume(&Expr, Polarity)`, which absorbs
  `narrowed_expression_paths` and `AbsenceScope`'s branch rule. Runtime
  composition and DMLS call the same code.
- **Stable subjects.** Facts apply only to a resolved variable or a path with
  constant members or indices. Equivalent bare and `fm` paths share a fact
  only when both resolve to the same document binding. `is_number(length(x))`
  proves nothing about `x`; dynamic indices and repeated function calls are
  not stable subjects. Reserved roots (`ctx`, `env`, `seq`, `loop`, `group`)
  retain any existing descriptor information; fuller host types arrive with
  `2026-10-31-type-aware-evaluation`. Unknown host values receive only facts
  justified by the guard. Resolve all registered globals through the existing
  binding model, including `err`, `timing`, and `current` where available;
  narrowing cannot turn an unavailable global into an available one.
- **Static only.** Narrowing never rewrites a stored value, proves that a file
  exists, or grants any access.

```mermaid
flowchart LR
    D[declared type<br/>EffectiveSchema] --> S[TypeScope at site]
    G[enclosing guards<br/>block, lifecycle when, task when,<br/>ternary, && / ||] -->|assume| S
    S --> P[shell presence + template holes]
    S --> T[transclusion target nullability]
    S --> A[absence warnings]
    S --> F[function argument checks]
    S --> E[DMLS hover, completion, diagnostics]
```

### 2. Guards narrow both sides

Every guard narrows its true side and its false side. What each side learns
depends on whether the guard is **true-iff** (its result is exactly the test)
or not.

| Guard | True side | False side |
|---|---|---|
| truthiness `x` | `null` removed | nothing (`0`, `''`, `false`, and empty containers are falsy) |
| `!is_null(x)` (true-iff) | `null` removed | `x` is `null` |
| `x != null` | `null` removed | retain `null` and the empty string where admitted; do not infer null alone |
| `x == null` | empty-string possibilities only; never infer null | no general presence fact |
| `is_string(x)`, `is_number(x)`, `is_array(x)`, `is_object(x)`, `is_null(x)` (true-iff) | intersect with that runtime family | that family removed from `x`'s union |
| `file_exists(x)` (true-implies) | before argument conversion exists, intersect with present strings; after migration, use the conversion-safe fact agreed in Open Questions | nothing |
| `x == v` with a non-null scalar literal | retain all matching possibilities; see the equality rules below | remove matching literals only when the remainder is representable |
| discriminant `x.kind == 'a'` | retain arms whose tag may match | remove only arms whose tag must match |

- `!` swaps the sides; parentheses preserve facts.
- `a && b`: check the right operand under `a`'s true facts only; it must
  not assume its own result. The true branch combines those facts with `b`'s
  true facts. The false branch joins the `a`-false route and the
  `a`-true/`b`-false route, retaining only evidence valid on both.
- `a || b`: check the right operand under `a`'s false facts only. The true
  branch joins the `a`-true route and the `a`-false/`b`-true route; the false
  branch combines `a`'s false facts with `b`'s false facts.
- `c ? t : e` gives `t` the true facts and `e` the false facts.
- After a guarded region, restore the incoming facts only where no possible
  continuing branch invalidated them. This fix does not propagate new branch
  facts beyond the region or reason about control actions that cannot return.
- **Truthiness is limited evidence.** Removing `null` from `unknown` is an
  internal non-null fact, not a new schema type, and does not establish
  compatibility with a concrete parameter. The evaluator's truthiness and
  value-returning `||` are unchanged.
- **Predicates observe original values.** A registered fact names the
  parameter position and the runtime value family it tests, and must hold for
  the original argument before any conversion. Narrowing preserves compatible
  refinements and constraints; a family predicate does not invent item types,
  object members, or file existence. Match predicates through the canonical
  function registry, honoring aliases and the evaluator's case handling.
  `is_boolean` is not currently registered and is not introduced by this fix;
  any later boolean predicate must land with its handler and fact together.
- **Conservative exclusions.** A false-side removal applies to a finite
  declared union or enum. When the remainder cannot be represented (`unknown`
  excluding strings), `unknown` is kept. A true side can still establish its
  tested family from `unknown`.
- **Impossible branches remain checked.** A guard that contradicts the
  incoming type (`x: enum(a, b)` with `x == 'z'`, or `is_number(flag)` with
  `flag: boolean(required)`) keeps the incoming type for that subject in that
  branch. Contradiction never waives an independent shell presence check or
  runtime validator. There is no empty type that makes every call acceptable, so a call known to
  be invalid is still an error in code that will not run.

Example, with `target: string | number | null` and
`mode: enum(release, debug, test; required)`:

```md
::block when="is_string(target)"
::shell git checkout {{ target }}         <!-- target: string -->
::end-block
::block when="!is_string(target) && target"
::shell gh pr checkout {{ target }}       <!-- target: number -->
::end-block
::block when="mode != 'release'"
<!-- mode: enum(debug, test) -->
::end-block
```

### 3. Equality narrows to literals of every scalar type

`literal` already types strings, numbers, and booleans (bare `4` is a number,
`true` a boolean, quoted `'4'` a string). Equality narrowing therefore applies
to every scalar family, not to strings alone.

Darkmatter's equality compares the text produced by
[`scalar_string`](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs),
Darkmatter's shared value-to-text helper. Thus `4 == '4'` is true. Its
[evaluator](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs)
also makes **both** equality and inequality false for two nulls. Null becomes
empty text when compared with a non-null value, and arrays and objects become
JSON text. These existing semantics are preserved.

For a known scalar union, keep every admitted scalar whose actual comparison
result matches the branch. Preserve refinements and constraints when
intersecting; equality to `'x'` does not make `'x'` an `email`. Remove matching
literals on the other side only when the remainder is representable. Do not
infer a scalar family from equality against `unknown` or a union admitting
containers: `[] == '[]'` is also true. Retain unmodeled possibilities. Reverse
operand order uses the same rules.

| `x` declared | Guard | True side | False side |
|---|---|---|---|
| `string(required)` | `x == 'release'` | `literal('release')` | `string` |
| `enum(release, debug; required)` | `x == 'release'` | `literal('release')` | `literal('debug')` |
| `number(required)` | `x == 4` | `literal(4)` when uniqueness is established by the current formatter | `number` |
| `string \| number` | `x == '4'` | matching string and number literals, excluding null | original union, conservatively |
| `boolean(required)` | `x == true` | `literal(true)` | `literal(false)` |
| `string` (optional) | `x == ''` | `literal('') \| null` | `string` without a presence proof |
| `string` (optional) | `x != null` | non-null string | `literal('') \| null` |
| `string` (optional) | `x == null` | `literal('')` | original optional type |
| `enum(a, b; required)` | `x == 'z'` | incoming type (contradiction kept) | incoming type |

For number narrowing, use the evaluator's current formatting, not a new parser
that merely accepts the literal's spelling. If distinct numeric values can
format identically, retain the numeric family instead of claiming one literal.
Later numeric formatting changes must keep these evaluator-versus-analysis
cases consistent. No formatting fix is part of this specification.

A discriminant narrows a parent object only when the projected union's tag
members and their comparison outcomes establish which arms remain. Keep an
arm with an optional, unknown, or otherwise inconclusive tag. Until nested
union projection exists, retain the parent type; narrowing the accessed path
must not invent a parent shape or establish unrelated sibling presence.

### 4. One subtype relation

Darkmatter's
[`is_subtype`](../../../darkmatter/lib/src/markdown/schemas/simplified/subtype.rs),
currently a bounded comparison of underlying types that deliberately ignores
presence markers and ordinary constraints, becomes the shared structural
relation. Extend it without changing those existing callers' treatment of
markers. Scope-level comparisons explicitly include effective nullability;
passing an optional atom to the old relation is not a presence check.

Function binding uses this relation for structural exact fit, then the
existing constraint checks and planned conversion engine for the remaining
cases. A failed subtype comparison does not mean a conversion is impossible;
a successful one does not prove that runtime constraints hold. It grows to cover:

- `literal(v)` ⊆ its scalar family: a string literal ⊆ `string`,
  `literal(4)` ⊆ `number`, `literal(true)` ⊆ `boolean`;
- a string `literal(v)` ⊆ `enum(…)` that has the same string as a member;
  numeric `literal(4)` is not a structural subtype of string `enum('4')`,
  even though the evaluator compares them equal;
- `enum(X)` ⊆ `enum(Y)` when X ⊆ Y;
- unions on either side: compare each source arm against the destination
  union, then require at least one destination arm to cover it. Thus
  `string | number` is covered by `string | number | boolean`, independently
  of arm order. Unknown information must not count as proof of a concrete
  type, and excluding null from unknown does not make it a concrete subtype.

Its existing rules (alphanumeric options, always-text types ⊆ `string`) and its
bounded-reasoning contract are unchanged. A function parameter typed `string`
therefore accepts a narrowed `literal('release')` structurally with no
special case at the call site. Constraints still require runtime validation.

**Approval boundary.** Expanding the shared relation must not silently expand
the saved shell-rule grammar. Saved holes remain supported string-like atoms;
non-string narrowed types still use the existing broad `string` approval form
with their original validator. A narrowed string-like type may match an
existing narrower rule only when the live value is validated against that
refinement as well as its original declaration before dispatch. Preserve
argument boundaries, blacklist precedence, frozen approval behavior, and the
rule that missing or invalid inputs cannot be suppressed by `no_error` or
process-failure recovery.

### 5. Every consumer reads the scope

| Consumer | Reads from the scope |
|---|---|
| Shell missing-value check (`2026-10-23-shell-template-approval` §1) | approval-time null is an error unless a live enclosing guard proves presence (§6) |
| Shell template holes (same spec, §2) | the hole's type is the narrowed type at the site, not the declared type |
| Transclusion target nullability | replaces `narrowed_expression_paths` |
| Absence warnings | shared branch facts, while preserving intentional absence handling in fallback operands and predicate arguments |
| Function argument checks (`2026-10-28-schema-enhancements`, `2026-10-31-type-aware-evaluation`) | the argument's narrowed type, compared by §4's relation |
| DMLS hover, completion, diagnostics | the same scope through the same library |

Intentional absence handling remains a warning policy, not a proof of
presence. In `x || 'default'`, reading missing `x` is intentional although it
is nullable; `is_null(x)` intentionally examines absence as well. Preserve
those exemptions, aliases, warning identity, and runtime short-circuit
reachability while replacing the private branch-fact walk. An exemption for
`x.a` must not prove sibling `x.b` present, and a false ternary branch must
not inherit its condition's true-side fact. Static editor analysis can inspect
both branches without causing runtime warnings for unread branches.

### 6. A guarded missing value is checked when its branch runs

At preflight, an actual null still fails unless a live enclosing guard proves
the placeholder present at that site, or an existing late-binding contract
already defers it. A non-null declaration alone cannot excuse an actual null.
Presence of one property does not defer another property in the same command.
For example, `when: x` does not defer missing `y` in `tool {{ x }} {{ y }}`.

For guard-proven placeholders, retain the authored expression and require a
typed template when the approval-time value is missing. Never cache a resolved
empty string from discovery in place of that expression. Extend the existing
late-value mechanism with a **site- and expression-specific reason**; do not
add a document key to `DeferredShellRoots` globally, which would incorrectly
exempt an unguarded use elsewhere. Invalid template grammar or a variable
program word still fails under the existing template-required rules. No
approval mode may bypass runtime validation.

Guard-dependent refinements used for approval must also be validated at run
time, even when the discovery value was present. Once the guard actually
selects the action, resolve each hole once from live state, reject null, check
the original schema and approved refinement, and use the same checked value
for dispatch. A changed type is a template-value refusal; null remains
`MissingValue`. This prevents a guard followed by `set` from authorizing a
value of a different type under an earlier narrow approval.

**Conditional validation boundary.** Guard-deferred holes are not swept into
an unconditional task-start late-value barrier: an unselected branch must not
fail because its value is absent. Validate them at the reached action before
its first command-chain segment runs. Ordinary unguarded sequence late values
keep their existing task-start barrier. A standalone frontmatter `$( … )`
gets no lifecycle or body guard merely because a later branch reads the key:
only a whole-value shell authored inside that guarded action belongs to it.

Discovery remains condition-blind: collect and approve commands from both
branches before launch, without evaluating guards, executing predicates, or
running mutations to discover facts. Parse and unavailable-binding errors
remain their existing typed failures. An actual false guard skips its action;
a missing value after a selected guard fails that action before launching any
of its command segments. Effects of earlier actions remain.

**Reader's note:** Static presence and live value validity answer different
questions. The earlier wording made declared non-null types sufficient to
hide an actual missing input and reused root-wide deferral for local evidence.
The narrower exception above fixes the motivating guarded command while
preserving the shell approval contract elsewhere.

### 7. Where guards come from

`TypeScope` is extended on entering each of these regions:

- a `::block when="…"` body (true side), nested blocks composing; body blocks
  have no `else`, so a false side is reached by negating the guard
  (`when="!is_string(x)"`);
- a lifecycle `when:` item: `then:` gets the true side, `else:` the false
  side, nesting to the grammar's five levels. Actions authored beside `when:`
  use the same true-side scope as an explicit `then:`;
- a sequence task's `when:`, for values the sequence itself evaluates for that
  task (its `with:` values and `shell:` command), using the bindings visible
  when that guard is evaluated; the task's own `with:` values cannot prove
  their own guard. A task document's own body
  has its own schema and scope; caller guards do not narrow it;
- within an expression: ternaries, `&&`, `||`.

Until `2026-10-27-darkmatter-lifecycle-ownership`, Claudine's lifecycle walk
extends the scope through Darkmatter's `TypeScope::assume`; that spec moves
the walk, and the scope travels with it.

### 8. Lifetime of a fact

Facts end at their guarded region and never persist across lifecycle events,
loop iterations, retries, resumes, or proxy handoffs. For `set`, check the
right-hand expression under the incoming scope, then
invalidate facts for its destination and its ancestors and descendants before
checking later actions. A `set` whose destination is not static discards all
document-property facts. Canonical bare/`fm` aliases invalidate together.
At a conditional join, a possible write on either continuing branch prevents
restoring the affected incoming fact, including after an inner branch ends.
Analyze authored mutations without executing them. Effects that can replace
document bindings must declare their write set or invalidate all such facts;
loop and document re-entry always starts fresh. Re-entering a guard computes
its facts again;
neither runtime nor editor analysis trusts a fact after a mutation.

### 9. Enforcement

The scope must be impossible to bypass, not merely available.

- **By construction.** Static checking and shell candidate construction
  require a scope; `TypeScope` has no `Default`. Code with no enclosing guard
  calls `TypeScope::unguarded(..)`. Runtime hole validation keeps its concrete
  value checks and typed error conversion. Do not move every type diagnostic
  into the scope module or change public error enums merely to restrict where
  variants can be constructed: schema, argument-binding, and runtime errors
  also have legitimate owners.
- **Guard test.** Audit named static presence/type decision functions in shell
  candidate construction and both Claudine preflight modules. Fail on new
  competing static classifiers or unreviewed unguarded scope construction;
  every allowlist entry needs a reason and must match a live site. Do not ban
  `is_null()` generally: runtime validation, predicate implementations, and
  intentional absence handling require it.
- **Conformance matrix.** Reuse a compact table of applicable guard cases
  across body `::shell`, `::shell-block`, transclusion targets, frontmatter
  `$( … )`, lifecycle shells (including the inline guard form), lifecycle
  `set` with a whole-value shell, and sequence task shells and `with:` values.
  An exhaustive site enum makes a new site require an explicit applicable or
  inapplicable entry; do not force lifecycle guards onto unguarded frontmatter.
  Compare passive DMLS analysis with the same library results at the sites it
  supports. Runtime warning reachability and passive editor analysis remain
  intentionally different; both use the same branch facts.
- **Passive analysis.** Scope construction, predicate fact lookup, branch
  joins, and editor analysis perform no I/O or expression evaluation. Keep
  source spans, document ownership, and resolved binding identity attached to
  facts and diagnostics. Share the declared projection and unchanged parent
  scopes rather than rebuilding a whole document schema for each hole. No
  performance spike is required for this change unless implementation reveals
  a concrete cost.

## Amendments to pipeline specs

The initial amendments were written into these specs on 2026-10-16. The
review below clarifies their boundaries; implementation must reconcile those
contracts without introducing a second projection or fact registry:

- **`2026-10-27-darkmatter-lifecycle-ownership`.** The lifecycle walk that
  extends `TypeScope` moves with the engine; preflight's shell audit reads
  presence and hole types from the scope; the conformance matrix passes
  unchanged across the move.
- **`2026-10-28-schema-enhancements`.** Its variable projection is
  `TypeScope`'s declared-type layer; argument types are read from the scope at
  the call site; predicate facts live in the scope's fact registry; exact fit
  is §4's `is_subtype`. Worked-example rows g and h become exact there.
- **`2026-10-31-type-aware-evaluation`.** Its Phase D section defers to this
  fix and keeps only its extensions (`is_file_type` and host predicate facts,
  the projection extensions as the declared-type layer).

## Out of scope

- New guard functions, including `is_boolean`; later predicates register
  facts only when their runtime implementation exists.
- Narrowing through function return values (`is_number(length(x))` proves
  nothing about `x`).
- Changing `==` semantics.

## Delivery and documentation

The dependency `2026-10-23-shell-template-approval` supplies the existing
candidate, template, and runtime validation paths. Deliver the shared scope,
current guard producers, current shell/transclusion/absence consumers, and
DMLS integration in this fix. Build on the existing schema projection now;
do not depend on the later lifecycle move or complete argument-binding system.
The later schema and type-aware evaluation work plugs into this scope for
function arguments, richer expression results, nested unions, and host types.
Cases requiring those projections are explicitly planned, not claimed as
already supported by this fix.

When code lands, update the Darkmatter expression and shell-approval topic
docs, Claudine lifecycle/composition and sequence topic docs, and the relevant
skills with the guard exception, runtime checks, and fact lifetime. Preserve
READMEs as overviews. Docs describe current behavior without linking to this
specification. Record any departure from this draft in the implementation log.

## Open Questions

### What may `file_exists` prove after argument conversion is introduced?

Today Darkmatter's
[`file_exists_fn`](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs)
requires an original string (or returns false for null). The planned argument
binder in `2026-10-28-schema-enhancements` permits `file_exists(42)` by
converting `42` to `"42"`. A successful check would then say nothing about
whether the original variable was a string or a file value. This matters
because an unsound string fact could authorize a narrower shell template or
make function checks silently accept the wrong original type.

1. **Recommend: keep conversion and register only guaranteed original-value
   facts.** Before conversion lands, success proves a present string. After
   it lands, success proves presence only, unless conversion metadata supports
   a more precise original-input union. `is_file_type` later supplies passive
   structural evidence. **Pros:** preserves the planned binding contract,
   avoids coercion-specific exceptions, and keeps authorization sound.
   **Cons:** a `file_exists` guard alone no longer proves a string argument
   when the original type is unknown. This is recommended because weaker
   evidence is preferable to claiming a property of a converted copy as a
   property of the author's original value.
2. **Require original string/file input for this predicate.** Reject numeric
   and boolean inputs before binding. **Pros:** retains a simple original-type
   fact. **Cons:** deliberately changes the planned conversion contract and
   adds a per-function exclusion that must be documented and maintained.
3. **Keep converted-argument facts separate from original-binding facts.**
   **Pros:** preserves conversion and can support precise future analysis.
   **Cons:** introduces another value identity and more analysis machinery
   without solving the motivating shell-presence bug more simply.

The author must resolve this before the conversion migration registers
`file_exists` narrowing. Until then, never infer original string/file type
from a converted argument; the fix can ship against today's strict handler.

## Acceptance

1. `prompts/_implement/implement-plan.md` with
   `git commit -m "{{commit_message}}"` under
   `when: "ctx.dirty_files && commit_message"` passes preflight with no
   `commit_message`, after the discovered template is approved; its false
   branch launches no command. With `commit_message='msg'` and dirty files,
   a recording executable receives exactly that message as one argument.
   Use a frozen prompt fixture and a fake executable; no test creates a real
   commit or reads the mutable internal prompt as a CI input.
2. The same command outside any guard still fails preflight with
   `MissingValue`.
3. A guarded command whose value becomes `null` (via `set`) before its branch
   runs fails that action with `MissingValue` and runs nothing.
4. Guard and equality rules hold at every applicable current site in the
   conformance matrix. Cases needing future projections remain explicitly
   planned until those consumers land.
5. At the shared structural-relation boundary, `literal('release')`, `literal(4)`, `literal(true)`, and
   `literal('a') | literal('b')` each satisfy a parameter typed respectively
   `string`, `number`, `boolean`, and `enum(a, b, c)`, through `is_subtype`.
6. A template hole under `is_string(target)` with
   `target: string | number | null` is approved as `{{ string }}`.
7. Composition and DMLS report the same narrowed type at each supported
   matrix site; future function-binding integration uses that same scope.
8. The guard test fails on a competing static classifier or an unallowlisted
   `TypeScope::unguarded` call, and permits runtime null validation.
9. A guard on `x` does not defer missing `y` or an unguarded occurrence of
   `x` elsewhere. A non-null declaration with an actual null still fails.
10. False guarded branches do not fail at a sequence late-value barrier;
    ordinary unguarded late values retain their existing barrier behavior.
11. Cover null versus empty string, both-null comparisons, container/string
    equality, reversed operands, predicate aliases, optional declarations,
    and both-side union comparison against the real evaluator.
12. Cover `set` invalidation through bare/`fm` aliases and nested branch joins,
    plus a present discovery value changed to a different type before dispatch.
    Validate the live value against the approved refinement and original schema.
13. Scope and DMLS analysis of `file_exists` cause no filesystem or network
    access. Narrowing an unavailable global preserves its typed failure.
14. Existing exact/prefix/template approvals, blacklist precedence, argument
    boundaries, and unsuppressible missing/invalid-value errors remain covered
    by their existing regression tests. Use the repository's L1 nextest recipes
    for affected packages; this static-analysis change does not require a new
    terminal/browser integration tier or an additional CI environment.
