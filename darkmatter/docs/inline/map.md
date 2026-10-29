# Map Directive

**Planned:** `::map` and `::end-map` are not implemented yet. The examples and options below describe the intended behavior.

The planned map directive has this structure:

```md
<!-- start map block -->
::map <variable> "{expression}" <opt: filter>
<!-- mapped content block with local variable -->
- and don't forget: "{{i}}"
::end-map
```

- Like most other directives we have clear start (`::map`) and stop (`::end-map`) tokens which bound the mapping behavior.
- The central idea of this directive is to allow a Markdown author to iterate over [_iterables_](../iterables.md) and render them into the composed document with high precision.
- Just like all other block directives, the interior section can include any combination of other directives (e.g., conditional blocks, transclusion, etc.)

## Expressions

The simplest expression is no expression at all and the map directive allows for that. So this example is perfectly valid:

```md
---
data: ["foo","bar","baz"]
---
::map
- {{i}}
::end-map
```

This example renders an unordered list of:

```md
- foo
- bar
- baz
```

And it does that with the help of the variable `i`. What is the variable `i`? Well it's the default name for the _local variable_ that will represent the value of each item in the iterable.

## Iterables

Any tuple, array, or key/value object is considered iterable. In the case of the key/value object, the iterable value on each iteration will be placed into a:

```yaml
{ key: string(required), value: unknown(required) }
```

So that means in the following Markdown:

```md
---
data: { foo: "bar", bar: "baz"}
---
::map

```


## Variable Scoping

A map _variable_ must always be a reference to a Frontmatter property -- _and to ensure better type safety_ -- we also require that any variable used in a map block be defined in the schema.

That means in the following example Darkmatter would produce a `UntypedMapVariable` error:

```md
---
data:
    - foo
    - bar
    - baz
---
::map data fn="i => i"
- {i}!
::end-map
```

If you want to manually override this requirement you can use the `--allow untyped-map-variables` when calling compose from the CLI (or provide DarkmatterException::untyped_map_variables to the options of a library call).

Below is an example of how you could make the first example pass without needing to overriding this requirement:

```md
---
$schema:
    data: string[]
data:
    - foo
    - bar
    - baz
---
::map data fn="i -> i"
- {i}!
::end-map
```

> **Note:** all _functions_ (both map functions as well as filtering functions) should express themselves as an unknown/inline function but can use either the `->` or `=>` operator to separate the parameter passed in and the functional expression

## Map Function

You are required to provide a _mapping function_ who's responsibility is two fold:

- identifies the _local variable_ to be assigned during each iteration
- allows each value to be mutated into a different shape or type

The most basic function, and the one you'll likely use the most often, is something like `i -> i` which is an _identity_ function that doesn't change the incoming shape/type of the values coming in at all but establishes that `i` is the local variable that will be made available inside the map block.

However, as the next example illustrates, you can apply any of Darkmatter's expression syntax to modify the input items coming from the iterable list:

```md
TODO
```

## Sorting

TODO

## Map Filtering

We provide three primitives to use as _filtering_ constructs for the [iterable](../iterables.md) being iterated over:

- `when` - runs each list item through the logical expression and renders when `true`
- `until` - renders each list item through the logical expression _until_ one of the expressions returns a `true` value
- `while` - renders each list item through the logical expression _while_ the expressions have all evaluated to `true` but stopping once the first `false` evaluation is encountered

All three primitives, when used, must be assigned to a logical function like:

- `i => is_even(i)`
- `i => starts_with(i, '_')`
- etc.

> Note: in the examples we used `i` to be the parameter we're evaluating but you can use any valid variable name; `i` is just used as a popular convention.

Filtering is **not** a requirement for mapping but provides a mechanism to filter down the results to only those which you are interested in.

## Pipeline Sequencing

Because we allow nesting of both _inline_ and _transclusion_ operations inside the map block we need to be careful to design the composition pipeline's sequencing strategically so that the runtime behavior is both inline with author's expectations and provides as much "safe" functionality that we can provide to authors.
