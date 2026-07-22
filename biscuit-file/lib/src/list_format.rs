//! List-shape classification and delimiter-aware conversion.
//!
//! Alongside [`DataFormat`](crate::DataFormat) (structured file formats) and
//! [`FileType`](crate::FileType) (file-type detection), this module owns the
//! third detection axis: **how a plain string encodes a list of entries**.
//!
//! A single string carrying several values can arrive in many shapes — a
//! Markdown list, one entry per line, a CSV/TSV row, or a space-separated run.
//! Rather than force callers to declare which one they produced,
//! [`ListFormat::classify`] inspects the string and picks a shape, and
//! [`ListFormat::split`] turns it into ordered entries.
//!
//! Typed lists (e.g. a JSON/YAML array) do **not** belong here: they are
//! already ordered entries and bypass classification entirely. `ListFormat`
//! exists only for string inputs such as shell-expansion output or a raw
//! textual source.
//!
//! ## Classification precedence
//!
//! [`ListFormat::classify`] applies a fixed precedence so ambiguous inputs
//! resolve deterministically:
//!
//! 1. Markdown list markers (ordered `1.` / unordered `-`, `*`, `+`)
//! 2. more than one non-empty line → line-separated
//! 3. an unquoted tab → TSV
//! 4. an unquoted comma → CSV
//! 5. an unquoted space → space-separated
//! 6. otherwise → a single-item (scalar) list
//!
//! Steps 3–5 are quote-aware: a delimiter that appears only inside a quoted
//! field does not trigger that format.

/// The detected shape of a string that encodes a list of entries.
///
/// Obtain one with [`ListFormat::classify`], then call [`ListFormat::split`] to
/// produce the ordered entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListFormat {
    /// A Markdown ordered list (`1. one`, `2. two`).
    MarkdownOrdered,
    /// A Markdown unordered list (`- one`, `* two`, `+ three`).
    MarkdownUnordered,
    /// One entry per line.
    LineSeparated,
    /// Tab-separated values on a single line.
    Tsv,
    /// Comma-separated values on a single line.
    Csv,
    /// Space-separated values on a single line.
    SpaceSeparated,
    /// A single entry (no recognized delimiter).
    Scalar,
}

impl ListFormat {
    /// Classifies `input` into a [`ListFormat`] using the documented precedence.
    ///
    /// The input's newlines are normalized (`\r\n` and lone `\r` become `\n`)
    /// before line-based detection, so CRLF sources classify identically to LF
    /// sources.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::ListFormat;
    ///
    /// assert_eq!(ListFormat::classify("- a\n- b"), ListFormat::MarkdownUnordered);
    /// assert_eq!(ListFormat::classify("1. a\n2. b"), ListFormat::MarkdownOrdered);
    /// assert_eq!(ListFormat::classify("a\nb\nc"), ListFormat::LineSeparated);
    /// assert_eq!(ListFormat::classify("a\tb"), ListFormat::Tsv);
    /// assert_eq!(ListFormat::classify("a, b, c"), ListFormat::Csv);
    /// assert_eq!(ListFormat::classify("a b c"), ListFormat::SpaceSeparated);
    /// assert_eq!(ListFormat::classify("solo"), ListFormat::Scalar);
    /// ```
    #[must_use]
    pub fn classify(input: &str) -> Self {
        let normalized = normalize_newlines(input);

        let non_empty: Vec<&str> = normalized
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .collect();

        // 1. Markdown list markers — every non-empty line must carry a marker.
        if !non_empty.is_empty() {
            if non_empty.iter().all(|l| unordered_marker_len(l).is_some()) {
                return Self::MarkdownUnordered;
            }
            if non_empty.iter().all(|l| ordered_marker_len(l).is_some()) {
                return Self::MarkdownOrdered;
            }
        }

        // 2. Multiple non-empty lines → line-separated.
        if non_empty.len() > 1 {
            return Self::LineSeparated;
        }

        // Steps 3–5 examine the single content line, ignoring delimiters that
        // appear inside quoted fields.
        let line = non_empty.first().copied().unwrap_or("");
        if has_unquoted(line, '\t') {
            return Self::Tsv;
        }
        if has_unquoted(line, ',') {
            return Self::Csv;
        }
        if has_unquoted(line, ' ') {
            return Self::SpaceSeparated;
        }

        // 6. No recognized delimiter → a one-item list.
        Self::Scalar
    }

