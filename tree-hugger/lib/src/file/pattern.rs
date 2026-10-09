//! Name and type-text patterns: the one matcher behind every symbol name,
//! container and type filter.
//!
//! `*` is the only wildcard. Name patterns use smart case; type globs are
//! case-sensitive and whole-string. The user-facing rules are in
//! `docs/symbol-reports/selectors.md`.

use crate::shared::ProgrammingLanguage;

/// Whether `value` is exactly `pattern`, where each `*` in `pattern` matches
/// any sequence of characters (including none). Case-sensitive; no other
/// character is special.
///
/// ## Examples
///
/// ```
/// use tree_hugger::glob_match;
///
/// assert!(glob_match("Result<*", "Result<(), Error>"));
/// assert!(!glob_match("Result", "Result<()>"));
/// assert!(glob_match("*Error", "TreeHuggerError"));
/// ```
pub fn glob_match(pattern: &str, value: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut remaining) = value.strip_prefix(first) else {
        return false;
    };
    let rest: Vec<&str> = parts.collect();
    let Some((last, middle)) = rest.split_last() else {
        // No `*`: the whole value had to be the pattern.
        return remaining.is_empty();
    };
    for part in middle {
        match remaining.find(part) {
            Some(found) => remaining = &remaining[found + part.len()..],
            None => return false,
        }
    }
    remaining.ends_with(last)
}

/// A symbol or container name pattern with smart-case matching.
///
/// - A pattern with no uppercase letter matches case-insensitively; any
///   uppercase letter makes it case-sensitive.
/// - A pattern with `*` matches the whole candidate, `*` matching any
///   sequence; one without matches any substring.
/// - An exact pattern ([`exact`](Self::exact), or a trailing `!`) matches the
///   whole candidate literally, `*` included, still under smart case.
///
/// Case is folded with [`str::to_lowercase`] on both sides, so no byte offset
/// ever splits a character.
///
/// ## Examples
///
/// ```
/// use tree_hugger::NamePattern;
///
/// assert!(NamePattern::new("parse").matches("parseWidth"));
/// assert!(NamePattern::new("parse").matches("ParseWidth"));
/// assert!(!NamePattern::new("Parse").matches("parseWidth"));
/// assert!(NamePattern::new("parse!").matches("Parse"));
/// assert!(!NamePattern::new("parse!").matches("parseWidth"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePattern {
    /// The pattern as given, without a trailing `!`.
    text: String,
    /// `text` lowercased when matching is case-insensitive.
    folded: String,
    case_sensitive: bool,
    exact: bool,
}

impl NamePattern {
    /// A pattern; a trailing `!` (after at least one other character) makes
    /// it exact.
    pub fn new(pattern: &str) -> Self {
        match pattern.strip_suffix('!').filter(|stripped| !stripped.is_empty()) {
            Some(stripped) => Self::build(stripped, true),
            None => Self::build(pattern, false),
        }
    }

    /// An exact pattern: the whole candidate, literally (`*` included), under
    /// smart case. A trailing `!` is dropped as in [`new`](Self::new).
    pub fn exact(pattern: &str) -> Self {
        let stripped = pattern
            .strip_suffix('!')
            .filter(|stripped| !stripped.is_empty())
            .unwrap_or(pattern);
        Self::build(stripped, true)
    }

    fn build(text: &str, exact: bool) -> Self {
        let case_sensitive = text.chars().any(char::is_uppercase);
        Self {
            text: text.to_string(),
            folded: if case_sensitive {
                text.to_string()
            } else {
                text.to_lowercase()
            },
            case_sensitive,
            exact,
        }
    }

    /// The pattern as given, without a trailing `!`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[allow(missing_docs)]
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    #[allow(missing_docs)]
    pub fn is_exact(&self) -> bool {
        self.exact
    }

    /// Whether `candidate` matches.
    pub fn matches(&self, candidate: &str) -> bool {
        let folded;
        let candidate = if self.case_sensitive {
            candidate
        } else {
            folded = candidate.to_lowercase();
            folded.as_str()
        };
        if self.exact {
            candidate == self.folded
        } else if self.folded.contains('*') {
            glob_match(&self.folded, candidate)
        } else {
            candidate.contains(self.folded.as_str())
        }
    }

    /// Whether the pattern names a qualified name in `language`: it contains
    /// the file separator `#` or one of the language's own qualified-name
    /// separators (`::` in Rust, C++ and Perl; `\` or `::` in PHP; `.` or `:`
    /// in Lua; `.` elsewhere).
    pub fn is_qualified_for(&self, language: ProgrammingLanguage) -> bool {
        self.text.contains('#')
            || qualified_name_separators(language)
                .iter()
                .any(|separator| self.text.contains(separator))
    }
}

