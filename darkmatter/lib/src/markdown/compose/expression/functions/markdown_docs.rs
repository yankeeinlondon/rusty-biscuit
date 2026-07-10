use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_FILE, P_FILE_OBJ, P_FILE_STRING, R_ANY_ERR, R_BOOL_ERR, R_OBJ_ERR, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "frontmatter", aliases: &[], catalog_order: 56, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "frontmatter(file)",
                parameters: P_FILE,
                returns: R_OBJ_ERR,
                description: "Reads the frontmatter of a Markdown file as an object.",
                category: "Filesystem",
                order: 4,
        
                example: Some(Example { invocation: "frontmatter(\"fixture.md\")", result: "{\"title\":\"Fixture Title\"}", verification: ExampleVerification::Executable }),
        
            },
        ExpressionFunctionDescriptor {
        
                signature: "frontmatter(file, prop)",
                parameters: P_FILE_STRING,
                returns: R_ANY_ERR,
                description: "Reads a single frontmatter property from a Markdown file.",
                category: "Filesystem",
                order: 5,
        
                example: Some(Example { invocation: "frontmatter(\"fixture.md\", \"title\")", result: "Fixture Title", verification: ExampleVerification::Executable }),
        
            },
    ], handler: FunctionHandler::Context(super::frontmatter_fn) },
    FunctionRegistration { canonical: "markdown_body_empty", aliases: &["markdownbodyempty"], catalog_order: 58, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "markdown_body_empty(file)",
                parameters: P_FILE,
                returns: R_BOOL_ERR,
                description: "Returns true when the Markdown body has only whitespace.",
                category: "Filesystem",
                order: 6,
        
                example: Some(Example { invocation: "markdown_body_empty(\"fixture.md\")", result: "false", verification: ExampleVerification::Executable }),
        
            },
    ], handler: FunctionHandler::Context(super::markdown_body_empty_fn) },
    FunctionRegistration { canonical: "markdown_title", aliases: &["markdowntitle"], catalog_order: 59, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "markdown_title(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the title from frontmatter or the first H1 heading.",
                category: "Filesystem",
                order: 7,
        
                example: Some(Example { invocation: "markdown_title(\"fixture.md\")", result: "Fixture Title", verification: ExampleVerification::Executable }),
        
            },
    ], handler: FunctionHandler::Context(super::markdown_title_fn) },
    FunctionRegistration { canonical: "validate_schema", aliases: &["validateschema"], catalog_order: 60, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "validate_schema(file)",
                parameters: P_FILE,
                returns: R_BOOL_ERR,
                description: "Validates a Markdown document against its declared schema.",
                category: "Filesystem",
                order: 8,
        
                example: Some(Example { invocation: "validate_schema(\"fixture.md\")", result: "true", verification: ExampleVerification::Executable }),
        
            },
        ExpressionFunctionDescriptor {
        
                signature: "validate_schema(file, obj)",
                parameters: P_FILE_OBJ,
                returns: R_BOOL_ERR,
                description: "Two-argument form accepted for forward compatibility.",
                category: "Filesystem",
                order: 9,
        
                example: Some(Example { invocation: "validate_schema(\"fixture.md\", {})", result: "true", verification: ExampleVerification::DisplayOnly("forward-compatible overload with no evaluable behavior yet") }),
        
            },
    ], handler: FunctionHandler::Context(super::validate_schema_fn) },
];
