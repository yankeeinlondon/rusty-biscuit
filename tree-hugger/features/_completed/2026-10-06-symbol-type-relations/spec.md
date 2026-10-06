---
kind: feature
name: symbol-type-relations
date: 2026-10-06
status: implemented
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-06
review_iterations: 12
human_review: false
human_review_items: []
message_to_agent: |-
  The author accepted Option B in this conversation: retain precise extends
  and implements lists and add relations.unclassified_bases for ambiguous
  written bases. Classification may be proved by declaration syntax without
  resolving a target ID. Swift extension conformances attach only to a
  uniquely resolved same-file owner; external owners remain a documented gap.
  The accepted decision and follow-up implementation are recorded in log.md.
  Tasks 6.5-6.7 and the review-2 cache robustness matrix are implemented.
  Review-7 findings are implemented: nested Swift generic contexts, indexed
  C++ inline lookup, and indexed Rust/C#/Swift bound attachment. Ready for
  review iteration 8; repair details and verification are recorded in log.md.
  Local verification and remaining limitations are recorded in log.md.
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
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-05-deferred-kind-precision
completed: true
---

# Generics, Inheritance, Modifiers, Attributes and Body Line Counts

## Problem

After `2026-10-03-symbol-filters-and-reports`, the `tree-hugger` library's
[SymbolRecord](../../lib/src/shared/schema_v2/mod.rs), the declaration record
returned by selective queries and serialized by the CLI, carries scope,
qualified names, static and async flags, deprecation and test flags, and
parameter and documentation metrics. Several existing fields still lack data:

- `type_info.generic_params` is empty. The older model already collects
  parameter names for Rust, TypeScript, Go, Java, C#, Swift and Scala in the
  library's [type metadata readers](../../lib/src/file/extract/type_metadata.rs),
  but those lists do not reach the record's generic-parameter field or retain
  structured bounds and defaults.
- `relations.extends` is empty.
- `relations.implements` is populated only on Rust trait-impl members. The
  library's [Rust impl reader](../../lib/src/file/extract/rust_impl.rs)
  reduces the trait to its last path segment, and
  [record construction](../../lib/src/file/tree_file.rs) links it when exactly
  one same-named trait is in the file. This can incorrectly link an explicitly
  external trait to a local declaration. The implementing type gets no relation.
- Modifiers beyond static, static storage and async, and the raw attribute
  list, are empty.

No metric reports a callable's body size. Callers need these source facts
without invoking a compiler or project-wide type resolver.

**Reader's note:** the draft described the older generic-name extraction as
Rust-only. The existing readers cover several languages; this feature adds
structured record data and reuses those readers where their meaning agrees.

## Origin, scheduling and prerequisites

These items were proposed in `2026-10-03-symbol-filters-and-reports` and moved
here by the author on 2026-10-04. Scheduled 2026-10-06, this feature follows:

- `2026-10-03-symbol-filters-and-reports`: selective queries, metrics absence
  semantics, attribute parsing for test/deprecation flags, and `--tree`.
- `2026-10-05-deferred-kind-precision`: precise constructors, aliases, fields,
  variants and namespaces on which this feature operates.

The latter specification leaves broader Go/Swift type distinctions and some
constructor forms to the author. This feature must work with the kinds that
actually land; it cannot assume Go interfaces become Interface or Swift
classes become Class. Read declaration syntax where a kind is too broad.
`2026-10-07-cyclomatic-complexity` follows and shares the metrics field extended
here with `lines`.

The author ruled on 2026-10-05 that lack of a report consumer is not a reason
to defer these facts: callers need the data before they can use it.

## Goal and boundaries

Populate structured generics, declared inheritance/conformance, the specified
modifiers and raw attributes on existing declaration records; add a callable
body-line metric; and show inheritance on existing outline container rows.
The complete index and a selective query with matching capabilities must
agree, including after a cache hit, narrowing or widening.

This feature adds no symbol kinds, selectors, commands, compiler integration,
cross-file resolution, inferred declarations, transitive inheritance or
method-override matching. A written `override` modifier does not populate
`relations.overrides`. Runtime mixins, Python `Generic[...]` inference,
JSDoc/PHPDoc templates and Go structural interface satisfaction are outside
scope. Existing names, IDs, access rules, scope and static/async behavior stay
stable unless a separately accepted prerequisite changes them.

