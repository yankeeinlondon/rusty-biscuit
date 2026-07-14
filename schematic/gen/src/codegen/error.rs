//! Error type generation for schematic runtime.
//!
//! Generates the `SchematicError` enum that is used by generated API client code
//! for runtime error handling. The error type includes variants for HTTP errors,
//! JSON deserialization errors, API errors, and other runtime failures.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates the RequestParts type alias for the complex tuple return type.
///
/// This type alias simplifies the `into_parts()` method signature and avoids
/// clippy's `type_complexity` lint.
///
/// The tuple contains:
/// - HTTP method as a static string (e.g., "GET", "POST")
/// - URL path with parameters substituted
/// - Optional JSON body
/// - Headers as key-value pairs
pub fn generate_request_parts_type() -> TokenStream {
    quote! {
        /// The components of a prepared HTTP request.
        ///
        /// This tuple contains all information needed to execute an API request:
        /// - `0`: HTTP method (e.g., "GET", "POST", "DELETE")
        /// - `1`: URL path with path parameters substituted
        /// - `2`: Optional JSON request body
        /// - `3`: Additional headers as (name, value) pairs
        pub type RequestParts = (&'static str, String, Option<String>, Vec<(String, String)>);
    }
}

/// Generates the `ResponseKind` type describing how an endpoint's response is
/// decoded.
///
/// The generic `request::<T>()` method decodes as JSON; each request enum
/// reports its endpoint's [`ResponseKind`] so the JSON path can reject text,
/// binary, and empty endpoints with an actionable error instead of a spurious
/// JSON parse failure.
pub fn generate_response_kind_type() -> TokenStream {
    quote! {
        /// How a generated endpoint's response body is decoded.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ResponseKind {
            /// Decoded as JSON via `request::<T>()`.
            Json,
            /// Returned as text via `request_text()`.
            Text,
            /// Returned as raw bytes via `request_bytes()`.
            Binary,
            /// Discarded via `request_empty()`.
            Empty,
        }
    }
}

