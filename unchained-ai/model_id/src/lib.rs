use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

/// Derive `ModelId` for enums.
///
/// Encoding rules:
/// - Aggregator delimiter: `___`
///   Provider___ModelEncoded  ->  "provider/model"
/// - Model encoding:
///   `__` -> '-'
///   `_`  -> '.'
/// - Primary (no `___`):
///   ModelEncoded -> "model"
/// - Variant override:
///   #[model_id("...")]
/// - Safety hatch:
///   Bespoke(String) -> s.as_str()
///
/// ## Generated Code
///
/// This macro generates:
/// - `model_id(&self) -> &str` - Returns the wire-format model ID
/// - `FromStr` implementation - Parses wire IDs back to variants
/// - `ALL: &'static [Self]` - Array of all unit variants (excludes Bespoke)
///
/// ## Optional Metadata Lookup
///
/// Add a `#[model_id_metadata(...)]` attribute to generate a `metadata()` method:
///
/// ```ignore
/// #[derive(ModelId)]
/// #[model_id_metadata(
///     lookup = "super::metadata_generated::MODEL_METADATA",
///     returns = "crate::models::model_metadata::ModelMetadata"
/// )]
/// pub enum ProviderModelOpenAi { ... }
/// ```
///
/// This generates:
/// ```ignore
/// pub fn metadata(&self) -> Option<&'static ModelMetadata> {
///     MODEL_METADATA.get(self.model_id())
/// }
/// ```
#[proc_macro_derive(ModelId, attributes(model_id, model_id_metadata))]
pub fn derive_model_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// The derive's whole expansion, kept outside the `proc_macro` boundary so
/// unit tests can assert its compile errors (message and span) in-process.
fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = input.ident.clone();

    // Parse optional metadata lookup configuration from #[model_id_metadata(...)]
    let metadata_config = parse_metadata_config(&input);

    let data_enum = match input.data {
        Data::Enum(e) => e,
        _ => {
            return Err(syn::Error::new_spanned(
                enum_ident,
                "ModelId can only be derived for enums",
            ));
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
                let lit: LitStr = attr.parse_args()?;
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
    let has_bespoke = data_enum.variants.iter().any(|v| {
        v.ident == "Bespoke" && matches!(&v.fields, Fields::Unnamed(u) if u.unnamed.len() == 1)
    });

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

    // Generate optional metadata method if configured
    let metadata_method = if let Some(config) = metadata_config {
        let lookup_path: syn::Path =
            syn::parse_str(&config.lookup_path).expect("invalid lookup path in model_id_metadata");
        let returns_path: syn::Path = syn::parse_str(&config.returns_type)
            .expect("invalid returns type in model_id_metadata");

        quote! {
            /// Returns metadata for this model if available.
            ///
            /// Metadata includes context window, modalities, and capabilities.
            #[must_use]
            pub fn metadata(&self) -> Option<&'static #returns_path> {
                #lookup_path.get(self.model_id())
            }
        }
    } else {
        quote! {}
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

            #metadata_method
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

    Ok(expanded)
}

/// Normalize provider name to lowercase with hyphens.
///
/// - Trims leading/trailing underscores
/// - Converts underscores to hyphens
/// - Lowercases all characters
fn normalize_provider(s: &str) -> String {
    decode_model(s)
}

/// Decode the model name according to:
/// - `__` -> '-'
/// - `_`  -> '.'
///
/// Plus lowercase.
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

/// Configuration for optional metadata lookup generation.
struct MetadataConfig {
    /// Path to the lookup table (e.g., "super::metadata_generated::MODEL_METADATA")
    lookup_path: String,
    /// Return type path (e.g., "crate::models::model_metadata::ModelMetadata")
    returns_type: String,
}

/// Parses the `#[model_id_metadata(lookup = "...", returns = "...")]` attribute.
fn parse_metadata_config(input: &DeriveInput) -> Option<MetadataConfig> {
    for attr in &input.attrs {
        if !attr.path().is_ident("model_id_metadata") {
            continue;
        }

        let mut lookup_path: Option<String> = None;
        let mut returns_type: Option<String> = None;

        let result = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("lookup") {
                let value: LitStr = meta.value()?.parse()?;
                lookup_path = Some(value.value());
                Ok(())
            } else if meta.path.is_ident("returns") {
                let value: LitStr = meta.value()?.parse()?;
                returns_type = Some(value.value());
                Ok(())
            } else {
                Err(meta.error("expected `lookup` or `returns`"))
            }
        });

        if let Err(e) = result {
            panic!("failed to parse model_id_metadata: {}", e);
        }

        let lookup_path = lookup_path.expect("model_id_metadata requires `lookup = \"...\"`");
        let returns_type = returns_type.expect("model_id_metadata requires `returns = \"...\"`");

        return Some(MetadataConfig {
            lookup_path,
            returns_type,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    #[test]
    fn test_normalize_provider_decodes_hyphen_encoded_namespaces() {
        assert_eq!(normalize_provider("X__Ai"), "x-ai");
        assert_eq!(normalize_provider("Z__Ai"), "z-ai");
        assert_eq!(normalize_provider("Meta__Llama"), "meta-llama");
        assert_eq!(normalize_provider("Openai"), "openai");
    }

    // Each `tests/fail/*.rs` case runs the derive's expansion in-process and
    // renders its error the way rustc prints a single-line diagnostic, then
    // compares that with the case's `.stderr` file. Nothing here starts Cargo,
    // so a cold archive consumer pays no nested dependency build.
    const COMPILE_FAIL_CASES: &[&str] = &["bad_attribute", "not_enum"];

    #[test]
    fn bad_attribute() {
        assert_diagnostic(
            "tests/fail/bad_attribute.rs",
            include_str!("../tests/fail/bad_attribute.rs"),
            include_str!("../tests/fail/bad_attribute.stderr"),
        );
    }

    #[test]
    fn not_enum() {
        assert_diagnostic(
            "tests/fail/not_enum.rs",
            include_str!("../tests/fail/not_enum.rs"),
            include_str!("../tests/fail/not_enum.stderr"),
        );
    }

    #[test]
    fn every_compile_fail_fixture_has_a_test_and_an_expected_diagnostic() {
        let dir = biscuit_test_harness::manifest_dir!().join("tests/fail");
        let mut cases = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("tests/fail is readable") {
            let path = entry.expect("a tests/fail entry").path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                assert!(
                    path.with_extension("stderr").is_file(),
                    "{} has no expected `.stderr` diagnostic",
                    path.display()
                );
                let stem = path.file_stem().expect("a fixture file name");
                cases.push(stem.to_string_lossy().into_owned());
            }
        }
        cases.sort();
        assert_eq!(
            cases, COMPILE_FAIL_CASES,
            "every tests/fail fixture needs its own #[test] in src/lib.rs and an entry in COMPILE_FAIL_CASES"
        );
    }

    /// Expands the fixture's `#[derive(ModelId)]` item and asserts the
    /// rendered error equals `expected` (a trybuild-style `.stderr`).
    fn assert_diagnostic(path: &str, source: &str, expected: &str) {
        let file = syn::parse_file(source).expect("the fixture is valid Rust syntax");
        let item = file
            .items
            .iter()
            .find(|item| {
                let attrs = match item {
                    syn::Item::Enum(item) => &item.attrs,
                    syn::Item::Struct(item) => &item.attrs,
                    syn::Item::Union(item) => &item.attrs,
                    _ => return false,
                };
                attrs.iter().any(|attr| {
                    attr.path().is_ident("derive")
                        && attr.meta.to_token_stream().to_string().contains("ModelId")
                })
            })
            .expect("the fixture derives ModelId");
        let input: DeriveInput =
            syn::parse2(item.to_token_stream()).expect("the derive input parses");
        let error = expand(input).expect_err("the fixture must fail to expand");
        let rendered = render(path, source, error);
        assert_eq!(
            rendered.trim_end(),
            expected.replace("\r\n", "\n").trim_end(),
            "{path}: the derive's diagnostic differs from its .stderr file"
        );
    }

    /// Renders each error as rustc prints a single-line-span diagnostic.
    fn render(path: &str, source: &str, error: syn::Error) -> String {
        error
            .into_iter()
            .map(|error| {
                let span = error.span();
                let (start, end) = (span.start(), span.end());
                assert_eq!(
                    start.line, end.line,
                    "{path}: only single-line spans are rendered; extend `render` for this case"
                );
                let text = source
                    .lines()
                    .nth(start.line - 1)
                    .expect("the span's line is in the fixture");
                let gutter = " ".repeat(start.line.to_string().len());
                format!(
                    "error: {message}\n{gutter}--> {path}:{line}:{column}\n{gutter} |\n{line} | {text}\n{gutter} | {indent}{carets}",
                    message = error,
                    line = start.line,
                    column = start.column + 1,
                    indent = " ".repeat(start.column),
                    carets = "^".repeat(end.column - start.column),
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
