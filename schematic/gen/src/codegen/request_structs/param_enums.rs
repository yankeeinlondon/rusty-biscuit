//! Rust enums for parameters whose values are a fixed set of strings.
//!
//! A `QueryParamType::Enum` parameter becomes
//! `pub enum {RequestType}{ParamPascalCase}`, the request type name already
//! carrying the API's resolved suffix. Each wire value becomes one variant
//! whose serde name and `as_str` spelling are that wire value, so a request
//! serializer sends exactly what the definition declared:
//!
//! ```text
//! #[non_exhaustive]
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
//! pub enum GetReportRequestFormat {
//!     #[serde(rename = "csv")]
//!     Csv,
//!     #[serde(rename = "json-lines")]
//!     JsonLines,
//! }
//! ```
//!
//! Variant identifiers are derived by [`enum_variant_name`]. Two wire values
//! can map to one identifier (`a-b` and `a_b`); validation reports that, and
//! the emitters never rename to resolve it.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::openapi::import::naming::to_pascal_case;

/// The name of the enum generated for a parameter whose Rust field is
/// `field_name` on the request type `request_type`.
pub(crate) fn param_enum_name(request_type: &str, field_name: &str) -> String {
    format!("{request_type}{}", to_pascal_case(field_name))
}

/// The variant identifier of an enum wire value.
///
/// The value is PascalCased with `_ - space / . :` as word breaks, then every
/// character outside `[A-Za-z0-9]` is dropped. An empty result is `Empty`, a
/// leading digit is prefixed with `V`, and a keyword (`Self`) gains a
/// trailing `_`. Distinct values may share a result; see
/// [`variant_collisions`].
pub(crate) fn enum_variant_name(wire_value: &str) -> String {
    let mut name: String = to_pascal_case(wire_value)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if name.is_empty() {
        name = "Empty".to_string();
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, 'V');
    }
    if syn::parse_str::<syn::Ident>(&name).is_err() {
        name.push('_');
    }
    name
}

/// Every variant identifier claimed by more than one wire value, with those
/// values in declaration order.
pub(crate) fn variant_collisions(wire_values: &[String]) -> Vec<(String, Vec<&str>)> {
    let mut claimed: Vec<(String, Vec<&str>)> = Vec::new();
    for value in wire_values {
        let variant = enum_variant_name(value);
        match claimed.iter_mut().find(|(name, _)| *name == variant) {
            Some((_, values)) => values.push(value),
            None => claimed.push((variant, vec![value])),
        }
    }
    claimed.retain(|(_, values)| values.len() > 1);
    claimed
}

/// Generates the enum `enum_name` with one variant per wire value, its
/// `as_str` wire accessor, and a `Display` impl that writes that spelling.
///
/// `doc` is the enum's doc text (one line, without the leading space).
/// Callers validate first: colliding variant identifiers would emit a
/// duplicate variant rather than be renamed.
pub(crate) fn generate_param_enum(enum_name: &str, doc: &str, wire_values: &[String]) -> TokenStream {
    let name = format_ident!("{}", enum_name);
    let doc = format!(" {doc}");
    let variants: Vec<_> = wire_values
        .iter()
        .map(|value| format_ident!("{}", enum_variant_name(value)))
        .collect();
    let variant_docs = wire_values.iter().map(|value| format!(" `{value}`"));

    quote! {
        #[doc = #doc]
        #[non_exhaustive]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum #name {
            #(
                #[doc = #variant_docs]
                #[serde(rename = #wire_values)]
                #variants,
            )*
        }

        impl #name {
            /// The value as it is sent on the wire.
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(Self::#variants => #wire_values,)*
                }
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::format_generated_code;

    #[test]
    fn variant_names_are_pascal_case_identifiers() {
        let cases = [
            ("asc", "Asc"),
            ("created_at", "CreatedAt"),
            ("json-lines", "JsonLines"),
            ("api_key.created", "ApiKeyCreated"),
            ("ASC", "Asc"),
            ("2fa", "V2fa"),
            ("a+b", "Ab"),
            ("", "Empty"),
            ("self", "Self_"),
        ];
        for (wire, expected) in cases {
            assert_eq!(enum_variant_name(wire), expected, "variant for `{wire}`");
        }
    }

    #[test]
    fn variant_collisions_name_every_value_that_claims_an_identifier() {
        let values = ["a-b", "c", "a_b", "AB"].map(String::from);

        let collisions = variant_collisions(&values);

        assert_eq!(collisions, vec![("AB".to_string(), vec!["a-b", "a_b"])]);
    }

    #[test]
    fn enum_name_is_the_request_type_then_the_pascal_cased_field() {
        assert_eq!(param_enum_name("GetReportParams", "sort_order"), "GetReportParamsSortOrder");
    }

    #[test]
    fn generated_enum_is_non_exhaustive_with_wire_renames_accessor_and_display() {
        let values = ["csv", "json-lines"].map(String::from);

        let code = format_generated_code(&generate_param_enum("ReportFormat", "Formats.", &values))
            .expect("enum parses");

        assert!(code.contains("#[non_exhaustive]"), "{code}");
        assert!(
            code.contains(
                "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]"
            ),
            "{code}"
        );
        assert!(code.contains("#[serde(rename = \"csv\")]\n    Csv,"), "{code}");
        assert!(code.contains("#[serde(rename = \"json-lines\")]\n    JsonLines,"), "{code}");
        assert!(code.contains("pub fn as_str(&self) -> &'static str"), "{code}");
        assert!(code.contains("Self::JsonLines => \"json-lines\""), "{code}");
        assert!(code.contains("impl ::core::fmt::Display for ReportFormat"), "{code}");
        assert!(code.contains("f.write_str(self.as_str())"), "{code}");
        assert!(!code.contains("#[deprecated"), "{code}");
    }
}
