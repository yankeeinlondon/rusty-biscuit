//! Argon2id password hashing utilities.
//!
//! This module provides secure password hashing using the Argon2id algorithm,
//! the winner of the Password Hashing Competition and recommended by OWASP.
//!
//! Argon2id is a hybrid that combines:
//! - Argon2i (resistant to side-channel attacks)
//! - Argon2d (resistant to GPU cracking)
//!
//! ## When to Use
//!
//! Use Argon2id for:
//! - Password storage and verification
//! - Key derivation from passwords
//! - Any security-critical credential hashing
//!
//! Do NOT use for:
//! - Content hashing (use xxHash or BLAKE3 instead)
//! - Hash maps or deduplication (too slow by design)
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_hash::{hash_password, verify_password};
//!
//! // Hash a password (generates random salt internally)
//! let hash = hash_password("my-secret-password").unwrap();
//!
//! // Verify password against hash
//! assert!(verify_password("my-secret-password", &hash).unwrap());
//! assert!(!verify_password("wrong-password", &hash).unwrap());
//! ```

use argon2::{
    Argon2, Params,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::rngs::OsRng;
use thiserror::Error;

/// Errors that can occur during password hashing operations.
#[derive(Debug, Error)]
pub enum Argon2idError {
    /// Failed to hash the password.
    #[error("failed to hash password: {0}")]
    HashError(String),

    /// Failed to parse the password hash.
    #[error("invalid password hash format: {0}")]
    InvalidHash(String),

    /// Failed to verify the password.
    #[error("password verification failed: {0}")]
    VerifyError(String),

    /// Invalid parameters provided.
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}

/// Default memory cost in KiB (19 MiB - OWASP recommended minimum).
pub const DEFAULT_MEMORY_COST_KIB: u32 = 19456;

/// Default time cost (number of iterations).
pub const DEFAULT_TIME_COST: u32 = 2;

/// Default parallelism (number of threads).
pub const DEFAULT_PARALLELISM: u32 = 1;

/// Default output length in bytes.
pub const DEFAULT_OUTPUT_LEN: usize = 32;

/// Hashes a password using Argon2id with default parameters.
///
/// Uses OWASP-recommended parameters:
/// - Memory: 19 MiB
/// - Iterations: 2
/// - Parallelism: 1
///
/// The returned string is in PHC format, which includes the algorithm,
/// version, parameters, salt, and hash - everything needed for verification.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::hash_password;
///
/// let hash = hash_password("my-password").unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
///
/// ## Errors
///
/// Returns `Argon2idError::HashError` if hashing fails.
pub fn hash_password(password: &str) -> Result<String, Argon2idError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| Argon2idError::HashError(e.to_string()))
}

/// Hashes a password using Argon2id with custom parameters.
///
/// ## Arguments
///
/// * `password` - The password to hash
/// * `memory_cost_kib` - Memory cost in KiB (e.g., 19456 for 19 MiB)
/// * `time_cost` - Number of iterations
/// * `parallelism` - Degree of parallelism (threads)
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::hash_password_with_params;
///
/// // Higher security settings (slower)
/// let hash = hash_password_with_params("password", 65536, 3, 4).unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
///
/// ## Errors
///
/// Returns `Argon2idError::InvalidParams` if parameters are invalid.
/// Returns `Argon2idError::HashError` if hashing fails.
pub fn hash_password_with_params(
    password: &str,
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<String, Argon2idError> {
    let params = Params::new(
        memory_cost_kib,
        time_cost,
        parallelism,
        Some(DEFAULT_OUTPUT_LEN),
    )
    .map_err(|e| Argon2idError::InvalidParams(e.to_string()))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| Argon2idError::HashError(e.to_string()))
}

/// Verifies a password against an Argon2id hash.
///
/// The hash must be in PHC format (as produced by `hash_password`).
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::{hash_password, verify_password};
///
/// let hash = hash_password("correct-password").unwrap();
///
/// assert!(verify_password("correct-password", &hash).unwrap());
/// assert!(!verify_password("wrong-password", &hash).unwrap());
/// ```
///
/// ## Errors
///
/// Returns `Argon2idError::InvalidHash` if the hash format is invalid.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, Argon2idError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| Argon2idError::InvalidHash(e.to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Hashes a password with a provided salt (for testing or deterministic needs).
///
/// ## Warning
///
/// In production, prefer `hash_password` which generates a cryptographically
/// secure random salt. Only use this function when you specifically need
/// deterministic hashing (e.g., for tests or migration scenarios).
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::hash_password_with_salt;
///
/// // Salt must be valid base64 and at least 8 bytes
/// let hash = hash_password_with_salt("password", "somesaltvalue123").unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
///
/// ## Errors
///
/// Returns `Argon2idError::InvalidParams` if the salt is invalid.
/// Returns `Argon2idError::HashError` if hashing fails.
pub fn hash_password_with_salt(password: &str, salt: &str) -> Result<String, Argon2idError> {
    let salt = SaltString::encode_b64(salt.as_bytes())
        .map_err(|e| Argon2idError::InvalidParams(e.to_string()))?;
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| Argon2idError::HashError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_produces_valid_format() {
        let hash = hash_password("test-password").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.contains("$v=19$")); // Version 0x13 = 19
    }

    #[test]
    fn test_hash_password_unique_salts() {
        let hash1 = hash_password("same-password").unwrap();
        let hash2 = hash_password("same-password").unwrap();
        // Same password should produce different hashes due to random salt
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let hash = hash_password("correct-password").unwrap();
        assert!(verify_password("correct-password", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let hash = hash_password("correct-password").unwrap();
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Argon2idError::InvalidHash(_)));
    }

    #[test]
    fn test_hash_password_with_params() {
        // Use smaller params for faster tests
        let hash = hash_password_with_params("password", 4096, 1, 1).unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_with_params_invalid() {
        // Parallelism of 0 is invalid
        let result = hash_password_with_params("password", 4096, 1, 0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Argon2idError::InvalidParams(_)
        ));
    }

    #[test]
    fn test_hash_password_with_salt_deterministic() {
        let salt = "testsaltvalue123"; // Must be >= 8 bytes
        let hash1 = hash_password_with_salt("password", salt).unwrap();
        let hash2 = hash_password_with_salt("password", salt).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_password_with_salt_verifiable() {
        let salt = "anothersaltval16";
        let hash = hash_password_with_salt("my-password", salt).unwrap();
        assert!(verify_password("my-password", &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_empty_string() {
        // Empty password should still hash
        let hash = hash_password("").unwrap();
        assert!(verify_password("", &hash).unwrap());
        assert!(!verify_password("not-empty", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "パスワード🔐";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_long_password() {
        let password = "a".repeat(1000);
        let hash = hash_password(&password).unwrap();
        assert!(verify_password(&password, &hash).unwrap());
    }
}
