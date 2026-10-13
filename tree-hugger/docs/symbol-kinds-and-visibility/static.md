# `static`

Part of [Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

`hug classes` groups a type's members by whether they need an instance:

```python
from typing import ClassVar

class Point:
    origin: ClassVar[int] = 0
    x: int = 0

    def __init__(self, x):
        self.x = x

    @staticmethod
    def unit():
        return Point(1)
```

```text
$ hug classes shapes.py
shapes.py (Python)
  class Point [3:1]
    Static Methods (1)
      - unit() [in class Point] [12:5]
    Instance Methods (1)
      - __init__(self, x) [in class Point] [8:5]
    Static Fields (1)
      - origin: ClassVar[int]
    Instance Fields (1)
      - x: int
```

That grouping reads the record's member `static` flag. This page lists which
syntax sets it in each language, and how it differs from storage `static`.

`static` has two unrelated meanings, and Tree Hugger keeps them apart:

| | Member `static` | Storage `static` |
|---|---|---|
| Meaning | the member is used without an instance | storage duration or file linkage |
| v1 | `SymbolInfo::is_static` (copied to `FunctionSignature::is_static` and `FieldInfo::is_static`) | `SymbolInfo::is_static_storage` |
| v2 | `Modifier::Static` | `Modifier::StaticStorage` |
| `hug classes` | listed under *Static Methods* / *Static Fields* | never listed as a static member |

Member `static` is set for:

- TypeScript, JavaScript, Java, C# and PHP `static`
- members the language makes static without the keyword: Java interface
  constants (implicitly `static` whether or not they say so), C# `const`
  fields (in a class, struct, record or interface), Rust associated `const`s
  (in an inherent `impl` or a trait) and PHP `const` constants (in a class,
  interface, trait or enum; a top-level `const` is not a member)
- C++ `static` data members and member functions
- Swift `static` and `class` members
- Scala members whose nearest owning type is an `object` (a standalone or
  companion object, also one nested in a class)
- Python methods declared in a class body with the decorator `@staticmethod`
  or `@classmethod`, also spelled `@builtins.staticmethod` /
  `@builtins.classmethod`, and class attributes (below)
- Rust associated functions without a `self` receiver, in impls and traits; a
  typed receiver such as `self: Box<Self>` is a receiver

No other supported language makes a member static without a keyword: a C++
`static constexpr` member says `static`, and Swift, TypeScript, JavaScript and
Scala `let`/`readonly`/`val` members are instance members unless declared
`static` (or, in Scala, in an `object`).

Constructors and enum variants are never static. A constructor has no
receiver parameter in most languages, and an enum constant behaves like a
static value, but neither says `static`; a Scala enum `case` is owned by its
enum, not by an object.

```csharp
class Limits {
    public const int Max = 1;    // Static Fields
    int count;                   // Instance Fields
}
```

```rust
trait Shape { const SIDES: u32; }   // SIDES: Static Fields of Shape
```

Python class methods and Swift `class` methods are grouped with static members
because they need no instance, although they are not identical to static
methods.

Only the declaration's own keyword, modifier or decorator counts, never a name
or text that contains it, and only the type that directly owns the member
decides:

```python
class Sweep:
    @notstaticmethod        # some other decorator: instance method
    def probe(self): ...
    @wrap(staticmethod)     # a call to `wrap`: instance method
    def wrapped(self): ...
    @staticmethod           # Static Methods
    def make(): ...

@staticmethod               # not in a class: never a static member
def helper(): ...
```

```scala
object Registry {
  def lookup(): Unit = ()                  // static: owned by the object
  trait Entry { def key(): String }        // instance: owned by the trait
  class Slot { def fill(): Unit = () }     // instance: owned by the class
  val anon = new Runnable { def run(): Unit = () } // `run` is an instance member
}
```

A member of a Scala `class`, `trait`, `case class`, `enum`, anonymous class
(`new T { … }`) or `given` instance is never static, even when that type is
declared inside an object. An `extension` method written in an object belongs
to the object and is static; one written in a class is an instance method. A
`package object` is not a container, so its members are top-level Functions
and never static.

Storage `static` is set for Rust `static` and `static mut` items, C and C++
file-scope or namespace-scope `static` functions and variables, and C and C++
function-local `static` variables:

```cpp
class Counter {
    static int total;            // member static: Static Fields
    int bump() {
        static int calls = 0;    // storage static: not a member
        return ++calls;
    }
};
static int helper() { return 0; } // storage static (and ModulePrivate)
```

## Python class attributes

A name declared in a class body is a Field of that class; its annotation
decides whether it is static. The annotation's expression is read, not its
text: `ClassVar`, bare or subscripted, under any qualifier (`typing.ClassVar`,
`t.ClassVar[int]`), makes a class attribute; `ClassVarLike[int]`,
`MyClassVar` and a string annotation such as `"ClassVar[int]"` do not:

```python
from typing import ClassVar

class Point:
    x: int                       # instance Field
    y: int = 0                   # instance Field
    origin: ClassVar[int] = 0    # static Field (class attribute)
    count = 0                    # static Field (class attribute)

    def move(self):
        self.z = 1               # not reported
```

An unannotated `count = 0` is a class attribute that an instance may shadow by
assigning `self.count`; the record describes the class-level declaration.
This follows PEP 526 and dataclass semantics. `hug symbols` and `hug classes`
apply the same rule.

Every name an unannotated class-body assignment binds is a static Field: each
target of a chain and each name directly in a tuple or list pattern, with or
without parentheses. A chained assignment outside a class declares Variables.

```python
class Pair:
    first = second = 0           # both static Fields
    a, b = (1, 2)                # both static Fields
    [c, d] = 1, 2                # both static Fields
```
