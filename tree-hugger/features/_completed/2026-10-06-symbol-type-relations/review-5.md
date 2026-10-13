---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation lookup loses binding evidence between syntax scopes and display-name indexes
    priority: high
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T00:45:08-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-5.md
previous: 2026-10-06-symbol-type-relations/review-4.md
next: 2026-10-06-symbol-type-relations/review-6.md
---

# Review of Symbol Type Relations — Iteration 5

The feature is **not production ready**. The cases specifically listed in
review 4 have regression coverage and pass, including its five new cache
member columns. A further sweep finds one recurring defect class: relation
lookup loses binding evidence when it transfers between syntax scopes and
indexes keyed by displayed names. This creates false target identities,
omits provable identities, changes mixed-base classification, and loses Rust
impl ownership. All remaining work is actionable without a human decision.

## Previous findings and blocked work

Read the specification, implementation and repair logs, and reviews 1–4.
The assessment below distinguishes repairs to the listed cases from completion
of the whole defect class.

| Earlier finding | Assessment |
|---|---|
| Review 4: Relation lookup still substitutes record names for source binding evidence | Its 83 added path-matrix rows, including the 69 reported defect rows and 14 controls, pass. The new lexical index fixes the listed block, position, name-group, pattern, anonymous-owner, import and extension cases. The transitions out of that index remain defective in the instances below. |
| Review 4: Attribute and modifier members lack permanent cache robustness columns | Implemented. All five columns now use populated Java extraction controls, nine shapes per column, and a public-output sentinel that distinguishes service from recomputation. All 45 cells pass. |
| Reviews 1–3: Previously listed path roots, Python nested classes, Scala imports, generic-list overflow, built-in spellings and Rust ownership | Their registered regressions pass. They do not cover the additional qualified-owner, binding-transfer and directive-write cases below. |
| Review 1: Template-template defaults and callable requirements | The class/function/alias/method/constructor default cases and wrapped/unwrapped callable requirements remain repaired. Their complete, selective and cache tests pass. |
| Review 3: C# record superclass classification and current-version documentation | Repaired. Record-class/record-struct controls pass; current examples agree with schema 5.0 and extractor revision 32. |
| Reviews 2–3: Generic/base cache columns and nested relation-span columns | Implemented. Their registered format-wide matrices pass, including each of the six span members. |

Review 4 had no blocked findings. The blocked C#, Swift and Scala work from
reviews 1–2 was unblocked before its follow-up implementation: the author
accepted precise `extends`/`implements`, a separate `unclassified_bases`
list, and Swift extension conformance attached only to a uniquely resolved
same-file owner. The three readers implement that accepted design. No
previous decision needs reaffirmation.

## Verification and scope

Used the tree-hugger and rust-testing skills, with the Rust skill for code
review and the terminal/harness skills for rendering verification. Reviewed
the 16 language acceptance rows, structured generics, raw attributes and
modifiers, body spans and line counts, declared bases, Rust impl ownership,
Swift extensions, selective analysis, cache decoding/projection/widening,
outline rendering, documentation, and test registration and tier selection.
This review changes only the requested review documents and spec metadata.

- `just test`: **1,210 passed, zero skipped**, 57.965 seconds.
- `just lint`: passed for both packages.
- `just test-l2`: six selected entry points passed, 23.136 seconds. Declaration
  rendering actually ran in tmux and WezTerm. Kitty's entry points returned
  through the unavailable-backend gate and provide no Kitty rendering evidence.
  The 331 tests excluded by this tier's selection are not missing L2 tests.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- Supplemental checks copied the existing language fixtures into isolated
  temporary directories and appended one controlled source variation per case.
  Compared `hug --json-full symbols --no-cache`, declarations from
  `hug --json types --all-scopes --no-cache`, a complete cache-writing run,
  and an actual complete warm hit. The failing relations agree across these
  paths; warm runs report `1 hits` and `parsed 0 files`. All reproductions
  reported in the findings parse without error diagnostics. Malformed exploratory
  snippets were discarded rather than used as evidence.
