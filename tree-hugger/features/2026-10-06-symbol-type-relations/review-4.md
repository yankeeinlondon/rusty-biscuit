---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation lookup still substitutes record names for source binding evidence
    priority: high
  - title: Attribute and modifier members lack permanent cache robustness columns
    priority: medium
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-05T23:17:30-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-4.md
previous: 2026-10-06-symbol-type-relations/review-3.md
next: 2026-10-06-symbol-type-relations/review-5.md
---

# Review of Symbol Type Relations — Iteration 4

The feature is **not production ready**. All five repairs specifically listed
in review 3 are implemented. A wider sweep of the recurring name-resolution
class still finds incorrect target identities, omitted provable targets, and
incorrect Rust impl ownership. Individual raw-attribute and custom-modifier
fields also lack permanent malformed-cache tests.

## Previous findings and blocked work

Read the specification, plan, implementation log, and all three earlier reviews.
The following assessments concern each earlier finding's listed instances;
they do not claim that its entire defect class was eliminated.

| Review-3 unblocked finding | Assessment |
|---|---|
| Relation lookup still drops import and generic-shadowing evidence | Repaired for Scala multiple imports/renames and all nine oversized enclosing generic-list readers. The 65,536/65,537 boundaries retain their declared-name evidence independently of numbered metadata. Broader binding forms remain defective below. |
| Built-in name exclusions erase valid Go and Swift bases | Repaired for all 19 Go spellings, 15 Swift raw-type spellings, same-file aliases, imported type names, and explicit `Swift.Int`. Binding category and block visibility still affect these same decisions incorrectly below. |
| C# record superclasses remain unclassified | Repaired. Record classes, positional record classes, record structs, and interface-only controls retain their syntax-based classification. |
| Current schema examples advertise obsolete versions | Repaired. Current documentation agrees with the final schema 5.0 and extractor revision 31; subsequent reference-occurrence work accounts for the later version. |
| New relation source-span members lack permanent robustness columns | Repaired. All six members have individual columns with populated controls and public-output assertions. The sibling raw-attribute/modifier members were omitted from the compound-field sweep. |

Review 3 had no blocked findings. Review 2's blocked language support was
unblocked before its follow-up implementation: the author accepted precise
`extends`/`implements` plus `unclassified_bases`, and Swift extension
conformance attached only to a uniquely resolved same-file owner. The listed
C#, Swift, and Scala implementations and their later repairs follow that
accepted design. No design choice needs to be reaffirmed.

Review 1's template-template defaults and callable requirements retain their
repairs. Review 2's explicitly listed Python nested-class cases and generic/base
cache columns also pass. The remaining findings are actionable under the
accepted contract and require no human decision.

## Verification and scope

Used the tree-hugger, rust-testing, and rust skills. Reviewed all 16 language
acceptance rows; generic and base readers; raw attributes and modifiers;
body-line metrics; relation lookup and Rust ownership; selective analysis;
cache decoding/projection/widening; schema/documentation; outline rendering;
and declared test targets and tier selection. This review changes only the
review documents and requested specification metadata.

Final verification against the current working tree:

- `just test`: **1,206 passed, zero skipped**, 54.260 seconds.
- `just lint`: passed for the library and CLI.
- `just test-l2`: six registered entry points passed, 25.298 seconds. Rendering
  actually ran in tmux and WezTerm. Kitty returned through its unavailable
  backend path and provides no Kitty rendering evidence. The 327 filtered
  tests in this command belong to other selections, not missing L2 tests.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- Supplemental checks copied the existing language fixtures into temporary
  directories and changed one binding, enclosing scope, import, or owner form
  per case. More than 200 source cases compared complete uncached output,
  type-only uncached output, a cached complete run, and an actual warm hit.
  Relevant relations agreed across these paths, including the defects. Warm
  runs reported `1 hits` and `parsed 0 files`; complete indexes reported no
  error diagnostics for the tabled reproductions.
- Additional all-spelling sweeps exercised 19 Go built-ins and all 15 Swift
  raw-type spellings with each of `import func`, `import let`, and `import var`.
  Rust member-only output and declarations-to-containment cache widening were
  checked separately, including the rendered outline.
