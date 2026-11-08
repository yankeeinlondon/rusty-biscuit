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
status: finalized-spec
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
clarified: false
implemented: false
review_iterations: 0
packages:
    - darkmatter
    - darkmatter-lsp
related:
    - 2026-11-04-loading-data
depends-on:
    - 2026-11-04-loading-data
    - 2026-11-05-dynamic-schema-types
    - 2026-11-05-expression-recovery
---

# Member Property Check

Report a member read that a value's known object type does not declare,
such as `h.nmae` when `h` is `{ name: string, port: number }`. Apply the
same rules to schema-typed frontmatter, typed function results, and local
bindings in composition and in the Darkmatter language server (DMLS).
The warning identifies a likely typo without changing the value returned
by the expression.

## Motivation

A literal `as` contract gives `load_data` results a static type, and DMLS
already offers `name` and `port` after `h.` inside a callback. The type is
known, but a misspelled member still evaluates to `null` and renders as
empty text with no report. The prerequisite 2026-11-04-loading-data
intentionally excluded this check: checking only `load_data` results would
make them behave differently from other typed values. This feature closes
that gap for all expression values whose member declarations are known.

## Current behavior and implementation boundaries

The following behavior was checked against the source during the
2026-11-08 review. These are the contracts this feature extends.

- **Only undeclared roots are reported.** In the `darkmatter` package,
  [root reconciliation](../../lib/src/markdown/compose/unknown_identifiers.rs)
  reports `dm.expression.undeclared_property` when an evaluated, unhandled
  root is absent from final state, caller inputs, and the effective schema.
  `{{ sever.name }}` warns; `{{ server.nmae }}` does not. DMLS checks roots
  statically in the `darkmatter-lsp` package's
  [expression findings](../../dmls/src/overlay/expressions.rs).
- **Missing members return `null`.** In `darkmatter`,
  [evaluate_member](../../lib/src/markdown/compose/expression/mod.rs)
  returns `null` for a missing object member. A dotted variable can also be
  resolved as one path, so observing only this function would miss reads
  such as `server.nmae`.
- **Static types already exist.** In `darkmatter`,
  [TypeScope](../../lib/src/markdown/compose/expression/scope/mod.rs)
  describes the types and branch facts at an expression site;
  [expression_type](../../lib/src/markdown/compose/expression/call_check.rs)
  also types literal objects, function returns, and literal `load_data`
  contracts. Inline `map`/`filter` callbacks receive the element type.
  Region item locals receive their source's schema-derived type, not a
  new inferred type from a frontmatter initializer.
- **Absence handling and reachability already have shared rules.** In
  `darkmatter`, [AbsenceScope and the static read inventory](../../lib/src/markdown/compose/expression/absence.rs)
  distinguish an intentional absence check from an ordinary read.
  Composition observes evaluated branches only; DMLS examines all branches
  using each branch's facts. The two consumers therefore share a verdict
  for the same read and scope, rather than promising identical warning
  lists for an entire document.
- **Warnings participate in policy.** The `darkmatter` package's
  [BlockingPolicy](../../lib/src/markdown/compose/policy.rs) can stop
  composition on a selected warning. Advisory severity does not mean that
  a warning can never block a command.

**Sequencing (2026-11-08).** Implement after 2026-11-05-dynamic-schema-types
and 2026-11-05-expression-recovery: this feature reads schema declarations that
dynamic-schema-types makes possibly unresolved, and builds on the same
`expression_type` and absence code that expression-recovery changes.

## Design

### Decide from the base type, not the runtime object

For each constant member selector, inspect its base's type at that site
before projecting through the selector. Report an unhandled read only when:

1. There is at least one non-null alternative in the base type.
2. Every non-null alternative is an object with a known, closed set of
   property names.
3. None of those alternatives declares the selected property.

Ignore null alternatives for this decision. A null-only base stays silent:
there is no known object declaration against which to check the name.
A declared optional member stays silent even when absent at runtime; this
feature checks declarations, not requiredness or data validity.

