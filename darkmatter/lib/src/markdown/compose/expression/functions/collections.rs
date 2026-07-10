use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_ANY, P_ANY2, P_LIST, P_NUM_CONV, P_OBJ_STRING, R_ANY_ERR, R_BOOL_ERR, R_NUM_ERR, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "first", aliases: &[], catalog_order: 12, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "first(x)",
                parameters: P_LIST,
                returns: R_ANY_ERR,
                description: "Returns the first element of an array, or null when empty.",
                category: "Collection",
                order: 1,

                example: Some(Example { invocation: "first(items)", result: "1", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::first_fn) },
    FunctionRegistration { canonical: "last", aliases: &[], catalog_order: 13, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "last(x)",
                parameters: P_LIST,
                returns: R_ANY_ERR,
                description: "Returns the last element of an array, or null when empty.",
                category: "Collection",
                order: 2,

                example: Some(Example { invocation: "last(items)", result: "3", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::last_fn) },
    FunctionRegistration { canonical: "has_key", aliases: &["haskey"], catalog_order: 48, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "has_key(obj, key)",
                parameters: P_OBJ_STRING,
                returns: R_BOOL_ERR,
                description: "Returns true when the object contains the given key.",
                category: "Collection",
                order: 3,

                example: Some(Example { invocation: "has_key(obj, \"a\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::has_key_fn) },
    FunctionRegistration { canonical: "contains", aliases: &[], catalog_order: 49, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "contains(haystack, needle)",
                parameters: P_ANY2,
                returns: R_BOOL_ERR,
                description: "Returns true when haystack contains needle (array, object, or string).",
                category: "Collection",
                order: 4,

                example: Some(Example { invocation: "contains(\"hello\", \"ell\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::contains_fn) },
    FunctionRegistration { canonical: "length", aliases: &[], catalog_order: 50, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "length(x)",
                parameters: P_ANY,
                returns: R_NUM_ERR,
                description: "Returns the length of a string, array, or object.",
                category: "Collection",
                order: 5,

                example: Some(Example { invocation: "length(\"hello\")", result: "5", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::length_fn) },
    FunctionRegistration { canonical: "number", aliases: &[], catalog_order: 51, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "number(x, [default])",
                parameters: P_NUM_CONV,
                returns: R_NUM_ERR,
                description: "Converts a value to a number, with an optional default.",
                category: "Type Conversion",
                order: 1,

                example: Some(Example { invocation: "number(\"42\")", result: "42", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::number_fn) },
    FunctionRegistration { canonical: "as_line_separated", aliases: &["aslineseparated"], catalog_order: 79, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_line_separated(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Joins a list into a newline-separated string (the default bare-array rendering).",
                category: "List Formatting",
                order: 1,
                example: Some(Example { invocation: "as_line_separated(items)", result: "1\n2\n3", verification: ExampleVerification::DisplayOnly("multi-line output; verified via example file") }),
            },
    ], handler: FunctionHandler::Pure(super::as_line_separated) },
    FunctionRegistration { canonical: "as_csv", aliases: &["ascsv"], catalog_order: 80, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_csv(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Joins a list into a comma-separated string.",
                category: "List Formatting",
                order: 2,
                example: Some(Example { invocation: "as_csv(items)", result: "1, 2, 3", verification: ExampleVerification::Executable }),
            },
    ], handler: FunctionHandler::Pure(super::as_csv) },
    FunctionRegistration { canonical: "as_tsv", aliases: &["astsv"], catalog_order: 81, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_tsv(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Joins a list into a tab-separated string.",
                category: "List Formatting",
                order: 3,
                example: Some(Example { invocation: "as_tsv(items)", result: "1\t2\t3", verification: ExampleVerification::DisplayOnly("tab-delimited output; verified via example file") }),
            },
    ], handler: FunctionHandler::Pure(super::as_tsv) },
    FunctionRegistration { canonical: "as_space_separated", aliases: &["asspaceseparated"], catalog_order: 82, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_space_separated(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Joins a list into a space-separated string.",
                category: "List Formatting",
                order: 4,
                example: Some(Example { invocation: "as_space_separated(items)", result: "1 2 3", verification: ExampleVerification::Executable }),
            },
    ], handler: FunctionHandler::Pure(super::as_space_separated) },
    FunctionRegistration { canonical: "as_unordered_list", aliases: &["asunorderedlist"], catalog_order: 83, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_unordered_list(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
                category: "List Formatting",
                order: 5,
                example: Some(Example { invocation: "as_unordered_list(items)", result: "- 1\n- 2\n- 3", verification: ExampleVerification::DisplayOnly("multi-line Markdown list; verified via example file") }),
            },
    ], handler: FunctionHandler::Pure(super::as_unordered_list) },
    FunctionRegistration { canonical: "as_ordered_list", aliases: &["asorderedlist"], catalog_order: 84, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "as_ordered_list(list)",
                parameters: P_LIST,
                returns: R_STRING_ERR,
                description: "Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
                category: "List Formatting",
                order: 6,
                example: Some(Example { invocation: "as_ordered_list(items)", result: "1. 1\n2. 2\n3. 3", verification: ExampleVerification::DisplayOnly("multi-line Markdown list; verified via example file") }),
            },
    ], handler: FunctionHandler::Pure(super::as_ordered_list) },
];
