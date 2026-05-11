# Page Blocks

In Darkmatter you can specify start and end blocks in a page and then add:

- conditional expressions which determine if the block will be rendered when _composed_
- NOTE: we may add other features for these blocks in the future but for now they're mainly for conditional rendering.

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

    The primary use case for using page blocks is 

2. FUTURE: more later


## Conditional Logic

The conditional logic in the `when` variable is the same conditional logic provided by the [block transclusion](@darkmatter/docs/transclusion/block-transclusion.md#conditional-transclusion)'s `when` clauses. 

### ENV variables

In the prior example we showed conditional comparisons we made to frontmatter properties but we can also compare against environment variables:

```md

::block when="env.AGENT"
display when the AGENT environment variable is set
::end-block

::block when="env.AGENT == 'claude'"
display when the AGENT environment variable is equal to 'claude'
::end-block

```