- Rust self/trait cases were also checked with member-only `functions` output,
  declarations-to-containment widening via `symbols --tree --all-scopes`, and
  subsequent complete warm output. The same missing ownership persists through
  these consumers.
- Standalone Rust examples for const-name separation, impl-parameter scope
  and local-module paths were accepted by `rustc`. The C++ and Swift
  same-function/sibling-owner positive examples were accepted by `clang++`
  and `swiftc`. No Java, C# or Scala compiler proof is claimed.

Concurrent reference-occurrence functionality is present in this worktree;
its public behavior is outside this feature's review. The current full suite
includes it. Cross-OS execution evidence is left to CI and does not affect
the readiness decision.

## Unblocked Findings

### High — Relation lookup loses binding evidence between syntax scopes and display-name indexes

**Defect class:** lookup establishes or redirects a binding in a syntax scope,
then discards its owner identity, scope or type/value category when using a
display-name index, so the later decision no longer proves the target's meaning.

The tree-hugger library's
[Declarations::resolve](../../lib/src/file/tree_file/relations.rs:575), which
decides target identities for bases and Rust impls, now correctly asks the
lexical index about the first segment. However, it then retains only that
declaration's qualified-name string. Later segments use
[Declarations::named](../../lib/src/file/tree_file/relations.rs:447), which
combines members of distinct local owners that have that same display name.
The selected owner's syntax identity is lost.

The tree-hugger library's
[Lexical::place](../../lib/src/file/tree_file/relations/lexical.rs:245) exports
unrecorded named-scope bindings as bare name nodes. The receiving
[Declarations::walk](../../lib/src/file/tree_file/relations.rs:398) assigns
each one all name categories and locates it by the nearest recorded owner.
This loses a destructured value's category and an unrecorded impl's own
parameter scope. The directive paths in
[Lexical::lookup](../../lib/src/file/tree_file/relations/lexical.rs:395) similarly
redirect reads without redirecting the assignments and imports that change
the destination binding.

These are observable record defects. A false ID makes consumers attribute
inheritance to an unrelated type; a missing ID can erase Rust conformance or
membership and changes a provable Swift/Scala superclass into an unclassified
base. A warm hit faithfully preserves those incorrect results.

#### Qualified-owner identity and member reachability

Append this source to a copy of the C++ fixture and run the complete CLI:

```cpp
void reviewProbe() {
    { struct ReviewHolder { struct ReviewBase {}; }; }
    { struct ReviewHolder {};
      struct ReviewChild : ReviewHolder::ReviewBase {}; }
}
```

The relation names `reviewProbe::ReviewHolder::ReviewBase` **with the first
block's Base ID**, although the second block's selected Holder has no such
member. Preserve the written target without an ID instead. If both Holders
declare a Base, lookup reports text only even though the selected Holder has
exactly one Base. Both errors come from combining members by display name.