    /// Splits `input` into ordered entries according to this format.
    ///
    /// Delimiter-aware formats (CSV, TSV, space-separated, scalar) honor quoted
    /// fields: a delimiter inside quotes is literal, `""` inside a quoted field
    /// is a literal `"`, and an unquoted field is trimmed of surrounding
    /// whitespace while its interior whitespace is preserved. Markdown and
    /// line-separated formats split on lines. Whitespace-only entries are
    /// dropped in every format; interior whitespace and Unicode are preserved.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::ListFormat;
    ///
    /// assert_eq!(
    ///     ListFormat::Csv.split(r#"a,"b,c",d"#),
    ///     vec!["a".to_string(), "b,c".to_string(), "d".to_string()],
    /// );
    /// assert_eq!(
    ///     ListFormat::MarkdownUnordered.split("- one\n- two"),
    ///     vec!["one".to_string(), "two".to_string()],
    /// );
    /// ```
    #[must_use]
    pub fn split(self, input: &str) -> Vec<String> {
        let normalized = normalize_newlines(input);
        match self {
            Self::MarkdownOrdered => split_markers(&normalized, ordered_marker_len),
            Self::MarkdownUnordered => split_markers(&normalized, unordered_marker_len),
            Self::LineSeparated => normalized
                .split('\n')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(ToString::to_string)
                .collect(),
            Self::Tsv => split_delimited(single_line(&normalized), '\t'),
            Self::Csv => split_delimited(single_line(&normalized), ','),
            Self::SpaceSeparated => split_delimited(single_line(&normalized), ' '),
            // A scalar is one field; run it through the same quote/trim logic
            // (unwraps a fully-quoted value) using a delimiter it cannot contain.
            Self::Scalar => split_delimited(single_line(&normalized), '\0'),
        }
    }
}

/// Classifies `input` and returns both the detected [`ListFormat`] and the
/// ordered entries it yields.
///
/// Convenience for the common pairing of [`ListFormat::classify`] followed by
/// [`ListFormat::split`].
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::{ListFormat, classify_list};
///
/// let (format, entries) = classify_list("a, b, c");
/// assert_eq!(format, ListFormat::Csv);
/// assert_eq!(entries, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
/// ```
#[must_use]
pub fn classify_list(input: &str) -> (ListFormat, Vec<String>) {
    let format = ListFormat::classify(input);
    (format, format.split(input))
}

