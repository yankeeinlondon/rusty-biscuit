---
$schema: feature-review.yaml
ready: false
findings:
  - title: Relation IDs are assigned without proving path meaning
    priority: high
  - title: Template-template parameter defaults are dropped
    priority: medium
  - title: Callable requirements disappear without a direct template wrapper
    priority: medium
  - title: Three languages still omit required base relations
    priority: high
human_review: true
human_review_items:
  - |-
    Choose how to store C#, Swift, and Scala lists that mix inheritance with
    interface, protocol, or trait conformance. Their relation lists are
    currently empty. This is the unresolved design question already in the
    specification; the review introduces no replacement decision.

    - Recommended: retain written entries in `extends`, moving an entry to
      `implements` only when a declaration in the same file proves conformance.
      For Swift extensions, attach conformance only to a uniquely identified
      type declared in the same file; document the loss for external owners.
    - Add a separate list for bases whose category cannot be established.
      This preserves precise meanings but requires a schema and output change.
    - Retain only bases whose category can be established. This discards many
      external targets and requires explicitly accepting that reduced scope.

    Select the intended meaning, including the treatment of Swift extensions,
    so the missing readers and their expected results can be implemented.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-05T19:13:32-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
log: tree-hugger/features/2026-10-06-symbol-type-relations/log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-1.md
next: 2026-10-06-symbol-type-relations/review-2.md
---

# Review of Symbol Type Relations

The feature is **not production ready**. The implemented readers can assign
relations to the wrong local declaration, two C++ readers lose explicitly
written metadata, and C#, Swift, and Scala base relations remain unimplemented.
Only the last finding needs an author decision; the other fixes can proceed.

## Verification and scope

Reviewed the specification, implementation log, source readers, record
construction, query planning, cache projection and decoding, CLI rendering,
fixture matrices, current documentation, and skill updates. No implementation
fixes were made.

- `just test`: **1,074 passed, zero skipped**.
- `just lint`: passed for the library and CLI.
- `just test-l2`: six selected tests passed. The declaration and god-files
  tests executed in tmux and WezTerm; the two Kitty entry points returned early
  because that backend is unavailable. Those returns are not Kitty evidence.
- `just check-tier-coverage tree-hugger`: no stranded tests.
- Supplemental public-API checks copied the existing relations and generics
  matrix sources into temporary files, changed or added the problematic
  declaration, and compared complete and declarations-only results. The copied
  sources parsed without syntax diagnostics. The temporary test registration
  was removed after execution.
- Shipped `hug --json-full symbols --no-cache <file>` probes expanded the
  sibling sweeps to additional owner kinds and path shapes. Run that command
  from the directory containing the reproduction file, passing its filename.
- A separate cache probe copied the real `types.rs` fixture, added populated
  generics and relations, let `hug` write its cache, and changed one stored
  field per matrix cell. A renamed sentinel record distinguished a served
  entry from silent recomputation. Its outcomes are recorded below.

Passing the existing suite did not detect the defects below. They need
regression assertions through returned records and CLI output, including warm
cache results, rather than assertions about private reader return values.

## Findings

### High — Relation IDs are assigned without proving path meaning

**Defect class:** matching a complete qualified-name string is treated as proof
of a path's meaning, without enforcing its root, intermediate bindings, or all
visible generic parameters.

In the `tree-hugger` library,
[Declarations::resolve](../../lib/src/file/tree_file/relations.rs) assigns
same-file IDs for declared bases and Rust impl owners.
[base_path](../../lib/src/file/extract/relations.rs) supplies its path evidence.
The resolver tries `scope + full_path`, then outer prefixes, but never checks
whether a nearer binding of the first segment prevents that lookup. It also
accepts a callable's lexical name as a namespace accessible through a dotted
expression. Leading C++ `::` is discarded without establishing a root anchor;
Rust's leading `::` is similarly treated as an ordinary relative path. PHP
class paths incorrectly fall back outside their namespace.

For Rust impls,
[resolve_impl](../../lib/src/file/tree_file/relations.rs) supplies only the
impl's own generic parameters. An enclosing callable's parameter can therefore
be mistaken for a file-level type. The same wrong self-type decision feeds
type-level `implements`, member `parent`/`container`, and the type's `members`.

