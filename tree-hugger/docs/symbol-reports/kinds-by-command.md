# Kinds by Command

Part of [Symbol Reports](../symbol-reports.md).

Which command lists a declaration depends on its kind: each command asks for
a fixed set of kinds, summarized in
[the four commands](../symbol-reports.md#the-four-commands-and-what-they-analyze).
This page walks through the kinds that are easy to misplace (constructors,
type aliases, enum variants, fields and namespaces) over a small
mixed-language directory:

```java
// Shapes.java
class Counter {
    Counter(int start) {}
    void tick() {}
}

enum Color { RED, BLUE }
```

```go
// shapes.go
package shapes

type Alias = int

type Point struct {
	X, Y int
	label string
}
```

```scala
// model.scala
class Box {
  val width: Int = 1
  var height = 2
  def area(): Int = width
}

object Registry {
  val count = 0
  type Key = String
}
```

`--kind constructor` keeps the constructors and `--kind method` drops them:

```text
$ hug functions --kind constructor
  - package constructor Counter(start: int) [in class Counter] Shapes.java:2:5
$ hug functions Shapes.java --kind method
  - package method tick() -> void [in class Counter] Shapes.java:3:5
$ hug types --kind type_alias
  - public type alias Key model.scala:9:3
  - public type alias Alias shapes.go:3:1
```

`Key` is a member of `Registry` (JSON `identity.container_name`), but the
terminal prints the `[in …]` suffix only for member kinds such as methods,
fields and variants, never for a type or alias.

`hug symbols` lists an enum with its variant list and then each variant as
its own record, and a struct with its field list and then each field:

```text
$ hug symbols Shapes.java shapes.go
  - package class Counter Shapes.java:1:1
  - package constructor Counter(start: int) [in class Counter] Shapes.java:2:5
  - package method tick() -> void [in class Counter] Shapes.java:3:5
  - package enum Color { RED, BLUE } Shapes.java:6:1
  - public enum variant RED [in enum Color] Shapes.java:6:14
  - public enum variant BLUE [in enum Color] Shapes.java:6:19
  - namespace shapes shapes.go:1:9
  - public type alias Alias shapes.go:3:1
  - public type Point { X int, Y int, label string } shapes.go:5:1
  - public field Y [in type Point] shapes.go:6:2
  - public field X [in type Point] shapes.go:6:2
  - package field label [in type Point] shapes.go:7:2
```

The list after a type's name and its standalone records always name the same
members, once each: a list is completed from the records, and a record that
is also in a reader's list is paired with that entry rather than added
again. A field list entry keeps the type written in the source; a variant
list entry keeps the value (`BLUE = 2` would list `BLUE` with `2` as its
`discriminant` in JSON). A standalone variant record has no payload of its
own (`kind_data` is `Unknown`), so JSON consumers read values from the
enum's list. Variants never appear in a field list. `X` and `Y` above share
one declaration, so they are ordered by the [tie-breakers](../symbol-reports.md#ordering), not
by their written order.

`hug classes` puts constructors with the methods, and its field sections
list the same fields as `hug symbols`:

```text
$ hug classes
Shapes.java (Java)
  class Counter [1:1]
    Instance Methods (2)
      - package Counter(start: int) [in class Counter] [2:5]
      - package tick() -> void [in class Counter] [3:5]

model.scala (Scala)
  class Box [1:1]
    Instance Methods (1)
      - public area() -> Int [in class Box] [4:3]
    Instance Fields (2)
      - public width: Int
      - public height

shapes.go (Go)
  type Point [5:1]
    Instance Fields (3)
      - public X int
      - public Y int
      - package label string
```

The field sections follow source order (`X` before `Y`). `Registry` is an
`object`, a Module, so `classes` does not list it; its `count` is a static
Field in `hug symbols`. Counts, `--stats`, `--json` and `--jsonl` read the
same records as these listings, so `hug functions --kind constructor
--count` prints `1` here.
