# Theme Strategy

## Strategy Dimensions

Our theme strategy has two major dimensions:

1. Mode (light/dark)
2. Scope (prose/code block)

We'll cover each in the following sections.

### Mode

How we address the `mode` of output will depend on our target output:

1. Terminal
2. HTML / Browser

When we're targeting the terminal we must render to a _specific_ mode. Light or dark; not both. When we're targeting the browser, however, we address both modes but the HTML file must contain the appropriate styling to adjust to the host's preferred mode for display purposes.

### Scope

We are _styling_/_highlighting_ two distinct scopes:

1. Prose content
2. Fenced Code Blocks

It is important that the code blocks **stand out** from the rest of the page and for that reason we will use a _different_ theme for code blocks then that we use for the prose.

It's also worth mentioning that not only our theme changes based on scope but our approach. Using `syntect` and `two-face` for highlighting code blocks is relatively straight forward. Far less so for the prose content and for that reason we've come up with this strategy document on how to implement styling for prose:

- [Highlighting the Prose](./highlighting-the-prose.md)


## Choosing a Theme

This section really ought to be called "choosing the themes" (plural) as what we need to be able to do is to resolve a `ThemePair` for both prose and fenced code blocks. In addition, although this step is relatively easy, we need to map the `ThemePair` back to it's constituent `Theme` definitions.

Let's start by choosing the `ThemePair` for the **prose** content.

- by default the prose will use `ThemePair::OneHalf`
- Overrides:
    - if the environment variable `THEME` is set to a valid _kebab-cased_ variant of `ThemePair` then this theme will be used 
    - if the caller explicitly states the theme by setting the `theme` property to a `ThemePair` variant then this will override not only the default but any ENV settings found.

Typically the `ThemePair` for code blocks is _derived_. This is achieved via a lookup table `CODE_THEME_LOOKUP`:

```rust
lazy_static {
    /// the keys represent the `ThemePair` for the prose content
    /// which map to the `ThemePair` for the code blocks
    static ref CODE_THEME_LOOKUP: HashMap<ThemePair, ThemePair> = {
        let mut m = HashMap::new();
        // default mapping
        map.insert(ThemePair::OneHalf, ThemePair::Monokai);
        // other mappings
        map.insert(ThemePair::Base16Ocean, ThemePair::Github);
        map.insert(ThemePair::Github, ThemePair::Monokai);
        map.insert(ThemePair::Gruvbox, ThemePair::Github);
        map.insert(ThemePair::Solarized, ThemePair::Github);
        map.insert(ThemePair::Nord, ThemePair::Github);
        map.insert(ThemePair::Dracula, ThemePair::Github);
        map.insert(ThemePair::VisualStudioDark, ThemePair::Monokai);

        m
    }
}
```

However, just like the prose theme there are overrides where the inferred `ThemePair` is passed over in favor of an explicit choice:

- if the environment variable `CODE_THEME` is set to a valid _kebab-cased_ variant of `ThemePair` then 


### Background Colors

The way we handle _theme_ background colors depends on the output target:

- Terminal

    When targeting the terminal the **prose** will NOT use a background color but the code blocks **will** use the background color specified in the code block's theme.

    > **Note:** this discussion is purely about background color and not _inline_ background color. Read the [Terminal Output](./syntect-terminal-output.md) document for more details on this distinction as well as code examples for rendering to the terminal.

- HTML / Browser

    When targeting the browser with HTML we use the background color for _both_ the prose content and the code block content. However, the background color should be different based on each having it's own Theme defining the background color.

