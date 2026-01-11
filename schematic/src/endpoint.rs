//! Endpoint definition with type-state builder pattern.
//!
//! The [`Endpoint`] struct represents a single REST API endpoint with its
//! method, path, and response format. The [`EndpointBuilder`] uses a type-state
//! pattern to ensure all required fields are set at compile time.

use std::marker::PhantomData;

use crate::method::RestMethod;
use crate::response::ResponseFormat;

/// Marker traits for builder state tracking.
pub mod builder_state {
    /// Marker for a field that has not been set.
    pub struct Missing;
    /// Marker for a field that has been set.
    pub struct Present;
}

use builder_state::{Missing, Present};

/// A REST API endpoint definition.
///
/// Endpoints are parameterized by their response format, enabling type-safe
/// response handling at compile time.
///
/// ## Type Parameters
///
/// - `F`: The [`ResponseFormat`] implementation for this endpoint's response.
///
/// ## Examples
///
/// ```rust,ignore
/// use api::{Endpoint, RestMethod};
/// use api::response::JsonFormat;
///
/// #[derive(serde::Deserialize)]
/// struct User { id: u64, name: String }
///
/// let endpoint: Endpoint<JsonFormat<User>> = Endpoint::builder()
///     .id("get_user")
///     .method(RestMethod::Get)
///     .path("/users/{id}")
///     .description("Retrieve a user by ID")
///     .build();
/// ```
#[derive(Debug)]
pub struct Endpoint<F: ResponseFormat> {
    /// Unique identifier for this endpoint.
    id: String,
    /// HTTP method for this endpoint.
    method: RestMethod,
    /// URL path template (may contain `{param}` placeholders).
    path: String,
    /// Optional description of what this endpoint does.
    description: Option<String>,
    /// Phantom data for the response format type.
    _format: PhantomData<F>,
}

// Manual Clone implementation - PhantomData<F> is always Clone
impl<F: ResponseFormat> Clone for Endpoint<F> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            method: self.method,
            path: self.path.clone(),
            description: self.description.clone(),
            _format: PhantomData,
        }
    }
}

impl<F: ResponseFormat> Endpoint<F> {
    /// Creates a new endpoint builder.
    pub fn builder() -> EndpointBuilder<Missing, Missing, Missing, F> {
        EndpointBuilder::new()
    }

    /// Returns the endpoint's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the HTTP method for this endpoint.
    pub fn method(&self) -> RestMethod {
        self.method
    }

    /// Returns the path template for this endpoint.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the optional description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the full URL by combining a base URL with this endpoint's path.
    ///
    /// Path parameters should be substituted before calling this method.
    pub fn full_url(&self, base_url: &url::Url) -> Result<url::Url, url::ParseError> {
        base_url.join(&self.path)
    }

    /// Substitutes path parameters in the template.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// let path = endpoint.substitute_params(&[("id", "123"), ("name", "alice")]);
    /// // "/users/{id}/posts/{name}" becomes "/users/123/posts/alice"
    /// ```
    pub fn substitute_params(&self, params: &[(&str, &str)]) -> String {
        let mut path = self.path.clone();
        for (key, value) in params {
            path = path.replace(&format!("{{{key}}}"), value);
        }
        path
    }

    /// Extracts path parameter names from the template.
    ///
    /// Returns parameter names in the order they appear in the path.
    pub fn path_params(&self) -> Vec<&str> {
        let mut params = Vec::new();
        let mut chars = self.path.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut param = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    param.push(chars.next().unwrap());
                }
                if !param.is_empty() {
                    // Find the param in the original string to return a reference
                    if let Some(start) = self.path.find(&format!("{{{param}}}")) {
                        let param_start = start + 1;
                        let param_end = param_start + param.len();
                        params.push(&self.path[param_start..param_end]);
                    }
                }
            }
        }

        params
    }
}

/// Type-state builder for [`Endpoint`].
///
/// The builder uses phantom type parameters to track which required fields
/// have been set, preventing construction until all required fields are present.
///
/// ## Type Parameters
///
/// - `Id`: State of the ID field (`Missing` or `Present`).
/// - `Method`: State of the method field (`Missing` or `Present`).
/// - `Path`: State of the path field (`Missing` or `Present`).
/// - `F`: The response format type.
pub struct EndpointBuilder<Id, Method, Path, F: ResponseFormat> {
    id: Option<String>,
    method: Option<RestMethod>,
    path: Option<String>,
    description: Option<String>,
    _phantom: PhantomData<(Id, Method, Path, F)>,
}

impl<F: ResponseFormat> EndpointBuilder<Missing, Missing, Missing, F> {
    /// Creates a new endpoint builder with no fields set.
    pub fn new() -> Self {
        Self {
            id: None,
            method: None,
            path: None,
            description: None,
            _phantom: PhantomData,
        }
    }
}