Use the existing branch-refined type. Do not infer a closed object from the
keys present in a runtime value, execute an expression to learn its type,
or load a data file to inspect its keys. The message says that the **known
type** does not declare the member; it must not promise that runtime lookup
always returns `null`, since caller-supplied data can differ.

| Base type at the read | Selected member | Result before absence handling |
| --- | --- | --- |
| `{ name: string }` | `nmae` | Warn |
| `{ name: string }` | `name` | Silent, including an optional unset member |
| `{ name: string } \| null` | `nmae` | Warn |
| `null` | `nmae` | Silent |
| `{ name: string } \| { port: number }` | `name` | Silent: one alternative declares it |
| `{ name: string } \| { port: number }` | `nmae` | Warn |
| `{ name: string } \| string` | `nmae` | Silent: not every alternative is a known object |
| `unknown`, `any`, bare `object`, or a dictionary | any name | Silent |

An open object, pattern-key object, unresolved reference, or any unknown
alternative prevents a warning. A named type already available in the site's
definition table can be inspected through the existing bounded reference
projection; unresolved or cyclic alias chains remain unknown. A literal
`load_data` contract's named references retain their current unknown
projection. Do not add import resolution or file access to this check.

**Unresolved schema templates.** A document whose schema contains dynamic
slots has no resolved effective schema until resolution runs (and in DMLS,
until a snapshot is published). Slots never change which properties an object
declares, so read declarations from the template: a pending template still
supports this check, before and during resolution. A future slot that yields a
whole type makes that property's type unknown, which is silent under the rules
above. The same template declarations also feed the existing call check
while slots are pending (author decision, 2026-11-08), on the condition that a
constraint still waiting on a slot (a pending `enum(...)`, `suggest(...)`, or
value-position slot) is treated as unknown rather than as the literals
written beside it.

**Review decision:** retain the draft's conservative union rule. Reporting
only names absent from every known object alternative identifies definite
declaration mismatches without introducing a new requirement to narrow every
union before using a member. A possible absence in only some alternatives
remains outside this feature.

### Cover equivalent access forms

Check every step of dotted paths, including paths represented as a single
variable node, postfix member access on a call or parenthesized expression,
and constant string bracket access. For example, `server.nmae`,
`(server).nmae`, and `server["nmae"]` have the same classification.
A bracket key containing a dot is one literal key: `server["a.b"]` does not
mean `server.a.b`.

Computed keys such as `server[key]` are not themselves checked. Continue
checking ordinary reads inside `key`, and check later constant selectors if
the existing type projection knows their base. Numeric indexes do not gain
new bounds diagnostics; existing array and tuple projection may supply the
base type for `hosts[0].nmae`.

For a chain such as `server.nmae.length`, check `nmae` against `server`.
Once a missing step projects to unknown, do not invent a type or emit
cascading warnings for subsequent names. An undeclared root still receives
only its existing root warning; it has no known object type for a member
warning. Preserve existing namespace and host-global diagnostics: namespace
wrappers such as `fm` and `local` are not closed data objects, but a typed
value selected through them (`fm.server` or `local.h`) is eligible.

### Preserve intentional absence handling

Extend the existing shared absence classifier to member reads. Do not add
a separate function-name list or branch walker. The same direct-position,
nullable-parameter, overload, and branch-fact rules that apply to roots
apply to members.

For `server: { name: string }`:

| Expression | Result |
| --- | --- |
| `server.nmae` | Warn |
| `server.nmae \|\| "none"` | Silent: fallback primary |
| `server.nmae ? 1 : 0` | Silent: direct ternary condition |
| `is_null(server.nmae)` | Silent: direct nullable argument |
| `min(server.nmae, 5)` | Warn: parameter does not handle null |
| `is_empty(trim(server.nmae))` | Warn: nested call does not handle this read |
| `server.nmae == "x" ? 1 : 0` | Warn: comparison operand is an ordinary read |
| `server.nmae ? server.nmae : "none"` | Silent: true branch proves the path present |
| `server.nmae ? "some" : server.nmae` | Warn for the false-branch read when reached |

