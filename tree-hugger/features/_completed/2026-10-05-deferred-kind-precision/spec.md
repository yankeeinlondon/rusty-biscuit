---
kind: feature
name: deferred-kind-precision
date: 2026-10-05
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 2
completed: true
human_review: false
implemented: true
implemented_by: "claude/opus"
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
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-visibility-and-kind
  - 2026-10-03-symbol-filters-and-reports
---

# Precise kinds for previously deferred declarations

## Problem

`2026-10-03-visibility-and-kind` corrected kind, visibility, `static` and
container data for an accepted extraction set across all 16 languages. Its
kind matrix marked some constructs `FEAT` (deferred feature): a precision
upgrade that was not asserted by the fix. Those declarations are absent from
symbol reports or reported with a less precise kind.

## What was proposed

The fix originally assigned its `FEAT` cells to
`2026-10-03-symbol-filters-and-reports`. On 2026-10-04 the author kept that
feature to the fix's accepted extraction set, with no new kinds, and moved the
cells here.

| Language | Deferred constructs (`FEAT` cells) |
|---|---|
| TypeScript | EnumVariant; Namespace |
| Go | TypeAlias (`type A = B` is not reported at all today); Field |
| Python | Constructor |
| Java | Constructor; EnumVariant |
| C# | Constructor; EnumVariant |
| C | EnumVariant |
| C++ | Constructor; EnumVariant |
| Swift | Constructor; TypeAlias |
| Scala | EnumVariant; TypeAlias; Field |
| PHP | Constructor; EnumVariant; Namespace |

Constructors outside TypeScript and JavaScript are still reported as `Method`
or are absent.

Three cells the fix marked `FEAT` already extract precisely as a side effect
of other work, and `2026-10-03-symbol-filters-and-reports` asserts them as
landed: C++ `typedef`/`using` (`TypeAlias`, from the shared C typedef
capture), C++ `namespace` (`Namespace`) and Lua `M:m` (`Method`). They are no
longer part of this entry.

