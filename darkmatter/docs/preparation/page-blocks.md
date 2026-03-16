# Page Blocks

In Darkmatter you can specify start and end blocks in a page and then add:

- conditional expressions which determine if the block will be rendered when _composed_
- allow style and class attributes to be annotated (primarily for use when rendering to HTML)

## Example Blocks

```md
## My Section

::block when="state == 'foo'"

Foo is the best

::end-block
::block when="state == 'bar'"

Bar is the best

::end-block
```

## Parameters Attributes

1. `when`
2. `class`
3. `style`
4. `data-{xyz}`
