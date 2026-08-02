//! Emits an imported [`RestApi`] as `schematic-definitions`-shaped Rust source.
//!
//! The standalone importer output in [`crate::import_pipeline`] writes a
//! self-contained client. This module writes the other half of the house
//! layout instead: a `define_{api}_api()` function that reconstructs the
//! imported [`RestApi`] as a value, so an imported API sits in the definitions
//! catalog alongside the hand-authored ones and flows through the normal
//! `generate` pipeline.

use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EndpointParams, FormField, FormFieldKind,
    ParamDef, ParamStyle, QueryParamType, RestApi, RestMethod, Schema,
};

use crate::errors::GeneratorError;
use crate::output::{format_code, validate_code, write_atomic};

/// Writes `mod.rs` for a definitions-crate module describing `api`.
///
/// ## Errors
///
/// Returns `GeneratorError` if the emitted tokens do not parse as Rust or the
/// file cannot be written.
pub fn generate_definition_module(
    api: &RestApi,
    module_name: &str,
    model_names: &[String],
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let tokens = assemble_definition_module(api, module_name, model_names);
    let file = validate_code(&tokens)?;
    let formatted = format_code(&file);

    if dry_run {
        println!("=== {module_name}/mod.rs ===\n{formatted}\n");
    } else {
        write_atomic(&output_dir.join("mod.rs"), &formatted)?;
    }

    Ok(formatted)
}

/// Assembles the full module token stream.
fn assemble_definition_module(
    api: &RestApi,
    module_name: &str,
    model_names: &[String],
) -> TokenStream {
    let fn_name = format_ident!("define_{}_api", module_name);
    let api_expr = rest_api_tokens(api);
    let registry = registry_tokens(model_names);

    let module_doc = format!(
        " {} REST API definition, imported from an OpenAPI specification.",
        api.name
    );
    let regenerate_doc = format!(
        " Regenerate with `schematic-gen import --input <spec> --api-name {} --definitions-out <dir>`.",
        api.name
    );
    let endpoint_doc = format!(" Endpoints: {}.", api.endpoints.len());
    let fn_doc = format!(" Builds the {} REST API definition.", api.name);

    quote! {
        #![doc = #module_doc]
        #![doc = ""]
        #![doc = #endpoint_doc]
        #![doc = ""]
        #![doc = #regenerate_doc]

        mod types;

        pub use types::*;

        use crate::registry::SchemaRegistry;
        use schematic_define::*;

        #[doc = #fn_doc]
        #[must_use]
        pub fn #fn_name() -> RestApi {
            #api_expr
        }

        #registry
    }
}