A guard's existing ancestor-path coverage is preserved, including its
intentional suppression of descendant reads; a fact about one member does
not cover a sibling. Handling absence is warning policy, not a declaration
of a new member. Bare conditions on directives such as `when="server.nmae"`
are gates and remain ordinary reads, rather than becoming ternary-style
absence checks.

### Observe composition reads and analyze editor reads

Composition reports only selectors it evaluates. An unchosen branch, a
short-circuited operand, or a callback over an empty collection does not
produce a composition warning. DMLS checks authored expressions in all
branches and callback bodies, even when runtime execution would skip them.
It uses the same type and absence classifier for each site.

Cover the existing observed root-read surfaces wherever they have a typed
site: frontmatter interpolation, body interpolation, expression-typed
frontmatter, page-block and transclusion `when=` conditions, shell-template
holes, inline callbacks, and `::map`/`::filter` regions. Preserve each
surface's existing observation policy. In particular, do not add a second
warning to directive targets already handled by nullable-target diagnostics.
Lifecycle and embedding hosts can use the shared classifier with their own
typed site; this feature does not require changing every host's observation
API. Standalone unobserved evaluators retain their current behavior.

Carry the site's declared types and lexical item identity into observation;
the current root observer's untyped branch facts alone cannot classify
members. Reuse `TypeScope`, its invalidation on writes, its branch routing,
and the existing callback/region scope construction. Do not recapture process
state or resolve schemas per member or per loop iteration. Do not use a
static scan to emit warnings for composition branches that never ran.
Discovery and validation remain passive, and discovery-only candidates
never escape into a composition report. Frontmatter's repeated passes must
not report a declaration mismatch using a provisional schema: retain the
site information needed to decide under the prepared effective schema.

### Report one warning per authored access site

Use the new code `dm.expression.undeclared_member`, at Warning severity,
independent of the editor's strict-call setting. Keep
`dm.expression.undeclared_property` and its root behavior unchanged.
The library owns the code, message, classification, and expression-relative
span. Composition, CLI, and DMLS adapt that shared result to their output.
An example message is: `Member 'nmae' is not declared by the known object
type of 'server'.` A non-path base uses its authored expression as the
subject rather than inventing a dotted path.

Range the offending member name, not the whole expression. For constant
bracket keys, range the key's string token. Dotted variable tokens and
postfix member names can contain several segments; preserve or derive their
segment spans from authored tokens in the shared parser/read inventory.
If shared expression metadata must change, it belongs in the
`simplified-schema` package's
[expression syntax](../../../simplified-schema/lib/src/expression/ast.rs),
not an editor-only reparse. Project through existing source maps; unknown
authored positions remain absent rather than using guessed line numbers.
DMLS converts positions to its protocol coordinates, including Unicode.

Deduplicate by source document and authored selector span, not path spelling
alone, runtime item, or current output offset. Repeated iterations,
frontmatter passes, and repeated inclusion of the same authored site produce
one warning in a combined report. Separate sites with identically named
locals each retain their warning. Keep distinct child documents distinct
and preserve authored source attribution after composition rewrites.

**Reader's note:** this deliberately replaces the draft's proposed
once-per-path identity for members. Two callbacks can both spell `h.nmae`
while binding `h` to unrelated types. A path-only key could hide one mistake;
a source-site key avoids that problem and still prevents per-item warning
floods. Existing root-warning identity does not change.

Warnings are advisory under the default policy and member lookup retains
its current runtime result. Register the code in the existing compose policy
registry so `md compose --deny dm.expression.undeclared_member` and
`--deny all` can block it using the normal warning checkpoints. Blocking
changes the command outcome, never the diagnostic severity. Do not introduce
new policy flags. Preserve structured diagnostics and earlier warnings on
failure, and keep CLI data output free of diagnostic text on stderr.

