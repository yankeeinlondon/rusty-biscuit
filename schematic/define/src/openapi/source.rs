//! OpenAPI specification source types.
//!
//! This module provides the [`OpenApiSource`] enum for representing different
//! ways to provide an OpenAPI specification document.
//!
//! ## Supported Sources
//!
//! - [`Path`](OpenApiSource::Path) - File path to read from disk
//! - [`Json`](OpenApiSource::Json) - JSON string content
//! - [`Yaml`](OpenApiSource::Yaml) - YAML string content
//! - [`Bytes`](OpenApiSource::Bytes) - Raw bytes (auto-detected format)
//! - [`Document`](OpenApiSource::Document) - Pre-parsed OpenAPI document
//!
//! ## Examples
//!
//! From a file path:
//!
//! ```
//! use schematic_define::openapi::OpenApiSource;
//!
//! let source = OpenApiSource::path("/api/openapi.yaml");
//! ```
//!
//! From a JSON string:
//!
//! ```
//! use schematic_define::openapi::OpenApiSource;
//!
//! let source = OpenApiSource::json(r#"{"openapi": "3.0.0"}"#);
//! ```
//!
//! From a YAML string:
//!
//! ```
//! use schematic_define::openapi::OpenApiSource;
//!
//! let source = OpenApiSource::yaml("openapi: '3.0.0'");
//! ```

use std::path::PathBuf;

/// Source for an OpenAPI specification document.
///
/// This enum represents the various ways an OpenAPI document can be provided
/// to the parsing system. Each variant corresponds to a different input format
/// or mechanism.
///
/// ## Examples
///
/// Create from a file path:
///
/// ```
/// use schematic_define::openapi::OpenApiSource;
/// use std::path::PathBuf;
///
/// let source = OpenApiSource::Path(PathBuf::from("/api/openapi.yaml"));
/// ```
///
/// Create using convenience methods:
///
/// ```
/// use schematic_define::openapi::OpenApiSource;
///
/// let json_source = OpenApiSource::json(r#"{"openapi": "3.0.0"}"#);
/// let yaml_source = OpenApiSource::yaml("openapi: '3.0.0'");
/// ```
#[derive(Debug, Clone)]
pub enum OpenApiSource {
    /// File path to an OpenAPI specification file.
    ///
    /// The file format (YAML or JSON) is automatically detected based on
    /// file extension or content.
    Path(PathBuf),

    /// JSON string containing an OpenAPI specification.
    ///
    /// The string should contain valid JSON that conforms to the OpenAPI
    /// specification schema.
    Json(String),

    /// YAML string containing an OpenAPI specification.
    ///
    /// The string should contain valid YAML that conforms to the OpenAPI
    /// specification schema.
    Yaml(String),

    /// Raw bytes containing an OpenAPI specification.
    ///
    /// The format (YAML or JSON) is automatically detected from content.
    /// This is useful when reading from network streams or embedded resources.
    Bytes(Vec<u8>),

    /// Pre-parsed OpenAPI document.
    ///
    /// Use this variant when you already have a parsed `openapiv3::OpenAPI`
    /// document and want to skip the parsing step.
    ///
    /// The document is boxed to keep the enum size reasonable, as `OpenAPI`
    /// is a large struct (~1.5KB).
    Document(Box<openapiv3::OpenAPI>),
}

impl OpenApiSource {
    /// Create a source from a file path.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::OpenApiSource;
    ///
    /// let source = OpenApiSource::path("/api/openapi.yaml");
    /// ```
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }

    /// Create a source from a JSON string.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::OpenApiSource;
    ///
    /// let source = OpenApiSource::json(r#"{"openapi": "3.0.0"}"#);
    /// ```
    pub fn json(content: impl Into<String>) -> Self {
        Self::Json(content.into())
    }

    /// Create a source from a YAML string.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::OpenApiSource;
    ///
    /// let source = OpenApiSource::yaml("openapi: '3.0.0'");
    /// ```
    pub fn yaml(content: impl Into<String>) -> Self {
        Self::Yaml(content.into())
    }

    /// Create a source from raw bytes.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::OpenApiSource;
    ///
    /// let source = OpenApiSource::bytes(b"openapi: '3.0.0'".to_vec());
    /// ```
    pub fn bytes(content: Vec<u8>) -> Self {
        Self::Bytes(content)
    }

    /// Create a source from a pre-parsed document.
    ///
    /// The document is automatically boxed to keep the enum size small.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use schematic_define::openapi::OpenApiSource;
    ///
    /// let doc: openapiv3::OpenAPI = /* ... */;
    /// let source = OpenApiSource::document(doc);
    /// ```
    pub fn document(doc: openapiv3::OpenAPI) -> Self {
        Self::Document(Box::new(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_variant() {
        let source = OpenApiSource::Path(PathBuf::from("/test/openapi.yaml"));
        assert!(matches!(source, OpenApiSource::Path(_)));
    }

    #[test]
    fn json_variant() {
        let source = OpenApiSource::Json(r#"{"openapi": "3.0.0"}"#.to_string());
        assert!(matches!(source, OpenApiSource::Json(_)));
    }

    #[test]
    fn yaml_variant() {
        let source = OpenApiSource::Yaml("openapi: '3.0.0'".to_string());
        assert!(matches!(source, OpenApiSource::Yaml(_)));
    }

    #[test]
    fn bytes_variant() {
        let source = OpenApiSource::Bytes(b"openapi: '3.0.0'".to_vec());
        assert!(matches!(source, OpenApiSource::Bytes(_)));
    }

    #[test]
    fn path_convenience() {
        let source = OpenApiSource::path("/test/openapi.yaml");
        match source {
            OpenApiSource::Path(p) => assert_eq!(p, PathBuf::from("/test/openapi.yaml")),
            _ => panic!("Expected Path variant"),
        }
    }

    #[test]
    fn json_convenience() {
        let source = OpenApiSource::json(r#"{"test": true}"#);
        match source {
            OpenApiSource::Json(s) => assert!(s.contains("test")),
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn yaml_convenience() {
        let source = OpenApiSource::yaml("test: true");
        match source {
            OpenApiSource::Yaml(s) => assert!(s.contains("test")),
            _ => panic!("Expected Yaml variant"),
        }
    }

    #[test]
    fn bytes_convenience() {
        let source = OpenApiSource::bytes(b"test".to_vec());
        match source {
            OpenApiSource::Bytes(b) => assert_eq!(b, b"test"),
            _ => panic!("Expected Bytes variant"),
        }
    }

    #[test]
    fn clone_works() {
        let source = OpenApiSource::yaml("test: true");
        let cloned = source.clone();
        assert!(matches!(cloned, OpenApiSource::Yaml(_)));
    }

    #[test]
    fn debug_works() {
        let source = OpenApiSource::json("{}");
        let debug = format!("{:?}", source);
        assert!(debug.contains("Json"));
    }
}