## Structured generic parameters

Use the existing `tree-hugger` library
[GenericParam](../../lib/src/shared/schema_v2/facets.rs), which describes a
parameter's name, kind, constraints, default, variance and position. Populate
it on each captured declaration that owns an explicit parameter list,
including types, aliases, functions, methods and constructors where the
language permits them. Never copy an enclosing type's parameters onto its
methods, or an impl block's parameters onto the implemented type. Impl blocks
have no independent record in this feature.

- Keep source order and zero-based `position`. Preserve Rust lifetime names
  with their apostrophe. Map type, lifetime and const/non-type parameters to
  the existing Type, Lifetime and Const kinds. Do not truncate positions to
  fit `u16`; an unrepresentable list must produce an explicit analysis error.
- Read bounds and defaults from grammar nodes, retaining their source text
  with only outer whitespace trimmed. Split a bound list at grammar-defined
  entries, never punctuation inside a nested type. Keep generic arguments,
  qualification, lifetimes and expressions intact.
- Attach inline bounds and separately written bounds on a named parameter
  to that parameter's `constraints`, in source order. A declaration-level
  requirement such as Rust `where T::Item: Copy` belongs in
  `type_info.constraints`, since its subject is not just a declared parameter.
  Do not repeat the same clause at both levels or infer constraints from a
  use of a parameter.
- Record an explicit default without its assignment delimiter. Record
  explicit variance as written (`in`, `out`, `+`, `-`); otherwise use `None`.
  A const/non-type parameter's declared type is not a bound and is not added
  to `constraints`; its source declaration remains available in
  `source.declaration_text`. Packs and higher-kinded parameters retain their
  name and expressible bounds; this feature adds no recursive parameter model.

For example, Rust `struct Bag<T: Clone = String> { value: T }` yields one Type
parameter named `T`, position 0, constraints `["Clone"]`, default `"String"`
and no variance. A method `fn get<U>(&self, value: U)` owns only `U`.

Keep the older generic-name lists available; removing them is not part of this
feature. Prefer a shared syntactic reader with an older-model name projection
where it preserves existing behavior. The `tree-hugger` library's
[record adapters](../../lib/src/shared/schema_v2/mod.rs), which convert between
the older and newer symbol models, cannot currently retain structured generic
parameters, relations, raw attributes or these new modifiers on a round trip.
Document those losses instead of expanding the older model to mirror every
record field.

## Declared type relations

The existing `tree-hugger` library
[SymbolRef](../../lib/src/shared/schema_v2/facets.rs) represents a relation's
written target and optional same-file identity. For `extends` and `implements`:

- Set `name` to the complete written base expression, outer whitespace
  trimmed: `pkg.Base<T>` remains `pkg.Base<T>`, not `Base`. This intentionally
  corrects the shortened Rust trait text. Do not evaluate Python/JavaScript
  base expressions or strip arguments from the stored text.
- Set `id` only when declaration syntax, qualification and lexical scope
  identify exactly one eligible declaration in the same file. A bare name
  may match a unique visible local type; a qualified name must establish its
  path in that file. An imported name, alias requiring expansion, type
  parameter, dynamic expression or unproven external path stays unresolved,
  even if its final segment equals a local type name. Never choose the first
  name match or fall back from failed qualified matching to a bare name.
- A resolved target gets its in-file `qualified_name`; an unresolved target
  leaves both `id` and `qualified_name` absent. The displayed scan-root or
  module prefix must never enter stored relation targets.
- Preserve source order and deduplicate equal target expressions within each
  relation list. Repeated Rust impls aggregate in file order. Do not collapse
  differently written generic instantiations merely because their target ID
  is the same. No inherited members or transitive edges are synthesized.

Use a per-file index of all light declarations for candidate matching, even
when a query selects only one kind. Build it once; do not scan every record
again for each relation. Relation matching is a bounded source lookup, not
compiler-level name resolution. If a path's meaning cannot be proved under
the rules above, preserving its text is a successful result.

