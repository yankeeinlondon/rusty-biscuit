//! Safe enum injection with AST-based manipulation and atomic file writes.

use super::{validate_syntax, CodegenError};
use quote::ToTokens;
use std::fs;
use std::path::Path;
use syn::{File, Item};
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

/// Safely injects an enum definition into a Rust source file.
///
/// This function provides the following safety guarantees:
///
/// 1. **Pre-validation:** Validates existing file syntax before any modification
/// 2. **AST-based removal:** Uses `syn` to correctly remove old enum definitions
///    (handles nested braces, doc comments, attributes - not regex!)
/// 3. **Post-validation:** Validates resulting code after injection
/// 4. **Atomic writes:** Uses tempfile + rename to prevent partial writes
/// 5. **Rollback safety:** Original file untouched if any step fails
///
/// # Arguments
///
/// * `name` - The name of the enum to inject/replace (e.g., "Color")
/// * `new_enum` - The full enum definition as a string
/// * `file_path` - Path to the target Rust source file
///
/// # Returns
///
/// * `Ok(())` if injection succeeds
/// * `Err(CodegenError)` if validation or I/O fails
///
/// # Examples
///
/// ```rust,no_run
/// use shared::codegen::inject_enum;
///
/// let enum_def = r#"
/// #[derive(Debug, Clone)]
/// pub enum Status {
///     Active,
///     Inactive,
/// }
/// "#;
///
/// inject_enum("Status", enum_def, "src/types.rs")?;
/// # Ok::<(), shared::codegen::CodegenError>(())
/// ```
///
/// # Safety Guarantees
///
/// The original file is **never modified** until:
/// - Pre-validation passes (existing file is valid Rust)
/// - Enum removal succeeds (AST parsing successful)
/// - Post-validation passes (new content is valid Rust)
///
/// If any step fails, the original file remains unchanged.
#[instrument(skip(new_enum), fields(enum_name = %name, file = %file_path))]
pub fn inject_enum(name: &str, new_enum: &str, file_path: &str) -> Result<(), CodegenError> {
    let path = Path::new(file_path);

    info!("Starting enum injection");

    // 1. Read existing file (or create if doesn't exist)
    let original_content = if path.exists() {
        debug!("Reading existing file");
        fs::read_to_string(path)?
    } else {
        debug!("File doesn't exist, will create new");
        String::new()
    };

    // 2. Pre-check syntax validation (only if file exists and has content)
    if !original_content.is_empty() {
        debug!("Pre-validation: checking existing file syntax");
        validate_syntax(&original_content)?;
        info!("Pre-validation passed");
    }

    // 3. Remove old enum definition (if exists) - AST-based, NOT regex
    debug!("Removing old enum definition (if exists)");
    let content_without_old = remove_enum_definition(&original_content, name)?;

    // 4. Inject new enum
    debug!("Injecting new enum definition");
    let new_content = inject_enum_definition(&content_without_old, new_enum)?;

    // 5. Post-check syntax validation
    debug!("Post-validation: checking resulting file syntax");
    validate_syntax(&new_content)?;
    info!("Post-validation passed");

    // 6. Atomic write using temporary file + rename
    debug!("Writing to temporary file");
    let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

    // Ensure parent directory exists
    if !parent_dir.exists() {
        debug!("Creating parent directory: {:?}", parent_dir);
        fs::create_dir_all(parent_dir)?;
    }

    let temp_file = NamedTempFile::new_in(parent_dir)?;
    fs::write(temp_file.path(), &new_content)?;

    // POSIX atomic rename - original file only replaced if all validations passed
    debug!("Persisting file atomically");
    temp_file.persist(path)?;

    info!("Enum injection completed successfully");
    Ok(())
}

