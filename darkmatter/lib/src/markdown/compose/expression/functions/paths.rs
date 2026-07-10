use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_FILE, P_FILE_STRING, P_STRING, P_STRING2, R_BOOL, R_BOOL_ERR, R_FILE_ERR, R_NUM_ERR, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "absolute", aliases: &[], catalog_order: 53, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "absolute(file)",
                parameters: P_FILE,
                returns: R_FILE_ERR,
                description: "Resolves a file path to an absolute path.",
                category: "Filesystem",
                order: 1,

                example: Some(Example { invocation: "absolute(\"fixture.md\")", result: "/path/to/fixture.md", verification: ExampleVerification::DisplayOnly("resolves to an absolute path of the resolution context, which is not portable") }),

            },
    ], handler: FunctionHandler::Context(super::absolute_fn) },
    FunctionRegistration { canonical: "relative", aliases: &[], catalog_order: 54, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "relative(file)",
                parameters: P_FILE,
                returns: R_FILE_ERR,
                description: "Returns a best-effort relative path from the document base directory.",
                category: "Filesystem",
                order: 2,

                example: Some(Example { invocation: "relative(\"fixture.md\")", result: "fixture.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::relative_fn) },
    FunctionRegistration { canonical: "file_exists", aliases: &["fileexists"], catalog_order: 55, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "file_exists(file)",
                parameters: P_FILE,
                returns: R_BOOL_ERR,
                description: "Returns true when the file exists (local or remote URL).",
                category: "Filesystem",
                order: 3,

                example: Some(Example { invocation: "file_exists(\"fixture.md\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::file_exists_fn) },
    FunctionRegistration { canonical: "has_command", aliases: &["hascommand"], catalog_order: 76, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "has_command(cmd)",
                parameters: P_STRING,
                returns: R_BOOL,
                description: "Returns true when the command is found on PATH or is an existing executable absolute path.",
                category: "Filesystem",
                order: 24,

                example: None,

            },
    ], handler: FunctionHandler::Context(super::has_command_fn) },
    FunctionRegistration { canonical: "is_indexed_file", aliases: &["isindexedfile"], catalog_order: 62, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_indexed_file(file)",
                parameters: P_FILE,
                returns: R_BOOL_ERR,
                description: "Returns true when the filename stem matches the indexed grammar (base-NNN).",
                category: "Filesystem",
                order: 10,

                example: Some(Example { invocation: "is_indexed_file(\"review-1.md\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::is_indexed_file_fn) },
    FunctionRegistration { canonical: "file_index", aliases: &["fileindex"], catalog_order: 63, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "file_index(file)",
                parameters: P_FILE,
                returns: R_NUM_ERR,
                description: "Returns the parsed index suffix, or -1 when non-indexed.",
                category: "Filesystem",
                order: 11,

                example: Some(Example { invocation: "file_index(\"review-1.md\")", result: "1", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::file_index_fn) },
    FunctionRegistration { canonical: "increment_file_index", aliases: &["incrementfileindex"], catalog_order: 64, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "increment_file_index(file)",
                parameters: P_FILE,
                returns: R_FILE_ERR,
                description: "Increments the numeric index suffix, preserving zero-padding width.",
                category: "Filesystem",
                order: 12,

                example: Some(Example { invocation: "increment_file_index(\"review-1.md\")", result: "review-2.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::increment_file_index_fn) },
    FunctionRegistration { canonical: "decrement_file_index", aliases: &["decrementfileindex"], catalog_order: 65, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "decrement_file_index(file)",
                parameters: P_FILE,
                returns: R_FILE_ERR,
                description: "Decrements the numeric index suffix, clamped at 0.",
                category: "Filesystem",
                order: 13,

                example: Some(Example { invocation: "decrement_file_index(\"review-2.md\")", result: "review-1.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::decrement_file_index_fn) },
    FunctionRegistration { canonical: "basename", aliases: &[], catalog_order: 66, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "basename(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the final path component including extension.",
                category: "Filesystem",
                order: 14,

                example: Some(Example { invocation: "basename(\"sub/note.md\")", result: "note.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::basename_fn) },
    FunctionRegistration { canonical: "basename_without_index", aliases: &["basenamewithoutindex"], catalog_order: 67, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "basename_without_index(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the basename with any indexed suffix removed from the stem.",
                category: "Filesystem",
                order: 15,

                example: Some(Example { invocation: "basename_without_index(\"review-1.md\")", result: "review.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::basename_without_index_fn) },
    FunctionRegistration { canonical: "dirname", aliases: &[], catalog_order: 68, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "dirname(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the directory portion of the display path.",
                category: "Filesystem",
                order: 16,

                example: Some(Example { invocation: "dirname(\"sub/note.md\")", result: "sub", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::dirname_fn) },
    FunctionRegistration { canonical: "ext", aliases: &[], catalog_order: 69, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "ext(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the final extension without the leading dot.",
                category: "Filesystem",
                order: 17,

                example: Some(Example { invocation: "ext(\"sub/note.md\")", result: "md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::ext_fn) },
    FunctionRegistration { canonical: "parent_dir", aliases: &["parentdir"], catalog_order: 70, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "parent_dir(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the directory segment immediately above the basename.",
                category: "Filesystem",
                order: 18,

                example: Some(Example { invocation: "parent_dir(\"sub/note.md\")", result: "sub", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::parent_dir_fn) },
    FunctionRegistration { canonical: "file_trailing", aliases: &["filetrailing"], catalog_order: 71, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "file_trailing(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the last directory segment plus the basename.",
                category: "Filesystem",
                order: 19,

                example: Some(Example { invocation: "file_trailing(\"sub/note.md\")", result: "sub/note.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::file_trailing_fn) },
    FunctionRegistration { canonical: "dir_leading", aliases: &["dirleading"], catalog_order: 72, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "dir_leading(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing).",
                category: "Filesystem",
                order: 20,

                example: Some(Example { invocation: "dir_leading(\"sub/note.md\")", result: "", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::dir_leading_fn) },
    FunctionRegistration { canonical: "join", aliases: &[], catalog_order: 73, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "join(left, right)",
                parameters: P_STRING2,
                returns: R_STRING_ERR,
                description: "Joins two path strings with normalized separators.",
                category: "Filesystem",
                order: 21,

                example: Some(Example { invocation: "join(\"sub\", \"note.md\")", result: "sub/note.md", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Context(super::join_fn) },
    FunctionRegistration { canonical: "link", aliases: &[], catalog_order: 74, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "link(file)",
                parameters: P_FILE,
                returns: R_STRING_ERR,
                description: "Creates a Markdown link to a local file, using its relative path as the link text.",
                category: "Filesystem",
                order: 22,

                example: Some(Example { invocation: "link(\"fixture.md\")", result: "[fixture.md](/path/to/fixture.md)", verification: ExampleVerification::DisplayOnly("result includes an absolute path, which is not portable") }),

            },
        ExpressionFunctionDescriptor {

                signature: "link(target, desc)",
                parameters: P_FILE_STRING,
                returns: R_STRING_ERR,
                description: "Creates a Markdown link to a local file or HTTP(S) URL with the given description.",
                category: "Filesystem",
                order: 23,

                example: Some(Example { invocation: "link(\"fixture.md\", \"Fixture\")", result: "[Fixture](/path/to/fixture.md)", verification: ExampleVerification::DisplayOnly("result includes an absolute destination path, which is not portable") }),

            },
    ], handler: FunctionHandler::Context(super::link_fn) },
];
