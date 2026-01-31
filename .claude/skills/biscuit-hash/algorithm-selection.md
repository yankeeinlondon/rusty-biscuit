# Algorithm Selection Guide

## Decision Matrix

| Need | Algorithm | Why |
|------|-----------|-----|
| Cache keys | xxHash | Fastest, collision-resistant enough |
| Deduplication | xxHash | Speed matters, not cryptographic |
| Change detection | xxHash | Quick comparison of content |
| Hash maps/tables | xxHash | Designed for this use case |
| File integrity | BLAKE3 | Cryptographic, tamper-evident |
| Content signatures | BLAKE3 | Secure against manipulation |
| Checksums for download | BLAKE3 | Industry-standard replacement for SHA |
| User passwords | Argon2id | OWASP recommended, memory-hard |
| API keys/tokens | Argon2id | High-value secrets need slow hashing |
| Key derivation | Argon2id | Secure key stretching |

## Performance Characteristics

### xxHash (XXH64)

- **Speed**: ~30 GB/s on modern CPUs
- **Output**: 64-bit (8 bytes)
- **Collision resistance**: Excellent for practical use
- **Cryptographic**: NO - do not use for security

```rust
// Good uses
let cache_key = xx_hash(&request_body);
let content_id = xx_hash(&file_content);
let change_detected = old_hash != xx_hash(&new_content);
```

### BLAKE3

- **Speed**: ~6 GB/s on modern CPUs (still very fast for crypto)
- **Output**: 256-bit (32 bytes, 64 hex chars)
- **Security**: Cryptographically secure
- **Parallelizable**: Scales with CPU cores

```rust
// Good uses
let file_integrity = blake3_hash(&file_content);
let content_signature = blake3_hash(&document);
```

### Argon2id

- **Speed**: Intentionally slow (~200ms with defaults)
- **Output**: PHC format string with embedded parameters
- **Memory**: 19 MiB default (tunable)
- **Security**: PHC winner, hybrid Argon2i+Argon2d

```rust
// Good uses
let stored_hash = hash_password(&user_password)?;
let valid = verify_password(&input, &stored_hash)?;
```

## Common Mistakes

### Using xxHash for Security

```rust
// WRONG - xxHash is not cryptographic
let api_key_hash = xx_hash(&api_key);

// CORRECT - use Argon2id for secrets
let api_key_hash = hash_password(&api_key)?;
```

### Using BLAKE3 for Passwords

```rust
// WRONG - too fast, vulnerable to brute force
let password_hash = blake3_hash(&password);

// CORRECT - Argon2id is memory-hard and slow
let password_hash = hash_password(&password)?;
```

### Using Argon2id for Content

```rust
// WRONG - way too slow
let file_hash = hash_password(&file_content)?;

// CORRECT - use xxHash or BLAKE3
let file_hash = xx_hash(&file_content);
let secure_hash = blake3_hash(&file_content);
```

## Feature Flag Combinations

```toml
# Content caching only
biscuit-hash = "0.1"  # default xx_hash

# Content + integrity verification
biscuit-hash = { version = "0.1", features = ["blake3"] }

# User authentication
biscuit-hash = { version = "0.1", features = ["argon2id"] }

# All use cases
biscuit-hash = { version = "0.1", features = ["blake3", "argon2id"] }
```

## OWASP Argon2id Parameters

The default parameters follow OWASP guidelines:

| Parameter | Default | OWASP Minimum |
|-----------|---------|---------------|
| Memory | 19 MiB | 19 MiB |
| Iterations | 2 | 2 |
| Parallelism | 1 | 1 |

For higher security (slower):

```rust
// Higher security web application
let hash = hash_password_with_params(
    password,
    65536,  // 64 MiB
    3,      // 3 iterations
    4,      // 4 parallel lanes
)?;
```

## Migration from Other Algorithms

### From SHA-256 to BLAKE3

BLAKE3 is a drop-in replacement with better performance:

```rust
// Before (using sha2 crate)
// let hash = sha256::digest(&content);

// After (using biscuit-hash)
let hash = blake3_hash(&content);
```

### From bcrypt to Argon2id

```rust
// Argon2id produces PHC format, similar to bcrypt's modular crypt format
// Both can coexist during migration - verify() will detect the format

// New passwords use Argon2id
let hash = hash_password(&new_password)?;

// Old bcrypt hashes still work with the bcrypt crate for verification
// Rehash to Argon2id on successful login
```

## Recommendations Summary

1. **Default to xxHash** for non-security hashing
2. **Use BLAKE3** when you need cryptographic guarantees
3. **Always use Argon2id** for passwords and secrets
4. **Enable only needed features** to minimize binary size
5. **Use HashVariant** for semantic/whitespace-insensitive hashing