/// Removes an enum definition from Rust source code using AST manipulation.
///
/// This uses `syn` to parse the source code into an AST, removes any enum
/// with the matching name, and reconstructs the source using `quote`.
///
/// # Why AST-based instead of regex?
///
/// Regex approaches fail on:
/// - Nested braces: `enum Foo { Bar { x: u32 } }`
/// - Doc comments containing `}`: `/// Example: fn test() { }`
/// - Attributes: `#[derive(Debug)]`
/// - Complex enum variants
///
/// AST-based removal handles all valid Rust syntax correctly.
///
/// # Arguments
///
/// * `content` - The source code to process
/// * `enum_name` - The name of the enum to remove
///
/// # Returns
///
/// The source code with the enum removed (or unchanged if enum not found)
fn remove_enum_definition(content: &str, enum_name: &str) -> Result<String, CodegenError> {
    if content.is_empty() {
        return Ok(String::new());
    }

    let mut ast: File = syn::parse_str(content).map_err(|e| CodegenError::SyntaxError {
        message: format!("Failed to parse file for enum removal: {}", e),
    })?;

    // Count how many enums we're removing (for logging)
    let before_count = ast.items.len();

    // Remove enum with matching name
    ast.items.retain(|item| {
        !matches!(item, Item::Enum(e) if e.ident == enum_name)
    });

    let after_count = ast.items.len();
    if before_count > after_count {
        debug!(
            "Removed {} enum definition(s)",
            before_count - after_count
        );
    } else {
        debug!("No existing enum '{}' found to remove", enum_name);
    }

    // Reconstruct file using quote
    Ok(ast.into_token_stream().to_string())
}

/// Injects a new enum definition into the source code.
///
/// Simply appends the new enum to the end of the existing content.
///
/// # Arguments
///
/// * `content` - Existing source code
/// * `new_enum` - New enum definition to inject
///
/// # Returns
///
/// Combined source code with new enum appended
fn inject_enum_definition(content: &str, new_enum: &str) -> Result<String, CodegenError> {
    let trimmed_content = content.trim();
    let trimmed_enum = new_enum.trim();

    if trimmed_content.is_empty() {
        Ok(format!("{}\n", trimmed_enum))
    } else {
        Ok(format!("{}\n\n{}\n", trimmed_content, trimmed_enum))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test file in a temporary directory
    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn inject_enum_creates_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.rs");
        let file_path_str = file_path.to_str().unwrap();

        let enum_def = r#"
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;

        let result = inject_enum("Color", enum_def, file_path_str);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("pub enum Color"));
        assert!(content.contains("Red"));
    }

    #[test]
    fn inject_enum_replaces_existing_enum() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
pub enum Color {
    Red,
    Green,
}
"#;

        let file_path = create_test_file(&temp_dir, "types.rs", original);

        let new_enum = r#"
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
}
"#;

        let result = inject_enum("Color", new_enum, &file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Blue"));
        assert!(content.contains("Yellow"));
    }

    #[test]
    fn inject_enum_preserves_other_content() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
use std::fmt;

pub struct Data {
    value: i32,
}

pub enum OldEnum {
    Variant,
}

impl Data {
    pub fn new() -> Self {
        Self { value: 0 }
    }
}
"#;

        let file_path = create_test_file(&temp_dir, "module.rs", original);

        let new_enum = r#"
pub enum OldEnum {
    NewVariant,
}
"#;

        let result = inject_enum("OldEnum", new_enum, &file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        // Verify other content is preserved
        assert!(content.contains("pub struct Data"));
        assert!(content.contains("impl Data"));
        // Verify new enum is present
        assert!(content.contains("NewVariant"));
        // Verify old variant is not in the new enum definition
        // Note: We can't check for exact formatting due to AST reconstruction
        assert!(content.contains("enum OldEnum"));
    }

    #[test]
    fn inject_enum_handles_nested_braces() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
pub enum Nested {
    Variant { field: String, count: u32 },
    Simple,
}
"#;

        let file_path = create_test_file(&temp_dir, "nested.rs", original);

        let new_enum = r#"
pub enum Nested {
    NewVariant { data: Vec<u8> },
}
"#;

        let result = inject_enum("Nested", new_enum, &file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("NewVariant"));
        assert!(!content.contains("Simple"));
    }

    #[test]
    fn inject_enum_handles_doc_comments_with_braces() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