The fix's `LIM` (current extractor limitation) cells (Go interface and struct
reported as `Type`; Swift class, enum and struct reported as `Type`) are
documented extractor limits, not grammar limits. Whether to include these
broader type distinctions is an author decision in
[Open Questions](#open-questions); they are outside the required scope
until that decision is made.

## Scheduling

Scheduled 2026-10-05, first of the seven features split out of
`2026-10-03-symbol-improvements` and `2026-10-03-symbol-filters-and-reports`.
It goes first because it changes which records exist and what kind they
carry, and every later feature (type relations, complexity, references,
report formats) reports on those records.

Each cell needs its own extraction and grammar review, and grammar support
cannot be guaranteed in advance. A cell the grammar cannot support is
recorded as a grammar limit in the kind matrix, not dropped silently.

## Prerequisites

- `2026-10-03-visibility-and-kind` (the kind matrix and its cell states).
- `2026-10-03-symbol-filters-and-reports` (its tests distinguish
  "unsupported extraction" from "language has no construct"; landing a cell
  here flips the affected expectations from unsupported to supported).

## Goal and boundaries

After this feature, declarations in the acceptance table below appear once
with their precise kind in both library symbol models and the CLI. Their
parameters, access, containers, names and display headers must remain usable
by the existing filters and reports. A new capture alone does not complete a
cell if its metadata or selectors are wrong.

This feature uses existing kind values and record schema 4.0. It introduces
no new command, selector, record field, compiler integration or cross-file
resolution. It does not infer declarations absent from the source, such as
implicit constructors or fields assigned only through `self.x` or `this.x`.
The C++ aliases and namespaces and Lua methods already supported remain
regression coverage, rather than new deliverables.

**Reader's note:** the original draft listed kinds without defining which
source forms would satisfy them. The table now gives required source forms
and counterexamples. Broader constructor coverage and Go/Swift type
reclassification are separate author choices below, because they can change
existing report contents beyond the deferred declarations.

## Per-language acceptance table

Each row describes required support using the repository's current grammar.
Examples are source snippets; PHP examples assume `<?php`, and Go examples
assume `package p`. Negative cases must not acquire the requested kind.

| Language | Required declarations and resulting kinds | Boundaries and negative cases |
|---|---|---|
| TypeScript | `enum E { Red, Blue = 2 }`: both members → EnumVariant. `namespace Space {}` and `module Space {}` → Namespace; nested named namespaces remain nested. | A property named `Red` is not a variant. Imports, `export * as Space` and string-named ambient modules are not named namespace declarations in this feature. |
| Go | `type Alias = int`, alone or in a grouped declaration → TypeAlias. `type S struct { X, Y int }`: each named field → Field. | `type Defined int` remains Type. Method receivers, parameters and interface method elements are not fields. Embedded fields and fields inside anonymous struct types retain current behavior; promoting embedded names to standalone records is outside required scope. |
| Python | A directly declared class method `def __init__(self, value): ...` → Constructor, including a decorated method. | A top-level or function-local `__init__` stays Function. `__new__` and factory methods retain current kinds; no inherited constructor is synthesized. |
| Java | Explicit instance constructor `class C { C(int x) {} }` → Constructor. `enum E { RED, BLUE }`: each constant → EnumVariant. | A method with a return type, such as `void C()`, stays Method. Initializer blocks are not constructors. Methods in an enum constant's anonymous body do not become members of the enclosing enum. |
| C# | Explicit instance constructor `class C { public C(int x) {} }` → Constructor, including struct/record constructors with a body or expression body. `enum E { Red, Blue = 2 }`: each member → EnumVariant. | A method named `C` with a return type stays Method. Destructors, static constructors and primary constructors retain current behavior pending the broader-constructor decision. |
| C | `enum E { RED, BLUE = 2 };`: each enumerator → EnumVariant, including an anonymous enum. | A variable initialized from `RED` is not a variant. An anonymous enum gains no invented owner name. |
| C++ | Explicit in-class instance constructors, declared or defined (`C();`, `C(int x) {}`, `C() = default;`, `C() = delete;`) → Constructor, including template-wrapped declarations. Scoped and unscoped enum enumerators → EnumVariant. | Destructors, conversion operators and ordinary methods remain their existing kinds. Out-of-class constructors retain current behavior pending the broader-constructor decision; do not resolve class ownership by name alone. |
| Swift | Explicit `init`, `init?` and `init!` declarations in types or extensions → Constructor, retaining the source name `init`. `typealias Alias = Int` → TypeAlias. | `deinit` and factory functions are not constructors. Associated type requirements are not aliases unless they actually declare an alias. Existing Class/Enum/Type distinctions remain unchanged pending the type-classification decision. |
| Scala | Named enum cases (`case Red, Blue`, and `case Value(x: Int)`) → EnumVariant. `type Alias = Int` → TypeAlias. Direct type-body `val`/`var` definitions and declarations → Field, including members of traits and objects. | Abstract `type Element` is not an alias. Method-local `val`/`var` stay Variable. Class parameters retain their existing Parameter records and field-list behavior; destructuring fields are outside required scope. |
| PHP | A directly declared method named `__construct` → Constructor, matching the language's case-insensitive method spelling while preserving the source name. Unit and backed enum cases → EnumVariant. Named semicolon and braced namespaces → Namespace. | A top-level function named `__construct` stays Function. Legacy class-name constructors and factories keep current kinds. `use` imports are not namespace declarations; a global `namespace {}` block has no invented name. |

Within a row, support grouped names individually and preserve overloads as
separate declarations. Repeated namespace declarations remain separate
source records; this feature does not merge them into one synthetic symbol.
A declaration may produce several records when it declares several names,
but overlapping captures of the same name must still produce one record.

## Extraction and metadata contracts

All extraction changes belong in the `tree-hugger` library. Its
[refine_symbol_kind](../../lib/src/file/extract/classify.rs) decides the final
kind before metadata readers run, so adapters and the CLI cannot disagree.
Add query captures where a declaration is absent, and use declaration syntax
and the nearest owner to classify it. Name checks for Python and PHP
constructors are allowed only after establishing that the declaration is a
method. Do not classify a use of a name as a declaration.

Use the existing shared owner and wrapper rules in the `tree-hugger` library's
[member_slot](../../lib/src/file/extract/classify.rs) and
[resolve_declaration_node](../../lib/src/file/extract/declaration.rs), which
locate a declaration through wrappers and stop at scope boundaries. Extend
those shared rules when a new grammar form needs them; do not create a
separate owner search in each metadata reader. Preserve unnamed-owner
boundaries, including anonymous structs and enum constants with class bodies.

For each supported declaration:

- **Constructors:** retain the declared parameter list, defaults, docs,
  attributes and access. Use the existing function payload and parameter
  metrics; do not invent a return type from the owner. Constructor
  reclassification must not make an instance constructor static merely
  because it has no explicit receiver parameter. Parameters and locals keep
  their own scopes and kinds.
- **Aliases:** populate the existing alias target text from the declaration's
  right-hand type, including generic and grouped forms. Do not resolve the
  target or copy the target type's fields onto the alias.
- **Fields:** report each named Go field with its Unicode name-based access
  and instance membership. Scala object fields use the existing static-member
  rule; class and trait fields use the existing instance rule. Complete the
  type's field list from standalone members without duplicate entries or lost
  type annotations. Go interface method elements remain the existing
  documented field-list exception and are not converted to Field records.
- **Variants:** retain each declared name, initializer or discriminant text
  and payload that the existing enum metadata can express. Reconcile an enum's
  variant list with its standalone variants without duplicates. Do not put
  variants into the ordinary field list or add a new variant payload to the
  record schema. The existing `tree-hugger` library
  [SymbolKindData](../../lib/src/shared/schema_v2/kinds.rs) has no standalone
  variant payload; its current Unknown payload is not an Unknown symbol kind.
- **Namespaces:** preserve the full declared name and the existing separators
  and namespace-root rules. For `namespace App; function inside() {}` in PHP,
  adding the namespace record must keep the function's in-file name
  `App\inside`, without prefixing `App` twice. TypeScript namespace members
  similarly keep `Space.inside`. A namespace is never its own container.
- **Access and flags:** apply the established language rules, adding missing
  declaration cases where needed. Java, C# and PHP enum members are Public;
  C/C++ enumerators retain Unknown access under the existing linkage policy.
  TypeScript enum members inherit the owning enum's recorded access; do not
  infer exports beyond the existing export policy. Scala enum cases use the
  owning enum's recorded access when they have no explicit access. Do not
  infer enum-member static modifiers merely from constant-like behavior.
- **Names and containers:** a named enum owns its variants; a named struct
  owns its fields. Use the owner's actual extracted kind, including the
  retained Go/Swift Type limitations. Anonymous owners get no fabricated
  container. Keep the existing qualified-name policy for enumerators, even
  where language name lookup differs; compiler-level resolution is outside
  this feature.
- **Source evidence:** preserve accurate name and declaration spans, body
  spans where applicable, and declaration text. Read keywords from grammar
  tokens and headers from node boundaries. Omit a keyword or display header
  when the existing contract cannot represent it, rather than inventing one.

## Selective queries, reports and cache

The `tree-hugger` library's
[SymbolQuery](../../lib/src/file/query.rs) selects kinds and optional analysis
work. New kinds must be settled before kind selection, including for light
records whose expensive metadata is skipped. A narrow constructor or alias
query must agree with selecting that kind from a complete index.

Keep the command kind sets in `tree-hugger-cli`'s
[declaration commands](../../cli/src/declarations.rs): constructors participate
in `hug functions`; aliases participate in `hug types`; namespaces and
variants participate in `hug symbols` under its existing scope defaults.
The CLI's [class summaries](../../cli/src/classes.rs) already include
constructors with methods. New named fields must agree between `hug classes`
and `hug symbols`. Kind, scope, container, access and signature selectors,
counts, statistics and JSON reports all consume the same records. For
example, `hug functions --kind constructor` must select a Java constructor
while `hug functions --kind method` excludes that same declaration.

Extraction changes bump the `tree-hugger` library's
[EXTRACTOR_REVISION](../../lib/src/lib.rs), which invalidates old persistent
cache entries. Kind and qualified name are inputs to symbol identity, so
reclassified records and affected descendants can receive new IDs. This is
an intended consequence; do not retain obsolete IDs to hide the change.
Unrelated names and IDs must not change merely because a namespace becomes
captured. Record and report schema versions remain unchanged because the
kinds and fields already exist.

Use the existing cache-upgrade test mechanism to prove earlier-revision
entries are recomputed. For the new records, verify that a warm lookup is
actually a hit, and that results match uncached analysis after kind narrowing
or capability widening. A filtered result must never be stored as the file's
complete coverage. Preserve one definitions-query run per analysis and the
current optional-pass behavior; adding a kind must not run references,
semantics or diagnostics just to render a declaration report.

## Grammar limits and implementation sequence

1. Inspect each required form against the pinned grammar and current query.
   Use a small, cleanly parsed snippet to establish the declaration, name,
   owner and keyword nodes; do not upgrade grammars merely to finish a cell.
2. Implement one language row at a time, with its metadata and regression
   cases. Update both the positive kind matrix and the deferred-kind coverage
   matrix as support lands.
3. Verify selective queries, affected reports and cache behavior, then update
   current documentation and the skill.

A missing query capture is an extractor gap, not a grammar limit. If the
pinned grammar cannot represent a required form, record the exact snippet,
grammar identity, observed node or parse failure, and remaining behavior in
the implementation log and current limitations documentation. Keep that
construct in unsupported extraction coverage with an explained reason;
never place it among kinds the language does not have. One unsupported edge
form does not make the whole language/kind combination supported or
unsupported: keep separate test cases for the forms that land and those
that remain limited.

Phasing is permitted, but a phase must identify the rows and forms delivered.
The feature is implementation complete only when every required form is
supported or has a documented, demonstrated grammar limit. An extractor gap
that needs more work does not qualify as completion. Recommendations under
Open Questions do not expand required scope until the author accepts them.
Record any departure from this specification in the implementation log;
maintain current behavior in the docs rather than rewriting a settled spec.

## Validation and documentation

Extend existing tests rather than introducing a parallel test framework:

- In `tree-hugger`'s [kind matrix](../../lib/tests/l1/symbols_kinds.rs), add
  positive fixture assertions for kind and container. Move landed examples
  in its [kind coverage matrix](../../lib/tests/l1/symbols_kind_coverage.rs)
  from unsupported to landed, retaining already landed examples and
  language-wide coverage guards. Add the acceptance table's negative cases.
- Extend the affected scope, qualified-name, containment, access, static,
  header, signature, field-list and adapter tests. Cover repeated names,
  grouped declarations and overloaded constructors where the table requires
  them. Pin the existing adapter losses rather than requiring new schema
  fields to remove them.
- Extend selective-query and cache tests for changed kinds and new owners.
  Compare narrow results with complete results under matching capabilities,
  and check that skipped analysis stays skipped. Add CLI assertions for the
  affected selectors and class summaries, along with cache invalidation.
  Update existing golden reports only for the intended record changes.

Run `just test` and `just lint` in `tree-hugger/` for the implementation.
Existing cross-platform requirements remain in force: macOS, Linux, native
Windows and WSL2. This specification adds no CI environments or gates.
Real-terminal tests are needed only if rendering logic changes; if needed,
use `just test-l2` and the repository's test toolkit without focusing windows.
No new performance spike is required for query/classifier additions within
the existing single-pass extraction path. Work counters test the known
risk of accidentally repeating analysis. A proposed new full-file scan or
extra parse must be reviewed before expanding performance measurement.

Update the area, library and CLI READMEs when their stated support changes;
update [symbol kinds and visibility](../../docs/symbol-kinds-and-visibility.md),
[report behavior](../../docs/symbol-reports.md) and
[schema design](../../docs/symbol-schema-design.md) where the new behavior
changes their claims. Update the tree-hugger skill's limitations and extraction
rules, and query provenance and capture inventories for changed vendored
queries (including PHP's inherited `php_only` query). Current docs describe
behavior directly and do not point to this feature's lifecycle directory.
Leave this feature in its active directory for author review.

## Open Questions

### Should broader Go and Swift type classification be included?

Today Go structs/interfaces and Swift classes/enums/structs share Type.
Changing those owners changes kind filters, labels, container kinds, payload
selection and IDs for existing records. It can also move Swift enums out of
`hug classes`, whose accepted owner kinds exclude Enum.

- **Keep these distinctions outside this feature (recommended).** Pros:
  resolves the explicitly deferred declarations with fewer unrelated report
  changes; preserves tested Type behavior. Cons: users still cannot select
  Go interfaces or Swift classes/enums by their precise kinds.
- **Include them here.** Pros: improves owner precision before later relation
  features. Cons: requires an additional acceptance table and review of
  metadata, class selection, selectors, IDs, caches and documentation for
  previously accepted records. Go defined non-struct types and Swift actors
  also need explicit classification decisions.

Recommend the first option because the deferred records do not require
reclassifying their owners. If the author chooses the second, specify those
mappings and expected report changes before implementation.

### How far should constructor coverage extend beyond explicit instance declarations?

The required table covers explicit instance constructors and the established
Python/PHP names. It does not settle Java compact record constructors, C#
static or primary constructors, or C++ out-of-class definitions. Some forms
have no separate declared name; others need ownership decisions beyond the
current declaration's immediate owner. This affects source spans, signatures,
class summaries and how many records a class declaration creates.

- **Deliver the required forms and document these remaining limits
  (recommended).** Pros: matches the existing deferred examples and gives
  precise, testable support without synthetic records. Cons: constructor
  coverage remains incomplete and must be stated in the support docs.
- **Include every grammar-supported explicit constructor form.** Pros:
  avoids a later constructor follow-up. Cons: needs rules for implicit names,
  shared class/constructor spans, static-constructor access and grouping,
  and same-file ownership of out-of-class C++ definitions. Cross-file owners
  must still remain unresolved.
- **Include only additional forms with a distinct constructor node and name.**
  Pros: improves coverage without synthesizing declarations. Cons: support
  remains uneven and the author must identify exactly which forms qualify;
  grammar shape alone does not settle ownership or access.

Recommend the first option to make the feature's completion criterion clear.
If broader coverage is selected, add positive and negative examples and
metadata rules for each additional form before implementing it.
