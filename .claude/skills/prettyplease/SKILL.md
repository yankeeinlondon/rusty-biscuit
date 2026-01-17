---
name: prettyplease
description: Expert knowledge for formatting generated Rust code by pretty-printing `syn` syntax trees into readable source using `prettyplease::unparse`. Use when building proc-macros, code generators, AST transforms, or tools like expand/refactor where rustfmt is unavailable, too heavy, or may bail out.
tools: [Read, Write, Edit, Grep, Glob, Bash]
---

# prettyplease

prettyplease is a minimal, fast Rust **AST pretty-printer**: it turns a `syn::File` into well-formatted Rust source via `prettyplease::unparse(&file)`. It’s ideal for **generated code** (proc macros, build scripts, bindings, schema codegen) where you need deterministic “good enough” formatting and **never want the formatter to bail out**.

## When to use this skill
Use this skill when you need to:
- Format code generated from `quote!` / `proc_macro2::TokenStream`
- Reformat a `syn` AST after transformations (refactor tools, expand tools)
- Write readable Rust to `$OUT_DIR` in `build.rs`
- Avoid depending on an external `rustfmt` binary or toolchain variance

Avoid when you need:
- Exact rustfmt output or project-specific style config
- Reliable preservation of non-doc comments (AST-based formatting loses/changes comment placement)

## Quick API (the whole point)
- `prettyplease::unparse(file: &syn::File) -> String`

## Key constraints / gotchas
- `unparse` only accepts **`syn::File`** (wrap single items yourself)
- **No configuration knobs** (hard-coded width/indent; opinionated)
- Output is close to rustfmt but not identical (expect small diffs)
- Shebang must be supplied via `syn::File { shebang: Some(...) }` if needed

## Common workflows
- Parse string → format:
  - `syn::parse_file` / `syn::parse_str::<syn::File>` → `prettyplease::unparse`
- Tokens → format:
  - `quote!{...}` → `syn::parse2::<syn::File>(tokens)` → `unparse`
- Single item → format:
  - parse `syn::Item` then wrap into `syn::File { items: vec![item], .. }`

## Implementation playbook
1. Decide your input form (string vs tokens vs `syn` nodes).
2. Ensure you can produce a `syn::File` (wrap if necessary).
3. Call `prettyplease::unparse(&file)`.
4. Write output to disk / return from macro / print for debugging.

## Detailed guides
- [Setup & core usage](./setup-and-core-usage.md)
- [Recipes (build.rs, proc macros, expand tools)](./recipes.md)
- [Gotchas & troubleshooting](./gotchas-and-troubleshooting.md)
- [Choosing prettyplease vs alternatives](./alternatives-and-when-not-to-use.md)