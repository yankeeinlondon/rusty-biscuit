---
kind: concept
related: "darkmatter/docs/inline/map.md"
---

# Iterables

In Darkmatter, arrays, tuples, and key/value objects are _iterable_ types. The [map](./inline/map.md) and [filter](./inline/filter.md) directives iterate over these types. A source must have a known iterable type from its effective schema, namespace declaration, or enclosing local binding.

Optional and nullable collections are accepted: missing or null sources render
nothing, as do empty collections. Non-null scalar sources are errors. Arrays
and tuples preserve element order, and objects preserve insertion order unless
the map explicitly sorts them.

Map bindings live in the `local` namespace. A bare `i` prefers the nearest local
binding; `local.i` accesses that binding explicitly, while `fm.i` accesses the
frontmatter property even when a local shadows it.

## Iterating over an Array

```md
---
$schema:
    data: "{ name: string(required) }[](required)"
data: 
    - { name: foo } 
    - { name: bar }
    - { name: baz }
---
::map data "i -> i"
- {{i.name}}
::end-map
```

The example above takes the `data` array as input and converts it into an unordered markdown list by iterating and offsetting the elements with the `name` property. 

An alternative syntax that could have been used to provide the same results is:

```md
---
$schema:
    data: "{ name: string(required) }[](required)"
data: 
    - { name: foo } 
    - { name: bar }
    - { name: baz }
---
::map data "i -> i.name"
- {{i}}
::end-map
```

## Iterating over a Key/Value Object

```md
---
$schema:
    data: "{ foo: { nickname: string(required) }(required), bar: { nickname: string(required) }(required), baz: { nickname: string(required) }(required) }(required)"
data:
    foo: { nickname: foey }
    bar: { nickname: barred }
    baz: { nickname: bazzy }
---
::map data
- {{i.key}} is often referred to as "{{i.value.nickname}}"
::end-map
```

Objects yield entries with the general shape `{key: string, value: any}`. The
declared value type is preserved, so `i.value.nickname` remains typed as a
string in this example. Keys are explicit entry fields, not recovered through
an `index(i)` function.

Shallow destructuring can expose those fields directly:

```md
::map data "{key,value} => {key,value}"
- {{key}} is often referred to as "{{value.nickname}}"
::end-map
```

The quoted mapping function is optional and positional. Both `->` and `=>`
have the same meaning. Its parameter pattern binds the input for evaluating
the expression, then binds the result for rendering the body; see the map
directive for transformation and type-checking rules.
