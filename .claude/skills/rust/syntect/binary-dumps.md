# Binary Dumps

Loading syntax definitions and themes from `.sublime-syntax` and `.tmTheme` files can be slow (100ms+). For production applications, pre-compile them into **binary dumps** at build time for ~5ms startup.

## Why Binary Dumps?

**Problem:**
- Default `SyntaxSet::load_defaults_newlines()` parses many `.sublime-syntax` files
- Default `ThemeSet::load_defaults()` parses many `.tmTheme` XML files
- This adds 100-200ms to application startup

**Solution:**
- Build a binary representation once (during build or first run)
- Load the binary at runtime (5-10ms)

## Creating Binary Dumps

### Syntaxes

```rust
use syntect::parsing::SyntaxSetBuilder;
use std::fs::File;

fn main() {
    // Build from .sublime-syntax files in a directory
    let mut builder = SyntaxSetBuilder::new();
    builder.add_from_folder("path/to/syntaxes", true).unwrap();
    let syntax_set = builder.build();

    // Serialize to binary
    syntect::dumps::dump_to_file(&syntax_set, "syntaxes.bin").unwrap();
}
```

### Themes

```rust
use syntect::highlighting::ThemeSet;

fn main() {
    // Load themes from .tmTheme files
    let theme_set = ThemeSet::load_from_folder("path/to/themes").unwrap();

    // Serialize to binary
    syntect::dumps::dump_to_file(&theme_set, "themes.bin").unwrap();
}
```

## Loading Binary Dumps

### At Runtime

```rust
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::dumps::from_binary;

fn main() {
    // Load from binary files (fast!)
    let syntax_set: SyntaxSet = from_binary(include_bytes!("syntaxes.bin"));
    let theme_set: ThemeSet = from_binary(include_bytes!("themes.bin"));

    // Use normally
    let syntax = syntax_set.find_syntax_by_extension("rs").unwrap();
    let theme = &theme_set.themes["MyTheme"];
}
```

### Using `include_bytes!`

Embed the binary dumps directly into your executable:

```rust
const SYNTAX_SET_BYTES: &[u8] = include_bytes!("../assets/syntaxes.bin");
const THEME_SET_BYTES: &[u8] = include_bytes!("../assets/themes.bin");

lazy_static::lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = from_binary(SYNTAX_SET_BYTES);
    static ref THEME_SET: ThemeSet = from_binary(THEME_SET_BYTES);
}
```

## Build-time Integration

### Using `build.rs`

Create a `build.rs` script to generate dumps during compilation:

```rust
// build.rs
use syntect::parsing::SyntaxSetBuilder;
use syntect::highlighting::ThemeSet;
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Build syntax set
    let mut builder = SyntaxSetBuilder::new();
    builder.add_from_folder("syntaxes", true).unwrap();
    let syntax_set = builder.build();
    syntect::dumps::dump_to_file(&syntax_set, out_dir.join("syntaxes.bin")).unwrap();

    // Build theme set
    let theme_set = ThemeSet::load_from_folder("themes").unwrap();
    syntect::dumps::dump_to_file(&theme_set, out_dir.join("themes.bin")).unwrap();

    println!("cargo:rerun-if-changed=syntaxes/");
    println!("cargo:rerun-if-changed=themes/");
}
```

Then load in your code:

```rust
const SYNTAX_SET: SyntaxSet = from_binary(include_bytes!(concat!(env!("OUT_DIR"), "/syntaxes.bin")));
const THEME_SET: ThemeSet = from_binary(include_bytes!(concat!(env!("OUT_DIR"), "/themes.bin")));
```

## Performance Comparison

| Method | Startup Time | Binary Size |
|--------|--------------|-------------|
| `load_defaults_newlines()` | ~150ms | N/A (loads from disk) |
| Binary dump (file load) | ~10ms | +500KB to app |
| Binary dump (`include_bytes!`) | ~5ms | +500KB to executable |

## Common Patterns

### Using Default Syntaxes + Custom Themes

```rust
// Use built-in syntaxes but custom themes
let syntax_set = SyntaxSet::load_defaults_newlines();
let theme_set: ThemeSet = from_binary(include_bytes!("themes.bin"));
```

### Lazy Loading

Use `lazy_static` or `once_cell` to load dumps once on first use:

```rust
use once_cell::sync::Lazy;
use syntect::parsing::SyntaxSet;
use syntect::dumps::from_binary;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    from_binary(include_bytes!("syntaxes.bin"))
});

// First access initializes, subsequent accesses reuse
let syntax = SYNTAX_SET.find_syntax_by_extension("rs").unwrap();
```

## Gotchas

- **Binary format may change**: When upgrading Syntect, regenerate binary dumps
- **Platform compatibility**: Binaries are platform-independent (portable across OS/arch)
- **Theme updates**: If you modify `.tmTheme` files, regenerate dumps
- **Syntax updates**: If you add/modify `.sublime-syntax` files, regenerate dumps
- **Version lock**: Binary dumps are tied to the Syntect version; regenerate after upgrading

