# Code Generation Safety

The `codegen` module provides safe AST-based Rust code injection, ensuring code modifications are syntactically valid and atomically applied.

## Core Principles

1. **AST-based manipulation**: Uses `syn` for parsing, never regex
2. **Pre/Post validation**: All code validated before and after modification
3. **Atomic writes**: Tempfile + rename prevents partial writes
4. **Rollback safety**: Original files untouched if validation fails

## API Overview

```rust
use shared::codegen::{inject_enum, inject_enum_variants, validate_syntax};

// Replace entire enum
inject_enum("Color", new_enum_code, "src/types.rs")?;

// Add variants to existing enum
inject_enum_variants("Status", new_variants, "src/models.rs")?;

// Validate syntax without modifying
validate_syntax(rust_code)?;
```

## Enum Injection

### Full Enum Replacement

```rust
let new_enum = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
    #[deprecated]
    Yellow,
}
"#;

inject_enum("Color", new_enum, "src/types.rs")?;
```

Features:
- Preserves file structure around the enum
- Maintains proper formatting with `prettyplease`
- Validates both old and new syntax
- Atomic file replacement

### Variant Injection

```rust
let new_variants = r#"
    /// New status for pending review
    PendingReview,
    /// Item has been archived
    #[deprecated(since = "2.0", note = "Use Deleted instead")]
    Archived,
"#;

inject_enum_variants("Status", new_variants, "src/models.rs")?;
```

Behavior:
- Appends variants to existing enum
- Preserves all existing variants
- Maintains attributes and documentation
- Handles trailing commas correctly

## Safety Guarantees

### Pre-validation

```rust
// The inject functions internally validate:
// 1. Target file is valid Rust
// 2. Enum exists in file
// 3. New code is syntactically valid
```

### Post-validation

```rust
// After injection:
// 1. Modified code is re-parsed
// 2. Enum presence is verified
// 3. File is formatted with prettyplease
```

### Atomic Operations

```rust
// Under the hood:
use tempfile::NamedTempFile;

// 1. Write to temporary file
let temp_file = NamedTempFile::new_in(parent_dir)?;
temp_file.write_all(new_content.as_bytes())?;

// 2. Validate temporary file
validate_syntax(&new_content)?;

// 3. Atomic rename (prevents partial writes)
temp_file.persist(target_path)?;
```

## Error Handling

```rust
use shared::codegen::CodegenError;

match inject_enum("MyEnum", code, path) {
    Ok(()) => println!("Enum updated successfully"),

    Err(CodegenError::SyntaxError { message }) => {
        eprintln!("Invalid Rust syntax: {}", message);
    }

    Err(CodegenError::EnumNotFound { name }) => {
        eprintln!("Enum '{}' not found in file", name);
    }

    Err(CodegenError::IoError(e)) => {
        eprintln!("File operation failed: {}", e);
    }

    Err(CodegenError::PersistError(e)) => {
        eprintln!("Failed to save changes: {}", e);
    }
}
```

## Use Cases

### Model Updates

Used by the provider discovery system:

```rust
// Update provider models from API
cargo run --bin update-provider-models

// This internally uses:
inject_enum("ModelId", generated_enum, "src/model/providers.rs")?;
```

### Code Generation Scripts

```rust
// Generate enum from data
let variants: Vec<String> = fetch_variants_from_api().await?;
let enum_code = format!(
    "pub enum ApiStatus {{\n{}\n}}",
    variants.join(",\n")
);

inject_enum("ApiStatus", &enum_code, "src/generated.rs")?;
```

## Implementation Details

### AST Traversal

```rust
use syn::{File, Item, ItemEnum};

// Parse file into AST
let ast: File = syn::parse_str(&content)?;

// Find target enum
let enum_item = ast.items.iter()
    .find_map(|item| match item {
        Item::Enum(e) if e.ident == target => Some(e),
        _ => None,
    })
    .ok_or(CodegenError::EnumNotFound { name: target })?;
```

### Formatting

```rust
// Format with prettyplease for consistent style
let formatted = prettyplease::unparse(&ast);
```

## Best Practices

1. **Always validate first**: Use `validate_syntax()` for testing
2. **Backup critical files**: The module is safe but backups are wise
3. **Use in build scripts**: Ideal for code generation tasks
4. **Version control**: Commit before bulk updates

## Examples

### Build Script Integration

```rust
// build.rs
use shared::codegen::inject_enum;

fn main() {
    // Generate enums at build time
    let schema = read_schema_file()?;
    let enum_code = generate_enum_from_schema(&schema);

    inject_enum(
        "GeneratedTypes",
        &enum_code,
        "src/generated/types.rs"
    ).expect("Failed to inject enum");
}
```

### Safe Updates

```rust
// Safely update multiple enums
fn update_enums(updates: Vec<(String, String, PathBuf)>) -> Result<()> {
    for (enum_name, new_code, file_path) in updates {
        // Validate syntax first
        validate_syntax(&new_code)?;

        // Each injection is atomic
        inject_enum(&enum_name, &new_code, &file_path)?;

        println!("Updated {} in {:?}", enum_name, file_path);
    }
    Ok(())
}
```

## Testing

The module includes comprehensive tests:

```rust
#[test]
fn test_inject_enum_basic() {
    let original = "pub enum Foo { A, B }";
    let new_enum = "pub enum Foo { X, Y, Z }";
    // Test injection logic...
}

#[test]
fn test_atomic_file_operations() {
    // Verify tempfile + rename behavior
}
```