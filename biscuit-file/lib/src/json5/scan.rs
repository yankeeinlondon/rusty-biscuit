//! What a JSON5 value tree no longer carries: duplicate keys and each number's
//! written spelling.
//!
//! [`Json5`](super::Json5) deserializes into `serde_json::Value`, where a later
//! duplicate has already replaced an earlier one and a number has lost its
//! spelling. This reads the same text at the token level to recover both.

use std::collections::HashSet;

use json_five::tokenize::{TokType, tokenize_str};

use super::Json5Error;

/// The result of [`json5_scan`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Json5Scan {
    /// The first key repeated within one object, in document order.
    pub duplicate_key: Option<Json5DuplicateKey>,
    /// Every number in the document, in document order, including one a later
    /// duplicate key shadows.
    pub numbers: Vec<Json5Number>,
}

/// A key written more than once in the same object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json5DuplicateKey {
    /// The decoded key.
    pub key: String,
    /// JSON Pointer to the object holding it (`""` for the root).
    pub object_pointer: String,
    /// Byte offset of the repeated key.
    pub offset: usize,
}

/// One number as the document wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json5Number {
    /// The spelling with its sign and without any whitespace after the sign:
    /// `-1.50`, `+0x10`, `Infinity`.
    pub spelling: String,
    /// JSON Pointer to the number (`""` for a root number).
    pub pointer: String,
    /// Byte offset of the number's first token, its sign when it has one.
    pub offset: usize,
}

enum Frame {
    Object { keys: HashSet<String>, expecting_key: bool, pointer_length: usize },
    Array { next_index: usize, pointer_length: usize },
}

/// Reads `text`'s duplicate keys and number spellings.
///
/// Keys compare after decoding, so `{a:1,"a":2}` is a duplicate. The scan
/// checks tokens rather than grammar, so call it on text that has already
/// parsed; on text that has not, its result describes whatever tokens it found.
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::json5_scan;
///
/// let scan = json5_scan("{a: 1.10, b: [-2], a: 3}")?;
/// assert_eq!(scan.duplicate_key.unwrap().key, "a");
/// let spellings: Vec<_> = scan.numbers.iter().map(|n| n.spelling.as_str()).collect();
/// assert_eq!(spellings, ["1.10", "-2", "3"]);
/// # Ok::<(), biscuit_file::Json5Error>(())
/// ```
///
/// ## Errors
///
/// [`Json5Error::Parse`] when `text` does not tokenize.
pub fn json5_scan(text: &str) -> Result<Json5Scan, Json5Error> {
    let tokens = tokenize_str(text).map_err(|error| Json5Error::Parse(error.to_string()))?;
    let mut scan = Json5Scan::default();
    let mut frames: Vec<Frame> = Vec::new();
    let mut pointer = String::new();
    let mut sign: Option<(usize, &str)> = None;

    for (start, kind, end) in &tokens.tok_spans {
        let lexeme = &text[*start..*end];
        if let Some(Frame::Object { keys, expecting_key: expecting_key @ true, .. }) = frames.last_mut()
            && !matches!(kind, TokType::RightBrace | TokType::Comma | TokType::EOF)
        {
            let key = decode_key(lexeme, kind);
            if !keys.insert(key.clone()) && scan.duplicate_key.is_none() {
                scan.duplicate_key = Some(Json5DuplicateKey {
                    key: key.clone(),
                    object_pointer: pointer.clone(),
                    offset: *start,
                });
            }
            *expecting_key = false;
            push_segment(&mut pointer, &key);
            continue;
        }
        match kind {
            TokType::Comma | TokType::Colon | TokType::Whitespace | TokType::BlockComment | TokType::LineComment => {}
            TokType::EOF => break,
            TokType::LeftBrace => {
                begin_value(&mut frames, &mut pointer);
                frames.push(Frame::Object {
                    keys: HashSet::new(),
                    expecting_key: true,
                    pointer_length: pointer.len(),
                });
            }
            TokType::LeftBracket => {
                begin_value(&mut frames, &mut pointer);
                frames.push(Frame::Array { next_index: 0, pointer_length: pointer.len() });
            }
            TokType::RightBrace | TokType::RightBracket => {
                frames.pop();
                end_value(&mut frames, &mut pointer);
            }
            TokType::Plus | TokType::Minus => sign = Some((*start, lexeme)),
            TokType::Integer | TokType::Float | TokType::Exponent | TokType::Hexadecimal | TokType::Infinity
            | TokType::Nan => {
                begin_value(&mut frames, &mut pointer);
                let (offset, written_sign) = sign.take().unwrap_or((*start, ""));
                scan.numbers.push(Json5Number {
                    spelling: format!("{written_sign}{lexeme}"),
                    pointer: pointer.clone(),
                    offset,
                });
                end_value(&mut frames, &mut pointer);
            }
            TokType::Name
            | TokType::SingleQuotedString
            | TokType::DoubleQuotedString
            | TokType::True
            | TokType::False
            | TokType::Null => {
                begin_value(&mut frames, &mut pointer);
                end_value(&mut frames, &mut pointer);
            }
        }
    }
    Ok(scan)
}

/// Whether `text` is grammatical JSON5, whether or not a value can be built.
///
/// [`Json5::from_str`](super::Json5::from_str) also fails on an integer beyond
/// `u64`, which is a problem with the number and not with the text's syntax.
/// This tells the two apart.
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::{Json5, json5_is_well_formed};
///
/// let oversized = "{a: 99999999999999999999}";
/// assert!(Json5::from_str(oversized).is_err());
/// assert!(json5_is_well_formed(oversized));
/// assert!(!json5_is_well_formed("{a: 1"));
/// ```
#[must_use]
pub fn json5_is_well_formed(text: &str) -> bool {
    json_five::model_from_str(text).is_ok()
}

