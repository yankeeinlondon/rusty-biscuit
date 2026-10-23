//! The body of a request type's `EndpointSpec::into_parts`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::pagination::ParamLocation;
use schematic_define::{ApiRequest, Endpoint};

use crate::codegen::pagination::OrdinaryDefault;
use crate::codegen::value_rules::{BodyRules, RequestRules};

use crate::codegen::request_structs::named_params::generate_headers_assembly;
use crate::codegen::request_structs::path_params::{PathField, generate_typed_path_binding};
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, generate_path_format,
    generate_query_binding,
};

/// Generates the statements that assemble `schematic_runtime::RequestParts`.
///
/// The parts carry no credential; the runtime attaches one per hop. A JSON
/// body that fails to serialize, and a static `Cookie` header that cannot be
/// merged with cookie parameters, are `InvalidArgument`s reported before any
/// network work.
///
/// `typed_paths` are the endpoint's declared path fields; `None` substitutes
/// the untyped `path_params` instead. A non-finite typed number is an
/// `InvalidArgument` too.
///
/// `defaults` are adopted parameters the endpoint declares required: they
/// are not request fields, so their pager default is appended here.
///
/// A request with value rules checks them first, the same checks
/// `validate()` runs: parameters on their typed values before they are
/// encoded, then text and form values, then JSON values. Every violation is
/// returned together as one `InvalidArgument`. A checked JSON body or form
/// field is serialized once to a `serde_json::Value`, checked, and that
/// value is what is sent.
pub(super) fn generate_into_parts_body(
    endpoint: &Endpoint,
    path_params: &[&str],
    typed_paths: Option<&[PathField]>,
    query_params: &[QueryParamInfo],
    defaults: &[OrdinaryDefault],
    rules: &RequestRules,
) -> TokenStream {
    let path_format = match typed_paths {
        Some(fields) => {
            let path = generate_typed_path_binding(&endpoint.path, fields);
            let query = generate_query_binding(query_params);
            quote! { #path #query }
        }
        None => generate_path_format(&endpoint.path, path_params, query_params),
    };

    let (checks, checked_body) = generate_checks(endpoint, rules);
    let body_expr = match (&endpoint.request, &rules.body) {
        (Some(ApiRequest::Json(_)), BodyRules::Json { .. }) => quote! {
            schematic_runtime::RequestBody::Json(
                serde_json::to_string(&body).map_err(|_| {
                    schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON")
                })?
            )
        },
        (Some(ApiRequest::FormData { .. }), BodyRules::FormJson(_)) => quote! {
            schematic_runtime::RequestBody::Multipart(parts)
        },
        (Some(ApiRequest::Json(_)), _) => quote! {
            schematic_runtime::RequestBody::Json(
                serde_json::to_string(&self.body).map_err(|_| {
                    schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON")
                })?
            )
        },
        (Some(ApiRequest::FormData { .. }), _) => quote! {
            schematic_runtime::RequestBody::Multipart(self.body.into_form_parts())
        },
        (Some(ApiRequest::UrlEncoded { .. }), _) => quote! {
            schematic_runtime::RequestBody::UrlEncoded(self.body.into_form_pairs())
        },
        (Some(ApiRequest::Text { content_type }), _) => quote! {
            schematic_runtime::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body.into_bytes().into(),
            }
        },
        (Some(ApiRequest::Binary { content_type }), _) => quote! {
            schematic_runtime::RequestBody::Raw {
                content_type: #content_type.to_string(),
                bytes: self.body.into(),
            }
        },
        _ => quote! { schematic_runtime::RequestBody::Empty },
    };

    let (headers_statements, headers_init) = generate_headers_assembly(
        &endpoint.headers,
        &extract_header_params(&endpoint.params),
        &extract_cookie_params(&endpoint.params),
    );
    // The assembly binds `headers` itself when parameters add to it;
    // otherwise its expression is the static list, bound here.
    let headers_binding = if headers_statements.is_empty() {
        quote! { let headers: Vec<(String, String)> = #headers_init; }
    } else {
        headers_statements
    };

    let defaults_statements = generate_defaults(defaults);

    quote! {
        #checks
        #path_format
        #headers_binding
        #defaults_statements
        #checked_body
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query,
            headers,
            body: #body_expr,
        })
    }
}

