# Setup & Core Usage (prettyplease)

## Add dependencies

### Cargo.toml (typical)

````toml
[dependencies]
prettyplease = "0.2"
syn = { version = "2", default-features = false, features = ["full", "parsing"] }
````

Notes:

* prettyplease works on **stable** Rust and is lightweight compared to invoking rustfmt.
* Ensure `syn` features include what you parse: `full` + `parsing` are common.

## The core API

````rust
pub fn unparse(file: &syn::File) -> String
````

Prettyplease prints a whole Rust file AST. If you don’t have a `syn::File`, you must produce one.

## Format a Rust source string

````rust
use prettyplease::unparse;

fn format_source(input: &str) -> syn::Result<String> {
    let file = syn::parse_file(input)?;
    Ok(unparse(&file))
}
````

## Format tokens produced by quote!

````rust
use quote::quote;

fn format_tokens() -> String {
    let tokens = quote! {
        struct Generated { field: i32 }
        impl Generated { fn new() -> Self { Self { field: 0 } } }
    };

    let file: syn::File = syn::parse2(tokens).expect("tokens must form a file");
    prettyplease::unparse(&file)
}
````

If your tokens are an *item* (not a full file), parse as `syn::Item` and wrap (see next section).

## Format a single item (function/struct/impl)

`unparse` needs a `syn::File`, so wrap the item:

````rust
use prettyplease::unparse;
use syn::{File, Item};

fn format_item(item_src: &str) -> syn::Result<String> {
    let item: Item = syn::parse_str(item_src)?;

    let file = File {
        shebang: None,
        attrs: vec![],
        items: vec![item],
    };

    Ok(unparse(&file))
}
````

## Preserving a shebang

If you’re formatting something intended to be executable with a shebang:

````rust
use syn::File;

fn with_shebang(items: Vec<syn::Item>) -> String {
    let file = File {
        shebang: Some("#!/usr/bin/env rustc".to_string()),
        attrs: vec![],
        items,
    };
    prettyplease::unparse(&file)
}
````