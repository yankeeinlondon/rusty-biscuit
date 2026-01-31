//! XXH64 hashing utilities for fast, non-cryptographic hashing.
//!
//! This module provides wrappers around the xxHash algorithm for common
//! use cases like content hashing, change detection, and caching.
//!
//! ## Core Functions
//!
//! - [`xx_hash`] - Direct hash of a string
//! - [`xx_hash_bytes`] - Direct hash of a byte slice
//! - [`xx_hash_variant`] - Hash with configurable normalization via [`HashVariant`]
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_hash::{xx_hash, xx_hash_variant, HashVariant};
//!
//! let content = "Hello, World!";
//! let hash = xx_hash(content);
//!
//! // BlockTrimming variant ignores leading/trailing whitespace
//! assert_eq!(
//!     xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
//!     xx_hash("hello")
//! );
//!
//! // BlankLine variant ignores empty lines
//! let with_blanks = "line1\n\nline2";
//! assert_eq!(
//!     xx_hash_variant(with_blanks, vec![HashVariant::BlankLine]),
//!     xx_hash_variant("line1\nline2", vec![HashVariant::BlankLine])
//! );
//! ```

use std::collections::HashMap;
use xxhash_rust::xxh64::xxh64;

/// The **HashVariant** enumeration lets you express characteristics about
/// the content you're hashing which you want to remove from being a factor
/// in the hash which is being created.
///
/// ## Examples
///
/// ```rust
/// use std::collections::HashMap;
/// use biscuit_hash::HashVariant;
///
/// let mut replacements = HashMap::new();
/// replacements.insert("\u{2019}".to_string(), "'".to_string());
///
/// let hash_strategy = HashVariant::ReplacementMap(replacements);
/// ```
#[derive(Clone)]
pub enum HashVariant {
    /// Trims the whitespace at the beginning and end of the
    /// content block being hashed.
    BlockTrimming,
    /// Removes all blank lines in the content block being hashed.
    BlankLine,
    /// Removes the leading whitespace on every line.
    LeadingWhitespace,
    /// Removes the trailing whitespace on every line.
    TrailingWhitespace,
    /// Removes all _extra_ interior whitespace; this means that
    /// whitespace in the interior of a line's content is removed
    /// after the first space.
    InteriorWhitespace,
    /// Allows the caller to specify a dictionary of FROM -> TO content.
    ReplacementMap(HashMap<String, String>),
    /// Drop characters from the document before creating the hash.
    DropChars(Vec<char>),
}

/// Computes XXH64 hash of the input string.
///
/// This is a direct hash of the input bytes with no preprocessing.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::xx_hash;
///
/// let hash = xx_hash("Hello, World!");
/// assert_eq!(hash, xx_hash("Hello, World!")); // Deterministic
/// assert_ne!(hash, xx_hash("Hello, World")); // Different content = different hash
/// ```
#[inline]
pub fn xx_hash(data: &str) -> u64 {
    xxh64(data.as_bytes(), 0)
}

/// Computes XXH64 hash of the input bytes.
///
/// This is a direct hash with no preprocessing.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::xx_hash_bytes;
///
/// let hash = xx_hash_bytes(b"Hello, World!");
/// assert_eq!(hash, xx_hash_bytes(b"Hello, World!"));
/// ```
#[inline]
pub fn xx_hash_bytes(data: &[u8]) -> u64 {
    xxh64(data, 0)
}

