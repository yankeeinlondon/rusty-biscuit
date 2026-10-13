# Invocation Forms

Part of [Scope-Aware References](../references.md).

Every [occurrence](../references.md#occurrences) has a `kind`: `Call` when
the name is what gets invoked, `Usage` for any other read. This page lists,
language by language, which syntax counts as a call.

`kind` is decided from syntax alone, the same way in every language, so a
call is a `Call` even where its binding can only be `NameOnly`. The callee
may be written inside wrappers that evaluate to exactly the wrapped value;
those are looked through. Anything that computes a different value is not:
its names are `Usage`s.

```ts
f();                    // Call
((f))();                // Call: parentheses only restate f
(f as () => void)();    // Call: `as`, `satisfies`, `<T>f` and `f!` are type-only
g((f));                 // f is a Usage: an argument, not the callee
(obj).m();              // obj is a Usage, m is the Call
(0, f)();               // f is a Usage: a sequence computes another value
(c ? f : g)();          // f and g are Usages: either may be called
```

A conditional or nullsafe call (C# `obj?.F()`, TS `obj?.m()`) is still a
`Call`: the syntax invokes the member, though a null receiver may skip it.
PHP callable creation is the opposite: `f(...)` (the literal `...`
placeholder) only makes a closure, so `f` is a `Usage`, while `f(...$args)`
unpacks arguments into a real call. A variable's value is invoked by PHP
`$cb()` and Perl `$cb->()` / `&$cb()`; the variable is the `Call`, and no
initializer is followed to find what it holds.

```php
f(...);          // f is a Usage: creates a closure
consume(f(...)); // consume is the Call, f a Usage
f(...$args);     // f is the Call: unpacking calls
($cb)();         // cb is the Call: a variable's value
(f)();           // f is a Usage: the constant f's value is called
```

| Language | Calls | Wrappers looked through | Never the callee |
|---|---|---|---|
| Rust | `f()`, `::f()`, `(::f)()`, `f::<T>()`, `x.f()`, `Type::f()`, `W(1)`, a value `cb()` | parentheses | `(*p)()`, `name!(…)` (a `Usage`) |
| TypeScript | `f()`, `f<T>()`, `f?.()`, `` f`x` ``, `new F()`, `x.m()`, `@f` | parentheses, `as`, `satisfies`, `<T>f`, `f!`, and an instantiation written apart from its call (`(f<T>)()`, `new (C<T>)()`, `(x.m<T>)()`) | sequence, conditional; an instantiation that is only a value or an argument (`const h = f<T>`, `g(f<T>)`) |
| JavaScript | as TypeScript, without the type forms | parentheses | sequence, conditional |
| Python | `f()`, `x.m()`, `@f`, `@a.b` | parentheses | conditional expression |
| Go | `f()`, `x.m()`, `pkg.F()` | parentheses | `f[T]()`: written like indexing `fs[0]()`, so never read as the callee |
| C | `f()`, `s.cb()`, `p->cb()` | parentheses | `(*f)()`, comma expression |
| C++ | as C, plus `f<T>()`, `ns::f()`, `::f()`, `((::f))()`, `obj.m()`, `obj.template m<T>()`, `new F()`, `new F`, `new ns::C<int>()`, `new (place) C<int>`, braced construction `F{}`, `ns::C<int>{x}` | parentheses | as C; an array `new F[n]` (no constructor is written); a declaration's type `C<int> v{}`; a C compound literal `(S){1}` constructs nothing |
| C# | `f()`, `f<T>()`, `x.m()`, `x!.m()`, `x?.m()`, `x?.M<T>()`, `x?.A()` (the delegate member), `x?.A?.Invoke()` and `cb?.Invoke()` (the last name), `new F()`, `new List<T>()` | parentheses, the null-forgiving `!` at any depth (`cb!()`, `(cb!)!()`, `((obj.F)!)()`, `obj?.F!()`, `obj.F!!()`) | casts; `A` in `x?.A?.Invoke()`; `cb!` that is not invoked (`consume(cb!)`); postfix `++`/`--` |
| Java | `f()`, `this.<T>f()`, `x.m()`, `new F()`, `new a.b.F()`, `new C<String>()`, `new a.b.C<>()`, `outer.new C<String>()`, `new C<String>() {}`, `new a.b.C<String>.D<Integer>()` (only `D`) | none: a method name is not an expression | array creation `new C<String>[2]`; type arguments, qualifiers, the receiver and annotations of a construction |
| Swift | `f()`, `f { }`, `x.m()`, `F()`, `C<Int>()`, `N.C<Int>()`, an implicit member `.f()`, `.init()`, `.some(x)`, and `f` in `!f()` / `-f()` (the operator applies to the call's result) | parentheses, the force unwrap `!` at any depth (`cb!()`, `(cb!)()`, `x.cb!()`, `(cb!)!()`, `(cb!)?()`), beside optional chaining `cb?()` | an implicit member that is not invoked (`let e: E = .none`), `(-f)()` (the negated value is called); a subscript's operand (`a[0]`, `a?[0]`, `x.items[0]`, `!a[0]`, `a[0]()`, `a[0] { … }`): the grammar writes a subscript like a call, but it invokes nothing |
| Scala | `f()`, `f[T]()`, `f { }`, `x.m()`, `a(0)` (Scala has no subscript syntax: it is `a.apply(0)`), `x m 1`, `x m`, `new F()`, `new a.b.F()`, `new C[Int]()`, `new C[Int]`, `new a.b.C[Int] {}`, `C` in `new C with T` | parentheses | the traits of a mixin construction; a declared base `extends C(1)` |
| PHP | `f()`, `N\f()`, `new F()`, `new F`, `F::m()`, `$x->m()`, `$x?->m()`, a variable's value `$cb()`, `new $C()`, and argument unpacking (`f(...$args)`, `$cb(...$args)`) | parentheses around a variable only (`($cb)()`, `(($cb))()`) | callable creation `f(...)`, `\Ns\f(...)`, `W::s(...)`, `$o->m(...)`, `$cb(...)`: it makes a closure and calls nothing; `(f)()`: a bare name in parentheses is a constant, so the call uses the constant's value; `($o->m)()` calls a property's value; `$o->$m()`: `$m` holds the method's name |
| Lua | `f()`, `f "s"`, `f {}`, `x:m()`, `x.m()` | parentheses | |
| Perl | `f()`, `f 1`, `f`, `&f()`, `&f`, `Pkg::f()`, `goto &f`, `$x->m()`, `Pkg->m()`, a code reference `$cb->()`, `$cb->(1)`, `&$cb()`, `&{$cb}(1)` | | `\&f`, `\&$cb`, `defined &f`, `exists &$cb`: they name the sub without calling it; `$cb` in `$cb->m()` is the receiver |
| Bash, Zsh | a command's name (`f`, `X=1 f arg`, `$(f)`, `$cmd arg`) | | an argument (`echo f`), the variable a command name expands (`cmd` in `$cmd`) |

Comments between the constructed name and type arguments do not change the call:
`C /* note */ <Int>()` is a Swift constructor call; `x: C /* note */ <Int>`
is a type usage.

A generic type is a call only where a construction holds it: in
`new C<String>()` (Java), `new C[Int]` (Scala), `C<Int>()` (Swift) or
`new C<int>` (C++) the constructed name is the `Call`, and its type
arguments and qualifiers are usages, while `C<String> x` stays a usage.

Grammar limit: the pinned Swift grammar parses `C<Int>.init()` as the
comparisons `C < Int > .init()`, so `init` is a `Call` and `C` a `Usage`
(the same kinds as `C.init()`), but `init` has no written path.

Grammar limit: the pinned Swift grammar reads the adjacent operators in
`cb!!()` and `cb?!()` as one custom infix operator (`!!`, `?!`) with a
missing operand, so the region is limited by the parse error and `cb` is a
`Usage`; the parenthesized spellings `(cb!)!()` and `(cb!)?()` are calls.

```java
new a.b.C<String>(y);   // C is the Call; a, b, String and y are Usages
C<String> x = null;     // C is a Usage: a type, not a construction
new C<String>[2];       // C is a Usage: array creation calls no constructor
```

Coverage reads a callee the same way. A call whose callee, once these
wrappers and generic instantiations are removed, is a plain name or path
(Rust `(f)()`, `(f::<u8>)()`, `(S::f)()`; TS/JS `((f))()`, `(f)?.()`,
``(f)`x` ``, `new (F)()`; TS `(f as any as () => void)()`, `f!()`,
`(f<number>)()`, `new (C<number>)()`) limits
its region no more than `f()` would: a call of a function the name binds is
`Complete`, so a caller of another function is `is_recursive: No`, never
`Unknown`. A sequence, conditional, dereference, member access or variable
still adds `indirect_dispatch` (see [Coverage](coverage.md)). An `eval`
written inside these wrappers (`(eval)(…)`, `(eval as any)(…)`) is still a
direct `eval`; `(0, eval)(…)`, `globalThis.eval(…)` and an alias
(`const e = eval; e(…)`) are not, and neither are an optional call
(`eval?.(…)`, `(eval)?.(…)`) or a tagged template (``eval`…` ``).

```ts
function f() {}
function g() { (f as () => void)(); new (C)(); } // Complete: g is No
function h() { (0, f)(); }                      // Limited (indirect_dispatch): h is Unknown
class C {}
```

A member callee is a call through any depth of these wrappers:
`((obj.m))()`, `(obj.m as T)()`, `(<T>obj.m)()` and `(obj.m!)()` in
TypeScript, `(x.cb!)()` in Swift, `((obj.F)!)()` in C#, `((obj.m))()` in JavaScript, Go (`((pkg.F))()`), C
(`((s.cb))()`) and C++ (`((obj.template m<T>))()`), and `new ((ns.W))()`.
A member access passed as an argument, inside a sequence or conditional, or
dereferenced (`(*s.cb)()`) is not the callee. In Rust, `(x.f)()` calls the
field `f`, never a method, so `f` is no occurrence.

A member name that is not invoked is an occurrence only where the
language's reference query captures member accesses, and then in every
position it is written. Python, Java, C#, PHP, Lua, Swift and Scala report
it as a `Usage` with its path in an argument and equally as an assignment
or compound-assignment target, under `++`/`--`, `del`/`unset`, address-of,
a unary operator or a cast. TypeScript, JavaScript, C, C++, Go and Rust
report a member name only where it is invoked, so `f` is no occurrence in
any of these:

```ts
use(x.f); delete x.f; ++x.y; x.y += 1;  // only x is an occurrence
x.m();                                  // m is a Call, path x.m
```

Perl hash keys (`$x->{f}`) are not member names. A Lua assignment
`x.y = 1` declares the table member `y`, so `y` there is a record, not an
occurrence.

## Related pages

- [Coverage and limitation reasons](coverage.md): how an uncertain callee
  limits its region.
- [Identity and written names](identity-and-written-names.md#written-paths):
  the written path a call keeps.
- [Language support](language-support.md): the binding rules per language.
