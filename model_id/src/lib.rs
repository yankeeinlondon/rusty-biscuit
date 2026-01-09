use proc_macro::TokenStream;
use quote::quote;
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
///
/// ## Generated Code
///
/// This macro generates:
/// - `model_id(&self) -> &str` - Returns the wire-format model ID
/// - `FromStr` implementation - Parses wire IDs back to variants
/// - `ALL: &'static [Self]` - Array of all unit variants (excludes Bespoke)
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

    let mut model_id_arms = Vec::new();
    let mut from_str_arms = Vec::new();
    let mut all_variants = Vec::new();

    for v in &data_enum.variants {
        let v_ident = &v.ident;
        let v_ident_str = v_ident.to_string();

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

        // Detect Bespoke variant by name (more robust than type inspection)
        let is_bespoke = v_ident_str == "Bespoke"
            && matches!(&v.fields, Fields::Unnamed(u) if u.unnamed.len() == 1);

        if is_bespoke {
            model_id_arms.push(quote! { Self::#v_ident(s) => s.as_str() });
            // Bespoke is handled as fallback in FromStr, not as a match arm
            continue;
        }

        // Compute the canonical wire ID
        let canonical: String = if let Some(ref id) = override_id {
            id.clone()
        } else if let Some((provider_raw, model_raw)) = v_ident_str.split_once("___") {
            // Aggregator: provider/model
            let provider = normalize_provider(provider_raw);
            let model = decode_model(model_raw);
            format!("{provider}/{model}")
        } else {
            // Primary: model only
            decode_model(&v_ident_str)
        };

        model_id_arms.push(quote! { Self::#v_ident => #canonical });
        from_str_arms.push(quote! { #canonical => Ok(Self::#v_ident) });
        all_variants.push(quote! { Self::#v_ident });
    }

    // Check if there's a Bespoke variant for the fallback
    let has_bespoke = data_enum
        .variants
        .iter()
        .any(|v| v.ident == "Bespoke" && matches!(&v.fields, Fields::Unnamed(u) if u.unnamed.len() == 1));

    let from_str_fallback = if has_bespoke {
        quote! { _ => Ok(Self::Bespoke(s.to_string())) }
    } else {
        quote! {
            _ => Err(UnknownModelIdError {
                model_id: s.to_string(),
                enum_name: stringify!(#enum_ident).to_string(),
            })
        }
    };

    let expanded = quote! {
        /// Error returned when parsing an unknown model ID.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct UnknownModelIdError {
            /// The unrecognized model ID string.
            pub model_id: String,
            /// The enum type that failed to parse.
            pub enum_name: String,
        }

        impl std::fmt::Display for UnknownModelIdError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "unknown model ID '{}' for {}", self.model_id, self.enum_name)
            }
        }

        impl std::error::Error for UnknownModelIdError {}

        impl #enum_ident {
            /// All known unit variants (excludes `Bespoke`).
            ///
            /// Useful for discovery, iteration, and validation.
            pub const ALL: &'static [Self] = &[
                #(#all_variants,)*
            ];

            /// Canonical model id to send over the wire.
            ///
            /// - Unit variants map to a static string literal.
            /// - `Bespoke(String)` returns a borrow of the stored string.
            #[must_use]
            pub fn model_id(&self) -> &str {
                match self {
                    #(#model_id_arms,)*
                }
            }
        }

        impl std::str::FromStr for #enum_ident {
            type Err = UnknownModelIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms,)*
                    #from_str_fallback
                }
            }
        }
    };

    expanded.into()
}

/// Normalize provider name to lowercase with hyphens.
///
/// - Trims leading/trailing underscores
/// - Converts underscores to hyphens
/// - Lowercases all characters
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
            // ASCII-fast path
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
