//! Lazily compiled `pattern` rules.

use std::fmt;
use std::sync::OnceLock;

use regex::Regex;

/// A `pattern` rule: Rust `regex` source compiled on first use, once per
/// process.
///
/// The source is the generator's translation of a definition's pattern,
/// already proven to compile and to mean what the definition's pattern
/// means; it is compiled with the `regex` crate's default syntax. Matching
/// searches, as JSON Schema patterns do: `b` matches `abc`, and only explicit
/// `^`/`$` anchor.
///
/// A source that fails to compile despite that proof leaves the rule
/// unenforced; it never panics and is never retried per request.
///
/// A `Pattern` holds a lazily filled cell, so a [`Plan`](super::Plan)
/// refers to it from its own `static` rather than holding it inline:
///
/// ```
/// use schematic_runtime::validate::Pattern;
///
/// static SLUG: Pattern = Pattern::new("^[a-z0-9-]+$");
///
/// assert_eq!(SLUG.is_match("rust-lang"), Some(true));
/// assert_eq!(SLUG.is_match("Rust Lang"), Some(false));
/// ```
pub struct Pattern {
    source: &'static str,
    compiled: OnceLock<Option<Regex>>,
}

impl Pattern {
    /// A pattern rule for `source`; nothing is compiled yet.
    pub const fn new(source: &'static str) -> Self {
        Self {
            source,
            compiled: OnceLock::new(),
        }
    }

    /// The Rust `regex` source.
    pub const fn source(&self) -> &'static str {
        self.source
    }

    /// Whether `text` contains a match, or `None` when the source does not
    /// compile and the rule is unenforced.
    pub fn is_match(&self, text: &str) -> Option<bool> {
        self.compiled
            .get_or_init(|| {
                #[cfg(test)]
                tests::COMPILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Regex::new(self.source).ok()
            })
            .as_ref()
            .map(|regex| regex.is_match(text))
    }
}

/// Shows the source only: patterns come from the definition, not from a
/// request, and violation reasons never quote them.
impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Pattern").field(&self.source).finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Pattern;

    /// Compilations in this process. Nextest runs each test in its own
    /// process, so a test reads only its own compilations.
    pub(super) static COMPILES: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn a_pattern_compiles_once_however_often_it_is_matched() {
        static DIGITS: Pattern = Pattern::new("^[0-9]+$");
        let before = COMPILES.load(Ordering::Relaxed);
        for _ in 0..50 {
            assert_eq!(DIGITS.is_match("123"), Some(true));
            assert_eq!(DIGITS.is_match("12a"), Some(false));
        }
        assert_eq!(COMPILES.load(Ordering::Relaxed) - before, 1);
    }

    #[test]
    fn a_source_that_does_not_compile_is_unenforced_and_does_not_panic() {
        static BROKEN: Pattern = Pattern::new("(unclosed");
        let before = COMPILES.load(Ordering::Relaxed);
        assert_eq!(BROKEN.is_match("anything"), None);
        assert_eq!(BROKEN.is_match("anything else"), None);
        assert_eq!(COMPILES.load(Ordering::Relaxed) - before, 1, "a failed compile is not retried");
    }
}
