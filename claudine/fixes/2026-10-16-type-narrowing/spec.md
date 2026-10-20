---
kind: fix
created: 2026-10-16
status: draft-spec
clarified: true
reviewed: false
implemented: false
completed: false
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

The command can never run with a missing value, yet the document is refused.
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

## Today: five places decide presence or type

| Site | Decides by | Honors guards |
|---|---|---|
| `expression/absence.rs` `AbsenceScope` | `x \|\| d`, `x ? x : …`, `is_null(x)` inside one expression | within one expression only |
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

`TypeScope` is an immutable, Darkmatter-owned scope mapping property paths to
their narrowed type at one site. It starts from the declared type in the
site's `EffectiveSchema.simplified` (falling back as
`2026-10-23-shell-template-approval` §2 does) and is extended, never mutated,
on entering a guarded region.

- **One question.** Consumers ask `scope.type_of(path)`. A value is *proven
  present* when that type does not admit `null`. No consumer tests a value for
  `null` to decide presence or type outside the scope module.
- **One producer.** Guard facts come only from
  `TypeScope::assume(&Expr, Polarity)`, which absorbs
  `narrowed_expression_paths` and `AbsenceScope`'s branch rule. Runtime
  composition and DMLS call the same code.
- **Stable subjects.** Facts apply only to a resolved variable or a path with
  constant members or indices. Equivalent bare and `fm` paths share a fact
  only when both resolve to the same document binding. `is_number(length(x))`
  proves nothing about `x`; dynamic indices and repeated function calls are
  not stable subjects. Reserved roots (`ctx`, `env`, `seq`, `loop`, `group`)
  carry no declared type until `2026-10-31-type-aware-evaluation` projects
  them, and until then receive only presence facts.
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
| truthiness `x` | `null` removed | nothing (`0`, `''`, `false` are falsy) |
| `x != null`, `!is_null(x)` | `null` removed | `x` is `null` |
| `is_string(x)`, `is_number(x)`, `is_boolean(x)`, `is_array(x)`, `is_object(x)`, `is_null(x)` (true-iff) | `x` is that family | that family removed from `x`'s union |
| `file_exists(x)` (true-implies; `2026-10-31-type-aware-evaluation` later registers `is_file_type` the same way) | `x` is present and file-suitable | nothing |
| `x == v` with `v` a scalar literal (true-iff, see §3) | `x` is the literals of its type whose string form is `v` | those literals removed from a finite union or enum |
| discriminant `x.kind == 'a'` | `x` is the arm tagged `a` | that arm removed |

- `!` swaps the sides; parentheses preserve facts.
- `a && b`: the right operand and the true side get `a`'s true facts and
  `b`'s true facts; the false side gets only facts common to both failing
  routes.
- `a || b`: the right operand gets `a`'s false facts; the true side gets only
  facts common to both succeeding routes.
- `c ? t : e` gives `t` the true facts and `e` the false facts.
- After a guarded region, the incoming type is restored except for facts
  common to every exit.
- **Truthiness is limited evidence.** Removing `null` from `unknown` is an
  internal non-null fact, not a new schema type, and does not establish
  compatibility with a concrete parameter. The evaluator's truthiness and
  value-returning `||` are unchanged.
- **Predicates observe original values.** A registered fact names the
  parameter position and the runtime value family it tests, and must hold for
  the original argument before any conversion. Narrowing preserves compatible
  refinements and constraints; a family predicate does not invent item types,
  object members, or file existence.
- **Conservative exclusions.** A false-side removal applies to a finite
  declared union or enum. When the remainder cannot be represented (`unknown`
  excluding strings), `unknown` is kept. A true side can still establish its
  tested family from `unknown`.
- **Impossible branches remain checked.** A guard that contradicts the
  incoming type (`x: enum(a, b)` with `x == 'z'`, or `is_number(flag)` with
  `flag: boolean(required)`) keeps the incoming type for that subject in that
  branch. There is no empty type that makes every call acceptable, so a call
  known to be invalid is still an error in code that will not run.

Example, with `target: string | number | null` and
`mode: enum(release, debug, test)`:

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

Darkmatter's `==` compares scalar string forms, so `4 == '4'` is true. A true
`x == v` proves only that `x`'s string form is `v`'s. The true side narrows `x`
to the union of literals, one per scalar family `x`'s type admits, whose string
form equals `v`'s; the false side removes exactly those literals.

| `x` declared | Guard | True side | False side |
|---|---|---|---|
| `string` | `x == 'release'` | `literal('release')` | `string` |
| `enum(release, debug)` | `x == 'release'` | `literal('release')` | `literal('debug')` |
| `number` | `x == 4` | `literal(4)` | `number` |
| `string \| number` | `x == '4'` | `literal('4') \| literal(4)` | `string \| number` |
| `boolean` | `x == true` | `literal(true)` | `literal(false)` |
| `enum(a, b)` | `x == 'z'` | `enum(a, b)` (contradiction kept) | `enum(a, b)` |

### 4. One subtype relation

`is_subtype` in `schemas/simplified/subtype.rs` becomes the only
type-acceptance relation. Every consumer that asks "is a value of type A
acceptable where B is expected" calls it. It grows to cover:

- `literal(v)` ⊆ its scalar family: a string literal ⊆ `string`,
  `literal(4)` ⊆ `number`, `literal(true)` ⊆ `boolean`;