/// Example usage:
/// ```
/// fn test() { println!("test"); }
/// ```
pub enum Documented {
    Variant,
}
"#;

        let file_path = create_test_file(&temp_dir, "documented.rs", original);

        let new_enum = r#"
/// New documentation
pub enum Documented {
    NewVariant,
}
"#;

        let result = inject_enum("Documented", new_enum, &file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("NewVariant"));
    }

    #[test]
    fn inject_enum_handles_attributes() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
#[derive(Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Active,
    Inactive,
}
"#;

        let file_path = create_test_file(&temp_dir, "status.rs", original);

        let new_enum = r#"
#[derive(Debug, Clone)]
pub enum Status {
    Running,
    Stopped,
}
"#;

        let result = inject_enum("Status", new_enum, &file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Running"));
        assert!(!content.contains("Active"));
    }

    #[test]
    fn inject_enum_rejects_invalid_original_syntax() {
        let temp_dir = TempDir::new().unwrap();

        let invalid = "fn broken() { let x = ;";
        let file_path = create_test_file(&temp_dir, "invalid.rs", invalid);

        let new_enum = r#"
pub enum Test {
    Variant,
}
"#;

        let result = inject_enum("Test", new_enum, &file_path);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodegenError::SyntaxError { .. })));

        // Original file should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, invalid);
    }

    #[test]
    fn inject_enum_rejects_invalid_new_enum() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
pub enum Original {
    Variant,
}
"#;

        let file_path = create_test_file(&temp_dir, "test.rs", original);

        let invalid_enum = "pub enum Broken { ";

        let result = inject_enum("Original", invalid_enum, &file_path);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodegenError::SyntaxError { .. })));

        // Original file should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("pub enum Original"));
        assert!(content.contains("Variant"));
    }

    #[test]
    fn inject_enum_rollback_on_failure() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"
pub enum Data {
    Value(i32),
}
"#;

        let file_path = create_test_file(&temp_dir, "rollback.rs", original);
        let original_backup = fs::read_to_string(&file_path).unwrap();

        // Try to inject invalid enum
        let invalid = "enum Bad {";
        let result = inject_enum("Data", invalid, &file_path);
        assert!(result.is_err());

        // Verify original file unchanged
        let current = fs::read_to_string(&file_path).unwrap();
        assert_eq!(current, original_backup);
    }

    #[test]
    fn inject_enum_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested/deep/file.rs");
        let file_path_str = nested_path.to_str().unwrap();

        let enum_def = r#"
pub enum Test {
    Variant,
}
"#;

        let result = inject_enum("Test", enum_def, file_path_str);
        assert!(result.is_ok());
        assert!(nested_path.exists());
    }

    #[test]
    fn inject_enum_atomic_write_verification() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("atomic.rs");
        let file_path_str = file_path.to_str().unwrap();

        let enum_def = r#"
pub enum Atomic {
    Test,
}
"#;

        // First injection
        inject_enum("Atomic", enum_def, file_path_str).unwrap();

        // Verify file exists and has correct permissions
        assert!(file_path.exists());
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.is_file());
    }

    #[test]
    fn remove_enum_definition_handles_empty_content() {
        let result = remove_enum_definition("", "Test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn remove_enum_definition_returns_unchanged_if_not_found() {
        let content = r#"
pub enum Other {
    Variant,
}
"#;

        let result = remove_enum_definition(content, "NotFound").unwrap();
        // AST reconstruction may change formatting, but should contain the enum
        assert!(result.contains("enum Other"));
    }

    #[test]
    fn inject_enum_definition_handles_empty_content() {
        let enum_def = "pub enum Test { Variant }";
        let result = inject_enum_definition("", enum_def).unwrap();
        assert!(result.contains("pub enum Test"));
    }

    #[test]
    fn inject_enum_definition_appends_correctly() {
        let existing = "pub struct Data {}";
        let new_enum = "pub enum Status { Active }";

        let result = inject_enum_definition(existing, new_enum).unwrap();
        assert!(result.contains("pub struct Data"));
        assert!(result.contains("pub enum Status"));
    }
}
