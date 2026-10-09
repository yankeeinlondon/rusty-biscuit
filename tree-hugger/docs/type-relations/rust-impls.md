# Rust Impls

Part of [Declared Type Relations](../type-relations.md).

In Rust, a type's trait implementations are not written in its header but in
separate `impl` blocks. Tree Hugger reads each `impl` to add a type-level
`implements` summary to the type it is for (its **self type**), to record on
each member which trait supplied it, and, with the Containment capability,
to decide which type the members belong to.

```rust
use std::fmt;
pub struct Local;
impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { Ok(()) }
}
impl Local { pub fn new() -> Self { Local } }
```

```console
$ hug types --tree impls.rs
impls.rs
  public struct Local implements fmt::Display [2:1]
```

`fmt` carries `"implements": [{"name": "fmt::Display"}]` as well, and `new`
carries no trait.

## How the self type is found

Each `impl` block in a Rust file is read once, empty ones included
(`impl Marker for Local {}` has no members but still says something). Its
self type is resolved by the [name lookup rules](./name-lookup.md), using the
full written path. `Gen<T>` and `crate::m::W<T>` match their base declaration. A reference,
tuple, `dyn` or other composite self type (`&Local`) attaches to no
component type. Every generic parameter in view of the impl counts, not only
its own: those of the functions, impls and traits around it too, so
`fn outer<T>() { impl Base for T {} }` never attaches to a file-level
`struct T`. The self type and the trait are looked up from the impl's
position like a base: a same-named enum or trait in a finished block is out
of view, and a parameter of the enclosing function never hides a type.
The impl's own parameters are in view only inside the impl: `impl<Base>
Holder<Base> {}` does not hide a file-level `trait Base` from a declaration
before or after it, and a `const` parameter is a value, so it never hides a
type even inside (`impl<const Shape: usize>`, or a trait's
`<const Shape: usize>` around an impl in its method, still attaches to the
file-level `Shape`).

```rust
pub struct Shape;
pub trait Draw {}
fn f(Shape: u8) {
    { enum Shape {} trait Draw {} }
    impl Draw for Shape { fn draw(&self) {} }   // the file-level Shape and Draw
}

fn g() {
    mod shapes { pub struct Shape; pub trait Draw {} }
    impl Draw for shapes::Shape { fn draw(&self) {} }   // g's shapes::Shape, the file-level Draw
    impl shapes::Draw for Shape {}                        // the file-level Shape, g's shapes::Draw
}
```

## What an impl adds

| Impl | Members' `implements` | Self type's `implements` | Members belong to (with Containment) |
|---|---|---|---|
| `impl fmt::Display for Local { fn fmt… }` | `fmt::Display` | `fmt::Display` added | `Local` |
| `impl Local { fn new… }` (inherent) | none | nothing added | `Local` |
| `impl !Send for Local {}` (negative) | none | nothing added | `Local` |
| `impl Other for &Local { fn by_ref… }` | `Other` | nothing added | a textual container `&Local`, no parent |
| `impl Other for other::Local { fn ext… }` | `Other` | nothing added | a textual container `other::Local`, no parent |
| `impl<T> Base for T { fn blanket… }` | `Base` | nothing added (`T` is the impl's parameter) | a textual container `T`, no parent |
| `fn outer<T>() { impl Base for T { fn probe… } }` | `Base` | nothing added (`T` is `outer`'s parameter) | a textual container `T`, no parent |
| `impl ::ext::B { fn probe… }` next to a local `mod ext { struct B; }` | none | nothing added (an external crate's type) | a textual container `::ext::B`, no parent |

- The self type's `implements` lists every positive trait impl in file
  order, with repeats of the same written trait removed.
- Each member keeps its own trait relation, so a query that returns methods
  alone still shows which trait supplied each method.
- With Containment, each member (and anything the container search places on
  the impl's type from inside it, such as a method's parameters) belongs to
  the resolved type. It is added to `relations.members` next to the type's
  ordinary members and the members of its other impls, in record order.
- When the self type does not resolve, the member's `relations.container` is
  the self type as written, with no `id`; it has no `parent`, and no local
  type lists it. A self type or trait written through
  [competing owners](./name-lookup.md#competing-owners-of-one-name)
  (`mod H { pub struct S; }`, `mod H {}`, then `impl T for H::S`) does not
  resolve: `S` gains neither the trait nor the members, the members get no
  container kind, and the trait, when it resolves on its own, still stays
  on the members; `impl H::T for S` keeps `S` as the owner and `H::T` as
  text.
- A member's `identity.container_kind` follows the same decision, with or
  without Containment: the resolved type's kind, and no kind when the self
  type does not resolve, even beside a same-named local type.
- Qualified names, `identity.container_name` and symbol IDs are not changed
  by any of this (`impl Other for &Local { fn by_ref }` is still
  `Local::by_ref`).

## Impls and the cache

The self type's resolution exists only while the file is parsed; no record
retains it. Widening a cached Rust entry by Containment therefore parses the
file once (`CacheLookup::WidenedCapabilities { parsed: true }`) instead of
guessing a container from records. Swift extension members do the same
(see [C#, Swift and Scala Base Lists](./mixed-base-lists.md)). Other
languages still add Containment from stored records. See
[How a Lookup Is Decided](../cache-design/lookup.md#widening-without-a-parse).

## Related pages

- [Declared Type Relations](../type-relations.md): the entry page.
- [Name Lookup for Base Types](./name-lookup.md): the rules the self type and
  the trait are resolved by.