For each applicable reader, tested: one owner; two sibling owners each with
the member; a sibling with the member and a selected owner without it; and a
three-segment path through another nested owner. The three-segment sweep also
used a differently named sibling as a control. “Selected member ID” means
the member of the owner visible at the relation, identified by its declaration
position as well as its ID. “Text only” means absent `id` and `qualified_name`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ local class/struct bases | Two-segment and three-segment paths, both same-named sibling owners have the member | Text only | Selected member ID in `extends` |
| C++ local class/struct bases | Selected owner lacks the member; sibling has it, at either path depth | Sibling member ID | Text only |
| Java local-class superclasses | Both owners have the member, at both depths | Text only | Selected member ID in `extends` |
| Java local-class superclasses | Selected owner lacks the member, at both depths | Sibling member ID | Text only |
| Swift local-class bases | Both owners have the member, at both depths | Unclassified text | Selected class ID in `extends` |
| Swift local-class bases | Selected owner lacks the member, at both depths | Sibling class ID in `extends` | Unclassified text, no ID |
| Scala local-object type paths | Two-segment path; both objects have the class | Unclassified text | Selected class ID in `extends` |
| Scala local-object type paths | Two-segment path; selected object lacks the class | Sibling class ID in `extends` | Unclassified text, no ID |
| Scala local-object type paths | Three-segment `reviewHolder.reviewMiddle.ReviewBase`, including one-owner and differently named sibling controls | Unclassified text in every case | Selected class ID when the selected owner contains that chain; text only when it does not |
| Rust local-module paths | One owner, repeated sibling owners, and deeper nested modules | Text only, including the one-owner control | Selected trait ID when its module contains the path; text only for a missing member |
| Rust paths inside local modules | `self::ReviewBase`; nested `super::ReviewBase` | Text only | Trait ID in that local module |
| Rust local-module controls | Bare `ReviewBase` inside its own module; `crate::ReviewBase` naming a file-level trait | Correct trait IDs | Observed result; clean |
| C++, Java, Swift controls | One local owner; differently named sibling; both path depths | Correct selected member IDs | Observed result; clean |
| Python class bases | One local owner; competing owner in a separate nested function; selected owner with and without the nested class | Correct selected ID / text only | Observed result; separate function names already distinguish these owners; Python suites do not create equivalent sibling scopes |
| TypeScript and JavaScript class bases | Qualified local-class fields holding class expressions, with one owner and sibling owners | Text only | Observed conservative result; turning a field initializer into a named type target would require inference outside this feature |
| Go interface embeds | Named local interface and shadowing controls | Correct local ID or text only | Observed result; Go has no nested member-type declarations equivalent to these class/module paths |
| C# bases | Existing namespace, nested-type and rooted-path controls | Correct local IDs | Observed result; C# has no equivalent function-local class declaration |
| PHP bases | Namespace qualification and function-name controls | Correct namespace target or text only | Those controls are clean; function-declared class names have the separate defect below |
| C, Perl, Bash, Zsh, Lua | Original fixtures and unsupported-inheritance controls | No required base edges invented | Observed result; no applicable base reader |

Rust's local-module case is not an ambiguity or a grammar limit:

```rust
fn reviewProbe() {
    mod reviewHolder { pub trait ReviewBase {} }
    trait ReviewChild: reviewHolder::ReviewBase {}
}
```

