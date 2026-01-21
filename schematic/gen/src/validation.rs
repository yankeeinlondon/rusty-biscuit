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
//!     env_auth: vec![],
//!     env_username: None,
//!     headers: vec![],
//!     endpoints: vec![],
//!     module_path: None,
//!     request_suffix: None,
//! };
//!
//! assert!(validate_api(&api).is_ok());
//! ```

use schematic_define::{ApiRequest, RestApi};

use crate::errors::GeneratorError;

/// Default suffix appended to endpoint IDs to form request struct names.
const DEFAULT_REQUEST_SUFFIX: &str = "Request";

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
///     env_auth: vec![],
///     env_username: None,
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
///         },
///     ],
///     module_path: None,
///     request_suffix: None,
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
    let suffix = api.request_suffix.as_deref().unwrap_or(DEFAULT_REQUEST_SUFFIX);

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

    // Check 2: Validate body type names don't conflict with wrapper struct names
    for endpoint in &api.endpoints {
        // Only JSON requests have typed body fields that could conflict
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
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
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
        }];

        // FormData doesn't have a body type name, so no collision possible
        assert!(validate_api(&api).is_ok());
    }
}
