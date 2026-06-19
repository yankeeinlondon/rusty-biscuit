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

/// Enumerates all of the `rel` values allowed on HTML `<a>` and `<area>` tags.
pub enum ARel {
    /// Alternate representation of the current document.
    Alternate,
    /// Author of the current document or article.
    Author,
    /// Permalink for the nearest ancestor section.
    Bookmark,
    /// Indicates that the referenced document is not part of the same site as the current document.
    External,
    /// Link to context-sensitive help.
    Help,
    /// Indicates that the main content of the current document is covered by the copyright license described
    /// by the referenced document.
    License,
    /// Indicates that the current document represents the person who owns the linked content.
    Me,
    /// Indicates that the current document is a part of a series and that the next document in the series is
    /// the referenced document.
    Next,
    /// Indicates that the current document's original author or publisher does not endorse the referenced
    /// document.
    NoFollow,
    /// Instructs the browser to open the linked resource without granting the new browsing context access to
    /// the document that opened it.
    NoOpener,
    /// Instructs the browser to navigate to the referenced document without sending an HTTP `Referer` header
    /// or exposing the opener via `window.opener`.
    NoReferrer,
    /// Opts out of the implicit `noopener` behavior that applies to `target="_blank"` links, allowing the new
    /// browsing context to access the opener.
    Opener,
    /// Indicates that the current document is a part of a series and that the previous document in the series
    /// is the referenced document.
    Prev,
    /// Gives a link to information about the data collection and usage practices that apply to the current document.
    PrivacyPolicy,
    /// Gives a link to a resource that can be used to search through the current document and its related pages.
    Search,
    /// Gives a tag (identified by the referenced document) that applies to the current document.
    Tag,
    /// Link to the agreement, or terms of service, between the document's provider and users who wish to use the
    /// document.
    TermsOfService,
}

/// Enumerates all of the `rel` values allowed on an HTML `<form>` tag.
pub enum FormRel {
    /// Indicates that the form's destination is not part of the same site as the current document.
    External,
    /// Link to context-sensitive help for the form's destination.
    Help,
    /// Indicates that the main content of the form's destination is covered by the copyright license described
    /// by the referenced document.
    License,
    /// Indicates that the form's destination is the next document in a series.
    Next,
    /// Indicates that the form's destination should not be endorsed by the current document's author or publisher.
    NoFollow,
    /// Instructs the browser to open the form's destination without granting the new browsing context access to
    /// the document that submitted it.
    NoOpener,
    /// Instructs the browser to navigate to the form's destination without sending an HTTP `Referer` header
    /// or exposing the opener via `window.opener`.
    NoReferrer,
    /// Opts out of the implicit `noopener` behavior that applies to `target="_blank"` form submissions, allowing
    /// the new browsing context to access the opener.
    Opener,
    /// Indicates that the form's destination is the previous document in a series.
    Prev,
    /// Indicates that the form's destination can be used to search through the current document and its related
    /// pages.
    Search,
}

/// The valid enumeration for the `rel` type depends on the tag which is using it
/// and this enumeration let's you choose from the three variant groups:
///
/// 1. for `<link>` tags
/// 2. for `<a>` hyperlinks for `<area>` tags
/// 3. for `<form>` tags
pub enum RelAttribute {
    ForLink(LinkRel),
    ForHyperLinkOrArea(ARel),
    ForForm(FormRel),
}

impl LinkRel {
    /// The `rel` keyword token for this link relationship.
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkRel::Alternate => "alternate",
            LinkRel::Author => "author",
            LinkRel::Canonical => "canonical",
            LinkRel::CompressionDictionary => "compression-dictionary",
            LinkRel::Expect => "expect",
            LinkRel::Help => "help",
            LinkRel::License => "license",
            LinkRel::Manifest => "manifest",
            LinkRel::Me => "me",
            LinkRel::Next => "next",
            LinkRel::Prev => "prev",
            LinkRel::PrivacyPolicy => "privacy-policy",
            LinkRel::Search => "search",
            LinkRel::Stylesheet => "stylesheet",
            LinkRel::TermsOfService => "terms-of-service",
        }
    }
}

impl ARel {
    /// The `rel` keyword token for this hyperlink/area relationship.
    pub fn as_str(&self) -> &'static str {
        match self {
            ARel::Alternate => "alternate",
            ARel::Author => "author",
            ARel::Bookmark => "bookmark",
            ARel::External => "external",
            ARel::Help => "help",
            ARel::License => "license",
            ARel::Me => "me",
            ARel::Next => "next",
            ARel::NoFollow => "nofollow",
            ARel::NoOpener => "noopener",
            ARel::NoReferrer => "noreferrer",
            ARel::Opener => "opener",
            ARel::Prev => "prev",
            ARel::PrivacyPolicy => "privacy-policy",
            ARel::Search => "search",
            ARel::Tag => "tag",
            ARel::TermsOfService => "terms-of-service",
        }
    }
}

impl FormRel {
    /// The `rel` keyword token for this form relationship.
    pub fn as_str(&self) -> &'static str {
        match self {
            FormRel::External => "external",
            FormRel::Help => "help",
            FormRel::License => "license",
            FormRel::Next => "next",
            FormRel::NoFollow => "nofollow",
            FormRel::NoOpener => "noopener",
            FormRel::NoReferrer => "noreferrer",
            FormRel::Opener => "opener",
            FormRel::Prev => "prev",
            FormRel::Search => "search",
        }
    }
}

impl RelAttribute {
    /// The `rel` keyword token, dispatching to the inner relationship enum.
    pub fn as_str(&self) -> &'static str {
        match self {
            RelAttribute::ForLink(rel) => rel.as_str(),
            RelAttribute::ForHyperLinkOrArea(rel) => rel.as_str(),
            RelAttribute::ForForm(rel) => rel.as_str(),
        }
    }
}
