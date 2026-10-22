//! Emits an imported [`RestApi`] as `schematic-definitions`-shaped Rust source.
//!
//! The standalone importer output in [`crate::import_pipeline`] writes a
//! self-contained client. This module writes the other half of the house
//! layout instead: a `define_{api}_api()` factory backed by fallible typed builders for the
//! imported [`RestApi`] as a value, so an imported API sits in the definitions
//! catalog alongside the hand-authored ones and flows through the normal
//! `generate` pipeline.
//!
//! An imported document carries no pagination, so authored pagination for an
//! imported API lives in a hand-written overlay function the importer never
//! writes. When one is configured, the emitted fallible assembly passes the built
//! value through it, so `define_{api}_api()` returns the augmented definition
//! to every caller.

use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{
    ApiKeyLocation, ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, FormField,
    FormFieldKind, OAuth2Config, ParamDef, ParamStyle, PrimitiveType, QueryParamType, RestApi,
    RestMethod, TypeRef,
};

use crate::errors::GeneratorError;
use crate::output::{format_code, validate_code, write_atomic};

/// Writes `mod.rs` for a definitions-crate module describing `api`.
///
/// `overlay`, when given, is the Rust path of a `fn(RestApi) -> RestApi` the
/// emitted factory applies to the imported value (see the module docs).
///
/// ## Errors
///
/// Returns `GeneratorError::ConfigError` if `overlay` is not a plain Rust path
/// (see [`parse_overlay_path`]), and another `GeneratorError` if the emitted
/// tokens do not parse as Rust or the file cannot be written. Nothing is
/// written on error.
pub fn generate_definition_module(
    api: &RestApi,
    module_name: &str,
    model_names: &[String],
    overlay: Option<&str>,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let overlay = overlay.map(parse_overlay_path).transpose()?;
    let tokens = assemble_definition_module(api, module_name, model_names, overlay.as_ref());
    let file = validate_code(&tokens)?;
    let formatted = format_code(&file);

    if dry_run {
        println!("=== {module_name}/mod.rs ===\n{formatted}\n");
    } else {
        write_atomic(&output_dir.join("mod.rs"), &formatted)?;
    }

    Ok(formatted)
}

/// Parses an `--overlay` value as a plain Rust path to a function, such as
/// `crate::openai_pagination::apply`.
///
/// Generic arguments are rejected: the overlay is called with one `RestApi`
/// argument, so its type parameters, if any, are always inferable.
///
/// ## Errors
///
/// Returns `GeneratorError::ConfigError` naming the option when `value` is
/// empty, is not a path, or has generic arguments on any segment.
pub fn parse_overlay_path(value: &str) -> Result<syn::Path, GeneratorError> {
    let invalid = |reason: &str| {
        GeneratorError::ConfigError(format!(
            "--overlay `{value}` {reason}; expected a Rust path to a `fn(RestApi) -> RestApi`, such as `crate::openai_pagination::apply`"
        ))
    };

    if value.trim().is_empty() {
        return Err(invalid("is empty"));
    }
    let path: syn::Path = syn::parse_str(value).map_err(|_| invalid("is not a Rust path"))?;
    if path.segments.iter().any(|segment| !segment.arguments.is_none()) {
        return Err(invalid("has generic arguments"));
    }
    Ok(path)
}

/// Renders `path` without the spaces `TokenStream`'s `Display` puts around `::`.
fn path_text(path: &syn::Path) -> String {
    let segments: Vec<String> =
        path.segments.iter().map(|segment| segment.ident.to_string()).collect();
    let leading = if path.leading_colon.is_some() { "::" } else { "" };
    format!("{leading}{}", segments.join("::"))
}

/// Assembles the full module token stream.
fn assemble_definition_module(
    api: &RestApi,
    module_name: &str,
    model_names: &[String],
    overlay: Option<&syn::Path>,
) -> TokenStream {
    let fn_name = format_ident!("define_{}_api", module_name);
    let api_expr = rest_api_tokens(api, overlay);
    // Bound debug-build stack use even for documents with thousands of models.
    let registrations = model_names.chunks(32).enumerate().map(|(index, names)| {
        let function = format_ident!("register_types_{}", index);
        let registrations = names.iter().map(|name| {
            let ty: syn::Type = syn::parse_str(name).expect("validated model type");
            quote! { ctx.register::<#ty>()?; }
        });
        quote! {
            fn #function(ctx: &mut DefinitionContext) -> Result<(), DefinitionError> {
                #(#registrations)*
                Ok(())
            }
            #function(ctx)?;
        }
    });
    let module_path = format!("::schematic_definitions::{module_name}");
    let module_doc = format!(" {} REST API definition, imported from an OpenAPI specification.", api.name);
    let overlay_arg = overlay.map(|p| format!(" --overlay {}", path_text(p))).unwrap_or_default();
    let regenerate_doc = format!(" Regenerate with `schematic-gen import --input <spec> --api-name {} --definitions-out <dir>{overlay_arg}`.", api.name);
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
        use crate::registry::assemble_module_group;
        use schematic_define::*;

        #[doc = #fn_doc]
        #[must_use]
        pub fn #fn_name() -> RestApi {
            module_group().expect("valid imported API").remove(0)
        }

        /// Assemble the API and its ordered schemas from one registration context.
        pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
            assemble_module_group(#module_path, |ctx| {
                #(#registrations)*
                let api = imported_api(ctx)?;
                Ok(vec![api])
            })
        }

        fn imported_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
            #api_expr
        }
    }
}

