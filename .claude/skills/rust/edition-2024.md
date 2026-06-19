# Rust 2024 Edition Features

The Rust 2024 edition (introduced with Rust 1.85) is a special release that makes backward-incompatible changes to clean up the language. While Rust releases every six weeks with new features, Editions come every three years.

The primary benefit of moving to the 2024 edition is a more intuitive, safe, and modern developer experience.

## Ergonomic Lifetime & Scope Improvements

One of the biggest quality-of-life upgrades is how temporary values and lifetimes are handled, reducing "fighting the borrow checker."

### Tail Expression Scopes

**Change:** In a block's tail expression, temporaries are now dropped **before** the block's local variables (in 2021 they were dropped after). This drop-order change resolves a class of borrow-checker errors where a temporary in the return expression outlived a local it conflicted with.

```rust
fn last_len(cell: &RefCell<Vec<i32>>) -> usize {
    // The temporary `Ref` from `borrow()` is released before any locals,
    // so patterns that previously over-extended the borrow now compile.
    cell.borrow().len()
}
```

### if let Temporary Scopes

**Problem in 2021:** `if let` kept temporaries alive for the entire else block, potentially keeping Mutex locks longer than needed.

```rust
// 2021: Mutex stays locked through else block
if let Some(val) = mutex.lock().unwrap().get(0) {
    // ...
} else {
    // Mutex still locked here! Risk of deadlock
}
```

**Solution in 2024:** Temporaries are dropped sooner, preventing accidental deadlocks and borrow issues.

### RPIT Lifetime Capture

**Change:** How `impl Trait` in return positions captures lifetimes is now more consistent with regular functions, removing many "hidden" lifetime requirements that confused developers.

```rust
// More predictable lifetime behavior
fn process(data: &str) -> impl Iterator<Item = &str> {
    data.split(',')
}
```

## Modernized Async Support

Rust 2024 is the foundation for "Async Rust 2.0."

### Async Closures

**New:** Native `async || { ... }` syntax with optimized internal handling.

```rust
// Annotate the future's output type so `?` resolves.
let fetch = async |url: &str| -> reqwest::Result<serde_json::Value> {
    let response = reqwest::get(url).await?;
    response.json().await
};
```

### Prelude Updates

**Change:** `Future` and `IntoFuture` are now in the prelude.

```rust
// 2021: Manual import required
use std::future::Future;

// 2024: No import needed, cleaner async code
```

## Increased Safety "By Default"

Rust 2024 tightens rules to make "the right way" the "only way."

### Unsafe Operations in Unsafe Functions

**Change:** `unsafe fn` body is no longer automatically an unsafe block. You must wrap specific unsafe operations explicitly.

```rust
// 2024: Explicit unsafe blocks required
unsafe fn raw_access(ptr: *const u32) -> u32 {
    // This is NOT automatically unsafe anymore
    let x = 5; // Safe operation

    unsafe {
        // Only this is unsafe
        *ptr
    }
}
```

**Benefit:** Pinpoints exactly where danger lies, easier to audit.

### Unsafe Attributes

**Change:** Attributes like `#[no_mangle]` or `#[export_name]` now require the `unsafe` keyword.

```rust
// 2024: Acknowledge system-level assumptions
#[unsafe(no_mangle)]
pub extern "C" fn exported_function() {
    // ...
}
```

### Disallowing static mut References

**Change:** Creating standard references (`&` or `&mut`) to `static mut` is now a hard error.

```rust
static mut COUNTER: u32 = 0;

// 2021: Allowed (unsafe)
unsafe {
    let r = &COUNTER; // Dangerous for concurrency
}

// 2024: Error - must use raw pointers
unsafe {
    let ptr = std::ptr::addr_of!(COUNTER); // Safer
}
```

**Benefit:** Much safer for concurrent access.

## Future-Proofing

### gen Keyword Reserved

**Change:** The `gen` keyword is reserved so future generator syntax won't break existing code. The feature itself is **not yet stabilized**; the shape below is illustrative.

```rust
// Illustrative — `gen` blocks are not stable yet.
// A `gen { }` block evaluates to an iterator; `yield` produces items.
fn fibonacci() -> impl Iterator<Item = u32> {
    gen {
        let (mut a, mut b) = (0, 1);
        loop {
            yield a;
            (a, b) = (b, a + b);
        }
    }
}
```

**Benefit:** First-class generators (like Python/JavaScript) will work without breaking existing code.

## Migration Guide

To upgrade to 2024 edition:

1. **Update Cargo.toml:**
   ```toml
   [package]
   edition = "2024"
   ```

2. **Run cargo fix:**
   ```bash
   cargo fix --edition
   ```

3. **Run tests:**
   ```bash
   cargo test
   ```

4. **Address any remaining warnings** about deprecated patterns

## Related

- [Best Practices](./best-practices.md) - Idiomatic Rust patterns
