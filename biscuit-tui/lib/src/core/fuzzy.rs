//! Inline fuzzy filter used by the choose components for search-on-type.
//!
//! [`FuzzyFilter`] wraps a [`nucleo_matcher::Matcher`] and a
//! [`Pattern`](nucleo_matcher::pattern::Pattern) together with the list
//! of option indices that currently pass the filter. The filter is
//! intentionally decoupled from `ChoiceOption<V>`'s generic parameter:
//! callers pass in a `&[String]` of labels on every mutation so the
//! filter itself owns no option-specific state.
//!
//! ## Semantics
//!
//! - An empty pattern is **inactive**: [`FuzzyFilter::visible`]
//!   returns every index in `0..labels.len()`, in source order.
//! - A non-empty pattern filters labels using fuzzy scoring and sorts
//!   the surviving indices by descending score, breaking ties by
//!   source order so the list is stable.
//! - [`FuzzyFilter::is_active`] reports whether the pattern is
//!   non-empty, which callers use to decide whether to show the search
//!   prompt row.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::core::FuzzyFilter;
//!
//! let labels = vec![
//!     "Apple".to_string(),
//!     "Berry".to_string(),
//!     "Blueberry".to_string(),
//! ];
//! let mut filter = FuzzyFilter::new();
//! filter.clear(&labels);
//! assert!(!filter.is_active());
//! assert_eq!(filter.visible(), &[0, 1, 2]);
//!
//! filter.set_pattern("berry", &labels);
//! assert!(filter.is_active());
//! assert_eq!(filter.visible(), &[1, 2]);
//! ```

use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

/// Fuzzy filter over a `&[String]` of option labels, powering the
/// search-on-type interaction for the choose components.
///
/// `FuzzyFilter` is stateless with respect to the option list it
/// filters: callers pass `labels` in on every mutation. This keeps the
/// filter decoupled from
/// [`ChoiceOption`](crate::components::ChoiceOption)'s generic
/// parameter.
///
/// The cached [`visible`](Self::visible) index set is always kept in
/// sync with the current pattern and the most recently supplied
/// `labels` slice. When the pattern is empty, the cached set contains
/// `0..labels.len()` in source order.
///
/// ## Examples
///
/// See the [module-level example](crate::core::fuzzy).
#[derive(Clone)]
pub struct FuzzyFilter {
    matcher: Matcher,
    pattern: Pattern,
    raw_pattern: String,
    visible: Vec<usize>,
}

impl FuzzyFilter {
    /// Creates a new `FuzzyFilter` with an empty pattern and an empty
    /// cached visible set.
    ///
    /// Callers typically follow this with a call to
    /// [`FuzzyFilter::clear`] (or any other mutation) that passes the
    /// live `labels` slice so the cached visible set is populated.
    ///
    /// ## Examples
    ///
    /// ```
    /// use tui_chrome::core::FuzzyFilter;
    ///
    /// let filter = FuzzyFilter::new();
    /// assert!(!filter.is_active());
    /// assert_eq!(filter.pattern(), "");
    /// ```
    pub fn new() -> Self {
        Self {
            matcher: Matcher::default(),
            pattern: Pattern::default(),
            raw_pattern: String::new(),
            visible: Vec::new(),
        }
    }

    /// Returns the current raw pattern string.
    ///
    /// This is the buffer shown in the search prompt row.
    pub fn pattern(&self) -> &str {
        &self.raw_pattern
    }

    /// Returns whether the filter is currently filtering anything.
    ///
    /// This is equivalent to `!self.pattern().is_empty()`.
    pub fn is_active(&self) -> bool {
        !self.raw_pattern.is_empty()
    }

    /// Replaces the pattern with `pattern` and recomputes the visible
    /// index set against `labels`.
    ///
    /// With a non-empty pattern, the visible set is sorted by
    /// descending fuzzy score; ties are broken by source order so
    /// equal-score labels keep the user's input order.
    ///
    /// With an empty pattern, the visible set is reset to
    /// `0..labels.len()`.
    pub fn set_pattern(&mut self, pattern: impl Into<String>, labels: &[String]) {
        self.raw_pattern = pattern.into();
        self.reparse();
        self.recompute(labels);
    }

    /// Appends `c` to the pattern and recomputes the visible index set
    /// against `labels`.
    ///
    /// Equivalent to calling [`FuzzyFilter::set_pattern`] with the
    /// current pattern plus `c`, but avoids allocating a new `String`.
    pub fn push_char(&mut self, c: char, labels: &[String]) {
        self.raw_pattern.push(c);
        self.reparse();
        self.recompute(labels);
    }

    /// Removes the last character from the pattern and recomputes the
    /// visible index set against `labels`.
    ///
    /// Popping from an empty pattern is a no-op.
    pub fn pop_char(&mut self, labels: &[String]) {
        if self.raw_pattern.pop().is_some() {
            self.reparse();
            self.recompute(labels);
        }
    }