fn rest_api_tokens(api: &RestApi, overlay: Option<&syn::Path>) -> TokenStream {
    let name = &api.name;
    let description = &api.description;
    let base_url = &api.base_url;
    let auth = auth_strategy_tokens(&api.auth);
    let env_auth = string_vec_tokens(&api.env_auth);
    let mut builder = quote! { RestApi::builder(ctx, #name, #base_url).describes(#description).authentication(#auth, #env_auth) };
    for (value, setter) in [
        (&api.docs_url, "docs_url"),
        (&api.module_path, "module_path"), (&api.request_suffix, "request_suffix"),
        (&api.version, "version"), (&api.request_id_header, "request_id_header"),
    ] {
        if let Some(value) = value {
            let setter = format_ident!("{}", setter);
            builder = quote! { #builder.#setter(#value) };
        }
    }
    let policy = cross_origin_tokens(&api.cross_origin);
    builder = quote! { #builder.cross_origin(#policy) };
    for name in &api.secret_names { builder = quote! { #builder.secret_header(#name) }; }
    for (key, value) in &api.headers { builder = quote! { #builder.header(#key, #value) }; }
    let endpoints = api.endpoints.iter().enumerate().map(|(index, endpoint)| {
        let function = format_ident!("endpoint_{}", index);
        let expression = endpoint_tokens(endpoint);
        let typed_request = match &endpoint.request {
            Some(ApiRequest::Json(_)) => true,
            Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) => fields.iter().any(|field| matches!(field.kind, FormFieldKind::Json(_))),
            _ => false,
        };
        let argument = if matches!(endpoint.response, ApiResponse::Json(_)) || typed_request { format_ident!("ctx") } else { format_ident!("_ctx") };
        quote! {
            fn #function(#argument: &mut DefinitionContext) -> Result<Endpoint, DefinitionError> {
                #expression
            }
            endpoints.push(#function(ctx)?);
        }
    });
    let count = api.endpoints.len();
    let tail = overlay.map(|overlay| quote! { #overlay(api) }).unwrap_or_else(|| quote! { api });
    quote! {
        let mut endpoints = Vec::with_capacity(#count);
        #(#endpoints)*
        let api = #builder.endpoints(endpoints).build()?;
        Ok(#tail)
    }
}

fn endpoint_tokens(endpoint: &Endpoint) -> TokenStream {
    let id = &endpoint.id;
    let method = method_tokens(endpoint.method);
    let path = &endpoint.path;
    let description = &endpoint.description;
    let mut builder = quote! { Endpoint::new(#id, #method, #path).describes(#description) };
    builder = match &endpoint.response {
        ApiResponse::Json(reference) => {
            let ty = imported_type(reference);
            quote! { #builder.returns_json::<#ty>(ctx) }
        }
        ApiResponse::Text => quote! { #builder.returns_text() },
        ApiResponse::Binary => quote! { #builder.returns_binary() },
        _ => quote! { #builder.returns_empty() },
    };
    if let Some(request) = &endpoint.request {
        builder = match request {
            ApiRequest::Json(reference) => {
                let ty = imported_type(reference);
                quote! { #builder.request_json::<#ty>(ctx) }
            }
            ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields } => {
                let setter = if matches!(request, ApiRequest::FormData { .. }) { format_ident!("request_form") } else { format_ident!("request_urlencoded") };
                let fields = fields.iter().map(form_field_tokens);
                quote! { #builder.#setter(Vec::from([#(#fields),*])) }
            }
            ApiRequest::Text { content_type } => quote! { #builder.request_text_with_content_type(#content_type) },
            ApiRequest::Binary { content_type } => quote! { #builder.request_binary_with_content_type(#content_type) },
            _ => builder,
        };
    }
    if let Some(policy) = &endpoint.cross_origin {
        let policy = cross_origin_tokens(policy);
        builder = quote! { #builder.cross_origin(#policy) };
    }
    if let Some(params) = &endpoint.params {
        for (params, setter) in [(&params.query, "query"), (&params.header, "header_param"), (&params.cookie, "cookie")] {
            let setter = format_ident!("{}", setter);
            for param in params { let param = param_def_tokens(param); builder = quote! { #builder.#setter(#param) }; }
        }
    }
    for (key, value) in &endpoint.headers { builder = quote! { #builder.header(#key, #value) }; }
    quote! { #builder.build() }
}

/// The Rust type an emitted builder registers for an imported reference.
///
/// Names are the generated models of the same module (`pub use types::*`),
/// and free-form JSON is spelled `serde_json::Value`, as the importer has
/// always written it. Wrappers spell their standard-library type.
///
/// ## Panics
///
/// Panics on a composition or `Unknown`, which the importer never produces
/// for a body or response.
fn imported_type(reference: &TypeRef) -> syn::Type {
    fn spelling(reference: &TypeRef) -> String {
        match reference {
            TypeRef::Named(name) => name.clone(),
            TypeRef::Primitive(PrimitiveType::Json) => "serde_json::Value".to_string(),
            TypeRef::Primitive(primitive) => primitive.rust_path().to_string(),
            TypeRef::Array(item) => format!("Vec<{}>", spelling(item)),
            TypeRef::Optional(inner) => format!("Option<{}>", spelling(inner)),
            TypeRef::Map(value) => format!("std::collections::HashMap<String, {}>", spelling(value)),
            TypeRef::KeyedMap { key, value } => format!("std::collections::HashMap<{key}, {}>", spelling(value)),
            other => panic!("imported schema reference {other:?} has no Rust type"),
        }
    }
    let spelling = spelling(reference);
    syn::parse_str(&spelling).unwrap_or_else(|error| panic!("imported type `{spelling}` is not a Rust type: {error}"))
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
        FormFieldKind::Json(reference) => {
            let ty = imported_type(reference);
            quote! { FormField::json_desc(#name, ctx.register::<#ty>()?) }
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

fn param_def_tokens(param: &ParamDef) -> TokenStream {
    let name = &param.name;
    let required = param.required;
    let secret = param.secret;
    let pager_managed = param.pager_managed;
    // The importer does not map OpenAPI `deprecated` yet, so this is false.
    let deprecated = param.deprecated;
    let rust_name = option_string_tokens(param.rust_name.as_deref());
    let description = option_string_tokens(param.description.as_deref());
    let param_type = query_param_type_tokens(&param.param_type);
    let explode = param.explode;
    let style = param_style_tokens(param.style);

    quote! {
        ParamDef {
            name: #name.to_string(),
            required: #required,
            secret: #secret,
            rust_name: #rust_name,
            pager_managed: #pager_managed,
            deprecated: #deprecated,
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
        AuthStrategy::Basic { username_env, password_env } => {
            let username_env = string_vec_tokens(username_env.names());
            let password_env = string_vec_tokens(password_env.names());
            quote! { AuthStrategy::Basic { username_env: EnvList::new(#username_env), password_env: EnvList::new(#password_env) } }
        }
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
        AuthStrategy::ApiKeyParam { name, location } => {
            let location = match location {
                ApiKeyLocation::Query => quote! { ApiKeyLocation::Query },
                ApiKeyLocation::Cookie => quote! { ApiKeyLocation::Cookie },
                _ => unreachable!("unsupported imported API-key location"),
            };
            quote! { AuthStrategy::ApiKeyParam { name: #name.to_string(), location: #location } }
        }
        AuthStrategy::OAuth2(config) => {
            let client_auth = format_ident!("{}", format!("{:?}", config.client_auth()));
            let token_url = config.token_url();
            let revocation_url = option_string_tokens(config.revocation_url());
            let scopes = string_vec_tokens(config.default_scopes());
            let shared = quote! {
                token_url: #token_url.to_string(), revocation_url: #revocation_url,
                default_scopes: #scopes, client_auth: OAuth2ClientAuthMethod::#client_auth,
            };
            match config {
                OAuth2Config::AuthorizationCodePkce(grant) => {
                    let authorization_url = &grant.authorization_url;
                    let pkce = format_ident!("{}", format!("{:?}", grant.pkce));
                    quote! { AuthStrategy::OAuth2(OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
                        authorization_url: #authorization_url.to_string(), #shared
                        pkce: PkceRequirement::#pkce,
                    })) }
                }
                OAuth2Config::ClientCredentials(_) => {
                    quote! { AuthStrategy::OAuth2(OAuth2Config::ClientCredentials(ClientCredentialsGrant {
                        #shared
                    })) }
                }
                OAuth2Config::DeviceCode(grant) => {
                    let device_authorization_url = &grant.device_authorization_url;
                    quote! { AuthStrategy::OAuth2(OAuth2Config::DeviceCode(DeviceCodeGrant {
                        device_authorization_url: #device_authorization_url.to_string(), #shared
                    })) }
                }
                _ => unreachable!("unsupported imported OAuth2 grant"),
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

fn cross_origin_tokens(policy: &CrossOrigin) -> TokenStream {
    match policy {
        CrossOrigin::SameOrigin => quote! { CrossOrigin::same_origin() },
        CrossOrigin::AllowOrigins(origins) if origins.is_empty() => quote! { CrossOrigin::allow_origins(Vec::<String>::new()) },
        CrossOrigin::AllowOrigins(origins) => {
            let origins = origins.iter();
            quote! { CrossOrigin::allow_origins([#(#origins),*]) }
        }
        CrossOrigin::AllowWithoutCredentials => quote! { CrossOrigin::allow_without_credentials() },
        CrossOrigin::DangerAllowAny => quote! { CrossOrigin::danger_allow_any() },
        CrossOrigin::Deny => quote! { CrossOrigin::deny() },
    }
}

fn string_vec_tokens(values: &[String]) -> TokenStream {
    let values = values.iter();
    quote! { Vec::<String>::from([#(#values.to_string()),*]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::EndpointParams;

    fn sample_api() -> RestApi {
        {
            let mut api = schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), "Sample".to_string(), "https://api.example.com/v1".to_string())
                .describes("Sample API".to_string())
                .authentication(AuthStrategy::BearerToken { header: None }, vec!["SAMPLE_TOKEN".to_string()])
                .build().expect("valid API fixture");
            api.docs_url = Some("https://docs.example.com".to_string());
            api.endpoints = vec![{
                let mut endpoint = Endpoint::new("ListItems".to_string(), RestMethod::Get, "/items".to_string())
                    .describes("List items".to_string())
                    .returns_empty()
                    .build().expect("valid endpoint fixture");
                endpoint.response = ApiResponse::json_type("ListItemsResponse");
                endpoint
            }];
            api
        }
    }

    #[test]
    fn emits_parseable_rust() {
        let tokens = assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".to_string()], None);
        assert!(validate_code(&tokens).is_ok());
    }

    #[test]
    fn emits_the_conventional_function_name() {
        let tokens = assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".to_string()], None);
        let code = format_code(&validate_code(&tokens).unwrap());

        assert!(code.contains("pub fn define_sample_api() -> RestApi"));
    }

    #[test]
    fn emits_typed_builders_and_one_ordered_context() {
        let code = emit(None);
        assert!(code.contains(r#"Endpoint::new("ListItems", RestMethod::Get, "/items")"#));
        assert!(code.contains(".returns_json::<ListItemsResponse>(ctx)"));
        assert!(code.contains("ctx.register::<ListItemsResponse>()?"));
        assert!(code.contains("RestApi::builder(ctx,"));
        assert!(!code.contains("let api = RestApi {"));
        assert!(!code.contains("Endpoint {"));
        assert!(!code.contains("Schema::new"));
        assert!(code.contains("assemble_module_group"));
    }

    #[test]
    fn emitted_builders_preserve_runtime_and_parameter_forms() {
        let mut api = sample_api();
        api.cross_origin = CrossOrigin::allow_origins(["https://uploads.example.com"]);
        api.request_id_header = Some("x-request-id".into());
        api.secret_names = vec!["X-Pin".into()];
        api.endpoints[0].cross_origin = Some(CrossOrigin::deny());
        api.endpoints[0].params = Some(EndpointParams {
            query: vec![ParamDef::new("ref", QueryParamType::String).rust_name("git_ref").secret()],
            ..Default::default()
        });
        let code = format_code(&validate_code(&assemble_definition_module(&api, "sample", &["ListItemsResponse".into()], None)).unwrap());
        assert!(code.contains(".cross_origin(CrossOrigin::deny())"));
        assert!(code.contains(r#".request_id_header("x-request-id")"#));
        assert!(code.contains(r#".secret_header("X-Pin")"#));
        assert!(code.contains(r#"rust_name: Some("git_ref".to_string())"#));
        assert!(code.contains("secret: true"));
    }

    fn emit(overlay: Option<&str>) -> String {
        let overlay = overlay.map(|value| parse_overlay_path(value).unwrap());
        format_code(&validate_code(&assemble_definition_module(&sample_api(), "sample", &["ListItemsResponse".into()], overlay.as_ref())).unwrap())
    }

    #[test]
    fn configured_overlay_is_generic_and_runs_after_builder_validation() {
        let code = emit(Some("crate::sample_pagination::apply"));
        assert_eq!(code.matches("crate::sample_pagination::apply(api)").count(), 1);
        assert!(code.contains("Ok(crate::sample_pagination::apply(api))"));
        assert!(code.contains("--overlay crate::sample_pagination::apply`"));
        assert!(!emit(None).contains("--overlay"));
    }

    #[test]
    fn imported_query_cookie_and_oauth_authentication_are_not_lost() {
        for location in [ApiKeyLocation::Query, ApiKeyLocation::Cookie] {
            let code = render_expr(auth_strategy_tokens(&AuthStrategy::ApiKeyParam { name: "key".into(), location }));
            assert!(code.contains("AuthStrategy::ApiKeyParam"));
            assert!(code.contains("name: \"key\".to_string()"));
        }
        let config = OAuth2Config::ClientCredentials(schematic_define::ClientCredentialsGrant {
            token_url: "https://example.com/token".into(), revocation_url: None,
            default_scopes: vec!["read".into()],
            client_auth: schematic_define::OAuth2ClientAuthMethod::ClientSecretPost,
        });
        let code = render_expr(auth_strategy_tokens(&AuthStrategy::OAuth2(config)));
        assert!(code.contains("OAuth2Config::ClientCredentials(ClientCredentialsGrant"));
        assert!(code.contains("https://example.com/token"));
        assert!(code.contains("ClientSecretPost"));
        assert!(!code.contains("authorization_url") && !code.contains("pkce"));

        let config = OAuth2Config::AuthorizationCodePkce(schematic_define::AuthorizationCodeGrant {
            authorization_url: "https://example.com/authorize".into(),
            token_url: "https://example.com/token".into(),
            revocation_url: Some("https://example.com/revoke".into()),
            default_scopes: vec![],
            pkce: schematic_define::PkceRequirement::Supported,
            client_auth: schematic_define::OAuth2ClientAuthMethod::None,
        });
        let code = render_expr(auth_strategy_tokens(&AuthStrategy::OAuth2(config)));
        assert!(code.contains("OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant"));
        assert!(code.contains("authorization_url: \"https://example.com/authorize\".to_string()"));
        assert!(code.contains("Some(\"https://example.com/revoke\".to_string())"));
        assert!(code.contains("PkceRequirement::Supported"));
    }

    #[test]
    fn plain_function_paths_are_accepted_as_overlays() {
        for value in [
            "crate::openai_pagination::apply",
            "::extern_crate::apply",
            "super::apply",
            "apply",
        ] {
            let path = parse_overlay_path(value).unwrap_or_else(|e| panic!("{value}: {e}"));
            assert_eq!(path_text(&path), value);
        }
    }

    #[test]
    fn non_paths_and_generic_paths_are_rejected_as_overlays() {
        for value in [
            "",
            "   ",
            "not a path",
            "crate::x::<T>",
            "crate::x::apply::<RestApi>",
            "crate::x::apply()",
            "crate::x;",
        ] {
            let Err(error) = parse_overlay_path(value) else { panic!("{value:?} was accepted") };
            assert!(
                matches!(&error, GeneratorError::ConfigError(message) if message.contains("--overlay")),
                "{value:?}: {error}"
            );
        }
    }

    #[test]
    fn an_invalid_overlay_writes_nothing() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let result = generate_definition_module(
            &sample_api(),
            "sample",
            &[],
            Some("crate::x::<T>"),
            temp_dir.path(),
            false,
        );

        assert!(result.is_err());
        assert!(!temp_dir.path().join("mod.rs").exists());
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

    #[test]
    fn emits_basic_with_its_ordered_environment_lists() {
        let mut api = sample_api();
        api.auth = AuthStrategy::Basic {
            username_env: schematic_define::EnvList::from_strs(&["USER", "LOGIN"]),
            password_env: schematic_define::EnvList::from_strs(&["PASS", "SECRET"]),
        };
        api.env_auth = vec![];
        let code = rest_api_tokens(&api, None).to_string().replace(' ', "");
        assert!(code.contains(r#"AuthStrategy::Basic{username_env:EnvList::new(Vec::<String>::from(["USER".to_string(),"LOGIN".to_string()])),password_env:EnvList::new(Vec::<String>::from(["PASS".to_string(),"SECRET".to_string()]))}"#), "{code}");
        assert!(!code.contains("env_username"));
    }
}
