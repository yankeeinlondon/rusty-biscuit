//! Type options of the `alphanumeric` SimplifiedSchema type.
//!
//! An `alphanumeric` value is a string of ASCII letters and digits. The nine
//! [`AlphanumericOption`]s, written before the first `;` of the type's
//! parenthesized list, widen or narrow that set and are part of the type's
//! identity. [`AlphanumericOptions::canonical`] is the one canonicalizer every
//! consumer (identity comparison, serialization, lowering, descriptors, the
//! subtype relation, and the editor) shares, so two spellings with the same
//! meaning are the same type.

use std::fmt;

/// One `alphanumeric` type option. Declaration order is the canonical
/// (table) order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AlphanumericOption {
    /// Also allows `-`.
    AllowDashes,
    /// Also allows `_`.
    AllowUnderscores,
    /// Also allows `/` (never `\`).
    AllowSlashes,
    /// Also allows `.`.
    AllowPeriods,
    /// Also allows the ASCII space character (U+0020), never other whitespace.
    AllowWhitespace,
    /// The value may not contain `..`.
    BlockDoublePeriods,
    /// The first character may not be `-`.
    BlockLeadingDash,
    /// The first character may not be `/`.
    BlockLeadingSlash,
    /// The first character must be an ASCII letter.
    LeadingAlpha,
}

impl AlphanumericOption {
    /// Every option, in canonical order.
    pub const ALL: [AlphanumericOption; 9] = [
        AlphanumericOption::AllowDashes,
        AlphanumericOption::AllowUnderscores,
        AlphanumericOption::AllowSlashes,
        AlphanumericOption::AllowPeriods,
        AlphanumericOption::AllowWhitespace,
        AlphanumericOption::BlockDoublePeriods,
        AlphanumericOption::BlockLeadingDash,
        AlphanumericOption::BlockLeadingSlash,
        AlphanumericOption::LeadingAlpha,
    ];

    /// The option's keyword as written in the grammar.
    pub fn as_keyword(self) -> &'static str {
        match self {
            AlphanumericOption::AllowDashes => "allow-dashes",
            AlphanumericOption::AllowUnderscores => "allow-underscores",
            AlphanumericOption::AllowSlashes => "allow-slashes",
            AlphanumericOption::AllowPeriods => "allow-periods",
            AlphanumericOption::AllowWhitespace => "allow-whitespace",
            AlphanumericOption::BlockDoublePeriods => "block-double-periods",
            AlphanumericOption::BlockLeadingDash => "block-leading-dash",
            AlphanumericOption::BlockLeadingSlash => "block-leading-slash",
            AlphanumericOption::LeadingAlpha => "leading-alpha",
        }
    }

    /// Parses an option keyword. Case-sensitive, like type keywords.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|option| option.as_keyword() == keyword)
    }

    /// A one-line description of the option's effect.
    pub fn summary(self) -> &'static str {
        match self {
            AlphanumericOption::AllowDashes => "also allows `-`",
            AlphanumericOption::AllowUnderscores => "also allows `_`",
            AlphanumericOption::AllowSlashes => "also allows `/`",
            AlphanumericOption::AllowPeriods => "also allows `.`",
            AlphanumericOption::AllowWhitespace => "also allows the ASCII space character",
            AlphanumericOption::BlockDoublePeriods => "the value may not contain `..`",
            AlphanumericOption::BlockLeadingDash => "the first character may not be `-`",
            AlphanumericOption::BlockLeadingSlash => "the first character may not be `/`",
            AlphanumericOption::LeadingAlpha => "the first character must be an ASCII letter",
        }
    }

    fn bit(self) -> u16 {
        1 << self as u16
    }
}

impl fmt::Display for AlphanumericOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_keyword())
    }
}

/// A set of ASCII characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AsciiCharSet(u128);

impl AsciiCharSet {
    /// The ASCII letters and digits.
    pub fn alphanumeric() -> Self {
        let mut set = Self::default();
        for byte in (b'A'..=b'Z').chain(b'a'..=b'z').chain(b'0'..=b'9') {
            set.insert(byte);
        }
        set
    }

    /// The ASCII letters.
    pub fn letters() -> Self {
        let mut set = Self::default();
        for byte in (b'A'..=b'Z').chain(b'a'..=b'z') {
            set.insert(byte);
        }
        set
    }

    /// Whether `ch` is in the set. A non-ASCII character never is.
    pub fn contains(self, ch: char) -> bool {
        ch.is_ascii() && self.0 & (1u128 << ch as u32) != 0
    }

