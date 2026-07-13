//! Validation module for API definitions.
//!
//! This module provides pre-generation validation to detect issues like
//! naming collisions before code generation begins. Running validation
//! early prevents confusing errors during code generation.
//!
//! ## Validation Checks
//!
//! - **Naming collisions**: Ensures body type names don't conflict with
//!   generated request struct names
//! - **Request suffix format**: Validates the request suffix is alphanumeric
//!
//! ## Examples
//!
//! ```
//! use schematic_define::{RestApi, Endpoint, RestMethod, ApiRequest, ApiResponse, AuthStrategy};
//! use schematic_gen::validation::validate_api;
//!
//! let api = RestApi {
//!     name: "TestApi".to_string(),
//!     description: "Test API".to_string(),
//!     base_url: "https://api.test.com".to_string(),
//!     docs_url: None,
//!     auth: AuthStrategy::None,
//!     auth_policy: None,
//!     env_auth: vec![],
//!     env_username: None,
//!     env_mapping: None,
//!     headers: vec![],
//!     endpoints: vec![],
//!     module_path: None,
//!     request_suffix: None,
//!     version: None,
//! };
//!
//! assert!(validate_api(&api).is_ok());
//! ```

use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
use schematic_define::{ApiRequest, RestApi};

use crate::errors::GeneratorError;

/// Default suffix appended to endpoint IDs to form request struct names.
const DEFAULT_REQUEST_SUFFIX: &str = "Request";

/// Checks that `name` can be used directly as a Rust type/variant identifier.
///
/// Emitters feed the API name and endpoint IDs straight into `format_ident!`,
/// which panics on an empty string, a leading digit, punctuation, or a Rust
/// keyword. Parsing as a [`syn::Ident`] reproduces exactly that acceptance test
/// without the panic.
fn ensure_valid_type_ident(context: &str, name: &str) -> Result<(), GeneratorError> {
    if syn::parse_str::<syn::Ident>(name).is_ok() {
        return Ok(());
    }

    let reason = if name.is_empty() {
        "name is empty".to_string()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        "a Rust identifier cannot start with a digit".to_string()
    } else {
        "name is a Rust keyword or contains characters that are not valid in an identifier"
            .to_string()
    };

    Err(GeneratorError::InvalidIdentifier {
        context: context.to_string(),
        name: name.to_string(),
        reason,
    })
}

/// Checks that a path or query parameter name yields a usable, distinct field
/// identifier after sanitization.
///
/// Parameter names are routed through [`sanitize_rust_field_ident`] before
/// becoming struct fields, so unlike type identifiers they cannot panic. The
/// only failure worth flagging is a name that sanitizes to nothing meaningful
/// (the sanitizer's `field` fallback), which would silently rename the
/// parameter to an opaque `field`.
fn ensure_valid_param_ident(context: &str, name: &str) -> Result<(), GeneratorError> {
    if sanitize_rust_field_ident(name) == "field" && name != "field" {
        return Err(GeneratorError::InvalidIdentifier {
            context: context.to_string(),
            name: name.to_string(),
            reason: "name has no representable identifier characters".to_string(),
        });
    }
    Ok(())
}