Why this matters: an unresolved written target is useful and permitted by the
contract. A wrong ID makes consumers attribute inheritance, conformance, or
members to an unrelated declaration. The incorrect facts reach JSON and
cached results; this is not merely a display issue.

The sweep covered every implemented base reader and the Rust impl consumers:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust supertraits | Local `mod a { trait Dup {} }`, then `trait ReviewAbsolute: ::a::Dup {}` | Target ID names local `a::Dup` | Preserve `::a::Dup` unresolved; the extern-style path does not establish that local module |
| Rust impl self type and containment | Local `mod ext { struct B; }`, then `impl ::ext::B { fn probe(&self) {} }` | Method parent/container IDs name local `ext::B` | Textual container `::ext::B`, no local parent or membership |
| Rust impl aggregation and containment | File-level `struct T`, then `fn outer<T>() { impl Base for T { fn probe(&self) {} } }` | File-level `T` gains `implements Base` and `probe`; method gets that parent | The written generic self type must remain unresolved; no facts attach to file-level `T` |
| Rust qualified imports | Inner module imports `other::ns`, then writes `ns::Base`; another module has a local `ns::Base` | Unresolved | Unresolved; clean control |
| Rust explicit local root | `trait Child: crate::ext::Base {}` | Resolves to `ext::Base` | Same; clean control |
| TypeScript bases | Existing `namespace ns { class Inner {} }`, then `function f(ns: any) { class Child extends ns.Inner {} }` | Resolves to namespace `ns.Inner` | Unresolved: the parameter supplies the runtime base |
| JavaScript bases | `function outer() { class Local {} }`, then `class Child extends outer.Local {}` | Resolves to lexical `outer.Local` | Unresolved: a lexical class is not a property of its enclosing function |
| Python bases, shadowing | Existing `Outer.Base`, then `def f(Outer):` containing `class Child(Outer.Base)` | Resolves to the outer class's `Outer.Base` | Unresolved: the parameter shadows that class |
| Python bases, lexical path | A local class inside `def outer()`, then `class Child(outer.Local)` | Resolves to lexical `outer.Local` | Unresolved: the source establishes no such function attribute |
| C++ bases, global anchor | Global `B` and `ns::B`, then `namespace ns { struct Child : ::B {}; }` | Resolves to `ns::B` | Resolve to global `B` |
| C++ bases, intermediate shadowing | Global `ns::B`, empty `inner::ns`, then `inner::Child : ns::B` | Falls back to global `ns::B` | Unresolved: lookup of `ns` is already decided by `inner::ns` |
| C++ bases, global control | Global `B`, no competing `ns::B`, then `ns::Child : ::B` | Resolves to global `B` | Same; clean control that alone would miss the defect |
| PHP bases, relative qualification | Existing `App\Root`; `namespace Other; class Child extends App\Root {}` | Resolves to `App\Root` | Unresolved: the written relative path names `Other\App\Root` |
| PHP bases, import alias | Existing `ns\B`; another namespace imports `other\ns` and writes `ns\B` | Resolves to the unrelated local `ns\B` | Unresolved; expanding the import is outside this feature |
| PHP explicit root | `class Child extends \ns\B` | Resolves to `ns\B` | Same; clean control |
| Java bases | `class Outer<Root> { class Child extends Root {} }`, with an outer local `Root` class | Unresolved | Same; clean generic-shadowing control |
| Go interface embeds | Existing `Base`; change the matrix owner to `type Iface[Base any] interface { Base ... }` | `Base` remains unresolved | Same; clean generic-shadowing control |
| C#, Swift, Scala | Their matrix base-list fixtures | No base relations | Separate blocked finding below; no implemented resolver path to judge |
| C, Perl, Bash, Zsh, Lua | Existing negative relation rows | No invented base relations | Same; these constructs are outside the required relation scope |

The enclosing-generic Rust snippet tests source extraction, not Rust compiler
acceptance of an impl capturing an outer generic. This library does not run a
compiler; the explicit generic name still cannot prove ownership by the
unrelated file-level type.

