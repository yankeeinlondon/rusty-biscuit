---
kind: concept
related: "darkmatter/docs/inline/map.md"
---

# Iterables

In Darkmatter we consider both arrays and key/value types as _iterable_ types. The planned [map directive](./inline/map.md) will iterate over either type. `::map` and `::end-map` are not implemented yet; the examples below describe the intended behavior:


## Iterating over an Array

```md
---
data: 
    - { name: foo } 
    - { name: bar }
    - { name: baz }
---
::map data fn="i -> i"
- {{i.name}}
::end-map
```

The example above takes the `data` array as input and converts it into an unordered markdown list by iterating and offsetting the elements with the `name` property. 

An alternative syntax that could have been used to provide the same results is:

```md
---
data: 
    - { name: foo } 
    - { name: bar }
    - { name: baz }
---
::map data fn="i -> i.name"
- {{i}}
::end-map
```

## Iterating over a Key/Value Object

```md
---
data:
    foo: { nickname: foey },
    bar: { nickname: barred },
    baz: { nickname: bazzy }
---
::map data fn="i => i"
- {{index(i)}} is often referred to as "{{i.nickname}}"
::end-map
```

This example leverages the `index(_iterable_)` expression to leverage both the key and value of each key/value pair in `data`.

**Note:** the `index(_iterable_)` expression is only available inside the [map](./inline/map.md) directive but it's available for both numeric arrays as well as key/value objects.
