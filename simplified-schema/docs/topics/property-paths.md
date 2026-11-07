# Property Paths

`PropertyPath` names a value inside a document's data, such as `style.page.max-width`. Use it when your host takes a path from a person (a command line, a config value) and needs to read or change that value. It owns the one grammar, so a host never splits on `.` itself.

```rust
use serde_json::json;
use simplified_schema::PropertyPath;

let root = json!({ "items": [{ "name": "a" }, { "name": "b" }] });
let path = PropertyPath::parse("items.1.name")?;
assert_eq!(path.resolve(&root).value(), Some(&json!("b")));
```

For how the `md` commands use it, and what each does with a path that cannot be followed, see [Property Paths in Darkmatter](../../../darkmatter/docs/cli/index.md#property-paths).

## Parsing

`PropertyPath::parse` takes text after the shell has removed its quoting, and looks at no data.

| Text | Segments | Why |
|---|---|---|
| `style.page` | `style`, `page` | `.` separates |
| `a\.b` | `a.b` | `\.` is a literal dot |
| `C:\\.name` | `C:\`, `name` | `\\` is a literal backslash |
| `C:\dir` | `C:\dir` | a `\` before anything else is itself |
| (empty) | one empty segment | the whole empty argument names an empty key |
| `a.`, `.a`, `a..b` | `PropertyPathError::EmptySegment` | an empty segment in a longer path is a mistake |

Nothing is trimmed or normalized. Escape pairs match from left to right.

- `as_written()` returns the argument as given, for result fields and messages.
- `segments()` returns the decoded segments.
- `Display` prints the canonical spelling, and parsing it gives the same segments back. `from_segments` goes the other way.
- Equality and hashing compare decoded segments, so `C:\dir` and `C:\\dir` are the same path, while `a\.b` and `a.b` differ.

## Walking a value

A segment's meaning depends on the value it meets, so the text alone cannot say whether `0` is an index. `resolve` (for a `serde_json::Value`) and `resolve_map` (for a top-level `IndexMap`) make that decision and return a `Resolved`.

- On an array, a segment is an index only in canonical form: `0`, or digits with no leading zero. `01`, signs, and `-` are not indexes, and a number too large to hold is out of range rather than a wrapped index or a panic.
- On a mapping, every segment is a key, so `0` and `00` differ.
- A present `null` or scalar cannot be walked through.

| `Resolved` method | Gives |
|---|---|
| `value()` | the value found, if the walk finished |
| `stop()` | `ResolveStop::Found`, `MissingKey`, `OutOfRange`, or `NotContainer`: why the walk stopped |
| `steps()` | the typed `PathStep`s walked so far, for a schema query such as `completion::def_at_path` |
| `unresolved()` | the segments the walk did not reach |
| `pointer()` | the walked part as a `JsonPointer`, as diagnostics report it |

Convert through `Resolved` only. `PathStep` and `JsonPointer` need to know indexes from keys, and only a walk over the data knows that.

## Editing a value

`set_at` and `remove_at` change a frontmatter-shaped map (`IndexMap<String, Value>`). Each decides success before changing anything, so a failed edit leaves its input untouched.

```rust
use indexmap::IndexMap;
use serde_json::json;
use simplified_schema::{PropertyPath, set_at};

let mut root = IndexMap::new();
set_at(&mut root, &PropertyPath::parse("author.name")?, json!("Ken"))?;
assert_eq!(root["author"], json!({ "name": "Ken" }));
```

| `set_at` meets | Result |
|---|---|
| an absent key or parents | created; new keys append, and a replaced key keeps its position |
| a number under an absent parent | `PathEditError::NumberUnderMissingParent`: nothing says whether it is an index or a key |
| an index that does not exist | `PathEditError::IndexOutOfRange`, with the length |
| text, a number, or `null` as a parent | `PathEditError::ParentNotContainer`, with its type |
| the `null` or scalar named by the path itself | replaced |

For removal, `RemovalPlan::new` checks every path against the original value before anything changes, and `apply` removes them. `remove_at` does both.

- A missing target is `PathEditError::Missing`. An index that does not exist and a scalar parent report as above.
- Two paths naming one target, or an ancestor with its descendant, are `PathEditError::RemovalConflict`.
- Array indexes refer to the original array, so `items.0 items.2` on `[a, b, c, d]` leaves `[b, d]` in either order.
- Emptied parents stay, as `{}` or `[]`.
