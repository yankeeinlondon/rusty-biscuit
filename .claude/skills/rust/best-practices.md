# Rust Best Practices

Developing in Rust is unique because the language shifts many runtime worries (memory safety, data races) to compile-time responsibilities.

## 1. Embrace the Type System

**Goal:** Make illegal states unrepresentable using the type system to enforce business logic.

### Newtype Pattern

Avoid "primitive obsession" by wrapping primitives in unique structs.

```rust
struct UserId(u32);
struct OrderId(u32);

// Can't accidentally pass OrderId where UserId expected
fn get_user(id: UserId) -> User { /* ... */ }
```

### Enums for State

Use enums to represent mutually exclusive states.

```rust
// Instead of this (multiple Options, confusing state):
struct Connection {
    connecting: Option<TcpStream>,
    connected: Option<TcpStream>,
    error: Option<Error>,
}

// Do this (clear, exclusive states):
enum Connection {
    Connecting(TcpStream),
    Connected(TcpStream),
    Error(Error),
}
```

### Zero-Sized Types (ZSTs)

Use empty structs in state machine patterns for compile-time state transitions with zero runtime overhead.

```rust
struct Locked;
struct Unlocked;

struct Safe<State> {
    contents: Vec<String>,
    _state: PhantomData<State>,
}

impl Safe<Locked> {
    fn unlock(self, code: &str) -> Result<Safe<Unlocked>, Self> {
        if code == "1234" {
            Ok(Safe {
                contents: self.contents,
                _state: PhantomData,
            })
        } else {
            Err(self)
        }
    }
}

impl Safe<Unlocked> {
    fn access(&self) -> &[String] {
        &self.contents
    }

    fn lock(self) -> Safe<Locked> {
        Safe {
            contents: self.contents,
            _state: PhantomData,
        }
    }
}
```

## 2. Memory & Ownership Best Practices

### Don't Fight the Borrow Checker

**Wrong approach:** Adding `.clone()` everywhere to fix compiler errors.

**Right approach:** Rethink data ownership. Ask: "Who *owns* this data, and who just needs to *see* it?"

```rust
// Bad: Unnecessary clones
fn process(data: String) -> String {
    let copy = data.clone();
    copy.to_uppercase()
}

// Good: Borrow when possible
fn process(data: &str) -> String {
    data.to_uppercase()
}
```

### Prefer Borrowing over Ownership

Use borrowed types for function arguments to maximize flexibility.

| Owned Type | Borrowed Type | When to Use |
|------------|---------------|-------------|
| `String` | `&str` | Function arguments |
| `Vec<T>` | `&[T]` | Function arguments |
| `PathBuf` | `&Path` | Function arguments |

```rust
// Good: Accepts both String and &str
fn greet(name: &str) {
    println!("Hello, {name}!");
}

greet("Alice");           // &str literal
greet(&name_string);      // &String (coerces to &str)
```

### Smart Pointers

Know when to use each:

```rust
// Box<T>: Heap allocation, recursive types
struct Node {
    value: i32,
    next: Option<Box<Node>>, // Recursive type
}

// Rc<T>: Shared ownership (single-threaded)
use std::rc::Rc;
let shared = Rc::new(vec![1, 2, 3]);
let clone1 = Rc::clone(&shared);
let clone2 = Rc::clone(&shared);

// Arc<T>: Shared ownership (multi-threaded)
use std::sync::Arc;
let shared = Arc::new(vec![1, 2, 3]);
std::thread::spawn(move || {
    println!("{:?}", shared);
});

// RefCell<T>: Interior mutability
use std::cell::RefCell;
let data = RefCell::new(vec![1, 2, 3]);
data.borrow_mut().push(4); // Mutate behind immutable reference
```

### Minimize unsafe

Only use `unsafe` when:
- Interfacing with C code (FFI)
- Extreme performance hot-paths with proven benchmarks
- Implementing low-level primitives

**Rules:**
- Isolate in small, well-documented modules
- Document safety invariants clearly
- Provide safe wrappers around unsafe code

```rust
/// SAFETY: Caller must ensure ptr is valid and aligned
unsafe fn read_unchecked(ptr: *const u32) -> u32 {
    *ptr
}

// Safe wrapper
fn read_safely(data: &[u32], index: usize) -> Option<u32> {
    if index < data.len() {
        Some(unsafe { read_unchecked(data.as_ptr().add(index)) })
    } else {
        None
    }
}
```

