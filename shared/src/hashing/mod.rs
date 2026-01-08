//! Hashing utilities for content fingerprinting and change detection.
//!
//! This module provides both fast non-cryptographic hashing (xxHash) and
//! secure cryptographic hashing (BLAKE3) for various use cases.
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
//! ## Examples
//!
//! ```rust
//! use shared::hashing::{xx_hash, xx_hash_trimmed, blake3_hash};
//!
//! // Fast hash for change detection
//! let hash = xx_hash("content");
//!
//! // Trimmed hash ignores surrounding whitespace
//! assert_eq!(xx_hash_trimmed("  hello  "), xx_hash_trimmed("hello"));
//!
//! // Secure hash for integrity
//! let secure_hash = blake3_hash("content");
//! ```

pub mod blake3;
pub mod xx_hash;

// Re-export commonly used functions at module level
pub use blake3::{blake3_hash, blake3_hash_bytes, blake3_hash_trimmed};
pub use xx_hash::{xx_hash, xx_hash_bytes, xx_hash_normalized, xx_hash_trimmed};
