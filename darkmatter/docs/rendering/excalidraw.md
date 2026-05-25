# Excalidraw Rendering

Darkmatter is able to render [Excalidraw](../research/excalidraw.md) drawing in two complimentary ways:

1. Code Block
2. `::excalidraw <file>` Directive

## Code Blocks

While **Excalidraw** drawings use a `.excalidraw` file extension they are just JSON documents. Though it a may be a less common format for people to leverage, you can embed the JSON inside a code block like so:

```excalidraw
{
    // config goes here
}
```

## Excalidraw Directive



## Configuration Options

- `width` the width can be expressed in `ch` (e.g., `40ch`) or a numeric percentage (e.g., `50%`); if a bare number is used it is assumed to be a `ch` width.
    - the natural aspect ratio will always be used to preserve the fidelity of the drawing
- `max-width`
- `alignment`

The same configuration elements are available in both code block and directive variants. In both cases the
