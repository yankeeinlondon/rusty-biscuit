# Declaration Text, Headers and Keywords

Part of [Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

Every record keeps three pieces of the declaration's own source:

| Field | What it holds | Java `@Deprecated int old;` |
|---|---|---|
| `source.declaration_text` | everything up to the body: evidence | `@Deprecated int old;` |
| `source.signature_text` | the header a reader recognizes: display | `int old` |
| `identity.keyword` | the token that introduces the declaration | none (a field has no keyword) |

This page defines each of them, lists the keywords reported per language and
how `hug` uses them as labels, and explains how C and C++ names and
parameters are found behind declarators (pointers, arrays, function
pointers and attributes).

## Declaration text

Every record carries `source.declaration_text`: the declaration from its start
up to, but not including, its body, with trailing whitespace trimmed. A
declaration without a body contributes its whole text. It includes the
modifiers, access keywords and attributes that the grammar parses as part of
the declaration node, so syntax a level cannot express is never lost. Doc
comments are not included.

| Declaration | `declaration_text` |
|---|---|
| `pub(in crate::a) fn f() { … }` | `pub(in crate::a) fn f()` |
| `fn area(&self) -> u32;` (trait method, no body) | `fn area(&self) -> u32;` |
| `public private(set) var count = 0` | `public private(set) var count = 0` |
| `public int Count { get; private set; }` | `public int Count { get; private set; }` |
| `private[Shape] def secret() = 2` | `private[Shape] def secret() =` |
| `my ($a, $b) = @_;` | `my ($a, $b)`, for both `$a` and `$b` |

Syntax the grammar places outside the declaration node, such as a Rust
`#[…]` attribute, a Python decorator or a TypeScript `export` wrapper, is not
included. The text appears in records, not in `SymbolInfo`. It is evidence
rather than display, so the CLI's default `--json` report omits it and
`--json-full` carries it.

## Display header

`source.signature_text` is the declaration as a reader recognizes it: the
header without what follows it. `declaration_text` is evidence (everything up
to the body, initializers included); `signature_text` is display, and is
always a contiguous part of `declaration_text`, read from the same node in
the same pass.

Starting from `declaration_text`, the header drops:

- **leading annotations**: annotations, attributes and decorators written
  before the first modifier or keyword (Java `@Override`, C# `[Obsolete]`,
  Swift `@available`, Scala `@deprecated`, JavaScript `@decorator`, PHP
  `#[…]`, C/C++ `[[…]]`)
- **initializers**: everything from the `=` (Go: `:=`) that binds a value,
  including a type alias's aliased type and a C macro's replacement text
- **bodies and accessors**: a C# `{ get; set; }` list, a Swift `{ get }`
  requirement or computed body, a Go struct field list or interface method
  list
- **introducers and terminators**: the `=`, `=>` or `:` that introduces a
  body, and a `;` (Lua: `end`) that ends a bodyless declaration

Outer whitespace is stripped; line breaks and indentation inside a
multi-line header are kept.

| Declaration | `signature_text` |
|---|---|
| `@Override public void run() { … }` | `public void run()` |
| `public @Deprecated static void m() { … }` | `public @Deprecated static void m()` (an annotation between modifiers stays) |
| `pub const TABLE: [u32; 4] = [ … ];` | `pub const TABLE: [u32; 4]` |
| `fn area(&self) -> u32;` (trait method) | `fn area(&self) -> u32` |
| `def total(self) -> int:` | `def total(self) -> int` |
| `const twice = (value: number): number => value * 2;` | `(value: number): number` (the arrow function is the declaration) |
| `type Point struct { X int }` | `type Point struct` |
| `local function helper() end` (Lua, empty body) | `local function helper()` |
| `type ( A struct{ … }; B int )` (Go group, for `B`) | `B int` (each spec is its own declaration) |
| `public init(x: Int, y: Int = 2) {}` (Swift) | `public init(x: Int, y: Int = 2)` (a parameter default is not an initializer) |
| `X, y int` (Go struct fields, for both) | `X, y int` |
| `enum E { RED, BLUE = 2 };` (C, for `BLUE`) | `BLUE` |
| `case Value(x: Int)` (Scala) | `Value(x: Int)`; `case`, which one group `case A, B` shares, is the keyword but not in the header |

The header is omitted when no contiguous part names the symbol by itself:

| Declaration | Why `signature_text` is omitted |
|---|---|
| `typedef struct { int x; } Point;` | the alias name follows the struct body |
| `int a = 1, b = 2;` (for `b`) | `a`'s initializer ends the header before `b` |
| `first = second = 0` (Python, for `second`) | `first`'s `=` ends the header before `second` |

## Declaring keywords

`identity.keyword` is the grammar token that introduces a declaration. It is
read from the syntax tree, so a comment naming a keyword never supplies one,
and it is a single token: Scala `case class` reports `class`, C++ `enum
class` reports `enum`, Swift `class func` reports `func`, Lua `local function`
reports `function`. It is omitted when the construct has no keyword. A
construct the extractor does not report as a symbol (see
[Current limitations](./limitations.md#current-limitations)) has no record to carry one.

| Language | Keywords reported | No keyword |
|---|---|---|
| Rust | `fn`, `struct`, `union`, `enum`, `trait`, `type`, `mod`, `const`, `static`, `let`, `macro_rules!` | fields, enum variants, parameters |
| Go | `func`, `type` (also on a type alias), `const`, `var`, `package` | `:=` locals, parameters, struct fields |
| Swift | `class`, `struct`, `enum`, `actor`, `protocol`, `func`, `init`, `typealias`, `case`, `var`, `let` | parameters |
| Scala | `def`, `val`, `var`, `class`, `object`, `trait`, `enum`, `type`, `case` (enum cases); `val`/`var` on a class parameter | plain class and method parameters |
| TypeScript | `function`, `class`, `interface`, `enum`, `type`, `namespace`, `module`, `const`, `let`, `var` | methods, fields, constructors, enum members, arrow functions, parameters |
| JavaScript | `function`, `class`, `const`, `let`, `var` | methods, fields, constructors, arrow functions, parameters |
| Java | `class`, `interface`, `enum`, `record`, `package` | methods, constructors, fields, enum constants, locals, parameters |
| C# | `class`, `struct`, `interface`, `enum`, `record`, `namespace`, `event` | methods, constructors, properties, fields (`const` is a modifier), enum members, locals, parameters |
| C | `struct`, `union`, `enum`, `typedef`, `#define` | functions, variables, fields, enumerators, parameters |
| C++ | C's, plus `class`, `namespace`, `using`, and `class`/`typename` template parameters | as C, plus methods and constructors |
| PHP | `function` (also on a constructor), `class`, `interface`, `trait`, `enum`, `case`, `namespace`, `const`, legacy `var` properties | typed properties, parameters |
| Python | `def` (also on `__init__`), `class` | assignments, parameters |
| Perl | `sub`, `package`, `my`, `our`, `state` | — |
| Lua | `function`, `local` | global assignments, parameters |
| Bash, Zsh | `function`; `local`, `declare`, `typeset`, `export`, `readonly` (and Zsh `integer`, `float`) on a variable | `name()` functions, plain assignments |

For a Go `const`/`var` spec and a Lua or shell `local`, the keyword belongs
to the statement that wraps the declaration, so the record reports it even
though its `declaration_text` and `signature_text` do not contain it:

```go
const Limit = 3 // keyword "const", signature_text "Limit"
```

### Labels in `hug` output

`hug` prints the keyword as the label before a name only for Type,
Variable, Field and Constant records, whose kind name alone would hide the
construct (`struct Point`, `let total`, `my $queue`, `local M`). Every other
kind, and a record without a keyword, is labeled with its kind (`function`,
`method`, `enum variant`), so a Rust function prints `function parse`, not
`fn parse`. A container suffix always names the container's kind:
`[in type Point]`.

## C and C++ names behind declarators

A C or C++ name is reported wherever its declarator puts it: behind an
attribute written after the name, inside extra parentheses, or under the
pointer, reference, array and callback layers of a return type. What the
declarator declares decides the kind, so a function-pointer field stays a
Field while a member function returning a callback is a Method:

```cpp
int count [[maybe_unused]];              // Variable
int legacy [[deprecated]] (int value);   // Function, parameter value
int (wrapped)(int value);                // Function
struct Owner {
    int values [[maybe_unused]] [2];     // Field, listed by `hug classes`
    int (*callback)(int);                // Field: a pointer, not a function
    int (*handler(int value))(long);     // Method, parameter value
    int **lookup(int key) { return 0; }  // Method
    int size();                          // Method
};
int Owner::size [[nodiscard]] () { return 0; }  // Function (out of class)
typedef int (*Callback)(int);            // TypeAlias
```

These names get the same access, `static` flags and signature as the same
declaration without the wrapper, and an attributed field is in its type's
field list. An attribute before the declaration (`[[maybe_unused]] int x;`)
was never in the way. A typedef name behind a declarator (`typedef struct
node *NodePtr;`) carries the typedef's doc comment but no field list: the
fields belong to the struct, not to the pointer.

## C and C++ signatures

A C or C++ function's signature lists the parameters of that function, read
from the declarator nearest its name with only parentheses between. Pointers,
references, arrays and function types wrapped around it belong to the return
type, so their parameters are never the function's:

```c
int *pointer_return(int value);                  // value
int (*callback_return(int actual))(long other);  // actual, not other
int (*array_return(int actual))[2];              // actual
```

The same rule applies to prototypes, definitions, `static`, `#if`,
`extern "C"`, templates, in-class members and out-of-class `A::f`
definitions. Each parameter is named by its own declarator, under any
pointer, reference, array, parenthesized, attributed or function-pointer
layer; a callback parameter is named by its own name, never one from its
parameter list:

| Parameter | Reported |
|---|---|
| `char **argv`, `int values[2]`, `int (*value)`, `int value [[maybe_unused]]` | the name |
| `int (*callback)(long ignored)` | `callback` |
| `int defaulted = 1` (C++) | `defaulted`, default value `1` |
| `Ts... values`, `Ts&&... values` (C++ pack) | `values`, variadic |
| `...` (C and C++) | `...`, variadic |
| `int`, `int (*)(long)` (unnamed) | not listed |

`int (value)` is an unnamed parameter of function type, not a parenthesized
name: the grammar applies C's rule that a parenthesized identifier in a
parameter is read as a type name.