/// Normalizes CRLF (`\r\n`) and lone CR (`\r`) line endings to LF (`\n`).
fn normalize_newlines(input: &str) -> String {
    if !input.contains('\r') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Returns the first non-empty line of a normalized string (or `""`).
///
/// Delimited formats are single-line by construction (multiple non-empty lines
/// classify as line-separated first), so this collapses a stray trailing
/// newline without discarding the content line.
fn single_line(normalized: &str) -> &str {
    normalized
        .split('\n')
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

/// Length of a leading Markdown unordered-list marker (indentation + bullet +
/// at least one space), or `None` if the line is not a bullet.
fn unordered_marker_len(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let mut chars = trimmed.chars();
    let bullet = chars.next()?;
    if !matches!(bullet, '-' | '*' | '+') {
        return None;
    }
    // A bullet must be followed by whitespace to distinguish `- x` from `-x`.
    let after = chars.next()?;
    if !after.is_whitespace() {
        return None;
    }
    Some(indent + trimmed.len() - chars.as_str().len())
}

/// Length of a leading Markdown ordered-list marker (indentation + `N.` + at
/// least one space), or `None` if the line is not an ordered item.
fn ordered_marker_len(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let mut rest = trimmed[digits.len()..].chars();
    if rest.next()? != '.' {
        return None;
    }
    let after = rest.next()?;
    if !after.is_whitespace() {
        return None;
    }
    Some(indent + trimmed.len() - rest.as_str().len())
}

/// Splits a Markdown list into entries by stripping each line's marker.
fn split_markers(normalized: &str, marker_len: fn(&str) -> Option<usize>) -> Vec<String> {
    normalized
        .split('\n')
        .filter_map(|line| {
            let len = marker_len(line)?;
            let entry = line[len..].trim();
            (!entry.is_empty()).then(|| entry.to_string())
        })
        .collect()
}

/// Returns `true` if `delim` appears in `line` outside any quoted field.
fn has_unquoted(line: &str, delim: char) -> bool {
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == delim {
            return true;
        }
    }
    false
}

/// Splits a single line on `delim`, honoring quoted fields and escaped quotes.
///
/// Quoted content is taken verbatim (protecting delimiters, and turning `""`
/// into a literal `"`); an unquoted field is trimmed of surrounding whitespace.
/// Empty and whitespace-only entries are dropped.
fn split_delimited(line: &str, delim: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted_run = false;
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
            quoted_run = true;
        } else if ch == delim {
            push_field(&mut fields, &current, quoted_run);
            current.clear();
            quoted_run = false;
        } else {
            current.push(ch);
        }
    }
    push_field(&mut fields, &current, quoted_run);
    fields
}

