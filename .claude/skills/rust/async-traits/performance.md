# Performance Implications of async-trait

Understanding the overhead of boxing futures and when it matters.

## The Boxing Overhead

Every async method call through `#[async_trait]` incurs:

1. **Heap allocation** - `Box::pin()` allocates the future on the heap
2. **Vtable lookup** - Dynamic dispatch through `dyn Future`
3. **Pointer indirection** - Accessing the boxed future

## Actual Overhead Numbers

Based on real-world benchmarks:

| Metric | Value |
|--------|-------|
| Per-call overhead | ~20 nanoseconds |
| 100,000 calls | ~2 milliseconds |
| 1,000,000 calls | ~20 milliseconds |

**Context:** A typical HTTP request takes 1-100ms. The async-trait overhead is negligible.

## When Overhead Matters

### High-Frequency Tight Loops

```rust
// POTENTIALLY PROBLEMATIC: millions of trait calls
#[async_trait]
trait Tokenizer {
    async fn tokenize(&self, char: char) -> Token;
}

async fn process_text(tokenizer: &dyn Tokenizer, text: &str) {
    for char in text.chars() {  // Could be millions of chars
        let token = tokenizer.tokenize(char).await;  // Boxing per char!
    }
}
```

**Better approach:** Batch processing

```rust
#[async_trait]
trait Tokenizer {
    async fn tokenize_batch(&self, text: &str) -> Vec<Token>;
}
```

### Embedded/Microcontrollers

On resource-constrained systems:
- Limited heap space
- No allocator in `#![no_std]` environments
- Every allocation has relative impact

### Real-Time Systems

Where predictable latency is critical:
- Audio processing at sample level
- Hardware control loops
- High-frequency trading (though Rust rarely used here)

## When Overhead Doesn't Matter

### I/O-Bound Operations (99% of cases)

```rust
#[async_trait]
trait Database {
    async fn query(&self, sql: &str) -> Vec<Row>;
}

// The database query takes 1-100ms
// The 20ns boxing overhead is 0.00002% of total time
```

### Web Services

```rust
#[async_trait]
trait HttpHandler: Send + Sync {
    async fn handle(&self, req: Request) -> Response;
}

// Network latency: 10-1000ms
// Boxing overhead: 20ns = negligible
```

### File Operations

```rust
#[async_trait]
trait Scanner: Send + Sync {
    async fn scan(&self) -> Vec<Model>;  // Reads filesystem
}

// Disk I/O: 0.1-10ms per operation
// Boxing overhead: invisible
```

## Benchmarking Your Code

Use criterion to measure real impact:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

#[async_trait]
trait Worker {
    async fn work(&self) -> u64;
}

struct BoxedWorker;

#[async_trait]
impl Worker for BoxedWorker {
    async fn work(&self) -> u64 { 42 }
}

struct DirectWorker;

impl DirectWorker {
    async fn work(&self) -> u64 { 42 }
}

fn bench_comparison(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("async_trait", |b| {
        let worker: Box<dyn Worker> = Box::new(BoxedWorker);
        b.iter(|| {
            rt.block_on(async { worker.work().await })
        })
    });

    c.bench_function("direct", |b| {
        let worker = DirectWorker;
        b.iter(|| {
            rt.block_on(async { worker.work().await })
        })
    });
}

criterion_group!(benches, bench_comparison);
criterion_main!(benches);
```

## Alternatives When Performance Critical

### 1. Use Generics Instead of Trait Objects

```rust
// Instead of dyn Trait with boxing:
async fn process(worker: &dyn Worker) { }

// Use generics for static dispatch:
async fn process<W: Worker>(worker: &W) { }
```

No boxing, no vtable lookup, enables inlining.

### 2. Native Async Traits (Rust 1.75+)

If you don't need `dyn Trait`:

```rust
// Native async trait - no boxing
trait Worker {
    async fn work(&self) -> u64;
}

impl Worker for MyWorker {
    async fn work(&self) -> u64 { 42 }
}

// Use with generics
async fn process<W: Worker>(worker: W) {
    worker.work().await;
}
```

### 3. Batch Operations

Amortize overhead across many items:

```rust
// BAD: Boxing per item
#[async_trait]
trait Processor {
    async fn process_one(&self, item: Item) -> Output;
}

// BETTER: Boxing once for batch
#[async_trait]
trait Processor {
    async fn process_batch(&self, items: Vec<Item>) -> Vec<Output>;
}
```

### 4. Hybrid Approach

Use traits for cold paths, direct calls for hot paths:

```rust
#[async_trait]
trait Scanner: Send + Sync {
    async fn scan(&self) -> Vec<Model>;  // Called once
}

impl OllamaScanner {
    // Direct method for internal hot path
    async fn parse_manifest(&self, path: &Path) -> Model {
        // No trait overhead here
    }
}
```

## Memory Allocation Patterns

### Box Size

The boxed future size depends on:
- Captured variables
- Async block complexity
- Compiler optimizations

Typical sizes: 64-256 bytes per future.

### Allocation Frequency

```rust
// Each call allocates:
for _ in 0..1000 {
    scanner.scan().await;  // 1000 allocations
}

// Consider pooling for extreme cases:
use futures::stream::{self, StreamExt};

stream::iter(0..1000)
    .buffer_unordered(10)  // Limits concurrent allocations
    .for_each(|_| async { scanner.scan().await; })
    .await;
```

## Production Guidance

1. **Start with async-trait** - It's the right default for trait objects
2. **Profile before optimizing** - Most async code is I/O-bound
3. **Batch when possible** - Reduces relative overhead
4. **Use generics for hot paths** - When you know the concrete type
5. **Measure in production** - Synthetic benchmarks can be misleading

## Real-World Perspective

Libraries like Tower, Axum, and Tonic use dynamic dispatch extensively. If async-trait overhead was significant, these production systems would have issues. They don't.

The overhead becomes significant only when:
- Calling async trait methods millions of times per second
- In tight loops with no I/O
- On severely resource-constrained hardware

For most applications, including model-citizen's scanner registry pattern, async-trait overhead is completely negligible.
