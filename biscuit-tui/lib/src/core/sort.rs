//! Ordering policy applied to [`ChoiceOption`] lists before state
//! construction.
//!
//! `SortOrder` is a small, explicit enum that surfaces the four
//! orderings the CLI needs to expose via `--sort`. Callers build a
//! `Vec<ChoiceOption<V>>` from whatever source, then invoke
//! [`SortOrder::apply`] to reorder in place before handing off to
//! [`ChooseOneState`](crate::components::ChooseOneState) or
//! [`ChooseManyState`](crate::components::ChooseManyState).
//!
//! Sort runs on labels, not raw source strings, so users sort the
//! human-visible text after `--delimiter` parsing has split a
//! `label⟂value` pair.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::components::ChoiceOption;
//! use tui_chrome::core::SortOrder;
//!
//! let mut options: Vec<ChoiceOption<String>> = vec![
//!     ChoiceOption::new("b", "Berry", "b"),
//!     ChoiceOption::new("a", "Apple", "a"),
//!     ChoiceOption::new("c", "Cherry", "c"),
//! ];
//! SortOrder::Asc.apply(&mut options);
//! assert_eq!(options[0].label, "Apple");
//! assert_eq!(options[1].label, "Berry");
//! assert_eq!(options[2].label, "Cherry");
//! ```

use crate::components::ChoiceOption;

/// Ordering applied to a [`ChoiceOption`] list before state
/// construction.
///
/// ## Variants
///
/// - [`SortOrder::Natural`] preserves source order.
/// - [`SortOrder::Inverse`] reverses source order without comparing
///   labels.
/// - [`SortOrder::Asc`] sorts lexically by label (ascending).
/// - [`SortOrder::Desc`] sorts lexically by label (descending).
///
/// ## Notes
///
/// `Inverse` is the canonical name (matching the public CLI surface and
/// the spec). The historical `Reverse` spelling remains available as a
/// deprecated alias via the [`SortOrder::Reverse`] associated constant
/// for any in-tree call sites that have not yet migrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortOrder {
    /// Preserve source order (the default).
    #[default]
    Natural,
    /// Reverse source order.
    Inverse,
    /// Sort lexically by label (ascending).
    Asc,
    /// Sort lexically by label (descending).
    Desc,
}

impl SortOrder {
    /// Deprecated alias for [`SortOrder::Inverse`]. Kept so that
    /// pre-rename call sites continue to compile while they migrate to
    /// the canonical `Inverse` name.
    #[deprecated(since = "0.2.0", note = "use `SortOrder::Inverse` instead")]
    #[allow(non_upper_case_globals)]
    pub const Reverse: SortOrder = SortOrder::Inverse;

    /// Applies this ordering to `options` in place.
    ///
    /// The sort is stable (the standard library's `slice::sort_by` is
    /// guaranteed stable); options with equal labels keep their
    /// relative input order.
    pub fn apply<V>(self, options: &mut [ChoiceOption<V>]) {
        match self {
            SortOrder::Natural => {}
            SortOrder::Inverse => options.reverse(),
            SortOrder::Asc => options.sort_by(|a, b| a.label.cmp(&b.label)),
            SortOrder::Desc => options.sort_by(|a, b| b.label.cmp(&a.label)),
        }
    }
}

/// Preferred ordering vocabulary for choice components.
///
/// Mirrors [`SortOrder`] but uses `Inverse` instead of `Reverse` to
/// match the public CLI surface. Conversions in both directions are
/// provided so existing code can migrate incrementally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptionSort {
    /// Preserve source order (the default).
    #[default]
    Natural,
    /// Reverse source order.
    Inverse,
    /// Sort lexically by label (ascending).
    Asc,
    /// Sort lexically by label (descending).
    Desc,
}

impl OptionSort {
    /// Applies this ordering to `options` in place.
    pub fn apply<V>(self, options: &mut [ChoiceOption<V>]) {
        SortOrder::from(self).apply(options);
    }
}

impl From<SortOrder> for OptionSort {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Natural => OptionSort::Natural,
            SortOrder::Inverse => OptionSort::Inverse,
            SortOrder::Asc => OptionSort::Asc,
            SortOrder::Desc => OptionSort::Desc,
        }
    }
}

