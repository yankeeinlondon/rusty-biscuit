//! Pre-parsed scope cache for prose highlighting.
//!
//! Provides a cache of pre-parsed scopes for common markdown prose elements
//! to avoid repeated parsing overhead.

use lazy_static::lazy_static;
use pulldown_cmark::Tag;
use syntect::parsing::Scope;

/// Pre-parsed scopes for markdown prose elements.
///
/// This struct caches commonly-used scopes to avoid parsing overhead
/// when highlighting prose elements in markdown.
#[derive(Debug, Clone)]
pub struct ScopeCache {
    pub heading: Scope,
    pub bold: Scope,
    pub italic: Scope,
    pub quote: Scope,
    pub link: Scope,
    pub code_inline: Scope,
    pub list: Scope,
    pub base: Scope,
}

impl ScopeCache {
    /// Creates a new scope cache with pre-parsed scopes.
    ///
    /// ## Panics
    ///
    /// Panics if any hardcoded scope string is invalid (should never happen).
    fn new() -> Self {
        Self {
            base: Scope::new("text.html.markdown")
                .expect("Invalid hardcoded scope: text.html.markdown"),
            heading: Scope::new("markup.heading.markdown")
                .expect("Invalid hardcoded scope: markup.heading.markdown"),
            bold: Scope::new("markup.bold.markdown")
                .expect("Invalid hardcoded scope: markup.bold.markdown"),
            italic: Scope::new("markup.italic.markdown")
                .expect("Invalid hardcoded scope: markup.italic.markdown"),
            quote: Scope::new("markup.quote.markdown")
                .expect("Invalid hardcoded scope: markup.quote.markdown"),
            link: Scope::new("markup.underline.link.markdown")
                .expect("Invalid hardcoded scope: markup.underline.link.markdown"),
            code_inline: Scope::new("markup.raw.inline.markdown")
                .expect("Invalid hardcoded scope: markup.raw.inline.markdown"),
            list: Scope::new("markup.list.markdown")
                .expect("Invalid hardcoded scope: markup.list.markdown"),
        }
    }

    /// Returns a reference to the global scope cache.
    pub fn global() -> &'static Self {
        &SCOPE_CACHE
    }

    /// Returns the appropriate scope for a pulldown_cmark Tag.
    ///
    /// Returns `None` for tags that don't have a corresponding scope.
    pub fn scope_for_tag(&self, tag: &Tag) -> Option<Scope> {
        match tag {
            Tag::Heading { .. } => Some(self.heading),
            Tag::Strong => Some(self.bold),
            Tag::Emphasis => Some(self.italic),
            Tag::BlockQuote(_) => Some(self.quote),
            Tag::Link { .. } => Some(self.link),
            Tag::List(_) => Some(self.list),
            _ => None,
        }
    }
}

lazy_static! {
    /// Global scope cache instance.
    static ref SCOPE_CACHE: ScopeCache = ScopeCache::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_cache_new() {
        let cache = ScopeCache::new();
        assert_eq!(cache.heading.to_string(), "markup.heading.markdown");
        assert_eq!(cache.bold.to_string(), "markup.bold.markdown");
        assert_eq!(cache.italic.to_string(), "markup.italic.markdown");
    }

    #[test]
    fn test_scope_cache_global() {
        let cache = ScopeCache::global();
        assert_eq!(cache.heading.to_string(), "markup.heading.markdown");
    }

    #[test]
    fn test_all_scopes_valid() {
        let cache = ScopeCache::global();
        assert!(!cache.heading.to_string().is_empty());
        assert!(!cache.bold.to_string().is_empty());
        assert!(!cache.italic.to_string().is_empty());
        assert!(!cache.quote.to_string().is_empty());
        assert!(!cache.link.to_string().is_empty());
        assert!(!cache.code_inline.to_string().is_empty());
        assert!(!cache.list.to_string().is_empty());
        assert!(!cache.base.to_string().is_empty());
    }

    #[test]
    fn test_scope_for_tag_heading() {
        let cache = ScopeCache::global();
        let tag = Tag::Heading { level: pulldown_cmark::HeadingLevel::H1, id: None, classes: vec![], attrs: vec![] };
        let scope = cache.scope_for_tag(&tag);
        assert!(scope.is_some());
        assert_eq!(scope.unwrap(), cache.heading);
    }

    #[test]
    fn test_scope_for_tag_strong() {
        let cache = ScopeCache::global();
        let tag = Tag::Strong;
        let scope = cache.scope_for_tag(&tag);
        assert!(scope.is_some());
        assert_eq!(scope.unwrap(), cache.bold);
    }

    #[test]
    fn test_scope_for_tag_emphasis() {
        let cache = ScopeCache::global();
        let tag = Tag::Emphasis;
        let scope = cache.scope_for_tag(&tag);
        assert!(scope.is_some());
        assert_eq!(scope.unwrap(), cache.italic);
    }

    #[test]
    fn test_scope_for_tag_paragraph() {
        let cache = ScopeCache::global();
        let tag = Tag::Paragraph;
        let scope = cache.scope_for_tag(&tag);
        assert!(scope.is_none());
    }

    #[test]
    fn test_scope_for_tag_link() {
        let cache = ScopeCache::global();
        let tag = Tag::Link { link_type: pulldown_cmark::LinkType::Inline, dest_url: "".into(), title: "".into(), id: "".into() };
        let scope = cache.scope_for_tag(&tag);
        assert!(scope.is_some());
        assert_eq!(scope.unwrap(), cache.link);
    }
}
