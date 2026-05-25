const UTF8: &'static str = "utf-8";

/// The <meta> HTML element represents metadata that cannot be represented by other meta-related elements,
/// such as <base>, <link>, <script>, <style>, or <title>.
pub struct MetaTag {
    /// This attribute declares the document's character encoding. If the attribute is present, its value
    /// must be an ASCII case-insensitive match for the string "utf-8", because UTF-8 is the only valid
    /// encoding for HTML5 documents. <meta> elements which declare a character encoding must be located
    /// entirely within the first 1024 bytes of the document.
    charset: Option<&'static str>,
    /// This attribute contains the _value_ for the `http-equiv` or `name` attribute, depending on which is used.
    content: Option<String>,
    /// Defines a pragma directive, which are instructions for the browser for processing the document. The
    /// attribute's name is short for http-equivalent because the allowed values are names of equivalent HTTP headers.
    http_equiv: Option<String>,
    /// The media attribute defines which media the theme color defined in the content attribute should be applied to.
    /// Its value is a media query, which defaults to all if the attribute is missing. This attribute is only relevant
    /// when the element's name attribute is set to `theme-color`. Otherwise, it has no effect, and should not be included.
    media: Option<String>,
    name: Option<String>,
}

impl Default for MetaTag {
    fn default() -> MetaTag {
        MetaTag {
            charset: None,
            content: None,
            http_equiv: None,
            media: None,
            name: None,
        }
    }
}

impl MetaTag {
    pub fn new() -> MetaTag {
        MetaTag::default()
    }

    pub fn set_charset(&mut self) -> &mut self {
        self.charset = Some(UTF8);
        self
    }

    /// render the meta tag for the HTML page
    pub fn render() -> String {
        !todo()
    }
}
