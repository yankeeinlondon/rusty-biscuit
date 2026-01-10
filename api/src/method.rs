//! HTTP method types for REST APIs.

use strum::{Display, EnumIter, EnumString};

/// HTTP methods for REST API endpoints.
///
/// This enum covers all standard HTTP methods used in REST APIs,
/// with utility methods for common operations.
///
/// ## Examples
///
/// ```rust
/// use api::RestMethod;
///
/// let method = RestMethod::Get;
/// assert!(!method.has_body());
/// assert!(method.is_idempotent());
///
/// // Parse from string
/// let parsed: RestMethod = "POST".parse().unwrap();
/// assert_eq!(parsed, RestMethod::Post);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
pub enum RestMethod {
    /// HTTP GET - Retrieve a resource.
    Get,
    /// HTTP POST - Create a resource or trigger an action.
    Post,
    /// HTTP PUT - Replace a resource entirely.
    Put,
    /// HTTP PATCH - Partially update a resource.
    Patch,
    /// HTTP DELETE - Remove a resource.
    Delete,
    /// HTTP HEAD - Retrieve headers only.
    Head,
    /// HTTP OPTIONS - Query supported methods.
    Options,
    /// HTTP TRACE - Echo the request for debugging.
    Trace,
}

impl RestMethod {
    /// Returns `true` if this method typically has a request body.
    ///
    /// POST, PUT, and PATCH typically include request bodies.
    /// Other methods do not.
    pub fn has_body(&self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch)
    }

    /// Returns `true` if this method is idempotent.
    ///
    /// Idempotent methods can be called multiple times with the same
    /// effect as calling once. POST and PATCH are not idempotent.
    pub fn is_idempotent(&self) -> bool {
        !matches!(self, Self::Post | Self::Patch)
    }

    /// Returns `true` if this method is safe (read-only).
    ///
    /// Safe methods should not modify server state.
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Options | Self::Trace)
    }

    /// Converts to the equivalent `reqwest::Method`.
    pub fn to_reqwest(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Patch => reqwest::Method::PATCH,
            Self::Delete => reqwest::Method::DELETE,
            Self::Head => reqwest::Method::HEAD,
            Self::Options => reqwest::Method::OPTIONS,
            Self::Trace => reqwest::Method::TRACE,
        }
    }
}

impl From<RestMethod> for reqwest::Method {
    fn from(method: RestMethod) -> Self {
        method.to_reqwest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_display() {
        assert_eq!(RestMethod::Get.to_string(), "GET");
        assert_eq!(RestMethod::Post.to_string(), "POST");
        assert_eq!(RestMethod::Delete.to_string(), "DELETE");
    }

    #[test]
    fn test_parse() {
        assert_eq!("GET".parse::<RestMethod>().unwrap(), RestMethod::Get);
        assert_eq!("POST".parse::<RestMethod>().unwrap(), RestMethod::Post);
        assert_eq!("DELETE".parse::<RestMethod>().unwrap(), RestMethod::Delete);
    }

    #[test]
    fn test_has_body() {
        assert!(!RestMethod::Get.has_body());
        assert!(RestMethod::Post.has_body());
        assert!(RestMethod::Put.has_body());
        assert!(RestMethod::Patch.has_body());
        assert!(!RestMethod::Delete.has_body());
        assert!(!RestMethod::Head.has_body());
    }

    #[test]
    fn test_is_idempotent() {
        assert!(RestMethod::Get.is_idempotent());
        assert!(!RestMethod::Post.is_idempotent());
        assert!(RestMethod::Put.is_idempotent());
        assert!(!RestMethod::Patch.is_idempotent());
        assert!(RestMethod::Delete.is_idempotent());
    }

    #[test]
    fn test_is_safe() {
        assert!(RestMethod::Get.is_safe());
        assert!(RestMethod::Head.is_safe());
        assert!(RestMethod::Options.is_safe());
        assert!(RestMethod::Trace.is_safe());
        assert!(!RestMethod::Post.is_safe());
        assert!(!RestMethod::Put.is_safe());
        assert!(!RestMethod::Delete.is_safe());
    }

    #[test]
    fn test_enum_iteration() {
        let methods: Vec<_> = RestMethod::iter().collect();
        assert_eq!(methods.len(), 8);
    }

    #[test]
    fn test_to_reqwest() {
        assert_eq!(RestMethod::Get.to_reqwest(), reqwest::Method::GET);
        assert_eq!(RestMethod::Post.to_reqwest(), reqwest::Method::POST);
    }
}
