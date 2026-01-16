//! Hashing utilities for integrity verification and password security.
//!
//! This module provides cryptographic hashing (BLAKE3) and password hashing (Argon2id).
//! For fast non-cryptographic hashing (xxHash), use the `biscuit_hash` crate.
//!
//! ## When to Use Which
//!
//! - **biscuit_hash::xx_hash**: Fast, non-cryptographic. Use for:
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
//! use shared::hashing::{blake3_hash};
//! use shared::hashing::argon2id::{hash_password, verify_password};
//! use biscuit_hash::{xx_hash, xx_hash_variant, HashVariant};
//!
//! // Fast hash for change detection (from biscuit_hash)
//! let hash = xx_hash("content");
//!
//! // Hash with normalization using HashVariant
//! // BlockTrimming removes leading/trailing whitespace
//! assert_eq!(
//!     xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
//!     xx_hash("hello")
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

// Re-export commonly used functions at module level
pub use blake3::{blake3_hash, blake3_hash_bytes, blake3_hash_trimmed};

// Re-export argon2id types and functions
pub use argon2id::{
    Argon2idError, DEFAULT_MEMORY_COST_KIB, DEFAULT_OUTPUT_LEN, DEFAULT_PARALLELISM,
    DEFAULT_TIME_COST, hash_password, hash_password_with_params, hash_password_with_salt,
    verify_password,
};
