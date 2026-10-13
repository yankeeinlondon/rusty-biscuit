# Language Support

Part of [Scope-Aware References](../references.md).

This page holds the per-language rules: what each language resolves, what
an import binds, and which names are never occurrences.

Rust, TypeScript and JavaScript resolve the forms marked **Resolved** below.
JavaScript shares TypeScript's query and lookup rules but has no type names.
Every other language reports occurrences conservatively. Their targets are
never `Scoped`, and every region they own has `Limited` coverage with the
reason `unsupported_language`. A limited language never yields an empty
result that looks complete.

## Rust

| Rule | Behavior | State |
|---|---|---|
| Block vs function scope | Each `block`, function body, closure, `match` arm, `if`/`while`/`for`/`loop` body, inline `mod` body, trait, `const` and `static` is a scope; so is every item with generic parameters, so they never leak out. | Resolved |
| Item and module boundaries | Code inside a nested item (`fn`, `const`, `static`, type, trait, `impl`) sees the items of the blocks around it but not their `let`s, parameters, pattern bindings or generic parameters; an `impl` or trait member still sees its `impl`'s or trait's generic parameters. An inline `mod` sees only what it declares or imports, plus earlier `macro_rules!` macros. | Resolved |
| Declaration order | `let` is visible only after its statement, so `let f = f();` reads the *outer* `f` in its initializer. Items (`fn`, `struct`, `const`, `static`, `mod`, `use`) are visible throughout their block, before or after; a `macro_rules!` macro only after its definition. | Resolved |
| Shadowing | The latest preceding `let` of a name in a block wins; an inner scope's binding hides every outer one. | Resolved |
| Parameters and patterns | Function and closure parameters, including destructured ones, bind in the body; `while let`/`match` arm/`for` pattern bindings bind in their arm or body; an `if let` binding (and each `let` of a chain) only from its `let` to the end of the consequence, never in `else`; a `let … else` binding after the statement, never in its `else` block. A struct-pattern field shorthand (`S { x }`) is a pattern name like any other. | Resolved |
| Pattern names: binding or constant | Spelling decides nothing: Rust's naming conventions are lints. A pattern name written with `mut`, `ref` or `name @ …` always binds. A bare name, in any pattern (`let`, `let … else`, parameters, closure parameters, `for`, `match` arms, `if let`, `while let`, nested in tuple, struct, slice, or `&` patterns), is looked up where it is written, as rustc does: when it reaches a `const`, a unit struct or a `const` generic parameter, the pattern **reads** that declaration (a `Usage`, `Scoped` to it), and so does every use of the name in the arm or body; when it reaches a function, a local or nothing, it **binds** a new local, so `fn F(x: fn()) { if let F = x { F(); } }` calls the local (a call through a value) and is not recursive. `None` (the prelude's unit variant) and `Self` never bind. When an import or glob `use` is reached first, the file cannot tell whether it supplies a constant: in a refutable pattern (`match`, `if let`, `while let`, `let … else`) the name is then a `NameOnly`/`Unresolved` usage and every use of it is uncertain; in an irrefutable one it binds, because only a unit struct or a single-valued constant could be matched there. Declarations follow the same rule: a pattern name that reads a declaration, or whose identity is unknown in a refutable pattern, is never listed as a variable by `hug symbols` (no declaration is proved), and neither is the path of a tuple-struct pattern (`Some` in `Some(x)`), a range bound or path pattern in `if let` (`MAX` in `1..=MAX`, `E::A`) or a TypeScript/JavaScript parameter default (`a` in `b = a`). A binding nested in a pattern with `mut`, `ref` or `@` (`Some(mut x)`), or a bare `match` arm name, binds but has no declaration record either (the declaration query does not capture those forms); `let mut x` and a `mut x` parameter are listed. | Resolved; unknown identity through an import or glob |
| Name categories | Types (`struct`, `enum`, `trait`, `type`, `mod`, generic type parameters) and values (`fn`, `const`, `static`, `let`, parameters) are separate. A tuple or unit struct is in both, and macros are their own category. A `use` binding enters every category, because the file alone cannot tell which one it names. | Resolved |
| Imports and aliases | `use a::b as c;` binds `c` locally (`Scoped`, no target ID), and so does every name a `use` brings in: `use a;` binds `a`, `h` in `use a::{g::h};`, `a` for the `self` in `use a::{self, b};`, and `z` in `extern crate y as z;` (never `y`). The import record keeps the source path and the original name `b`. A glob `use a::*;` never binds a unique target: it is an [on-demand import](#what-an-import-binds) record and binds nothing. | Resolved |
| Associated items | Methods, fields, variants and associated consts are not lexical names: a bare `f()` never reaches an `impl` member. `impl` and trait headers resolve in the enclosing scope, never in the impl body. | Resolved |
| Paths | `Self::f()` and `Type::f()` are `Scoped` only when `Type` (or the self type of the `impl` around `Self`) resolves to one in-file struct or enum and exactly one inherent `impl` of it declares `f`; `Enum::Variant` resolves the same way. Any other path keeps its written qualifier and is `NameOnly` or `Unresolved`; so is a rooted path (`::f`, `::m::f`), which is never looked up lexically and so never binds a shadowing `let` or parameter. Calling a path that names a value (`Self::C()` or `(S::C)()` with `const C: fn() = Self::f`), or whose candidates include a value, keeps the binding or candidates but is a call through a value: the constant's initializer is never followed. | Resolved; Limited (`indirect_dispatch`) for calls of a value |
| Method calls | `x.f()` and `self.f()` are `NameOnly` with in-file `f` methods as candidates: trait dispatch and auto-deref can select another implementation. | Limited (`indirect_dispatch`) |
| Nested callables | A closure is an anonymous owner. A nested `fn` is its own owner and is visible in its enclosing block. Calling a variable or parameter (a closure held in a `let`) is a call through a value. | Limited (`indirect_dispatch`) for calls through a value |
| Macros | `name!(…)` is a `Usage` of the macro. Its expansion is unknown, so the enclosing region's call coverage is limited. | Limited (`unexpanded_macro`) |

## TypeScript and JavaScript

| Rule | Behavior | State |
|---|---|---|
| Block vs function scope | `let`, `const`, `class` and block-level `function` are block scoped; `var` is function scoped. | Resolved |
| Declaration order | A block's `let`/`const`/`class` hides the outer name for the **whole** block, even before its declaration (the temporal dead zone). `function` and `var` are hoisted. | Resolved |
| Parameters | Plain, default, rest and destructured parameters bind in the function and are visible to later defaults and to the body. A default is evaluated before the body runs: it sees the parameters and everything outside the function, never a declaration of the body (`let`, `const`, `class`, `function` or `var`). When the parameter list holds an expression (a default, or a computed key in a destructured parameter), the body's `var`s form their own scope, so in `function g(x = f()) { var f = () => {}; }` the default calls the outer `f`, while every `f` in the body, before or after the `var` and in closures, is the body's `f`, and a body `var` named like a parameter is the body's own copy. Without such an expression a `var` named like a parameter declares the parameter again, so a use of that name is `NameOnly` with both declarations as candidates. A direct `eval` in the body limits only the body, never the defaults. | Resolved |
| Expression names | A named function, generator or class expression (`const g = function f() { f(); }`) binds its name only inside itself: its parameters, body and, for a class, its `extends` clause. Inside, `f` is `Scoped` with no target ID (the expression has no declaration record) and hides any outer `f`; outside, `f` is the outer declaration. A self-call through that name proves nothing about an outer function of the same name, which keeps its own recursion verdict. | Resolved |
| Calls through values | Calling a `const` bound to a function or class expression calls that function. Calling any other variable or parameter is a call through a value. | Limited (`indirect_dispatch`) for calls through a value |
| Loop and catch bindings | `for (let …)`, `for…of`/`for…in` and `catch (e)` bind in their statement. | Resolved |
| Export clauses | A local export reads its binding: `k` in `export { k }`, `export { k as j }` and `export = k` is `Scoped` to `k`. The exported name (`j`, `default` in `export { k as default }`) names the module's export and is no occurrence, and nothing in a re-export from another module is one either (`export { k as j } from "m"`, `export * as ns from "m"`, `export { default } from "m"`, TypeScript `export type { T } from "m"`), since those names are the other module's exports, never a local `k`. TypeScript `export as namespace X` declares a global name (no occurrence); `export import A = N.S` binds `A` like `import A = N.S` and reads `N`. Export records and their re-export flags are unchanged. | Resolved |
| Name categories (TypeScript) | `interface` and `type` are types only; `class`, `enum` and `namespace` are both; `function`, `var`, `let` and `const` are values. Type-only imports (`import type { F }` or `import { type F }`) bind only the type category: a local `function F() { F(); }` remains a proved self-call while `let x: F` uses the import. `typeof F` queries the value category even inside a type expression. | Resolved |
| Imports and aliases | `import { a as b }` binds `b`; `import * as ns` binds `ns`; a default import `import d from "m"` binds `d`; TypeScript `import q = require("m")` binds `q` and `import R = N.S` binds `R` (`N` is read). The original name is kept in the import record, never bound and never an import of its own. | Resolved |
| Members | Class and interface members are not lexical names. `this.f()` and `obj.f()` are `NameOnly`. | Limited (`indirect_dispatch`) |
| Class field initializers and static blocks | References resolve lexically, but the owner is absent and the region limited. | Limited (`ownership_unknown`) |
| Decorators | Evaluated in the scope enclosing the class, and owned by it. | Resolved |
| Dynamic access | `obj[name]()` and other calls of computed values never bind a unique target. A name inside a `with` body that the body does not declare, and a name inside a function holding a direct `eval(…)` that the function does not declare, is never `Scoped` (`NameOnly` or `Unresolved`), so it proves no caller and no recursion. An optional `eval?.(…)` or tagged ``eval`…` `` is no direct `eval` and hides no name. | Limited (`indirect_dispatch`; `unsupported_binding_form` for a direct `eval` and `with`; an optional `eval` only `indirect_dispatch`, a tagged one nothing) |

## Every other language

| Language | Scope captures today | Occurrences reported | State |
|---|---|---|---|
| Go | blocks, functions, `if`/`for`/`switch`, func literals | identifiers, call targets, package selectors, selector callees (`x.m()`, `pkg.F()`) | Limited (`unsupported_language`) |
| Python | module, functions, classes, comprehensions | identifiers, call targets, attribute objects, decorators | Limited (`unsupported_language`) |
| Java | blocks, methods, constructors, lambdas, loops, `catch` | identifiers (including method names), annotations, `new` types | Limited (`unsupported_language`) |
| C# | blocks only | identifiers, invocation targets, `new` types, attributes | Limited (`unsupported_language`) |
| C | translation unit, functions, compound statements, loops | identifiers, call targets, field-access objects, field callees (`s.cb()`) | Limited (`unsupported_language`) |
| C++ | as C, plus classes, namespaces, lambdas, templates, `catch` | as C, plus namespace qualifiers and method callees (`obj.m()`, `obj.template m<T>()`) | Limited (`unsupported_language`) |
| Scala | blocks, functions, lambdas, template bodies, `for` | identifiers, call targets, annotations | Limited (`unsupported_language`) |
| Swift | statements, functions, types, control flow | identifiers, call targets, navigation targets | Limited (`unsupported_language`) |
| PHP | classes, interfaces, traits, enums, functions, methods | names, variables, call targets, class constants | Limited (`unsupported_language`) |
| Lua | chunk, functions, control-flow blocks | identifiers, call targets, table objects | Limited (`unsupported_language`) |
| Perl | blocks, named subs | variables, bareword calls, method names, package names | Limited (`unsupported_language`) |
| Bash, Zsh | function bodies only | variable expansions, command names, words | Limited (`unsupported_language`) |

The scope captures exist, but several of them sit on declaration or import
nodes rather than bodies (Python imports, C++ function declarators, PHP
classes). None of these languages captures call targets separately from
other uses, so their scope trees and call coverage cannot yet support
`Scoped` results. Their occurrences are `NameOnly` with every same-named
declaration in the file as a candidate, or `Unresolved`; `kind` is `Call`
where the invocation syntax shows it.

A name that only reads something is an occurrence in these languages too,
never a declaration that would hide it: a parameter's type and default
value (`def g(x=external)` in Python, `void g(W x, int y = external)` in
C#), a PHP constant's value (`const B = A;`) and a PHP parameter default
are listed as usages, and the only candidates of `external`, `W` or `A` are
their real declarations. `hug refs external` therefore selects the function
alone, not an invented parameter of the same name.

A Java qualified name is an import only inside an `import` clause, and only
its final segment is: `import a.b.C;` imports `C`, `import a.b.*;` imports
everything in `b`, and `import static a.b.C.m;` imports `m`. Every other
qualified name is read where it is written, with its
[written path](identity-and-written-names.md#written-paths), exactly like a qualified type (`p.q.A x;`): a
qualified annotation, and every operand of a module directive.

```java
class A {}
@p.q.A class G {}
// ^ ^ ^ `p` and `q` are usages; `A` is a usage written `p.q.A`,
//       a suggestion for the class `A`. The file imports nothing.
```

```java
module m {
    requires a.b;
    exports p.q to x.y, z.w;
    uses a.b.C;
    provides a.b.C with d.e.D, f.g.F;
}
// every segment is a usage (`C` written `a.b.C`, `D` written `d.e.D`, …);
// `hug imports` lists nothing for this file
```

A C# `using` directive imports one name, and every other name in it is
read, the same way: `using A.B;` and `using static C.D;` import their last
segment (`B`, `D`; the record's `source` is the whole path), `global using`
and `using global::A.B;` likewise, and an alias directive imports its alias
(`using E = F.G;` imports `E` from `F.G`, whose `F` and `G` are reads; a
pointer or tuple target such as `using unsafe P = int*;` has no original
name). C# is limited, so a later `E` is an `Unresolved` usage, never bound
to the import.

```csharp
using A.B;      // `A` is a usage; `B` is the import, no occurrence
using E = F.G;  // `E` is the import; `F` and `G` (written `F.G`) are usages
```

A Swift selective import (`import struct Foo.Bar`, and likewise with
`typealias`, `class`, `enum`, `protocol`, `func`, `var` or `let`) imports
the one name it binds: the record is named `Bar`, its `source` is the path
`Foo.Bar`, and the module `Foo` is a usage. A whole-module import names the
module path it imports (`import Foo.Sub` is the import `Foo.Sub`), and no
segment of that path is an occurrence.

```swift
import struct Foo.Bar  // `Bar` is the import; `Foo` is a usage
import Foo.Sub         // the import `Foo.Sub`; neither segment is an occurrence
let z = Bar            // a usage (Swift is limited: `Unresolved`, never bound)
```

## What is not an occurrence

An occurrence is a read of a name. These are not reads, so they are never
occurrences in any language:

- Declaration names, including the name of a TS/JS named function,
  generator or class expression, and declarations no symbol record reports,
  such as a generic parameter or a pattern binder (see
  [Declaration names in the other languages](#declaration-names-in-the-other-languages)).
- Built-in syntax markers that only look like names, such as Java's
  inferred-type `var` or a Lua `<const>` (see
  [Syntax markers in the other languages](#syntax-markers-in-the-other-languages)).
- Comments, strings and attributes.
- Rust lifetimes, loop and block labels, and the Rust `Self` keyword.
- The wildcard `_` in Rust, Go, Swift and Scala: `_ = f()` discards a value
  and names nothing. In Python, JS/TS, Lua, C/C++ and C# `_` is an ordinary
  name, so a read of it is an occurrence.
- Everything inside a Rust `use` or TS/JS `import` statement or a TS/JS
  re-export (`export … from "m"`), and the exported name of a TS/JS export
  (`j` in `export { k as j }`).
- The name any import binds. In the other languages an import's qualifier
  segments are reads (`a` and `b` in Java `import a.b.C;`, Scala
  `import a.b.C` or Python `from a.b import c`, `b` in Python `import a.b`);
  only the bound name is excluded (see
  [What an import binds](#what-an-import-binds)).

Two captures of the same token are one occurrence; two calls on one line
are two. Occurrences are sorted by span, then owner span, kind, written
target and confidence, so the result never depends on the order the queries
match in.

## What an import binds

An import record (`hug imports`, `FileSymbolIndex::imports`) is the one name
an import brings into the file, at that name's token. Its `source` is the
path as written, and an alias also keeps the `original_name` it renames. The
token is never an occurrence, so an import that recorded a name it does not
bind would hide a real read, and one it missed would leave its name as a
read. An **on-demand import**, which binds no single name (`import a.b.*`),
is one record named by the last segment of the path it opens, with that name
as `alias` and `*` as `original_name`.

| Language | Import records (name ← written form) | Reads |
|---|---|---|
| Rust | `a` ← `use a;`; `h` ← `use a::{g::h};` (`source` `a::g`); `f` ← `e as f` (original `e`); `a` ← the `self` in `use a::{self};`; `z` ← `extern crate y as z;` (original `y`); on demand: `b` ← `use a::b::*;` (`source` `a::b`) | none: nothing inside a `use` is an occurrence |
| TypeScript, JavaScript | `d` ← `import d from "m"`; `b` ← `{ a as b }` (original `a`); `ns` ← `* as ns` (original `*`); TypeScript `q` ← `import q = require("m")`, `R` ← `import R = N.S` (original `N.S`) | `N` in `import R = N.S` |
| Python | `a` ← `import a.b` (`source` `a.b`: only `a` is bound); `c` ← `import a.b as c` (original `a.b`); `y` ← `from m import x as y` (original `x`); `annotations` ← `from __future__ import annotations` (`source` `__future__`); on demand: `b` ← `from a.b import *`, `.` ← `from . import *` | every other module path segment (`b` written `a.b` in `import a.b`, `a` and `b` in `from a.b import c`) and an aliased original (`x`) |
| Scala | `C` ← `import a.b.C` (`source` `a.b.C`); `b` and `d` ← `import a.b, c.d`; `D` ← `import a.{D}` (`source` `a.D`); `F` ← `E => F` or `E as F` (original `E`), including unbraced `import a.E as F` (`source` `a.E`); symbolic names and qualifiers work too (`import a.+`, `import a.+.C`, `import a.{C as ++}`); backticks stay as written; a hidden alias (`C as _`) binds nothing; on demand: `b` ← `import a.b.*` or `a.b._`, `a` ← `import a.given` or a selector wildcard `a.{_, given T}` (original `given` when it opens only given instances) | qualifiers (`b` written `a.b`), a rename's original, a hidden selector (`x` in `x => _`), a `given` selector's type |
| PHP | `A` ← `use A;`; `B` ← `use A\B;` (`source` `A\B`); `C` ← `use A\B as C;` (original `B`); `B` and `D` ← `use A\{B, C as D};` (`source` `A\B`, `A\C`); `use function` and `use const` alike | qualifiers and an aliased original |
| Go | `fmt` ← `import "fmt"` or the raw-string `` import `fmt` `` (`source` the path without quotes); `f` ← `import f "a/fmt"` (original `fmt`); `_` ← `import _ "a/fmt"` (a blank import binds nothing); on demand: `fmt` ← `import . "a/fmt"` | none (the path is a string) |
| Java, C#, Swift | see [Every other language](#every-other-language) | |
| C, C++, Lua, Perl, Bash, Zsh | none: `#include`, `using namespace`, `require`, `use` and `source` are not import records | their operands are ordinary occurrences |

```scala
import a.b.C              // `C` is the import; `a` and `b` (written `a.b`) are usages
import a.{D => E, F}      // `E` and `F` are imports; `D` is a usage
import a.b.*              // an on-demand import named `b`; `a` is a usage
import a.*                // an on-demand import named `a`; no qualifier read
import static a.*         // the same on-demand record convention
```

```python
import a.b         # `a` is the import (source `a.b`); `b` is a usage written `a.b`
from c import *    # an on-demand import named `c`
```

Within a supported import form, comments do not change its bound names. A comment is never an
import's name or original, it never ends a path early, and in Scala it never
decides whether a path segment is the bound name or a qualifier. A `source`
or written path that spans a comment keeps it as written; a TypeScript
entity alias's original is the target without its comments, because its
last segment is what [direct recursion](../references.md#direct-recursion) compares.

```typescript
import D /* note */ = N.S;  // `D`, original `N.S`
import E = N. /* note */ S; // `E`, original `N.S`
```

```scala
import a /* note */ .b.C   // `C`, source `a /* note */ .b.C`; `b` written `a /* note */ .b`
import a.b. /* note */ *   // an on-demand import named `b`, source `a.b`
import C /* note */, d.E   // `C` and `E` are both imports
```

## Declaration names in the other languages

The token that declares a name is never an occurrence, even when the
declaration is not reported as a symbol record. Only the reads around it
are listed:

```java
class T {}
class G<T> { T f(T x) { return x; } }
//      ^ declares the type parameter: not an occurrence
//           ^   ^ read `T`: usages (suggestions for the class `T`)
```

A declared name written as a path declares every one of its segments, so
no segment is an occurrence: a package, module or namespace path (Java
`package a.b;` and `module a.b`, Scala `package a.b`, Perl `package A::B;`,
PHP `namespace A\B;`, C# `namespace A.B`, C++ `namespace A::B` and
`namespace A::inline B`, TypeScript `namespace A.B` and `module A.B`) and a
Swift whole-module import path. A C++ type declared through a qualified
name (`union A::U {}`, `enum class A::E {}`) declares only its last name;
the qualifier `A` reads the enclosing scope. This holds in TypeScript too,
whose namespace prefix is never bound to a same-named class:

```typescript
class A {}
namespace A.B { export function f() {} } // `A` declares: no occurrence
let x: A;                                // reads the class: Scoped to `A`
```

The declaration sites below are recognized by their syntax, never by
spelling (except Scala's own rule that a lower-case pattern name binds and a
capitalized one, or one in backquotes, reads a stable identifier). Bounds,
constraints, defaults, iterables, initializers, computed keys and later
uses stay occurrences, a bare name included: in `int k = xs;` only `k` is
the declaration, and `xs` is a usage.

| Language | Declaration names that are not occurrences, beyond reported declarations |
|---|---|
| Java | type parameters (`<T extends Bound>`), a compact constructor's name, statement labels, pattern binders (`o instanceof P p`, `case String s`, record pattern components), `try` resources, annotation types (`@interface A`, top-level or nested; a later `@A` reads) and their elements, module names (`module m`, `open module a.b`; directive operands read), package names |
| Scala | type parameters (also variant, higher-kinded, polymorphic-function and type-lambda ones, `[X] =>> F[X]`), named context bounds (`[T: Ord as o]`), abstract `type` members, `given` names, package clauses (statement, stacked and braced) and package objects, an import rename's new name (`import a.{b => bb, c as cc}`: `b` and `c` read), named tuple element names (`(x: Int)`), lower-case names bound by `val`/`var` patterns, `case` patterns (`x @ …` binders whatever their spelling) and `for` enumerators (`x <- xs`, `y = …`) |
| Go | type parameters, a type switch's `t :=`, a `select` case's `m := <-ch` (not `m = <-ch`) |
| Python | `from __future__ import` feature names and aliases (an aliased feature `division as d` reads `division`), PEP 695 type parameters (`T`, the name of `T: Bound`, `*Ts`, `**P`), a `type` alias's name, lambda parameters, `*args`/`**kwargs` with annotations, `match` captures (sequence, mapping values and `**rest`, class pattern arguments, `as` aliases), assignment, `for` and `with … as` targets including starred and nested ones. `del`, `global`/`nonlocal` names and augmented assignment targets stay occurrences |
| C++ | defaulted and pack type parameters, template-template parameter names, range-`for` variables, structured bindings (`auto [a, b]`), namespace aliases and namespace names, `struct`/`class`/`union`/`enum` names that are defined or declared alone (`enum class E;`, `enum class G : int;`; an explicit specialization `S<int>` reads `S`), destructor names in declarations (`~C()`, `C::~C()`), lambda init-captures (`[z = xs]`) |
| C# | `catch` variables, declaration and designation patterns (`x is C p`, `x is var v`, `case C q`, `{ … } named`), delegate and destructor names, `static` constructor names, event names with accessors (`event Action E { add {} remove {} }`; in `IFace.E` only `E`), `extern alias` names (a later `A::T` reads `A`), namespace names, statement labels, query variables (`from`, `let`, `join`, `into`), tuple element names |
| Swift | macro names (`macro foo() = …`; a call `foo()` reads), precedence group names (`precedencegroup P`; `higherThan: P` and an operator's `: P` read), external parameter labels and `_`, closure parameters, parameter packs (`each T`; `repeat each T` reads), `let`/`var` pattern names (`if let`, `guard let`, `while let`, `case let`, tuple patterns, `for (i, j) in`), setter and observer parameter names, capture-list initializers (`[k = o]`; `[weak self]` reads), enum associated-value and subscript labels |
| PHP | namespace names, promoted constructor parameters (`private int $p`, `&$p`), variadic parameters (`...$xs`), `catch` variables, `static` variables, destructuring targets (`[$a, $b] = …`, `list($a, $b)`, in `foreach` too; a key before `=>` reads) |
| Perl | package names (`package A::B;`, `package A::B v1.0 { … }`), `for my $x`/`foreach our $x` iterators (a `for $x` without `my`/`our` reads), signature parameters |
| Lua | literal table keys (`{ field = v }`; `{ [key] = v }` reads the key), labels (`::name::`) |
| Bash, Zsh | the operands of `declare`, `typeset`, `local`, `export` and `readonly` (and Zsh `integer`, `float`) after their options: `local x`, `typeset +x y`, `declare -f g`. Only a declaration with a value (`local x=1`) is a reported record, so a bare `local x` is excluded without one. An expansion operand (`local $v`) reads, and so does any word passed to an ordinary command (Bash has no `integer` builtin, so `integer i` there is a command and `i` a usage) |
| C | `struct`/`union`/`enum` names that are defined or declared alone (also reported records); an elaborated use (`struct S x;`, a parameter `struct S *p`, `typedef struct S T;`) reads `S` and is no record |

Recognizing a declaration adds no record, no candidate and no binding: the
language stays `Limited (unsupported_language)` and its reads stay
`NameOnly` or `Unresolved`.

## Syntax markers in the other languages

Some tokens the reference queries capture as names are built-in syntax: they
read no binding, so they are never occurrences and never suggestions for a
declaration that happens to share their spelling. The same spelling in a
name role is still an occurrence:

```lua
local const = 1
local x <const> = const
--       ^ the variable attribute: not an occurrence
--                ^ reads the local `const`: a usage
```

```java
void g() { var x = factory(); int var = 1; use(var); }
//         ^ the inferred-type marker: not an occurrence
//                                              ^ reads the variable `var`: a usage
```

Each marker is recognized by its grammar role. Where that role also holds
real names, only the one spelling the language reserves there is a marker:

| Language | Markers that are not occurrences | Still occurrences |
|---|---|---|
| Java | a bare `var` as the type of a local variable (with or without `final`), an enhanced `for` variable, a `try` resource, an explicitly typed lambda parameter (`(var z) -> z`) and a record pattern component (`P(var a)`) | a variable named `var` and its reads; `var` as a field, parameter or return type, or qualified (`a.var`) |
| Lua | the attribute of a local variable (`<const>`, `<close>`) | the initializer and every other read of a local named `const` or `close` |
| C# | `global` in `global::X.Y` (also in `using global::System.Text;`) | the path's names (`X`, `Y`), an `extern alias` qualifier (`A::T`) |
| PHP | `self`, `static` and `parent` (any letter case) as a type (`: static`, `?self`, `self\|null`), after `new` and after `instanceof` | a method named `static`, `self` or `parent` and its calls (`$this->static()`, `static::static()`), a constant `C::self` |
| Swift | in a `precedencegroup` body, each directive name (`associativity`, `assignment`, `higherThan`, `lowerThan`) and the value of `associativity` (`left`, `right`, `none`); every word of a pre-Swift 3 operator body (`infix operator +++ { associativity left precedence 140 }`); the platforms of `#available(iOS 13, *)`; the name of a built-in attribute (`@objc`, `@escaping`, `@_spi`; list below), every identifier argument of `@available`, `@objc`, `@convention`, `@inline` and `@exclusivity`, and an attribute argument's label (`of:`); the operands of an `#if`/`#elseif` condition (`os(macOS)`, `DEBUG`, `canImport(UIKit)`) | the group named after `higherThan` or `lowerThan`, an operator's group (`infix operator +++ : P`), a property wrapper, result builder or global actor name (`@W`, `@MainActor`), `@Sendable`, the arguments of any other attribute (`@Wrapper(message)`, `@derivative(of: f)`'s `f`), and any variable named like a marker (`left`, `macOS`, `DEBUG`), with its reads |
| Bash, Zsh | the options of `declare`, `typeset`, `local`, `export` and `readonly` (`-i`, `+x`, through `--`) | any word passed to an ordinary command (`grep -i`); the operands after the options are declaration names (see [Declaration names in the other languages](#declaration-names-in-the-other-languages)) |

Java has reserved `var` as a type name since Java 10, so no class may be
called `var` and a bare `var` in these slots is the marker. Source written
for Java 9 or earlier could name a type `var`; a file that declares one
(a class, interface, enum, record, annotation type or type parameter) or
imports one by name with an ordinary single-type import (`import a.b.var;`)
can only be such source, so there every `var` in these slots stays a usage
of that type:

```java
class var {}
class R { void g() { var x = make(); } } // `var` is a usage of the class
```

Other imports prove nothing, and the marker is excluded: an on-demand import
(`import a.b.*;`, `import static a.B.*;`), an import that writes `var` only as
a qualifier (`import p.var.T;`), and a static import, even one ending in
`var`. A static import names a static field or method as readily as a
member type, so it is no evidence of a type:

```java
import static p.Methods.var;
record G() {
    void g() { var x = var(); }
    //         ^ the inferred-type marker: not an occurrence
    //                 ^ calls the imported method `var`: a call
}
```

In the slots where Java never infers a type (fields, method
parameters, return types) a bare `var` is always a usage.

PHP reserves `self`, `static` and `parent` as class names, so in a type,
`new` or `instanceof` operand they always mean the class around the code;
`static::f()` and `self::K` write them as a scope, which was never an
occurrence. C#'s implicitly typed `var`, C++ `auto` and the modifiers of
every other language are not captured at all. A contextual keyword that can
name a declaration stays an occurrence: C# `dynamic` and `nameof`, Go `any`
and `comparable`, Python's soft keywords `match` and `type` used as names.
Swift's `Self` also stays a usage: a declaration can take that name only in
backquotes (``func `Self`()``), so it never becomes a suggestion.

A Swift attribute's name slot holds both built-in attributes and real types
(a property wrapper `@W`, a result builder, a global actor `@MainActor`, an
attached macro), so a name there is a marker only when it is spelled as one
of the compiler's built-in attributes, which the compiler matches before it
looks for a custom attribute. The list is the Swift 5.10 one (the language
the pinned grammar parses): `available`, `objc`, `nonobjc`, `objcMembers`,
`discardableResult`, `escaping`, `autoclosure`, `convention`, `inline`,
`inlinable`, `usableFromInline`, `frozen`, `main`, `testable`, `unknown`,
`unchecked`, `retroactive`, `preconcurrency`, `exclusivity`,
`dynamicCallable`, `dynamicMemberLookup`, `propertyWrapper`,
`resultBuilder`, `globalActor`, `attached`, `freestanding`, `backDeployed`,
`specialize`, `derivative`, `differentiable`, `transpose`, `noDerivative`,
`storageRestrictions`, `requires_stored_property_inits`,
`warn_unqualified_access`, `NSCopying`, `NSManaged`, `NSApplicationMain`,
`UIApplicationMain`, `LLDBDebuggerFunction`, `GKInspectable` and the `IB…`
attributes (`IBAction`, `IBDesignable`, `IBInspectable`, `IBOutlet`,
`IBSegueAction`), plus every `_`-prefixed name (`@_spi`). A spelling that
also names a real type stays a usage: `@Sendable` (the `Sendable` protocol)
and `@MainActor`. Only the arguments of `@available`, `@objc` (selector
pieces), `@convention`, `@inline` and `@exclusivity` are all keywords; any
other argument is an expression and stays a read except for its label
(`@derivative(of: f)` reads `f`, `@_spi(Internal)` keeps `Internal`):

```swift
@objc func h() {}
// ^ a built-in attribute: not an occurrence
struct S { @W var q: Int }
//          ^ a property wrapper type: a usage
```

### Argument and tuple labels

A label is not a read either. A named argument names a parameter of the
callee and a tuple label names an element, so neither reads a binding in
view, while the value written after it does:

```python
x = 1
f(x=x)
# ^ the label: not an occurrence
#   ^ reads the module's `x`: a usage
```

| Language | Labels that are not occurrences | Still occurrences |
|---|---|---|
| Python | a keyword argument's name (`f(x=v)`, `class K(metaclass=M)`) | the value |
| C# | a named argument (`f(x: v)`) and a named attribute argument (`[A(x: 1)]`) | the value; a member assigned in an attribute or object initializer (`[A(X = 2)]`, `new C { X = 1 }`) |
| PHP | a named argument (`f(x: v)`) | the value |
| Swift | a call's argument label (`f(x: v)`), the labels of a function reference (`f(x:)`, `#selector(f(x:))`), tuple labels in an expression (`(x: v)`), a type (`(x: Int)`) and a pattern (`case (x: let a, y: _)`) | the value, a member (`t.x`), a dictionary key (`[k: v]`) |
| Scala | a named argument, an assignment written directly in an argument list (`f(x = v)`, `new C(x = v)`) | the value; a parenthesized assignment (`f((x = 2))`) |
| Perl | the word before a fat comma (`x => v`), which Perl quotes | the value; a call `x()` |
| Go | a field key of a composite literal whose type is a written `struct { … }` | the value; a map key (`map[K]V{X: 1}`) and the key of a literal of a named type (`T{X: 1}`), which the grammar cannot tell from a map key or array index |

Rust, TypeScript and JavaScript capture no field or property key (`S { x: y }`,
`{ x: y }`), and their shorthand (`S { x }`, `{ x }`) is a read. C and C++
designators (`.x = v`) and Lua table keys (`{ x = v }`) are never captured.

Like a declaration name, a marker adds no record, no candidate and no
binding, and changes no coverage: the language stays
`Limited (unsupported_language)`.

## Related pages

- [Name lookup](name-lookup.md): the scope walk these rules plug into.
- [Invocation forms](invocation-forms.md): which syntax is a `Call`.
- [Coverage and limitation reasons](coverage.md): what `Limited` means for
  a region.
