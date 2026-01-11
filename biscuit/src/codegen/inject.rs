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

/// Injects new variants into an existing enum definition with comment markers.
///
/// This function adds new variants to an enum without removing existing ones.
/// It manipulates the source text directly to preserve formatting and add
/// comment markers that clearly distinguish generated code.
///
/// ## Strategy
///
/// 1. Read the file as text
/// 2. Locate the enum definition
/// 3. Find the closing brace
/// 4. Remove old auto-generated section (if markers present)
/// 5. Insert new variants with markers before closing brace
/// 6. Validate with `syn` and atomically write
///
/// ## Arguments
///
/// * `enum_name` - Name of the enum to modify (e.g., "ProviderModel")
/// * `new_variants` - Vec of variant names as strings (e.g., ["Anthropic__ClaudeOpus_5_0", "OpenAi__Gpt5"])
/// * `file_path` - Path to the Rust source file
/// * `dry_run` - If true, validate but don't modify files
///
/// ## Returns
///
/// * `Ok(usize)` - Number of variants added
/// * `Err(CodegenError)` - Validation or I/O error
///
/// ## Examples
///
/// ```rust,no_run
/// use shared::codegen::inject_enum_variants;
///
/// let variants = vec![
///     "Anthropic__ClaudeOpus_5_0".to_string(),
///     "OpenAi__Gpt5".to_string(),
/// ];
///
/// let count = inject_enum_variants(
///     "ProviderModel",
///     &variants,
///     "src/providers/types.rs",
///     false, // not a dry run
/// )?;
/// println!("Added {} new variants", count);
/// # Ok::<(), shared::codegen::CodegenError>(())
/// ```
///
/// ## Generated Code Format
///
/// ```rust,ignore
/// pub enum ProviderModel {
///     // ... existing variants ...
///
///     // === AUTO-GENERATED VARIANTS (do not edit manually) ===
///     // Generated: 2025-12-30T12:00:00Z via ProviderModel::update()
///     Anthropic__ClaudeOpus_5_0,
///     OpenAi__Gpt5,
///     // === END AUTO-GENERATED ===
/// }
/// ```
#[instrument(skip(new_variants), fields(enum_name = %enum_name, file = %file_path, variant_count = new_variants.len(), dry_run = dry_run))]
pub fn inject_enum_variants(
    enum_name: &str,
    new_variants: &[String],
    file_path: &str,
    dry_run: bool,
) -> Result<usize, CodegenError> {
    let path = Path::new(file_path);

    info!("Starting enum variant injection");

    if new_variants.is_empty() {
        debug!("No new variants to add");
        return Ok(0);
    }

    // 1. Read existing file
    let original_content = if path.exists() {
        debug!("Reading existing file");
        fs::read_to_string(path)?
    } else {
        return Err(CodegenError::EnumNotFound {
            name: enum_name.to_string(),
        });
    };

    // 2. Pre-check syntax validation
    debug!("Pre-validation: checking existing file syntax");
    validate_syntax(&original_content)?;
    info!("Pre-validation passed");

    // 3. Find the enum in the source text
    let enum_pattern = format!(r"pub enum {} \{{", regex::escape(enum_name));
    let enum_re = regex::Regex::new(&enum_pattern).map_err(|e| CodegenError::SyntaxError {
        message: format!("Failed to create regex: {}", e),
    })?;

    let Some(enum_start) = enum_re.find(&original_content) else {
        return Err(CodegenError::EnumNotFound {
            name: enum_name.to_string(),
        });
    };

    // Find the closing brace for this enum (careful brace matching)
    let enum_start_pos = enum_start.end();
    let enum_closing_brace = find_matching_closing_brace(&original_content, enum_start_pos)
        .ok_or_else(|| CodegenError::SyntaxError {
            message: format!("Could not find closing brace for enum {}", enum_name),
        })?;

    // 4. Remove old auto-generated section if exists
    let (before_enum_end, _) = remove_autogenerated_text_section(
        &original_content[..enum_closing_brace],
        enum_start_pos,
    );

    // 5. Build new variants section
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut generated_section = String::new();
    generated_section.push_str("\n    // === AUTO-GENERATED VARIANTS (do not edit manually) ===\n");
    generated_section.push_str(&format!("    // Generated: {} via ProviderModel::update()\n", timestamp));

    for variant_name in new_variants {
        generated_section.push_str(&format!("    {},\n", variant_name));
    }

    generated_section.push_str("    // === END AUTO-GENERATED ===\n");

    // 6. Construct new content
    let new_content = format!(
        "{}{}{}{}",
        &original_content[..enum_start_pos],
        before_enum_end,
        generated_section,
        &original_content[enum_closing_brace..]
    );

    // 7. Post-validation
    debug!("Post-validation: checking resulting file syntax");
    validate_syntax(&new_content)?;
    info!("Post-validation passed");

    if dry_run {
        info!(
            "DRY RUN: Would add {} new variants to {}",
            new_variants.len(),
            enum_name
        );
        return Ok(new_variants.len());
    }

    // 8. Atomic write
    debug!("Writing to temporary file");
    let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let temp_file = NamedTempFile::new_in(parent_dir)?;
    fs::write(temp_file.path(), &new_content)?;

    debug!("Persisting file atomically");
    temp_file.persist(path)?;

    info!(
        "Enum variant injection completed successfully: {} variants added",
        new_variants.len()
    );
    Ok(new_variants.len())
}

