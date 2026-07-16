# Setup & feature flags (critical)

## Install

Add to `Cargo.toml`:

````toml
[dependencies]
two-face = "0.5.1"
````

or:

````bash
cargo add two-face
````

## Regex backend must match syntect

This is the #1 integration failure mode. `syntect` can be built with different regex engines; **two-face must use the same one**.

### Option A (default): Oniguruma (C-based)

Use when you want the **broadest syntax compatibility**.

````bash
cargo add two-face --features syntect-default-onig
````

### Option B: fancy-regex (pure Rust)

Use when you need **pure Rust builds** (e.g., some WASM/cross-compile environments) and accept that some syntaxes may be excluded.

````bash
cargo add two-face --features syntect-default-fancy
````

### Example: forcing syntect to fancy-regex (must match two-face)

````toml
[dependencies]
syntect = { version = "5.3", default-features = false, features = ["default-fancy-regex"] }
two-face = { version = "0.5", features = ["syntect-default-fancy"] }
````

## Recommended initialization choices

* Most apps: `two_face::syntax::extra_newlines()`
* Line-by-line/highlight streaming (TUI-ish): consider `extra_no_newlines()` only if you observe newline parsing artifacts or need strict line isolation.

## Binary size note

two-face embeds many syntaxes/themes; expect ~**0.6 MiB** increase, but the linker can discard unused assets in many cases.