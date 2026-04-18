# Page Blocks

Page blocks let you keep conditional regions directly inside a Markdown document. During composition, Darkmatter evaluates each block and either keeps its body content or removes it entirely.

The syntax is:

```md
::block when="..."
content
::end-block
```

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

## Options

### `when`

The `when` option controls whether the block body is kept.

```md
::block when="draft"
This renders only when `draft` is truthy.
::end-block
```

If `when` is omitted, the block is treated as enabled and its body is rendered.

## Nesting

Page blocks can be nested. Inner blocks are only evaluated if their parent block rendered.

```md
::block when="outer"
Outer content

::block when="inner"
Inner content
::end-block

::end-block
```

## Boolean Conditional Logic

The `when` expression uses Darkmatter's shared [Boolean Conditional Logic](../topics/boolean-conditional-logic.md) system. That same evaluator is also used by block transclusion.

Common examples:

```md
::block when="env.AGENT"
display when the AGENT environment variable is set
::end-block

::block when="env.AGENT == 'claude'"
display when the AGENT environment variable is equal to `claude`
::end-block

::block when="And(draft, user.role == 'admin')"
display only when both conditions are true
::end-block
```

### ENV variables

Environment variables are available under `env.*`:

```md
::block when="env.AGENT"
display when the AGENT environment variable is set
::end-block

::block when="env.AGENT == 'claude'"
display when the AGENT environment variable is equal to 'claude'
::end-block
```

## Notes

- Page blocks are ignored inside fenced code blocks.
- Unknown page-block options are reported as warnings during composition.
- Page blocks run before later inline stages such as body interpolation, so a false block is removed before its inner content can be interpolated or otherwise processed.
