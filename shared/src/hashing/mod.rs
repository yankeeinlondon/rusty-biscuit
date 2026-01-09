//! Hashing utilities for content fingerprinting, integrity verification, and password security.
//!
//! This module provides fast non-cryptographic hashing (xxHash), secure cryptographic
//! hashing (BLAKE3), and password hashing (Argon2id) for various use cases.
//!
//! ## When to Use Which
//!
//! - **xxHash**: Fast, non-cryptographic. Use for:
//!   - Content change detection
//!   - Cache keys
//!   - Hash maps / deduplication
//!   - Mermaid diagram caching
//!
//! - **BLAKE3**: Cryptographically secure. Use for:
//!   - Content integrity verification
//!   - Secure fingerprinting
//!   - When collision resistance matters
//!
//! - **Argon2id**: Password hashing (slow by design). Use for:
//!   - Password storage and verification
//!   - Key derivation from passwords
//!   - Any security-critical credential hashing
//!
//! ## Examples
//!
//! ```rust
//! use shared::hashing::{xx_hash, xx_hash_variant, blake3_hash, HashVariant};
//! use shared::hashing::argon2id::{hash_password, verify_password};
//!
//! // Fast hash for change detection
//! let hash = xx_hash("content");
//!
//! // Hash with normalization using HashVariant
//! // BlockTrimming removes leading/trailing whitespace
//! assert_eq!(
//!     xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
//!     xx_hash("hello")
//! );
//!
//! // Combine multiple variants for semantic comparison
//! // (ignores whitespace differences)
//! let semantic_hash = xx_hash_variant(
//!     "  content  \n\n  more  ",
//!     vec![
//!         HashVariant::LeadingWhitespace,
//!         HashVariant::TrailingWhitespace,
//!         HashVariant::BlankLine,
//!     ],
//! );
//!
//! // Secure hash for integrity
//! let secure_hash = blake3_hash("content");
//!
//! // Password hashing (argon2id functions accessed via submodule)
//! let pwd_hash = hash_password("my-secret").unwrap();
//! assert!(verify_password("my-secret", &pwd_hash).unwrap());
//! ```

pub mod argon2id;
pub mod blake3;
pub mod xx_hash;

use std::collections::HashMap;

// Re-export commonly used functions at module level
pub use blake3::{blake3_hash, blake3_hash_bytes, blake3_hash_trimmed};
pub use xx_hash::{xx_hash, xx_hash_bytes, xx_hash_variant};

// Re-export argon2id types and functions
pub use argon2id::{
    Argon2idError, DEFAULT_MEMORY_COST_KIB, DEFAULT_OUTPUT_LEN, DEFAULT_PARALLELISM,
    DEFAULT_TIME_COST, hash_password, hash_password_with_params, hash_password_with_salt,
    verify_password,
};


/// The **HashVariant** enumeration let's you express
/// characteristics about the content you're hashing
/// which you want to remove from being a factor in the
/// hash which is being created.
///
/// The **HashVariant** is currently used in the **xx_hash_variant**
/// function and may be added to the cryptographic `blake3` implementation
/// at some future point.
#[derive(Clone)]
pub enum HashVariant {
    /// Trims the whitespace at the beginning and end of the
    /// content block being hashed.
    BlockTrimming,
    /// Removes all blank lines in the content block being hashed.
    BlankLine,
    /// Removes the leading whitespace on every line
    LeadingWhitespace,
    /// Removes the trailing whitespace on every line
    TrailingWhitespace,
    /// Removes all _extra_ interior whitespace; this means that
    /// whitespace in the interior of a line's content is removed
    /// after the first space.
    InteriorWhitespace,
    /// Allows the caller to specify a dictionary of FROM -> TO content.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use shared::hashing::HashVariant;
    ///
    /// let mut replacements = HashMap::new();
    /// replacements.insert("\u{2019}".to_string(), "'".to_string());
    ///
    /// let hash_strategy = HashVariant::ReplacementMap(replacements);
    /// ```
    ///
    /// In this example we have created a HashVariant which, when used
    /// with `xx_hash_variant`, will convert the smart quote `'` (U+2019)
    /// to a normal single quote `'` character.
    ReplacementMap(HashMap<String, String>),
    /// Drop characters from the document before creating the
    /// hash.
    DropChars(Vec<char>)
}