/// Produces an xxHash of a _mutated_ version of the content/data passed in.
///
/// The caller provides one or more `HashVariant` enum variants that describe
/// transformations to apply before hashing. This allows producing hashes that
/// ignore certain content variations (whitespace, formatting, etc.).
///
/// ## Variant Application Order
///
/// Variants are applied in a fixed order regardless of input order:
/// 1. `BlockTrimming` - trims whitespace from entire block first
/// 2. `BlankLine` - removes empty lines
/// 3. `LeadingWhitespace` - removes leading whitespace per line
/// 4. `TrailingWhitespace` - removes trailing whitespace per line
/// 5. `InteriorWhitespace` - collapses interior whitespace per line
/// 6. `ReplacementMap` - applies text substitutions
/// 7. `DropChars` - removes specified characters
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::{xx_hash, xx_hash_variant, HashVariant};
///
/// // Empty variants = same as xx_hash
/// assert_eq!(xx_hash_variant("hello", vec![]), xx_hash("hello"));
///
/// // BlockTrimming removes surrounding whitespace
/// assert_eq!(
///     xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
///     xx_hash("hello")
/// );
///
/// // BlankLine removes empty lines
/// assert_eq!(
///     xx_hash_variant("a\n\nb", vec![HashVariant::BlankLine]),
///     xx_hash_variant("a\nb", vec![HashVariant::BlankLine])
/// );
/// ```
pub fn xx_hash_variant(data: &str, variants: Vec<HashVariant>) -> u64 {
    if variants.is_empty() {
        return xx_hash(data);
    }

    // Check which variants are requested
    let has_block_trimming = variants
        .iter()
        .any(|v| matches!(v, HashVariant::BlockTrimming));
    let has_blank_line = variants.iter().any(|v| matches!(v, HashVariant::BlankLine));
    let has_leading_ws = variants
        .iter()
        .any(|v| matches!(v, HashVariant::LeadingWhitespace));
    let has_trailing_ws = variants
        .iter()
        .any(|v| matches!(v, HashVariant::TrailingWhitespace));
    let has_interior_ws = variants
        .iter()
        .any(|v| matches!(v, HashVariant::InteriorWhitespace));

    // Extract ReplacementMap and DropChars (there can be multiple of each)
    let replacements: Vec<_> = variants
        .iter()
        .filter_map(|v| {
            if let HashVariant::ReplacementMap(map) = v {
                Some(map)
            } else {
                None
            }
        })
        .collect();

    let drop_chars: Vec<char> = variants
        .iter()
        .filter_map(|v| {
            if let HashVariant::DropChars(chars) = v {
                Some(chars.as_slice())
            } else {
                None
            }
        })
        .flatten()
        .copied()
        .collect();

    // Start with the input data
    let mut result = data.to_string();

    // 1. BlockTrimming - trim entire block first
    if has_block_trimming {
        result = result.trim().to_string();
    }

    // 2-5. Line-level operations (BlankLine, Leading/Trailing/Interior whitespace)
    if has_blank_line || has_leading_ws || has_trailing_ws || has_interior_ws {
        let lines: Vec<String> = result
            .lines()
            .filter_map(|line| {
                let mut processed = line.to_string();

                // 3. LeadingWhitespace
                if has_leading_ws {
                    processed = processed.trim_start().to_string();
                }

                // 4. TrailingWhitespace
                if has_trailing_ws {
                    processed = processed.trim_end().to_string();
                }

                // 5. InteriorWhitespace - collapse multiple spaces to single space
                if has_interior_ws {
                    processed = collapse_interior_whitespace(&processed);
                }

                // 2. BlankLine - filter out empty lines (after other processing)
                if has_blank_line && processed.trim().is_empty() {
                    None
                } else {
                    Some(processed)
                }
            })
            .collect();

        result = lines.join("\n");
    }

    // 6. ReplacementMap - apply all replacement maps
    for map in &replacements {
        for (from, to) in *map {
            result = result.replace(from, to);
        }
    }

    // 7. DropChars - remove specified characters
    if !drop_chars.is_empty() {
        result = result.chars().filter(|c| !drop_chars.contains(c)).collect();
    }

    xxh64(result.as_bytes(), 0)
}

