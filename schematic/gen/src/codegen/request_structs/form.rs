//! Form body struct generation for multipart and URL-encoded requests.
//!
//! JSON endpoints name an existing Rust type for their body. Form endpoints
//! describe their body field-by-field instead, so the body type has to be
//! synthesized here — one struct per endpoint, named `{EndpointId}Form`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{ApiRequest, Endpoint, FormField, FormFieldKind, RestApi};

use crate::codegen::request_structs::shared::{json_type_name, param_field_name, type_name_to_tokens};

/// Returns the generated form body type name for an endpoint, if it has one.
pub(super) fn form_body_type_name(endpoint: &Endpoint) -> Option<String> {
    match &endpoint.request {
        Some(ApiRequest::FormData { .. }) | Some(ApiRequest::UrlEncoded { .. }) => {
            Some(format!("{}Form", endpoint.id))
        }
        _ => None,
    }
}

/// Returns the endpoint's form fields, if its body is a form.
pub(super) fn form_fields(endpoint: &Endpoint) -> Option<&[FormField]> {
    match &endpoint.request {
        Some(ApiRequest::FormData { fields }) | Some(ApiRequest::UrlEncoded { fields }) => {
            Some(fields)
        }
        _ => None,
    }
}

/// Reports whether the endpoint's form body is multipart rather than URL-encoded.
fn is_multipart(endpoint: &Endpoint) -> bool {
    matches!(&endpoint.request, Some(ApiRequest::FormData { .. }))
}

/// Generates the `{EndpointId}Form` struct and its body-conversion method.
///
/// Returns an empty stream for endpoints whose body is not a form.
pub(super) fn generate_form_struct(api: &RestApi, endpoint: &Endpoint) -> TokenStream {
    let (Some(type_name), Some(fields)) = (form_body_type_name(endpoint), form_fields(endpoint))
    else {
        return quote! {};
    };

    let struct_name = format_ident!("{}", type_name);
    let multipart = is_multipart(endpoint);

    let declarations = fields.iter().map(|field| declare_field(api, field, multipart));
    let parts = fields.iter().map(|field| build_part(field, multipart, &quote! { self }, false));

    let doc = format!(
        " Body for the `{}` endpoint, sent as `{}`.",
        endpoint.id,
        if multipart {
            "multipart/form-data"
        } else {
            "application/x-www-form-urlencoded"
        }
    );

    let conversion = if multipart {
        quote! {
            /// Converts the body into its multipart parts.
            pub fn into_form_parts(self) -> Vec<schematic_runtime::FormPart> {
                let mut parts = Vec::new();
                #(#parts)*
                parts
            }
        }
    } else {
        quote! {
            /// Converts the body into its URL-encoded field pairs.
            pub fn into_form_pairs(self) -> Vec<(String, String)> {
                let mut parts = Vec::new();
                #(#parts)*
                parts
            }
        }
    };

    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct #struct_name {
            #(#declarations)*
        }

        impl #struct_name {
            #conversion
        }
    }
}

/// Emits the struct field declaration for one form field.
fn declare_field(api: &RestApi, field: &FormField, multipart: bool) -> TokenStream {
    let name = format_ident!("{}", param_field_name(&field.name));
    let wire_name = &field.name;
    let rename = if param_field_name(&field.name) == field.name {
        quote! {}
    } else {
        quote! { #[serde(rename = #wire_name)] }
    };

    let doc = match &field.description {
        Some(description) => {
            let text = format!(" {}", description.trim());
            quote! { #[doc = #text] }
        }
        None => quote! {},
    };
    let rules = field.constraints.as_deref().map(crate::codegen::value_rules::rule_doc_lines).unwrap_or_default();
    let doc = match (doc.is_empty(), rules.is_empty()) {
        (_, true) => doc,
        (true, false) => quote! { #(#[doc = #rules])* },
        (false, false) => quote! { #doc #[doc = ""] #(#[doc = #rules])* },
    };

    let base = field_base_type(api, field, multipart);

    // Optional fields are `Option<T>` so `Default` can construct the struct and
    // omitted parts are skipped rather than sent empty.
    let ty = if field.required {
        base
    } else {
        quote! { Option<#base> }
    };

    quote! {
        #doc
        #rename
        pub #name: #ty,
    }
}

/// Returns the Rust type carrying one form field's value.
///
/// URL-encoded forms cannot carry files, so every field there is a string
/// regardless of how the definition described it.
fn field_base_type(api: &RestApi, field: &FormField, multipart: bool) -> TokenStream {
    if !multipart {
        return quote! { String };
    }

    match &field.kind {
        FormFieldKind::File { .. } => quote! { crate::shared::FormFile },
        FormFieldKind::Files { .. } => quote! { Vec<crate::shared::FormFile> },
        FormFieldKind::Json(reference) => type_name_to_tokens(&json_type_name(api, reference)),
        _ => quote! { String },
    }
}

/// The statements of a request's `into_parts` that build the multipart
/// parts of its `body` into `parts`, sending each field named in `checked`
/// from the `serde_json::Value` already bound to `checked_{field}` (the
/// value its rules were checked against) rather than serializing it again.
pub(super) fn generate_checked_form_parts(endpoint: &Endpoint, checked: &[String]) -> TokenStream {
    let Some(fields) = form_fields(endpoint) else { return quote! {} };
    let parts = fields.iter().map(|field| {
        let is_checked = checked.contains(&param_field_name(&field.name));
        build_part(field, true, &quote! { self.body }, is_checked)
    });
    quote! {
        let mut parts = Vec::new();
        #(#parts)*
    }
}

/// Emits the statement that appends one field's value, read from
/// `receiver`, to the body parts; a `checked` JSON field is read from its
/// `checked_{field}` binding instead.
fn build_part(field: &FormField, multipart: bool, receiver: &TokenStream, checked: bool) -> TokenStream {
    let name = format_ident!("{}", param_field_name(&field.name));
    let wire_name = &field.name;

    let source = if checked {
        let binding = format_ident!("checked_{}", param_field_name(&field.name));
        quote! { #binding }
    } else {
        quote! { #receiver.#name }
    };
    let push = |value: TokenStream| {
        if field.required {
            quote! { { let value = #source; #value } }
        } else {
            quote! { if let Some(value) = #source { #value } }
        }
    };

    if checked {
        return push(quote! {
            parts.push(schematic_runtime::FormPart::Json { name: #wire_name.to_string(), value: value.to_string() });
        });
    }

    if !multipart {
        return push(quote! { parts.push((#wire_name.to_string(), value)); });
    }

    match &field.kind {
        FormFieldKind::File { .. } => push(quote! {
            parts.push(value.into_part(#wire_name));
        }),
        FormFieldKind::Files { .. } => push(quote! {
            for file in value {
                parts.push(file.into_part(#wire_name));
            }
        }),
        FormFieldKind::Json(_) => push(quote! {
            if let Ok(encoded) = serde_json::to_string(&value) {
                parts.push(schematic_runtime::FormPart::Json { name: #wire_name.to_string(), value: encoded });
            }
        }),
        _ => push(quote! {
            parts.push(schematic_runtime::FormPart::Text { name: #wire_name.to_string(), value });
        }),
    }
}