### Rust impl ownership

Retain `implements` on each directly declared trait-impl member, and also
aggregate the trait relation on the uniquely resolved implementing type.
The member relation identifies which trait supplied the member; removing it
would lose that fact when a query returns methods alone. Inherent impls add
members but no `implements` relation. Negative impls do not assert conformance
and add no `implements` edge.

Traverse impl declarations once, including empty impls: deriving relations
only from extracted members would miss `impl Marker for Local {}`. Match the
self type with the same conservative rules as base targets, using its full
written path. Bare or same-file-qualified generic self types may match their
base declaration without expanding aliases; reference, tuple and other
composite self types do not attach to a component type.

When Containment is requested, aggregate same-file impl members onto the
resolved type's `relations.members` without overwriting ordinary members or
members from another impl. An unresolved self type retains the member's
existing textual identity container and a textual `relations.container`
whose name is the full self-type expression, without a fabricated parent ID
or local membership. Do not change qualified-name construction or symbol
IDs as a side effect of preserving richer relation text.

**Reader's note:** the draft left duplicate trait relations as an open choice.
Keeping them preserves the established method-level contract while adding
useful type-level summaries. The current containment pass rewrites containers;
its Rust impl path must preserve this resolution result rather than guessing
again from the short container name. The same evidence must survive cache
widening from declarations to containment.

## Per-language acceptance

Every supported language needs separate positive, negative and limitation
cases for each applicable column. The examples below establish required
forms, subject to demonstrated limits of the pinned grammar. Mixed base-list
classification needs the author decision under Open Questions before those
cells can be finalized.

| Language | Explicit generics | Declared relations | Attributes and selected modifier forms |
|---|---|---|---|
| Rust | Type/lifetime/const parameters, defaults and `where` clauses on items. | Trait supertraits → extends; positive trait impls → implements on members and resolved self type. Lifetime bounds are constraints, not base types. | Outer attributes; `unsafe`, `const fn`, binding/receiver `mut`. |
| TypeScript | Type, alias and callable parameters, bounds and defaults. | Class extends → extends; class implements → implements; interface extends → extends. | Decorators; abstract, override and readonly tokens. |
| JavaScript | No declared generic parameters. | Written class extends expression → extends, unresolved if dynamic. | Grammar-supported decorators; no invented TypeScript modifiers. |
| Go | Type and callable type parameters and constraint text. | No inferred implements; embedded struct fields are composition. Interface embedded types → extends only for a named type expression, not a union or `~` constraint term. | No raw attributes or modifiers from the selected new set. |
| Python | Explicit class/function/type-alias parameter syntax where the pinned grammar supports it. | Positional class bases → extends; `metaclass=` is not a base. | Decorators; no inferred abstract/final/override modifier from a decorator. |
| Java | Type and callable parameters and bounds. | Class extends → extends; implements → implements; interface extends → extends. | Annotations; abstract and final tokens. |
| C# | Type and callable parameters, variance and `where` constraints. | Base lists mix inheritance and interface implementation; see Open Questions. | Attribute lists; abstract, override, virtual and readonly tokens; sealed retained as Custom("sealed"), not renamed final. |
| C | No declared generic parameters or inheritance. | None. | No newly required attribute forms; no invented C++ modifiers. |
| C++ | Template type and non-type parameters, defaults and constraints representable by the existing fields. | All declared class/struct base specifiers → extends, including private/virtual bases; access/virtual keywords are not part of the target type text. | Existing `[[…]]` reader; virtual, override and final declaration tokens. |
| Swift | Type/callable parameters and explicit requirements. | Inheritance/conformance lists, including extensions, need classification; see Open Questions. | Attributes; final, override and mutating tokens (the last as Custom("mutating"), not Rust Mut). |
| Scala | Type/callable parameters, variance and bounds. | `extends`/`with` lists mix classes and traits; see Open Questions. | Annotations; abstract, final and override tokens. |
| PHP | No native declared generic parameters in required scope. | Class extends → extends; class/enum implements → implements; interface extends → extends. Trait `use` is composition, not conformance. | Abstract and final tokens; PHP attributes are outside this feature's required attribute set. |
| Perl | None in required scope. | Do not infer inheritance from `@ISA` or `use parent`. | No newly required attribute forms. |
| Bash | None. | None. | None. |
| Zsh | None. | None. | None. |
| Lua | None in required scope. | Do not infer inheritance from metatables. | None. |