- `literal(v)` ⊆ `enum(…)` that has `v` as a member;
- `enum(X)` ⊆ `enum(Y)` when X ⊆ Y;
- unions on either side: `A₁ | A₂` ⊆ `B` when every `Aᵢ` ⊆ `B`, and
  `A` ⊆ `B₁ | B₂` when `A` ⊆ some `Bᵢ`.

Its existing rules (alphanumeric options, always-text types ⊆ `string`) and its
bounded-reasoning contract are unchanged. A function parameter typed `string`
therefore accepts a narrowed `literal('release')` with no special case at the
call site.

### 5. Every consumer reads the scope

| Consumer | Reads from the scope |
|---|---|
| Shell missing-value check (`2026-10-23-shell-template-approval` §1) | missing only when the narrowed type admits `null` (§6) |
| Shell template holes (same spec, §2) | the hole's type is the narrowed type at the site, not the declared type |
| Transclusion target nullability | replaces `narrowed_expression_paths` |
| Absence warnings | replaces `AbsenceScope`'s private branch rule |
| Function argument checks (`2026-10-28-schema-enhancements`, `2026-10-31-type-aware-evaluation`) | the argument's narrowed type, compared by §4's relation |
| DMLS hover, completion, diagnostics | the same scope through the same library |

### 6. A guarded missing value is checked when its branch runs

At preflight, a shell placeholder whose value is `null` but whose narrowed
type excludes `null` is not an error. It takes the existing late-value route
(`DeferredShellRoots`, already used for `loop.*`): the command is discovered
and approved as a typed template, and the value is checked again when the
branch runs. If it is missing then, the action fails with the existing
`MissingValue` error before the command runs.

Discovery stays condition-blind and every command is still approved before
launch (`2026-10-25-loops-and-flow-control` §6,
`2026-10-27-darkmatter-lifecycle-ownership` §4). "A shell command never runs
with a missing value" still holds; only the moment of the check moves for a
guarded value.

### 7. Where guards come from

`TypeScope` is extended on entering each of these regions:

- a `::block when="…"` body (true side), nested blocks composing; body blocks
  have no `else`, so a false side is reached by negating the guard
  (`when="!is_string(x)"`);
- a lifecycle `when:` item: `then:` gets the true side, `else:` the false
  side, nesting to the grammar's five levels;
- a sequence task's `when:`, for values the sequence itself evaluates for that
  task (its `with:` values and `shell:` command). A task document's own body
  has its own schema and scope; caller guards do not narrow it;
- within an expression: ternaries, `&&`, `||`.

Until `2026-10-27-darkmatter-lifecycle-ownership`, Claudine's lifecycle walk
extends the scope through Darkmatter's `TypeScope::assume`; that spec moves
the walk, and the scope travels with it.

### 8. Lifetime of a fact

Facts end at their guarded region and never persist across lifecycle events,
loop iterations, retries, resumes, or proxy handoffs. A `set` to a path
invalidates facts for that path and its ancestors and descendants before later
actions are checked; a `set` whose destination is not static discards all
document-property facts. Re-entering a guard computes its facts again;
neither runtime nor editor analysis trusts a fact after a mutation.

### 9. Enforcement

The scope must be impossible to bypass, not merely available.

- **By construction.** `ShellExpansionError::MissingValue`, and every type
  mismatch diagnostic, is constructible only inside the scope module, from a
  `TypeScope`. `TypeScope` has no `Default`; code with genuinely no enclosing
  guard calls `TypeScope::unguarded(..)`.
- **Guard test.** In the style of `spawn_site_guard.rs`: a bare `is_null()`
  presence test in `shell_expansion/` or either Claudine preflight module
  fails, and every `TypeScope::unguarded` call needs a reasoned allowlist
  entry; an entry matching no live call fails too.
- **Conformance matrix.** One table crosses guard shapes (§2, §3, nesting,
  `else`, contradiction, invalidation after `set`) with every site kind. Site
  kinds are an enum the matrix matches exhaustively, so a new site kind does
  not compile without rows. Site kinds today: body `::shell`, transclusion
  target, frontmatter `$( … )`, lifecycle `shell`, lifecycle `set` with a
  `$( … )` value, sequence task `shell`, DMLS diagnostic.

## Amendments to pipeline specs

Written into each spec on 2026-10-16:

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

- New guard functions or predicates beyond those listed in §2.
- Narrowing through function return values (`is_number(length(x))` proves
  nothing about `x`).
- Changing `==` semantics.

## Acceptance

1. `prompts/_implement/implement-plan.md` with
   `git commit -m "{{commit_message}}"` under
   `when: "ctx.dirty_files && commit_message"` passes preflight with no
   `commit_message`; with `commit_message='msg'` and dirty files, the commit
   runs with that message.
2. The same command outside any guard still fails preflight with
   `MissingValue`.
3. A guarded command whose value becomes `null` (via `set`) before its branch
   runs fails that action with `MissingValue` and runs nothing.
4. Every row of §2 and §3 holds for every site kind in the conformance matrix.
5. `literal('release')`, `literal(4)`, `literal(true)`, and
   `literal('a') | literal('b')` each satisfy a parameter typed respectively
   `string`, `number`, `boolean`, and `enum(a, b, c)`, through `is_subtype`.
6. A template hole under `is_string(target)` with
   `target: string | number | null` is approved as `{{ string }}`.
7. Composition and DMLS report the same narrowed type at each matrix site.
8. The guard test fails on a new bare `is_null()` presence check and on an
   unallowlisted `TypeScope::unguarded` call.