**Required change:** establish language-specific root and namespace behavior,
resolve the first path segment before its remaining segments, reject paths
through unproven runtime or lexical owners, and include enclosing generic
evidence for impl lookup. Reuse that conservative decision for all Rust
containment and conformance consumers. Preserve written text and stable IDs.
Extend the relation matrix with every failing and clean row above, including
method-only queries and cache widening to Containment. Keep lookup indexes
per file rather than replacing these checks with repeated whole-file scans.

### Medium — Template-template parameter defaults are dropped

**Defect class:** a default stored on a nested parameter declaration is read
only from the outer parameter node, so the record silently omits it.

In the `tree-hugger` library,
[cpp_param](../../lib/src/file/extract/generics.rs) finds the nested declaration
to obtain a template-template parameter's name, then reads `default_type` and
`default_value` from the original outer node. For
`template <template<class> class TT = Box>`, `TT` is reported but its default
is absent. This is supported syntax, not the documented concept-parameter
grammar limitation.

The existing C++ generic fixture already contains a template-template
parameter. Changing its `class TT` to `class TT = Box` reproduces the loss
through both complete and declarations-only public queries.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ class/struct template | `template<template<class> class TT = Box> struct Child {}` | `TT.default` absent | `"Box"` |
| C++ function template | Same parameter list on `void probe() {}` | Default absent | `"Box"` |
| C++ alias template | Same parameter list on `using Child = TT<int>` | Default absent | `"Box"` |
| C++ member template | Same parameter list on a class method | Default absent | `"Box"` |
| C++ constructor template | Same parameter list on `Child(TT<int> x) {}` | Default absent | `"Box"` |
| C++ ordinary type parameter | `class T = int` in the same mixed parameter list | `"int"` retained | Same; clean |
| C++ non-type parameter | `int N = 3` in the same mixed parameter list | `"3"` retained | Same; clean |
| Rust item and alias defaults | Existing `Bag<T: Clone = String>`, const parameter default, and alias default rows | Defaults retained | Same; clean |
| TypeScript type and alias defaults | Existing class, interface, and alias generic rows | Defaults retained | Same; clean |
| Python defaults | Existing PEP 696 default probe | Syntax error under the pinned grammar | Documented grammar limit; not this defect |
| Other generic readers | Go, Java, C#, Swift, Scala control rows | No default invented | Same; no required native default form in these rows |
| Languages without required generics | JavaScript, C, PHP, Perl, Bash, Zsh, Lua rows | No generic defaults | Same |

**Required change:** obtain the default from the same nested declaration that
owns `TT`, while preserving its outer list position and name. Add class,
function, alias, method, and constructor regressions, retaining the clean
ordinary-type and non-type controls. Exercise their cache projections too.

### Medium — Callable requirements disappear without a direct template wrapper

**Defect class:** extraction of an independently written requirement is gated
on the presence of a parameter-list wrapper that the declaration need not own.

In the `tree-hugger` library,
[cpp_generics](../../lib/src/file/extract/generics.rs) returns immediately when
the declaration's parent is not `template_declaration`. That skips its later
trailing `requires` reader. A non-template member of a template class can
write its own requirement using the class's parameter; the member should
report that requirement without inheriting the class's generic parameter list.

For example:

```cpp
template<class T> struct Child {
    void probe() requires Small<T> {}
    Child() requires Small<T> {}
};
```

