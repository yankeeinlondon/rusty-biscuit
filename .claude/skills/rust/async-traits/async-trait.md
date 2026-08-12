---
name: async-trait
description: Expert knowledge for the Rust async-trait crate — the #[async_trait] macro for object-safe async traits, Send/Sync bounds, the ?Send variant, and the performance trade-offs of boxing futures. Use when adding async methods to traits, building object-safe async trait objects, or deciding between async-trait and native async fn in traits.
hash: async-trait-skill-v1
---

# async-trait

A procedural macro by David Tolnay that enables async functions in traits to work with dynamic dispatch (`dyn Trait`). Essential when you need trait objects with async methods.

**Version:** 0.1.89 (latest as of 2025)
**Use for:** Dynamic dispatch with async traits, plugin systems, dependency injection with async interfaces.

## When to Use async-trait vs Native Async Traits

| Scenario | Solution |
|----------|----------|
| Static dispatch only (generics) | Native `async fn` in traits (Rust 1.75+) |
| Need `dyn Trait` / trait objects | `#[async_trait]` required |
| Pre-Rust 1.75 compatibility | `#[async_trait]` required |
| Performance-critical tight loops | Consider avoiding trait objects entirely |

**Key insight:** Native async traits (Rust 1.75+) do NOT support `dyn Trait`. If you need trait objects with async methods, async-trait is still required.

## Core Usage

Apply `#[async_trait]` to both trait definition AND all implementations:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ModelScanner: Send + Sync {
    async fn scan(&self) -> Result<Vec<Model>, ScanError>;
    async fn is_available(&self) -> bool;
    fn name(&self) -> &'static str;  // Non-async methods work normally
}

#[async_trait]
impl ModelScanner for OllamaScanner {
    async fn scan(&self) -> Result<Vec<Model>, ScanError> {
        // Implementation
    }

    async fn is_available(&self) -> bool {
        self.client.health_check().await.is_ok()
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}
```

## Macro Transformation

The macro transforms async methods into boxed futures:

```rust
// Your code:
async fn scan(&self) -> Vec<Model>;

// Expands to:
fn scan<'async_trait>(&'async_trait self)
    -> Pin<Box<dyn Future<Output = Vec<Model>> + Send + 'async_trait>>
where
    Self: Sync + 'async_trait
{
    Box::pin(async move { /* your implementation */ })
}
```

## Topics

### Bounds and Constraints

- [Send Bounds](./send-bounds.md) - Default Send requirement, ?Send variant, thread safety

### Performance

- [Boxing Overhead](./performance.md) - Allocation costs, benchmarks, when it matters

### Patterns

- [Trait Object Usage](./trait-objects.md) - Registry pattern, plugin systems, dependency injection

## Send + Sync Bounds

**Default behavior:** Futures are `Send` (can move between threads).

```rust
// Default: Send bound on future
#[async_trait]
trait MyTrait {
    async fn method(&self);  // Future: Pin<Box<dyn Future + Send>>
}
```

**For single-threaded contexts** (e.g., `!Send` types, `Rc`, `RefCell`):

```rust
// Remove Send bound - use on BOTH trait and impl
#[async_trait(?Send)]
trait LocalTrait {
    async fn method(&self);  // Future: Pin<Box<dyn Future>> (no Send)
}

#[async_trait(?Send)]
impl LocalTrait for MyType {
    async fn method(&self) { /* ... */ }
}
```

## Trait Object Pattern

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Scanner: Send + Sync {
    async fn scan(&self) -> Vec<Item>;
}

// Registry holding boxed trait objects
pub struct Registry {
    scanners: Vec<Box<dyn Scanner>>,
}

impl Registry {
    pub fn add(&mut self, scanner: impl Scanner + 'static) {
        self.scanners.push(Box::new(scanner));
    }

    pub async fn scan_all(&self) -> Vec<Item> {
        let futures: Vec<_> = self.scanners.iter().map(|s| s.scan()).collect();
        futures::future::join_all(futures).await.into_iter().flatten().collect()
    }
}
```

## Performance Implications

**Overhead per call:** ~20 nanoseconds (heap allocation for boxed future)

**When it matters:**
- Millions of calls per second in tight loops
- Embedded/microcontroller environments
- Latency-critical hot paths

**When it doesn't matter (most cases):**
- Web servers, API handlers
- Database operations
- File I/O
- Network requests

**Benchmark perspective:** 100K calls = ~2ms overhead. Usually negligible compared to actual I/O.

## Common Mistakes

### Forgetting macro on impl

```rust
#[async_trait]
trait MyTrait { async fn method(&self); }

// WRONG: Missing #[async_trait]
impl MyTrait for MyType {
    async fn method(&self) { }  // Compile error!
}

// CORRECT:
#[async_trait]
impl MyTrait for MyType {
    async fn method(&self) { }
}
```

### Mismatched Send bounds

```rust
#[async_trait]        // Send bound
trait MyTrait { ... }

#[async_trait(?Send)] // No Send bound - MISMATCH!
impl MyTrait for MyType { ... }  // Compile error
```

### Missing trait bounds for trait objects

```rust
// Won't work as dyn Trait:
#[async_trait]
trait BadTrait {
    async fn method(&self);
}

// Works as dyn Trait:
#[async_trait]
trait GoodTrait: Send + Sync {
    async fn method(&self);
}

let scanner: Box<dyn GoodTrait> = Box::new(MyImpl);  // Works!
```

## Lifetime Elision

Async-trait supports lifetime elision in `&` and `&mut` references only:

```rust
#[async_trait]
trait Valid {
    async fn process(&self, data: &str);  // OK: elision works
}

#[async_trait]
trait NeedsExplicit {
    // Must use explicit lifetime or '_ for non-reference types
    async fn process(&self, data: Cow<'_, str>);
}
```

## Alternative: trait_variant (Native + Send)

For native async traits that need Send bounds without full boxing:

```rust
use trait_variant::make;

#[trait_variant::make(SendScanner: Send)]
trait LocalScanner {
    async fn scan(&self) -> Vec<Model>;
}

// Generates two traits:
// - LocalScanner: no Send bound
// - SendScanner: with Send bound on futures
```

**Limitation:** Still no `dyn Trait` support - use async-trait for that.

## Resources

- [Crate Documentation](https://docs.rs/async-trait)
- [GitHub Repository](https://github.com/dtolnay/async-trait)
- [trait_variant crate](https://docs.rs/trait-variant) - For native async + Send bounds
- [Rust Async Book](https://rust-lang.github.io/async-book/)