"None" describes this feature's supported source forms, not an assertion
that a language can never express an equivalent idea. A missing capture or
reader is an extractor gap, not a grammar limit. For an unsupported form,
record the snippet, pinned grammar identity and observed parse/node limitation
in the implementation log and current documentation. Leave supported forms
covered independently. Do not upgrade grammars or add symbol records solely
to make a metadata cell pass.

## Modifiers and raw attributes

Use existing `tree-hugger` library
[Modifier and Attribute](../../lib/src/shared/schema_v2/facets.rs) values to
report written declaration facts. Modifiers use Const, Unsafe, Readonly and
Mut where their meanings match; use exact lowercase Custom values for
`abstract`, `final`, `override` and `virtual`, plus the spellings specified
in the language table. Keep existing Static, StaticStorage and Async behavior.
Do not turn Java final or C++ object const into Const: Const here covers
explicit const-callable syntax. Readonly covers explicit declaration readonly
syntax, and Mut covers Rust mutable bindings/receivers. Extending those
meanings needs a separate decision. A modifier inside a body or on a nested
parameter never marks the enclosing callable.

Read declaration-owned tokens through the established shared declaration and
wrapper helpers, including templates and decorated declarations. Do not
search arbitrary source text for keywords. A wrapper's facts attach only to
its declarations, not every descendant. Deduplicate modifiers and serialize
in a fixed order: Static, StaticStorage, Async, Const, Readonly, Unsafe, Mut,
then distinct Custom values in lexical order. Existing modifier sets must
remain intact.

Reuse the `tree-hugger` library's
[declaration_attributes](../../lib/src/file/extract/attributes.rs), which reads
syntactic attributes for flags, to populate raw `attributes` on the record;
add TS/JS decorators where the pinned grammar supports them. Include C++ and
Scala forms already read by this helper. Share parsed attributes with the
flag readers within an analysis rather than parsing them twice.

For `#[tokio::test]`, emit `{name: "tokio::test"}`. For
`@Deprecated(since = "9")`, emit `{name: "Deprecated", value: 'since = "9"'}`.
The name excludes the attribute/decorator marker and compacts whitespace in
syntactic paths. The value is the raw argument text without outer delimiters,
trimmed; no arguments means `None`, while explicit empty parentheses mean
`Some("")`. Preserve attribute order and repetitions, qualification, literal
whitespace within arguments, and list entries individually. Rust `cfg_attr`
remains one raw attribute, without expansion. Inner/file/assembly attributes
and doc comments do not become attributes on every declaration.

For an unusual decorator expression that is not a path or a call of a path,
preserve its expression text as the name with no argument splitting; never
compact whitespace inside literals. This does not infer test/deprecation
flags from that expression. Adding TS/JS raw decorators alone does not change
the existing flag recognition table.

## Callable body-line metric

Extend the `tree-hugger` library's
[MetricsFacet](../../lib/src/shared/schema_v2/facets.rs), the measurements
included only when Metrics is requested, with `lines: Option<u32>`. Like the
other metrics, the key is required inside a present facet and serializes
`None` as `null`; the entire facet stays absent when unrequested.

Count physical lines intersecting the half-open `source.body_span` of a
Function, Method or Constructor. Other kinds have `lines = None` even if
they have a class/module body span. A callable without an implementation body
has `None`, not zero; an empty byte span has `Some(0)`.

For a nonempty span, with one-based locations:

`lines = end.line - start.line + (end.column > 1 ? 1 : 0)`.

Thus `{}` on one line counts 1; a span starting on line 3 and ending at line
6, column 1 counts 3. Braces, blank lines, comments and nested declarations
inside the span all count. CRLF and LF versions of the same source count the
same. This is a source-size measurement, not executable-line counting.