**Review decision:** a separate code makes the new behavior selectable
without changing the meaning of the root warning. The small registry and
documentation cost is preferable to making existing root-warning filters
silently include a different class of problem.

## Non-goals

- Changing member lookup, making undeclared members inherently fatal, or
  validating runtime data against object schemas again.
- Checking types unavailable to the existing static projection, including
  computed `as` contracts, or adding data-file reads to editor analysis.
- Diagnosing scalar member access, missing array slots, or members absent
  from only some union alternatives.
- Inferring a region source's type from its frontmatter initializer,
  suggesting spellings, or adding quick fixes.
- Adding a performance spike or a new test tier. Reuse prepared site types
  and hash-backed diagnostic identity; no known OS-specific cost motivates
  separate measurements for this change.

## Acceptance criteria and verification

Use focused L1 library and language-server tests, plus CLI policy/output
coverage through the normal command path. No real terminal or browser is
needed. Run the package area's existing test and lint recipes when this
feature is implemented; this specification review changes no runtime code.

1. With a fixture containing at least one host, this expression reports at
   `nmae` in composition and DMLS and still composes under default policy:

   ```markdown
   {{ map(load_data("hosts.yaml", "{ name: string, port: number }[]"), h => h.nmae) }}
   ```

   Several input rows still produce one warning for that authored selector.
   An empty typed input produces no composition warning, while DMLS still
   reports the authored typo. A computed contract produces neither member
   warning nor inferred member names.
2. A schema-typed `server` reports `server.nmae` in dot, parenthesized,
   `fm.server.nmae`, and constant-bracket forms. `server.name` stays silent,
   including when the declared optional member is unset. Cover a typed
   function return and literal object as bases, not just variables.
3. A schema with a pending enum slot still reports `server.nmae` in DMLS before
   its snapshot is published; composition, which always resolves the schema
   first, reports it after resolution. While slots are pending, any constraint
   waiting on a slot is treated as unknown, never as a narrower type (Ruling
   B, 2026-11-08).
4. Pin every row of the type and absence tables, including mixed scalar
   unions, null-only bases, dictionaries, open/pattern-key objects, bounded
   named references, and unresolved imports. Narrowed branches use the
   narrowed type; a later write invalidates facts under existing rules.
5. Cover nested dotted paths, a literal bracket key containing a dot,
   computed keys with an ordinary undeclared root inside them, and a typed
   array element followed by a misspelled member. Report the first missing
   step without cascading to the next name.
6. Inline callbacks and region locals obey lexical shadowing, including
   `local` access and outer captures. Two separately authored callbacks
   using `h.nmae` retain separate warnings; iterations of each do not add
   warnings. Repeated includes deduplicate the same source site, while
   different source files remain distinct.
7. Exercise the listed composition surfaces with the same shared cases.
   Assert library/editor agreement on code, message, severity, and span for
   equivalent sites, and separately assert their documented reachability
   difference. Include body edits, multiline frontmatter, and Unicode before
   the member to verify authored positions.
8. Default composition succeeds, explicit denial and denial of all warnings
   stop through existing policy checkpoints, and structured CLI output
   carries the shared diagnostic with clean stderr. A later failure retains
   previously reported member warnings. Existing root diagnostics,
   suppression rules, namespace diagnostics, and unobserved evaluators pass
   unchanged.
9. Update the current behavior documentation in
   [Darkmatter expressions](../../docs/topics/darkmatter-expressions.md),
   [DMLS diagnostics](../../dmls/docs/diagnostics.md), and
   [CLI policy](../../docs/cli/policy.md), plus the Darkmatter skill's
   expression/observation guidance. Explain the union rule, absence handling,
   site identity, and runtime/editor reachability difference with examples.
   Keep the README an overview; current documentation must not refer back
   to this specification as its authority.

## Open questions

None remaining from this review. The diagnostic code, conservative union
rule, equivalent access forms, and source-site warning identity are decided
above. Implementation should preserve the stated boundaries rather than
expand inference or warning coverage opportunistically.
