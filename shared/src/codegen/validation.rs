//! Rust syntax validation using the `syn` crate.

use super::CodegenError;
use syn::parse_file;
use tracing::{debug, instrument};

/// Validates that the provided content is syntactically valid Rust code.
///
/// This uses the `syn` crate's full parser to ensure the content can be
/// parsed as a valid Rust file.
///
/// # Arguments
///
/// * `content` - The Rust source code to validate
///
/// # Returns
///
/// * `Ok(())` if the content is valid Rust
/// * `Err(CodegenError::SyntaxError)` if parsing fails
///
/// # Examples
///
/// ```rust
/// use shared::codegen::validate_syntax;
///
/// // Valid Rust code
/// let valid = "fn main() {}";
/// assert!(validate_syntax(valid).is_ok());
///
/// // Invalid Rust code
/// let invalid = "fn main() {";
/// assert!(validate_syntax(invalid).is_err());
/// ```
#[instrument(skip(content), fields(content_len = content.len()))]
pub fn validate_syntax(content: &str) -> Result<(), CodegenError> {
    debug!("Validating Rust syntax");

    parse_file(content).map(|_| ()).map_err(|e| {
        debug!("Syntax validation failed: {}", e);
        CodegenError::SyntaxError {
            message: e.to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_syntax_accepts_valid_rust() {
        let valid_code = r#"
            fn main() {
                println!("Hello, world!");
            }
        "#;

        assert!(validate_syntax(valid_code).is_ok());
    }

    #[test]
    fn validate_syntax_accepts_empty_file() {
        assert!(validate_syntax("").is_ok());
    }

    #[test]
    fn validate_syntax_accepts_complex_enum() {
        let complex_enum = r#"
            /// Documentation with } braces
            #[derive(Debug, Clone)]
            pub enum ComplexEnum {
                /// Variant with nested struct
                Nested { field: String, count: u32 },
                /// Variant with tuple
                Tuple(i32, i32),
                /// Simple variant
                Simple,
            }
        "#;

        assert!(validate_syntax(complex_enum).is_ok());
    }

    #[test]
    fn validate_syntax_rejects_invalid_rust() {
        let invalid_code = "fn main() {";

        let result = validate_syntax(invalid_code);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodegenError::SyntaxError { .. })));
    }

    #[test]
    fn validate_syntax_rejects_malformed_enum() {
        let malformed = r#"
            pub enum Bad {
                Variant,
                // Missing closing brace
        "#;

        assert!(validate_syntax(malformed).is_err());
    }

    #[test]
    fn validate_syntax_accepts_module_with_multiple_items() {
        let module = r#"
            use std::collections::HashMap;

            pub struct Data {
                map: HashMap<String, i32>,
            }

            pub enum Status {
                Active,
                Inactive,
            }

            impl Data {
                pub fn new() -> Self {
                    Self {
                        map: HashMap::new(),
                    }
                }
            }
        "#;

        assert!(validate_syntax(module).is_ok());
    }

    #[test]
    fn validate_syntax_error_message_is_descriptive() {
        let invalid = "fn broken() { let x = ;";

        match validate_syntax(invalid) {
            Err(CodegenError::SyntaxError { message }) => {
                // syn error messages should not be empty
                assert!(!message.is_empty());
                // Basic check that it contains some error indication
                assert!(message.len() > 5);
            }
            _ => panic!("Expected SyntaxError"),
        }
    }
}
