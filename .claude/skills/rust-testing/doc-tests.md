# Documentation Tests

Doc tests are executable code examples in documentation comments. They serve as both documentation and tests.

## Basic Example

````rust
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use my_crate::add;
///
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
````

## Running Doc Tests

```bash
cargo test --doc        # Run only doc tests
cargo test              # Runs all tests including doc tests
```

## Hiding Setup Code

Use `#` to hide lines from rendered docs while keeping them in the test:

````rust
/// Returns the first element of a slice.
///
/// # Examples
///
/// ```
/// # use my_crate::first;
/// # let items = vec![1, 2, 3];
/// let result = first(&items);
/// assert_eq!(result, Some(&1));
/// ```
pub fn first<T>(slice: &[T]) -> Option<&T> {
    slice.first()
}
````

Rendered docs show only:

```rust
let result = first(&items);
assert_eq!(result, Some(&1));
```

## Showing Errors

Document how errors are handled:

````rust
/// Parses a string as an integer.
///
/// # Errors
///
/// Returns an error if the string is not a valid integer.
///
/// # Examples
///
/// ```
/// use my_crate::parse_int;
///
/// assert_eq!(parse_int("42"), Ok(42));
/// assert!(parse_int("not a number").is_err());
/// ```
pub fn parse_int(s: &str) -> Result<i32, std::num::ParseIntError> {
    s.parse()
}
````

## Testing Panics

Use `should_panic` attribute:

````rust
/// Divides two numbers.
///
/// # Panics
///
/// Panics if `b` is zero.
///
/// # Examples
///
/// ```should_panic
/// use my_crate::divide;
///
/// divide(10, 0); // This panics!
/// ```
pub fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("division by zero");
    }
    a / b
}
````

## Compile-Only Examples

Use `no_run` for examples that should compile but not execute:

````rust
/// Connects to the database.
///
/// # Examples
///
/// ```no_run
/// use my_crate::connect;
///
/// let conn = connect("postgres://localhost/mydb").unwrap();
/// ```
pub fn connect(url: &str) -> Result<Connection, Error> {
    // ...
}
````

## Ignoring Examples

Use `ignore` for incomplete or platform-specific examples:

````rust
/// Platform-specific initialization.
///
/// # Examples
///
/// ```ignore
/// // This only works on Windows
/// init_windows_specific();
/// ```
````

## Testing Compile Failures

Use `compile_fail` to verify code that shouldn't compile:

````rust
/// This type cannot be cloned.
///
/// ```compile_fail
/// use my_crate::UniqueHandle;
///
/// let h1 = UniqueHandle::new();
/// let h2 = h1.clone(); // Error: Clone not implemented
/// ```
pub struct UniqueHandle { /* ... */ }
````

## Multi-Line Examples

````rust
/// Processes a configuration file.
///
/// # Examples
///
/// ```
/// use my_crate::Config;
///
/// let config = Config::builder()
///     .name("my-app")
///     .version("1.0.0")
///     .debug(true)
///     .build()
///     .unwrap();
///
/// assert_eq!(config.name(), "my-app");
/// ```
````

## Best Practices

1. **Always include `# Examples` section** for public APIs
2. **Use `#` to hide boilerplate** - keep examples focused
3. **Test the happy path** - show typical usage
4. **Document edge cases** in separate examples
5. **Keep examples simple** - complex logic belongs in integration tests

## Related

- [Unit Tests](./unit-tests.md) - Testing internal implementation
- [Integration Tests](./integration-tests.md) - Testing public API comprehensively