/// Finalizes one delimited field: quoted fields are kept verbatim, unquoted
/// fields are trimmed, and whitespace-only results are dropped.
fn push_field(fields: &mut Vec<String>, raw: &str, quoted: bool) {
    let value = if quoted { raw } else { raw.trim() };
    if !value.is_empty() {
        fields.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_markdown_unordered() {
        assert_eq!(ListFormat::classify("- a\n- b\n- c"), ListFormat::MarkdownUnordered);
        assert_eq!(ListFormat::classify("* a\n+ b"), ListFormat::MarkdownUnordered);
        // Single bullet line is still an unordered list.
        assert_eq!(ListFormat::classify("- only"), ListFormat::MarkdownUnordered);
        // Indented bullets (the darkmatter renderer's nested form).
        assert_eq!(ListFormat::classify("- a\n  - b"), ListFormat::MarkdownUnordered);
    }

    #[test]
    fn classifies_markdown_ordered() {
        assert_eq!(ListFormat::classify("1. a\n2. b"), ListFormat::MarkdownOrdered);
        assert_eq!(ListFormat::classify("1. solo"), ListFormat::MarkdownOrdered);
    }

    #[test]
    fn dash_without_space_is_not_a_bullet() {
        // `-a` is not a Markdown bullet; falls through to scalar.
        assert_eq!(ListFormat::classify("-a"), ListFormat::Scalar);
    }

    #[test]
    fn classifies_line_separated() {
        assert_eq!(ListFormat::classify("a\nb\nc"), ListFormat::LineSeparated);
        // Mixed marker/non-marker lines are not a clean Markdown list.
        assert_eq!(ListFormat::classify("- a\nplain"), ListFormat::LineSeparated);
    }

    #[test]
    fn precedence_line_before_delimiters() {
        // Multiple lines win over an in-line comma or tab.
        assert_eq!(ListFormat::classify("a,b\nc,d"), ListFormat::LineSeparated);
        assert_eq!(ListFormat::classify("a\tb\nc"), ListFormat::LineSeparated);
    }

    #[test]
    fn precedence_tsv_before_csv_before_space() {
        assert_eq!(ListFormat::classify("a\tb,c d"), ListFormat::Tsv);
        assert_eq!(ListFormat::classify("a,b c"), ListFormat::Csv);
        assert_eq!(ListFormat::classify("a b c"), ListFormat::SpaceSeparated);
    }

    #[test]
    fn classifies_scalar() {
        assert_eq!(ListFormat::classify("hello"), ListFormat::Scalar);
        assert_eq!(ListFormat::classify(""), ListFormat::Scalar);
        // A comma that appears only inside quotes does not make it CSV.
        assert_eq!(ListFormat::classify("\"a,b\""), ListFormat::Scalar);
    }

    #[test]
    fn splits_csv_with_spaces() {
        assert_eq!(
            ListFormat::Csv.split("a, b, c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn splits_csv_quoted_delimiters() {
        assert_eq!(
            ListFormat::Csv.split(r#"a,"b,c",d"#),
            vec!["a".to_string(), "b,c".to_string(), "d".to_string()]
        );
    }

    #[test]
    fn splits_csv_escaped_quotes() {
        // `""` inside a quoted field is a literal quote.
        assert_eq!(
            ListFormat::Csv.split(r#""say ""hi""",bye"#),
            vec!["say \"hi\"".to_string(), "bye".to_string()]
        );
    }

    #[test]
    fn quoted_field_preserves_interior_whitespace() {
        // Explicitly quoted leading/trailing spaces survive; unquoted are trimmed.
        assert_eq!(
            ListFormat::Csv.split(r#""  padded  ", trimmed "#),
            vec!["  padded  ".to_string(), "trimmed".to_string()]
        );
    }

    #[test]
    fn splits_tsv_quote_aware() {
        assert_eq!(
            ListFormat::Tsv.split("a\t\"b\tc\"\td"),
            vec!["a".to_string(), "b\tc".to_string(), "d".to_string()]
        );
    }

    #[test]
    fn splits_space_separated() {
        assert_eq!(
            ListFormat::SpaceSeparated.split("one  two   three"),
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }

    #[test]
    fn splits_scalar_unwraps_quotes() {
        assert_eq!(ListFormat::Scalar.split("solo"), vec!["solo".to_string()]);
        // A fully-quoted scalar keeps its interior verbatim.
        assert_eq!(
            ListFormat::Scalar.split("\"a, b with spaces\""),
            vec!["a, b with spaces".to_string()]
        );
    }

    #[test]
    fn drops_whitespace_only_entries() {
        assert_eq!(
            ListFormat::Csv.split("a, , ,b"),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(
            ListFormat::LineSeparated.split("a\n\n  \nb"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn preserves_interior_whitespace_line_separated() {
        assert_eq!(
            ListFormat::LineSeparated.split("hello world\nfoo bar"),
            vec!["hello world".to_string(), "foo bar".to_string()]
        );
    }

    #[test]
    fn preserves_unicode() {
        assert_eq!(
            ListFormat::Csv.split("café, naïve, 日本語"),
            vec!["café".to_string(), "naïve".to_string(), "日本語".to_string()]
        );
    }

    #[test]
    fn normalizes_crlf() {
        assert_eq!(ListFormat::classify("a\r\nb\r\nc"), ListFormat::LineSeparated);
        assert_eq!(
            ListFormat::LineSeparated.split("a\r\nb\r\nc"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        // A lone CR is also normalized.
        assert_eq!(
            ListFormat::LineSeparated.split("a\rb"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn splits_markdown_ordered_strips_markers() {
        assert_eq!(
            ListFormat::MarkdownOrdered.split("1. first\n2. second\n10. tenth"),
            vec!["first".to_string(), "second".to_string(), "tenth".to_string()]
        );
    }

    #[test]
    fn splits_markdown_unordered_strips_markers_and_indent() {
        assert_eq!(
            ListFormat::MarkdownUnordered.split("- a\n  - b\n* c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn classify_list_pairs_format_and_entries() {
        let (format, entries) = classify_list("x\ty\tz");
        assert_eq!(format, ListFormat::Tsv);
        assert_eq!(entries, vec!["x".to_string(), "y".to_string(), "z".to_string()]);
    }

    #[test]
    fn round_trips_darkmatter_csv_form() {
        // Darkmatter's `as_csv` emits `", "` between entries; classification and
        // splitting must recover the original entries.
        let rendered = "alpha, beta, gamma";
        let (format, entries) = classify_list(rendered);
        assert_eq!(format, ListFormat::Csv);
        assert_eq!(
            entries,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
    }
}
