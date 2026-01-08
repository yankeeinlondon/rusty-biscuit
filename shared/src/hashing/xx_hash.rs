//! XXH64 hashing utilities for fast, non-cryptographic hashing.
//!
//! This module provides wrappers around the xxHash algorithm for common
//! use cases like content hashing, change detection, and caching.
//!
//! ## Examples
//!
//! ```rust
//! use shared::hashing::{xx_hash, xx_hash_trimmed, xx_hash_normalized};
//!
//! let content = "Hello, World!";
//! let hash = xx_hash(content);
//!
//! // Trimmed hash ignores leading/trailing whitespace
//! assert_eq!(xx_hash_trimmed("  hello  "), xx_hash_trimmed("hello"));
//!
//! // Normalized hash ignores blank lines
//! let with_blanks = "line1\n\nline2";
//! let without_blanks = "line1\nline2";
//! assert_eq!(xx_hash_normalized(with_blanks), xx_hash_normalized(without_blanks));
//! ```

use xxhash_rust::xxh64::xxh64;

/// Computes XXH64 hash of the input string.
///
/// This is a direct hash of the input bytes with no preprocessing.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::xx_hash;
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
/// use shared::hashing::xx_hash_bytes;
///
/// let hash = xx_hash_bytes(b"Hello, World!");
/// assert_eq!(hash, xx_hash_bytes(b"Hello, World!"));
/// ```
#[inline]
pub fn xx_hash_bytes(data: &[u8]) -> u64 {
    xxh64(data, 0)
}

/// Computes XXH64 hash of the input string after trimming whitespace.
///
/// Leading and trailing whitespace is removed before hashing, which is
/// useful for comparing content where surrounding whitespace is insignificant.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::xx_hash_trimmed;
///
/// assert_eq!(xx_hash_trimmed("  hello  "), xx_hash_trimmed("hello"));
/// assert_eq!(xx_hash_trimmed("\n\nhello\n\n"), xx_hash_trimmed("hello"));
/// ```
#[inline]
pub fn xx_hash_trimmed(data: &str) -> u64 {
    xxh64(data.trim().as_bytes(), 0)
}

/// Computes XXH64 hash of the input string with blank lines removed.
///
/// This function normalizes the input by removing all blank lines before
/// hashing, ensuring that content with different amounts of vertical
/// whitespace produces the same hash.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::xx_hash_normalized;
///
/// let with_blanks = "flowchart LR\n\n    A --> B\n\n";
/// let without_blanks = "flowchart LR\n    A --> B";
/// assert_eq!(xx_hash_normalized(with_blanks), xx_hash_normalized(without_blanks));
/// ```
pub fn xx_hash_normalized(data: &str) -> u64 {
    let normalized: String = data
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    xxh64(normalized.as_bytes(), 0)
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

    #[test]
    fn test_xx_hash_trimmed_removes_whitespace() {
        assert_eq!(xx_hash_trimmed("  hello  "), xx_hash_trimmed("hello"));
        assert_eq!(xx_hash_trimmed("\thello\t"), xx_hash_trimmed("hello"));
        assert_eq!(xx_hash_trimmed("\n\nhello\n\n"), xx_hash_trimmed("hello"));
    }

    #[test]
    fn test_xx_hash_trimmed_preserves_internal_whitespace() {
        assert_ne!(xx_hash_trimmed("hello world"), xx_hash_trimmed("helloworld"));
    }

    #[test]
    fn test_xx_hash_normalized_removes_blank_lines() {
        let with_blanks = "flowchart LR\n\n    A --> B\n\n";
        let without_blanks = "flowchart LR\n    A --> B";
        assert_eq!(xx_hash_normalized(with_blanks), xx_hash_normalized(without_blanks));
    }

    #[test]
    fn test_xx_hash_normalized_different_content() {
        let a = "flowchart LR\n    A --> B";
        let b = "flowchart LR\n    A --> C";
        assert_ne!(xx_hash_normalized(a), xx_hash_normalized(b));
    }

    #[test]
    fn test_xx_hash_normalized_empty_string() {
        let hash = xx_hash_normalized("");
        assert_eq!(hash, xx_hash_normalized("   \n\n  ")); // All blank = empty
    }
}
