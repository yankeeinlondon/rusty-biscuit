# Canonical JSON hashing

The `json` feature gives you a stable text form for a `serde_json::Value` and
hashes of that form. Two values that hold the same data but list object keys
in a different order produce the same text and the same hash. Use it when you
need an identity for structured data (a cache key, a change-detection
fingerprint) and key order is not part of what you care about.

```toml
biscuit-hash = { version = "0.1", features = ["json"] }
```

## Rendering

`canonical_json(&Value) -> String` writes compact JSON (no whitespace) with
these rules:

- **Object keys are sorted at every depth**, including objects inside arrays.
  Keys compare by `str` order, which is UTF-8 byte order (`"Z" < "a" < "é"`).
- **Array elements keep their order.** `[1, 2]` and `[2, 1]` are different.
- **Scalars and string escaping are exactly serde_json's.** For a value whose
  objects are already sorted, the output equals `serde_json::to_string`.
- **The input is never changed**, so a map that keeps insertion order (serde_json's
  `preserve_order` feature) still iterates in its original order afterward.
- **It cannot fail.** It returns a `String`, not a `Result`.

```rust
use biscuit_hash::canonical_json;
use serde_json::json;

let first = json!({"title": "T", "meta": {"owner": "ken", "tags": ["a", "b"]}});
let second = json!({"meta": {"tags": ["a", "b"], "owner": "ken"}, "title": "T"});

assert_eq!(canonical_json(&first), canonical_json(&second));
assert_eq!(
    canonical_json(&first),
    r#"{"meta":{"owner":"ken","tags":["a","b"]},"title":"T"}"#
);
```

The rendering does not depend on whether serde_json's `preserve_order` feature
is on anywhere in your build: the same value always renders to the same bytes.

## Hash algorithms

The hash functions hash the bytes of `canonical_json`; there is no second
algorithm that could drift from it.

| Function | Returns | Same as |
|---|---|---|
| `xx_hash_json(&Value)` | `u64` | `xx_hash_bytes(canonical_json(v).as_bytes())` |
| `blake3_hash_json(&Value)` | 64-char hex `String` | `blake3_hash(&canonical_json(v))` |

Pick xxHash for fast, non-security identities (cache keys, change detection)
and BLAKE3 when the hash must resist deliberate collisions.

## Feature combinations

`json` turns on only the renderer. Each hash function also needs its
algorithm's feature; `json` never enables an algorithm by itself.

| Features | You get | Example |
|---|---|---|
| `json` (with `default-features = false`) | `canonical_json` | `canonical_json(&v)` |
| `json` (default features on) | `canonical_json`, `xx_hash_json` | `xx_hash_json(&v)` |
| `json`, `blake3` | `canonical_json`, `xx_hash_json`, `blake3_hash_json` | `blake3_hash_json(&v)` |

With `default-features = false`, add `xx_hash` explicitly to get
`xx_hash_json`:

```toml
biscuit-hash = { version = "0.1", default-features = false, features = ["json", "xx_hash"] }
```