Both callable records have an empty `type_info.constraints`; each should have
`["Small<T>"]` and an empty `generic_params`. Consumers otherwise cannot
distinguish these conditional declarations from unrestricted ones.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ method declaration | `void probe() requires Small<T>;` inside a template class | No constraints | `["Small<T>"]` |
| C++ method definition | `void probe() requires Small<T> {}` in the same position | No constraints | `["Small<T>"]` |
| C++ constructor definition | `Child() requires Small<T> {}` | No constraints | `["Small<T>"]` |
| C++ directly wrapped callable | `template<class T> void probe() requires Small<T> {}` | Requirement retained | Same; clean |
| C++ template-level requirement | Existing `template<typename T> requires Small<T> ...` row | Requirement retained | Same; clean |
| Rust member with enclosing parameter | `impl<T> Child<T> { fn probe(&self) where T: Copy {} }` | Member constraints `["T: Copy"]`, no owned generics | Same; clean |
| Swift member with enclosing parameter | `struct Child<T> { func probe() where T: Equatable {} }` | Member constraints `["T: Equatable"]`, no owned generics | Same; clean |
| C# separate constraints | Existing class and method `where` rows | Bounds attach to their declared parameters | Same; clean |
| Other readers | TypeScript, Java, Go, Python, Scala generic matrix rows | Their own supported inline bounds retained | Same; no equivalent required trailing-requirement reader |
| Languages without required generics | JavaScript, C, PHP, Perl, Bash, Zsh, Lua rows | No constraints invented | Same |

**Required change:** make collection of template parameters conditional on
the wrapper, but inspect the callable's own trailing requirement regardless
of that wrapper. Add public-result tests for declarations, definitions, and
constructors, plus the existing wrapped positive controls. Verify narrowing
and cache hits keep both the requirement and the empty owned-parameter list.

### High — Three languages still omit required base relations

**Defect class:** supported source forms remain unimplemented because their
classification decision is unresolved, while tests assert the empty output.

**Blocked on the author decision listed in frontmatter.** The other findings
are not blocked by this decision.

The `tree-hugger` library's
[declared_bases](../../lib/src/file/extract/relations.rs) dispatcher has no
C#, Swift, or Scala reader. The corresponding
[RELATIONS_MATRIX](../../lib/tests/l1/symbols_relations.rs) rows are `Pending`
and explicitly assert that relations remain empty. Those are honest current
behavior tests, but cannot satisfy the specified acceptance requirements.
Current docs correctly call these relations planned; no comment drift was
found here.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C# class base list | `class Child : Base, I {}` with both targets declared locally | Empty extends/implements | Preserve both written targets using the accepted classification |
| Swift class base list | `class Child: Base, P {}` with local class and protocol | Empty lists | Preserve inheritance and conformance |
| Swift struct conformance | `struct S: P {}` | Empty lists | Preserve the declared conformance |
| Swift extension | `extension S: P {}` with a unique local `S` | No conformance attached to `S` | Implement the accepted extension-owner rule |
| Scala base list | `class Child extends Base with T` with local class and trait | Empty lists | Preserve both written targets using the accepted classification |
| Rust, TS, JS, Go, Python, Java, C++, PHP | Existing positive relation rows | Written relation lists populated | Source-reading support exists; resolution defects are grouped above |
| C, Perl, Bash, Zsh, Lua | Existing negative relation rows | Empty lists | Correct for this feature's required scope |

**Required change:** record the author's selected meaning, implement all
three readers and the agreed Swift extension behavior, replace pending rows
with positive and negative assertions, and update docs and outline/cache
tests. If reduced language scope is chosen instead, it must be explicitly
accepted as a scope change; a planned label alone does not complete the spec.

## Input robustness sweep

The changed persisted format is JSON. No YAML/TOML configuration, manifest,
or lockfile reader was added. The strict metrics reader already has an
automated public-output matrix. Supplemental probes checked the newly
populated generic and relation fields through edited caches produced by
`hug`, including nested parameter and target fields.

In the tables, **serve empty** means the schema-defined empty collection,
whose JSON key is omitted; **serve none** means an optional value is absent
from the report. **Recompute** means the edited entry was rejected and the
public result came from fresh analysis. Every control served the populated
value, proving the edited field was load-bearing. Absence and an empty
collection intentionally share a meaning for these defaulted collections;
explicit null does not.

| Shape | `generic_params` | Declaration `constraints` | `extends` | `implements` |
|---|---|---|---|---|
| Populated control | Serve populated | Serve populated | Serve populated | Serve populated |
| Absent | Serve empty | Serve empty | Serve empty | Serve empty |
| Null | Recompute | Recompute | Recompute | Recompute |
| Whole field wrong type | Recompute | Recompute | Recompute | Recompute |
| One element wrong type | Recompute | Recompute | Recompute | Recompute |
| Every element wrong type | Recompute | Recompute | Recompute | Recompute |
| Empty array | Serve empty | Serve empty | Serve empty | Serve empty |
| Duplicate key | Recompute | Recompute | Recompute | Recompute |
| Invalid content | Recompute | Recompute | Recompute | Recompute |

