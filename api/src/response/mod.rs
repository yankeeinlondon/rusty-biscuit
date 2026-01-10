//! Response handling module.
//!
//! This module provides the [`ResponseFormat`] trait for type-safe response
//! parsing, along with format-specific implementations and the [`ApiResponseValue`]
//! enum for representing parsed responses.

mod format;
mod value;

pub use format::{
    BinaryFormat, CsvFormat, HtmlFormat, JsonFormat, PlainTextFormat, ResponseFormat, XmlFormat,
    YamlFormat,
};
pub use value::ApiResponseValue;

/// Trait for XML response types with optional XSD validation.
///
/// Implement this trait for types that should be deserialized from XML responses.
/// XSD validation is separate from parsing and uses the schema returned by
/// [`xsd_schema`](XmlSchema::xsd_schema).
///
/// ## Examples
///
/// ```rust,ignore
/// use api::response::XmlSchema;
/// use std::borrow::Cow;
///
/// #[derive(serde::Deserialize)]
/// struct MyXmlResponse {
///     data: String,
/// }
///
/// impl XmlSchema for MyXmlResponse {
///     fn xsd_schema() -> Option<Cow<'static, str>> {
///         Some(Cow::Borrowed(include_str!("schema.xsd")))
///     }
/// }
/// ```
pub trait XmlSchema: serde::de::DeserializeOwned + Send + Sync {
    /// Returns the XSD schema for validation, if available.
    ///
    /// ## Returns
    ///
    /// - `None` to skip XSD validation (parse-only mode)
    /// - `Some(Cow::Borrowed(...))` for static XSD strings
    /// - `Some(Cow::Owned(...))` for runtime-loaded XSD
    fn xsd_schema() -> Option<std::borrow::Cow<'static, str>> {
        None
    }
}

/// Blanket implementation for types without XSD schema.
impl<T: serde::de::DeserializeOwned + Send + Sync> XmlSchema for T {}