/// Generates the SchematicError enum for runtime errors.
///
/// This error type is used by generated API client code and provides variants
/// for all error conditions that can occur during API requests:
///
/// - `Http`: HTTP request failures (network errors, timeouts)
/// - `Json`: JSON deserialization failures
/// - `ApiError`: API returned non-success status codes
/// - `UnsupportedMethod`: Unknown HTTP method (should never occur with generated code)
/// - `SerializationError`: Request body serialization failures
///
/// ## Examples
///
/// ```text
/// let error_tokens = generate_error_type();
/// // Produces:
/// // #[derive(Debug, thiserror::Error)]
/// // pub enum SchematicError {
/// //     #[error("HTTP request failed: {0}")]
/// //     Http(#[from] reqwest::Error),
/// //     ...
/// // }
/// ```
///
/// ## Generated Code
///
/// The generated error type uses `thiserror::Error` derive macro for ergonomic
/// error handling. The `#[from]` attribute enables automatic conversion from
/// `reqwest::Error` and `serde_json::Error` via the `?` operator.
pub fn generate_error_type() -> TokenStream {
    quote! {
        /// Errors that can occur when making API requests.
        ///
        /// This enum captures all error conditions that may arise during
        /// API communication, including network failures, serialization
        /// issues, and API-level errors.
        ///
        /// ## Error Handling Examples
        ///
        /// ```text
        /// match client.request::<Response>(req).await {
        ///     Ok(response) => { /* success */ }
        ///     Err(SchematicError::AuthenticationRequired { explicit_methods, env_fallback_vars, .. }) => {
        ///         eprintln!("Accepted explicit auth: {:?}", explicit_methods);
        ///         eprintln!("Env fallback vars: {:?}", env_fallback_vars);
        ///     }
        ///     Err(SchematicError::ApiError { status: 429, body }) => {
        ///         // Rate limited - implement backoff
        ///         eprintln!("Rate limited: {}", body);
        ///     }
        ///     Err(SchematicError::ApiError { status, body }) => {
        ///         eprintln!("API error {}: {}", status, body);
        ///     }
        ///     Err(e) => return Err(e.into()),
        /// }
        /// ```
        #[derive(Debug, thiserror::Error)]
        pub enum SchematicError {
            /// HTTP request failed (network error, timeout, etc.).
            #[error("HTTP request failed: {0}")]
            Http(#[from] reqwest::Error),

            /// Failed to deserialize JSON response.
            #[error("JSON deserialization failed: {0}")]
            Json(#[from] serde_json::Error),

            /// API returned an error response (non-2xx status code).
            #[error("API error (status {status}): {body}")]
            ApiError {
                /// HTTP status code from the response.
                status: u16,
                /// Response body text (may contain error details from the API).
                body: String,
            },

            /// Unsupported HTTP method encountered.
            ///
            /// This error should never occur when using generated request types,
            /// as they always produce valid HTTP methods.
            #[error("Unsupported HTTP method: {0}")]
            UnsupportedMethod(String),

            /// Failed to serialize request body to JSON.
            #[error("Failed to serialize request body: {0}")]
            SerializationError(String),

            /// Missing authentication credentials.
            ///
            /// None of the configured environment variables contained a value.
            #[error("Missing credentials: none of the following environment variables are set: {env_vars:?}")]
            MissingCredential {
                /// The environment variable names that were checked.
                env_vars: Vec<String>,
            },

            /// Authentication is required but neither explicit credentials nor
            /// configured environment fallbacks were available.
            #[error("{message}")]
            AuthenticationRequired {
                /// Human-readable next-step guidance.
                message: String,
                /// Explicit credential types accepted by the client.
                explicit_methods: Vec<String>,
                /// Environment variables accepted as fallback sources.
                env_fallback_vars: Vec<String>,
            },

            /// Header validation or construction failed.
            ///
            /// This occurs when header names contain invalid characters or when
            /// required environment variables for header values are not set.
            #[error("Header error: {0}")]
            Header(#[from] schematic_define::HeaderError),

            /// The generic JSON `request()` was used on a non-JSON endpoint.
            ///
            /// Text, binary, and empty endpoints must be called through
            /// `request_text()`, `request_bytes()`, or `request_empty()`
            /// respectively. Routing them through `request::<T>()` would decode
            /// the body as JSON, so it is rejected up front with this error.
            #[error(
                "endpoint `{endpoint_id}` returns a {kind} response; call `{expected_method}` instead of the generic `request()`"
            )]
            WrongResponseDecoder {
                /// The endpoint that was called through the wrong decoder.
                endpoint_id: &'static str,
                /// The endpoint's declared response kind (e.g. "text").
                kind: &'static str,
                /// The convenience method that should be used instead.
                expected_method: &'static str,
            },

            /// Internal error in schematic runtime.
            ///
            /// This indicates a bug in the generated code or type system violation.
            /// Should never occur in normal operation.
            #[error("Internal error: {0}")]
            InternalError(String),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::{format_generated_code, validate_generated_code};

    #[test]
    fn generate_error_type_produces_valid_syntax() {
        let tokens = generate_error_type();
        assert!(
            validate_generated_code(&tokens).is_ok(),
            "Generated error type should be syntactically valid"
        );
    }

    #[test]
    fn generate_error_type_contains_all_variants() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check all variant names are present
        assert!(code.contains("Http("), "Missing Http variant");
        assert!(code.contains("Json("), "Missing Json variant");
        assert!(code.contains("ApiError {"), "Missing ApiError variant");
        assert!(
            code.contains("UnsupportedMethod("),
            "Missing UnsupportedMethod variant"
        );
        assert!(
            code.contains("SerializationError("),
            "Missing SerializationError variant"
        );
        assert!(
            code.contains("MissingCredential {"),
            "Missing MissingCredential variant"
        );
    }

    #[test]
    fn generate_error_type_has_thiserror_derive() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("thiserror::Error"),
            "Should derive thiserror::Error"
        );
    }

    #[test]
    fn generate_error_type_has_from_attributes() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check #[from] attributes for automatic conversions
        assert!(
            code.contains("#[from] reqwest::Error"),
            "Http variant should have #[from] reqwest::Error"
        );
        assert!(
            code.contains("#[from] serde_json::Error"),
            "Json variant should have #[from] serde_json::Error"
        );
    }

    #[test]
    fn generate_error_type_has_error_messages() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check error message attributes
        assert!(
            code.contains(r#"#[error("HTTP request failed: {0}")]"#),
            "Http variant should have error message"
        );
        assert!(
            code.contains(r#"#[error("JSON deserialization failed: {0}")]"#),
            "Json variant should have error message"
        );
        assert!(
            code.contains(r#"#[error("API error (status {status}): {body}")]"#),
            "ApiError variant should have error message"
        );
        assert!(
            code.contains(r#"#[error("Unsupported HTTP method: {0}")]"#),
            "UnsupportedMethod variant should have error message"
        );
        assert!(
            code.contains(r#"#[error("Failed to serialize request body: {0}")]"#),
            "SerializationError variant should have error message"
        );
    }

    #[test]
    fn generate_error_type_api_error_has_fields() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check ApiError struct fields
        assert!(
            code.contains("status: u16"),
            "ApiError should have status field"
        );
        assert!(
            code.contains("body: String"),
            "ApiError should have body field"
        );
    }

    #[test]
    fn generate_error_type_has_debug_derive() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("Debug"), "Should derive Debug");
    }

    #[test]
    fn generate_error_type_has_doc_comments() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that main enum has documentation
        assert!(
            code.contains("/// Errors that can occur when making API requests"),
            "Enum should have doc comment"
        );

        // Check that variants have documentation
        assert!(
            code.contains("/// HTTP request failed"),
            "Http variant should have doc comment"
        );
        assert!(
            code.contains("/// Failed to deserialize JSON response"),
            "Json variant should have doc comment"
        );
        assert!(
            code.contains("/// API returned an error response"),
            "ApiError variant should have doc comment"
        );
    }

    #[test]
    fn generate_error_type_enum_is_public() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("pub enum SchematicError"),
            "Enum should be public"
        );
    }

    #[test]
    fn generate_error_type_has_error_handling_examples() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that the Error Handling Examples section is present
        assert!(
            code.contains("## Error Handling Examples"),
            "Should have Error Handling Examples section"
        );

        // Check for AuthenticationRequired example
        assert!(
            code.contains("SchematicError::AuthenticationRequired"),
            "Should show AuthenticationRequired handling example"
        );

        // Check for rate limiting (429) example
        assert!(
            code.contains("SchematicError::ApiError { status: 429, body }"),
            "Should show rate limit (429) handling example"
        );

        // Check for generic ApiError example
        assert!(
            code.contains("SchematicError::ApiError { status, body }"),
            "Should show generic ApiError handling example"
        );

        // Check for fallback error handling
        assert!(
            code.contains("Err(e) => return Err(e.into())"),
            "Should show fallback error handling"
        );
    }

    #[test]
    fn generate_error_type_has_header_variant() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Check that Header variant exists with #[from] attribute
        assert!(
            code.contains("Header(#[from] schematic_define::HeaderError)"),
            "Should have Header variant with #[from] attribute"
        );

        // Check that Header variant has documentation
        assert!(
            code.contains("/// Header validation or construction failed"),
            "Header variant should have doc comment"
        );
    }

    #[test]
    fn generate_error_type_supports_from_conversion() {
        let tokens = generate_error_type();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        // Verify #[from] attributes enable automatic conversion via ? operator
        assert!(
            code.contains("#[from] reqwest::Error"),
            "reqwest::Error should have #[from] for automatic conversion"
        );
        assert!(
            code.contains("#[from] serde_json::Error"),
            "serde_json::Error should have #[from] for automatic conversion"
        );
        assert!(
            code.contains("#[from] schematic_define::HeaderError"),
            "HeaderError should have #[from] for automatic conversion"
        );
    }
}