- A 45-cell supplemental cache sweep changed five nested-field columns in
  a real CLI-written Java entry. A separately observable cached signature
  proved service; rejected entries produced exactly the uncached public index.
  These checks establish current behavior but are not permanent regressions.
- Independent small programs were accepted by `rustc`, `clang++`, Go, and
  `swiftc` for the relevant scope/name-group examples. A temporary Swift module
  proved value-only imports coexist with local types and standard enum raw
  types. PHP execution confirmed that a class declared inside a function uses
  its namespace when naming a base. No Java/C#/Scala compiler proof is claimed.

Concurrent reference-occurrence work was present throughout this review.
Final tests were rerun after its changes; the cache sweep observed schema 5.0
and extractor revision 31. Reference-occurrence behavior is outside this
feature's review. Cross-OS proof is left to CI and does not affect readiness.

## Unblocked Findings

### High — Relation lookup still substitutes record names for source binding evidence

**Defect class:** target lookup treats displayed declaration names as complete
binding evidence, losing source-language scope boundaries, binding patterns,
and the separate groups in which type names and value names are looked up.

This breaks both sides of the contract: some bases receive the identity of
an inaccessible or unrelated declaration; other bases lose a uniquely provable
local identity. In C#, Swift, and Scala that second failure also changes a
provable superclass/conformance into an unclassified base. Rust self-type
lookup can assign members and conformance to the wrong type.

In the tree-hugger library, [Declarations](../../lib/src/file/tree_file/relations.rs:178)
is the per-file target index. It indexes displayed qualified names and makes
only recorded named declarations into scopes. Its
[binding_scope](../../lib/src/file/tree_file/relations.rs:381) chooses a name
without considering the required name group or source position;
[resolve](../../lib/src/file/tree_file/relations.rs:458) requires uniqueness
across every record kind. Its
[type_parameters_around](../../lib/src/file/tree_file/relations.rs:532) sees
recorded owners and Rust impls, but not all anonymous owners or Swift extension
owner parameters. These shortcuts explain the sibling failures below.

The tree-hugger library's
[import_binding](../../lib/src/file/extract/relations.rs:594), which supplies
names that must remain unresolved because they came from imports, omits Java
and Go entirely and loses Swift selective-import categories. Its comment says
Java/Go imports would conflict with same-file declarations; the implied
package-prefix reproductions below disprove that assumption. That comment
must change with the reader.

#### Reproduction conventions

Each table uses a same-file `ReviewBase` and a declaration `ReviewChild` whose
written base has that spelling, unless the row states otherwise. “Outer ID”
means that eligible same-file declaration's ID and in-file qualified name.
“Text only” means preserving the complete written target with neither an ID
nor a qualified name. For mixed base lists, an unresolved target also remains
unclassified unless syntax independently proves its category.

One minimal false-identity reproduction is:

```rust
trait ReviewBase {}
fn factory() {
    { trait ReviewBase {} }
    trait ReviewChild: ReviewBase {}
}
```

Run the shipped CLI with `hug --json-full symbols --no-cache probe.rs`.
`ReviewChild.extends[0]` identifies `factory::ReviewBase`, although that item
was declared inside a finished block. It must identify the outer trait.
`hug --json types --all-scopes --no-cache probe.rs` and a genuine complete
warm-cache hit report the same wrong target.

#### Block and declaration-position sweep

