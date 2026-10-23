//! Query parameter shapes, fields, setters, enums, and wire forms.
//!
//! Each query parameter has one [`QueryShape`]: its Rust value type and how
//! it is written into the query. The supported combinations follow OpenAPI's
//! parameter serialization table:
//!
//! | Type | Style | `explode` | Wire form of `["a,b", "c"]` or `"a b"` |
//! | --- | --- | --- | --- |
//! | scalar or enum | `Form` | either | `q=a%20b` |
//! | array of scalars or enums | `Form` | `true` | `tags=a%2Cb&tags=c` |
//! | array of scalars or enums | `Form` | `false` | `tags=a%2Cb,c` |
//! | array of scalars or enums | `SpaceDelimited` | `false` | `tags=a%2Cb%20c` |
//! | array of scalars or enums | `PipeDelimited` | `false` | `tags=a%2Cb%7Cc` |
//!
//! An empty array is sent as `tags=`, and an unset optional parameter is not
//! sent. Every other combination (a JSON value, an array of arrays or of JSON
//! values, `DeepObject` or `Simple` style, a delimited scalar, an exploded
//! delimited array) has no shape: the parameter is left out of the request
//! type and validation reports it.
//!
//! A required parameter is a `T` field taken by `new()`; an optional one is
//! `Option<T>`. Both have a `with_*` setter that assigns that shape.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{ParamDef, ParamStyle, QueryParamType};

use crate::codegen::request_structs::param_enums::generate_param_enum;
use crate::codegen::request_structs::shared::QueryParamInfo;

/// The type of one query value, or of each element of an array.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryScalar {
    String,
    Integer,
    Number,
    Boolean,
    /// A generated enum named `type_name` with one variant per value.
    Enum { type_name: String, values: Vec<String> },
}

/// How a query value is written: one pair, one pair per element, or one
/// pair of delimited elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryForm {
    Single,
    Exploded,
    /// The `schematic_runtime::QueryDelimiter` variant that joins the
    /// elements.
    Delimited(&'static str),
}

/// The Rust value type and wire form of a query parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryShape {
    pub scalar: QueryScalar,
    pub form: QueryForm,
}

/// The shape of query parameter `param`, whose enum (if any) is named
/// `enum_name`, or a description of the declaration the Rust client cannot
/// send (`a JSON value in \`form\` style with explode false`).
pub fn query_shape(param: &ParamDef, enum_name: &str) -> Result<QueryShape, String> {
    let scalar = |param_type: &QueryParamType| match param_type {
        QueryParamType::String => Some(QueryScalar::String),
        QueryParamType::Integer => Some(QueryScalar::Integer),
        QueryParamType::Number => Some(QueryScalar::Number),
        QueryParamType::Boolean => Some(QueryScalar::Boolean),
        QueryParamType::Enum(values) => {
            Some(QueryScalar::Enum { type_name: enum_name.to_string(), values: values.clone() })
        }
        _ => None,
    };
    let (element, array) = match &param.param_type {
        QueryParamType::Array(inner) => (scalar(inner), true),
        param_type => (scalar(param_type), false),
    };
    let form = match (array, param.style, param.explode) {
        (false, ParamStyle::Form, _) => Some(QueryForm::Single),
        (true, ParamStyle::Form, true) => Some(QueryForm::Exploded),
        (true, ParamStyle::Form, false) => Some(QueryForm::Delimited("Comma")),
        (true, ParamStyle::SpaceDelimited, false) => Some(QueryForm::Delimited("Space")),
        (true, ParamStyle::PipeDelimited, false) => Some(QueryForm::Delimited("Pipe")),
        _ => None,
    };
    if let (Some(scalar), Some(form)) = (element, form) {
        return Ok(QueryShape { scalar, form });
    }
    let declared = match &param.param_type {
        QueryParamType::Array(inner) => match **inner {
            QueryParamType::Array(_) => "an array of arrays",
            QueryParamType::Json => "an array of JSON values",
            _ => "an array",
        },
        QueryParamType::Json => "a JSON value",
        _ => "a single value",
    };
    let style = match param.style {
        ParamStyle::Form => "form",
        ParamStyle::SpaceDelimited => "spaceDelimited",
        ParamStyle::PipeDelimited => "pipeDelimited",
        ParamStyle::DeepObject => "deepObject",
        ParamStyle::Simple => "simple",
        _ => "an unknown",
    };
    Err(format!("{declared} in `{style}` style with explode {}", param.explode))
}

