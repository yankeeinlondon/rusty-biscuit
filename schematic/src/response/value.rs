//! Parsed response value types.

/// Parsed response value from an API call.
///
/// This enum represents the OUTPUT of a successful API call, holding the
/// parsed response data in a type-safe manner. The type parameters encode
/// the expected response type for structured formats.
///
/// ## Type Parameters
///
/// - `T`: The type for JSON/YAML responses.
/// - `X`: The type for XML responses (defaults to unit if not used).
///
/// ## Examples
///
/// ```rust
/// use api::response::ApiResponseValue;
///
/// #[derive(Debug)]
/// struct User { name: String }
///
/// let response: ApiResponseValue<User> = ApiResponseValue::Json(User {
///     name: "Alice".to_string(),
/// });
///
/// if let Some(user) = response.as_json() {
///     println!("User: {:?}", user);
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ApiResponseValue<T, X = ()> {
    /// Parsed JSON response.
    Json(T),
    /// Parsed YAML response.
    Yaml(T),
    /// Parsed XML response (with optional XSD validation).
    Xml(X),
    /// Plain text response.
    PlainText(String),
    /// HTML response.
    Html(String),
    /// CSV response (unparsed - caller handles parsing).
    Csv(String),
    /// Binary response.
    Binary(Vec<u8>),
}

impl<T, X> ApiResponseValue<T, X> {
    /// Returns `true` if this is a structured data response (JSON/YAML/XML).
    pub fn is_structured(&self) -> bool {
        matches!(self, Self::Json(_) | Self::Yaml(_) | Self::Xml(_))
    }

    /// Returns `true` if this is a text-based response (PlainText/HTML/CSV).
    pub fn is_text(&self) -> bool {
        matches!(self, Self::PlainText(_) | Self::Html(_) | Self::Csv(_))
    }

    /// Returns `true` if this is a binary response.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    /// Attempt to get the JSON value, returning `None` for other formats.
    pub fn as_json(&self) -> Option<&T> {
        match self {
            Self::Json(v) => Some(v),
            _ => None,
        }
    }

    /// Attempt to get the YAML value, returning `None` for other formats.
    pub fn as_yaml(&self) -> Option<&T> {
        match self {
            Self::Yaml(v) => Some(v),
            _ => None,
        }
    }

    /// Attempt to get the XML value, returning `None` for other formats.
    pub fn as_xml(&self) -> Option<&X> {
        match self {
            Self::Xml(v) => Some(v),
            _ => None,
        }
    }

    /// Attempt to get text content, returning `None` for non-text formats.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::PlainText(s) | Self::Html(s) | Self::Csv(s) => Some(s),
            _ => None,
        }
    }

    /// Attempt to get binary content, returning `None` for non-binary formats.
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Self::Binary(b) => Some(b),
            _ => None,
        }
    }

    /// Convert into the JSON value, returning `Err(self)` for other formats.
    pub fn into_json(self) -> Result<T, Self> {
        match self {
            Self::Json(v) => Ok(v),
            other => Err(other),
        }
    }

    /// Convert into the YAML value, returning `Err(self)` for other formats.
    pub fn into_yaml(self) -> Result<T, Self> {
        match self {
            Self::Yaml(v) => Ok(v),
            other => Err(other),
        }
    }

    /// Convert into the XML value, returning `Err(self)` for other formats.
    pub fn into_xml(self) -> Result<X, Self> {
        match self {
            Self::Xml(v) => Ok(v),
            other => Err(other),
        }
    }
}

impl<T: PartialEq, X: PartialEq> PartialEq for ApiResponseValue<T, X> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Json(a), Self::Json(b)) => a == b,
            (Self::Yaml(a), Self::Yaml(b)) => a == b,
            (Self::Xml(a), Self::Xml(b)) => a == b,
            (Self::PlainText(a), Self::PlainText(b)) => a == b,
            (Self::Html(a), Self::Html(b)) => a == b,
            (Self::Csv(a), Self::Csv(b)) => a == b,
            (Self::Binary(a), Self::Binary(b)) => a == b,
            _ => false,
        }
    }
}

impl<T: Eq, X: Eq> Eq for ApiResponseValue<T, X> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        value: i32,
    }

    #[test]
    fn test_is_structured() {
        let json: ApiResponseValue<TestData> = ApiResponseValue::Json(TestData { value: 1 });
        assert!(json.is_structured());

        let text: ApiResponseValue<TestData> = ApiResponseValue::PlainText("hello".to_string());
        assert!(!text.is_structured());
    }

    #[test]
    fn test_is_text() {
        let html: ApiResponseValue<(), ()> = ApiResponseValue::Html("<p>hi</p>".to_string());
        assert!(html.is_text());

        let binary: ApiResponseValue<(), ()> = ApiResponseValue::Binary(vec![1, 2, 3]);
        assert!(!binary.is_text());
    }

    #[test]
    fn test_as_json() {
        let response: ApiResponseValue<TestData> = ApiResponseValue::Json(TestData { value: 42 });
        assert_eq!(response.as_json(), Some(&TestData { value: 42 }));

        let text: ApiResponseValue<TestData> = ApiResponseValue::PlainText("hello".to_string());
        assert_eq!(text.as_json(), None);
    }

    #[test]
    fn test_as_text() {
        let plain: ApiResponseValue<(), ()> = ApiResponseValue::PlainText("hello".to_string());
        assert_eq!(plain.as_text(), Some("hello"));

        let html: ApiResponseValue<(), ()> = ApiResponseValue::Html("<p>hi</p>".to_string());
        assert_eq!(html.as_text(), Some("<p>hi</p>"));

        let binary: ApiResponseValue<(), ()> = ApiResponseValue::Binary(vec![]);
        assert_eq!(binary.as_text(), None);
    }

    #[test]
    fn test_into_json() {
        let response: ApiResponseValue<TestData> = ApiResponseValue::Json(TestData { value: 42 });
        let result = response.into_json();
        assert_eq!(result, Ok(TestData { value: 42 }));

        let text: ApiResponseValue<TestData> = ApiResponseValue::PlainText("hello".to_string());
        let result = text.into_json();
        assert!(result.is_err());
    }

    #[test]
    fn test_equality() {
        let a: ApiResponseValue<i32> = ApiResponseValue::Json(42);
        let b: ApiResponseValue<i32> = ApiResponseValue::Json(42);
        let c: ApiResponseValue<i32> = ApiResponseValue::Json(43);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
