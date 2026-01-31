# biscuit-hash Library API Reference

## Feature Flags

| Feature | Default | Dependencies | Description |
|---------|---------|--------------|-------------|
| `xx_hash` | Yes | `xxhash-rust` | XXH64 for content hashing |
| `blake3` | No | `blake3` | Cryptographic hashing |
| `argon2id` | No | `argon2`, `rand`, `thiserror` | Password hashing |

## xxHash API (default feature)

### Core Functions

```rust
use biscuit_hash::{xx_hash, xx_hash_bytes, xx_hash_variant, HashVariant};

/// Direct hash of a string (XXH64)
pub fn xx_hash(data: &str) -> u64;

/// Direct hash of bytes (XXH64)
pub fn xx_hash_bytes(data: &[u8]) -> u64;

/// Hash with content normalization
pub fn xx_hash_variant(data: &str, variants: Vec<HashVariant>) -> u64;
```

### HashVariant Enum

The `HashVariant` enum enables semantic hashing by normalizing content before hashing. This is useful for change detection in whitespace-insensitive contexts (Markdown, HTML, etc.).

```rust
pub enum HashVariant {
    /// Trims leading/trailing whitespace from entire content block
    BlockTrimming,

    /// Removes all blank lines (including whitespace-only lines)
    BlankLine,

    /// Removes leading whitespace on every line
    LeadingWhitespace,

    /// Removes trailing whitespace on every line
    TrailingWhitespace,

    /// Collapses multiple whitespace to single space within lines
    InteriorWhitespace,

    /// Apply text replacements before hashing
    ReplacementMap(HashMap<String, String>),

    /// Remove specific characters before hashing
    DropChars(Vec<char>),
}
```

### Variant Application Order

Variants are applied in a **fixed order** regardless of input order:

1. `BlockTrimming` - trims entire block first
2. `BlankLine` - removes empty lines
3. `LeadingWhitespace` - per-line leading whitespace
4. `TrailingWhitespace` - per-line trailing whitespace
5. `InteriorWhitespace` - collapses interior whitespace
6. `ReplacementMap` - text substitutions
7. `DropChars` - character removal

### HashVariant Examples

```rust
use biscuit_hash::{xx_hash, xx_hash_variant, HashVariant};
use std::collections::HashMap;

// BlockTrimming - equivalent to .trim()
assert_eq!(
    xx_hash_variant("  hello  ", vec![HashVariant::BlockTrimming]),
    xx_hash("hello")
);

// BlankLine - ignore empty lines
assert_eq!(
    xx_hash_variant("a\n\nb", vec![HashVariant::BlankLine]),
    xx_hash_variant("a\nb", vec![HashVariant::BlankLine])
);

// Combine variants for Markdown-style normalization
let hash = xx_hash_variant(markdown_content, vec![
    HashVariant::LeadingWhitespace,
    HashVariant::TrailingWhitespace,
    HashVariant::BlankLine,
]);

// ReplacementMap - normalize smart quotes
let mut map = HashMap::new();
map.insert("\u{2019}".to_string(), "'".to_string());  // Right single quote
map.insert("\u{201C}".to_string(), "\"".to_string()); // Left double quote
map.insert("\u{201D}".to_string(), "\"".to_string()); // Right double quote

let hash = xx_hash_variant(content, vec![HashVariant::ReplacementMap(map)]);

// DropChars - ignore punctuation
let hash = xx_hash_variant("hello!", vec![HashVariant::DropChars(vec!['!', '?', '.'])]);
```

## BLAKE3 API (requires `blake3` feature)

```rust
use biscuit_hash::{blake3_hash, blake3_hash_bytes, blake3_hash_trimmed};

/// Returns 64-character hex string (32 bytes)
pub fn blake3_hash(data: &str) -> String;

/// Returns raw 32-byte hash
pub fn blake3_hash_bytes(data: &[u8]) -> [u8; 32];

/// Hash after trimming whitespace
pub fn blake3_hash_trimmed(data: &str) -> String;
```

### BLAKE3 Examples

```rust
use biscuit_hash::{blake3_hash, blake3_hash_bytes};

let hex = blake3_hash("hello");
assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars

let bytes = blake3_hash_bytes(b"hello");
assert_eq!(bytes.len(), 32);

// Trimmed variant for whitespace-insensitive hashing
use biscuit_hash::blake3_hash_trimmed;
assert_eq!(blake3_hash_trimmed("  hello  "), blake3_hash_trimmed("hello"));
```

## Argon2id API (requires `argon2id` feature)

### Constants

```rust
/// Default memory cost: 19 MiB (OWASP recommended minimum)
pub const DEFAULT_MEMORY_COST_KIB: u32 = 19456;

/// Default time cost: 2 iterations
pub const DEFAULT_TIME_COST: u32 = 2;

/// Default parallelism: 1 thread
pub const DEFAULT_PARALLELISM: u32 = 1;

/// Default output length: 32 bytes
pub const DEFAULT_OUTPUT_LEN: usize = 32;
```

### Functions

```rust
use biscuit_hash::{hash_password, verify_password, hash_password_with_params,
                   hash_password_with_salt, Argon2idError};

/// Hash password with OWASP-recommended defaults
/// Returns PHC format string: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
pub fn hash_password(password: &str) -> Result<String, Argon2idError>;

/// Verify password against PHC-format hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, Argon2idError>;

/// Hash with custom parameters
pub fn hash_password_with_params(
    password: &str,
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<String, Argon2idError>;

/// Hash with explicit salt (testing/migration only!)
pub fn hash_password_with_salt(
    password: &str,
    salt: &str
) -> Result<String, Argon2idError>;
```

### Error Type

```rust
#[derive(Debug, Error)]
pub enum Argon2idError {
    #[error("failed to hash password: {0}")]
    HashError(String),

    #[error("invalid password hash format: {0}")]
    InvalidHash(String),

    #[error("password verification failed: {0}")]
    VerifyError(String),

    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}
```

### Argon2id Examples

```rust
use biscuit_hash::{hash_password, verify_password, hash_password_with_params};

// Basic usage with OWASP defaults
let hash = hash_password("my-secret").unwrap();
assert!(hash.starts_with("$argon2id$"));

// Verification
assert!(verify_password("my-secret", &hash).unwrap());
assert!(!verify_password("wrong", &hash).unwrap());

// Custom parameters for higher security (slower)
let hash = hash_password_with_params(
    "password",
    65536,  // 64 MiB memory
    3,      // 3 iterations
    4,      // 4 threads
).unwrap();

// Each hash uses random salt - same password = different hash
let hash1 = hash_password("same").unwrap();
let hash2 = hash_password("same").unwrap();
assert_ne!(hash1, hash2); // Different due to random salt
```

## Testing

```bash
# Test all features
cargo test -p biscuit-hash --all-features

# Test specific feature
cargo test -p biscuit-hash --features blake3

# Test CLI
cargo test -p biscuit-hash-cli
```
