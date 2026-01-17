# Property-Based Testing

Property-based testing verifies that code properties hold for many randomly generated inputs. Use `proptest` for this in Rust.

## Setup

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1"
```

## Basic Property Test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn addition_is_commutative(a: i32, b: i32) {
        prop_assert_eq!(a + b, b + a);
    }

    #[test]
    fn sort_preserves_length(mut vec: Vec<i32>) {
        let original_len = vec.len();
        vec.sort();
        prop_assert_eq!(vec.len(), original_len);
    }
}
```

## Common Properties to Test

| Property | Example |
|----------|---------|
| **Commutativity** | `f(a, b) == f(b, a)` |
| **Associativity** | `f(f(a, b), c) == f(a, f(b, c))` |
| **Identity** | `f(a, identity) == a` |
| **Idempotence** | `f(f(a)) == f(a)` |
| **Inverse/Round-trip** | `decode(encode(x)) == x` |
| **Invariants** | Property holds before and after operation |

## Custom Strategies

Control input generation with strategies:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_with_bounded_values(
        x in 0..100i32,           // Range
        s in "[a-z]{1,10}",       // Regex for strings
        v in prop::collection::vec(0..1000i32, 1..50),  // Vec with size range
    ) {
        prop_assert!(x >= 0 && x < 100);
        prop_assert!(s.len() >= 1 && s.len() <= 10);
        prop_assert!(v.len() >= 1 && v.len() < 50);
    }
}
```

## Testing Round-Trip Serialization

```rust
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Config {
    name: String,
    value: i32,
}

// Custom strategy for generating Config
fn config_strategy() -> impl Strategy<Value = Config> {
    ("[a-z]{1,20}", any::<i32>()).prop_map(|(name, value)| Config { name, value })
}

proptest! {
    #[test]
    fn json_roundtrip(config in config_strategy()) {
        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config, decoded);
    }
}
```

## Testing Invariants

```rust
use proptest::prelude::*;

struct SortedVec {
    inner: Vec<i32>,
}

impl SortedVec {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }

    fn insert(&mut self, value: i32) {
        let pos = self.inner.binary_search(&value).unwrap_or_else(|p| p);
        self.inner.insert(pos, value);
    }

    fn is_sorted(&self) -> bool {
        self.inner.windows(2).all(|w| w[0] <= w[1])
    }
}

proptest! {
    #[test]
    fn sorted_vec_maintains_order(values: Vec<i32>) {
        let mut sv = SortedVec::new();
        for v in values {
            sv.insert(v);
            prop_assert!(sv.is_sorted(), "Invariant violated after inserting {}", v);
        }
    }
}
```

## Shrinking

When a test fails, proptest automatically shrinks the input to find the minimal failing case:

```rust
proptest! {
    #[test]
    fn finds_minimal_failure(v: Vec<i32>) {
        // If this fails for [1, 2, 3, 4, 5], proptest will shrink
        // to find the smallest failing input (e.g., [3] if len > 2 is the issue)
        prop_assert!(v.len() <= 2);
    }
}
```

## Configuring Test Runs

```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn many_iterations(x: i32) {
        // Runs 1000 times instead of default 256
        prop_assert!(x.checked_add(0) == Some(x));
    }
}
```

## When to Use Property Testing

**Good candidates:**

- Parsers and serializers (round-trip property)
- Mathematical operations (algebraic properties)
- Data structures (invariant maintenance)
- String manipulation (length preservation, etc.)
- Sorting and searching algorithms

**Less suitable:**

- UI interactions
- Tests requiring specific inputs
- External service integration

## Combining with Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit test: specific known case
    #[test]
    fn add_specific_values() {
        assert_eq!(add(2, 3), 5);
    }

    // Property test: general behavior
    proptest! {
        #[test]
        fn add_commutative(a: i32, b: i32) {
            prop_assert_eq!(add(a, b), add(b, a));
        }
    }
}
```

## Related

- [Unit Tests](./unit-tests.md) - Specific example-based tests
- [Fuzzing](./fuzzing.md) - Finding crashes with random input
- [Benchmarking](./benchmarking.md) - Performance testing
