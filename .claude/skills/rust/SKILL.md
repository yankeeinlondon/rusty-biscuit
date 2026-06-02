---
name: rust
description: Expert knowledge for Rust systems programming — ownership, borrowing, type safety, error handling, async patterns, performance optimization, and 2024-edition improvements. Use when writing or reviewing idiomatic Rust, resolving borrow-checker or lifetime issues, structuring error handling, or optimizing performance.
last_updated: 2026-06-02T00:00:00Z
hash: 352b2caf7cdd68a6-5e51ae1f8d2f661c
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
- **Use the tooling** - Clippy catches 700+ mistakes and cargo audit finds vulnerabilities; run formatting as a separate periodic pass, not during implementation (see [In this monorepo](#in-this-monorepo))

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

// Application/CLI: color-eyre for rich, unified error reports
use color_eyre::eyre::Result;

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

Encode states as ZSTs so invalid transitions fail to compile, with zero runtime cost. See [Best Practices → Zero-Sized Types](./best-practices.md#zero-sized-types-zsts) for a worked `Safe<Locked>` / `Safe<Unlocked>` example.

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

> **In this monorepo:** drive builds and tests through `just` and `cargo … -p <pkg>`; `cargo fmt` runs only as a periodic standalone pass. See [In this monorepo](#in-this-monorepo).

## In this monorepo

The generic advice above is overridden here by rusty-biscuit conventions.

### Build, test, format

- Prefer `just` recipes: `just test | lint | build | doctest`. The root `justfile` covers a curated area list (not every workspace member).
- Never run a bare `cargo build` / `cargo test` at the repo root — scope to a package: `cargo build -p <pkg>`. There are 48 workspace members and the target dir is large.
- Tests run under **nextest** with an L1/L2/L3 level taxonomy — see the `rust-testing` skill for `require_level!` and the canonical recipes.
- **`cargo fmt` is a periodic standalone pass — never run it during implementation or testing.** It produces large, noisy diffs that bury behavior changes.

### Error handling

- Libraries: `thiserror` (as above).
- Binaries / CLIs: **`color-eyre`**, not `anyhow` — it is the repo standard for rich, colorized error reports.

### Rustdoc & comments

- No `# H1` inside `///` — rustdoc already titles the item.
- `## H2` sections, in order: `Examples` → `Returns` → `Errors` → `Panics` → `Safety` → `Notes`.
- Comments should carry what the code cannot (contracts, invariants, the surprising *why*). Remove on sight: HOW-narration, tautological examples, format/glyph narration, and stale comments. Full rules: [`docs/comment-quality.md`](../../../docs/comment-quality.md) and the rustdoc section of the repo `CLAUDE.md`.

### Hashing

Use **biscuit-hash** (xxHash) for general content. For Markdown documents use the **Darkmatter** hasher (`md hash <file>`), which segments frontmatter from body.

### Related skills

When working in a specific domain, reach for the specialized skill:

- **Testing**: `rust-testing`, `nextest`
- **Errors**: `thiserror`, `color-eyre`
- **CLIs**: `clap`
- **Serialization**: `serde`
- **Async**: `tokio`

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)