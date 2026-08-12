# Using TypeScript Grammar with Syntect

Using the `syntect` crate with TypeScript (TS) is a common point of confusion because **TypeScript is often not included in the default `SyntaxSet`** (or is an older version that doesn't handle modern TS features).

To use a proper TypeScript grammar, you have two main options: manually loading a `.sublime-syntax` file or using a "batteries-included" crate like `two-face`.

## Option 1: Manual (Loading a Custom Grammar)

`syntect` is built to use Sublime Text syntax definitions. You need to obtain a `TypeScript.sublime-syntax` file and load it using a `SyntaxSetBuilder`.

### 1. Obtain the Grammar

The official TypeScript grammar for Sublime is maintained by Microsoft, but it is often distributed as a `.tmLanguage` file. `syntect` works best with the newer `.sublime-syntax` format.

**Recommended Source:** Download the [TypeScript.sublime-syntax](https://github.com/sharkdp/bat/blob/master/assets/syntaxes/02_Extra/TypeScript.sublime-syntax) from the `bat` project (which uses `syntect` internally).

### 2. Load it in Rust

Put the `.sublime-syntax` file in a folder (e.g., `assets/syntaxes/`) and load it as follows:

```rust
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn main() {
    // 1. Start with a builder
    let mut builder = SyntaxSetBuilder::new();

    // 2. Add the default syntaxes (JS, Rust, etc.) if you want them too
    builder.add_from_folder("assets/syntaxes/").expect("Failed to load TS grammar");

    // Optional: add defaults as well
    // builder.add_plain_text_syntax();

    // 3. Build the final set (this links internal references)
    let ps = builder.build();
    let ts = ThemeSet::load_defaults();

    // 4. Find the TypeScript syntax specifically
    let syntax = ps.find_syntax_by_extension("ts")
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let s = "interface User { id: number; name: string; }";

    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        println!("{}", escaped);
    }
}
```

## Option 2: The "Batteries-Included" Way (`two-face`)

If you don't want to manage syntax files manually, the [two-face](https://crates.io/crates/two-face) crate provides extra syntax definitions (including TypeScript, TOML, and Dockerfile) that are missing from `syntect`'s default set.

**Cargo.toml:**

```toml
[dependencies]
syntect = "5.2"
two-face = { version = "0.4", features = ["syntect-onig"] }
```

**Rust code:**

```rust
use syntect::parsing::SyntaxSet;
use two_face;

fn main() {
    // Get a SyntaxSet that includes TypeScript automatically
    let ps = two_face::syntax::extra_newlines();

    let syntax = ps.find_syntax_by_extension("ts")
        .expect("TypeScript should be here!");

    // Proceed with highlighting...
}
```

## Important: `onig` vs `fancy-regex`

Modern TypeScript grammars use advanced regular expressions (like lookaheads).

- **Oniguruma (`onig`):** This is the default. It is a C library and is the most compatible with Sublime grammars.
- **fancy-regex:** A pure-Rust alternative. If you use this, some complex TypeScript grammars might fail to compile or behave incorrectly. If you run into issues, ensure the `onig` feature is enabled in your `Cargo.toml`.

## Comparison of Methods

| Feature | Manual `.sublime-syntax` | `two-face` Crate |
| --- | --- | --- |
| **Control** | Full control over grammar version | Fixed to what the crate provides |
| **Setup** | Requires managing external files | Just a dependency |
| **Binary Size** | Minimal (only what you add) | Larger (bundles many syntaxes) |
| **Reliability** | High (if using a good `.sublime-syntax`) | High |

## When to Use

| Scenario | Recommendation |
|----------|----------------|
| TypeScript-focused project | Use `two-face` for simplicity |
| Custom/modified grammars | Manual approach with specific `.sublime-syntax` |
| Minimal binary size | Manual with only TS grammar |
| Multiple extra languages | `two-face` (includes TOML, Dockerfile, etc.) |

## Related

- [Binary Dumps](./binary-dumps.md) - Pre-compile custom syntaxes for fast startup