/// Validates an API definition before code generation.
///
/// Performs the following checks:
///
/// 1. **Request suffix validation**: If a custom `request_suffix` is provided,
///    it must be alphanumeric (letters and numbers only).
///
/// 2. **Naming collision detection**: For each endpoint with a JSON body type,
///    checks that the body type name doesn't match the generated wrapper struct
///    name (`{endpoint_id}{suffix}`).
///
/// ## Examples
///
/// Valid API passes validation:
///
/// ```
/// use schematic_define::{RestApi, Endpoint, RestMethod, ApiRequest, ApiResponse, AuthStrategy};
/// use schematic_gen::validation::validate_api;
///
/// let api = RestApi {
///     name: "TestApi".to_string(),
///     description: "Test".to_string(),
///     base_url: "https://api.test.com".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     env_mapping: None,
///     headers: vec![],
///     endpoints: vec![
///         Endpoint {
///             id: "CreateUser".to_string(),
///             method: RestMethod::Post,
///             path: "/users".to_string(),
///             description: "Create user".to_string(),
///             request: Some(ApiRequest::json_type("CreateUserBody")), // Different from CreateUserRequest
///             response: ApiResponse::json_type("User"),
///             headers: vec![],
///             params: None,
///             oauth_scopes: None,
///         },
///     ],
///     module_path: None,
///     request_suffix: None,
///     version: None,
/// };
///
/// assert!(validate_api(&api).is_ok());
/// ```
///
/// ## Errors
///
/// Returns `GeneratorError::InvalidRequestSuffix` if the suffix contains
/// non-alphanumeric characters.
///
/// Returns `GeneratorError::NamingCollision` if a body type name matches
/// the generated wrapper struct name.
pub fn validate_api(api: &RestApi) -> Result<(), GeneratorError> {
    // Get the effective suffix (custom or default)
    let suffix = api
        .request_suffix
        .as_deref()
        .unwrap_or(DEFAULT_REQUEST_SUFFIX);

    // Check 1: Validate request_suffix is alphanumeric (if provided)
    if let Some(ref custom_suffix) = api.request_suffix {
        if !custom_suffix.chars().all(|c| c.is_alphanumeric()) {
            return Err(GeneratorError::InvalidRequestSuffix {
                suffix: custom_suffix.clone(),
                reason: "suffix must contain only alphanumeric characters (letters and numbers)"
                    .to_string(),
            });
        }

        if custom_suffix.is_empty() {
            return Err(GeneratorError::InvalidRequestSuffix {
                suffix: custom_suffix.clone(),
                reason: "suffix cannot be empty".to_string(),
            });
        }
    }

    // Check 2: The API name becomes a type identifier (e.g. `{name}Request`).
    ensure_valid_type_ident("API name", &api.name)?;

    for endpoint in &api.endpoints {
        // Check 3: Endpoint IDs become type and variant identifiers.
        ensure_valid_type_ident("endpoint ID", &endpoint.id)?;

        // Check 4: Path and query parameter names must sanitize to a usable
        // field identifier rather than the opaque `field` fallback.
        let path_params = crate::parser::extract_path_params(&endpoint.path);
        for param in path_params {
            ensure_valid_param_ident("path parameter", param)?;
        }
        if let Some(params) = &endpoint.params {
            for query in &params.query {
                ensure_valid_param_ident("query parameter", &query.name)?;
            }
        }

        // Check 5: Validate body type names don't conflict with wrapper struct
        // names. Only JSON requests have typed body fields that could conflict.
        if let Some(ApiRequest::Json(schema)) = &endpoint.request {
            let wrapper_name = format!("{}{}", endpoint.id, suffix);

            if schema.type_name == wrapper_name {
                // Suggest a renamed body type
                let suggestion = format!("{}Body", schema.type_name.trim_end_matches(suffix));

                return Err(GeneratorError::NamingCollision {
                    endpoint_id: endpoint.id.clone(),
                    body_type: schema.type_name.clone(),
                    suggestion,
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestMethod};

    /// Helper to create a minimal API for testing.
    fn make_test_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    /// Helper to create an endpoint with a JSON body.
    fn make_endpoint_with_body(id: &str, body_type: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method: RestMethod::Post,
            path: format!("/{}", id.to_lowercase()),
            description: format!("Test endpoint {}", id),
            request: Some(ApiRequest::json_type(body_type)),
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }
    }

    /// Helper to create an endpoint without a body.
    fn make_endpoint_no_body(id: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method: RestMethod::Get,
            path: format!("/{}", id.to_lowercase()),
            description: format!("Test endpoint {}", id),
            request: None,
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }
    }

    #[test]
    fn valid_api_passes_validation() {
        let mut api = make_test_api();
        api.endpoints = vec![
            make_endpoint_with_body("CreateUser", "CreateUserBody"),
            make_endpoint_with_body("UpdateUser", "UpdateUserPayload"),
        ];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn body_type_collision_is_detected_with_helpful_error() {
        let mut api = make_test_api();
        // Body type "CreateUserRequest" collides with endpoint "CreateUser" + suffix "Request"
        api.endpoints = vec![make_endpoint_with_body("CreateUser", "CreateUserRequest")];

        let result = validate_api(&api);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            GeneratorError::NamingCollision {
                endpoint_id,
                body_type,
                suggestion,
            } => {
                assert_eq!(endpoint_id, "CreateUser");
                assert_eq!(body_type, "CreateUserRequest");
                assert_eq!(suggestion, "CreateUserBody");
            }
            other => panic!("Expected NamingCollision, got: {:?}", other),
        }
    }

    #[test]
    fn invalid_request_suffix_is_rejected() {
        let mut api = make_test_api();
        api.request_suffix = Some("Request-Type".to_string()); // Contains hyphen

        let result = validate_api(&api);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            GeneratorError::InvalidRequestSuffix { suffix, reason } => {
                assert_eq!(suffix, "Request-Type");
                assert!(reason.contains("alphanumeric"));
            }
            other => panic!("Expected InvalidRequestSuffix, got: {:?}", other),
        }
    }

    #[test]
    fn api_with_no_body_types_passes() {
        let mut api = make_test_api();
        api.endpoints = vec![
            make_endpoint_no_body("ListUsers"),
            make_endpoint_no_body("GetUser"),
        ];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn multiple_endpoints_with_unique_names_all_pass() {
        let mut api = make_test_api();
        api.endpoints = vec![
            make_endpoint_with_body("CreateUser", "NewUserData"),
            make_endpoint_with_body("UpdateUser", "UserUpdate"),
            make_endpoint_with_body("DeleteUser", "DeleteParams"),
            make_endpoint_no_body("ListUsers"),
            make_endpoint_no_body("GetUser"),
        ];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn empty_suffix_is_rejected() {
        let mut api = make_test_api();
        api.request_suffix = Some(String::new());

        let result = validate_api(&api);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            GeneratorError::InvalidRequestSuffix { suffix, reason } => {
                assert!(suffix.is_empty());
                assert!(reason.contains("empty"));
            }
            other => panic!("Expected InvalidRequestSuffix, got: {:?}", other),
        }
    }

    #[test]
    fn suffix_with_spaces_is_rejected() {
        let mut api = make_test_api();
        api.request_suffix = Some("Request Type".to_string());

        let result = validate_api(&api);
        assert!(result.is_err());

        match result.unwrap_err() {
            GeneratorError::InvalidRequestSuffix { suffix, .. } => {
                assert_eq!(suffix, "Request Type");
            }
            other => panic!("Expected InvalidRequestSuffix, got: {:?}", other),
        }
    }

    #[test]
    fn suffix_with_underscores_is_rejected() {
        let mut api = make_test_api();
        api.request_suffix = Some("Request_Type".to_string());

        let result = validate_api(&api);
        assert!(result.is_err());

        match result.unwrap_err() {
            GeneratorError::InvalidRequestSuffix { suffix, .. } => {
                assert_eq!(suffix, "Request_Type");
            }
            other => panic!("Expected InvalidRequestSuffix, got: {:?}", other),
        }
    }

    #[test]
    fn valid_alphanumeric_suffix_passes() {
        let mut api = make_test_api();
        api.request_suffix = Some("Params".to_string());
        api.endpoints = vec![make_endpoint_with_body("CreateUser", "CreateUserBody")];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn collision_detected_with_custom_suffix() {
        let mut api = make_test_api();
        api.request_suffix = Some("Params".to_string());
        // Body type "CreateUserParams" collides with endpoint "CreateUser" + suffix "Params"
        api.endpoints = vec![make_endpoint_with_body("CreateUser", "CreateUserParams")];

        let result = validate_api(&api);
        assert!(result.is_err());

        match result.unwrap_err() {
            GeneratorError::NamingCollision {
                endpoint_id,
                body_type,
                suggestion,
            } => {
                assert_eq!(endpoint_id, "CreateUser");
                assert_eq!(body_type, "CreateUserParams");
                assert_eq!(suggestion, "CreateUserBody");
            }
            other => panic!("Expected NamingCollision, got: {:?}", other),
        }
    }

    #[test]
    fn numeric_suffix_is_valid() {
        let mut api = make_test_api();
        api.request_suffix = Some("V2".to_string());
        api.endpoints = vec![make_endpoint_with_body("CreateUser", "CreateUserBody")];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn error_display_is_actionable() {
        let mut api = make_test_api();
        api.endpoints = vec![make_endpoint_with_body("CreateUser", "CreateUserRequest")];

        let err = validate_api(&api).unwrap_err();
        let msg = err.to_string();

        // Error message should be actionable
        assert!(msg.contains("CreateUser"));
        assert!(msg.contains("CreateUserRequest"));
        assert!(msg.contains("CreateUserBody"));
        assert!(msg.contains("Suggestion") || msg.contains("rename"));
    }

    #[test]
    fn endpoint_id_with_leading_digit_is_rejected() {
        let mut api = make_test_api();
        api.endpoints = vec![make_endpoint_no_body("2fa")];

        match validate_api(&api).unwrap_err() {
            GeneratorError::InvalidIdentifier {
                context,
                name,
                reason,
            } => {
                assert_eq!(context, "endpoint ID");
                assert_eq!(name, "2fa");
                assert!(reason.contains("digit"));
            }
            other => panic!("Expected InvalidIdentifier, got: {other:?}"),
        }
    }

    #[test]
    fn endpoint_id_with_punctuation_is_rejected() {
        let mut api = make_test_api();
        // A raw operationId like "list-models" would panic in `format_ident!`.
        api.endpoints = vec![make_endpoint_no_body("list-models")];

        assert!(matches!(
            validate_api(&api),
            Err(GeneratorError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn keyword_endpoint_id_is_rejected() {
        let mut api = make_test_api();
        api.endpoints = vec![make_endpoint_no_body("struct")];

        assert!(matches!(
            validate_api(&api),
            Err(GeneratorError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn invalid_api_name_is_rejected() {
        let mut api = make_test_api();
        api.name = "9Lives".to_string();

        match validate_api(&api).unwrap_err() {
            GeneratorError::InvalidIdentifier { context, .. } => {
                assert_eq!(context, "API name");
            }
            other => panic!("Expected InvalidIdentifier, got: {other:?}"),
        }
    }

    #[test]
    fn unrepresentable_query_param_name_is_rejected() {
        use schematic_define::params::{EndpointParams, ParamStyle, QueryParamType};
        use schematic_define::ParamDef;

        let mut api = make_test_api();
        let mut endpoint = make_endpoint_no_body("ListItems");
        endpoint.params = Some(EndpointParams {
            query: vec![ParamDef {
                // An empty name sanitizes to the opaque `field` fallback.
                name: String::new(),
                required: false,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
            pagination: None,
            response_pagination: None,
        });
        api.endpoints = vec![endpoint];

        match validate_api(&api).unwrap_err() {
            GeneratorError::InvalidIdentifier { context, .. } => {
                assert_eq!(context, "query parameter");
            }
            other => panic!("Expected InvalidIdentifier, got: {other:?}"),
        }
    }

    #[test]
    fn special_char_param_names_that_sanitize_cleanly_pass() {
        use schematic_define::params::{EndpointParams, ParamStyle, QueryParamType};
        use schematic_define::ParamDef;

        let mut api = make_test_api();
        let mut endpoint = make_endpoint_no_body("ListItems");
        // `$top` sanitizes to `_top` — representable, so validation passes.
        endpoint.params = Some(EndpointParams {
            query: vec![ParamDef {
                name: "$top".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::Integer,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
            pagination: None,
            response_pagination: None,
        });
        api.endpoints = vec![endpoint];

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn form_data_request_does_not_cause_collision() {
        use schematic_define::FormField;

        let mut api = make_test_api();
        api.endpoints = vec![Endpoint {
            id: "UploadFile".to_string(),
            method: RestMethod::Post,
            path: "/upload".to_string(),
            description: "Upload file".to_string(),
            request: Some(ApiRequest::form_data(vec![FormField::file("document")])),
            response: ApiResponse::json_type("UploadResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }];

        // FormData doesn't have a body type name, so no collision possible
        assert!(validate_api(&api).is_ok());
    }
}
