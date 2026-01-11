# The shared `inject_enum()` function

```rust
/// Injects the enumeration definition `new_enum` which is named `name` into the file located at the
/// filepath `file`.
/// 
/// - if the file _doesn't_ exist then then it is created and the enumeration added
/// - if the file _does_ exist 
///    - **Pre-check** before any changes are made it will use the `syn` crate to make sure the existing file has no 
///      syntax errors. If it does then returns `FileSyntax` error.
///    - assuming a successful syntax pre-check, it will copy into memory the string content of the existing 
///      implementation and then replace any pre-existing implementation of the enumeration before injecting the
///      new one from `new_enum`
///    - **Post-check** the file with the new enumeration is now checked for syntax errors; if found then the original 
///      file is left unchanged and 
/// it looks for a pre-existing enumeration of the same name and removes it before adding the new enumeration 
///      syntax errors.
/// 
async function inject_enum(name: &str, new_enum: &str, file: &str): Result<()> {
    // 
}
```

This utility function is used to inject/update a Rust enumeration into a source file so that the next time the surrounding crate is compiled the new enumeration will be used.

> **Note:** often used with [`generateProviderList()`](./generateProviderList.md) utility function

## Use of `syn` crate

The **Pre-check** and **Post-check** will use the `syn` crate to validate: 

```rust
syn::parse_file(&content)?;

Ok(())
```

