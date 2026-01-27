# Biscuit Terminal

<table>
  <tr>
    <td><img src="../assets/biscuit-terminal-512.png" style="max-width='25%'" width=200px /></td>
    <td>
      <h2>biscuit-terminal</h2>
      <p>This shared library provides support for working with the terminal:</p>
      <ul>
        <li>terminal <b>metadata</b> (<i>color support, character width and height, light/dark themed, etc.</i>)</li>
        <li>OSC8 Links</li>
        <li>multiplex support for Wezterm, Ghostty</li>
        <li>smart rendering utilities to write to the terminal in style</li>
      </ul>
    </td>
  </tr>
</table>

## The `Terminal` struct

While this library provides plenty of useful utility functions, the `Terminal` struct is a great starting point as it brings a lot of the capabilities into focus in one type-safe way.

```rust
use biscuit_terminal::prelude::*;

let term = Terminal::new();
```

This simple code block give you access to key global information. The `Terminal` properties represent **static**/non-changing aspects of the terminal whereas there are many other methods which will probe aspects to the terminal which can be changing over time.

### Static Discovery

```rust
use biscuit_terminal::prelude::*;
let term = Terminal::new();

// the name of the terminal app being used (e.g., WezTerm, Ghostty, etc.)
let app = term::app.clone();
// `ColorDepth` enum describing the color depth the terminal supports
let color_depth = term::color_depth.clone();
// boolean flag indicating whether the terminal supports OSC8 based links
let osc_link_support = term::osc_link_support.clone();
// boolean flag indicating whether the terminal is a TTY terminal
let is_tty = term::is_tty.clone();
// whether or not the terminal supports the rendering of italic characters
let italics_support = term::italics_support.clone();
// the kinds of underlines supported and whether colored underlines are supported
let underline_support = term::underline_support.clone();
// what type of image support the terminal provides (if any)
let image_support = term::image_support.clone();
// whether the terminal supports OSC52 clipboard interactions
let clipboard_support = term::clipboard_support.clone();
// whether the terminal supports the Mode 2027 Proposal to support grapheme clustering
let mode_2027_support = term::mode_2027_support;
// the operating system this terminal is running in
let os = term::os.clone();
// the distro this terminal is running in (if in a Linux OS)
let distro = term::distro.clone();
// the configuration file used to configure the App's exposed settings
let config_file = term::config_file.clone();

let multiplexer = term::inside_multiplexer().clone();
```


> **Note:** the code example is just illustrative, actually cloning each individual property of a terminal is nonsensical.

### Dynamic Discovery

While certain things, like a terminal's background color, don't change often they _can_ change and therefore there is another whole set of terminal metadata which is provided via function calls:

```rust
use biscuit_terminal::prelude::*;
let term = Terminal::new();

// the width of the terminal (in characters)
let width = Terminal::width();
// the height of the terminal pane (in rows)
let height = Terminal::height();
// the color mode (light/dark) this terminal is using
let color_mode = Terminal::color_mode();
// The default text color (what the RESET escape code will return to).
// Uses OSC11 to observe this.
let text_color = Terminal::text_color();
// The default background color (what the RESET escape code will return to).
// Uses OSC10 to observe this.
let bg_color = Terminal::bg_color();
// Uses OSC12 to observe the color of the cursor
let cursor_color = Terminal::cursor_color();

let clipboard = Terminal::get_clipboard();
```

### Terminal Multiplexing and Mutation

```rust
use biscuit_terminal::prelude::*;
let term = Terminal::new();

// set the terminal's window title (if supported)
term::set_title("My Terminal");

// split the current window pane vertically (if supported)
// and provide focus to the upper pane.
term::split_vertically(Focus::Top);
// split the current window pane horizontally (if supported)
// and provide focus to the left-hand pane.
term::split_horizontally(Focus::Left);
```



## Rendering to the Terminal

Beyond _discovery_ this crate plays an active role in helping callers to write to the terminal in a rich manner. This is achieved in part by the `Compose` struct and the `Renderable` trait:

- `Compose` struct

    Allows for the composition of one or more _renderable_ blocks.

- `Renderable` trait

    A struct which implements the `Renderable` trait must provide a `render() -> String` function. With this assurance, all renderable structs can be reduced to a string thereby making them _composable_.

### Renderable Components

The following renderable _components_ are provided in this library:

- `Prose`
- `TextBlock`
- `Currency`, `Numeric`, `Metric`
- `Table`
- `UnorderedList`
- `OrderedList`
- `TerminalImage`

In the following sections we'll cover each in more detail but before we do let's quickly mention that you can write to the terminal either _via_ the `Terminal` crate or _via_ the `Compose` struct:

```rust

```

### `Prose` struct

The `Prose` struct allows for plain text to be parsed for two token types:

1. Atomic Tokens (e.g., `{{red}}`, `{{bold}}`, `{{reset}}`, etc.)
2. Block Tokens (e.g., `<i>italics</i>`, `<b>bold</b>`, etc.)

These tokens are defined in the documentation for the struct but in a nutshell they are both ways to indicate styling (color, bold, dim, italics, etc.) within the body of the text. When the `Prose` **render** function is called it will replace these tokens with the appropriate escape codes to achieve the desired effect.

### `TextBlock` struct

The `TextBlock` struct is a block of text which the caller wants to have styled uniformly across the text. It uses a builder pattern to define what the escape codes which should _wrap_ the text are.

```rust
let heading = TextBlock("This is my heading")
    .as_bold()
    .text_color(Color::Basic(Blue))
    .bg_color(Color::WebColor(HoneyDew))
    .dotted_underline()
    .build();
```

### Numeric structs: `Currency`, `Numeric`, and `Metric`

The numeric structs provided are meant as a way to uniformly (and with nice formatting) represent numeric values in the terminal:

- `Numeric` is the most basic and simply provides some basic options such as setting a precision, use of commas, etc.

    ```rust
    let population = Numeric::new("123900".into())
        // how many decimal places of precision do we want?
        .precision(0)
        // How to handle moving the desired precision:
        // - Precision::{Round,Floor,Error}
        //
        // the default is Precision::Round.
        .handle_precision(Precision::Round)
        // use commas between every three digits
        .use_commas(Comma::ThreeDigits)
        // how should we treat an undefined value for this
        .handle_undefined(HandleUndefined::AsZero)
        .build();
    ```

- `Currency` is a numeric value meant to represent a monetary amount in some currency. It adds to the builder pattern configuration of `Numeric` to accommodate it's more specific task.
- `Metric` is a numeric quantity that has both a numeric and "unit of measure" component to it.

### `Table` struct

Of all the components offered, the `Table` struct is the most complex and therefore will be defined not here, but in the `./lib/src/components/table/README.md` file. In general though, it's function is to provide a well formed table that is responsive to the amount of space available and context-aware of the escape codes which may be used in a modern terminal and able to not be

### Lists: `UnorderedList` and `OrderedList`

Both list structs are represented by a 1:M _elements_ where the elements are any _renderable_ element. They are, however, most typically either a `String` (the text of an element) or another list struct (to achieve nesting).


## The `bt` CLI

While this package is mainly about providing terminal capabilities to other libraries, it does also come with a CLI which can be used to inspect the terminal.