For the first eight rows, the same three edits were tested: a competing type
in a finished inner block, a competing type in a sibling block, and a sibling
block containing the only declaration of that name. The first two must select
the outer declaration; the last must retain text only.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust trait bases | Three block edits | Inaccessible inner trait ID in all three | Outer ID / outer ID / text only |
| Go interface embeds | Three block edits | Inaccessible inner interface ID | Outer ID / outer ID / text only |
| C++ base clauses | Three block edits | Inaccessible inner struct ID | Outer ID / outer ID / text only |
| Java local-class superclasses | Three block edits | Inaccessible inner class ID | Outer ID / outer ID / text only |
| TypeScript class extends | Three block edits | Inaccessible inner class ID | Outer ID / outer ID / text only |
| JavaScript class extends | Three block edits | Inaccessible inner class ID | Outer ID / outer ID / text only |
| Swift class bases | Three `do`-block edits | Inaccessible inner class ID | Outer ID / outer ID / text only |
| Scala class bases | Three block edits | Inaccessible inner class ID | Outer ID / outer ID / text only |
| The same eight readers — controls | Child inside the competing type's own block; competing type in the same function scope; no competing type | Correct local/local/outer target IDs | Observed result; clean |
| Rust, C++, Scala imports | Competing import inside a finished sibling block | Outer target remains unresolved | Outer ID |
| Rust, C++, Java, TypeScript, JavaScript, Swift, Scala values | Competing value inside a finished sibling block | Outer target remains unresolved/unclassified | Outer ID |
| Go built-in filter | Finished sibling block declares an interface named each of the 19 built-in spellings | Adds an extends edge to the inaccessible interface | No edge for the built-in term |
| Swift raw-type filter | Finished `do` block declares `typealias Int = String`; later enum uses `Int` | Adds unclassified `Int` | No base edge for the standard raw type |
| Go, C++, Java declaration order | Child precedes a later local type declaration with the same name | Later local type ID | Outer ID; that local declaration is not yet in scope |
| Rust, Swift, Scala forward-type controls | Equivalent later local declaration | Local ID | Observed result for these hoisted/forward type forms; clean |
| Python suites and callable boundaries | Type declared in an `if` suite before Child; separately, type only in a nested function | Same-function type ID; outer ID | Observed result; Python suites do not introduce block scope |
| C# namespace boundaries | Separate namespaces; explicitly rooted `global::Example.Symbols.ReviewBase` in the copied file-scoped namespace fixture | Correct rooted ID | Observed result; clean; C# has no equivalent local class declaration |
| PHP callable boundary | Namespace class `ReviewBase`; function declares `ReviewChild extends ReviewBase` | Text only | Namespace's ReviewBase ID; the function display prefix is not a class-name namespace |
| PHP conditional control | Same Child inside `if` in the namespace | Namespace's ReviewBase ID | Observed result; clean |
| C, Perl, Bash, Zsh, Lua | Original fixture controls | No required inheritance/conformance edges invented | Observed result; no equivalent language relation in required scope |

The Go built-in sweep covered `bool`, `byte`, `complex64`, `complex128`,
`float32`, `float64`, `int`, `int8`, `int16`, `int32`, `int64`, `rune`, `string`,
`uint`, `uint8`, `uint16`, `uint32`, `uint64`, and `uintptr`.

