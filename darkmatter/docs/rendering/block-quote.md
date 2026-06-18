# Block Quote Rendering

## Feature

The `::block-quote` ... `::end-block-quote` directives in Darkmatter block content which you want to have encapsulated as a block quote.

- this allows you to more easily bring in content, no longer needing to precede each line with `> `, and ensure that it will all be included as one block quote
- it also allows you to provide custom styling to the given block quote
- and finally, it also provides conditional logic too


## Syntax

The syntax for this command is straightforward:

```md
::block-quote {{params}}
(content to be block quoted)
::end-block-quote
```

## Parameters

On the opening `::block-quote` directive we can add any of the following parameters (though none are required):

- `width`
- `max-width`
- `min-width`
- `left-margin`, `right-margin`
- `color`
- 

Parameters are added using the `{param}={value}` convention.

## Output Targets

- `Markdown`
    - the output is left unchanged
- `MarkdownPlus`:
    - we will convert to native Markdown block quote syntax
    - TODO: figure out how to maintain the 'style/layout'
- `Terminal` and `Browser`
    - will be rendered using the standard tree-based renderer