/// Emits `openapi_registry()`, registering every generated model type.
///
/// The registry is what supplies `components.schemas` on OpenAPI export, so it
/// has to name every type the endpoints can reference; `validate_completeness`
/// checks exactly that.
fn registry_tokens(model_names: &[String]) -> TokenStream {
    let count = model_names.len();
    let registrations = model_names.iter().map(|name| {
        let ident = format_ident!("{}", name);
        quote! { .register::<#ident>(#name) }
    });

    let doc = format!(" Builds a schema registry holding all {count} generated model types.");

    quote! {
        #[doc = #doc]
        #[must_use]
        pub fn openapi_registry() -> SchemaRegistry {
            SchemaRegistry::new()
                #(#registrations)*
        }
    }
}

fn rest_api_tokens(api: &RestApi) -> TokenStream {
    let name = &api.name;
    let description = &api.description;
    let base_url = &api.base_url;
    let docs_url = option_string_tokens(api.docs_url.as_deref());
    let auth = auth_strategy_tokens(&api.auth);
    let env_auth = string_vec_tokens(&api.env_auth);
    let env_username = option_string_tokens(api.env_username.as_deref());
    let headers = header_pairs_tokens(&api.headers);
    let module_path = option_string_tokens(api.module_path.as_deref());
    let request_suffix = option_string_tokens(api.request_suffix.as_deref());
    let version = option_string_tokens(api.version.as_deref());

    let endpoints = api.endpoints.iter().map(endpoint_tokens);

    // `Vec::from([..])` rather than `vec![..]`: prettyplease does not format
    // inside a macro invocation, so a `vec!` here would emit the whole endpoint
    // list as one unformatted token soup.
    quote! {
        let endpoints = Vec::from([#(#endpoints),*]);

        RestApi {
            name: #name.to_string(),
            description: #description.to_string(),
            base_url: #base_url.to_string(),
            docs_url: #docs_url,
            auth: #auth,
            auth_policy: None,
            env_auth: #env_auth,
            env_username: #env_username,
            headers: #headers,
            endpoints,
            module_path: #module_path,
            request_suffix: #request_suffix,
            version: #version,
            env_mapping: None,
        }
    }
}

fn endpoint_tokens(endpoint: &Endpoint) -> TokenStream {
    let id = &endpoint.id;
    let method = method_tokens(endpoint.method);
    let path = &endpoint.path;
    let description = &endpoint.description;
    let request = match &endpoint.request {
        Some(request) => {
            let request = api_request_tokens(request);
            quote! { Some(#request) }
        }
        None => quote! { None },
    };
    let response = api_response_tokens(&endpoint.response);
    let headers = header_pairs_tokens(&endpoint.headers);
    let params = match &endpoint.params {
        Some(params) => {
            let params = endpoint_params_tokens(params);
            quote! { Some(#params) }
        }
        None => quote! { None },
    };
    let oauth_scopes = match &endpoint.oauth_scopes {
        Some(scopes) => {
            let scopes = string_vec_tokens(scopes);
            quote! { Some(#scopes) }
        }
        None => quote! { None },
    };

    quote! {
        Endpoint {
            id: #id.to_string(),
            method: #method,
            path: #path.to_string(),
            description: #description.to_string(),
            request: #request,
            response: #response,
            headers: #headers,
            params: #params,
            oauth_scopes: #oauth_scopes,
        }
    }
}

fn method_tokens(method: RestMethod) -> TokenStream {
    let variant = format_ident!(
        "{}",
        match method {
            RestMethod::Get => "Get",
            RestMethod::Post => "Post",
            RestMethod::Put => "Put",
            RestMethod::Patch => "Patch",
            RestMethod::Delete => "Delete",
            RestMethod::Head => "Head",
            RestMethod::Options => "Options",
            _ => "Get",
        }
    );
    quote! { RestMethod::#variant }
}

fn api_request_tokens(request: &ApiRequest) -> TokenStream {
    match request {
        ApiRequest::Json(schema) => {
            let schema = schema_tokens(schema);
            quote! { ApiRequest::Json(#schema) }
        }
        ApiRequest::FormData { fields } => {
            let fields = fields.iter().map(form_field_tokens);
            quote! { ApiRequest::FormData { fields: vec![#(#fields),*] } }
        }
        ApiRequest::UrlEncoded { fields } => {
            let fields = fields.iter().map(form_field_tokens);
            quote! { ApiRequest::UrlEncoded { fields: vec![#(#fields),*] } }
        }
        ApiRequest::Text { content_type } => {
            quote! { ApiRequest::Text { content_type: #content_type.to_string() } }
        }
        ApiRequest::Binary { content_type } => {
            quote! { ApiRequest::Binary { content_type: #content_type.to_string() } }
        }
        // `ApiRequest` is `#[non_exhaustive]`; an unmodelled variant degrades to
        // "no body" rather than emitting source that would not compile.
        _ => quote! { ApiRequest::Text { content_type: "application/octet-stream".to_string() } },
    }
}

fn api_response_tokens(response: &ApiResponse) -> TokenStream {
    match response {
        ApiResponse::Json(schema) => {
            let schema = schema_tokens(schema);
            quote! { ApiResponse::Json(#schema) }
        }
        ApiResponse::Text => quote! { ApiResponse::Text },
        ApiResponse::Binary => quote! { ApiResponse::Binary },
        ApiResponse::Empty => quote! { ApiResponse::Empty },
        _ => quote! { ApiResponse::Empty },
    }
}

fn schema_tokens(schema: &Schema) -> TokenStream {
    let type_name = &schema.type_name;
    match &schema.module_path {
        Some(path) => quote! { Schema::with_path(#type_name, #path) },
        None => quote! { Schema::new(#type_name) },
    }
}

fn form_field_tokens(field: &FormField) -> TokenStream {
    let name = &field.name;

    let base = match &field.kind {
        FormFieldKind::Text => quote! { FormField::text(#name) },
        FormFieldKind::File { accept } if accept.is_empty() => quote! { FormField::file(#name) },
        FormFieldKind::File { accept } => {
            let accept = string_vec_tokens(accept);
            quote! { FormField::file_accept(#name, #accept) }
        }
        FormFieldKind::Files { .. } => quote! { FormField::files(#name) },
        FormFieldKind::Json(schema) => {
            let schema = schema_tokens(schema);
            quote! {
                FormField {
                    name: #name.to_string(),
                    kind: FormFieldKind::Json(#schema),
                    required: true,
                    description: None,
                }
            }
        }
        _ => quote! { FormField::text(#name) },
    };

    let base = if field.required {
        base
    } else {
        quote! { #base.optional() }
    };

    match &field.description {
        Some(description) => quote! { #base.with_description(#description) },
        None => base,
    }
}

fn endpoint_params_tokens(params: &EndpointParams) -> TokenStream {
    let query = params.query.iter().map(param_def_tokens);
    let header = params.header.iter().map(param_def_tokens);
    let cookie = params.cookie.iter().map(param_def_tokens);

    // `pagination` and `response_pagination` are not recovered from an imported
    // spec; OpenAPI has no vocabulary for either.
    quote! {
        EndpointParams {
            query: vec![#(#query),*],
            header: vec![#(#header),*],
            cookie: vec![#(#cookie),*],
            pagination: None,
            response_pagination: None,
        }
    }
}

fn param_def_tokens(param: &ParamDef) -> TokenStream {
    let name = &param.name;
    let required = param.required;
    let description = option_string_tokens(param.description.as_deref());
    let param_type = query_param_type_tokens(&param.param_type);
    let explode = param.explode;
    let style = param_style_tokens(param.style);

    quote! {
        ParamDef {
            name: #name.to_string(),
            required: #required,
            description: #description,
            param_type: #param_type,
            explode: #explode,
            style: #style,
        }
    }
}

fn query_param_type_tokens(param_type: &QueryParamType) -> TokenStream {
    match param_type {
        QueryParamType::String => quote! { QueryParamType::String },
        QueryParamType::Integer => quote! { QueryParamType::Integer },
        QueryParamType::Number => quote! { QueryParamType::Number },
        QueryParamType::Boolean => quote! { QueryParamType::Boolean },
        QueryParamType::Array(inner) => {
            let inner = query_param_type_tokens(inner);
            quote! { QueryParamType::Array(Box::new(#inner)) }
        }
        QueryParamType::Enum(values) => {
            let values = string_vec_tokens(values);
            quote! { QueryParamType::Enum(#values) }
        }
        QueryParamType::Json => quote! { QueryParamType::Json },
        _ => quote! { QueryParamType::String },
    }
}

fn param_style_tokens(style: ParamStyle) -> TokenStream {
    let variant = format_ident!(
        "{}",
        match style {
            ParamStyle::Form => "Form",
            ParamStyle::Simple => "Simple",
            ParamStyle::SpaceDelimited => "SpaceDelimited",
            ParamStyle::PipeDelimited => "PipeDelimited",
            ParamStyle::DeepObject => "DeepObject",
            _ => "Form",
        }
    );
    quote! { ParamStyle::#variant }
}

fn auth_strategy_tokens(auth: &AuthStrategy) -> TokenStream {
    match auth {
        AuthStrategy::None => quote! { AuthStrategy::None },
        AuthStrategy::Basic => quote! { AuthStrategy::Basic },
        AuthStrategy::BearerToken { header } => {
            let header = option_string_tokens(header.as_deref());
            quote! { AuthStrategy::BearerToken { header: #header } }
        }
        AuthStrategy::ApiKey {
            header,
            value_prefix,
        } => {
            let value_prefix = option_string_tokens(value_prefix.as_deref());
            quote! {
                AuthStrategy::ApiKey {
                    header: #header.to_string(),
                    value_prefix: #value_prefix,
                }
            }
        }
        _ => quote! { AuthStrategy::None },
    }
}

fn option_string_tokens(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value.to_string()) },
        None => quote! { None },
    }
}

fn string_vec_tokens(values: &[String]) -> TokenStream {
    let values = values.iter();
    quote! { vec![#(#values.to_string()),*] }
}

fn header_pairs_tokens(headers: &[(String, String)]) -> TokenStream {
    let pairs = headers
        .iter()
        .map(|(key, value)| quote! { (#key.to_string(), #value.to_string()) });
    quote! { vec![#(#pairs),*] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_api() -> RestApi {
        RestApi {
            name: "Sample".to_string(),
            description: "Sample API".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            docs_url: Some("https://docs.example.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["SAMPLE_TOKEN".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        }
    }

    #[test]
    fn emits_parseable_rust() {
        let tokens = assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".to_string()]);
        assert!(validate_code(&tokens).is_ok());
    }

    #[test]
    fn emits_the_conventional_function_name() {
        let tokens = assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".to_string()]);
        let code = format_code(&validate_code(&tokens).unwrap());

        assert!(code.contains("pub fn define_sample_api() -> RestApi"));
    }

    #[test]
    fn emits_each_endpoint() {
        let tokens = assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".to_string()]);
        let code = format_code(&validate_code(&tokens).unwrap());

        assert!(code.contains(r#"id: "ListItems".to_string()"#));
        assert!(code.contains("RestMethod::Get"));
        assert!(code.contains(r#"path: "/items".to_string()"#));
    }

    /// Renders a bare expression inside a function so it parses as a file.
    fn render_expr(tokens: TokenStream) -> String {
        let wrapped = quote! { fn probe() { let _ = #tokens; } };
        format_code(&validate_code(&wrapped).unwrap())
    }

    #[test]
    fn emits_form_fields_with_optionality_and_description() {
        let field = FormField::file("audio")
            .optional()
            .with_description("The audio file");
        let code = render_expr(form_field_tokens(&field));

        assert!(code.contains(r#"FormField::file("audio")"#));
        assert!(code.contains(".optional()"));
        assert!(code.contains(r#".with_description("The audio file")"#));
    }

    #[test]
    fn emits_required_form_fields_without_optional() {
        let code = render_expr(form_field_tokens(&FormField::text("model")));

        assert!(code.contains(r#"FormField::text("model")"#));
        assert!(!code.contains(".optional()"));
    }
}
