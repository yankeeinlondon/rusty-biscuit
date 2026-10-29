//! Hashing utilities for the dockhand ecosystem.
//!
//! This crate provides various hashing algorithms with feature flags to control
//! which implementations are included:
//!
//! - **`xx_hash`** (default): Fast non-cryptographic hashing using XXH64
//! - **`blake3`**: Fast cryptographic hashing using BLAKE3
//! - **`argon2id`**: Secure password hashing using Argon2id
//! - **`json`**: Canonical (key-sorted) JSON rendering and hashing
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `xx_hash` | Yes | XXH64 for content hashing, change detection |
//! | `blake3` | No | BLAKE3 for cryptographic integrity |
//! | `argon2id` | No | Argon2id for password storage |
//! | `json` | No | [`canonical_json`]; with `xx_hash`/`blake3` also `xx_hash_json`/`blake3_hash_json` |
//!
//! ## Examples
//!
//! ```rust
//! // With default features (xx_hash)
//! use biscuit_hash::xx_hash;
//!
//! let hash = xx_hash("Hello, World!");
//! ```

// Conditional module compilation based on features

#[cfg(feature = "xx_hash")]
pub mod xx;

#[cfg(feature = "blake3")]
pub mod blake;

#[cfg(feature = "argon2id")]
pub mod argon;

#[cfg(feature = "json")]
pub mod json;

// Re-exports for convenience

#[cfg(feature = "xx_hash")]
pub use xx::{HashVariant, XxHasher, xx_hash, xx_hash_bytes, xx_hash_variant};

#[cfg(feature = "blake3")]
pub use blake::{blake3_hash, blake3_hash_bytes, blake3_hash_reader, blake3_hash_trimmed};

#[cfg(feature = "argon2id")]
pub use argon::{
    Argon2idError, DEFAULT_MEMORY_COST_KIB, DEFAULT_OUTPUT_LEN, DEFAULT_PARALLELISM,
    DEFAULT_TIME_COST, hash_password, hash_password_with_params, hash_password_with_salt,
    verify_password,
};

#[cfg(feature = "json")]
pub use json::canonical_json;

#[cfg(all(feature = "json", feature = "xx_hash"))]
pub use json::xx_hash_json;

#[cfg(all(feature = "json", feature = "blake3"))]
pub use json::blake3_hash_json;
