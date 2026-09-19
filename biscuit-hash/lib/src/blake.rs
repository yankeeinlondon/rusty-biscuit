//! BLAKE3 cryptographic hashing utilities.
//!
//! This module provides wrappers around the BLAKE3 algorithm for
//! secure content hashing and integrity verification.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_hash::{blake3_hash, blake3_hash_bytes};
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
/// use biscuit_hash::blake3_hash;
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
/// use biscuit_hash::blake3_hash_bytes;
///
/// let hash = blake3_hash_bytes(b"Hello, World!");
/// assert_eq!(hash.len(), 32);
/// ```
#[inline]
pub fn blake3_hash_bytes(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Computes the BLAKE3 hash of everything `reader` yields, as a hex string.
///
/// Streaming, so a caller checksumming a multi-gigabyte artifact — a CI build
/// archive, a model file — never holds it in memory. Reaches the same digest as
/// [`blake3_hash_bytes`] over the same bytes.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::{blake3_hash, blake3_hash_reader};
///
/// let mut source = "Hello, World!".as_bytes();
/// assert_eq!(blake3_hash_reader(&mut source).unwrap(), blake3_hash("Hello, World!"));
/// ```
///
/// ## Errors
///
/// Propagates the first read error `reader` returns.
pub fn blake3_hash_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    // 64 KiB: large enough that syscall overhead disappears, small enough to
    // stay off the stack-size cliff on every platform's default thread stack.
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(hasher.finalize().to_hex().to_string());
        }
        hasher.update(&buffer[..read]);
    }
}

/// Computes BLAKE3 hash of the input string after trimming whitespace.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::blake3_hash_trimmed;
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
    fn test_blake3_hash_reader_matches_the_in_memory_digest() {
        let mut source = "Hello, World!".as_bytes();
        assert_eq!(
            blake3_hash_reader(&mut source).expect("reading a slice cannot fail"),
            blake3_hash("Hello, World!")
        );
    }

    #[test]
    fn test_blake3_hash_reader_spans_more_than_one_buffer() {
        // Longer than the 64 KiB read buffer, so the digest is only correct if
        // every chunk reached the hasher in order.
        let body = "biscuit".repeat(40_000);
        let mut source = body.as_bytes();
        assert_eq!(
            blake3_hash_reader(&mut source).expect("reading a slice cannot fail"),
            blake3_hash(&body)
        );
    }

    #[test]
    fn test_blake3_hash_reader_of_nothing_is_the_empty_digest() {
        let mut source: &[u8] = b"";
        assert_eq!(
            blake3_hash_reader(&mut source).expect("reading a slice cannot fail"),
            blake3_hash("")
        );
    }

    #[test]
    fn test_blake3_hash_reader_propagates_a_read_failure() {
        struct Broken;
        impl std::io::Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("device on fire"))
            }
        }
        let error = blake3_hash_reader(&mut Broken).expect_err("a failing reader must not hash");
        assert!(error.to_string().contains("device on fire"));
    }

    #[test]
    fn test_blake3_known_value() {
        // Known test vector: blake3("") should produce a specific hash
        let empty_hash = blake3_hash("");
        // BLAKE3 empty string hash is well-defined
        assert!(empty_hash.starts_with("af1349"));
    }
}
