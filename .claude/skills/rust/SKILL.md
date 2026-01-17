---
name: rust
description: Expert knowledge for Rust systems programming covering ownership, borrowing, type safety, error handling, async patterns, performance optimization, and the 2024 edition improvements for building safe, concurrent, and high-performance applications
last_updated: 2025-12-20T00:00:00Z
hash: 987826de0dddb854
---

# Rust

Expert guidance for Rust systems programming. Rust shifts runtime worries (memory safety, data races) to compile-time, enabling safe, concurrent, and high-performance code.

## Core Principles

- **Make illegal states unrepresentable** - Use the type system to enforce business logic at compile time
- **Don't fight the borrow checker** - Rethink data ownership rather than adding `.clone()` everywhere
- **Prefer borrowing over ownership** - Use `&str` over `String`, `&[T]` over `Vec<T>` for function arguments
- **Treat errors as data** - Never ignore `Result` or `Option`, use `?` operator for clean propagation
- **Minimize unsafe** - Only use when absolutely necessary (FFI, extreme performance), isolate in small modules
- **Iterators over loops** - Rust iterators allow compiler optimizations that manual loops don't
- **Static dispatch by default** - Use `<T: Trait>` for speed, `&dyn Trait` only when binary size matters
- **Use the tooling** - Clippy catches 700+ mistakes, rustfmt ensures consistency, cargo audit finds vulnerabilities

## Quick Reference

### Common Smart Pointers

```rust
// Heap allocation or recursive types
let data = Box::new(value);

// Shared ownership (single-threaded)
let shared = Rc::new(value);

// Shared ownership (multi-threaded)
let shared = Arc::new(value);

// Interior mutability (mutate behind immutable reference)
let cell = RefCell::new(value);
let mut borrowed = cell.borrow_mut();
```

### Newtype Pattern

```rust
// Avoid primitive obsession
struct UserId(u32);
struct OrderId(u32);

fn get_user(id: UserId) { /* can't pass OrderId by mistake */ }
```

### Error Handling

```rust
// Library: custom error types with thiserror
#[derive(thiserror::Error, Debug)]
enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid input: {0}")]
    Invalid(String),
}

// Application: anyhow for unified error handling
use anyhow::Result;

fn process() -> Result<()> {
    let data = read_file()?;
    let parsed = parse_data(data)?;
    Ok(())
}
```

## Topics

### Edition Updates

- [2024 Edition Features](./edition-2024.md) - Lifetime improvements, async support, enhanced safety, gen keyword

### Development Practices

- [Best Practices](./best-practices.md) - Type system usage, ownership patterns, error handling, performance, tooling, project structure

## Common Patterns

### State Machine with Zero-Sized Types

```rust
struct Locked;
struct Unlocked;

struct Door<State> {
    _state: PhantomData<State>,
}

impl Door<Locked> {
    fn unlock(self) -> Door<Unlocked> {
        Door { _state: PhantomData }
    }
}

impl Door<Unlocked> {
    fn lock(self) -> Door<Locked> {
        Door { _state: PhantomData }
    }
}
```

### Async Closures (2024 Edition)

```rust
// Native async closure syntax
let async_op = async || {
    fetch_data().await
};

// Future and IntoFuture are now in prelude (no manual import needed)
```

## Essential Tooling

| Tool | Purpose | Command |
|------|---------|---------|
| **Cargo** | Build system and package manager | `cargo build` |
| **Clippy** | Linter (700+ checks) | `cargo clippy` |
| **Rustfmt** | Code formatter | `cargo fmt` |
| **Cargo Audit** | Security vulnerability scanner | `cargo audit` |
| **Flamegraph** | Performance profiling | `cargo flamegraph` |

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