## 3. Idiomatic Error Handling

Rust treats errors as data - not exceptions.

### Result and Option

Never ignore these types. Use `?` operator for clean propagation.

```rust
fn read_config() -> Result<Config, Error> {
    let content = std::fs::read_to_string("config.toml")?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
```

### Custom Error Types (Libraries)

Use **`thiserror`** crate for libraries.

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Query failed: {0}")]
    Query(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),
}
```

### Application-Level Errors

Use **`color-eyre`** for binaries — rich, colorized error reports with context. (In the rusty-biscuit monorepo this is the standard for CLI error reporting; `anyhow` is a viable alternative in other projects.)

```rust
use color_eyre::eyre::{Result, WrapErr};

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = read_config()
        .wrap_err("Failed to read config file")?;

    let db = connect_db(&config.db_url)
        .wrap_err("Failed to connect to database")?;

    Ok(())
}
```

### Avoid unwrap() and expect()

These trigger panics (crashes). Only use in:
- Tests
- Cases where failure is mathematically impossible

```rust
// Bad: Will panic on error
let data = std::fs::read_to_string("file.txt").unwrap();

// Good: Handle error properly
let data = std::fs::read_to_string("file.txt")
    .context("Failed to read file.txt")?;

// OK in tests:
#[test]
fn test_parse() {
    let result = parse("123").unwrap();
    assert_eq!(result, 123);
}
```

## 4. Performance Optimization

Rust is "zero-cost abstraction," but your code might not be.

### Iterators over Loops

Iterators allow compiler optimizations (bounds check elimination).

```rust
// Good: Iterator (compiler can optimize)
let sum: i32 = vec.iter().filter(|x| *x > 0).sum();

// Less optimal: Manual loop
let mut sum = 0;
for &x in &vec {
    if x > 0 {
        sum += x;
    }
}
```

### Static vs. Dynamic Dispatch

| Type | Syntax | Performance | Binary Size | Use When |
|------|--------|-------------|-------------|----------|
| Static | `<T: Trait>` | Fast (inlining) | Larger (monomorphization) | Default choice |
| Dynamic | `&dyn Trait` | Slower (vtable) | Smaller | Binary size matters |

```rust
// Static dispatch (faster, larger binary)
fn process<T: Display>(item: T) {
    println!("{}", item);
}

// Dynamic dispatch (slower, smaller binary)
fn process(item: &dyn Display) {
    println!("{}", item);
}
```

### Collection Choice

```rust
// Default: Vec (cache-friendly, fast)
let mut items = Vec::new();

// Sorted keys needed: BTreeMap
let mut sorted = BTreeMap::new();

// Small collections: SmallVec (stack allocation)
use smallvec::SmallVec;
let mut small: SmallVec<[u32; 4]> = SmallVec::new(); // Stack for ≤4 items
```

## 5. Tooling and Ecosystem

Use these tools daily:

```bash
# Build and test
cargo build
cargo test
cargo bench

# Code quality
cargo clippy        # Linter (700+ checks)

# Security and performance
cargo audit        # Vulnerability scanner
cargo flamegraph   # Performance profiling
cargo bloat        # Binary size analysis

# Documentation
cargo doc --open   # Generate and view docs
```

Formatting (`cargo fmt`) is best run as a periodic standalone pass rather than on every change, since it can produce large diffs that obscure behavior changes.

## 6. Project Structure

Standard layout for maintainability:

```
my-project/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Library code (logic)
│   ├── main.rs         # Binary (thin wrapper)
│   └── modules/        # Submodules
├── tests/              # Integration tests
│   └── integration_test.rs
├── examples/           # Usage examples
│   └── basic_usage.rs
└── benches/            # Benchmarks
    └── performance.rs
```

**Benefits:**
- `lib.rs`: Testable and reusable logic
- `main.rs`: Minimal CLI wrapper
- `tests/`: External user perspective
- `examples/`: Documentation through code

### Running Examples

```bash
cargo run --example basic_usage
```

## Related

- [2024 Edition Features](./edition-2024.md) - Latest language improvements
