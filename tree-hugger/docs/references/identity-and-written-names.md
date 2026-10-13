# Identity and Written Names

Part of [Scope-Aware References](../references.md).

An occurrence keeps the name exactly as the source spells it, together with
the path or receiver written in front of it. Lookup, on the other hand,
compares names by identity, so two spellings of one identifier match. This
page covers both, and how an occurrence records which import it is bound
to.

```rust
fn r#f() { f(); }        // `f` is a self-call: `r#f` and `f` are one identifier
fn run(other: &P) { other.parse(); } // target name `parse`, qualified_name `other.parse`
```

## Written paths

`target.qualified_name` is the source text from the outermost qualifier,
root or receiver through the name token, so it always ends at the
occurrence's `span`, and is absent for a name written alone. Type arguments
written after the name are outside it; those written inside a qualifier
stay. The span is still the name token alone, and a name is still one
occurrence however long its path.

| Written | Name | `qualified_name` |
|---|---|---|
| Rust `::f()`, `::m::f()` | `f`; `m` | `::f`, `::m::f`; `::m` |
| C++ `((::f))()` | `f` | `::f` |
| C++ `A::B::f()` | `B`; `f` | `A::B`; `A::B::f` |
| C++ `r4::f<int>()`, `N::C<int>::member()` | `f`; `C`; `member` | `r4::f`; `N::C`; `N::C<int>::member` |
| C++ `obj.template m<int>()`, `p->template m<int>()` | `m` | `obj.template m`, `p->template m` |
| C# `new global::N.C<int>()`, `X::C` | `N`; `C` | `global::N`; `global::N.C`, `X::C` |
| C# `obj?.A?.Invoke()` | `A`; `Invoke` | `obj?.A`; `obj?.A?.Invoke` |
| PHP `($o)?->m()`, `\A\B\f()` | `m`; `B`; `f` | `($o)?->m`; `\A\B`; `\A\B\f` |
| PHP `X::K`, `\N\X::K`, `static::K`, `X::$f` | `K`; `f` | `X::K`, `\N\X::K`, `static::K`; `X::$f` |
| Python `[*x.s] = r`, `f(*x.p)` | `s`; `p` | `x.s`; `x.p` |
| Perl `&R4::r4f()`, `R4->r4f()`, `$obj->r4f()` | `r4f` | `R4::r4f`, `R4->r4f`, `$obj->r4f` |
| Java/Scala `a.b.C`, Swift `Swift.Array<Int>`, Go `pkg.T` | `C`; `Array`; `T` | `a.b.C`; `Swift.Array`; `pkg.T` |
| Java `outer.new C()`, `(outer).new C<String>()`, `factory().new C() {}` | `C` | `outer.new C`, `(outer).new C`, `factory().new C` |
| Swift `!x.isEmpty()`, `-x.y.f()`, `-.some.f()`, `(!x).f()` | `isEmpty`; `f` | `x.isEmpty`; `x.y.f`, `.some.f`, `(!x).f` |
| Bash/Zsh `other/external` | `other/external` | absent: the command word is the name |

A Java construction can name an inner class through an enclosing instance
written before `new`. That receiver is part of the written target, and the
`new` between them stays in the path, the same way C++ keeps `template` in
`obj.template m`. Type arguments and an anonymous class body after the name
are not part of it. The receiver keeps its own occurrences: `factory` in
`factory().new C()` is a separate `Call`, `outer` in `holder.outer.new C()`
has the path `holder.outer`, and the keyword `this` is not an occurrence.

```java
outer.new C();          // C: Call, qualified_name "outer.new C"
holder.outer.new C();   // outer: "holder.outer"; C: "holder.outer.new C"
outer.new C<String>();  // C: "outer.new C"; String: Usage, no path
```

A Swift prefix operator applies to the result of the whole member chain
after it, so it is never part of a member's path: `!x.isEmpty()` negates
the result of `x.isEmpty()`, and the path of `isEmpty` is `x.isEmpty`.
Written parentheses change that: in `(!x).f()` the operator's result is the
receiver, so the path is `(!x).f`. An implicit member written after an
operator has no path of its own (`some` in `-.some.f()`), exactly as
without the operator, and the member after it keeps the leading `.`
(`.some.f`). Other languages' grammars already place a prefix operator,
`await`, `try` or a cast outside the member access, so it never enters a
path there. Python's grammar likewise writes the star of an unpacking
assignment target inside the receiver (`[*x.s] = r` as `(*x).s`), but the
star unpacks the whole target, so the path of `s` is `x.s`, as in
`f(*x.s)`.

A path is never evidence of one target: a qualified name that no
[language rule](language-support.md) establishes (`Self::f` and `Type::f`
in Rust, outside the cases the Rust rules prove), and a Java receiver before
`new`, is `NameOnly` or `Unresolved` and adds no reference edge. In
particular a rooted Rust name (`::f`) never reaches
ordinary lookup, so it never binds a `let f` or parameter that shadows the
item, and its root binding is unproved (`::f` is the crate root's `f` in
edition 2015 and the crate `f` from 2018), so a function calling `::f()`
from inside `f` is `is_recursive: Unknown`, never `No`.

## Identifier identity

Reference lookup compares Rust raw identifiers without `r#` and normalizes
Rust identifiers to NFC: `fn r#f() { f(); }` is a self-call. TypeScript and
JavaScript decode identifier Unicode escapes: `function \u0066() { f(); }`
is also a self-call. ECMAScript does not normalize literal Unicode names;
composed and decomposed spellings remain distinct. These rules apply to local
bindings, patterns, generic parameters, overload candidates and import aliases.

Public names, spans and written qualifiers retain their source spelling.
String-named imported exports decode string escapes as one name: `"\u0053"`
names `S`, while `"A.S"` is a single export name containing a dot. A call
through a file-local import still has no external declaration ID and remains
unknown for recursion when its original identity could name the callable.
TypeScript ambient functions and overload signatures retain their own
declaration IDs; multiple declarations of one identity stay uncertain.

## Selected import provenance

An occurrence's optional `import_span` is the name token of the exact import
record that lookup bound it to. It is present only for a `Scoped`
occurrence bound to an import, which has no target ID. Local bindings, declaration IDs, `NameOnly`
suggestions and `Unresolved` names carry no import span. The span remains the
written import token, including raw Rust or escaped JavaScript spellings.

```rust
fn f() { use other::g as q; q(); }
fn h() { use other::f as q; q(); }
```

The call `q()` in `f` is bound to `f`'s own import, of `g`. Because the
occurrence names that exact import, the import of `other::f` in `h`, which
binds the same local name `q`, cannot make `f` look potentially recursive. A
loop or pattern binding named `q` has no import span, even if its owner also
calls an import named `q`.

[Direct recursion](../references.md#direct-recursion) and `hug refs
--callees` read this field rather than matching a call's name against the
record's `dependencies`, which cannot tell two imports of one local name
apart. Schema 5.1 added the field; cache entries written in schema 5.0 are
recomputed (see [Cache Design](../cache-design/robustness.md#selected-import-provenance)).

## Related pages

- [Invocation forms](invocation-forms.md): which syntax is a `Call`.
- [Language support](language-support.md#what-an-import-binds): what each
  import binds.
