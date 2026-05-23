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

Page blocks can be nested to arbitrary depth. The parser pairs `::block` / `::end-block` with a stack, producing a region tree in which each parent keeps its children in source order. Siblings may freely sit next to nested children, and unrelated markdown can surround any block at any level.

```md
::block when="outer"
Outer content

::block when="inner"
Inner content
::end-block

::end-block
```

### Evaluation semantics

- **Top-down, lazy.** Children are only evaluated when their parent's `when` is true. If the outer region is skipped, the inner region's `when` expression is never parsed or evaluated — so references that only exist inside the outer scope cannot raise errors when the outer is false.
- **Literal text is preserved byte-for-byte.** Content between sibling blocks, and between the last child and its parent's `::end-block`, is emitted verbatim.
- **Shared state.** Every `when` at every depth evaluates against the same `EffectiveState`. Sibling blocks cannot influence each other's conditions; page-block evaluation runs before interpolation and shell expansion, so nested conditions cannot depend on values those stages produce.
- **Counters.** `ComposeReport.page_blocks_rendered` and `page_blocks_skipped` are incremented per evaluated block. A skipped parent contributes `+1` skipped and zero for its children (since they are never visited).

### Pairing rules and error reporting

- Pairing is strictly positional — there is **no label matching** between `::block` and `::end-block`. A stray `::end-block` inside a nested body closes the *innermost open* block, which can cause authoring mistakes to surface as `UnmatchedEnd` or `UnterminatedBlock` errors on lines far from the real mistake.
- An unterminated outer block is reported at the line of the **deepest unterminated** directive.
- `::block` / `::end-block` lines inside fenced code blocks are ignored at every depth, so nested examples in documentation do not interfere with real directives in the surrounding document.
- `::end-block` must stand alone on its line — any trailing non-whitespace content is a parse error.

## Darkmatter Expressions

The `when` expression uses Darkmatter's shared [Darkmatter Expressions](../topics/darkmatter-expressions.md) system. That same evaluator is also used by block transclusion.

Common examples:

```md
::block when="env.AGENT"
display when the AGENT environment variable is set
::end-block

::block when="env.AGENT == 'claude'"
display when the AGENT environment variable is equal to `claude`
::end-block

::block when="and(draft, user.role == 'admin')"
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
