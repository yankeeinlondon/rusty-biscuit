# `darkmatter::catalog` — Shared descriptor framework

This directory contains the shared descriptor framework used by all of
Darkmatter's runtime-accessible metadata catalogs: the static tables compiled
into the crate and the expression-function catalogs loaded at runtime (the
built-in catalog and a host's function catalog).

## What `Described` provides

The `Described` trait gives every catalog a uniform shape:

- `key()` — canonical lookup key (variable name, function signature, etc.).
- `description()` — short human-readable description.
- `category()` — logical grouping for reports.
- `order()` — stable display order within the category.
- `example()` — optional verified example with `invocation` and `result`,
  returned by value as an `Example<'_>` view borrowed from the descriptor.

Every string the trait returns borrows from the descriptor (`&str`, not
`&'static str`). A caller that needs a permanent borrow takes it from a
concrete static catalog, such as `expression_function_descriptors()`.

`Example<'a>` and `ExampleVerification<'a>` are `Copy` views; the
`DisplayOnly` reason borrows for the same lifetime as the invocation and
result. A static table stores `Example<'static>`; a loaded catalog owns its
example text and lends a view of it. That is why a host's function catalog can
be freed: nothing it hands out outlives it.

Utility functions for lookup, suggestion, and error text:

- `describe(catalog, key)` — exact lookup by key.
- `suggest(catalog, key, max)` — fuzzy nearest-match suggestions, with
  `ctx.`-prefix stripping and parenthesis stripping so typos like `uper`,
  `ctx.toady`, or `uper(x)` all find their intended match.
- `suggest_strings(candidates, key, max)` — the same distance gate over plain
  strings (such as sibling file names) rather than `Described` records;
  case-insensitive, with ties broken alphabetically.
- `describe_for_error(descriptor)` — plain-text formatter that emits key,
  description, and example; used to enrich evaluator errors and warnings.

## When to implement `Described`

Add `impl Described` when you are creating a new static catalog of things that
a user-facing report or error message needs to talk about. Existing examples:

- `ContextVariableDescriptor` — variables exposed as `ctx.*`.
- `ExpressionFunctionDescriptor` — functions available in `{{ ... }}` and
  `when="..."` expressions. It owns its data and derives `example()` from its
  first inline call example; it has no stored `example` field.
- `OperatorDescriptor`, `TruthinessDescriptor`, etc. — expression-language
  semantics in `expression/semantics.rs`.
- `EffectDescriptor` — mutating capabilities of `EffectEngine`.

## How to add a new catalog

1. Define your descriptor struct in its own module. Include `example:
   Option<Example<'static>>` whenever the items can be invoked or evaluated,
   and return it from `example()` as is.
2. `impl Described for YourDescriptor`.
3. Export a `pub const YOUR_DESCRIPTORS: &[YourDescriptor] = &[...]`.
4. Add a thin accessor: `pub fn your_descriptors() -> &'static [YourDescriptor]`.
5. Add unit tests for exact lookup (`describe`), fuzzy suggestion (`suggest`),
   and error formatting (`describe_for_error`).
6. If the descriptors carry examples, add an end-to-end test that runs each
   example through the real runtime and asserts the output matches the declared
   `result`.
7. Re-export the catalog from the appropriate public module so the CLI and
   other consumers can reach it without depending on the catalog module
   directly.

## In-crate Levenshtein distance

`catalog/mod.rs` implements Levenshtein distance directly (~30 lines) so the
suggestion machinery has no external dependency. Keep it that way: do not add a
crate just for string distance.

## Error-path text stays plain

`describe_for_error` emits plain text. Any terminal styling, color, or markup is
the responsibility of the caller (e.g. `claudine-cli`). This keeps the library
usable by non-terminal consumers.
