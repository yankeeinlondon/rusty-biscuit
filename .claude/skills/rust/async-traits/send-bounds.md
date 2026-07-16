# Send Bounds in async-trait

Understanding Send and Sync requirements for async trait methods.

## Default Behavior

By default, `#[async_trait]` adds `Send` bound to the returned future:

```rust
// Your code:
#[async_trait]
trait MyTrait {
    async fn process(&self);
}

// Expands to approximately:
trait MyTrait {
    fn process<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: Sync + 'a;
}
```

This enables:
- Moving futures between threads
- Using with work-stealing executors (tokio, async-std)
- Storing in `Arc<dyn Trait>` across spawn boundaries

## When Default Send Works

```rust
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
trait Counter: Send + Sync {
    async fn increment(&self);
    async fn get(&self) -> u64;
}

struct AtomicCounter {
    value: Arc<Mutex<u64>>,  // Mutex is Send + Sync
}

#[async_trait]
impl Counter for AtomicCounter {
    async fn increment(&self) {
        let mut guard = self.value.lock().await;
        *guard += 1;
    }

    async fn get(&self) -> u64 {
        *self.value.lock().await
    }
}

// Can spawn across threads
async fn use_counter(counter: Arc<dyn Counter>) {
    tokio::spawn(async move {
        counter.increment().await;
    });
}
```

## The ?Send Variant

Use `#[async_trait(?Send)]` when your async method uses non-Send types:

```rust
use async_trait::async_trait;
use std::rc::Rc;
use std::cell::RefCell;

// Types that are !Send
struct LocalState {
    data: Rc<RefCell<Vec<u8>>>,  // Rc is !Send
}

#[async_trait(?Send)]
trait LocalProcessor {
    async fn process(&self, state: &LocalState);
}

#[async_trait(?Send)]
impl LocalProcessor for MyProcessor {
    async fn process(&self, state: &LocalState) {
        // Can use Rc, RefCell, etc.
        state.data.borrow_mut().push(42);
    }
}
```

**Critical:** Must use `?Send` on BOTH trait definition AND all implementations.

## Common !Send Types

Types that require `#[async_trait(?Send)]`:

| Type | Reason |
|------|--------|
| `Rc<T>` | Reference counting not atomic |
| `RefCell<T>` | Runtime borrow checking not thread-safe |
| `Cell<T>` | Interior mutability not thread-safe |
| `*const T`, `*mut T` | Raw pointers are !Send by default |
| Most C FFI types | Thread safety unknown |

## Trait Object Constraints

For `Box<dyn Trait>` or `&dyn Trait`, the trait needs appropriate bounds:

```rust
#[async_trait]
trait ThreadSafe: Send + Sync {
    async fn method(&self);
}

// Works with dynamic dispatch across threads
fn spawn_with_trait(handler: Arc<dyn ThreadSafe>) {
    tokio::spawn(async move {
        handler.method().await;
    });
}
```

Without `Send + Sync` on the trait:

```rust
#[async_trait]
trait NotThreadSafe {
    async fn method(&self);
}

// Error: `dyn NotThreadSafe` cannot be sent between threads
fn spawn_with_trait(handler: Box<dyn NotThreadSafe>) {
    tokio::spawn(async move {
        handler.method().await;  // Compile error!
    });
}
```

## Sync Requirement for &self

When using `&self`, the macro adds `where Self: Sync`:

```rust
#[async_trait]
trait Example {
    async fn with_ref(&self);      // Requires Self: Sync
    async fn with_mut(&mut self);  // Does NOT require Sync
    async fn by_value(self);       // Does NOT require Sync
}
```

This ensures shared references (`&self`) are safe to hold across await points.

## Mixing Send and Non-Send

You can have different impls with different Send requirements using separate traits:

```rust
#[async_trait]
trait ThreadSafeProcessor: Send + Sync {
    async fn process(&self, data: &[u8]);
}

#[async_trait(?Send)]
trait LocalProcessor {
    async fn process(&self, data: &[u8]);
}

// Thread-safe implementation
struct SafeImpl;

#[async_trait]
impl ThreadSafeProcessor for SafeImpl {
    async fn process(&self, data: &[u8]) { /* ... */ }
}

// Local-only implementation
struct LocalImpl {
    cache: Rc<RefCell<HashMap<String, Vec<u8>>>>,
}

#[async_trait(?Send)]
impl LocalProcessor for LocalImpl {
    async fn process(&self, data: &[u8]) {
        self.cache.borrow_mut().insert("key".into(), data.to_vec());
    }
}
```

## Debugging Send Errors

Common error messages and solutions:

### "future cannot be sent between threads safely"

```
error: future cannot be sent between threads safely
   --> src/lib.rs:15:1
    |
15  | #[async_trait]
    | ^^^^^^^^^^^^^^ future created by async block is not `Send`
```

**Solution:** Either:
1. Replace !Send types with Send equivalents (Rc -> Arc, RefCell -> Mutex)
2. Use `#[async_trait(?Send)]` on both trait and impl

### "the trait bound `Self: Sync` is not satisfied"

```
error[E0277]: the trait bound `Self: Sync` is not satisfied
```

**Solution:** Add `Sync` bound to implementor or use `&mut self` instead of `&self`.

## Best Practices

1. **Default to Send** - Use standard `#[async_trait]` unless you have !Send types
2. **Explicit trait bounds** - Add `Send + Sync` to trait definition for trait objects
3. **Document thread safety** - Note when `?Send` is used and why
4. **Prefer Send types** - Use `Arc<Mutex<T>>` over `Rc<RefCell<T>>` when possible
5. **Test with spawn** - Verify trait objects work with `tokio::spawn` if needed
