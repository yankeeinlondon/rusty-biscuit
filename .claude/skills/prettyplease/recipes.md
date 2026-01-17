# Recipes (build scripts, proc macros, tools)

## 1) build.rs: write formatted generated code to $OUT_DIR

Pattern: generate tokens → parse to `syn::File` → unparse → write to disk.

````rust
// build.rs
use std::{env, fs, path::PathBuf};
use quote::quote;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("generated.rs");

    let tokens = quote! {
        pub fn generated() -> &'static str { "hello" }
    };

    let file: syn::File = syn::parse2(tokens).unwrap();
    let formatted = prettyplease::unparse(&file);

    fs::write(out_path, formatted).unwrap();
}
````

Tip: If your generator produces multiple items, put them all into one `syn::File`.

## 2) Procedural macro: format generated output (usually for debugging)

### A) Debug-print the expansion (recommended)

This keeps your macro output as tokens but prints a readable version.

````rust
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn my_macro(_input: TokenStream) -> TokenStream {
    let output = quote! {
        fn hello() { println!("hi"); }
    };

    // Debug only: parse + pretty-print
    if std::env::var_os("MY_MACRO_DEBUG").is_some() {
        if let Ok(file) = syn::parse2::<syn::File>(output.clone()) {
            eprintln!("{}", prettyplease::unparse(&file));
        }
    }

    output.into()
}
````

### B) Emit formatted code (less common)

If you *must* emit formatted code, you’ll roundtrip through a string parse:

````rust
use proc_macro::TokenStream;

pub fn emit_formatted(tokens: proc_macro2::TokenStream) -> TokenStream {
    let file = syn::parse2::<syn::File>(tokens).unwrap();
    let s = prettyplease::unparse(&file);
    s.parse().unwrap()
}
````

Caveat: string roundtrip may lose spans and affect diagnostics.

## 3) AST transform tool: parse → modify → reformat

Useful for refactoring tools and code generators that operate on `syn`:

````rust
fn add_attribute_to_all_items(src: &str) -> syn::Result<String> {
    let mut file = syn::parse_file(src)?;

    // mutate file.items here ...

    Ok(prettyplease::unparse(&file))
}
````

## 4) Formatting expanded macro code (cargo-expand style)

Pipeline: run tool → parse as `syn::File` → `unparse`.

* If the expanded output isn’t valid as a full `syn::File`, you may need to adjust (e.g., wrap items, remove leading non-Rust text).

## 5) Formatting a list of items produced programmatically

If you’re building `syn::Item`s directly:

````rust
fn format_items(items: Vec<syn::Item>) -> String {
    let file = syn::File { shebang: None, attrs: vec![], items };
    prettyplease::unparse(&file)
}
````