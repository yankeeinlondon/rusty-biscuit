# Subtyping

Ask whether one resolved type accepts only values another type also accepts.

## Subtype Relation (`is_subtype`)

`is_subtype(a, b)` answers "does every non-null value accepted by type `a`
also satisfy type `b`?". It is passive and works on resolved
`PropertyAtom`s, so resolve named types first. Use it to decide whether a
saved rule typed `b` covers a value typed `a`, for example whether an
approval for `alphanumeric(allow-slashes)` covers an argument typed
`alphanumeric`.

```rust
use simplified_schema::is_subtype;
use simplified_schema::simplified::grammar::parse_type_expr;

let atom = |text| parse_type_expr("p", text).unwrap();
let rule = atom("alphanumeric(allow-slashes)");
assert!(is_subtype(&atom("alphanumeric(min(3))"), &rule)); // constraints are ignored
assert!(!is_subtype(&atom("string"), &rule));
assert!(is_subtype(&atom("email"), &atom("string")));
assert!(is_subtype(&atom("literal(4)"), &atom("number")));
assert!(is_subtype(&atom("literal('a')"), &atom("enum(a, b, c)")));
assert!(!is_subtype(&atom("literal(4)"), &atom("enum('4')"))); // structure, not `==`
```

Only the type and its canonical options count. Ordinary constraints (`min`,
`max`, `pattern`, …), presence, the optional `null` wrapper, and coercion are
outside the relation. Those exclusions apply inside structured types too:
changing an inline object's child `min(...)` constraint does not change its
type identity. Field names and child types, tuple positions and their optional
markers, and other structural details still count. This does not make an
object with a different field shape a subtype merely because some fields
agree.

| `a` | `b` | `a` ⊆ `b`? |
|---|---|---|
| any type | the same type with the same canonical options (including an identical array, inline object, or union) | yes |
| `alphanumeric(X)` | `alphanumeric(Y)` | yes when X's allowed characters and allowed first characters are subsets of Y's, and X guarantees any ban on `..` that Y makes |
| an always-text type: `string`, `date`, `datetime`, `time`, `file`, `enum(…)`, `url`, `email`, `yaml`, `json`, `expression`, `alphanumeric(…)`, `shell`, or a string `literal(…)` | `string` | yes |
| a `literal(…)` | its own scalar family: `string`, `number`, or `boolean` | yes |
| a string `literal(v)` | an `enum(…)` that has `v` as a member | yes |
| a number or boolean `literal(…)` | any `enum(…)` | no |
| `enum(X)` | `enum(Y)` | yes when X's members are a subset of Y's |
| a union (not an array of one) | any type | yes when every arm is a subtype of it |
| any type | a union (not an array of one) | yes when one arm covers it, whatever the arm order |
| `string`, `email`, or any other non-`alphanumeric` type | `alphanumeric(…)` | no |
| `unknown`, `null`, `number`, `boolean`, an array, … | `string` | no |
| any other pair, including an unresolved reference | | no |

So `string | number` is a subtype of `boolean | number | string`, and
`unknown` is never a subtype of a concrete type. Arrays compare only by
identity: `alphanumeric[]` is not a subtype of `string[]`.

A saved shell rule typed `enum(a, b, c)` therefore covers a placeholder
declared `enum(a, b)` or `literal('a')`. The rule grammar is unchanged:
rule placeholders are still string-like types, and every other placeholder
still needs a broad `{{ string }}` rule.

The `alphanumeric` row compares what the options permit, not the keywords
written. X guarantees no `..` when it blocks double periods or does not allow
periods at all, so base `alphanumeric` ⊆
`alphanumeric(allow-periods, block-double-periods)`, and
`alphanumeric(leading-alpha, allow-dashes)` ⊆
`alphanumeric(allow-dashes, block-leading-dash)`. Redundant options cancel
out: `alphanumeric(block-leading-dash)` and `alphanumeric` are subtypes of
each other. This is bounded reasoning over the option table, not inclusion
checking for user regexes, and it never replaces a runtime check of the
actual value or shell quoting.
