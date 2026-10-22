//! Path parameter fields, constructor methods, and typed path values.
//!
//! An endpoint that declares no `params.path` keeps untyped placeholders:
//! one `String` field per placeholder. An endpoint that declares them gets
//! one [`PathField`] per distinct placeholder, in first-occurrence template
//! order, typed by its declaration.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, QueryParamType};

use crate::codegen::request_structs::param_enums::{
    enum_variant_name, generate_param_enum, param_enum_name,
};
use crate::codegen::request_structs::shared::{
    QueryParamInfo, param_field_name, param_serde_rename, type_name_to_tokens,
};

/// The Rust type of a typed path field.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PathValue {
    String,
    Integer,
    Number,
    Boolean,
    /// A generated enum named `type_name` with one variant per value.
    Enum { type_name: String, values: Vec<String> },
}

/// One field of a request struct for a declared path parameter.
#[derive(Debug, Clone)]
pub(crate) struct PathField {
    /// The placeholder name, without a reserved-expansion `+`.
    pub wire: String,
    /// The Rust field and constructor argument name.
    pub field: String,
    pub value: PathValue,
    pub description: Option<String>,
    pub deprecated: bool,
}

impl PathField {
    pub(crate) fn rust_type(&self) -> TokenStream {
        match &self.value {
            PathValue::String => quote! { String },
            PathValue::Integer => quote! { i64 },
            PathValue::Number => quote! { f64 },
            PathValue::Boolean => quote! { bool },
            PathValue::Enum { type_name, .. } => {
                let ident = format_ident!("{}", type_name);
                quote! { #ident }
            }
        }
    }

    /// The constructor argument type: strings accept anything `Into<String>`,
    /// every other value is taken by value.
    pub(crate) fn arg_type(&self) -> TokenStream {
        match self.value {
            PathValue::String => quote! { impl Into<String> },
            _ => self.rust_type(),
        }
    }

    /// A plausible argument for documentation examples.
    pub(crate) fn example_arg(&self) -> String {
        match &self.value {
            PathValue::String => format!("\"{}_value\"", self.wire),
            PathValue::Integer => "1".to_string(),
            PathValue::Number => "1.5".to_string(),
            PathValue::Boolean => "true".to_string(),
            PathValue::Enum { type_name, values } => {
                format!("{type_name}::{}", enum_variant_name(&values[0]))
            }
        }
    }

    fn ident(&self) -> proc_macro2::Ident {
        format_ident!("{}", self.field)
    }
}

/// The typed path fields of `endpoint`'s request type `request_type`, or
/// `None` when the endpoint declares no path parameters.
///
/// A placeholder without a declaration, or a declaration of a type a path
/// cannot carry, is a `String` field; validation reports both before any
/// output is written.
pub(crate) fn typed_path_fields(endpoint: &Endpoint, request_type: &str) -> Option<Vec<PathField>> {
    let declared = &endpoint.params.as_ref()?.path;
    if declared.is_empty() {
        return None;
    }
    let fields = endpoint
        .path_placeholders()
        .into_iter()
        .map(|wire| {
            let param = declared.iter().find(|param| param.name == wire);
            let field = param
                .and_then(|param| param.rust_name.clone())
                .unwrap_or_else(|| param_field_name(wire));
            let value = match param.map(|param| &param.param_type) {
                Some(QueryParamType::Integer) => PathValue::Integer,
                Some(QueryParamType::Number) => PathValue::Number,
                Some(QueryParamType::Boolean) => PathValue::Boolean,
                Some(QueryParamType::Enum(values)) if !values.is_empty() => PathValue::Enum {
                    type_name: param_enum_name(request_type, &field),
                    values: values.clone(),
                },
                _ => PathValue::String,
            };
            PathField {
                wire: wire.to_string(),
                field,
                value,
                description: param.and_then(|param| param.description.clone()),
                deprecated: param.is_some_and(|param| param.deprecated),
            }
        })
        .collect();
    Some(fields)
}

/// Generates field declarations for untyped path parameters.
pub(super) fn generate_path_param_fields(path_params: &[&str]) -> TokenStream {
    let fields = path_params.iter().map(|param| {
        let field_name = format_ident!("{}", param_field_name(param));
        let rename = param_serde_rename(param);
        let doc = format!(" Path parameter: {}", param);
        quote! {
            #[doc = #doc]
            #rename
            pub #field_name: String,
        }
    });

    quote! { #(#fields)* }
}

/// Generates field declarations for typed path parameters.
pub(super) fn generate_typed_path_fields(fields: &[PathField]) -> TokenStream {
    let fields = fields.iter().map(|path| {
        let field_name = path.ident();
        let wire = &path.wire;
        let rename = if path.field == *wire { quote! {} } else { quote! { #[serde(rename = #wire)] } };
        let rust_type = path.rust_type();
        let mut doc = vec![match &path.description {
            Some(description) => format!(" {description}"),
            None => format!(" Path parameter: {wire}"),
        }];
        if path.deprecated {
            doc.extend([String::new(), " Deprecated.".to_string()]);
        }
        quote! {
            #(#[doc = #doc])*
            #rename
            pub #field_name: #rust_type,
        }
    });

    quote! { #(#fields)* }
}

/// Generates the enum of every enum-typed path field.
pub(super) fn generate_path_enums(fields: &[PathField], request_type: &str) -> TokenStream {
    let enums = fields.iter().filter_map(|path| match &path.value {
        PathValue::Enum { type_name, values } => Some(generate_param_enum(
            type_name,
            &format!("Values of the `{}` path parameter of [`{request_type}`].", path.wire),
            values,
        )),
        _ => None,
    });
    quote! { #(#enums)* }
}

/// One required path argument of `new()`.
pub(super) struct PathArg {
    name: proc_macro2::Ident,
    ty: TokenStream,
    init: TokenStream,
}

/// The `new()` arguments of untyped placeholders: each takes
/// `impl Into<String>`.
pub(super) fn untyped_path_args(path_params: &[&str]) -> Vec<PathArg> {
    path_params
        .iter()
        .map(|param| {
            let name = format_ident!("{}", param_field_name(param));
            PathArg { ty: quote! { impl Into<String> }, init: quote! { #name: #name.into() }, name }
        })
        .collect()
}

/// The `new()` arguments of typed path fields, in their template order.
pub(super) fn typed_path_args(fields: &[PathField]) -> Vec<PathArg> {
    fields
        .iter()
        .map(|path| {
            let name = path.ident();
            let init = match path.value {
                PathValue::String => quote! { #name: #name.into() },
                _ => quote! { #name },
            };
            PathArg { ty: path.arg_type(), init, name }
        })
        .collect()
}

/// Generates a `new()` constructor method body for the request struct.
///
/// The constructor requires all path parameters and the body (if present),
/// providing compile-time enforcement of required fields.
///
/// ## Returns
///
/// - For structs with path params or body: method body for `new()` (no impl wrapper)
/// - For empty structs (no params, no body): empty `TokenStream` (use `Default`)
pub(super) fn generate_new_method(
    path_args: &[PathArg],
    query_params: &[QueryParamInfo],
    has_body: bool,
    body_type: Option<&str>,
) -> TokenStream {
    let params: Vec<_> = path_args
        .iter()
        .map(|PathArg { name, ty, .. }| quote! { #name: #ty })
        .collect();

    let path_field_inits: Vec<_> = path_args.iter().map(|arg| &arg.init).collect();

    // Initialize query params to None
    let query_field_inits: Vec<_> = query_params
        .iter()
        .map(|qp| {
            let name = format_ident!("{}", qp.field_name());
            quote! { #name: None }
        })
        .collect();

    if has_body {
        let body_ty = type_name_to_tokens(body_type.unwrap_or("Body"));
        quote! {
            /// Creates a new request with the required path parameters and body.
            pub fn new(#(#params,)* body: #body_ty) -> Self {
                Self {
                    #(#path_field_inits,)*
                    #(#query_field_inits,)*
                    body,
                }
            }
        }
    } else if !path_args.is_empty() {
        quote! {
            /// Creates a new request with the required path parameters.
            pub fn new(#(#params),*) -> Self {
                Self {
                    #(#path_field_inits,)*
                    #(#query_field_inits,)*
                }
            }
        }
    } else if !query_params.is_empty() {
        // No path params but has query params - need to initialize them
        quote! {
            /// Creates a new request with default values.
            pub fn new() -> Self {
                Self {
                    #(#query_field_inits,)*
                }
            }
        }
    } else {
        // No params, no body - new() is just Default
        quote! {}
    }
}

/// Generates the statement that binds `path` inside `into_parts` for typed
/// path fields.
///
/// Each occurrence in the template is substituted with its field's wire
/// spelling: the decimal form of a number, `true`/`false`, an enum's
/// `as_str`, or the string itself. It is percent-encoded unless that
/// occurrence is written `{+name}`, which keeps the raw value. A non-finite
/// `f64` is an `InvalidArgument` naming the parameter, returned before the
/// path is built.
pub(super) fn generate_typed_path_binding(path: &str, fields: &[PathField]) -> TokenStream {
    let finite_checks = fields.iter().filter(|path| path.value == PathValue::Number).map(|path| {
        let field_name = path.ident();
        let reason = format!("path parameter `{}` must be a finite number", path.wire);
        quote! {
            if !self.#field_name.is_finite() {
                return Err(schematic_runtime::InvalidArgument::new(#reason));
            }
        }
    });

    let mut format_str = String::with_capacity(path.len());
    let mut format_args = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|offset| open + offset) else {
            break;
        };
        let placeholder = &rest[open + 1..close];
        let (wire, reserved) = match placeholder.strip_prefix('+') {
            Some(wire) => (wire, true),
            None => (placeholder, false),
        };
        format_str.push_str(&rest[..open].replace('}', "}}"));
        match fields.iter().find(|path| path.wire == wire) {
            Some(path) => {
                format_str.push_str("{}");
                format_args.push(occurrence_value(path, reserved));
            }
            // Not a placeholder of this template (an empty `{}`): kept as text.
            None => format_str.push_str(&rest[open..=close].replace('{', "{{").replace('}', "}}")),
        }
        rest = &rest[close + 1..];
    }
    format_str.push_str(&rest.replace('{', "{{").replace('}', "}}"));

    quote! {
        #(#finite_checks)*
        let path = format!(#format_str, #(#format_args),*);
    }
}

/// The value one placeholder occurrence interpolates. A reserved occurrence
/// passes a scalar to `format!` as itself, since its `Display` is already its
/// wire spelling.
fn occurrence_value(path: &PathField, reserved: bool) -> TokenStream {
    let field_name = path.ident();
    match (&path.value, reserved) {
        (PathValue::Enum { .. }, true) => quote! { self.#field_name.as_str() },
        (_, true) => quote! { self.#field_name },
        (PathValue::Enum { .. }, false) => quote! { urlencoding::encode(self.#field_name.as_str()) },
        (PathValue::String, false) => quote! { urlencoding::encode(&self.#field_name) },
        (_, false) => quote! { urlencoding::encode(&self.#field_name.to_string()) },
    }
}
