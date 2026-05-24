# Unit Tests

Unit tests verify individual functions and modules in isolation. They live alongside the code they test.

## Structure

```rust
// src/lib.rs or src/module.rs

pub fn calculate(x: i32) -> i32 {
    internal_helper(x, 2)
}

fn internal_helper(a: i32, b: i32) -> i32 {
    a * b
}

#[cfg(test)]
mod tests {
    use super::*;  // Access both public and private items

    #[test]
    fn calculate_doubles_input() {
        assert_eq!(calculate(5), 10);
    }

    #[test]
    fn internal_helper_multiplies() {
        // Can test private functions!
        assert_eq!(internal_helper(3, 4), 12);
    }
}
```

## Key Points

- `#[cfg(test)]` ensures test code is excluded from release builds
- `use super::*;` imports everything from parent module, including private items
- Each `#[test]` function runs independently

## Organizing Tests

Group related tests in submodules for clarity:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod validation {
        use super::*;

        #[test]
        fn rejects_empty_input() { /* ... */ }

        #[test]
        fn rejects_invalid_format() { /* ... */ }
    }

    mod processing {
        use super::*;

        #[test]
        fn processes_valid_input() { /* ... */ }

        #[test]
        fn handles_edge_cases() { /* ... */ }
    }
}
```

## Test Helpers

Define helper functions inside the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_user() -> User {
        User {
            id: 1,
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
        }
    }

    #[test]
    fn user_has_valid_email() {
        let user = make_test_user();
        assert!(user.email.contains('@'));
    }
}
```

## Assertions

```rust
#[test]
fn assertion_examples() {
    // Basic assertions
    assert!(condition);
    assert_eq!(actual, expected);
    assert_ne!(actual, not_expected);

    // With custom messages
    assert!(x > 0, "x should be positive, got {}", x);
    assert_eq!(result, 42, "calculation failed for input {}", input);
}
```

## Testing Results and Options

```rust
#[test]
fn test_result_handling() {
    let result: Result<i32, &str> = Ok(42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);

    let option: Option<i32> = Some(42);
    assert!(option.is_some());
    assert_eq!(option.unwrap(), 42);
}

// Tests can return Result for cleaner error handling
#[test]
fn test_with_result() -> Result<(), String> {
    let value = parse_input("42").map_err(|e| e.to_string())?;
    assert_eq!(value, 42);
    Ok(())
}
```

## Testing Panics

```rust
#[test]
#[should_panic]
fn panics_on_zero() {
    divide(10, 0);
}

#[test]
#[should_panic(expected = "division by zero")]
fn panics_with_message() {
    divide(10, 0);
}
```

## Testing Concurrency Without Flaky Timing

Tests that verify work runs *concurrently* are tempting to write as "measure total
wall-clock time, assert it is below some threshold." This is flaky: the threshold
has to sit between the concurrent time and the serial time, and process/thread
startup variance easily pushes a concurrent run past a tight ceiling.

**Measure relative timing, not absolute wall-clock.** Have each unit of work record
a start timestamp, then compare the *offset between starts*:

- Concurrent execution → all units start within startup jitter of each other.
- Serial execution → each start is delayed by the previous unit's full duration.

```rust
#[test]
fn tasks_run_concurrently() {
    // Each task records when it started, then sleeps briefly.
    let started = run_tasks(|task_id| {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(300));
        (task_id, start)
    });

    // Concurrent => starts cluster together; serial => starts are ~300ms apart.
    let first = started.iter().map(|(_, t)| *t).min().unwrap();
    let last = started.iter().map(|(_, t)| *t).max().unwrap();
    let skew = last.duration_since(first);
    assert!(
        skew < Duration::from_millis(200),
        "expected concurrent starts (skew < 200ms), got {skew:?} — tasks ran serially"
    );
}
```

Why this is robust:

- The signal is the *start offset*, which is unaffected by how long each unit runs
  or how slow the machine is — only by scheduling order.
- Only one short sleep contributes to total runtime, so the test stays fast.
- The pass/fail gap is large (jitter of a few ms vs. a full task duration), so no
  fragile threshold tuning is needed.

For work that crosses process boundaries, emit a comparable wall-clock timestamp
(e.g. epoch seconds) from each process and compare those instead of `Instant`.

Note: this strategy detects *concurrency*, not *speed*. Absolute performance
assertions (`assert!(elapsed < 1ms)`) belong in [benchmarks](./benchmarking.md),
not unit tests — they fail unpredictably on loaded CI runners.

## Ignoring Tests

```rust
#[test]
#[ignore]  // Skip by default, run with: cargo test -- --ignored
fn slow_test() {
    // Long-running test
}

#[test]
#[ignore = "requires external service"]
fn integration_heavy_test() {
    // Test that needs setup
}
```

## Related

- [Integration Tests](./integration-tests.md) - Testing public API
- [Mocking](./mocking.md) - Isolating dependencies
- [Property Testing](./property-testing.md) - Testing with generated inputs
