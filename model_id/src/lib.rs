use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

/// Derive `ModelId` for enums.
///
/// Encoding rules:
/// - Aggregator delimiter: `___`
///     Provider___ModelEncoded  ->  "provider/model"
/// - Model encoding:
///     `__` -> '-'
///     `_`  -> '.'
/// - Primary (no `___`):
///     ModelEncoded -> "model"
/// - Variant override:
///     #[model_id("...")]
/// - Safety hatch:
///     Bespoke(String) -> s.as_str()
#[proc_macro_derive(ModelId, attributes(model_id))]
pub fn derive_model_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = input.ident;

    let data_enum = match input.data {
        Data::Enum(e) => e,
        _ => {
            return syn::Error::new_spanned(enum_ident, "ModelId can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    let mut arms = Vec::new();

    for v in data_enum.variants {
        let v_ident = v.ident;

        // Variant override: #[model_id("...")]
        let mut override_id: Option<String> = None;
        for attr in &v.attrs {
            if attr.path().is_ident("model_id") {
                let lit: LitStr = match attr.parse_args() {
                    Ok(l) => l,
                    Err(e) => return e.to_compile_error().into(),
                };
                override_id = Some(lit.value());
            }
        }

        // Detect tuple variant with single String field (e.g. Bespoke(String))
        let is_single_string_field = matches!(
            &v.fields,
            Fields::Unnamed(u)
                if u.unnamed.len() == 1
                    && u.unnamed[0].ty.to_token_stream().to_string() == "String"
        );

        if is_single_string_field {
            arms.push(quote! { Self::#v_ident(s) => s.as_str() });
            continue;
        }

        if let Some(id) = override_id {
            arms.push(quote! { Self::#v_ident => #id });
            continue;
        }

        let raw = v_ident.to_string();

        let canonical: String = if let Some((provider_raw, model_raw)) = split_once(&raw, "___") {
            // Aggregator: provider/model
            let provider = normalize_provider(provider_raw);
            let model = decode_model(model_raw);
            format!("{provider}/{model}")
        } else {
            // Primary: model only
            decode_model(&raw)
        };

        arms.push(quote! { Self::#v_ident => #canonical });
    }

    let expanded = quote! {
        impl #enum_ident {
            /// Canonical model id to send over the wire.
            ///
            /// - Unit variants map to a static string literal.
            /// - `Bespoke(String)` returns a borrow of the stored string.
            pub fn model_id(&self) -> &str {
                match self {
                    #(#arms,)*
                }
            }
        }
    };

    expanded.into()
}

/// Split only on the first occurrence of `delim`.
fn split_once<'a>(s: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    let idx = s.find(delim)?;
    let (a, b) = s.split_at(idx);
    Some((a, &b[delim.len()..]))
}

/// Provider normalization:
/// - lowercase
/// - `_` -> '-'  (if you ever need it)
fn normalize_provider(s: &str) -> String {
    s.trim_matches('_')
        .replace('_', "-")
        .to_ascii_lowercase()
}

/// Decode the model name according to:
/// - `__` -> '-'
/// - `_`  -> '.'
/// plus lowercase.
fn decode_model(s: &str) -> String {
    let s = s.trim_matches('_');
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'_' {
            // If next is also '_' => "__" -> '-'
            if i + 1 < bytes.len() && bytes[i + 1] == b'_' {
                out.push('-');
                i += 2;
            } else {
                // "_" -> '.'
                out.push('.');
                i += 1;
            }
        } else {
            // ASCII-fast path. (If you care about non-ASCII identifiers, this can be upgraded.)
            out.push((bytes[i] as char).to_ascii_lowercase());
            i += 1;
        }
    }

    // Collapse any accidental repeats (defensive)
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    while out.contains("..") {
        out = out.replace("..", ".");
    }

    out
}


