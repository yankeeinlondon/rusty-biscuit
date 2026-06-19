pub mod aria;
pub mod rel;

/// A document-unique `id` attribute value.
pub struct DomId(String);

impl DomId {
    /// Wraps a string as a `DomId`.
    pub fn new(value: impl Into<String>) -> Self {
        DomId(value.into())
    }

    /// Borrows the underlying identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One or more space-separated CSS class names.
pub struct ClassDefinition(String);

impl ClassDefinition {
    /// Wraps a string as a `ClassDefinition`.
    pub fn new(value: impl Into<String>) -> Self {
        ClassDefinition(value.into())
    }

    /// Borrows the underlying class string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The unique name component of a `data-{name}` HTML data attribute.
pub struct HtmlDataAttribute(String);

impl HtmlDataAttribute {
    /// Wraps a string as a `HtmlDataAttribute` name.
    pub fn new(value: impl Into<String>) -> Self {
        HtmlDataAttribute(value.into())
    }

    /// Borrows the underlying data-attribute name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