    /// Clears the pattern and recomputes the visible index set, which
    /// becomes `0..labels.len()`.
    pub fn clear(&mut self, labels: &[String]) {
        self.raw_pattern.clear();
        self.reparse();
        self.recompute(labels);
    }

    /// Returns the indices (into the most recently supplied `labels`
    /// slice) that currently pass the filter.
    ///
    /// When the pattern is empty, this is the full `0..labels.len()`
    /// range in source order; when the pattern is non-empty, it is
    /// the score-sorted set of matching indices.
    pub fn visible(&self) -> &[usize] {
        &self.visible
    }

    /// Returns the highlight indices (char offsets into `label`) for
    /// matched characters against the current pattern.
    ///
    /// This is used by the renderer to style the matched characters
    /// differently. When the pattern is empty or the label does not
    /// match, the returned vector is empty. Indices are deduplicated
    /// and sorted.
    ///
    /// ## Notes
    ///
    /// The indices are `u32` char offsets into the internal
    /// [`nucleo_matcher::Utf32Str`] representation of `label`, which
    /// for ASCII labels matches byte offsets. Multi-byte UTF-8
    /// callers should convert from char indices to byte offsets
    /// before slicing `label` directly.
    pub fn highlight_indices(&mut self, label: &str) -> Vec<u32> {
        if self.raw_pattern.is_empty() {
            return Vec::new();
        }
        let mut buf = Vec::new();
        let haystack = Utf32Str::new(label, &mut buf);
        let mut indices = Vec::new();
        self.pattern
            .indices(haystack, &mut self.matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    fn reparse(&mut self) {
        self.pattern
            .reparse(&self.raw_pattern, CaseMatching::Smart, Normalization::Smart);
    }

    fn recompute(&mut self, labels: &[String]) {
        self.visible.clear();
        if self.raw_pattern.is_empty() {
            self.visible.extend(0..labels.len());
            return;
        }
        let mut buf = Vec::new();
        let mut scored: Vec<(usize, u32)> = labels
            .iter()
            .enumerate()
            .filter_map(|(idx, label)| {
                let haystack = Utf32Str::new(label, &mut buf);
                self.pattern
                    .score(haystack, &mut self.matcher)
                    .map(|score| (idx, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.visible.extend(scored.into_iter().map(|(idx, _)| idx));
    }
}

impl Default for FuzzyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FuzzyFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuzzyFilter")
            .field("pattern", &self.raw_pattern)
            .field("visible", &self.visible)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn new_has_empty_pattern_and_empty_visible() {
        let filter = FuzzyFilter::new();
        assert!(!filter.is_active());
        assert_eq!(filter.pattern(), "");
        assert!(filter.visible().is_empty());
    }

    #[test]
    fn default_is_empty() {
        let filter = FuzzyFilter::default();
        assert_eq!(filter.pattern(), "");
        assert!(filter.visible().is_empty());
    }

    #[test]
    fn empty_pattern_returns_all() {
        let labels = labels(&["alpha", "beta", "gamma"]);
        let mut filter = FuzzyFilter::new();
        filter.clear(&labels);
        assert_eq!(filter.visible(), &[0, 1, 2]);
        assert!(!filter.is_active());
    }

    #[test]
    fn empty_pattern_against_empty_labels() {
        let labels: Vec<String> = vec![];
        let mut filter = FuzzyFilter::new();
        filter.clear(&labels);
        assert!(filter.visible().is_empty());
    }

    #[test]
    fn pattern_filters_labels() {
        let labels = labels(&["Apple", "Berry", "Blueberry", "Cherry"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("berry", &labels);
        let visible: Vec<usize> = filter.visible().to_vec();
        assert!(visible.contains(&1), "Berry (idx 1) should match");
        assert!(visible.contains(&2), "Blueberry (idx 2) should match");
        assert!(!visible.contains(&0), "Apple should not match");
        assert!(!visible.contains(&3), "Cherry should not match");
    }

    #[test]
    fn nucleo_scoring_orders_substrings_first() {
        let labels = labels(&["back", "book", "brook"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("bok", &labels);
        let visible = filter.visible();
        assert!(!visible.is_empty());
        let pos_book = visible.iter().position(|&i| i == 1);
        let pos_brook = visible.iter().position(|&i| i == 2);
        if let (Some(pb), Some(pbr)) = (pos_book, pos_brook) {
            assert!(
                pb < pbr,
                "'book' (pos={pb}) should rank ahead of 'brook' (pos={pbr})",
            );
        }
    }

    #[test]
    fn set_empty_pattern_is_inactive() {
        let labels = labels(&["alpha", "beta"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("", &labels);
        assert!(!filter.is_active());
        assert_eq!(filter.visible(), &[0, 1]);
    }

    #[test]
    fn push_char_appends_and_filters() {
        let labels = labels(&["alpha", "beta", "gamma"]);
        let mut filter = FuzzyFilter::new();
        filter.push_char('b', &labels);
        assert_eq!(filter.pattern(), "b");
        let visible: Vec<usize> = filter.visible().to_vec();
        assert!(visible.contains(&1), "'beta' should match 'b'");
        assert!(!visible.contains(&0), "'alpha' should not match 'b'");
    }

    #[test]
    fn pop_char_restores() {
        let labels = labels(&["alpha", "beta", "gamma"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("ab", &labels);
        let visible_ab_len = filter.visible().len();
        filter.pop_char(&labels);
        assert_eq!(filter.pattern(), "a");
        let visible_a_len = filter.visible().len();
        assert!(visible_a_len >= visible_ab_len);
    }

    #[test]
    fn pop_char_on_empty_is_noop() {
        let labels = labels(&["alpha"]);
        let mut filter = FuzzyFilter::new();
        filter.pop_char(&labels);
        assert_eq!(filter.pattern(), "");
        assert!(!filter.is_active());
    }

    #[test]
    fn clear_resets_pattern_and_visible() {
        let labels = labels(&["alpha", "beta", "gamma"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("bet", &labels);
        assert!(filter.is_active());
        filter.clear(&labels);
        assert_eq!(filter.pattern(), "");
        assert!(!filter.is_active());
        assert_eq!(filter.visible(), &[0, 1, 2]);
    }

    #[test]
    fn no_matches_returns_empty() {
        let labels = labels(&["alpha", "beta", "gamma"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("zzz", &labels);
        assert!(filter.visible().is_empty());
    }

    #[test]
    fn highlight_indices_points_at_matching_chars() {
        let labels = labels(&["Berry"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("br", &labels);
        let highlights = filter.highlight_indices("Berry");
        assert_eq!(highlights.len(), 2);
        assert!(highlights.contains(&0));
    }

    #[test]
    fn highlight_indices_empty_for_empty_pattern() {
        let mut filter = FuzzyFilter::new();
        assert!(filter.highlight_indices("anything").is_empty());
    }

    #[test]
    fn highlight_indices_empty_for_non_match() {
        let labels = labels(&["Berry"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("xyz", &labels);
        assert!(filter.highlight_indices("Berry").is_empty());
    }

    #[test]
    fn stable_order_for_score_ties() {
        let labels = labels(&["ab_1", "ab_2"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("ab", &labels);
        assert_eq!(filter.visible(), &[0, 1]);
    }

    #[test]
    fn visible_updates_on_each_mutation() {
        let labels = labels(&["alpha", "beta"]);
        let mut filter = FuzzyFilter::new();
        filter.clear(&labels);
        assert_eq!(filter.visible(), &[0, 1]);
        filter.set_pattern("b", &labels);
        assert_eq!(filter.visible(), &[1]);
        filter.clear(&labels);
        assert_eq!(filter.visible(), &[0, 1]);
    }

    #[test]
    fn is_active_tracks_pattern() {
        let labels = labels(&["alpha"]);
        let mut filter = FuzzyFilter::new();
        assert!(!filter.is_active());
        filter.push_char('a', &labels);
        assert!(filter.is_active());
        filter.pop_char(&labels);
        assert!(!filter.is_active());
    }

    #[test]
    fn smart_case_matches_case_insensitive_when_pattern_is_lower() {
        let labels = labels(&["Alpha", "beta", "GAMMA"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("a", &labels);
        let visible: Vec<usize> = filter.visible().to_vec();
        assert!(visible.contains(&0), "'Alpha' should match lowercase 'a'");
        assert!(visible.contains(&2), "'GAMMA' should match lowercase 'a'");
    }

    #[test]
    fn highlight_indices_handles_multibyte_labels() {
        // "Café" has a 2-byte 'é' at byte offset 3. Char offsets for
        // 'C' and 'a' are 0 and 1 respectively — byte offsets would be
        // 0 and 1 too, but the 'é' at char offset 3 / byte offset 3 is
        // where char- vs byte-indexing diverges on later chars. The
        // contract is char offsets, so 'C' and 'a' must be 0 and 1.
        let labels = labels(&["Café", "Grünes Tee"]);
        let mut filter = FuzzyFilter::new();
        filter.set_pattern("ca", &labels);
        let highlights = filter.highlight_indices("Café");
        assert_eq!(
            highlights.len(),
            2,
            "expected two highlight positions for pattern 'ca' against 'Café'"
        );
        assert!(
            highlights.contains(&0),
            "expected char index 0 (the 'C') in {highlights:?}"
        );
        assert!(
            highlights.contains(&1),
            "expected char index 1 (the 'a') in {highlights:?}"
        );
    }
}
