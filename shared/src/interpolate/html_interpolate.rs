pub enum HtmlTag {
    All,
    Body,
    Head,
    Div,
    Span,
    Section,
    Aside,
    Header,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Meta,
    Script,
    Pre,
    PreBlock,
    PreInline,
}

pub enum HtmlScope {
    /// allows you to find/replace the tag name and all of the attributes
    /// and if you change the tag-name then this is responsible for changing
    /// both the opening and closing tag (assuming it is a block tag with both opening and closing).
    TagAttributes(HtmlTag),
    /// Look for replacements within the inner HTML of a particular tag type
    InnerHtml(HtmlTag),
    /// Look for the replacements within the outer HTML of a particular tag type
    OuterHtml(HtmlTag),

    /// The HTML content of the DOM nodes which match the passed in query selector
    Selector(String),

    /// allows the search and replace to take place _only_ in
    /// the Body of the HTML (note: fragments that don't start with the
    /// `<html>` tag do not have to isolate on the `<body>` tag), and within
    /// the body only include non-tag content (aka, the concatenation of
    /// all DOM node's `.text()` content)
    Prose,
}

// TODO: should we also allow in CowStr in a smart way (aka, not copy strings when we don't need to?
pub fn html_interpolate<T: Into<String>, F: Into<String>, R: Into<String>>(
    content: T,
    find: F,
    replace: R,
    scope: HtmlScope,
) -> String {
  todo!()
}

// TODO: should we also allow in CowStr in a smart way (aka, not copy strings when we don't need to?
pub fn html_interpolate_regex<T: Into<String>, F: Into<String>, R: Into<String>>(
    content: T,
    find: F,
    replace: R,
    scope: HtmlScope,
) -> String {
  todo!()
}
