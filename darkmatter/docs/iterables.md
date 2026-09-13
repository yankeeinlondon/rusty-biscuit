---
kind: concept
related: "darkmatter/docs/inline/looping.md"
---

# Iterables

In Darkmatter we consider both arrays and key/value types as _iterable_ types. An iterable type is able to be indexed over using the [looping](./inline/looping.md) directive that Darkmatter provides:


## Iterating over an Array

```md
---
data: 
    - { name: foo } 
    - { name: bar }
    - { name: baz }
---
::loop data where="i -> i"
- {{i.name}}
::end-loop
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
::loop data where="i -> i.name"
- {{i}}
::end-loop
```

## Iterating over a Key/Value Object

```md
---
data:
    foo: { nickname: foey },
    bar: { nickname: barred },
    baz: { nickname: bazzy }
---
::loop data where="i => i"
- {{index(i)}} is often referred to as "{{i.nickname}}"
::end-loop
```

This example leverages the `index(_iterable_)` expression to leverage both the key and value of each key/value pair in `data`.

**Note:** the `index(_iterable_)` expression is only available inside the [loop](./inline/looping.md) directive but it's available for both numeric arrays as well as key/value objects.