impl QueryShape {
    fn scalar_type(&self) -> TokenStream {
        match &self.scalar {
            QueryScalar::String => quote! { String },
            QueryScalar::Integer => quote! { i64 },
            QueryScalar::Number => quote! { f64 },
            QueryScalar::Boolean => quote! { bool },
            QueryScalar::Enum { type_name, .. } => {
                let ident = format_ident!("{}", type_name);
                quote! { #ident }
            }
        }
    }

    /// The value type: the scalar, or a `Vec` of it for an array.
    pub fn rust_type(&self) -> TokenStream {
        let scalar = self.scalar_type();
        match self.form {
            QueryForm::Single => scalar,
            _ => quote! { Vec<#scalar> },
        }
    }

    /// Whether the value is a single string, which `new()` takes as
    /// `impl Into<String>`.
    pub fn is_text(&self) -> bool {
        self.form == QueryForm::Single && self.scalar == QueryScalar::String
    }

    /// The generated enum's name and values, for an enum or array of enums.
    pub fn enum_type(&self) -> Option<(&str, &[String])> {
        match &self.scalar {
            QueryScalar::Enum { type_name, values } => Some((type_name, values)),
            _ => None,
        }
    }

    /// Statements that push the pairs of `value`, a reference to the field's
    /// value type, under the wire name `name`. Each element is written by
    /// its `Display`: the decimal form of a number, `true`/`false`, or an
    /// enum's wire spelling.
    pub fn push_statements(&self, name: &str) -> TokenStream {
        match self.form {
            QueryForm::Single => quote! {
                query.push((#name.to_string(), value.to_string().into()));
            },
            QueryForm::Exploded => quote! {
                if value.is_empty() {
                    query.push((#name.to_string(), String::new().into()));
                }
                for element in value {
                    query.push((#name.to_string(), element.to_string().into()));
                }
            },
            QueryForm::Delimited(delimiter) => {
                let delimiter = format_ident!("{}", delimiter);
                quote! {
                    query.push((
                        #name.to_string(),
                        schematic_runtime::QueryValue::delimited(
                            value.iter().map(|element| element.to_string()),
                            schematic_runtime::QueryDelimiter::#delimiter,
                        ),
                    ));
                }
            }
        }
    }
}

/// Generates field declarations for query parameters: `T` when required,
/// `Option<T>` otherwise.
pub(super) fn generate_query_param_fields(query_params: &[QueryParamInfo]) -> TokenStream {
    let fields = query_params.iter().map(|qp| {
        let field_name = format_ident!("{}", qp.field_name());
        let rename = qp.serde_rename();
        let rust_type = qp.rust_type();
        let field_type = if qp.required { rust_type } else { quote! { Option<#rust_type> } };
        let doc_str = format!(" Query parameter: {}", qp.description.as_deref().unwrap_or(&qp.name));
        let deprecated = qp.deprecation_doc();
        let rules = qp.rules_doc();

        quote! {
            #[doc = #doc_str]
            #deprecated
            #rules
            #rename
            pub #field_name: #field_type,
        }
    });

    quote! { #(#fields)* }
}

/// Generates builder-style `with_*` methods for query parameters.
///
/// Each query parameter gets a `with_name(value)` method that returns `Self`
/// for method chaining; it replaces a required value and sets an optional
/// one.
pub(super) fn generate_query_builder_methods(query_params: &[QueryParamInfo]) -> TokenStream {
    let methods = query_params.iter().map(|qp| {
        let field = qp.field_name();
        let method_name = format_ident!("with_{}", field);
        let field_name = format_ident!("{}", field);
        let rust_type = qp.rust_type();
        let doc = format!(" Sets the `{}` query parameter.", field);
        let deprecated = qp.deprecation_doc();
        let assign = if qp.required { quote! { value } } else { quote! { Some(value) } };

        quote! {
            #[doc = #doc]
            #deprecated
            pub fn #method_name(mut self, value: #rust_type) -> Self {
                self.#field_name = #assign;
                self
            }
        }
    });

    quote! { #(#methods)* }
}

/// Generates the enum of every enum-typed query parameter of the request
/// type `request_type`.
pub(super) fn generate_query_enums(query_params: &[QueryParamInfo], request_type: &str) -> TokenStream {
    let enums = query_params.iter().filter_map(|qp| {
        let (type_name, values) = qp.shape.as_ref()?.enum_type()?;
        Some(generate_param_enum(
            type_name,
            &format!("Values of the `{}` query parameter of [`{request_type}`].", qp.name),
            values,
        ))
    });
    quote! { #(#enums)* }
}
