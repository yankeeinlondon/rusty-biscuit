# Gotchas & Troubleshooting

## 1) “Type mismatch: expected &syn::File”

Cause: you tried to pass `syn::ItemFn`, `syn::Item`, etc. to `unparse`.

Fix: wrap into a `syn::File`:

````rust
let item: syn::Item = syn::parse_str("fn f(){}")?;
let file = syn::File { shebang: None, attrs: vec![], items: vec![item] };
let s = prettyplease::unparse(&file);
````

## 2) “My tokens won’t parse as syn::File”

Cause: `syn::File` expects a sequence of **items** (and possibly attributes), not arbitrary expressions.

Fix options:

* If you have a single item, parse as `syn::Item` and wrap.
* If you have an expression, wrap it into an item:
  * e.g., generate `fn _tmp(){ <expr>; }` then parse.

## 3) “Formatting differs from rustfmt”

This is expected: prettyplease aims for ~95–98% of rustfmt’s output quality, not exact matching.

Action:

* Accept minor diffs for generated code.
* If exact rustfmt output is a hard requirement, use rustfmt (often via subprocess).

## 4) “Where are my comments?”

Because prettyplease prints from the `syn` AST, and `syn` does not preserve most non-doc comments in the parsed tree, comment fidelity can be lost.

Workarounds:

* For tools that must preserve comments, consider text-based formatting approaches or libraries designed for lossless parsing (outside prettyplease’s scope).
* If comments are only for debugging, accept the loss.

## 5) No configuration knobs

No `max_width`, no custom indentation, no style profile.

If you need configurability:

* Use `rustfmt` (configurable but heavier), or
* Consider a wrapper like `rust-format` (choose backend / fallback strategy), or
* Consider `prettier-please` if you need custom printers for DSL-like constructs.

## 6) Shebang handling

If your output must include a shebang line, it must be present in `syn::File.shebang`. It won’t be inferred from the input unless you parse it and set it yourself.