/// Collapses runs of whitespace characters to single spaces.
fn collapse_interior_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = false;

    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xx_hash_deterministic() {
        let content = "Hello, World!";
        assert_eq!(xx_hash(content), xx_hash(content));
    }

    #[test]
    fn test_xx_hash_different_content() {
        assert_ne!(xx_hash("hello"), xx_hash("world"));
    }

    #[test]
    fn test_xx_hash_empty_string() {
        let hash = xx_hash("");
        assert_eq!(hash, xx_hash("")); // Deterministic for empty
    }

    #[test]
    fn test_xx_hash_bytes() {
        let data = b"Hello, World!";
        assert_eq!(xx_hash_bytes(data), xx_hash_bytes(data));
        assert_eq!(xx_hash_bytes(data), xx_hash("Hello, World!"));
    }

    // ==================== xx_hash_variant Tests ====================

    #[test]
    fn test_xx_hash_variant_empty_variants_equals_xx_hash() {
        let data = "Hello, World!";
        assert_eq!(xx_hash_variant(data, vec![]), xx_hash(data));
    }

    #[test]
    fn test_xx_hash_variant_empty_string() {
        assert_eq!(xx_hash_variant("", vec![]), xx_hash(""));
        assert_eq!(
            xx_hash_variant("", vec![HashVariant::BlockTrimming]),
            xx_hash("")
        );
    }

    // --- BlockTrimming Tests ---

    #[test]
    fn test_xx_hash_variant_block_trimming() {
        assert_eq!(
            xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
            xx_hash("hello")
        );
        assert_eq!(
            xx_hash_variant("\n\nhello\n\n", vec![HashVariant::BlockTrimming]),
            xx_hash("hello")
        );
        assert_eq!(
            xx_hash_variant("\t hello \t", vec![HashVariant::BlockTrimming]),
            xx_hash("hello")
        );
    }

    #[test]
    fn test_xx_hash_variant_block_trimming_preserves_internal() {
        // BlockTrimming should NOT affect internal whitespace
        assert_eq!(
            xx_hash_variant("  hello world  ", vec![HashVariant::BlockTrimming]),
            xx_hash("hello world")
        );
    }

    #[test]
    fn test_xx_hash_variant_block_trimming_matches_trim() {
        // BlockTrimming should produce the same hash as xx_hash on trimmed content
        let test_cases = vec![
            ("  hello  ", "hello"),
            ("\thello\t", "hello"),
            ("\n\nhello\n\n", "hello"),
            ("hello", "hello"),
            ("  multi\nline\n  ", "multi\nline"),
        ];
        for (data, expected_trimmed) in test_cases {
            assert_eq!(
                xx_hash_variant(data, vec![HashVariant::BlockTrimming]),
                xx_hash(expected_trimmed),
                "BlockTrimming should match trimmed content for: {:?}",
                data
            );
        }
    }

    // --- BlankLine Tests ---

    #[test]
    fn test_xx_hash_variant_blank_line() {
        assert_eq!(
            xx_hash_variant("a\n\nb", vec![HashVariant::BlankLine]),
            xx_hash_variant("a\nb", vec![HashVariant::BlankLine])
        );
        assert_eq!(
            xx_hash_variant("a\n\n\n\nb", vec![HashVariant::BlankLine]),
            xx_hash_variant("a\nb", vec![HashVariant::BlankLine])
        );
    }

    #[test]
    fn test_xx_hash_variant_blank_line_only_whitespace() {
        // Lines with only whitespace should be treated as blank
        assert_eq!(
            xx_hash_variant("a\n   \nb", vec![HashVariant::BlankLine]),
            xx_hash_variant("a\nb", vec![HashVariant::BlankLine])
        );
    }

    #[test]
    fn test_xx_hash_variant_blank_line_removes_empty_lines() {
        // BlankLine should produce consistent hashes regardless of blank lines
        let test_cases = vec![
            (
                "flowchart LR\n\n    A --> B\n\n",
                "flowchart LR\n    A --> B",
            ),
            ("a\n\nb\n\nc", "a\nb\nc"),
            ("single line", "single line"),
            ("\n\nleading blanks", "leading blanks"),
            ("trailing blanks\n\n", "trailing blanks"),
        ];
        for (data, expected_normalized) in test_cases {
            assert_eq!(
                xx_hash_variant(data, vec![HashVariant::BlankLine]),
                xx_hash_variant(expected_normalized, vec![HashVariant::BlankLine]),
                "BlankLine should normalize blank lines for: {:?}",
                data
            );
        }
    }

    // --- LeadingWhitespace Tests ---

    #[test]
    fn test_xx_hash_variant_leading_whitespace() {
        assert_eq!(
            xx_hash_variant("  hello\n  world", vec![HashVariant::LeadingWhitespace]),
            xx_hash_variant("hello\nworld", vec![HashVariant::LeadingWhitespace])
        );
    }

    #[test]
    fn test_xx_hash_variant_leading_whitespace_preserves_trailing() {
        // LeadingWhitespace should NOT affect trailing whitespace
        assert_ne!(
            xx_hash_variant("hello  ", vec![HashVariant::LeadingWhitespace]),
            xx_hash_variant("hello", vec![HashVariant::LeadingWhitespace])
        );
    }

    // --- TrailingWhitespace Tests ---

    #[test]
    fn test_xx_hash_variant_trailing_whitespace() {
        assert_eq!(
            xx_hash_variant("hello  \nworld  ", vec![HashVariant::TrailingWhitespace]),
            xx_hash_variant("hello\nworld", vec![HashVariant::TrailingWhitespace])
        );
    }

    #[test]
    fn test_xx_hash_variant_trailing_whitespace_preserves_leading() {
        // TrailingWhitespace should NOT affect leading whitespace
        assert_ne!(
            xx_hash_variant("  hello", vec![HashVariant::TrailingWhitespace]),
            xx_hash_variant("hello", vec![HashVariant::TrailingWhitespace])
        );
    }

    // --- InteriorWhitespace Tests ---

    #[test]
    fn test_xx_hash_variant_interior_whitespace() {
        assert_eq!(
            xx_hash_variant("hello   world", vec![HashVariant::InteriorWhitespace]),
            xx_hash_variant("hello world", vec![HashVariant::InteriorWhitespace])
        );
        assert_eq!(
            xx_hash_variant("a  b  c", vec![HashVariant::InteriorWhitespace]),
            xx_hash_variant("a b c", vec![HashVariant::InteriorWhitespace])
        );
    }

    #[test]
    fn test_xx_hash_variant_interior_whitespace_tabs() {
        assert_eq!(
            xx_hash_variant("hello\t\tworld", vec![HashVariant::InteriorWhitespace]),
            xx_hash_variant("hello world", vec![HashVariant::InteriorWhitespace])
        );
    }

    // --- ReplacementMap Tests ---

    #[test]
    fn test_xx_hash_variant_replacement_map() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "bar".to_string());

        assert_eq!(
            xx_hash_variant("foo baz", vec![HashVariant::ReplacementMap(map)]),
            xx_hash("bar baz")
        );
    }

    #[test]
    fn test_xx_hash_variant_replacement_map_multiple() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("\u{2019}".to_string(), "'".to_string()); // smart single quote -> regular
        map.insert("\u{201C}".to_string(), "\"".to_string()); // left smart double quote
        map.insert("\u{201D}".to_string(), "\"".to_string()); // right smart double quote

        // Input: He said "hello" (with smart quotes U+201C and U+201D)
        let input = "He said \u{201C}hello\u{201D}";
        assert_eq!(
            xx_hash_variant(input, vec![HashVariant::ReplacementMap(map)]),
            xx_hash("He said \"hello\"")
        );
    }

    // --- DropChars Tests ---

    #[test]
    fn test_xx_hash_variant_drop_chars() {
        assert_eq!(
            xx_hash_variant("hello!", vec![HashVariant::DropChars(vec!['!'])]),
            xx_hash("hello")
        );
    }

    #[test]
    fn test_xx_hash_variant_drop_chars_multiple() {
        assert_eq!(
            xx_hash_variant("a-b_c", vec![HashVariant::DropChars(vec!['-', '_'])]),
            xx_hash("abc")
        );
    }

    // --- Combination Tests ---

    #[test]
    fn test_xx_hash_variant_semantic_combination() {
        // Semantic combination: trim each line, filter blank lines
        // This is LeadingWhitespace + TrailingWhitespace + BlankLine
        let test_cases = vec![
            ("  - item1  \n\n  - item2\n\n", "- item1\n- item2"),
            ("  line1\n  line2", "line1\nline2"),
            ("a\n\n\nb", "a\nb"),
            ("  hello  ", "hello"),
        ];
        for (data, expected) in test_cases {
            assert_eq!(
                xx_hash_variant(
                    data,
                    vec![
                        HashVariant::LeadingWhitespace,
                        HashVariant::TrailingWhitespace,
                        HashVariant::BlankLine
                    ]
                ),
                xx_hash(expected),
                "Semantic combination should match expected for: {:?}",
                data
            );
        }
    }

    #[test]
    fn test_xx_hash_variant_order_independence() {
        // Order of variants in input should not matter
        let data = "  hello  \n\n  world  ";

        let v1 = vec![
            HashVariant::BlockTrimming,
            HashVariant::BlankLine,
            HashVariant::LeadingWhitespace,
        ];
        let v2 = vec![
            HashVariant::LeadingWhitespace,
            HashVariant::BlockTrimming,
            HashVariant::BlankLine,
        ];
        let v3 = vec![
            HashVariant::BlankLine,
            HashVariant::LeadingWhitespace,
            HashVariant::BlockTrimming,
        ];

        let h1 = xx_hash_variant(data, v1);
        let h2 = xx_hash_variant(data, v2);
        let h3 = xx_hash_variant(data, v3);

        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn test_xx_hash_variant_all_whitespace_variants() {
        let data = "  hello  \n\n  world  ";

        // Applying all whitespace variants should normalize aggressively
        let hash = xx_hash_variant(
            data,
            vec![
                HashVariant::BlockTrimming,
                HashVariant::BlankLine,
                HashVariant::LeadingWhitespace,
                HashVariant::TrailingWhitespace,
                HashVariant::InteriorWhitespace,
            ],
        );

        // Result should be: "hello\nworld" (trimmed, no blank lines, no leading/trailing per line)
        assert_eq!(hash, xx_hash("hello\nworld"));
    }

    #[test]
    fn test_xx_hash_variant_deterministic() {
        let data = "test content";
        let variants = vec![HashVariant::BlockTrimming, HashVariant::BlankLine];

        // Same input should produce same output
        assert_eq!(
            xx_hash_variant(data, variants.clone()),
            xx_hash_variant(data, variants)
        );
    }
}
