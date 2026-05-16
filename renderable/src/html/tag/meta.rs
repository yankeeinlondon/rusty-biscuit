const UTF8: &str = "utf-8";

/// The `<meta>` HTML element represents metadata that cannot be represented by other meta-related elements,
/// such as `<base>`, `<link>`, `<script>`, `<style>`, or `<title>`.
#[derive(Default)]
#[allow(dead_code)]
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


impl MetaTag {
    pub fn new() -> MetaTag {
        MetaTag::default()
    }

    /// Mark this meta tag as the document charset declaration.
    pub fn set_charset(&mut self) -> &mut MetaTag {
        self.charset = Some(UTF8);
        self
    }

    /// Set the `name` attribute and its `content` value.
    pub fn set_named(&mut self, name: impl Into<String>, content: impl Into<String>) -> &mut MetaTag {
        self.name = Some(name.into());
        self.content = Some(content.into());
        self
    }

    /// Renders this meta tag as an HTML `<meta>` element.
    ///
    /// Emits `<meta charset="utf-8">` when a charset is set; otherwise
    /// emits the `name` / `http-equiv` / `content` / `media` attributes
    /// that are present.
    pub fn render(&self) -> String {
        if let Some(charset) = self.charset {
            return format!(r#"<meta charset="{charset}">"#);
        }
        let mut attrs = String::new();
        if let Some(http_equiv) = &self.http_equiv {
            attrs.push_str(&format!(r#" http-equiv="{http_equiv}""#));
        }
        if let Some(name) = &self.name {
            attrs.push_str(&format!(r#" name="{name}""#));
        }
        if let Some(content) = &self.content {
            attrs.push_str(&format!(r#" content="{content}""#));
        }
        if let Some(media) = &self.media {
            attrs.push_str(&format!(r#" media="{media}""#));
        }
        format!("<meta{attrs}>")
    }
}
