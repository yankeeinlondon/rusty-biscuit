//! BLAKE3 cryptographic hashing utilities.
//!
//! This module provides wrappers around the BLAKE3 algorithm for
//! secure content hashing and integrity verification.
//!
//! ## Examples
//!
//! ```rust
//! use shared::hashing::{blake3_hash, blake3_hash_bytes};
//!
//! let content = "Hello, World!";
//! let hash = blake3_hash(content);
//! assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
//!
//! let bytes = blake3_hash_bytes(content.as_bytes());
//! assert_eq!(bytes.len(), 32);
//! ```

/// Computes BLAKE3 hash of the input string and returns it as a hex string.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::blake3_hash;
///
/// let hash = blake3_hash("Hello, World!");
/// assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
/// assert_eq!(hash, blake3_hash("Hello, World!")); // Deterministic
/// ```
#[inline]
pub fn blake3_hash(data: &str) -> String {
    blake3::hash(data.as_bytes()).to_hex().to_string()
}

/// Computes BLAKE3 hash of the input bytes and returns the raw 32-byte hash.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::blake3_hash_bytes;
///
/// let hash = blake3_hash_bytes(b"Hello, World!");
/// assert_eq!(hash.len(), 32);
/// ```
#[inline]
pub fn blake3_hash_bytes(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Computes BLAKE3 hash of the input string after trimming whitespace.
///
/// ## Examples
///
/// ```rust
/// use shared::hashing::blake3_hash_trimmed;
///
/// assert_eq!(blake3_hash_trimmed("  hello  "), blake3_hash_trimmed("hello"));
/// ```
#[inline]
pub fn blake3_hash_trimmed(data: &str) -> String {
    blake3::hash(data.trim().as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash_deterministic() {
        let content = "Hello, World!";
        assert_eq!(blake3_hash(content), blake3_hash(content));
    }

    #[test]
    fn test_blake3_hash_length() {
        let hash = blake3_hash("test");
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_blake3_hash_different_content() {
        assert_ne!(blake3_hash("hello"), blake3_hash("world"));
    }

    #[test]
    fn test_blake3_hash_empty_string() {
        let hash = blake3_hash("");
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, blake3_hash("")); // Deterministic for empty
    }

    #[test]
    fn test_blake3_hash_bytes() {
        let data = b"Hello, World!";
        let hash = blake3_hash_bytes(data);
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, blake3_hash_bytes(data));
    }

    #[test]
    fn test_blake3_hash_bytes_matches_string() {
        let content = "Hello, World!";
        let hex_hash = blake3_hash(content);
        let byte_hash = blake3_hash_bytes(content.as_bytes());

        // Convert byte hash to hex and compare
        let byte_hex: String = byte_hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex_hash, byte_hex);
    }

    #[test]
    fn test_blake3_hash_trimmed() {
        assert_eq!(
            blake3_hash_trimmed("  hello  "),
            blake3_hash_trimmed("hello")
        );
        assert_eq!(
            blake3_hash_trimmed("\thello\t"),
            blake3_hash_trimmed("hello")
        );
        assert_eq!(
            blake3_hash_trimmed("\n\nhello\n\n"),
            blake3_hash_trimmed("hello")
        );
    }

    #[test]
    fn test_blake3_known_value() {
        // Known test vector: blake3("") should produce a specific hash
        let empty_hash = blake3_hash("");
        // BLAKE3 empty string hash is well-defined
        assert!(empty_hash.starts_with("af1349"));
    }
}