impl From<OptionSort> for SortOrder {
    fn from(value: OptionSort) -> Self {
        match value {
            OptionSort::Natural => SortOrder::Natural,
            OptionSort::Inverse => SortOrder::Inverse,
            OptionSort::Asc => SortOrder::Asc,
            OptionSort::Desc => SortOrder::Desc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Vec<ChoiceOption<String>> {
        vec![
            ChoiceOption::new("b", "Berry", "b"),
            ChoiceOption::new("a", "Apple", "a"),
            ChoiceOption::new("c", "Cherry", "c"),
        ]
    }

    #[test]
    fn natural_preserves_order() {
        let mut options = opts();
        SortOrder::Natural.apply(&mut options);
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Berry", "Apple", "Cherry"]
        );
    }

    #[test]
    fn inverse_inverts() {
        let mut options = opts();
        SortOrder::Inverse.apply(&mut options);
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Cherry", "Apple", "Berry"]
        );
    }

    #[test]
    #[allow(deprecated)]
    fn reverse_alias_still_resolves_to_inverse() {
        // The historical `SortOrder::Reverse` constant must continue
        // to compile and behave identically to `SortOrder::Inverse`
        // so in-tree callers can migrate incrementally.
        assert_eq!(SortOrder::Reverse, SortOrder::Inverse);
    }

    #[test]
    fn asc_lexical() {
        let mut options = opts();
        SortOrder::Asc.apply(&mut options);
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Apple", "Berry", "Cherry"]
        );
    }

    #[test]
    fn desc_lexical() {
        let mut options = opts();
        SortOrder::Desc.apply(&mut options);
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Cherry", "Berry", "Apple"]
        );
    }

    #[test]
    fn unicode_labels() {
        let mut options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("o", "Ölü", "o"),
            ChoiceOption::new("a", "Äpfel", "a"),
            ChoiceOption::new("z", "Zebra", "z"),
        ];
        SortOrder::Asc.apply(&mut options);
        // Rust's default `Ord` on `String` is byte-wise, so multi-byte
        // UTF-8 codepoints (Ä, Ö) sort after ASCII. We pin the
        // behaviour here so a future change to locale-aware sorting is
        // caught and tested explicitly.
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Zebra", "Äpfel", "Ölü"]
        );
    }

    #[test]
    fn default_is_natural() {
        assert_eq!(SortOrder::default(), SortOrder::Natural);
    }

    #[test]
    fn empty_list_is_no_op() {
        let mut options: Vec<ChoiceOption<String>> = vec![];
        SortOrder::Asc.apply(&mut options);
        assert!(options.is_empty());
        SortOrder::Inverse.apply(&mut options);
        assert!(options.is_empty());
    }

    #[test]
    fn option_sort_default_is_natural() {
        assert_eq!(OptionSort::default(), OptionSort::Natural);
    }

    #[test]
    fn option_sort_inverse_reverses() {
        let mut options = opts();
        OptionSort::Inverse.apply(&mut options);
        assert_eq!(
            options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
            vec!["Cherry", "Apple", "Berry"]
        );
    }

    #[test]
    fn sort_order_to_option_sort_roundtrips() {
        assert_eq!(OptionSort::from(SortOrder::Natural), OptionSort::Natural);
        assert_eq!(OptionSort::from(SortOrder::Inverse), OptionSort::Inverse);
        assert_eq!(OptionSort::from(SortOrder::Asc), OptionSort::Asc);
        assert_eq!(OptionSort::from(SortOrder::Desc), OptionSort::Desc);
    }

    #[test]
    fn option_sort_to_sort_order_roundtrips() {
        assert_eq!(SortOrder::from(OptionSort::Natural), SortOrder::Natural);
        assert_eq!(SortOrder::from(OptionSort::Inverse), SortOrder::Inverse);
        assert_eq!(SortOrder::from(OptionSort::Asc), SortOrder::Asc);
        assert_eq!(SortOrder::from(OptionSort::Desc), SortOrder::Desc);
    }
}
