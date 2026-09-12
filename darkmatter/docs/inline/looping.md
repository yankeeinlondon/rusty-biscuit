# Looping Directive

Darkmatter provides a looping directive that looks structurally like:

```md
<!-- start looping block -->
::loop <variable> fn="{expression}" <opt: filter>'
<!-- looped content block with local variable -->
- and don't forget: "{{i}}"
::end-loop

- Like most other directives we have clear start (`::loop`) and stop (`::end-loop`) tokens which bound the looping behavior.
- The central idea of this directive is to allow a Markdown author to iterate over [_iterables_](../iterables.md) and render them into the composed document with high precision.
- This idea extends to allow for this looped content to include both other _inline_ directives as well as _transclusion_ operations as well.

## Loop Variables

A loop _variable_ must always be a reference to a Frontmatter property -- _and to ensure better type safety_ -- we also require that any variable used in a looping block be defined in the schema.

That means in the following example Darkmatter would produce a `UntypedLoopVariable` error:

```md
---
data:
    - foo
    - bar
    - baz
---
::loop data fn="i => i"
- {i}!
::end-loop
```

If you want to manually override this requirement you can use the `--allow untyped-loop-variables` when calling compose from the CLI (or provide DarkmatterException::untyped_loop_variables to the options of a library call).

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
::loop data
- {i}!
::end-loop
```

## Loop Filtering

We provide three primitives to use as _filtering_ constructs for the [iterable](../iterables.md) being iterated over:

- `when`
- `until`
- `while`

All three primitives, when used, must be assigned to a logical function like:

- `i => is_even(i)`
- `i => starts_with(i, '_')`
- etc.

> Note: in the examples we used `i` to be the parameter we're evaluating but you can use any valid variable name; `i` is just used as a popular convention.

Filtering is **not** a requirement for looping but provides a mechanism to filter down the results to only those which you are interested in.

## Pipeline Sequencing

Because we allow nesting of both _inline_ and _transclusion_ operations inside the looping block we need to be careful to design the composition pipeline's sequencing strategically so that the runtime behavior is both inline with author's expectations and provides as much "safe" functionality that we can provide to authors.
