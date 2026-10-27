---
name: biscuit-hash
description: Hashing library with xxHash (fast non-crypto), BLAKE3 (crypto), Argon2id (passwords), and key-order-insensitive canonical JSON hashing. Use when implementing hashing, content fingerprinting, hashing structured JSON/YAML data, password storage, or adding the biscuit-hash dependency.
---

## Purpose

Expert knowledge for the `biscuit-hash` Rust crate and `bh` CLI tool providing:

- **xxHash (XXH64)** - Blazingly fast non-cryptographic hashing for content fingerprinting
- **BLAKE3** - Fast cryptographic hashing for integrity verification
- **Argon2id** - OWASP-recommended password hashing
- **Canonical JSON** (`json` feature) - key-sorted rendering of a `serde_json::Value` and its xxHash/BLAKE3 hashes

## When to Use Each Algorithm

| Algorithm | Use For | NOT For |
|-----------|---------|---------|
| xxHash | Cache keys, dedup, change detection, hash maps | Security, passwords |
| BLAKE3 | File integrity, signatures, content verification | Passwords |
| Argon2id | Password storage, key derivation | Content hashing (too slow) |

## Quick Reference

### Library (Rust)

```toml
# Cargo.toml - only enable features you need
biscuit-hash = { version = "0.1", features = ["blake3", "argon2id"] }
```

```rust
use biscuit_hash::{xx_hash, blake3_hash, hash_password, verify_password};

// Fast content hash
let hash = xx_hash("content");

// Cryptographic hash (64-char hex)
let crypto = blake3_hash("content");

// Password hashing (PHC format with random salt)
let pwd_hash = hash_password("secret").unwrap();
assert!(verify_password("secret", &pwd_hash).unwrap());
```

### CLI (`bh`)

```bash
bh "content"              # xxHash (default)
bh -c "content"           # BLAKE3 crypto hash
bh -p "secret"            # Argon2id password hash
bh -f path/to/file        # Hash file contents
echo "secret" | bh -p -   # Password from stdin (secure)
```

### Hashing structured data

Never hash `serde_json::to_string(&value)` when key order should not matter:
with serde_json's `preserve_order` on (it is, workspace-wide, via feature
unification) the bytes follow insertion order. Use `canonical_json`,
`xx_hash_json`, or `blake3_hash_json` (`json` feature) instead. Objects are
sorted at every depth; arrays keep their order. User docs:
`biscuit-hash/docs/json.md`.

## Detailed Documentation

- [Library API Reference](./library-api.md) - Full API with HashVariant system
- [CLI Reference](./cli-reference.md) - Complete CLI flags and examples
- [Algorithm Selection](./algorithm-selection.md) - When to use each algorithm

## Key Files

| Path | Description |
|------|-------------|
| `biscuit-hash/lib/src/lib.rs` | Library entry point and re-exports |
| `biscuit-hash/lib/src/xx.rs` | xxHash with HashVariant normalization |
| `biscuit-hash/lib/src/blake.rs` | BLAKE3 implementation |
| `biscuit-hash/lib/src/argon.rs` | Argon2id with OWASP defaults |
| `biscuit-hash/lib/src/json.rs` | Canonical JSON rendering and hashes (`json` feature) |
| `biscuit-hash/cli/src/main.rs` | CLI implementation |

## Common Tasks

### Adding biscuit-hash to a crate

```toml
# Default: xxHash only
biscuit-hash = "0.1"

# With crypto
biscuit-hash = { version = "0.1", features = ["blake3"] }

# With passwords
biscuit-hash = { version = "0.1", features = ["argon2id"] }

# Canonical JSON hashing (xx_hash_json; add blake3 for blake3_hash_json)
biscuit-hash = { version = "0.1", features = ["json"] }

# All features
biscuit-hash = { version = "0.1", features = ["blake3", "argon2id", "json"] }
```

### Content-insensitive hashing

Use `HashVariant` for semantic hashing that ignores whitespace differences:

```rust
use biscuit_hash::{xx_hash_variant, HashVariant};

// Ignore leading/trailing whitespace per line
let hash = xx_hash_variant(content, vec![
    HashVariant::LeadingWhitespace,
    HashVariant::TrailingWhitespace,
    HashVariant::BlankLine,
]);
```

See [library-api.md](./library-api.md) for all variants.

## Development Commands

```bash
just -f biscuit-hash/justfile build   # Build lib and CLI
just -f biscuit-hash/justfile test    # Run all tests (--all-features)
just -f biscuit-hash/justfile lint    # Clippy (--all-features)
just -f biscuit-hash/justfile install # Install `bh` binary
just -f biscuit-hash/justfile cli "test" # Run CLI in dev mode
```
