use crate::html::rel::LinkRel;

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

impl LinkTag {
    /// Returns an identity key used for page-level deduplication.
    ///
    /// Two `<link>` tags are considered duplicates when they share the same
    /// `(rel, href)` pair. Differences in `media`, `hreflang`, or `title` are
    /// intentionally ignored — when two components both pull in the same
    /// stylesheet but disagree on `media`, the first-registered wins. This
    /// avoids emitting near-identical `<link>` tags that browsers will fetch
    /// independently.
    pub fn dedup_key(&self) -> String {
        let rel = match self.rel {
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
        };
        format!("{}|{}", rel, self.href.as_deref().unwrap_or(""))
    }
}
