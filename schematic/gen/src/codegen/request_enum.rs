//! Request enum generation for REST APIs.
//!
//! Generates a unified request enum that wraps all endpoint-specific request
//! structs, with `From` implementations for easy conversion.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// Generates the request enum for an API.
///
/// Creates an enum with one variant per endpoint, plus:
/// - `into_parts()` method that delegates to inner request structs
/// - `From<XxxRequest>` impl for each variant
///
/// ## Examples
///
/// For an API with three endpoints:
/// ```ignore
/// // Input API with endpoints: ListModels, RetrieveModel, DeleteModel
///
/// // Generated enum:
/// pub enum OpenAiRequest {
///     ListModels(ListModelsRequest),
///     RetrieveModel(RetrieveModelRequest),
///     DeleteModel(DeleteModelRequest),
/// }
///
/// impl OpenAiRequest {
///     pub fn into_parts(self) -> Result<(&'static str, String, Option<String>, Vec<(String, String)>), SchematicError> {
///         match self {
///             Self::ListModels(req) => req.into_parts(),
///             Self::RetrieveModel(req) => req.into_parts(),
///             Self::DeleteModel(req) => req.into_parts(),
///         }
///     }
/// }
///
/// impl From<ListModelsRequest> for OpenAiRequest {
///     fn from(req: ListModelsRequest) -> Self {
///         Self::ListModels(req)
///     }
/// }
/// // ... individual From impls for each request struct
/// ```
pub fn generate_request_enum(api: &RestApi) -> TokenStream {
    let enum_name = format_ident!("{}Request", api.name);
    let enum_doc = format!("Request enum for {} API.", api.name);
    let enum_doc_detail = "Each variant wraps a strongly-typed request struct.";

    // Generate enum variants
    let variants = generate_enum_variants(api);

    // Generate into_parts match arms
    let match_arms = generate_match_arms(api);

    // Generate individual From implementations
    let from_impls = generate_from_impls(api, &enum_name);

    quote! {
        #[doc = #enum_doc]
        ///
        #[doc = #enum_doc_detail]
        pub enum #enum_name {
            #variants
        }

        impl #enum_name {
            /// Converts the request into (method, path, body, headers) parts.
            ///
            /// Delegates to the inner request struct's `into_parts()` method.
            ///
            /// ## Errors
            ///
            /// Returns `SchematicError::SerializationError` if the request body
            /// fails to serialize to JSON.
            pub fn into_parts(self) -> Result<(&'static str, String, Option<String>, Vec<(String, String)>), SchematicError> {
                match self {
                    #match_arms
                }
            }
        }

        #from_impls
    }
}