Use the exact same body node for the stored span and this metric. The
`tree-hugger` library's
[body node reader](../../lib/src/file/extract/classify.rs) currently looks for
block bodies; extend its shared logic for grammar-supported expression bodies
and transparent wrappers without mistaking a default value, type expression,
returned callback body or nested function for the callable's own body.
An expression body counts its expression's span, excluding an introducer such
as `=>` or `=`. Abstract/interface declarations and defaulted/deleted C++
constructors without a body stay `None`.

God-files' declaration-span size/risk calculation remains unchanged. It uses
a different unit; do not replace it with callable body lines. Preserve its
existing score regressions.

## Selective queries, cache and schema

The `tree-hugger` library's
[Capability](../../lib/src/file/query.rs) controls optional analysis. Keep
this feature within existing capabilities:

- Declarations supplies written generics, modifiers, raw attributes and
  extends/implements facts, including conservative target IDs. These fields
  remain available on a declarations-only query. Only selected records need
  full metadata; unselected records still supply light identity and scope
  evidence for resolution. No new capability is introduced.
- Containment supplies impl member aggregation and parent/container links.
  It must use the same conservative self-type evidence, including when
  widened from cached records. Persist enough evidence to do that correctly;
  if the existing record shape cannot retain it, document that limitation
  and reparse once on that widening path instead of making up a target.
- Metrics supplies all three measurements together. Widening a cache entry
  by Metrics reparses once, as it already does. Narrowing removes the entire
  facet when Metrics is not requested.

Extend the existing planner and cache projection in the `tree-hugger` library's
[coverage helpers](../../lib/src/analysis/coverage.rs), which make cached
results agree with uncached queries. Keep one definitions query and at most
one parse per analysis; do not run references, semantic analysis or diagnostics
just to obtain these source facts. Inspect impls and build lookup indexes once
per file instead of adding per-symbol whole-file walks.

**Schema decision:** filling already existing fields and using Custom modifier
values does not change the record shape. Adding the `lines` key does: follow
the repository's minor-version rule and emit 4.1 if 4.0 is still current when
this lands, otherwise the next minor version of the current major. Update
records, file indexes, CLI envelopes and fingerprints consistently. The area
[cache design](../../docs/cache-design.md) requires exact current versions on
both entries and every record; older versions are recomputed, never migrated.
Bump the library's [EXTRACTOR_REVISION](../../lib/src/lib.rs), the cache
invalidation revision for extracted facts, as well. Preserve symbol IDs:
schema/revision changes are not inputs to symbol identity.

**Reader's note:** earlier additive fields were shipped under 4.0 despite the
minor-version rule. This feature follows the documented rule rather than
repeating that exception. Update version-sensitive tests that currently treat
4.1 as incompatible, and replace that example with a genuinely different
version. Strict metrics deserialization must reject a missing or wrongly
typed `lines` key; do not default an old incomplete metrics object to a
computed null. Record-only older-model adapters remain explicitly lossy.

## Outline output

For `hug types --tree` and `hug symbols --tree`, append declared `extends`
and `implements` lists to a visible type container row, using the recorded
written targets in source order. For example, a class `Child` with these
relations displays `extends Base<T>; implements I`. Apply this even to a
container with no displayed children. A method's retained Rust trait relation
is not presented as the method's own inheritance heading.

Keep existing selector behavior: do not add a filtered-out container or target
merely to show a relation. `hug functions --tree` gains no synthetic type row.
Keep relations visible with `--signatures none`; with full/short signatures,
avoid repeating an inheritance clause already in the header. Preserve
wrapping, sorting and plain output rules using biscuit-terminal renderable
components. JSON/JSONL/full-index output exposes the same record facts and
metrics through existing formats. Counts and statistics gain no new category.
No additional class-summary, table or flat-list decoration is required.

## Validation, documentation and completion

Extend existing fixture/matrix tests in the `tree-hugger` library, including
its [metrics matrix](../../lib/tests/l1/symbols_metrics.rs),
[selective-query tests](../../lib/tests/l1/symbols_query.rs),
[cache projection tests](../../lib/tests/l1/symbols_cache.rs) and
[adapter tests](../../lib/tests/l1/symbols_adapter.rs). Add generics/relations
and modifiers/attributes matrices under the same framework and extend the
[coverage guard](../../lib/tests/l1/symbols_coverage.rs) to require a row for
all 16 languages. Each row distinguishes supported forms, absent language
constructs and demonstrated grammar limits.

