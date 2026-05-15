/// Enumerates all of the `rel` values allowed in an HTML Link Tag
pub enum LinkRel {
    /// Alternate representations of the current document.
    Alternate,
    /// Author of the current document or article.
    Author,
    /// Preferred URL for the current document.
    Canonical,
    /// Link to a compression dictionary that can be used to compress future downloads for resources on this site.
    CompressionDictionary,
    /// When used with blocking="render", allows the page to be render-blocked until the essential parts of the
    /// document are parsed so it will render consistently.
    Expect,
    /// Link to context-sensitive help.
    Help,
    /// Indicates that the main content of the current document is covered by the copyright license described
    /// by the referenced document.
    License,
    /// Web app manifest
    Manifest,
    /// Indicates that the current document represents the person who owns the linked content.
    Me,
    /// Indicates that the current document is a part of a series and that the next document in the series is
    /// the referenced document.
    Next,
    /// Indicates that the current document is a part of a series and that the previous document in the series
    /// is the referenced document.
    Prev,
    /// Gives a link to information about the data collection and usage practices that apply to the current document.
    PrivacyPolicy,
    /// Gives a link to a resource that can be used to search through the current document and its related pages.
    Search,
    /// Imports a style sheet
    Stylesheet,
    /// Link to the agreement, or terms of service, between the document's provider and users who wish to use the
    /// document.
    TermsOfService,
}

/// Defines the content for an HTML `<link>` tag
pub struct LinkTag {
    rel: LinkRel,
    href: Option<String>,
    /// If hreflang is given alongside alternate, and the value of hreflang is different from the current document's language,
    /// it indicates that the referenced document is a translation
    hreflang: Option<String>,
    title: Option<String>,
    media: Option<String>,
}
