# Biscuit Terminal

<table>
  <tr>
    <td><img src="../assets/biscuit-terminal-512.png" style="max-width='25%'" width=200px /></td>
    <td>
      <h2>biscuit-terminal</h2>
      <p>This shared library provides support for working with the terminal:</p>
      <ul>
        <li>terminal <b>metadata</b> (<i>13+ terminal emulators, color depth, light/dark mode, dimensions, font, OS</i>)</li>
        <li>OSC8 links, OSC52 clipboard, and OSC10/11/12 color queries</li>
        <li>multiplex detection for tmux/Zellij plus native WezTerm/Ghostty/Kitty support</li>
        <li>inline image rendering via Kitty/iTerm2 protocols with graceful fallback</li>
        <li>terminal-facing Mermaid rendering via <code>MermaidDiagram</code>, backed by <code>biscuit-visualized</code> (pure Rust)</li>
        <li>terminal-facing graph rendering via <code>GraphExpression</code>, with arrow, dash, and DOT syntax support</li>
        <li>color system: BasicColor (16 ANSI), RgbColor, WebColor (148 CSS), Tailwind (22 families × 11 shades)</li>
        <li>composable rendering components: Prose, Table, List, Section, FileSystem, TwoColumn, and more</li>
      </ul>
    </td>
  </tr>
</table>

## About

The `biscuit-terminal` package area contains both a Library and CLI which focus on two distinct but complementary things:

1. Feature Discovery

    The `biscuit-terminal` library is able to provide rich feature discovery on terminals to identify things like:

    - Color Depth
    - Terminal Size (cols and rows)
    - OSC8 Link Support
    - OSC10 and OSC11 support for terminal querying
    - Image rendering supporting
    - Background color and Color Mode (light/dark)
    - Support for Underlining variants (straight, double, curly, dotted, curly, dashed) and whether underlining can have it's own color
    - The name of the terminal app being used
    - The terminal's locale, and character encoding
    - Whether a nerd font is being used in terminal

    > this discovery in the library is enabled by the `Terminal` struct.

    When a user installs the CLI, they can run `bt` without any parameters to see the information that the `biscuit-terminal` library has discovered about the current terminal.

2. Component Rendering

    The `biscuit-terminal` library defines a **Renderable** trait which provides a consistent interface for components. These components include:

    - [`Table`](./docs/components/table.md)
    - [`TwoColumn`](./docs/components/two_column.md)
    - [`Prose`](./docs/components/prose.md)
    - [`BlockQuote`](./docs/components/block_quote.md)
    - [`TerminalImage`](./docs/components/terminal_image.md)
    - [`Filesystem`](./docs/components/file_system.md)
    - `Progress`
    - `MermaidDiagram` (Mermaid adapter backed by `biscuit-visualized`)
    - `GraphExpression` (graph adapter backed by `biscuit-visualized`)
    - `Todo`
    - `PadLeft` and `PadRight`
    - `OrderedList` and `UnorderedList`
    - `TextBlock`
  
    As well as compositional components:

    - `Compose` and `InlineContent`

    These components all respect the `Layout` struct's ideas of margins, word-wrap, and other useful features.

## More Information

For more information on either the library or CLI refer to more detailed documents on each:

- [Biscuit Terminal Library](./lib/README.md) for details on how to use `biscuit-terminal` programmatically
- [The Biscuit Terminal CLI](./cli/README.md) for details on how to leverage `biscuit-terminal` from the terminal