Required cases include generic bounds/defaults and declaration ownership;
qualified external targets sharing a local short name; duplicate names in
different scopes; ambiguous and alias targets; multiple and empty Rust impls;
negative impls; full trait/self text and method-only queries; mixed base lists
after the author's decision; attribute order, repetitions and empty arguments;
modifier keywords appearing only in a body/string; and body spans with LF,
CRLF, end-at-column-1, expression bodies, nested callables and absent bodies.

Compare narrow queries against complete results under matching capabilities.
Prove declarations-only analysis skips optional passes, and check work counters
for accidental repeated queries/scans. Test cache widening, projection,
invalidation, required metric keys and an actual warm CacheLookup::Hit, not
just equal output after a silent recomputation. Extend CLI JSON and outline
assertions and update affected golden output only for intended changes.

For implementation, run `just test` and `just lint` in `tree-hugger/`.
Rendering changes use existing real-terminal tests through `just test-l2` and
the shared test toolkit without focusing terminal/browser windows. Existing
macOS, Linux, native Windows and WSL2 requirements remain; this feature adds
no CI environments or gates. No new performance spike is required for source
readers using the existing parse. Counter-based checks address the concrete
risk of repeated work; a proposed extra whole-file scan must be justified
before expanding performance measurement.

Update affected area/library/CLI READMEs, current schema, cache and report
pages, API documentation and the tree-hugger skill. Document unsupported forms
and adapter losses in current docs. Docs describe behavior directly and never
link to or name this feature. Review comments on changed readers, particularly
those saying attributes are read only for flags or that metrics have only two
members. Record accepted decisions and departures in the implementation log.

Implementation is complete, ready for review, when every required form is
supported or has a demonstrated grammar limit, query/cache equivalence is
verified, and affected reports/docs agree. An unfinished extractor is not a
grammar limit. Leave the feature directory active; the author closes it after
review.

## Open Questions

### How should mixed inheritance and conformance lists be recorded?

C#, Swift and Scala can write classes and interfaces/protocols/traits in the
same syntactic list. For an external target, source syntax may not distinguish
inheritance from conformance. Guessing from position or spelling can put a
relation in the wrong field; resolving every external type would exceed this
feature's same-file boundary. Swift extensions also need to attach conformance
to an existing type without creating another type record.

- **Use extends for written base lists, except entries proved to be
  conformance (recommended).** Pros: preserves every written target using the
  existing schema; needs no compiler or cross-file lookup. Put a target in
  implements only when same-file declaration syntax establishes an interface,
  protocol or trait and the owner is a concrete implementing type. Interface,
  protocol and trait inheritance remains extends. Swift extension entries are
  syntactically conformance and use implements on a uniquely resolved local
  self type. Cons: extends means a written base relation for unresolved mixed
  entries, not necessarily class inheritance; consumers must understand this
  broader definition, and an external extension owner has no record on which
  to attach its relation.
- **Add a separate unclassified-base relation field.** Pros: extends and
  implements retain precise meanings without losing external base text.
  Cons: adds a schema field, another report label and consumer handling; it
  needs an explicit minor-version addition and cache coverage.
- **Emit only entries whose category is provable.** Pros: existing fields
  remain precise and no schema change is needed. Cons: discards common
  external base/conformance facts and undermines the goal of preserving written
  targets. The support docs would have to describe that loss prominently.

Recommend the first option because the feature is intended to expose written
source facts using existing fields. If the author requires extends to mean
class inheritance exclusively, choose the second option. Before implementing
these rows, record the accepted meanings with same-file and external examples,
including Swift struct/protocol/class/extension cases and Scala trait bases.
For an unresolved extension owner, the recommended option documents that the
relation cannot be attached without an extension record; it does not attach
it to a same-named unrelated type. Recommendations here do not become required
design until the author accepts them.