/// The statements that check `rules` and return their violations, and
/// those that build a multipart body from the checked JSON fields; both
/// empty for a request without rules.
fn generate_checks(endpoint: &Endpoint, rules: &RequestRules) -> (TokenStream, TokenStream) {
    if !rules.enforces() {
        return (TokenStream::new(), TokenStream::new());
    }
    let field_checks = &rules.field_checks;
    let mut json = TokenStream::new();
    let mut checked_body = TokenStream::new();
    match &rules.body {
        BodyRules::Unchecked => {}
        BodyRules::Json { plan } => json.extend(quote! {
            let body = serde_json::to_value(&self.body).map_err(|_| {
                schematic_runtime::InvalidArgument::new("the request body cannot be serialized as JSON")
            })?;
            schematic_runtime::validate::check_value(
                &body, &#plan, &[schematic_runtime::validate::PathSegment::Name("body")], &mut violations,
            );
        }),
        BodyRules::FormJson(fields) => {
            let mut checked = Vec::new();
            for (field, wire, required, plan) in fields {
                let ident = format_ident!("{field}");
                let binding = format_ident!("checked_{field}");
                let reason = format!("the form field `{wire}` cannot be serialized as JSON");
                let serialize = quote! {
                    serde_json::to_value(value).map_err(|_| schematic_runtime::InvalidArgument::new(#reason))?
                };
                let check = quote! {
                    schematic_runtime::validate::check_value(
                        json, &#plan,
                        &[schematic_runtime::validate::PathSegment::Name("body"), schematic_runtime::validate::PathSegment::Name(#wire)],
                        &mut violations,
                    );
                };
                json.extend(if *required {
                    quote! {
                        let #binding = { let value = &self.body.#ident; #serialize };
                        { let json = &#binding; #check }
                    }
                } else {
                    quote! {
                        let #binding = match &self.body.#ident { Some(value) => Some(#serialize), None => None };
                        if let Some(json) = &#binding { #check }
                    }
                });
                checked.push(field.clone());
            }
            checked_body = super::form::generate_checked_form_parts(endpoint, &checked);
        }
    }
    let checks = quote! {
        let mut violations = schematic_runtime::validate::Violations::new();
        #field_checks
        #json
        violations.into_result().map_err(schematic_runtime::InvalidArgument::from)?;
    };
    (checks, checked_body)
}

/// The body of a request's `validate()`: the same checks as `into_parts`,
/// with each JSON value serialized on its own. A value that cannot be
/// serialized is not reported; sending it fails with the preparation error.
pub(super) fn generate_validate_body(rules: &RequestRules) -> TokenStream {
    let field_checks = &rules.field_checks;
    let json = match &rules.body {
        BodyRules::Unchecked => TokenStream::new(),
        BodyRules::Json { plan } => quote! {
            if let Ok(body) = serde_json::to_value(&self.body) {
                schematic_runtime::validate::check_value(
                    &body, &#plan, &[schematic_runtime::validate::PathSegment::Name("body")], &mut violations,
                );
            }
        },
        BodyRules::FormJson(fields) => {
            let checks = fields.iter().map(|(field, wire, required, plan)| {
                let ident = format_ident!("{field}");
                let check = quote! {
                    if let Ok(json) = serde_json::to_value(value) {
                        schematic_runtime::validate::check_value(
                            &json, &#plan,
                            &[schematic_runtime::validate::PathSegment::Name("body"), schematic_runtime::validate::PathSegment::Name(#wire)],
                            &mut violations,
                        );
                    }
                };
                if *required {
                    quote! { { let value = &self.body.#ident; #check } }
                } else {
                    quote! { if let Some(value) = &self.body.#ident { #check } }
                }
            });
            quote! { #(#checks)* }
        }
    };
    quote! {
        let mut violations = schematic_runtime::validate::Violations::new();
        #field_checks
        #json
        violations.into_result()
    }
}

/// Appends each required adopted parameter's default to the assembled query
/// or headers.
fn generate_defaults(defaults: &[OrdinaryDefault]) -> TokenStream {
    let pushes = |location: ParamLocation, value_type: TokenStream| {
        defaults
            .iter()
            .filter(move |default| default.location == location)
            .map(|OrdinaryDefault { name, value, .. }| quote! { (#name.to_owned(), #value_type::from(#value)) })
            .collect::<Vec<_>>()
    };
    let query = pushes(ParamLocation::Query, quote! { schematic_runtime::QueryValue });
    let headers = pushes(ParamLocation::Header, quote! { String });
    let mut statements = TokenStream::new();
    if !query.is_empty() {
        statements.extend(quote! { let mut query = query; query.extend([#(#query),*]); });
    }
    if !headers.is_empty() {
        statements.extend(quote! { let mut headers = headers; headers.extend([#(#headers),*]); });
    }
    statements
}
