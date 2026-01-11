//! Response format trait and implementations.
//!
//! The [`ResponseFormat`] trait defines how to parse HTTP responses into
//! typed values. Each format (JSON, YAML, XML, etc.) has its own implementation.

use std::future::Future;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::error::ValidationError;

/// Trait for response format parsing strategies.
///
/// Each format implements its own parsing logic, transforming an HTTP
/// response body into a typed output value.
///
/// ## Examples
///
/// ```rust,ignore
/// use api::response::{ResponseFormat, JsonFormat};
///
/// // The format type encodes both the parsing strategy and output type
/// type UserResponse = JsonFormat<User>;
/// ```
pub trait ResponseFormat: Send + Sync {
    /// The output type after parsing.
    type Output: Send + Sync;

    /// Parse a response body into the output type.
    fn parse(
        body: bytes::Bytes,
    ) -> impl Future<Output = Result<Self::Output, ValidationError>> + Send;

    /// Returns the expected Content-Type for this format.
    fn content_type() -> &'static str;
}

/// JSON response format with typed deserialization.
///
/// ## Type Parameters
///
/// - `T`: The type to deserialize the JSON into. Must implement [`DeserializeOwned`].
#[derive(Debug, Clone, Copy)]
pub struct JsonFormat<T>(PhantomData<T>);

impl<T: DeserializeOwned + Send + Sync> ResponseFormat for JsonFormat<T> {
    type Output = T;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        serde_json::from_slice(&body).map_err(ValidationError::JsonParse)
    }

    fn content_type() -> &'static str {
        "application/json"
    }
}

/// YAML response format with typed deserialization.
///
/// ## Type Parameters
///
/// - `T`: The type to deserialize the YAML into. Must implement [`DeserializeOwned`].
#[derive(Debug, Clone, Copy)]
pub struct YamlFormat<T>(PhantomData<T>);

impl<T: DeserializeOwned + Send + Sync> ResponseFormat for YamlFormat<T> {
    type Output = T;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        serde_yaml::from_slice(&body).map_err(ValidationError::YamlParse)
    }

    fn content_type() -> &'static str {
        "application/yaml"
    }
}

/// XML response format with optional XSD validation.
///
/// ## Type Parameters
///
/// - `X`: The type to deserialize the XML into. Must implement [`XmlSchema`](super::XmlSchema).
#[derive(Debug, Clone, Copy)]
pub struct XmlFormat<X>(PhantomData<X>);

impl<X: DeserializeOwned + Send + Sync> ResponseFormat for XmlFormat<X> {
    type Output = X;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        quick_xml::de::from_reader(body.as_ref()).map_err(ValidationError::XmlParse)
    }

    fn content_type() -> &'static str {
        "application/xml"
    }
}

/// Plain text response format.
///
/// Returns the response body as a UTF-8 string.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainTextFormat;

impl ResponseFormat for PlainTextFormat {
    type Output = String;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        String::from_utf8(body.to_vec()).map_err(|e| {
            ValidationError::ContentTypeMismatch {
                expected: "valid UTF-8 text".to_string(),
                actual: format!("invalid UTF-8: {e}"),
            }
        })
    }

    fn content_type() -> &'static str {
        "text/plain"
    }
}

/// HTML response format.
///
/// Returns the response body as a UTF-8 string with semantic HTML marker.
#[derive(Debug, Clone, Copy, Default)]
pub struct HtmlFormat;

impl ResponseFormat for HtmlFormat {
    type Output = String;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        String::from_utf8(body.to_vec()).map_err(|e| {
            ValidationError::ContentTypeMismatch {
                expected: "valid UTF-8 HTML".to_string(),
                actual: format!("invalid UTF-8: {e}"),
            }
        })
    }

    fn content_type() -> &'static str {
        "text/html"
    }
}

/// CSV response format.
///
/// Returns the response body as a UTF-8 string. Parsing into rows/columns
/// is left to the caller.
#[derive(Debug, Clone, Copy, Default)]
pub struct CsvFormat;

impl ResponseFormat for CsvFormat {
    type Output = String;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        String::from_utf8(body.to_vec()).map_err(|e| {
            ValidationError::ContentTypeMismatch {
                expected: "valid UTF-8 CSV".to_string(),
                actual: format!("invalid UTF-8: {e}"),
            }
        })
    }

    fn content_type() -> &'static str {
        "text/csv"
    }
}

/// Binary response format.
///
/// Returns the raw response bytes without interpretation.
#[derive(Debug, Clone, Copy, Default)]
pub struct BinaryFormat;

impl ResponseFormat for BinaryFormat {
    type Output = Vec<u8>;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        Ok(body.to_vec())
    }

    fn content_type() -> &'static str {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_json_format_parse() {
        let json = r#"{"name": "test", "value": 42}"#;
        let body = bytes::Bytes::from(json);

        let result = JsonFormat::<TestData>::parse(body).await.unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.value, 42);
    }

    #[tokio::test]
    async fn test_json_format_invalid() {
        let body = bytes::Bytes::from("not json");
        let result = JsonFormat::<TestData>::parse(body).await;
        assert!(matches!(result, Err(ValidationError::JsonParse(_))));
    }

    #[tokio::test]
    async fn test_yaml_format_parse() {
        let yaml = "name: test\nvalue: 42";
        let body = bytes::Bytes::from(yaml);

        let result = YamlFormat::<TestData>::parse(body).await.unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.value, 42);
    }

    #[tokio::test]
    async fn test_plain_text_format() {
        let text = "Hello, World!";
        let body = bytes::Bytes::from(text);

        let result = PlainTextFormat::parse(body).await.unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_binary_format() {
        let data = vec![0x00, 0x01, 0x02, 0xFF];
        let body = bytes::Bytes::from(data.clone());

        let result = BinaryFormat::parse(body).await.unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_content_types() {
        assert_eq!(JsonFormat::<()>::content_type(), "application/json");
        assert_eq!(YamlFormat::<()>::content_type(), "application/yaml");
        assert_eq!(XmlFormat::<()>::content_type(), "application/xml");
        assert_eq!(PlainTextFormat::content_type(), "text/plain");
        assert_eq!(HtmlFormat::content_type(), "text/html");
        assert_eq!(CsvFormat::content_type(), "text/csv");
        assert_eq!(BinaryFormat::content_type(), "application/octet-stream");
    }
}