It compiles, but lookup drops the local module's child because its placement
is `Local`, while qualified continuation accepts only `Qualified` or `Member`.
Rust module members remain reachable through the module's path according to
the [Rust scope rules](https://doc.rust-lang.org/reference/names/scopes.html).
Fix member reachability using syntax-owner identity, rather than marking every
lexical declaration globally reachable.

#### Unrecorded binding scope and type/value category

The first reproduction below compiles. A parameter of one impl must not hide
the file-level trait from a different declaration:

```rust
trait ReviewBase {}
struct ReviewHolder<T>(T);
impl<ReviewBase> ReviewHolder<ReviewBase> {}
trait ReviewChild: ReviewBase {}
```

`ReviewChild.extends[0]` now has no ID. The impl parameter is collected as an
unrecorded binding and placed at the file root because the impl has no record.
This also happens when Child precedes the impl, and inside a named module.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust named impl parameters | Type parameter named ReviewBase; Child before and after the impl, at file root and inside a module | Text only in all four cases | The trait's ID; the parameter's scope ends at its impl |
| Rust named impl parameters | Const parameter named ReviewBase; same before/after and root/module variations, using `ReviewHolder<{ReviewBase}>` | Text only in all four cases | The trait's ID; also a distinct value category |
| Rust named impl parameters | Parameter named like a module qualifier; unrelated Child uses `reviewHolder::ReviewBase` | Text only | Module's trait ID |
| Rust lexical impl controls | The equivalent type/const impl and Child are inside a function | Correct local trait ID before and after the impl | Observed result; lexical transfer preserves the impl boundary |
| Rust named trait and struct owners | Const generic named ReviewBase; a member body declares `trait ReviewChild: ReviewBase` | Text only | File-level trait ID; a const parameter is a value |
| Rust const controls | Const parameter on Child itself or on its enclosing function | Correct file-level trait ID | Observed result; clean |
| Rust type-parameter control | Actual type parameter on an enclosing owner | Text only | Observed result; type shadowing still counts |
| TypeScript module-level type bases | Interface ReviewBase plus a value bound by flat array, nested array or nested object destructuring; Child implements ReviewBase | Text only | Interface ID in `implements`; the destructuring binds a value |
| TypeScript module-level controls | Flat shorthand object binding with a public record; nested pattern binding a different name | Correct interface ID | Observed result; clean |
| TypeScript lexical controls | The same nested object/array patterns inside a function | Correct interface ID | Observed result; lexical bindings retain VALUE |
| Scala type bases in named scopes | `class reviewBase`; tuple, nested tuple or unapply `val` binds the value `reviewBase`, at package level or in an object | Unclassified text in all six cases | Class ID in `extends`; term binding does not hide the type |
| Scala controls | Simple recorded `val` at package/object level; every pattern in a function | Correct class ID | Observed result; clean |
| C++ template/member controls | Enclosing built-in non-type parameter shares the base type's spelling | Correct struct ID | Observed result; clean |
| Java, C#, Swift, PHP controls | Same-named field/value/function beside an eligible type, using each language's accepted name groups | Correct base ID and category | Observed result; clean |
| JavaScript and Python controls | Unrelated named-scope destructuring; lexical parameters/patterns that actually bind the base name | Correct outer ID / text only | Observed result; one relevant value group, so a binding really hiding the base must remain unresolved |
| Go controls | Generic parameter, parameter, multi-name assignment or range binding hides an interface; unrelated binding does not | Text only / correct interface ID | Observed result; clean |
| C, Perl, Bash, Zsh, Lua | No applicable declared-base consumer | No new relation | Observed result; not applicable |

A minimal TypeScript case is:

```typescript
interface ReviewBase {}
const { x: { ReviewBase } } = external;
class ReviewChild implements ReviewBase {}
```

TypeScript distinguishes type and value declarations; see its
[declaration-group documentation](https://www.typescriptlang.org/docs/handbook/declaration-merging.html).
The same principle already drives the successful lexical controls.
Unrecorded bindings must carry their original categories and owning syntax
scope through collection. Converting them into all-category imports cannot
preserve that information.

#### Python directive destinations and writes

```python
class ReviewBase:
    pass
def reviewProbe(external):
    global ReviewBase
    ReviewBase = external
    class ReviewChild(ReviewBase):
        pass
```

The complete and selective results identify the original module class as the
base, even though its binding has been replaced with an unknown value.
Preserve `ReviewBase` without an ID. The same failure occurs with `nonlocal`
and an enclosing function's class. No expression evaluation is needed to know
that the old class binding is no longer proved.

The sweep tested every supported write family against **both** directives.
The outer class provides the positive control. For `nonlocal`, that class is
inside an enclosing function; for `global`, it is at module level.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Python global/nonlocal assignment | Simple assignment, nested tuple, nested list, starred target and augmented assignment | Original destination class ID for every shape, under both directives | Text only; the destination binding is changed |
| Python global/nonlocal imports | `from external import ReviewBase` | Original destination class ID under both directives | Text only; imported targets are unresolved |
| Python global/nonlocal declarations | Same-named class or function declared after the directive | Original destination class ID under both directives | Never the stale class ID; conservatively text only when the destination has competing definitions, and no function-as-type inference |
| Python global/nonlocal loop/context patterns | Nested `for` target; nested `with ... as` target | Original destination class ID under both directives | Text only |
| Python global/nonlocal exception and match patterns | `except ... as ReviewBase`; dictionary match capture ReviewBase, with Child inside that region | Original destination class ID under both directives | Text only |
| Python global/nonlocal walrus | `(ReviewBase := external)` before Child | Original destination class ID under both directives | Text only |
| Python directive controls | Directive plus read only; an unrelated assignment | Correct destination class ID | Observed result; preserve these cases |
| Python ordinary binding controls | Equivalent lexical assignment/pattern/import without a directive | Text only | Existing registered controls remain clean; redirecting a read must not bypass these bindings |
| Other ten base languages | Block, generic and import controls in their respective matrices | Results as listed in the preceding sweeps | No equivalent Python directive reader; do not apply Python rules to them |

Python's directives govern bindings as well as lookup; see the
[Python execution model](https://docs.python.org/3/reference/executionmodel.html).
Store directive destinations and apply them when placing writes/imports,
or conservatively mark the destination unproved when such a write is present.
Merely returning `Local::Global` or skipping a nonlocal-writing scope loses
the evidence that must invalidate the earlier declaration.

#### PHP class binding namespaces

The tree-hugger library's
[Declarations::site](../../lib/src/file/tree_file/relations.rs:690) correctly
looks PHP class names up in the namespace, even from inside a function. Its
candidate index still places a class declared inside that function under the
function's **display** prefix:

```php
namespace Example\Symbols;
function reviewProbe() {
    class ReviewBase {}
    class ReviewChild extends ReviewBase {}
}
```

The declaration record for Base is named
`Example\Symbols\reviewProbe::ReviewBase`; the base stays unresolved.
The two declarations belong to the same PHP class namespace, so Base's
unique record is a provable target. Retain the existing display name and ID;
index the binding separately under its actual namespace.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| PHP class bases | Base and Child both declared in a function | Text only | That Base record's ID and existing in-file qualified name |
| PHP class bases | Base declared in a function, Child in the enclosing namespace | Text only | Same-file Base ID under the namespace lookup; its function display prefix must not make a different class name |
| PHP rooted class bases | Same Base, Child writes `\Example\Symbols\ReviewBase` | Text only | Same-file Base ID |
| PHP conditional control | Base and Child in an `if` suite in the namespace | Correct Base ID | Observed result; clean |
| PHP existing repaired control | Namespace-level Base, Child declared in a function | Correct namespace Base ID | Observed result; clean |
| Other base-language callable controls | Rust/C++/Java/Swift/Scala lexical declarations, Python callable boundaries, TS/JS local classes, Go local interfaces; C# namespace declarations | Language-specific results in the preceding tables and passing registered cases | Keep their own scope rules; PHP's function display prefix is not a class namespace |

This is the inverse of review 4's repaired PHP lookup-site case. Both the
lookup site and the candidate binding must use the PHP namespace. The
[PHP namespace documentation](https://www.php.net/manual/en/language.namespaces.faq.php)
describes namespace membership for classes declared inside functions.

#### Consumer, cache and repair sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Complete and type-only base output | Every failing source row above | Same false or missing target/category | Correct decision on both paths |
| Complete persistent-cache hit | The same copied sources after a cache-writing run | Actual hits, zero parses, same defective facts | Correct facts retained on a hit |
| Rust impl self lookup | `impl ReviewTrait for reviewHolder::ReviewSelf`, with that module and type declared inside the function | Textual owner only; no type-level conformance or impl-member aggregation; member container kind absent | Local type ID, Type container kind, conformance and membership |
| Rust impl trait lookup | `impl reviewHolder::ReviewTrait for ReviewSelf`, with trait inside the local module | Owner resolves, but type/member trait relations lack the trait ID | Local trait ID in both retained relations |
| Rust impl self and trait together | Both paths go through the same local module | Both unresolved | Resolved self/trait IDs and ordinary ownership |
| Rust impl under a named const-generic trait | Const parameter shares the file-level self type's spelling; empty impl inside a member | Self type receives no conformance | File-level self type's conformance; const parameter cannot hide that type |
| Rust member-only output | The local-module impl cases | Same absent container kind or trait ID | Same correct decisions without requesting Containment |
| Rust containment widening and outline | Member declarations first, then cached `symbols --tree --all-scopes`, followed by full-index hit | Widening runs, but unresolved local owner stays under a textual external-owner heading; full hit retains absent parent/membership | Reparse using correct evidence, attach members to the local type, retain that result on the hit |
| Rust direct-local controls | Struct, trait and impl directly in a function, without a module qualifier; review-4 sibling-block/value-name cases | Correct owner, trait and member relations | Observed result; clean |
| Swift extension owner and conformance consumers | Registered unique/ambiguous/external/alias owners, generic-owner test, signature modes and real cache tests | Passing expected attachment/classification | Clean for these forms; keep this consumer in the shared resolver regression set |
| Built-in filters | Registered Go 19-spelling and Swift 15-spelling block/import/category controls | Passing expected exclusion or retained text | Clean; binding-transfer repair must also preserve `may_bind` behavior |
| Generic/modifier/attribute/line projections | Registered complete/selective/warm/narrowing/widening checks | Pass | No separate finding |

**Required repair:** keep binding evidence intact across every lookup stage.
Qualified continuation must stay within the selected syntax owner's subtree,
including deeper local modules/objects, rather than searching every record
with a matching display name. Unrecorded names must retain their scope and
categories; parameters of an owner without a record must still remain scoped
to that owner. Python directive writes must affect their destination bindings.
PHP candidate bindings must use class namespaces independently of display
names. Keep conservative unresolved output where the proof ends; preserve
existing symbol IDs and qualified-name construction.

Extend the tree-hugger library's
[PATH_MEANING_MATRIX](../../lib/tests/l1/symbols_relations.rs:817) with all failing
rows and clean controls above. Assert the exact selected target **ID**, using
source position to distinguish equal display names, not only qualified-name
text. Reuse those rows in complete/selective/actual-hit and projection tests.
Extend the CLI's [outline/cache ownership tests](../../cli/tests/symbols/outline.rs:354)
for local-module impl self and trait lookup, member-only container kind,
aggregation and containment widening. Equality between two wrong results is
not an oracle.

Preserve one definitions query, a bounded per-file index/walk, and the existing
work-counter checks. A syntax-owner index is sufficient; neither compiler
integration nor new public kinds are needed. Bump the extractor revision so
previous incorrect entries cannot be served. Review behavior-linked comments,
the current [type-relation documentation](../../docs/type-relations.md), and
the skill alongside the repair: their blanket claims that value names never
hide type bases and that Python directives select the correct binding are
currently stronger than the implementation.

## Blocked Findings

None. The remaining finding fits the accepted scope and requires no human
choice, external resolver or cross-OS proof to repair.

## Recurrence

This high finding repeats the class of:

- Review 1, **Relation IDs are assigned without proving path meaning**:
  qualified text is still substituted for the identity of the selected owner.
- Review 2, **Python nested class scopes still assign incorrect base identities**:
  namespace/scope membership is still lost at some lookup transitions.
- Review 3, **Relation lookup still drops import and generic-shadowing evidence**:
  unrecorded parameters and imported bindings still lose information when
  transferred; this time the scope and category are lost after collection.
- Review 4, **Relation lookup still substitutes record names for source binding
  evidence**: its lexical first-segment fix leaves subsequent member lookup,
  named-scope binding transfer and directive writes using incomplete evidence.

Those fixes should have swept complete path continuation through each selected
local owner, not only first-segment binding; recorded versus unrecorded values
in both lexical and named scopes; generic owners without records; reads and
writes at directive destinations; and both lookup-site and candidate PHP
namespaces. The implementation repaired the enumerated review-4 rows but did
not cover these sibling transitions. This review carries their complete tested
instance list, including the Rust impl, cache and extension consumers.
`recurrence` is therefore true.

The earlier compound-cache-field testing class is **closed for the fields
introduced by this feature**: the missing attribute/modifier members now have
permanent columns, alongside the generic, target, span and metric columns.
No additional cache decoder or matrix-coverage finding was identified.

## Input Robustness Matrix

JSON persistent cache is the only affected serialized input format. This
feature changes no manifest, lockfile or configuration reader; there is no
YAML/TOML input side. The registered CLI tests start from real fixture output,
edit one field per cell, retain a positive control and service sentinel, and
assert public output or equality with fresh extraction after rejection.

Below, **S** means the edited entry is served with the stated value;
**R** means it is rejected and recomputed. “Control” means its populated
original value. Wrong-type scalar cells use the opposite scalar type; numeric
fields use a string. The one/all-element shapes use arrays for scalar fields.
Empty means `[]` for lists, `""` for strings, `{}` for objects, or the explicit
zero control where shown. These are the cache format's documented outcomes,
not a claim that malformed source programs have these shapes.

### Collections populated by this feature

| Shape | generic_params | declaration constraints | extends | implements | unclassified_bases | attributes | modifiers |
|---|---|---|---|---|---|---|---|
| Unedited control | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) |
| Absent | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Explicit null | R | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R | R |
| Empty | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R | R |

Absent lists are explicitly the format's encoding for empty lists; explicit
null is rejected. This default is documented and independently asserted.

### Every generic-parameter member

| Shape | name | kind | constraints | default | variance | position |
|---|---|---|---|---|---|---|
| Unedited control | S(T) | S(Type) | S([MatrixBase]) | S(MatrixBase) | S(out) | S(0) |
| Absent | R | R | S([]) | S(None) | S(None) | R |
| Explicit null | R | R | R | S(None) | S(None) | R |
| Wrong whole type | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R |
| Empty | S("") | R | S([]) | S("") | S("") | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R |

### Every relation-target member

| Shape | name | id | qualified_name | source_span |
|---|---|---|---|---|
| Unedited control | S(MatrixBase) | S(matrix-id) | S(MatrixBase) | S(control span) |
| Absent | R | S(None) | S(None) | S(None) |
| Explicit null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| Wrong one element | R | R | R | R |
| Wrong every element | R | R | R | R |
| Empty | S("") | S("") | S("") | R |
| Duplicate key | R | R | R | R |
| Trailing invalid document content | R | R | R | R |

### Every relation-span member

| Shape | start.line | start.column | end.line | end.column | start_byte | end_byte |
|---|---|---|---|---|---|---|
| Unedited control | S(2) | S(3) | S(4) | S(5) | S(6) | S(7) |
| Absent | R | R | R | R | R | R |
| Explicit null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| Wrong one element | R | R | R | R | R | R |
| Wrong every element | R | R | R | R | R | R |
| Empty/zero control | S(0) | S(0) | S(0) | S(0) | S(0) | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R | R |

These cells exercise the unsigned-integer decoder contract. They do not claim
that an arbitrary edited span is semantically valid source evidence. The same
span decoder serves `source.body_span`; relation lists and container/member
references share the target decoder.

### Attribute/modifier member repair from review 4

| Shape | Attribute.name | Attribute.value | Modifier.type — Custom | Modifier.Custom.value | Modifier.type — Static |
|---|---|---|---|---|---|
| Unedited control | S(ReviewAttr) | S(`"raw value"`) | S(Custom) | S(final) | S(Static) |
| Absent | R | S(None) | R | R | R |
| Explicit null | R | S(None) | R | R | R |
| Wrong whole type | R | R | R | R | R |
| Wrong one element | R | R | R | R | R |
| Wrong every element | R | R | R | R | R |
| Empty string | S("") | S("") | R | S("") | R |
| Duplicate key | R | R | R | R | R |
| Trailing invalid document content | R | R | R | R | R |

All 45 cells are now permanent in the tree-hugger CLI's
[member matrix](../../cli/tests/symbols/cache_robustness.rs:921), including
unedited extraction controls for a nonempty raw argument, a Custom payload and
a Static tag. Absent/null optional attribute values are a documented None
contract, tested separately. No decoder defect was observed.

### Metrics inside a requested facet

| Shape | whole facet | parameters | has_docs | lines |
|---|---|---|---|---|
| Unedited control | S(control object) | S(1) | S(false) | S(3) |
| Absent | R | R | R | R |
| Explicit null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| Wrong one element | R | R | R | R |
| Wrong every element | R | R | R | R |
| Empty | R | R | R | S(0) |
| Duplicate key | R | R | R | R |
| Trailing invalid document content | R | R | R | R |

Facet absence when Metrics is unrequested is separately verified by cache
projection. The concurrent `cyclomatic` member also has a required-key column,
but its counting rules are outside this feature. Existing entry/record version,
fingerprint, coverage and reference-evidence matrices passed. Current schema
and revision invalidation tests passed. No additional input-robustness finding
remains for this feature's format fields.

## Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Strongest evidence and result |
|---|---|---|
| Explicit generics: ownership, order, bounds/defaults/variance and overflow | L1 public API | Per-language generic and overflow matrices, complete/selective/cache tests pass. The binding consequences of owner parameters are in the high finding. |
| Complete base text, order/deduplication, exact local IDs and precise mixed-list classification | L1 public API/CLI | Registered matrices pass; the supplemental qualified-owner and scope-transfer cases fail. This blocks readiness. |
| Rust positive/negative/empty/multiple impls, member trait facts, self ownership and aggregation | L1 public API/CLI | Registered cases pass; local-module and named const-owner cases fail through member-only output, widening and full hits. |
| Raw attributes, repetitions/empty arguments, selected modifiers and no body/string leakage | L1 public API/CLI | Language matrices and all five new cache member columns pass. No separate gap found. |
| Callable body lines, null/absent semantics, expression/nested/absent bodies and LF/CRLF | L1 public API | Body-span/line matrix, arithmetic boundaries, required-key and projection tests pass. |
| Selective analysis skips optional passes and does bounded per-file work | L1 work-counter assertions | Parse-only, query-count and linear-growth tests pass. Keep these checks during repair. |
| Genuine cache hits, narrowing, widening and schema/revision invalidation | L1 isolated public API/CLI | Registered cache tests pass; actual supplemental hits demonstrate that the high finding's wrong facts persist. |
| Styled outline base labels, signature suppression, wrapping, sorting and placement | L1 CLI plus L2 real-terminal capture | Registered outline tests and tmux/WezTerm capture pass for their fixtures. Incorrect upstream local-module ownership produces an incorrect textual outline in supplemental checks. |
| Terminal input encoding or keyboard-event behavior | L3 if required | No such requirement in this feature; no missing L3 verification. |
| Current schema/report/API/cache documentation and adapter losses | Document/source inspection plus L1 schema/adapter tests | Version examples and declared adapter losses agree. Lookup claims need correction alongside the behavior repair. |

Library test modules are included by the declared `tests/l1/main.rs` target
with `autotests = false`. The CLI matrices and ownership tests are included by
the compiled `tests/symbols/main.rs` target. Their names remain in L1. The
terminal targets declare `required-features = ["terminal-tests"]`, the live
L2 recipe enables that feature, and CI metadata selects it. The tier audit
found no stranded tests. No permanent test was added or renamed by this review.

**Implementation status: additional repair required, then ready for another
review.** Leave the feature active; author closure and cross-OS CI are external
to this readiness assessment.
