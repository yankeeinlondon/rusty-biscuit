# Using `syntect` when targeting the terminal

When the `syntect` crate produces output for the terminal it uses generators from the `syntect::util` module and `HighlightLines`. Below you'll find a fairly typical modules you'd want to use:

```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
```

## Background Colors

When we talk about _background colors_ we must distinguish between two things:

1. **Base Background**. A base background color applied to the background of each line.
2. **Inline**. Some theme's will suggest a background color be applied to certain inline portions of the page. A classic example of this is when we add `backticks` around a word (or set of words).

### Base Background

The base background comes from the `ThemeSettings` field of the `Theme` struct, specifically `settings.background`. This is the "canvas" color for your code block.

Critically you must print the base background color code **before** processing any lines and reset it after. Here's a code example of how you might do that:

```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn highlight_terminal_with_base_background() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("md").unwrap();
    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let text = "The whole line gets the base background color";

    // Extract the base background color from the theme settings
    let base_bg = theme.settings.background.expect("Theme has no base background");
    
    // ANSI escape code to set the background color for the ENTIRE block
    // The format is: \x1b[48;2;{r};{g};{b}m
    let bg_code = format!("\x1b[48;2;{};{};{}m", base_bg.r, base_bg.g, base_bg.b);
    let reset_code = "\x1b[0m";

    // 1. SET BASE BACKGROUND (applies to the whole block)
    print!("{}", bg_code);

    for line in LinesWithEndings::from(text) {
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ps).unwrap();
        
        // Inline backgrounds are INCLUDED here
        let escaped = as_24_bit_terminal_escaped(&ranges, true);
        print!("{}", escaped);
    }

    // 2. RESET BASE BACKGROUND (to prevent affecting subsequent terminal text)
    print!("{}", reset_code);
}
```

> **Key Point:** The `IncludeBackground` enum on the generator only controls the inline backgrounds. The base background must be manually extracted from `theme.settings.background` and applied to your HTML container.

### Inline Backgrounds

Inline backgrounds are specific to individual _scopes_ or _tokens_ (e.g., highlighting `code` spans with a gray background). These are defined in the `theme.scopes` (as `ThemeItems`), **not** in `ThemeSettings`.

To turn on or off the _inline_ backgrounds you will use the `as_24_bit_terminal_escaped()` function. It is named as such because the primary consideration on whether to use or not use the inline background colors is based on the terminal being able to have enough colors so that the background color can be rendered with enough contrast to make having it worth it.

> **IMPORTANT:**
>
> - In this repo we will detect the terminal's color depth using the `color_depth()` utility function in the [shared-library](../../../shared/docs/color_depth.md) as this is **not** a feature provided by the `syntect` crate.
> - Our rendering functions for the terminal should allow for this check to be overridden in favor of a fixed true/false value.

Here's a simple example of how you'd turn _inline_ syntax highlighting on or off.

```rust
// ... (inside the line loop)

let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ps).unwrap();

// To ENABLE inline backgrounds
let escaped_with_inline = as_24_bit_terminal_escaped(&ranges, true);

// To DISABLE inline backgrounds
let escaped_no_inline = as_24_bit_terminal_escaped(&ranges, false);

print!("{}", escaped_with_inline);
```

