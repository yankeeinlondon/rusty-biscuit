//! API response type definitions.
//!
//! This module defines the types of responses an API endpoint can return.
//! The response type determines how the generated client deserializes
//! the response body.

use crate::schema::Schema;

/// Describes the expected response from an API endpoint.
///
/// Each variant indicates a different response format, which affects
/// how the generated client handles the response body.
///
/// ## Examples
///
/// JSON response (most common):
///
/// ```
/// use schematic_define::ApiResponse;
///
/// // Using the convenience method
/// let response = ApiResponse::json_type("UserResponse");
/// ```
///
/// Text response:
///
/// ```
/// use schematic_define::ApiResponse;
///
/// let response = ApiResponse::Text;
/// ```
///
/// Empty response (for DELETE or 204 responses):
///
/// ```
/// use schematic_define::ApiResponse;
///
/// let response = ApiResponse::Empty;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiResponse {
    /// JSON response with a typed schema.
    ///
    /// The response body will be deserialized as JSON into the specified type.
    /// This is the most common response type for REST APIs.
    Json(Schema),

    /// Plain text response.
    ///
    /// The response body is returned as a `String`.
    Text,

    /// Binary data (bytes).
    ///
    /// The response body is returned as `Vec<u8>`. Use for file downloads,
    /// images, or other binary content.
    Binary,

    /// No response body expected.
    ///
    /// Used for endpoints that return 204 No Content or where the response
    /// body should be ignored.
    Empty,
}

impl ApiResponse {
    /// Creates a JSON response with the given schema.
    ///
    /// Use this when you have a pre-built [`Schema`] with a module path.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiResponse, Schema};
    ///
    /// let schema = Schema::with_path("User", "crate::models");
    /// let response = ApiResponse::json(schema);
    /// ```
    pub fn json(schema: Schema) -> Self {
        Self::Json(schema)
    }

    /// Creates a JSON response with just a type name.
    ///
    /// This is the most common way to specify a JSON response. The type
    /// name should match a struct in the generated or imported code.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::ApiResponse;
    ///
    /// let response = ApiResponse::json_type("ListModelsResponse");
    ///
    /// // Verify the schema was created correctly
    /// if let ApiResponse::Json(schema) = response {
    ///     assert_eq!(schema.type_name, "ListModelsResponse");
    /// }
    /// ```
    pub fn json_type(type_name: impl Into<String>) -> Self {
        Self::Json(Schema::new(type_name))
    }
}