    /// Whether every character of `self` is also in `other`.
    pub fn is_subset(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    /// The characters in the set, in ascending order.
    pub fn chars(self) -> impl Iterator<Item = char> {
        (0u8..128)
            .filter(move |byte| self.0 & (1u128 << byte) != 0)
            .map(char::from)
    }

    fn insert(&mut self, byte: u8) {
        self.0 |= 1u128 << byte;
    }

    fn remove(&mut self, byte: u8) {
        self.0 &= !(1u128 << byte);
    }

    fn contains_byte(self, byte: u8) -> bool {
        self.0 & (1u128 << byte) != 0
    }

    /// A regex character class for the set. Letters and digits appear as
    /// whole ranges, which is all an option set can produce; each other
    /// character is escaped where a class would read it as syntax.
    fn regex_class(self) -> String {
        let mut class = String::from("[");
        for (low, high) in [(b'A', b'Z'), (b'a', b'z'), (b'0', b'9')] {
            if (low..=high).all(|byte| self.contains_byte(byte)) {
                class.push(low as char);
                class.push('-');
                class.push(high as char);
            }
        }
        for ch in self.chars().filter(|ch| !ch.is_ascii_alphanumeric()) {
            if matches!(ch, '-' | '.' | '\\' | ']' | '[' | '^') {
                class.push('\\');
            }
            class.push(ch);
        }
        class.push(']');
        class
    }
}

/// A set of [`AlphanumericOption`]s.
///
/// The set may hold redundant options exactly as authored; identity,
/// serialization, and lowering use [`AlphanumericOptions::canonical`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AlphanumericOptions(u16);

impl AlphanumericOptions {
    /// The empty set: the base type, letters and digits only.
    pub fn base() -> Self {
        Self::default()
    }

    /// Whether `option` is in the set.
    pub fn contains(self, option: AlphanumericOption) -> bool {
        self.0 & option.bit() != 0
    }

    /// The set with `option` added.
    pub fn with(self, option: AlphanumericOption) -> Self {
        Self(self.0 | option.bit())
    }

    /// The set without the options that cannot change the accepted values:
    /// a leading or double-character block whose character is not allowed,
    /// and both leading blocks under `leading-alpha`.
    pub fn canonical(self) -> Self {
        use AlphanumericOption::*;
        let mut canonical = self;
        let mut drop = |option: AlphanumericOption, redundant: bool| {
            if redundant {
                canonical.0 &= !option.bit();
            }
        };
        drop(BlockLeadingDash, !self.contains(AllowDashes) || self.contains(LeadingAlpha));
        drop(BlockLeadingSlash, !self.contains(AllowSlashes) || self.contains(LeadingAlpha));
        drop(BlockDoublePeriods, !self.contains(AllowPeriods));
        canonical
    }

    /// The options in the set, in canonical (table) order.
    pub fn options(self) -> impl Iterator<Item = AlphanumericOption> {
        AlphanumericOption::ALL
            .into_iter()
            .filter(move |option| self.contains(*option))
    }

    /// The keywords of the options in the set, in canonical (table) order.
    pub fn keywords(self) -> impl Iterator<Item = &'static str> {
        self.options().map(AlphanumericOption::as_keyword)
    }

    /// Every character a value may contain.
    pub fn allowed_characters(self) -> AsciiCharSet {
        use AlphanumericOption::*;
        let mut set = AsciiCharSet::alphanumeric();
        for (option, byte) in [
            (AllowDashes, b'-'),
            (AllowUnderscores, b'_'),
            (AllowSlashes, b'/'),
            (AllowPeriods, b'.'),
            (AllowWhitespace, b' '),
        ] {
            if self.contains(option) {
                set.insert(byte);
            }
        }
        set
    }

    /// Every character a non-empty value may start with.
    pub fn first_characters(self) -> AsciiCharSet {
        use AlphanumericOption::*;
        if self.contains(LeadingAlpha) {
            return AsciiCharSet::letters();
        }
        let mut set = self.allowed_characters();
        if self.contains(BlockLeadingDash) {
            set.remove(b'-');
        }
        if self.contains(BlockLeadingSlash) {
            set.remove(b'/');
        }
        set
    }

    /// Whether no accepted value contains `..`: periods are not allowed, or
    /// double periods are blocked.
    pub fn forbids_double_period(self) -> bool {
        !self.contains(AlphanumericOption::AllowPeriods)
            || self.contains(AlphanumericOption::BlockDoublePeriods)
    }

    /// Whether `value` is accepted. Agrees with [`AlphanumericOptions::pattern`].
    pub fn accepts(self, value: &str) -> bool {
        let allowed = self.allowed_characters();
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return true;
        };
        self.first_characters().contains(first)
            && chars.all(|ch| allowed.contains(ch))
            && !(self.forbids_double_period() && value.contains(".."))
    }

    /// An anchored regex accepting exactly the values of this option set,
    /// including the empty string.
    ///
    /// The regex uses no lookaround or backreference, so the validator's
    /// linear-time engine compiles it, and `^`/`$` anchor to the whole value
    /// (no implicit trailing-newline match).
    pub fn pattern(self) -> String {
        let options = self.canonical();
        let allowed = options.allowed_characters();
        let first = options.first_characters();
        if !(options.contains(AlphanumericOption::AllowPeriods)
            && options.contains(AlphanumericOption::BlockDoublePeriods))
        {
            return format!("^(?:{}{}*)?$", first.regex_class(), allowed.regex_class());
        }
        // Every period must be followed by a non-period or the end, so the
        // value is a lead and then runs of non-periods each optionally
        // closed by one period.
        let mut non_period = allowed;
        non_period.remove(b'.');
        let mut first_non_period = first;
        first_non_period.remove(b'.');
        let lead = if first.contains('.') {
            format!(r"(?:{}\.?|\.)", first_non_period.regex_class())
        } else {
            format!(r"{}\.?", first_non_period.regex_class())
        };
        format!(r"^(?:{lead}(?:{}+\.?)*)?$", non_period.regex_class())
    }
}