/// The separators the in-file qualified names of `language` use, longest
/// first.
pub(crate) fn qualified_name_separators(language: ProgrammingLanguage) -> &'static [&'static str] {
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp | ProgrammingLanguage::Perl => &["::"],
        ProgrammingLanguage::Php => &["::", "\\"],
        ProgrammingLanguage::Lua => &[".", ":"],
        _ => &["."],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_the_whole_value() {
        let cases = [
            ("*", "", true),
            ("*", "anything", true),
            ("", "", true),
            ("", "a", false),
            ("ab", "ab", true),
            // A literal pattern is equality, not prefix-and-suffix.
            ("ab", "abab", false),
            ("ab*", "abc", true),
            ("*bc", "abc", true),
            ("a*c", "abc", true),
            ("a*c", "ac", true),
            ("ab*bc", "abc", false),
            ("ab*bc", "abbc", true),
            ("a*b*c", "aXbYc", true),
            ("a*b*c", "aXcYb", false),
            ("**", "x", true),
            ("Result<*", "Result<T, E>", true),
            ("Result<*", "result<T>", false),
            ("Ü*", "Ünï", true),
            ("*ï", "Ünï", true),
            ("*n*", "Ünï", true),
        ];
        for (pattern, value, expected) in cases {
            assert_eq!(glob_match(pattern, value), expected, "{pattern:?} against {value:?}");
        }
    }

    #[test]
    fn name_patterns_follow_smart_case_and_exactness() {
        let cases = [
            // (pattern, exact flag, candidate, expected)
            ("parse", false, "parse_width", true),
            ("parse", false, "PARSE_WIDTH", true),
            ("Parse", false, "parse_width", false),
            ("Parse", false, "ParseWidth", true),
            ("width", false, "parse_width", true),
            ("parse*", false, "parse_width", true),
            ("parse*", false, "my_parse_width", false),
            ("*width", false, "parse_width", true),
            ("parse_width!", false, "parse_width", true),
            ("parse_width!", false, "parse_width_spec", false),
            ("parse_width!", false, "Parse_Width", true),
            ("Parse_width!", false, "parse_width", false),
            ("parse_width", true, "parse_width_spec", false),
            ("parse_width", true, "PARSE_WIDTH", true),
            ("a*b", true, "a*b", true),
            ("a*b", true, "axb", false),
            ("a*b!", false, "axb", false),
            ("a*b!", false, "A*B", true),
            ("!", false, "a!b", true),
            ("", false, "anything", true),
        ];
        for (pattern, exact, candidate, expected) in cases {
            let parsed = if exact { NamePattern::exact(pattern) } else { NamePattern::new(pattern) };
            assert_eq!(parsed.matches(candidate), expected, "{pattern:?} (exact {exact}) against {candidate:?}");
        }
    }

    #[test]
    fn non_ascii_names_fold_whole_characters() {
        let cases = [
            ("ünï", "ÜNÏ", true),
            ("ÜNÏ", "ünï", false),
            ("Ünï", "Ünï", true),
            ("ünï!", "ÜNÏ", true),
            ("ü*", "Übung", true),
            ("*ße", "Straße", true),
            ("*ß", "Straße", false),
            ("straße", "STRASSE", false),
            ("ß", "ẞ", true),
            // Turkish dotted capital I lowercases to `i` plus a combining
            // dot; the lowercase pattern `i` still finds its `i`.
            ("i", "İstanbul", true),
            ("İ", "istanbul", false),
            ("İstanbul!", "İstanbul", true),
            ("名前", "名前空間", true),
            ("*間", "名前空間", true),
        ];
        for (pattern, candidate, expected) in cases {
            assert_eq!(NamePattern::new(pattern).matches(candidate), expected, "{pattern:?} against {candidate:?}");
        }
    }

    #[test]
    fn qualified_patterns_use_the_languages_separators() {
        let rust = ProgrammingLanguage::Rust;
        assert!(NamePattern::new("symbol.rs#*").is_qualified_for(rust));
        assert!(NamePattern::new("Symbol::new").is_qualified_for(rust));
        assert!(!NamePattern::new("Symbol.new").is_qualified_for(rust));
        assert!(NamePattern::new("Point.x").is_qualified_for(ProgrammingLanguage::Java));
        assert!(!NamePattern::new("Point::x").is_qualified_for(ProgrammingLanguage::Java));
        assert!(NamePattern::new(r"App\Point").is_qualified_for(ProgrammingLanguage::Php));
        assert!(NamePattern::new("M:m").is_qualified_for(ProgrammingLanguage::Lua));
        assert!(!NamePattern::new("plain").is_qualified_for(ProgrammingLanguage::Lua));
    }
}