Nested generic-parameter fields:

| Shape | `name` | `kind` | `constraints` | `default` | `variance` | `position` |
|---|---|---|---|---|---|---|
| Populated control | Serve text | Serve Type | Serve bounds | Serve text | Serve text | Serve number |
| Absent | Recompute | Recompute | Serve empty | Serve none | Serve none | Recompute |
| Null | Recompute | Recompute | Recompute | Serve none | Serve none | Recompute |
| Whole field wrong type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| One element wrong type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Recompute | Serve empty | Serve empty string | Serve empty string | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Invalid content | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |

Relation targets use the shared `SymbolRef` reader:

| Shape | Target `name` | Target `id` | Target `qualified_name` | `metrics.lines` |
|---|---|---|---|---|
| Populated control | Serve text | Serve ID | Serve text | Serve count |
| Absent | Recompute | Serve none | Serve none | Recompute |
| Null | Recompute | Serve none | Serve none | Serve null |
| Whole field wrong type | Recompute | Recompute | Recompute | Recompute |
| One element wrong type | Recompute | Recompute | Recompute | Recompute |
| Every element wrong type | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Serve empty string | Serve empty string | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute |
| Invalid content | Recompute | Recompute | Recompute | Recompute |

For scalar fields, element-error probes use an array containing the control
plus a wrong value, or only a wrong value. For `lines`, the automated suite
also checks fractional, negative, and out-of-range numbers. Attributes and
modifiers have existing nine-shape cache matrices: absent/empty are served;
null, wrong types, bad elements, duplicate keys, and invalid content cause
recomputation. No malformed-value coercion was found in this sweep. The
supplemental generic/extends/nested-field columns should become permanent
cases in the existing cache robustness matrix alongside the fixes.

## Requirement-to-verification assessment

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Explicit generics, ownership, bounds, defaults, variance, overflow | Level 1 public-query matrices and overflow tests | C++ default and callable-requirement gaps above |
| Declared relations, lexical identity, Rust impl aggregation and member ownership | Level 1 complete/selective/cache matrices plus review reproductions | Incorrect IDs and pending language readers above |
| Written modifiers and raw attribute order, values, and ownership | Level 1 sixteen-language matrices, flag and cache checks | No additional defect found |
| Callable body spans and line counts, absent bodies, LF/CRLF, expression bodies | Level 1 sixteen-language body matrix and half-open-span formula tests | Appropriate level; no additional defect found |
| Optional-pass avoidance, one definitions query, bounded shared work | Level 1 counters and growth checks | Appropriate level; retain these guards when repairing lookup |
| Query narrowing/widening and actual warm cache hits | Level 1 public query/cache tests | Existing shapes pass; add the newly failing shapes |
| Strict metric keys and exact schema/revision invalidation | Level 1 edited-cache public-output tests | Passing; metrics shape is 4.1 |
| JSON/JSONL/full indexes, selectors, counts, sorting, signature modes | Level 1 shipped-CLI tests | Appropriate level; incomplete or wrong source facts still propagate |
| Styled outline relations and hanging-indent wrapping | Level 2 pane capture in tmux and WezTerm, supported by Level 1 exact-output tests | Appropriate level present; Kitty was not exercised |
| Keyboard, mouse, paste, or input-encoder behavior | No such requirement added | Level 3 is not required |
| Older-model adapter losses and current docs | Level 1 adapter checks and source/document review | Losses documented; planned relations remain an acceptance gap |

All new matrix modules are declared by the library's `tests/l1/main.rs`; the
CLI outline module is declared by its `symbols` test target. Real-terminal
tests have a live `test-l2` recipe and the required feature. No test placement
or tier-selection defect was found. Cross-OS evidence is left to CI and was
not used as a reason to withhold readiness.
