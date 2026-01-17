# prettyplease — Consolidated Deep Dive (Definitive Reference)

## Table of Contents

1. [What prettyplease Is](#what-prettyplease-is)
1. [Why It Exists (Motivation)](#why-it-exists-motivation)
1. [Where It Fits in the Rust Formatting Landscape](#where-it-fits-in-the-rust-formatting-landscape)
1. [Installation & Compatibility](#installation--compatibility)
1. [Core API and Data Model](#core-api-and-data-model)
1. [Formatting Pipeline Patterns (From Simple to Advanced)](#formatting-pipeline-patterns-from-simple-to-advanced)
   1. [Format a Full Source File String](#1-format-a-full-source-file-string)
   1. [Format Generated Tokens (quote! → syn → prettyplease)](#2-format-generated-tokens-quote--syn--prettyplease)
   1. [Format a Single Item (Wrapping in syn::File)](#3-format-a-single-item-wrapping-in-synfile)
   1. [Use in Procedural Macros](#4-use-in-procedural-macros)
   1. [Use in build.rs Code Generation](#5-use-in-buildrs-code-generation)
   1. [Formatting Macro-Expanded Output (cargo-expand style)](#6-formatting-macro-expanded-output-cargo-expand-style)
   1. [AST Transform Tools (Read → Modify → Reprint)](#7-ast-transform-tools-read--modify--reprint)
1. [Algorithm & Design Notes](#algorithm--design-notes)
1. [Gotchas, Limitations, and Behavior Differences vs rustfmt](#gotchas-limitations-and-behavior-differences-vs-rustfmt)
1. [Ecosystem & Integration Partners (dtolnay ecosystem)](#ecosystem--integration-partners-dtolnay-ecosystem)
1. [When to Use / When Not to Use](#when-to-use--when-not-to-use)
1. [Similar and Adjacent Tools](#similar-and-adjacent-tools)
1. [Versioning & Changelog Notes (0.x SemVer)](#versioning--changelog-notes-0x-semver)
1. [Licensing](#licensing)

---

## What prettyplease Is

**prettyplease** is a minimalist **Rust syntax tree pretty-printer**: it converts a `syn` syntax tree—specifically a `syn::File`—into a well-formatted Rust source string.

It is designed primarily for **generated code** (proc macros, build scripts, binding generators, expand/debug tools), where you want readable output without depending on an external `rustfmt` binary and without risking “formatter bailout.”

Key characteristics:

* **Library-first** (embed in your crate; no shelling out required)
* **Opinionated and minimal** (effectively one public entry point for typical use)
* **Deterministic “good enough” formatting** (targets ~95–98% of rustfmt quality)
* **Reliability over perfection**: it **never gives up** and always produces output

---

## Why It Exists (Motivation)

`rustfmt` is the standard formatter for **human-written code**, but generated code has a different failure mode: if the formatter encounters deeply nested or complex structures, it may **bail out** and leave segments poorly formatted (or collapsed into unreadable single lines).

For human-authored code, a bailout can be mitigated by refactoring. For generated code, you typically **cannot** refactor the structure.

prettyplease is built to address that: it is optimized for generated output and is engineered to always produce *some* reasonable formatting, even under pathological nesting/complexity.

---

## Where It Fits in the Rust Formatting Landscape

Conceptually:

* **rustfmt**: best aesthetics + configurability, but heavyweight and may fail/bail on complex generated code; also harder to use as a stable library API.
* **prettyplease**: fast, stable-to-depend-on library that formats ASTs; not configurable; slightly different style than rustfmt; focuses on never bailing.
* **rustc_ast_pretty**: compiler-ish output (useful for debugging), but can be non-standard and pathological in edge cases.
* **quote::ToTokens / quote!**: generates tokens quickly with no formatting guarantees; output is often unreadable.

---

## Installation & Compatibility

### Recommended (cargo add)

````bash
cargo add prettyplease
````

### Manual `Cargo.toml`

````toml
[dependencies]
prettyplease = "0.2"
````

### Typical companion dependencies

prettyplease prints `syn` AST types; you typically also depend on `syn` and often `quote`:

````toml
[dependencies]
prettyplease = "0.2"
syn = { version = "2", default-features = false, features = ["full", "parsing"] }
quote = "1"          # common in codegen/proc-macros
proc-macro2 = "1"    # common in codegen/proc-macros
````

### Minimum Rust version

* Requires **Rust 1.62+**

---

## Core API and Data Model

### Primary entry point

````rust
pub fn unparse(file: &syn::File) -> String
````

**Important implication**: prettyplease is an **AST printer**, not a string formatter. It expects a `syn::File`—a full Rust source file model.

### What it can format

In practice it supports “virtually all syntax” `syn` can represent, including:

* Items: `fn`, `struct`, `enum`, `trait`, `impl`, `mod`, etc.
* Expressions and statements
* Patterns / matches
* Attributes and macros
* `use` trees
* Generics and `where` clauses

(When new Rust syntax lands, support is generally driven by `syn` evolution and prettyplease’s printer updates.)

---

## Formatting Pipeline Patterns (From Simple to Advanced)

### 1) Format a Full Source File String

Parse a Rust file (string) into `syn::File`, then print:

````rust
use prettyplease::unparse;
use syn::parse_file;

fn main() {
    const INPUT: &str = r#"
    use crate::{lazy::{Lazy,SyncLazy,SyncOnceCell},panic,sync::{atomic::{AtomicUsize,Ordering::SeqCst},mpsc::channel,Mutex,},thread,};
    impl<T,U> Into<U> for T where U:From<T>{fn into(self)->U{U::from(self)}}
    "#;

    let syntax_tree = parse_file(INPUT).unwrap();
    let formatted = unparse(&syntax_tree);
    print!("{}", formatted);
}
````

Example output (illustrative):

````rust
use crate::{
    lazy::{Lazy, SyncLazy, SyncOnceCell}, panic,
    sync::{atomic::{AtomicUsize, Ordering::SeqCst}, mpsc::channel, Mutex},
    thread,
};

impl<T, U> Into<U> for T
where
    U: From<T>,
{
    fn into(self) -> U { U::from(self) }
}
````

---

### 2) Format Generated Tokens (quote! → syn → prettyplease)

Common pattern in code generation:

1. Use `quote!` to produce tokens
1. Parse tokens into `syn::File`
1. Print with prettyplease

````rust
use quote::quote;

fn generate_code() -> String {
    let tokens = quote! {
        struct Generated { field: i32 }
        impl Generated { fn new() -> Self { Self { field: 0 } } }
    };

    let file: syn::File = syn::parse2(tokens).unwrap();
    prettyplease::unparse(&file)
}
````

This is the standard “make my generated code readable” pipeline.

---

### 3) Format a Single Item (Wrapping in syn::File)

**Gotcha**: `unparse` takes `syn::File`, not `ItemFn`, `ItemStruct`, etc.

Incorrect:

````rust
let item = syn::parse_str::<syn::ItemFn>("fn foo() {}")?;
let formatted = prettyplease::unparse(&item); // ❌ type mismatch
````

Correct: wrap the item in a `syn::File`:

````rust
use syn::{File, Item};

fn format_item(src: &str) -> String {
    let item = syn::parse_str::<Item>(src).unwrap();
    let file = File {
        shebang: None,
        attrs: vec![],
        items: vec![item],
    };
    prettyplease::unparse(&file)
}
````

---

### 4) Use in Procedural Macros

Two common uses:

#### A. Debugging macro output (human-readable)

````rust
#[proc_macro]
pub fn my_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let output: proc_macro2::TokenStream = /* build tokens */ todo!();

    // Debug only (consider gating behind a feature flag)
    let file = syn::parse2::<syn::File>(output.clone()).unwrap();
    eprintln!("Macro output:\n{}", prettyplease::unparse(&file));

    output.into()
}
````

#### B. Emitting formatted code (less common)

You *can* format and then parse back into tokens:

````rust
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MyDerive)]
pub fn my_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let generated = quote! {
        impl MyTrait for #input.ident {
            fn my_method(&self) {
                println!("Hello from {}!", stringify!(#input.ident));
            }
        }
    };

    let file = syn::parse2::<syn::File>(generated).unwrap();
    let formatted = prettyplease::unparse(&file);

    formatted.parse().unwrap()
}
````

**Note**: Parsing a formatted string back into tokens is valid, but it’s extra work. Many macros simply return tokens and use prettyplease only for debugging or for writing generated `.rs` files.

---

### 5) Use in build.rs Code Generation

Write formatted generated code into `$OUT_DIR` so users can inspect it (and you can debug it):

````rust
// build.rs
use quote::quote;

fn main() {
    let file: syn::File = syn::parse2(quote! {
        pub fn generated() -> &'static str { "hello" }
    }).unwrap();

    let formatted = prettyplease::unparse(&file);

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("generated.rs"), formatted).unwrap();
}
````

This avoids invoking `rustfmt` during builds (portability/performance win).

---

### 6) Formatting Macro-Expanded Output (cargo-expand style)

If you have expanded code (e.g., from `cargo expand`), you can parse and reprint:

````rust
use prettyplease::unparse;
use std::process::Command;

fn format_expanded_macro(target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["expand", target])
        .output()?;

    let expanded = String::from_utf8(output.stdout)?;
    let file = syn::parse_file(&expanded)?;
    Ok(unparse(&file))
}
````

This matches how some tooling achieves readable expansion output.

---

### 7) AST Transform Tools (Read → Modify → Reprint)

Prettyplease is often the “final step” after modifying a `syn::File`:

````rust
fn add_attribute_to_every_item(file: &mut syn::File) {
    // ... mutate AST ...
}

fn refactor_and_prettyprint(input_code: &str) -> String {
    let mut file: syn::File = syn::parse_str(input_code).unwrap();
    add_attribute_to_every_item(&mut file);
    prettyplease::unparse(&file)
}
````

This is a common approach for CLI refactoring tools, codegen pipelines, and “fixers.”

---

## Algorithm & Design Notes

prettyplease’s approach is inspired by Derek C. Oppen’s 1979 work on **pretty printing**.

High-level model:

* A **Scan** phase gathers sizing/break information for groups.
* A **Print** phase decides where to break lines based on target line width.
* Uses a **ring buffer** bounded by line length, keeping memory usage predictable even for deeply nested constructs.
* Supports both:
  * **Consistent breaking**: a group breaks “all-or-nothing”
  * **Inconsistent breaking**: greedy decisions where appropriate

Design goal: **simple, fast, reliable**, and avoids the “formatter gave up” outcome.

---

## Gotchas, Limitations, and Behavior Differences vs rustfmt

### 1) Input must be `syn::File`

* You must wrap items (functions, structs, impl blocks) into a file container.

### 2) No configuration knobs

* No `rustfmt.toml` equivalent.
* Uses fixed formatting decisions such as:
  * ~**100 character** line width
  * **4-space** indentation
* If you need custom styles, prettyplease is intentionally the wrong tool.

### 3) Not a 100% rustfmt match

* Expect **small stylistic differences** (often cited as ~2–3% of lines).
* Example difference:

````rust
// prettyplease might choose:
let s = Struct { x: 0, y: true };

// while rustfmt might choose:
let s = Struct {
    x: 0,
    y: true,
};
````

If exact rustfmt output is mandatory, use rustfmt.

### 4) Shebang handling is explicit

If you need a preserved shebang (e.g. `#!/usr/bin/env rustc`), set it on the `syn::File`:

````rust
let file = syn::File {
    shebang: Some("#!/usr/bin/env rustc".to_string()),
    attrs: vec![],
    items: vec![],
};
````

### 5) Comments and “non-AST trivia”

prettyplease prints from the `syn` AST, which generally does **not** preserve non-doc comments faithfully. Complex inline comments may be lost or repositioned. This is a major consideration for source-to-source refactoring tools that must preserve comments.

### 6) Feature flags / customization

* The crate is intentionally simple and **does not expose feature flags for formatting customization**.

---

## Ecosystem & Integration Partners (dtolnay ecosystem)

prettyplease is part of the “dtolnay ecosystem” of widely used foundational crates for Rust metaprogramming:

* **syn**: parses Rust code/token streams into an AST
* **quote**: generates token streams using quasi-quoting
* **proc-macro2**: stable TokenStream API usable outside proc macros
* (Adjacent ecosystem examples: **serde**, **anyhow**, **thiserror**)

Notable integrations and adopters:

* **cargo-expand**: formats expanded macro output using prettyplease
* **bindgen**: offers a `prettyplease` feature to format bindings without rustfmt
* **cxx**: generates Rust-side bridge code and formats it for readability
* **prettier-please** (DioxusLabs): a fork/wrapper direction that adds customization hooks

---

## When to Use / When Not to Use

### Use prettyplease when

* You are formatting **generated Rust code**
  * proc macros
  * build.rs codegen
  * binding generators (bindgen/cbindgen/uniffi-like workflows)
  * schema compilers (protobuf/GraphQL → Rust)
* You need formatting that is:
  * **fast**
  * **deterministic**
  * **doesn’t depend on an installed rustfmt**
  * **won’t bail** on complex nesting
* You want code that is readable for debugging and IDE navigation

### Prefer rustfmt when

* Formatting **human-written** code checked into the repo
* Team consistency depends on `rustfmt.toml` settings
* You need exact rustfmt layout and editor tooling integration
* You require incremental formatting or stable comment preservation behavior

---

## Similar and Adjacent Tools

|Tool|Best For|Tradeoffs|
|----|--------|---------|
|**prettyplease**|Generated code formatting from `syn` AST|Not configurable; minor diffs vs rustfmt; comment/trivia loss|
|**rustfmt**|Human-authored code, standard style|Heavier; harder to embed; can bail on generated code|
|**prettier-please**|Similar to prettyplease, with customization hooks|Fork; may lag upstream; less minimalist|
|**rust-format**|Abstraction layer over multiple formatters|Adds indirection; not a formatter itself|
|**quote**|Fast token generation|Output often unreadable; no formatting step|

Cross-language analogs (conceptually similar problems):

* Go: `gofmt`
* Java: JavaPoet
* JS/TS: Prettier API
* C#: Roslyn formatter

---

## Versioning & Changelog Notes (0.x SemVer)

* prettyplease is `0.x` (pre-1.0).
* In `0.x`, **minor version bumps may include breaking changes** (SemVer rule for 0.y.z).
* Much of the crate’s evolution is driven by:
  * new Rust syntax support
  * `syn` AST changes
  * formatting stability refinements

High-level timeline (approximate / not a verified per-release changelog):

* `0.1.x`: initial public releases; established the “syn AST → formatted Rust code” workflow.
* `0.2.0` and `0.2.x`: broadened syntax coverage, improved formatting quality, kept pace with `syn` and Rust language changes.

For authoritative current versions and detailed release notes, verify:

* https://crates.io/crates/prettyplease
* the repository linked from crates.io (tags/releases/CHANGELOG if present)

---

## Licensing

Dual-licensed under:

* **MIT** OR
* **Apache-2.0**

This is permissive and common across Rust metaprogramming crates, suitable for commercial and open-source use.

---

### Practical Summary

prettyplease is the go-to choice when you have **a `syn` AST** (often produced from `quote!` or parsed input) and you need to emit **readable Rust source** with a strong guarantee that formatting will complete. It intentionally trades configurability and perfect rustfmt parity for speed, simplicity, and a “never bail out” posture—exactly what generated-code pipelines need.