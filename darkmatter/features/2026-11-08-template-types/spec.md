---
kind: feature
created: 2026-11-08
status: finalized-spec
area: darkmatter
related:
    - 2026-11-05-dynamic-schema-types
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-09
review_iterations: 0
needs_rulings: false
---

# Template types

## Summary

A schema slot is a `{{ … }}` expression whose result supplies part of a
schema. Both kinds of slot produce types; they differ in how much of the
type the template supplies:

- A **template constraint** fills a constraint of a type the author wrote
  explicitly: `enum(...)` and `suggest(...)` elements, and the whole argument
  of `literal(...)`, `default(...)`, `min(...)`/`max(...)`, and
  `pattern(...)`. `2026-11-05-dynamic-schema-types` delivered these.
- A **template type** supplies a property's entire type definition, so data
  chooses its type, constraints, and nested structure together. This feature
  adds template types.

These two names replace the earlier "value slot" (also "value-position
slot") and "whole-type slot", which wrongly suggested that one kind
produced values rather than types. Neither old name survives this feature
(see "Terminology rename").

```yaml
$schema:
    status: "{{ load_data('./status-type.yaml', 'type-definition') }}"
    owner: "{{ load_data('./owner-type.yaml', 'type-definition') }}(required)"
```

For example, `status-type.yaml` can contain the YAML scalar
`'enum(draft, published)'`, and `owner-type.yaml` can contain `'string'`.
The document then validates as if those definitions were written in place.
The contract checks that each data value is a type definition; it does not
evaluate the definition or approve its imports.

Reuse the existing passive parser, expression admission, read approvals,
slot sessions, cache policy, and background editor worker. Extend them to
track reads and constraints introduced by the returned definition; reuse
does not mean that the current implementations already cover those reads.
The intended language change is the new type position. Existing template-constraint
behavior, static type behavior, and approval policy remain the contracts to
preserve, except that `md schema validate <schema file>` now resolves its
slots (see "Rulings"). Current references are the
[SimplifiedSchema slot contract](../../../simplified-schema/docs/topics/language/slots.md)
and [Darkmatter slot contract](../../docs/topics/schemas/schema-slots.md).

## Grammar and result contract

The slot occupies a complete type atom, optionally followed by the same
constraints, array suffix, and description that an imported named type
accepts. It is not textual interpolation inside a type name or constraint.

```yaml
$schema:
    status: "{{ load_data('./status-type.yaml', 'type-definition') }}"
    owners: "{{ load_data('./owner-type.yaml', 'type-definition') }}[](required)"
    fallback:
        - string
        - "{{ load_data('./fallback-type.yaml', 'type-definition') }}"
```

The following boundaries keep top-level property names knowable before any
expression runs. They also keep whole-property layering deterministic.

| Position | Allowed? |
| --- | --- |
| A property's type, including nested object properties and named type bodies | Yes |
| A property-level union arm, including the YAML list form | Yes; a returned union joins the surrounding property union |
| A tuple element, such as `tuple([number, {{ … }}])` | Yes; the tuple's own restrictions still apply after resolution |
| A tuple spread, a `function(...)` parameter or return, or a `property(...)` declaration | No; these are meta-level positions whose pending handling costs far more than any realistic use (see "Rulings") |
| The entire `$schema` root or an arm of a root union | No; these describe sets of top-level properties, rather than one property type |
| A property name, pattern key, import path fragment, or part of a primitive name | No |

Use the existing balanced expression scanner, including its handling of
quotes and braces, rather than adding a separate `}}` detector. Plain,
quoted, and block YAML scalars use the same type grammar after YAML decoding.
Quote a YAML scalar that starts with `{{`; an unquoted form can be parsed as
a flow mapping. If the source proves that mistake, report “quote the slot”
at the authored value, including when YAML decoding fails. Do not label an
unrelated mapping error as a slot error.

The host returns a JSON string or array representing the existing
`type-definition` domain. In the simplified-schema package,
[`parse_type_definition_value`](../../../simplified-schema/lib/src/simplified/mod.rs)
is the parser for that domain; reuse it to produce the property's parsed
definition. This includes its existing string-encoded YAML list form. A
mapping, number, boolean, or null result is a shape error; an empty array or
malformed definition is rejected by the ordinary definition rules. A string
`"null"` is a valid type definition, whereas a JSON null result is not.
Do not stringify or coerce other result shapes into schema text.

Parse the returned definition once as schema syntax, expand its named
imports, then apply the written suffix through the existing named-type
merge rules. Do not concatenate the returned text with the suffix and
reparse it. A returned union retains normal union semantics; an array suffix
makes an array of that union. Conflicting defaults or constraints fail as
they do for static imports. Recheck definition invariants after expansion
and suffix merging, including invalid `non-null` combinations.

Presence follows the existing nullable-type rule: `"null"`, `"unknown"`,
and a whole-value union with a null-admitting arm stay optional even when
the use site says `(required)`. Before resolution, a `required` suffix alone
cannot establish whether the property is required. A nullable array item
does not make the array itself nullable.

Reject active slots parsed directly from a returned definition, including
template constraints nested inside it. Literal text containing `{{ … }}`
inside a quoted enum member or default remains literal under the existing
grammar; a raw substring search would wrongly reject it. Slots already
authored in the use-site suffix remain ordinary slots and must pass
admission before evaluation.

**Decision: reject slots reachable through a returned definition's
imports.** A returned definition may import named types (for example
`Person@./types.yaml`), but if any definition reachable from it, directly or
transitively, contains an active slot, resolution fails before that slot
evaluates. Returned data can choose types; it can never cause another
expression to run. Only reachable definitions count: an unused slot
elsewhere in an imported file does not disqualify it. A named type that
contains a slot remains usable through a statically authored import.

All patterns introduced by a returned definition, including through its
imports, must pass the existing slot-pattern checks: at most 4096 bytes,
no lookaround or backreferences, and successful compilation with the
linear-time engine's size limit. Patterns written directly in the schema
keep their current rules. This prevents template types from bypassing
the protection already applied to `pattern({{ … }})`.

> **Reader's note:** template types deliberately parse returned data as
> type syntax, unlike template constraints. That can introduce imports,
> patterns, and nested definitions. The read checks and pattern checks here
> preserve the existing safety boundaries despite that intended difference.

## Imports, reads, and resolution

An import written in a returned definition (`Name@./x.yaml`) resolves
against the **slot's defining schema file**, as decided in
`2026-11-05-dynamic-schema-types`. A slot in an imported named type uses
that type's file; an inline document slot uses the document's context;
in-memory content uses the supplied request context. The data file that
supplied the string is never the import base, even if it is remote. Once a
referenced schema file is loaded, its own imports use the normal base of
that file.

Reject `Name@this` written in a returned definition because the result has
no authored local type table. A normally imported file can still use
`Name@this` to refer to its own named types. Resolve file references through
the existing resolver and `biscuit-file` file-reference handling, deriving
contexts from the original request; never capture process state again.

Every read caused by the returned definition belongs to the originating
slot's approval boundary. This includes direct and transitive schema
imports, referenced examples, and path probes. Check the grant before
opening a file, using an editor buffer, or consulting a shared memo. A
static schema import's normal ability to read a file does not authorize an
import selected by slot data. A denied read must retain its slot refusal
code and cannot become a successful schema through fallback handling.

Imports discovered only after evaluation are computed targets: ask through
the existing handler when permitted, or report approval required. A
pre-flight approval of the data file does not approve the imports its
contents name. Keep reads known in advance in the existing approval batch;
do not evaluate a slot merely to discover more prompt entries. Commands
without prompts and editor jobs keep their existing grant-only behavior.

The simplified-schema package owns grammar, type expansion, suffix merging,
and passive result checking. Darkmatter owns evaluation, grants, and cache
policy. Extend their host-neutral boundary as needed so the guard remains
active during returned-definition expansion, after the expression evaluator
has returned. Do not add a dependency from simplified-schema to Darkmatter.

Record all newly discovered dependencies, including imported files and
missing candidates, on both success and failure. Reuse the exact schema
texts already captured by the pending template; newly discovered files use
the current request's source-buffer reader and retain the bytes actually
read. Shared imports are read once per resolution, not once per property.
Track them in the existing dependency observations and appropriate read-set
records so a warm result has the same observable reads as a cold result.

Cache reuse must check grants before rereading dependencies and include
these new inputs in invalidation. Preserve the existing omitted-property
key, request-context inputs, and cross-command size/mtime recheck policy,
including its accepted same-size/same-mtime limitation. A command or
lifecycle write that changes any dependency forces re-resolution before
the next validation. An editor watches these imports and missing candidates,
honors unsaved buffers, and discards superseded jobs. Failure or cancellation
publishes no partial concrete schema and never validates with stale types.

## Allowed schemas and layering

Allow template types wherever document-selected schema payloads already
allow template constraints: inline `$schema`, referenced schema files,
reachable named types, and matching trigger payloads. An ordinary
`kind: schema` file remains eligible; a definition that claims a YAML
document kind does not. Kind definitions, the baseline, global catalogs,
the editor extension layer, and trigger match conditions retain their
existing slot prohibitions, including slots reached through imports.

**Decision: allow trigger payloads** (ratified 2026-11-09). The containing property name is
written in the payload; its unknown nested structure does not make the
top-level key unknown. Discovery and matching remain passive, and the
trigger is pending rather than invalid. Resolve only after matching and
whole-property replacement determine which declarations survive.

For example, a trigger's `owner: "{{ … }}"` does not run if the document
declares `owner: string`. A returned object's nested keys never become
sibling top-level properties. A root union replaces a lower declaration
only when every arm declares that property, just as today. Root-level
slots are excluded so this decision remains possible without evaluation.

## Pending types and user-facing behavior

An unresolved template type declares a property name, but not a usable
type. Represent that distinction explicitly in query and editor views.
Do not map it to `unknown`, an empty object, an optional property, or a
missing declaration. Lowering, validation, conversion, and schema layering
must retain their typed unresolved outcome until the effective template is
resolved. Passive inspection and source export evaluate nothing; export
preserves the slot and suffix.

The simplified-schema package's
[`TypeExpr`](../../../simplified-schema/lib/src/simplified/types.rs)
represents type atoms. Add a slot variant and update every reader that can
encounter it, including slot collection, origin stamping, source spans,
serialization, named-definition reachability, queries, subtype checks,
narrowing, lint, and conversion planning. Keep existing concrete-type
helpers behind a resolved-state check; pending consumers need an explicit
pending answer rather than inferring facts from boolean helpers. A pending
atom cannot prove subtype compatibility or a narrowing fact merely because
its type is not known. Statically known sibling union arms may still provide
facts that do not depend on the pending arm.

| Surface | Required behavior while a template type is pending |
| --- | --- |
| Key completion and hover | Show the authored property and “type pending”; do not invent nested keys, requiredness, defaults, or value suggestions. Preserve independent static sibling information. |
| `md schema validate <schema file>` | Resolve the template through the same grant gate as `md schema validate <document>` (prompting on a terminal, failing with approval required on a data output), then check the resolved schema: returned definitions, refused reachable slots, introduced pattern limits, and suffix conflicts. A refused or failed resolution is an unavailable check, exit 1, never a success. This applies to every slot in the file, template constraints included; there is no passive template mode (see "Rulings"). |
| `md schema validate <document>` | Resolve through the existing grant gate before validation or schema-aware assignment conversion; preserve the raw assignment text until the type is available. A refused resolution is an unavailable check, exit 1. |
| Composition, cleanup, and phase projection | Resolve before defaults, coercion, required-key decisions, phase filtering, or edits depend on the type. A failed resolution cannot be reported as valid or clean. |
| Reference checks (`md graph`, `md validate refs`) | Keep existing deferred behavior when grants are unavailable; do not validate against a substitute type. |
| Darkmatter Language Server (DMLS) | Resolve in the background, never on a keystroke. Show pending/failed status; any last-good completion or hover must be marked stale and must not produce current “valid” diagnostics. |
| Claudine shell completion | Property names complete from the authored template with nothing evaluated (`stat<TAB>` → `status=`). Value completion (`status=<TAB>`) resolves the template for a property that holds a template constraint or template type, using only grants that never prompt (the automatic grant and persisted user grants), then offers what the resolved type allows (enum members, matched files). An ungranted or failed resolution offers no schema candidates and the shell falls back; nothing is printed. A property with no slot completes its values straight from the template without evaluation. See "Claudine shell completion". |
| Other embedding consumers | Reuse the existing unresolved-template report with the property's pending context where available; do not silently drop an error into empty results. No separate approval mechanism. |

For a valid returned type, all these consumers must behave like they do for
the equivalent static definition. Audit downstream callers by symbol use
rather than treating the earlier estimate of roughly 44 affected files as
an exhaustive list.

## Diagnostics

A slot written in a refused position must fail with an error that leaves
the author in no doubt. It names the position the slot was found in (for
example "a `function(...)` parameter"), states that template types are
not allowed there, lists where they are allowed (a property's type,
including nested properties and named type bodies; a property-level union
arm; a tuple element; with an optional suffix such as `[]` or
`(required)`), and suggests the nearest permitted rewrite where one exists,
such as moving the slot into a named type used by the property instead.
The documentation for the slot positions carries the same table. This
applies equally to the root position, keys, and the other "No" rows above.

Keep the existing slot error structure and typed causes. Wrong result
shapes, invalid returned syntax, nested active slots, and invalid expanded
constraints must name the originating slot and property path. Keep import,
cycle, and grant failures distinguishable so command and editor policy can
handle them as it does today.

The primary source position is the authored slot, not a guessed offset in
the data file. If a real imported schema contains the rejected definition,
keep that file's proven location as related information. A conflict with an
authored suffix identifies the suffix as a participant too. Preserve spans
through YAML escaping, block scalars, Markdown frontmatter, and CRLF; when
no source text proves a location, leave it absent.

## Claudine shell completion

Claudine's tab completion for `claudine compose`, `inline-compose`, and
`sequence` reads the committed prompt's `$schema`. Today any slot makes the
whole schema `PromptSchema::Pending`: names still complete, but value
completion is switched off for **every** property, static ones included,
because completion never evaluates. That behavior is replaced:

```text
claudine compose doc.md stat<TAB>     → status=          (names: nothing runs)
claudine compose doc.md status=<TAB>  → status='draft'  status='published'
                                         (resolves status's slot, then offers
                                          the resolved enum's members)
claudine compose doc.md kind=<TAB>    → kind='a'  kind='b'
                                         (kind: enum(a, b) is static: no
                                          evaluation, even in a template)
```

- Names never evaluate. Required-first ordering uses the authored suffix;
  a template type's requiredness is not resolved for ordering.
- A value request for a property that holds a template constraint or a
  template type resolves the effective template through the existing
  resolver and cross-command cache, with only non-prompting grants (the
  automatic clean-committed grant and persisted user grants). A Tab never
  prompts and never writes to the terminal.
- When a needed read is ungranted or resolution fails, schema completion
  offers nothing for that property and the existing shell fallback applies.
  `claudine compose` itself still asks for approval when it runs.
- A property with no slot completes its values from the template's own
  declaration without evaluation.
- This applies to template constraints as well as template types, and is a
  phase of this feature. Update `claudine/docs/topics/completions/shell-completions.md`.

## Terminology rename

As a phase of this feature, remove "value slot", "value-position slot",
"value position" (in the schema-slot sense), and "whole-type slot" from code,
tests, fixtures, docs, and skills, replacing them with **template
constraint** and **template type**. Known sites include
`simplified_schema::simplified::ValueSlotKind`, `Constraint::DynamicValue`,
the `slot_tests` names, `simplified-schema/lib/tests/fixtures/dynamic_slots/value-positions.yaml`,
`simplified-schema/docs/topics/language/slots.md` (including its planned
"Whole Types" section), Darkmatter's schema-slot docs, and the
`simplified-schema` and `darkmatter` skills; find the rest by search, not
from this list. Claudine's completion code and docs also say "value slot"
for the cursor position after `=` in `name=value`, an unrelated meaning;
rename that to "setter value" in the same phase so the old term is gone
everywhere. Completed specs under `_completed` are snapshots and are not
edited.

## Verification and completion criteria

Implementation is complete when a template type resolves to the same
schema and validation behavior as the equivalent static definition, while
all passive operations remain passive and all introduced reads stay gated.
Use isolated unit and normal CLI invocation tests; this feature requires no
focused terminal or browser windows.

- In simplified-schema, cover string and list results, union flattening,
  suffix/array merging, nested positions, malformed and empty results,
  null-admitting presence rules, forbidden root positions, literal brace
  text, passive query/export, and unresolved lowering/validation.
- Cover defining-file import bases, local references inside imported files,
  import cycles, refusal of slots reachable through returned imports
  (including transitive and unused-slot cases), introduced pattern
  limits, the refused-position errors (each "No" row names its position
  and the allowed positions), and positioned errors through quoted/block YAML and CRLF.
- In Darkmatter, cover replaced trigger properties never evaluating or
  requesting reads, matching pending payloads, read approval before memo or
  buffer access, transitive imports, absent grants without prompts, cached
  read observations, and dependency invalidation after writes or edits.
- Exercise successful resolution and failed approvals through the normal
  compose and schema-validation command paths, including data output and
  type-aware assignments, and `md schema validate <schema file>` resolving
  (template constraints and template types) with granted, prompted, and refused reads. Test editor pending-to-resolved transitions and
  cancellation; verify affected Claudine and other host consumers.
- Keep passive shipped-schema corpus coverage and static-schema regression
  checks. Run affected package-area `just test` and `just lint` recipes and
  the library's existing minimal-feature checks. Preserve compilation on
  macOS, Linux, native Windows, and WSL2 without adding feature-specific CI
  environments.

Update SimplifiedSchema language docs, Darkmatter schema-slot and command
docs, editor docs, and affected skills when implementation lands. Update
README overviews only where their public examples or descriptions change.
No performance spike is required: reuse the existing shared-definition and
per-resolution memo mechanisms. Investigate a measured regression if one
appears; this spec introduces no multi-host benchmark requirement.

## Rulings

### Slots reachable through a returned definition's imports (2026-11-09)

Ruled **reject** (see "Grammar and result contract"). Allowing them through
more admission rounds was rejected as too much indirection: it needs
recursion limits, identity for computed imports, grants for each origin, and
cache and watch dependencies for every round, all so that data can trigger
evaluations the author never wrote. Forbidding every import in a returned
definition was rejected because it would also block harmless static types.

### Template types in trigger payloads (2026-11-09)

Ruled **allow** (see "Allowed schemas and layering"). Layering needs only
the property name, which stays authored in the payload; the slot runs only
after replacement shows the trigger's property survives. Match conditions
stay slot-free.

### Positions beyond property types and union arms (2026-11-09)

Ruled **tuple elements only**. Tuple spreads, `function(...)` parameters and
returns, and `property(...)` declarations refuse a template type: each is a
meta-level position that would need its own pending semantics (signature
subtyping, constraints over other schemas) for little realistic use. The
author required the refusal error to be explicit about the position and the
rules (see "Diagnostics").

### Terminology (2026-11-09)

"Value slot" and "whole-type slot" are replaced by **template constraint**
and **template type**: both kinds of slot produce types, and the old names
implied otherwise. The rename covers code, tests, docs, and skills (see
"Terminology rename"), including Claudine's unrelated "value slot" wording.

### Claudine completion evaluates on value requests (2026-11-09)

Ruled: names complete without evaluation; a value request on a templated
property resolves it with non-prompting grants and offers the resolved
type's candidates; static properties in a template complete without
evaluation; an ungranted read falls back silently. Rejected: the reviewer's
"reuse the pending report" (completion that never evaluates leaves value
completion off for every property in a template, which helps nobody).
Included as a phase of this feature rather than a separate spec.

### `md schema validate <schema file>` resolves (2026-11-09)

Ruled **always resolve**. A passive check certified little once a template type can
hide an entire type: a malformed returned definition still passed as a "valid
template". Schema-file mode now resolves through the existing grant gate,
exactly like document mode, for every slot in the file. Slots cannot read
the document, so a schema file alone has the context it needs. This removes
the passive template mode and its `slots` count as the command's answer;
passive syntax checking remains available to DMLS and other internal
callers. Rejected: keeping it passive (with or without a per-property
pending list) and adding an opt-in `--resolve` flag.

## Out of scope

- Slots anywhere a template constraint is refused today.
- Dynamic root schemas, top-level key generation, and slot syntax in keys.
- Changing the approval model, grant store, cross-command freshness policy,
  or the normal behavior of statically authored patterns. Extending existing
  read guards, dependency tracking, and cache inputs for returned types is
  required work within this feature.