/// An array element's pointer segment exists only once the element starts; an
/// object member's was pushed with its key.
fn begin_value(frames: &mut [Frame], pointer: &mut String) {
    if let Some(Frame::Array { next_index, .. }) = frames.last_mut() {
        pointer.push('/');
        pointer.push_str(&next_index.to_string());
    }
}

fn end_value(frames: &mut [Frame], pointer: &mut String) {
    match frames.last_mut() {
        Some(Frame::Object { expecting_key, pointer_length, .. }) => {
            *expecting_key = true;
            pointer.truncate(*pointer_length);
        }
        Some(Frame::Array { next_index, pointer_length }) => {
            *next_index += 1;
            pointer.truncate(*pointer_length);
        }
        None => {}
    }
}

fn push_segment(pointer: &mut String, key: &str) {
    pointer.push('/');
    pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
}

/// Decodes a key by the parser's own rules, so a key compares the way it reads
/// in the value tree. Only text with an escape needs that second parse.
fn decode_key(lexeme: &str, kind: &TokType) -> String {
    let unquoted = match kind {
        TokType::SingleQuotedString | TokType::DoubleQuotedString => &lexeme[1..lexeme.len() - 1],
        _ => lexeme,
    };
    if !lexeme.contains('\\') {
        return unquoted.to_string();
    }
    json_five::from_str::<serde_json::Map<String, serde_json::Value>>(&format!("{{{lexeme}:0}}"))
        .ok()
        .and_then(|map| map.into_iter().next())
        .map_or_else(|| unquoted.to_string(), |(key, _)| key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(text: &str) -> Vec<(String, String)> {
        json5_scan(text)
            .unwrap()
            .numbers
            .into_iter()
            .map(|number| (number.pointer, number.spelling))
            .collect()
    }

    fn pair(pointer: &str, spelling: &str) -> (String, String) {
        (pointer.to_string(), spelling.to_string())
    }

    #[test]
    fn number_spellings_survive_with_their_pointers() {
        assert_eq!(
            numbers("{a: 1.10, b: [+2, {c: -0x10}], 'd/e': 1e3, f: .5, g: 5.}"),
            [
                pair("/a", "1.10"),
                pair("/b/0", "+2"),
                pair("/b/1/c", "-0x10"),
                pair("/d~1e", "1e3"),
                pair("/f", ".5"),
                pair("/g", "5."),
            ]
        );
    }

    #[test]
    fn non_finite_words_and_a_root_number_are_numbers() {
        assert_eq!(numbers("[NaN, -Infinity]"), [pair("/0", "NaN"), pair("/1", "-Infinity")]);
        assert_eq!(numbers("7"), [pair("", "7")]);
    }

    #[test]
    fn a_sign_separated_from_its_number_by_a_comment_joins_it() {
        assert_eq!(numbers("[- /* c */ 1]"), [pair("/0", "-1")]);
    }

    #[test]
    fn strings_and_words_holding_digits_are_not_numbers() {
        assert_eq!(numbers("{a: '1', b: true, c: null, d: \"2\"}"), []);
    }

    #[test]
    fn a_key_spelled_like_a_number_word_is_a_key() {
        let scan = json5_scan("{NaN: 1, Infinity: 2, true: 3, null: 4}").unwrap();
        assert_eq!(scan.duplicate_key, None);
        let pointers: Vec<_> = scan.numbers.iter().map(|number| number.pointer.as_str()).collect();
        assert_eq!(pointers, ["/NaN", "/Infinity", "/true", "/null"]);
    }

    #[test]
    fn a_shadowed_number_is_still_listed() {
        assert_eq!(numbers("{a: 1, a: 2}"), [pair("/a", "1"), pair("/a", "2")]);
    }

    #[test]
    fn duplicates_compare_after_decoding_and_report_the_first() {
        for text in [
            "{a: 1, a: 2}",
            "{a: 1, 'a': 2}",
            "{a: 1, \"a\": 2}",
            r#"{a: 1, "\u0061": 2}"#,
            r"{a: 1, \u0061: 2}",
        ] {
            let duplicate = json5_scan(text).unwrap().duplicate_key.expect(text);
            assert_eq!((duplicate.key.as_str(), duplicate.object_pointer.as_str()), ("a", ""), "{text}");
        }
        let first = json5_scan("{x: {b: 1, b: 2}, a: 1, a: 2}").unwrap().duplicate_key.unwrap();
        assert_eq!((first.key.as_str(), first.object_pointer.as_str()), ("b", "/x"));
    }

    #[test]
    fn the_same_key_in_different_objects_is_no_duplicate() {
        assert_eq!(json5_scan("{a: {a: 1}, b: [{a: 1}, {a: 2}]}").unwrap().duplicate_key, None);
    }

    #[test]
    fn comments_and_trailing_commas_change_nothing() {
        let scan = json5_scan("// c\n{a: /* x */ 1, b: [2,], }").unwrap();
        assert_eq!(scan.duplicate_key, None);
        assert_eq!(scan.numbers.len(), 2);
    }

    #[test]
    fn offsets_point_at_the_written_token() {
        let text = "{a: 1, a: -2}";
        let scan = json5_scan(text).unwrap();
        assert_eq!(&text[scan.duplicate_key.unwrap().offset..][..1], "a");
        assert!(text[scan.numbers[1].offset..].starts_with("-2"));
    }

    #[test]
    fn untokenizable_text_is_a_parse_error() {
        assert!(matches!(json5_scan("{a: 'open"), Err(Json5Error::Parse(_))));
    }
}