## When to Use

| Scenario | Recommendation |
|----------|----------------|
| CLI tool run frequently | Yes - users notice startup time |
| Long-running server | Maybe - one-time cost at startup |
| Library for others to use | No - let users decide on caching |
| Prototype/dev | No - use `load_defaults_*()` for simplicity |

## Modern Themes

By default, `syntect` relies on a set of built-in themes (like `base16-ocean.dark` and `Solarized`) that are quite dated. To use modern themes like **Dracula**, **Nord**, **Ayu**, or **One Dark**, you have two main approaches.

### Loading External `.tmTheme` Files

The most stable way to bring in new themes is to download their `.tmTheme` (XML) versions.

**Where to find them:**
- **Dracula:** [dracula/textmate](https://github.com/dracula/textmate)
- **Nord:** [nordtheme/sublime-text](https://github.com/nordtheme/sublime-text)
- **One Dark:** [Ice-Man-007/sublime-text-one-dark-theme](https://github.com/Ice-Man-007/sublime-text-one-dark-theme)
- **Rainglow:** [320+ themes](https://github.com/rainglow/sublime) in `.tmTheme` format

**Loading themes:**

```rust
use syntect::highlighting::ThemeSet;

// Option A: Load a specific .tmTheme file
let theme = ThemeSet::get_theme("path/to/Dracula.tmTheme")
    .expect("Failed to load theme");

// Option B: Load a folder of themes into a Set
let mut theme_set = ThemeSet::load_defaults(); // Start with defaults
let extra_themes = ThemeSet::load_from_folder("assets/themes")
    .expect("Failed to load folder");

// Merge them or just use the new set
theme_set.themes.extend(extra_themes.themes);
```

### Bundling Themes in the Binary

For portable applications, embed themes directly using `include_bytes!`:

```rust
use std::io::Cursor;
use syntect::highlighting::{Theme, ThemeSet};

fn get_embedded_theme(name: &str) -> Option<Theme> {
    let bytes = match name {
        "dracula" => Some(include_bytes!("../assets/themes/Dracula.tmTheme") as &[u8]),
        "nord"    => Some(include_bytes!("../assets/themes/Nord.tmTheme") as &[u8]),
        "ayu"     => Some(include_bytes!("../assets/themes/Ayu.tmTheme") as &[u8]),
        _ => None,
    }?;

    ThemeSet::load_from_reader(&mut Cursor::new(bytes)).ok()
}
```

### Binary Dumps for Maximum Performance

For professional applications, use binary dumps to pre-compile themes at build time:

**Cargo.toml:**

```toml
[dependencies]
syntect = "5.3"

[build-dependencies]
syntect = { version = "5.3", features = ["plist-load"] }
```

**build.rs:**

```rust
use syntect::highlighting::ThemeSet;
use syntect::dumps::dump_to_file;
use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let theme_dir = "assets/themes";
    let dump_path = Path::new(&out_dir).join("themes.bin");

    // Load all .tmTheme files into a ThemeSet
    let theme_set = ThemeSet::load_from_folder(theme_dir)
        .expect("Failed to load themes from folder");

    // Serialize the set into a compressed binary file
    dump_to_file(&theme_set, &dump_path)
        .expect("Failed to dump themes to binary");

    println!("cargo:rerun-if-changed={}", theme_dir);
}
```

**main.rs:**

```rust
use syntect::highlighting::ThemeSet;
use syntect::dumps::from_binary;
use once_cell::sync::Lazy;

// Lazy-loaded static theme set from binary dump
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    let theme_data = include_bytes!(concat!(env!("OUT_DIR"), "/themes.bin"));
    from_binary(theme_data)
});

fn main() {
    let my_theme = &THEME_SET.themes["Dracula"];
    println!("Loaded {} themes instantly.", THEME_SET.themes.len());
}
```

### Performance Comparison

| Format | Pros | Cons | Startup Time |
| --- | --- | --- | --- |
| **XML (.tmTheme)** | Human readable, easy to edit | Slow to parse at runtime | ~100-200ms |
| **Binary (.bin)** | **Blazing fast** loading | Not human-readable | <5ms |
| **Embedded Binary** | Single executable, portable | Larger binary size | <1ms |

### Modern `.sublime-color-scheme` Support

Modern Sublime themes use a JSON-based `.sublime-color-scheme` format which `syntect` does **not** support natively. Use the [sublime-color-scheme](https://crates.io/crates/sublime-color-scheme) crate as a bridge:

```toml
[dependencies]
syntect = "5.0"
sublime-color-scheme = "0.2"
```

```rust
use syntect::highlighting::Theme;
use sublime_color_scheme::convert_sublime_scheme;

fn load_modern_theme(json_content: &str) -> Theme {
    convert_sublime_scheme(json_content)
        .expect("Failed to parse modern theme")
}
```

## Related

- [Terminal Output](./terminal-output.md) - Use with ANSI highlighting
- [HTML Generation](./html-generation.md) - Use with HTML output
- [Large File Handling](./large-files.md) - Combine with streaming for max performance
