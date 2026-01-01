//! Pre-parsed scope cache for prose highlighting.
//!
//! Provides a cache of pre-parsed scopes for common markdown prose elements
//! to avoid repeated parsing overhead.

use lazy_static::lazy_static;
use syntect::parsing::Scope;

/// Pre-parsed scopes for markdown prose elements.
///
/// This struct caches commonly-used scopes to avoid parsing overhead
/// when highlighting prose elements in markdown.
///
/// Note: Currently unused but part of planned prose highlighting infrastructure.
#[allow(dead_code)]
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

#[allow(dead_code)]
impl ScopeCache {
    /// Creates a new scope cache with pre-parsed scopes.
    fn new() -> Self {
        Self {
            heading: Scope::new("markup.heading").unwrap(),
            bold: Scope::new("markup.bold").unwrap(),
            italic: Scope::new("markup.italic").unwrap(),
            quote: Scope::new("markup.quote").unwrap(),
            link: Scope::new("markup.underline.link").unwrap(),
            code_inline: Scope::new("markup.raw.inline").unwrap(),
            list: Scope::new("markup.list").unwrap(),
            base: Scope::new("text.html.markdown").unwrap(),
        }
    }

    /// Returns a reference to the global scope cache.
    pub fn global() -> &'static Self {
        &SCOPE_CACHE
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
        assert_eq!(cache.heading.to_string(), "markup.heading");
        assert_eq!(cache.bold.to_string(), "markup.bold");
        assert_eq!(cache.italic.to_string(), "markup.italic");
    }

    #[test]
    fn test_scope_cache_global() {
        let cache = ScopeCache::global();
        assert_eq!(cache.heading.to_string(), "markup.heading");
    }

    #[test]
    fn test_all_scopes_valid() {
        let cache = ScopeCache::global();
        assert!(cache.heading.to_string().len() > 0);
        assert!(cache.bold.to_string().len() > 0);
        assert!(cache.italic.to_string().len() > 0);
        assert!(cache.quote.to_string().len() > 0);
        assert!(cache.link.to_string().len() > 0);
        assert!(cache.code_inline.to_string().len() > 0);
        assert!(cache.list.to_string().len() > 0);
        assert!(cache.base.to_string().len() > 0);
    }
}
