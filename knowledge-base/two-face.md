# two-face — Deep Dive (Rust)

A consolidated, standalone reference for the `two-face` crate: what it is, why it exists, how to use it with `syntect`, and what to watch out for in real projects.

---

## Table of Contents

1. [What is two-face?](#what-is-two-face)
1. [How two-face fits into the syntax highlighting ecosystem](#how-two-face-fits-into-the-syntax-highlighting-ecosystem)
1. [Installation and feature flags](#installation-and-feature-flags)
1. [Core concepts and mental model](#core-concepts-and-mental-model)
1. [API tour](#api-tour)
   * 5.1 [Syntax sets: `extra_newlines` vs `extra_no_newlines`](#syntax-sets-extra_newlines-vs-extra_no_newlines)
   * 5.2 [Theme sets and `EmbeddedThemeName`](#theme-sets-and-embeddedthemename)
   * 5.3 [`re_exports::syntect` convenience re-export](#re_exportssyntect-convenience-re-export)
   * 5.4 [Acknowledgements / asset licensing](#acknowledgements--asset-licensing)
1. [Getting started: highlight to HTML](#getting-started-highlight-to-html)
1. [Terminal highlighting (ANSI / 24-bit color)](#terminal-highlighting-ansi--24-bit-color)
1. [Advanced usage patterns](#advanced-usage-patterns)
   * 8.1 [Language detection strategies](#language-detection-strategies)
   * 8.2 [Theme curation for end-users](#theme-curation-for-end-users)
   * 8.3 [TUI/GUI rendering considerations](#tuigui-rendering-considerations)
   * 8.4 [Web services and container deployments](#web-services-and-container-deployments)
1. [Gotchas and limitations](#gotchas-and-limitations)
1. [Performance and binary size](#performance-and-binary-size)
1. [Licensing](#licensing)
1. [When to use two-face (and when not to)](#when-to-use-two-face-and-when-not-to)
1. [Alternatives and related libraries](#alternatives-and-related-libraries)
1. [Version and changelog notes](#version-and-changelog-notes)

---

## What is two-face?

**`two-face`** is a Rust crate that provides **additional syntax definitions and theme definitions** for the [`syntect`](https://crates.io/crates/syntect) syntax highlighting engine.

`syntect` can highlight using Sublime Text `.sublime-syntax` grammars and TextMate themes, but its default embedded sets are not exhaustive. `two-face` fills that gap by bundling a **curated set of extra syntaxes and themes**—notably including many modern/common formats that users expect “out of the box” (examples called out in the research: **TOML, TypeScript, Dockerfile**, plus many more).

A key detail: this collection is curated by the **`bat`** project team (the popular `cat`-like viewer with syntax highlighting). In practice, this means `two-face` is best thought of as a **battle-tested asset bundle** for `syntect`, based on what `bat` ships to users.

The name “two-face” is a playful reference to chasing the “bat” man (Batman).

---

## How two-face fits into the syntax highlighting ecosystem

`two-face` is **not** a highlighting engine. It does not tokenize text or render HTML/ANSI itself. Instead:

* **`syntect`** = the highlighting engine (parsing + theming + output utilities)
* **`two-face`** = a **data provider** (embedded syntaxes + themes) that plugs into `syntect`

This separation is useful because you can keep using the well-known `syntect` API and swap how you source syntax/theme sets.

---

## Installation and feature flags

Add to `Cargo.toml`:

````toml
[dependencies]
two-face = "0.5.1"
````

Or:

````bash
cargo add two-face
````

### Feature flags: regex backend compatibility

`two-face` must match `syntect`’s regex backend choice. The crate exposes feature flags that correspond to `syntect` defaults:

* **Default / Oniguruma (C-based regex):**
  ````bash
  cargo add two-face --features syntect-default-onig
  ````

* **Alternative / fancy-regex (pure Rust):**
  ````bash
  cargo add two-face --features syntect-default-fancy
  ````

**Important:** choose the same regex implementation that `syntect` uses in your project, or you can hit compilation failures and/or missing syntaxes.

Example of matching `syntect` + `two-face` in “fancy” mode:

````toml
[dependencies]
syntect = { version = "5.3", default-features = false, features = ["default-fancy-regex"] }
two-face = { version = "0.5", features = ["syntect-default-fancy"] }
````

---

## Core concepts and mental model

A typical `syntect` flow looks like:

1. Load a `SyntaxSet` (the language grammars)
1. Load a `ThemeSet` (themes)
1. Pick a syntax (often by extension, filename, or first line)
1. Pick a theme
1. Highlight and render (HTML, ANSI, etc.)

`two-face` simplifies steps (1) and (2) by returning ready-to-use sets compatible with `syntect`.

---

## API tour

`two-face` exposes three primary entry points:

|Module|Function|What you get|
|------|--------|------------|
|`two_face::syntax`|`extra_newlines()`|`syntect::parsing::SyntaxSet` containing extra syntaxes, configured with newline parsing|
|`two_face::syntax`|`extra_no_newlines()`|`SyntaxSet` variant without newline-based parsing|
|`two_face::theme`|`extra()`|`syntect::highlighting::ThemeSet` containing embedded themes|

There is also:

* `two_face::re_exports::syntect` (a convenience re-export so examples can import syntect types consistently)
* `two_face::theme::EmbeddedThemeName` (an enum representing embedded theme names—examples in the research include **Nord**, **Dracula**, **Monokai**, **SolarizedDark**, and more)
* `two_face::acknowledgement` for embedded asset attribution/licensing

### Syntax sets: `extra_newlines` vs `extra_no_newlines`

* Use **`extra_newlines()`** for most “batch” highlighting workflows (HTML generation, files, etc.).
* Use **`extra_no_newlines()`** when you explicitly need to avoid newline-based parsing behavior—often relevant in **line-by-line rendering** scenarios (TUI rendering loops), or when you want tighter control over incremental highlighting.

### Theme sets and `EmbeddedThemeName`

Themes are accessed by indexing into the returned theme set using the embedded theme name enum, e.g.:

````rust
let theme_set = two_face::theme::extra();
let theme = &theme_set[two_face::theme::EmbeddedThemeName::Nord];
````

### `re_exports::syntect` convenience re-export

The crate re-exports `syntect`:

````rust
use two_face::re_exports::syntect;
````

This is mainly ergonomic: your code examples and imports can consistently refer to `syntect` types/functions through `two-face`’s view of the dependency graph.

### Acknowledgements / asset licensing

Because `two-face` embeds third-party syntax and theme assets (from the TextMate/Sublime ecosystem via `bat`’s curation), you often need attribution details. `two-face` exposes a listing:

````rust
fn main() {
    let acknowledgements = two_face::acknowledgement::listing();
    println!("{}", acknowledgements);
}
````

---

## Getting started: highlight to HTML

This is the simplest end-to-end integration: pick a syntax by extension, pick a theme, render HTML.

````rust
use two_face::re_exports::syntect;

const TOML_TEXT: &str = r#"
[section]
key = 123
"#;

fn main() {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syn_ref = syn_set.find_syntax_by_extension("toml").unwrap();
    let theme = &theme_set[two_face::theme::EmbeddedThemeName::Nord];

    let html = syntect::html::highlighted_html_for_string(
        TOML_TEXT,
        &syn_set,
        syn_ref,
        theme,
    )
    .unwrap();

    println!("{}", html);
}
````

### Highlighting other common “gap” languages

TypeScript:

````rust
use two_face::re_exports::syntect;

const TYPESCRIPT_CODE: &str = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    return `Hello, ${user.name}!`;
}
"#;

fn main() {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syn_ref = syn_set.find_syntax_by_extension("ts").unwrap();
    let theme = &theme_set[two_face::theme::EmbeddedThemeName::Dracula];

    let html = syntect::html::highlighted_html_for_string(
        TYPESCRIPT_CODE,
        &syn_set,
        syn_ref,
        theme,
    )
    .unwrap();

    println!("{}", html);
}
````

Dockerfile:

````rust
use two_face::re_exports::syntect;

const DOCKERFILE: &str = r#"
FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /app/target/release/myapp /usr/local/bin/
CMD ["myapp"]
"#;

fn main() {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syn_ref = syn_set.find_syntax_by_extension("Dockerfile").unwrap();
    let theme = &theme_set[two_face::theme::EmbeddedThemeName::Monokai];

    let html = syntect::html::highlighted_html_for_string(
        DOCKERFILE,
        &syn_set,
        syn_ref,
        theme,
    )
    .unwrap();

    println!("{}", html);
}
````

---

## Terminal highlighting (ANSI / 24-bit color)

For CLI tools (a major `two-face` use case), you typically highlight line-by-line and convert styled ranges into ANSI escapes.

````rust
use two_face::re_exports::syntect;
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn main() {
    let syn_set = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();

    let syntax = syn_set.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(
        syntax,
        &theme_set[two_face::theme::EmbeddedThemeName::Nord],
    );

    let content = "fn main() { println!(\"Hello\"); }\n";
    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, &syn_set).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped);
    }
}
````

Notes:

* This yields best results in terminals that support **24-bit color**.
* For very large files, consider incremental/lazy rendering to keep UI responsive.

---

## Advanced usage patterns

### Language detection strategies

In real tools, extension-based detection is not always enough. `syntect` offers alternatives, and `two-face`’s syntax set participates normally.

Example: detect by shebang/first line, then fall back to extension:

````rust
fn main() {
    let syn_set = two_face::syntax::extra_newlines();

    let syntax = syn_set
        .find_syntax_by_first_line("#!/bin/bash")
        .or_else(|| syn_set.find_syntax_by_extension("sh"))
        .unwrap();

    println!("Detected syntax: {}", syntax.name);
}
````

### Theme curation for end-users

`two-face` includes many themes, but in product UX it’s often better to offer a small, curated set rather than an overwhelming list.

````rust
const SUPPORTED_THEMES: &[two_face::theme::EmbeddedThemeName] = &[
    two_face::theme::EmbeddedThemeName::Nord,
    two_face::theme::EmbeddedThemeName::Dracula,
    two_face::theme::EmbeddedThemeName::SolarizedDark,
];

fn main() {
    let theme_set = two_face::theme::extra();
    for t in SUPPORTED_THEMES {
        // Accessing ensures the theme is present and usable
        let _theme = &theme_set[*t];
    }
}
````

### TUI/GUI rendering considerations

For TUI editors or log viewers, you may render line-by-line in a UI loop. The research explicitly notes that `extra_no_newlines()` can be a better fit for line-oriented rendering:

````rust
fn main() {
    let _syn_set = two_face::syntax::extra_no_newlines();
    let _theme_set = two_face::theme::extra();
    // tokenize/paint lines into your TUI widgets...
}
````

Performance considerations:

* Highlighting can be CPU-intensive on large inputs.
* Consider background processing, viewport-only highlighting, or caching.

### Web services and container deployments

`two-face` is attractive for “single static binary” deployments (containers, serverless-ish setups) because it embeds assets. It also helps avoid managing `syntaxes/` and `themes/` directories at runtime.

A key trade-off from the research:

* Using **`fancy-regex`** avoids C dependencies (helpful for portability), but can reduce syntax coverage (see [Gotchas](#gotchas-and-limitations)).

---

## Gotchas and limitations

### 1) Regex implementation mismatch (common build failure)

If `syntect` and `two-face` are built with different regex backends, you can get compilation errors or incompatibilities.

Mitigation: explicitly align features for both crates (example shown earlier).

### 2) Fancy-regex limitations reduce syntax coverage

When using the pure-Rust `fancy-regex` backend, some regex features used by certain syntaxes may not be supported. The research notes that **some syntaxes are excluded** in fancy mode.

Mitigation:

* If you need the broadest syntax set, prefer **Oniguruma** mode.
* If you need “pure Rust / no C deps,” accept that coverage may be smaller and test the languages you care about.

### 3) Choosing the wrong newline variant affects highlighting

`extra_newlines()` vs `extra_no_newlines()` can change highlighting behavior.

Guideline:

* Default to **`extra_newlines()`**.
* Use **`extra_no_newlines()`** only with a clear reason (often line-by-line rendering constraints).

### 4) Theme overload / poor UX

Shipping every theme as a user-facing option can degrade UX: users won’t know which to pick, and many are context-specific.

Mitigation: curate a subset (example provided above).

---

## Performance and binary size

* `two-face` itself is small in code size (research mentions ~**394 SLoC**), but it embeds a substantial asset bundle.
* The research estimates about **~0.6 MiB** binary size increase due to embedded syntax definitions/themes.

Practical note from the research: the Rust linker can discard unused assets, so “you only pay for what you use” to an extent—though you should still measure in your specific target (especially **WASM**, **mobile**, or **serverless cold start** scenarios).

---

## Licensing

* The crate is **dual-licensed under MIT OR Apache-2.0**, consistent with `bat`’s licensing approach.
* Embedded syntax/theme assets may have **their own licenses**. Use:

````rust
let acknowledgements = two_face::acknowledgement::listing();
````

…to retrieve attribution information for bundled assets.

---

## When to use two-face (and when not to)

### Use two-face when

* You already use (or want to use) **`syntect`**, but need better coverage of modern languages and config formats.
* You’re building:
  * developer-centric **CLI tools** (bat-like file viewers)
  * **static site generators** or doc tooling that outputs highlighted HTML
  * **IDEs/TUI editors** needing portable embedded assets
  * **code snippet services** that want consistent highlighting with a curated theme set

### Consider alternatives when

* **Binary size is extremely constrained** (embedded, WASM, strict mobile targets).
* You only need a couple common languages and prefer to manually manage minimal assets with plain `syntect`.
* You want **dynamic runtime theme loading** rather than embedding.
* You need higher-fidelity parsing than regex grammars provide.

---

## Alternatives and related libraries

### syntect (alone)

* Best when you want full control over which grammars/themes you ship (and potentially a smaller footprint).
* Trade-off: more asset management boilerplate; lacks many “expected” syntaxes out of the box.

### inkjet (Tree-sitter based)

* Grammar-based, often more accurate than regex-based highlighters.
* Trade-off: Tree-sitter grammars often involve C/C++ components; portability and WASM/cross compile can be more complex.

### syntastica

* Higher-level abstraction potentially spanning multiple backends (notably Tree-sitter).
* Trade-off: extra abstraction/complexity for simple use cases; smaller ecosystem.

### bat as a library

* If you want bat’s full terminal UX (paging, git gutters, etc.).
* Trade-off: heavy dependency footprint; API stability may be secondary to the CLI.

---

## Version and changelog notes

The provided research includes one “changelog” source that explicitly states it **cannot reliably enumerate** `two-face`’s version history or release timeline. As a result, this document **does not claim** a verified changelog or evolution narrative beyond what was provided.

What we can say from the research:

* Installation examples reference `two-face = "0.5.1"`.
* If you need a real version-by-version breakdown, consult:
  * crates.io: https://crates.io/crates/two-face
  * the repository’s Git tags/releases and `CHANGELOG.md` (if present)

---

## Practical snippets: introspection utilities

### List available syntaxes

````rust
fn main() {
    let syn_set = two_face::syntax::extra_newlines();

    for syntax in syn_set.syntaxes() {
        println!("Extension: {:?}, Name: {}", syntax.file_extensions, syntax.name);
    }
}
````

### List available themes

````rust
fn main() {
    let theme_set = two_face::theme::extra();

    for theme_name in theme_set.themes.keys() {
        println!("Theme: {}", theme_name);
    }
}
````

---

## Summary

`two-face` is the “missing asset bundle” for `syntect`: it packages the `bat` project’s curated syntaxes and themes into an easy-to-consume Rust crate. The result is a straightforward developer experience—load `SyntaxSet` and `ThemeSet` from `two-face`, then use standard `syntect` APIs to render HTML or ANSI output.

The main engineering trade-offs are:

* **Regex backend compatibility** (Oniguruma vs fancy-regex must match)
* **Potentially reduced syntax coverage** in pure-Rust fancy-regex mode
* **Binary size increase** due to embedded assets (often acceptable, but measure on constrained targets)

If your goal is `syntect` highlighting with modern language coverage and “batteries included” assets, `two-face` is the most direct path.