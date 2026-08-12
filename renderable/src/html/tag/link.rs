use crate::html::attribute::rel::LinkRel;

/// Defines the content for an HTML `<link>` tag
#[allow(dead_code)]
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
    /// Constructs a `<link>` tag with the given `rel` and `href`.
    ///
    /// The optional `hreflang`, `title`, and `media` attributes default to
    /// unset; a feature or component that needs them can extend this later.
    pub fn new(rel: LinkRel, href: impl Into<String>) -> LinkTag {
        LinkTag {
            rel,
            href: Some(href.into()),
            hreflang: None,
            title: None,
            media: None,
        }
    }

    /// Returns an identity key used for page-level deduplication.
    ///
    /// Two `<link>` tags are considered duplicates when they share the same
    /// `(rel, href)` pair. Differences in `media`, `hreflang`, or `title` are
    /// intentionally ignored — when two components both pull in the same
    /// stylesheet but disagree on `media`, the first-registered wins. This
    /// avoids emitting near-identical `<link>` tags that browsers will fetch
    /// independently.
    pub fn dedup_key(&self) -> String {
        format!("{}|{}", self.rel_str(), self.href.as_deref().unwrap_or(""))
    }

    /// Returns the `rel` attribute as its HTML keyword string.
    fn rel_str(&self) -> &'static str {
        self.rel.as_str()
    }

    /// Renders this link tag as an HTML `<link>` element.
    pub fn render(&self) -> String {
        let mut out = format!(r#"<link rel="{}""#, self.rel_str());
        if let Some(href) = &self.href {
            out.push_str(&format!(r#" href="{href}""#));
        }
        if let Some(hreflang) = &self.hreflang {
            out.push_str(&format!(r#" hreflang="{hreflang}""#));
        }
        if let Some(title) = &self.title {
            out.push_str(&format!(r#" title="{title}""#));
        }
        if let Some(media) = &self.media {
            out.push_str(&format!(r#" media="{media}""#));
        }
        out.push('>');
        out
    }
}