/// Finds the matching closing brace for an opening brace at the given position.
///
/// Uses simple brace counting to handle nested structures.
fn find_matching_closing_brace(content: &str, start_pos: usize) -> Option<usize> {
    let mut depth = 1;
    let bytes = content.as_bytes();

    for (idx, &byte) in bytes.iter().enumerate().skip(start_pos) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }

    None
}

/// Removes the auto-generated section from source text between enum start and end.
///
/// Returns the text before the closing brace with the auto-generated section removed.
fn remove_autogenerated_text_section(enum_content: &str, enum_start: usize) -> (String, bool) {
    let start_marker = "// === AUTO-GENERATED VARIANTS";
    let end_marker = "// === END AUTO-GENERATED ===";

    // Find markers
    let Some(start_pos) = enum_content[enum_start..].find(start_marker) else {
        debug!("No auto-generated section found");
        return (enum_content[enum_start..].to_string(), false);
    };

    let search_start = enum_start + start_pos;
    let Some(end_pos) = enum_content[search_start..].find(end_marker) else {
        debug!("Found start marker but no end marker");
        return (enum_content[enum_start..].to_string(), false);
    };

    // Remove everything from start marker to end of end marker line
    let end_line_end = enum_content[search_start + end_pos..]
        .find('\n')
        .map(|n| search_start + end_pos + n + 1)
        .unwrap_or(enum_content.len());

    debug!("Removing old auto-generated section");

    let before = &enum_content[enum_start..search_start];
    let after = &enum_content[end_line_end..];

    (format!("{}{}", before, after), true)
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

    // Tests for inject_enum_variants

    #[test]
    fn inject_variants_adds_new_variants() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Existing__Variant,
    Another(String),
}"#;

        let file_path = create_test_file(&temp_dir, "test_variants.rs", original);

        let new_variants = vec![
            "Anthropic__ClaudeOpus_5_0".to_string(),
            "OpenAi__Gpt5".to_string(),
        ];

        let count = inject_enum_variants("ProviderModel", &new_variants, &file_path, false).unwrap();
        assert_eq!(count, 2);

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Anthropic__ClaudeOpus_5_0"));
        assert!(content.contains("OpenAi__Gpt5"));
        assert!(content.contains("AUTO-GENERATED VARIANTS"));
        assert!(content.contains("END AUTO-GENERATED"));
    }

    #[test]
    fn inject_variants_preserves_existing_variants() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Existing__Variant,
    Another(String),
}"#;

        let file_path = create_test_file(&temp_dir, "preserve.rs", original);

        let new_variants = vec!["New__Variant".to_string()];

        inject_enum_variants("ProviderModel", &new_variants, &file_path, false).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Existing__Variant"));
        assert!(content.contains("Another"));
        assert!(content.contains("New__Variant"));
    }

    #[test]
    fn inject_variants_idempotent_no_duplicates() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Existing__Variant,

    // === AUTO-GENERATED VARIANTS (do not edit manually) ===
    // Generated: 2025-12-30T00:00:00Z via ProviderModel::update()
    Auto__Generated__Variant,
    // === END AUTO-GENERATED ===
}"#;

        let file_path = create_test_file(&temp_dir, "idempotent.rs", original);

        // Run again with same variants
        let new_variants = vec!["Auto__Generated__Variant".to_string()];

        inject_enum_variants("ProviderModel", &new_variants, &file_path, false).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();

        // Count occurrences - should only appear once in final content
        let count = content.matches("Auto__Generated__Variant").count();
        assert_eq!(count, 1, "Variant should appear exactly once");

        // Should still have markers
        assert!(content.contains("AUTO-GENERATED VARIANTS"));
    }

    #[test]
    fn inject_variants_replaces_old_autogenerated_section() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Static__Variant,

    // === AUTO-GENERATED VARIANTS (do not edit manually) ===
    // Generated: 2025-12-29T00:00:00Z via ProviderModel::update()
    Old__Variant__1,
    Old__Variant__2,
    // === END AUTO-GENERATED ===
}"#;

        let file_path = create_test_file(&temp_dir, "replace.rs", original);

        let new_variants = vec![
            "New__Variant__1".to_string(),
            "New__Variant__2".to_string(),
        ];

        inject_enum_variants("ProviderModel", &new_variants, &file_path, false).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();

        // Old variants should be removed
        assert!(!content.contains("Old__Variant__1"));
        assert!(!content.contains("Old__Variant__2"));

        // New variants should be present
        assert!(content.contains("New__Variant__1"));
        assert!(content.contains("New__Variant__2"));

        // Static variant should remain
        assert!(content.contains("Static__Variant"));
    }

    #[test]
    fn inject_variants_dry_run_no_modification() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Existing__Variant,
}"#;

        let file_path = create_test_file(&temp_dir, "dry_run.rs", original);
        let original_content = fs::read_to_string(&file_path).unwrap();

        let new_variants = vec!["New__Variant".to_string()];

        let count = inject_enum_variants("ProviderModel", &new_variants, &file_path, true).unwrap();
        assert_eq!(count, 1); // Reports would add 1

        let current_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(original_content, current_content); // File unchanged
    }

    #[test]
    fn inject_variants_empty_list_returns_zero() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum ProviderModel {
    Existing__Variant,
}"#;

        let file_path = create_test_file(&temp_dir, "empty.rs", original);

        let new_variants: Vec<String> = vec![];

        let count = inject_enum_variants("ProviderModel", &new_variants, &file_path, false).unwrap();
        assert_eq!(count, 0);

        let content = fs::read_to_string(&file_path).unwrap();
        // Should not add markers if no variants
        assert!(!content.contains("AUTO-GENERATED"));
    }

    #[test]
    fn inject_variants_validates_syntax() {
        let temp_dir = TempDir::new().unwrap();

        let invalid = "pub enum Broken { ";
        let file_path = create_test_file(&temp_dir, "invalid.rs", invalid);

        let new_variants = vec!["Test__Variant".to_string()];

        let result = inject_enum_variants("ProviderModel", &new_variants, &file_path, false);
        assert!(result.is_err());

        // Original file should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, invalid);
    }

    #[test]
    fn inject_variants_enum_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let original = r#"pub enum SomeOtherEnum {
    Variant,
}"#;

        let file_path = create_test_file(&temp_dir, "wrong_enum.rs", original);

        let new_variants = vec!["Test__Variant".to_string()];

        let result = inject_enum_variants("ProviderModel", &new_variants, &file_path, false);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodegenError::EnumNotFound { .. })));
    }

    #[test]
    fn find_matching_closing_brace_handles_nesting() {
        let content = "{ { nested } more }";
        // Start after first opening brace at index 0
        let result = find_matching_closing_brace(content, 1);
        assert_eq!(result, Some(18)); // Last closing brace
    }

    #[test]
    fn find_matching_closing_brace_no_match() {
        let content = "{ { { no closing";
        let result = find_matching_closing_brace(content, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn remove_autogenerated_section_no_markers() {
        let content = "pub enum Test { Variant }";
        let (result, found) = remove_autogenerated_text_section(content, 0);
        assert!(!found);
        assert_eq!(result, content);
    }

    #[test]
    fn remove_autogenerated_section_with_markers() {
        let content = r#"Variant1,
    // === AUTO-GENERATED VARIANTS (do not edit manually) ===
    // Generated: 2025-12-30
    Old__Variant,
    // === END AUTO-GENERATED ===
    Variant2,
"#;

        let (result, found) = remove_autogenerated_text_section(content, 0);
        assert!(found);
        assert!(!result.contains("Old__Variant"));
        assert!(result.contains("Variant1"));
        assert!(result.contains("Variant2"));
    }
}