Rust's item scopes and Java's local-class declaration positions are explicit
language rules, not compiler inference requested by this feature.
[Rust scope rules](https://doc.rust-lang.org/reference/names/scopes.html),
[Java scope and shadowing rules](https://docs.oracle.com/javase/specs/jls/se17/html/jls-6.html).

#### Type-name versus value-name sweep

Some languages allow a type and a function, parameter, or variable to share a
spelling. A base clause that asks for a type must use the type's binding;
JavaScript-style class extends expressions instead ask for a value. Treating
all declaration kinds as one competing name group makes valid local types
look ambiguous.

For example, this compiler-accepted Rust source loses the trait ID:

```rust
trait ReviewBase {}
const ReviewBase: usize = 0;
trait ReviewChild: ReviewBase {}
```

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust trait bases | Same-named value parameter, local variable, function, constant, static; const generic parameter | Text only in every case | Outer trait ID; those names occupy the value group |
| Rust qualified trait path | Module containing the trait plus same-named function at the module prefix | Text only | Module's trait ID |
| Rust impl self and trait | Function has value parameters named like the outer struct and trait | No type-level conformance/member aggregation; member lacks resolved owner | Outer struct/trait IDs and normal member ownership |
| C++ base specifiers | Same-named variable, function, non-type template parameter | Text only | Struct ID; base-specifier type lookup ignores these non-types |
| C++ qualified type path | Type prefix plus same-named function | Text only | Nested type ID |
| Java superclasses | Same-named parameter, local variable, field, method | Text only | Outer class ID |
| C# mixed base reader | Same-named field, property, method | Unclassified text | Resolved class in extends |
| Swift mixed base reader | Same-named value parameter or local `let` | Unclassified text | Resolved class in extends |
| Scala mixed base reader | Same-named `val` or method | Unclassified text | Resolved class in extends |
| Scala companion declarations | Class/object companions; trait/object companions | Unclassified text | Class ID in extends; trait ID in implements |
| TypeScript type relations | Class implements and interface extends with same-named value parameter/local variable | Text only | Interface ID in the corresponding relation |
| TypeScript qualified type relation | Value parameter named like a namespace prefix | Text only | Namespace's interface ID |
| PHP class bases | Class and function share a spelling | Text only | Class ID; function names are separate |
| Rust, C++, Java, C#, Swift, Scala, TypeScript — generic controls | Actual type parameter shares the outer type spelling | Text only/unclassified | Observed result; type parameters correctly prevent outer matching |
| TypeScript and JavaScript class extends | Simple value parameter hides the outer class | Text only | Observed result; extends is a value expression |
| Python class bases | Simple parameter hides the outer class | Text only | Observed result; clean |
| Go interface embed | Value binding occupies the name | Text only | Observed conservative result; no fabricated type ID |
| PHP variable control | `$ReviewBase` alongside class ReviewBase | Class ID | Observed result; clean |
| C, Perl, Bash, Zsh, Lua | No required base reader | No added edges | Observed result; not applicable |

These rules are documented in the
[Rust name groups](https://doc.rust-lang.org/reference/names/namespaces.html),
[TypeScript declaration groups](https://www.typescriptlang.org/docs/handbook/declaration-merging.html),
[C# name lookup](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/basic-concepts),
and [Scala companion-object documentation](https://docs.scala-lang.org/overviews/scala-book/companion-objects.html).
PHP class names inside functions continue to use the enclosing namespace;
[PHP namespace rules](https://www.php.net/manual/en/language.namespaces.faq.php).

#### Binding-pattern and omitted-owner sweep

A binding need not produce a public symbol record. Its presence still prevents
a base expression from acquiring an unrelated outer declaration's identity.
In these rows a function's binding competes with the outer class, or with a
lowercase Scala object prefix `reviewNs` containing a class named ReviewBase.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| TypeScript and JavaScript parameters, checked separately | Nested object; nested array; object default; nested renamed property; object rest; array rest | Unrelated outer class ID for every form | Text only |
| TypeScript and JavaScript variables, checked separately | Flat renamed object; nested object; flat array; nested array; object default; object rest; array rest | Unrelated outer class ID for every form | Text only |
| TypeScript and JavaScript catch bindings | Identifier; object; nested object | Unrelated outer class ID | Text only |
| TypeScript and JavaScript `for…of` bindings | Identifier; shorthand object; nested object | Unrelated outer class ID | Text only |
| TypeScript and JavaScript controls | Simple parameter; flat shorthand object/flat array/flat renamed/default/bare-rest parameter; simple variable/flat shorthand object variable; assigned and anonymous arrow parameters | Text only | Observed result; these existing binding captures are clean |
| Python assignment patterns | Nested tuple; nested list; starred assignment | Unrelated outer class ID | Text only |
| Python other pattern bindings | Nested tuple in `for`; nested tuple in `with`; match capture; nested match capture | Unrelated outer class ID | Text only |
| Python controls | Simple parameter/assignment; flat tuple assignment; simple `for`/`with`; exception alias; walrus binding | Text only | Observed result; clean |
| Python explicit global | Inner function says `global ReviewBase`; enclosing function has a same-named class | Enclosing function's class ID | Module's class ID |
| Python nonlocal control | Same example with `nonlocal ReviewBase` | Enclosing function's class ID | Observed result; clean |
| Scala stable-path prefix | Tuple/nested-tuple/unapply `val`; identifier/nested-tuple `for`; match capture/nested capture bind `reviewNs` | Outer object's nested class ID | Text only; a pattern-bound value supplies the prefix |
| Scala stable-path controls | Simple `val`; tuple `for`; typed lambda parameter | Text only/unclassified | Observed result; clean |
| TypeScript anonymous generic class | `const Box = class<ReviewBase> { method() { class ReviewChild implements ReviewBase {} } }` | Outer interface ID | Text only; anonymous owner's type parameter binds the name |
| C++ generic lambda | `[]<class ReviewBase>() { struct ReviewChild: ReviewBase {}; }` | ID of the captured generic parameter, reported as a Type record | Text only; a template parameter is not an eligible declared base identity |
| Scala polymorphic lambda | `[ReviewBase] => () => { class ReviewChild extends ReviewBase }` | Outer class ID | Text only/unclassified |
| Swift extension generic owner | Generic `struct ReviewBox<ReviewBase>` followed by `extension ReviewBox { class ReviewChild: ReviewBase {} }` | Outer class ID | Text only/unclassified; the extension sees its owner's parameter |
| Anonymous/extension controls | Equivalent unparameterized TypeScript anonymous class, C++ lambda, Swift extension | Outer base ID | Observed result; clean |
| Nine named generic readers | Rust, TypeScript, Go, Java, C#, C++, Swift, Scala, Python ordinary/boundary rows | Named-owner parameter shadowing retained; oversized selected lists error explicitly | Observed result; clean for the listed supported forms, including Python's explicit parameter syntax |
| Java, C++, Scala anonymous inherited owners | Anonymous class/struct extends External; body declares Child using outer ReviewBase | Outer ReviewBase ID | Text only; External may supply a nearer member type |
| Named inherited-owner controls | Existing inherited-name path-matrix rows | Text only | Observed result; named owners preserve that uncertainty correctly |
| Other supported languages | C# has no anonymous local type declaration; Rust/Go/PHP/Java/C#/Swift ordinary value/type controls are enumerated in the preceding tables; C/Perl/Bash/Zsh/Lua have no required bases | Controls as listed; no additional required anonymous-generic relation form | Apply each language's type/value rule, not blanket value shadowing |

Java and Scala anonymous-owner cases were also checked in copies with the
package declaration removed, so accidental failure to traverse a package
prefix could not conceal a false identity. C++ used the original fixture.
Do not treat the generic parameter's existing C++ Type capture as permission
to resolve it as a concrete base.

#### Import and built-in decision sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go imported package prefix | Fixture package `symbols`; `import symbols "io"`; same-file Reader; Child embeds `symbols.Reader` | Same-file `symbols.Reader` ID | Text only; the qualifier denotes imported io. The standalone program builds successfully. |
| Java imported package prefix | Copied fixture's package changed to Entry; `import java.util.Map.Entry`; same-file ReviewBase; Child extends `Entry.ReviewBase` | Same-file `Entry.ReviewBase` ID | Text only; Entry is an imported type, not the implied current package prefix |
| Swift selective value imports — raw types | Each `import func`, `import let`, `import var` binds the first segment of each standard raw-type spelling | All 45 cases retain a spurious unclassified base | No raw-type base edge; a value-only import does not hide a type |
| Swift selective value imports — local targets | Value import shares a spelling with a local class used as a base | Target unresolved/unclassified | Local class ID in extends |
| Swift selective type-import controls | `import class`, `struct`, `enum`, `protocol`, `typealias` | Imported target remains text/unclassified | Observed result; genuine type imports must remain unresolved |
| Rust, C++, PHP, Python, TypeScript, JavaScript, C#, Scala import controls | Existing explicit/alias/multi-import/wildcard/root-path matrix cases | Imported targets stay unresolved; valid root/local paths retain IDs | Observed result for the listed repaired forms |
| Swift local-type/alias/import controls | Review-3 standard-name alias/import and explicit Swift-module cases | Correct retained or excluded target | Observed result; clean for the listed forms |
| C, Perl, Bash, Zsh, Lua | No required import-to-base resolution | No added edges | Observed result; not applicable |

The Swift sweep covered `Int`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt`,
`UInt8`, `UInt16`, `UInt32`, `UInt64`, `String`, `Character`, `Float`, `Double`,
and `Unicode.Scalar`. For the qualified spelling the competing import binds
`Unicode`, its first segment. Blocks and import categories affect whether
these names denote raw types; spelling alone cannot make that decision.

#### Consumer and cache-projection sweep

These consumers all use the same flawed evidence and must remain in the
regression sweep. They are part of this finding, not independent defects.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Ordinary written-base projection | Every failing source case, complete versus type-only output | Same false/missing target on both paths | Correct target decision on both paths |
| Rust trait/self resolution | Global struct ReviewSelf and trait ReviewTrait; a finished sibling block declares enum ReviewSelf and trait ReviewTrait; later impl uses the bare names | Impl resolves to the sibling enum and trait | Global struct and trait IDs |
| Rust member-only declaration metadata | Same source, `--json functions --all-scopes --no-cache` | Impl method gets Enum container kind and sibling trait ID | Struct container kind and global trait ID |
| Rust type conformance/member aggregation | Same source, complete index | Sibling enum gains conformance and impl member; global struct has neither | Global struct owns both; sibling enum stays untouched |
| Declarations-to-containment widening | Same source, declarations cached first, then `--plain symbols --tree --all-scopes` | Partial-hit parse renders impl method beneath the sibling enum | Global struct owns/rendered member |
| Full warm cache | Same source after complete cache population | Genuine hit preserves wrong parent/container/member IDs | Genuine hit preserves correct IDs |
| Swift extension consumer | Generic owner's nested Child case above; ordinary uniquely local extension-conformance controls | Nested Child falsely resolves outer type; ordinary extensions remain correct | Preserve owner's parameter binding and existing attachment rules |
| Named Rust ownership/rooted controls | Existing method-only, empty/multiple/negative impl, full self-path, cache-widening rows | Listed repairs remain correct | Observed result; clean |

#### Required repair and regression coverage

Retain syntax binding evidence independently of displayed record names. The
index needs actual enclosing blocks/anonymous owners, the point at which a
binding becomes visible where the language requires it, and the name group
required by the particular relation. It must include pattern bindings and
imports even when no public symbol was captured. Read generic binding kind as
well as spelling; a Rust const or C++ non-type parameter must not hide a type
used by a base clause. Swift extensions must inherit the proven local owner's
type-parameter bindings. PHP callable display prefixes must not enter class
namespace lookup. Anonymous inherited owners need the same uncertainty barrier
as named inherited owners.

The tree-hugger library already has a
[ScopeTree](../../lib/src/analysis/scope/mod.rs:152), used by reference analysis to
retain scope boundaries, unreported bindings, and type/value distinctions.
Inspect and share suitable syntax evidence rather than introducing another
incompatible model. Relation extraction must still work without enabling the
optional References pass. Preserve one definitions query, one bounded shared
walk/index, existing symbol IDs/names, and counter-based growth checks; do not
repair this with a whole-file search for each base or new public symbol kinds.

Add table-driven regressions for every failing row above and retain the clean
controls. Extend the existing relation/path matrix in the tree-hugger library's
[symbols_relations tests](../../lib/tests/l1/symbols_relations.rs:711), which
compare complete and selective target decisions, and the CLI's real-cache
ownership checks. Verify no-error diagnostic controls, exact target ID and
in-file qualified name, unresolved target absence, correct relation category,
member-only container kind, aggregation, and containment widening. Equality
between cached and uncached wrong output is not a sufficient assertion.
Update current lookup documentation and behavior-linked comments with the
repair. Some current comments accurately describe the overly broad shortcuts;
changing those comments alone would not repair the contract.

### Medium — Attribute and modifier members lack permanent cache robustness columns

**Defect class:** newly populated compound metadata has whole-collection
malformed-input tests but lacks separate columns for every member that changes
the public result.

The tree-hugger CLI's
[columns](../../cli/tests/symbols/cache_robustness.rs:152), which drives the
persistent-entry public-output matrix, tests the `attributes` array by
projecting attribute names from a Python `staticmethod` without a value.
The modifier-array controls project only static/async flags. Neither setup
pins a raw attribute's value or a Custom modifier's string.

The tree-hugger library's [Attribute](../../lib/src/shared/schema_v2/facets.rs:192)
stores the raw name and argument value; [Modifier](../../lib/src/shared/schema_v2/facets.rs:205)
encodes both fixed tags and Custom strings. They require columns for
`Attribute.name`, `Attribute.value`, `Modifier.type`, and `Modifier.Custom.value`,
with both fixed-tag and Custom controls for `Modifier.type`.

The supplemental sweep used the real Java fixture plus:

```java
@ReviewAttr("raw value")
final class MatrixSubject { static void unit_subject() {} }
```

It populated all four members directly from shipped extraction. Each cell
restored the CLI-written entry and changed exactly one member; a separate
cached signature marker proved whether the entry was served. A rejected entry
had to reproduce the complete uncached index. No decoder defect was observed
in these 45 cells. The missing permanent coverage remains a testing gap under
the required robustness-matrix contract.

#### Input robustness matrix — JSON persistent cache

`Served(x)` means the hit's public field has x. `Recomputed` means the edited
entry is rejected and the public result equals fresh extraction. Array shapes
are wrong whole-member types for these scalar members. The optional attribute
value has the cache's existing absent/null → None contract; those shapes are
explicitly tested separately rather than inferred from the Rust Option type.
Empty strings are currently accepted string values except for an enum tag.

| Shape | Attribute.name | Attribute.value | Modifier.type — Custom | Modifier.Custom.value | Modifier.type — Static |
|---|---|---|---|---|---|
| Unedited control | Served(ReviewAttr) | Served(`"raw value"`) | Served(Custom) | Served(final) | Served(Static) |
| Absent | Recomputed | Served(None) | Recomputed | Recomputed | Recomputed |
| Explicit null | Recomputed | Served(None) | Recomputed | Recomputed | Recomputed |
| Wrong whole type — 123 | Recomputed | Recomputed | Recomputed | Recomputed | Recomputed |
| Wrong one element — `[original, 123]` | Recomputed | Recomputed | Recomputed | Recomputed | Recomputed |
| Wrong every element — `[123]` | Recomputed | Recomputed | Recomputed | Recomputed | Recomputed |
| Empty string | Served(empty string) | Served(empty string) | Recomputed | Served(empty string) | Recomputed |
| Duplicate member key | Recomputed | Recomputed | Recomputed | Recomputed | Recomputed |
| Trailing invalid document content | Recomputed | Recomputed | Recomputed | Recomputed | Recomputed |

All cells matched the current serialized-type contract. The repair is to
retain this full table in the registered matrix, not to add a one-off test
for whichever cell next fails.

The sibling-field sweep found:

| Site | Shapes/control checked | Observed result | Expected result / permanent coverage |
|---|---|---|---|
| Attribute.name and Attribute.value | All nine shapes above, raw nonempty argument control | Table above | Add both nested columns; public assertion must preserve raw value too |
| Modifier.type, fixed and Custom | All nine shapes, Static and Custom controls | Table above | Add the nested tag column with both variant controls |
| Modifier.Custom.value | All nine shapes, `final` control | Table above | Add the nested payload column |
| Attributes array | Existing control/absent/null/wrong whole/one element/all elements/empty/duplicate/trailing rows | All permanent assertions pass | Clean at collection level; does not replace member columns |
| Modifiers array, static and async | Existing nine-shape columns | All permanent assertions pass | Clean at collection level; does not cover Custom payload |
| Generic parameters | Separate name, kind, constraints, default, variance, position columns; list and declaration-level constraints columns | Permanent assertions pass | Clean; each load-bearing member already has a column |
| Relation targets | Separate name, id, qualified_name, source_span columns and extends/implements/unclassified list columns | Permanent assertions pass | Clean for the registered target encoding |
| Relation target source span | Separate start.line, start.column, end.line, end.column, start_byte, end_byte columns | All 54 permanent member cells pass | Review-3 repair complete |
| Metrics | Separate parameters, has_docs, lines, cyclomatic plus whole-facet column | Permanent assertions pass | Clean; missing keys are not silently defaulted |
| Entry/record version and fingerprint | Existing current/incompatible major/minor and malformed-entry columns | Permanent assertions pass | Clean; current version required |
| Earlier source span fields; concurrent occurrence/coverage fields | Shared span types are exercised by the repaired target columns; occurrence work has a separate evidence matrix | Registered tests pass | No new finding for fields outside this feature |

This feature does not add a manifest, lockfile, or configuration reader. JSON
is the persistent-cache format; there is no YAML/TOML input side to duplicate
this matrix onto. Extend the existing test with a real populated control and
one edit per cell, preserve its service sentinel, and assert the public result.
Registering an unrelated new test framework would not improve this repair.

## Blocked Findings

None. Both findings can be repaired under the accepted feature contract.

## Recurrence

The high finding repeats review 1's **Relation IDs are assigned without proving
path meaning**, review 2's **Python nested class scopes still assign incorrect
base identities**, and review 3's **Relation lookup still drops import and
generic-shadowing evidence**. The built-in consequences also repeat review 3's
**Built-in name exclusions erase valid Go and Swift bases**.

Those repairs should have swept the binding evidence shared by every base
reader, Rust self/trait consumer, and Swift extension consumer: actual block
boundaries, separate type/value names, declaration-position rules, patterns
that lack public records, anonymous/generic owners, explicit Python scope
directives, and import qualifiers/categories. Earlier repairs covered selected
named scopes, Scala import shapes, numbered generic overflow, and built-in
spellings. They did not cover the sibling sites and shapes enumerated in this
review's tables. This repair must take the complete enumerated list together.

The medium finding repeats review 2's **Generic and base fields lack a permanent
cache robustness matrix** and review 3's **New relation source-span members lack
permanent robustness columns**. Their compound-field sweep correctly added the
listed generic/base/span members but missed `Attribute.name`, `Attribute.value`,
`Modifier.type` for both fixed and Custom variants, and `Modifier.Custom.value`.
The five-column, nine-shape table above carries the complete remaining list
identified by this sweep. `recurrence` is therefore true.

## Requirement-to-verification assessment

Test counts do not establish any individual requirement. The following maps
requirements to the strongest appropriate evidence currently present.

| User-facing requirement | Appropriate level | Evidence present / assessment |
|---|---|---|
| Written generic ownership, order, bounds/defaults/variance and overflow | L1 public API/CLI | Named-owner matrices and boundary tests pass; omitted-owner binding consequences are part of the high finding |
| Base text, source order, local IDs and precise classification | L1 public API/CLI | Registered relation/path matrices pass for their rows; supplemental source tables expose wrong IDs/classification |
| Rust empty/multiple/negative impls, type aggregation, member-only metadata, containment | L1 public API/CLI | Existing acceptance/cache tests pass; block/category sibling cases expose wrong ownership or lost aggregation |
| Raw attributes and specified modifiers without body/string leakage | L1 public API/CLI | Per-language modifier/attribute rows pass; nested persistent-member robustness coverage is incomplete |
| Callable body-line metric, LF/CRLF, missing/expression/nested bodies | L1 public API | Existing body-line matrix and required metric-cache keys pass; no finding |
| Optional-pass skipping and bounded per-file work | L1 counter assertions | Parse-only and linear-growth tests pass; repairs must preserve them |
| Selective output, real hits, narrowing/widening/invalidation | L1 public API/CLI with isolated cache | Registered tests and supplemental hit checks pass structurally; incorrect binding decisions survive the cache |
| Styled outline relation labels, wrapping and container placement | L1 output assertions plus L2 real-terminal capture | Registered output/outline tests and tmux/WezTerm capture ran; upstream wrong Rust ownership renders the wrong tree |
| Keyboard-event or terminal-input behavior | L3 when specified | No such requirement in this feature; no L3 mismatch |
| Current schema/API/cache/report documentation and adapter losses | Source/document inspection plus L1 schema tests | Current version examples and stated adapter losses agree; lookup limitations need correction with the high finding |

Library L1 files are included by the declared `tests/l1/main.rs` target; CLI
matrix files by the declared `tests/symbols/main.rs` target. Their manifests
use explicit test targets, and these names remain in L1. Terminal tests have
live L2 recipes and the `terminal-tests` feature enabled by the relevant
recipe/CI metadata. No newly added feature test was found outside a compiling
target or a running tier. No additional CI environment, gate, or human approval
is needed to implement these findings.