/// Generates enum variant declarations.
fn generate_enum_variants(api: &RestApi) -> TokenStream {
    let variants = api.endpoints.iter().map(|endpoint| {
        let variant_name = format_ident!("{}", endpoint.id);
        let struct_name = format_ident!("{}Request", endpoint.id);
        let doc = &endpoint.description;

        quote! {
            #[doc = #doc]
            #variant_name(#struct_name),
        }
    });

    quote! { #(#variants)* }
}

/// Generates match arms for `into_parts()` method.
fn generate_match_arms(api: &RestApi) -> TokenStream {
    let arms = api.endpoints.iter().map(|endpoint| {
        let variant_name = format_ident!("{}", endpoint.id);

        quote! {
            Self::#variant_name(req) => req.into_parts(),
        }
    });

    quote! { #(#arms)* }
}

/// Generates individual `From` implementations for each request struct.
fn generate_from_impls(api: &RestApi, enum_name: &proc_macro2::Ident) -> TokenStream {
    let impls = api.endpoints.iter().map(|endpoint| {
        let variant_name = format_ident!("{}", endpoint.id);
        let struct_name = format_ident!("{}Request", endpoint.id);

        quote! {
            impl From<#struct_name> for #enum_name {
                fn from(req: #struct_name) -> Self {
                    Self::#variant_name(req)
                }
            }
        }
    });

    quote! { #(#impls)* }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::{format_generated_code, validate_generated_code};
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestMethod, Schema};

    fn make_api(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
        RestApi {
            name: name.to_string(),
            description: format!("{} API", name),
            base_url: "https://api.example.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints,
        }
    }

    fn make_endpoint(
        id: &str,
        method: RestMethod,
        path: &str,
        request: Option<Schema>,
    ) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method,
            path: path.to_string(),
            description: format!("{} endpoint", id),
            request,
            response: ApiResponse::json_type("TestResponse"),
            headers: vec![],
        }
    }

    #[test]
    fn generate_enum_single_variant() {
        let api = make_api(
            "TestApi",
            vec![make_endpoint("ListItems", RestMethod::Get, "/items", None)],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check enum declaration
        assert!(code.contains("pub enum TestApiRequest"));
        assert!(code.contains("ListItems(ListItemsRequest)"));

        // Check into_parts method
        assert!(code.contains("fn into_parts(self)") || code.contains("fn into_parts(\n"));
        assert!(code.contains("Result<"));
        assert!(code.contains("Vec<(String, String)>"));
        assert!(code.contains("SchematicError"));
        assert!(code.contains("Self::ListItems(req) => req.into_parts()"));

        // Check From impl
        assert!(code.contains("impl From<ListItemsRequest> for TestApiRequest"));
        assert!(code.contains("Self::ListItems(req)"));
    }

    #[test]
    fn generate_enum_two_variants() {
        let api = make_api(
            "Simple",
            vec![
                make_endpoint("Create", RestMethod::Post, "/create", None),
                make_endpoint("Delete", RestMethod::Delete, "/delete/{id}", None),
            ],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check both variants
        assert!(code.contains("Create(CreateRequest)"));
        assert!(code.contains("Delete(DeleteRequest)"));

        // Check both match arms
        assert!(code.contains("Self::Create(req) => req.into_parts()"));
        assert!(code.contains("Self::Delete(req) => req.into_parts()"));

        // Check both From impls are INDIVIDUAL (not in a vec)
        assert!(code.contains("impl From<CreateRequest> for SimpleRequest"));
        assert!(code.contains("impl From<DeleteRequest> for SimpleRequest"));
    }

    #[test]
    fn generate_enum_three_variants() {
        let api = make_api(
            "OpenAi",
            vec![
                make_endpoint("ListModels", RestMethod::Get, "/models", None),
                make_endpoint("RetrieveModel", RestMethod::Get, "/models/{model}", None),
                make_endpoint("DeleteModel", RestMethod::Delete, "/models/{model}", None),
            ],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check enum name includes API name
        assert!(code.contains("pub enum OpenAiRequest"));

        // Check all three variants
        assert!(code.contains("ListModels(ListModelsRequest)"));
        assert!(code.contains("RetrieveModel(RetrieveModelRequest)"));
        assert!(code.contains("DeleteModel(DeleteModelRequest)"));

        // Check all three From impls
        assert!(code.contains("impl From<ListModelsRequest> for OpenAiRequest"));
        assert!(code.contains("impl From<RetrieveModelRequest> for OpenAiRequest"));
        assert!(code.contains("impl From<DeleteModelRequest> for OpenAiRequest"));
    }

    #[test]
    fn generate_enum_with_body_endpoints() {
        let api = make_api(
            "Chat",
            vec![
                make_endpoint(
                    "CreateCompletion",
                    RestMethod::Post,
                    "/completions",
                    Some(Schema::new("CreateCompletionBody")),
                ),
                make_endpoint("ListCompletions", RestMethod::Get, "/completions", None),
            ],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Enum should reference request structs, not body types
        assert!(code.contains("CreateCompletion(CreateCompletionRequest)"));
        assert!(code.contains("ListCompletions(ListCompletionsRequest)"));
    }

    #[test]
    fn generate_enum_validates_syntax() {
        let api = make_api(
            "Test",
            vec![
                make_endpoint("Get", RestMethod::Get, "/get", None),
                make_endpoint("Post", RestMethod::Post, "/post", None),
                make_endpoint("Put", RestMethod::Put, "/put", None),
            ],
        );

        let tokens = generate_request_enum(&api);
        assert!(validate_generated_code(&tokens).is_ok());
    }

    #[test]
    fn generate_enum_doc_comments() {
        let api = make_api(
            "Documented",
            vec![Endpoint {
                id: "GetUser".to_string(),
                method: RestMethod::Get,
                path: "/users/{id}".to_string(),
                description: "Retrieve a user by ID".to_string(),
                request: None,
                response: ApiResponse::json_type("User"),
                headers: vec![],
            }],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check enum-level docs
        assert!(code.contains("Request enum for Documented API"));
        assert!(code.contains("Each variant wraps a strongly-typed request struct"));

        // Check variant-level doc (from endpoint description)
        assert!(code.contains("Retrieve a user by ID"));
    }

    #[test]
    fn from_impls_are_individual_not_combined() {
        let api = make_api(
            "Individual",
            vec![
                make_endpoint("A", RestMethod::Get, "/a", None),
                make_endpoint("B", RestMethod::Get, "/b", None),
            ],
        );

        let tokens = generate_request_enum(&api);
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Count number of "impl From" occurrences - should match endpoint count
        let from_count = code.matches("impl From<").count();
        assert_eq!(
            from_count, 2,
            "Expected 2 individual From impls, got {}",
            from_count
        );

        // Ensure no array patterns in From impls
        assert!(!code.contains("[impl"));

        // Each From impl should be individual, not combined with Vec
        assert!(code.contains("impl From<ARequest> for IndividualRequest"));
        assert!(code.contains("impl From<BRequest> for IndividualRequest"));
    }

    #[test]
    fn generate_enum_empty_api_produces_valid_code() {
        let api = make_api("Empty", vec![]);

        let tokens = generate_request_enum(&api);

        // Should still produce valid (if empty) enum
        assert!(validate_generated_code(&tokens).is_ok());
        let code = format_generated_code(&tokens).expect("Failed to format code");
        assert!(code.contains("pub enum EmptyRequest"));
    }
}