impl<F: ResponseFormat> Default for EndpointBuilder<Missing, Missing, Missing, F> {
    fn default() -> Self {
        Self::new()
    }
}

// ID setter - transitions Id from Missing to Present
impl<M, P, F: ResponseFormat> EndpointBuilder<Missing, M, P, F> {
    /// Sets the endpoint ID.
    ///
    /// The ID should be a unique identifier for this endpoint within the API.
    pub fn id(self, id: impl Into<String>) -> EndpointBuilder<Present, M, P, F> {
        EndpointBuilder {
            id: Some(id.into()),
            method: self.method,
            path: self.path,
            description: self.description,
            _phantom: PhantomData,
        }
    }
}

// Method setter - transitions Method from Missing to Present
impl<I, P, F: ResponseFormat> EndpointBuilder<I, Missing, P, F> {
    /// Sets the HTTP method for this endpoint.
    pub fn method(self, method: RestMethod) -> EndpointBuilder<I, Present, P, F> {
        EndpointBuilder {
            id: self.id,
            method: Some(method),
            path: self.path,
            description: self.description,
            _phantom: PhantomData,
        }
    }
}

// Path setter - transitions Path from Missing to Present
impl<I, M, F: ResponseFormat> EndpointBuilder<I, M, Missing, F> {
    /// Sets the URL path template.
    ///
    /// The path may contain parameter placeholders like `{id}` or `{name}`.
    pub fn path(self, path: impl Into<String>) -> EndpointBuilder<I, M, Present, F> {
        EndpointBuilder {
            id: self.id,
            method: self.method,
            path: Some(path.into()),
            description: self.description,
            _phantom: PhantomData,
        }
    }
}

// Description setter - available in any state
impl<I, M, P, F: ResponseFormat> EndpointBuilder<I, M, P, F> {
    /// Sets an optional description for this endpoint.
    pub fn description(self, description: impl Into<String>) -> Self {
        EndpointBuilder {
            description: Some(description.into()),
            ..self
        }
    }
}

// Build method - only available when all required fields are Present
impl<F: ResponseFormat> EndpointBuilder<Present, Present, Present, F> {
    /// Builds the endpoint.
    ///
    /// This method is only available when all required fields (id, method, path)
    /// have been set.
    pub fn build(self) -> Endpoint<F> {
        Endpoint {
            id: self.id.expect("id set via type state"),
            method: self.method.expect("method set via type state"),
            path: self.path.expect("path set via type state"),
            description: self.description,
            _format: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::JsonFormat;

    #[derive(Debug, serde::Deserialize)]
    struct TestResponse {
        data: String,
    }

    #[test]
    fn test_builder_basic() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("test_endpoint")
            .method(RestMethod::Get)
            .path("/api/test")
            .build();

        assert_eq!(endpoint.id(), "test_endpoint");
        assert_eq!(endpoint.method(), RestMethod::Get);
        assert_eq!(endpoint.path(), "/api/test");
        assert_eq!(endpoint.description(), None);
    }

    #[test]
    fn test_builder_with_description() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("test")
            .method(RestMethod::Post)
            .path("/api/create")
            .description("Creates a new resource")
            .build();

        assert_eq!(endpoint.description(), Some("Creates a new resource"));
    }

    #[test]
    fn test_builder_order_independence() {
        // Fields can be set in any order
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .path("/api/test")
            .description("Test endpoint")
            .method(RestMethod::Get)
            .id("test")
            .build();

        assert_eq!(endpoint.id(), "test");
    }

    #[test]
    fn test_path_params() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_user_posts")
            .method(RestMethod::Get)
            .path("/users/{user_id}/posts/{post_id}")
            .build();

        let params = endpoint.path_params();
        assert_eq!(params, vec!["user_id", "post_id"]);
    }

    #[test]
    fn test_substitute_params() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_user")
            .method(RestMethod::Get)
            .path("/users/{id}")
            .build();

        let path = endpoint.substitute_params(&[("id", "123")]);
        assert_eq!(path, "/users/123");
    }

    #[test]
    fn test_full_url() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_users")
            .method(RestMethod::Get)
            .path("/api/v1/users")
            .build();

        let base = url::Url::parse("https://example.com").unwrap();
        let full = endpoint.full_url(&base).unwrap();
        assert_eq!(full.as_str(), "https://example.com/api/v1/users");
    }

    #[test]
    fn test_clone() {
        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("test")
            .method(RestMethod::Get)
            .path("/test")
            .build();

        let cloned = endpoint.clone();
        assert_eq!(cloned.id(), endpoint.id());
    }
}