impl FromIterator<AlphanumericOption> for AlphanumericOptions {
    fn from_iter<I: IntoIterator<Item = AlphanumericOption>>(iter: I) -> Self {
        iter.into_iter().fold(Self::base(), Self::with)
    }
}

#[cfg(test)]
mod tests {
    use super::AlphanumericOption::*;
    use super::*;

    fn options(list: &[AlphanumericOption]) -> AlphanumericOptions {
        list.iter().copied().collect()
    }

    #[test]
    fn keywords_round_trip_in_table_order() {
        let keywords: Vec<_> = AlphanumericOption::ALL.iter().map(|o| o.as_keyword()).collect();
        assert_eq!(
            keywords,
            [
                "allow-dashes",
                "allow-underscores",
                "allow-slashes",
                "allow-periods",
                "allow-whitespace",
                "block-double-periods",
                "block-leading-dash",
                "block-leading-slash",
                "leading-alpha",
            ]
        );
        for option in AlphanumericOption::ALL {
            assert_eq!(AlphanumericOption::from_keyword(option.as_keyword()), Some(option));
        }
        assert_eq!(AlphanumericOption::from_keyword("Allow-Dashes"), None);
        assert_eq!(AlphanumericOption::from_keyword("allow-dash"), None);
    }

    #[test]
    fn canonical_order_is_table_order_regardless_of_input_order() {
        let set = options(&[LeadingAlpha, AllowPeriods, AllowDashes]);
        assert_eq!(
            set.canonical().keywords().collect::<Vec<_>>(),
            ["allow-dashes", "allow-periods", "leading-alpha"]
        );
    }

    #[test]
    fn canonical_drops_blocks_whose_character_is_not_allowed() {
        assert_eq!(options(&[BlockLeadingDash]).canonical(), AlphanumericOptions::base());
        assert_eq!(options(&[BlockDoublePeriods]).canonical(), AlphanumericOptions::base());
        assert_eq!(options(&[BlockLeadingSlash]).canonical(), AlphanumericOptions::base());
        assert_eq!(
            options(&[AllowDashes, BlockLeadingDash]).canonical(),
            options(&[AllowDashes, BlockLeadingDash])
        );
    }

    #[test]
    fn canonical_drops_leading_blocks_under_leading_alpha() {
        assert_eq!(
            options(&[LeadingAlpha, AllowDashes, AllowSlashes, BlockLeadingDash, BlockLeadingSlash])
                .canonical(),
            options(&[LeadingAlpha, AllowDashes, AllowSlashes])
        );
    }

    #[test]
    fn canonical_preserves_meaning() {
        // Every subset of the nine options accepts the same samples as its
        // canonical form, and its regex agrees.
        let samples = [
            "", "a", "1", "-", "/", ".", "_", " ", "a-b", "-a", "/a", ".a", "a..b", "a.b", "a b",
            "1abc", "a/b", "_a", "é", "a\tb", "a\n",
        ];
        for bits in 0u16..(1 << 9) {
            let set = AlphanumericOptions(bits);
            let canonical = set.canonical();
            let regex = regex::Regex::new(&set.pattern()).expect("pattern compiles");
            for sample in samples {
                assert_eq!(set.accepts(sample), canonical.accepts(sample), "{bits:#b} {sample:?}");
                assert_eq!(regex.is_match(sample), set.accepts(sample), "{bits:#b} {sample:?}");
            }
        }
    }

    #[test]
    fn character_sets_reflect_options() {
        let base = AlphanumericOptions::base();
        assert!(base.allowed_characters().contains('a'));
        assert!(!base.allowed_characters().contains('-'));
        assert!(!base.allowed_characters().contains('é'));
        assert!(base.forbids_double_period());

        let dashes = options(&[AllowDashes, BlockLeadingDash]);
        assert!(dashes.allowed_characters().contains('-'));
        assert!(!dashes.first_characters().contains('-'));

        let alpha = options(&[LeadingAlpha, AllowUnderscores]);
        assert!(alpha.first_characters().is_subset(AsciiCharSet::letters()));
        assert!(!alpha.first_characters().contains('1'));

        let periods = options(&[AllowPeriods]);
        assert!(!periods.forbids_double_period());
        assert!(options(&[AllowPeriods, BlockDoublePeriods]).forbids_double_period());
    }

    #[test]
    fn regex_class_escapes_class_syntax() {
        let class = options(&[AllowDashes, AllowPeriods, AllowSlashes, AllowUnderscores, AllowWhitespace])
            .allowed_characters()
            .regex_class();
        assert_eq!(class, r"[A-Za-z0-9 \-\./_]");
    }
}
