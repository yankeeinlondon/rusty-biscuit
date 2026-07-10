use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_ANY2, P_STRING, P_STRING2, P_STRING3, R_BOOL_ERR, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "starts_with", aliases: &["startswith"], catalog_order: 14, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "starts_with(x, find)",
                parameters: P_STRING2,
                returns: R_BOOL_ERR,
                description: "Returns true when the string starts with the given prefix (case-sensitive).",
                category: "String Predicates",
                order: 1,

                example: Some(Example { invocation: "starts_with(\"hello\", \"he\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::starts_with) },
    FunctionRegistration { canonical: "ends_with", aliases: &["endswith"], catalog_order: 15, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "ends_with(x, find)",
                parameters: P_STRING2,
                returns: R_BOOL_ERR,
                description: "Returns true when the string ends with the given suffix (case-sensitive).",
                category: "String Predicates",
                order: 2,

                example: Some(Example { invocation: "ends_with(\"hello\", \"lo\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::ends_with) },
    FunctionRegistration { canonical: "lower", aliases: &[], catalog_order: 16, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "lower(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to lowercase.",
                category: "String Mutations",
                order: 1,

                example: Some(Example { invocation: "lower(\"HELLO\")", result: "hello", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::lower) },
    FunctionRegistration { canonical: "upper", aliases: &[], catalog_order: 17, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "upper(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to uppercase.",
                category: "String Mutations",
                order: 2,

                example: Some(Example { invocation: "upper(\"hello\")", result: "HELLO", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::upper) },
    FunctionRegistration { canonical: "capitalize", aliases: &[], catalog_order: 18, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "capitalize(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Capitalizes the first character of a string.",
                category: "String Mutations",
                order: 3,

                example: Some(Example { invocation: "capitalize(\"hello\")", result: "Hello", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::capitalize) },
    FunctionRegistration { canonical: "kebab_case", aliases: &["kebabcase"], catalog_order: 19, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "kebab_case(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to kebab-case.",
                category: "String Mutations",
                order: 4,

                example: Some(Example { invocation: "kebab_case(\"Hello World\")", result: "hello-world", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::kebab_case) },
    FunctionRegistration { canonical: "snake_case", aliases: &["snakecase"], catalog_order: 20, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "snake_case(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to snake_case.",
                category: "String Mutations",
                order: 5,

                example: Some(Example { invocation: "snake_case(\"Hello World\")", result: "hello_world", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::snake_case) },
    FunctionRegistration { canonical: "camel_case", aliases: &["camelcase"], catalog_order: 21, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "camel_case(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to camelCase.",
                category: "String Mutations",
                order: 6,

                example: Some(Example { invocation: "camel_case(\"hello world\")", result: "helloWorld", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::camel_case) },
    FunctionRegistration { canonical: "pascal_case", aliases: &["pascalcase"], catalog_order: 22, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "pascal_case(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to PascalCase.",
                category: "String Mutations",
                order: 7,

                example: Some(Example { invocation: "pascal_case(\"hello world\")", result: "HelloWorld", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::pascal_case) },
    FunctionRegistration { canonical: "title_case", aliases: &["titlecase"], catalog_order: 23, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "title_case(x)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Converts a string to Title Case.",
                category: "String Mutations",
                order: 8,

                example: Some(Example { invocation: "title_case(\"hello world\")", result: "Hello World", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::title_case) },
    FunctionRegistration { canonical: "without_date", aliases: &["withoutdate"], catalog_order: 24, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "without_date(string)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.",
                category: "String Mutations",
                order: 9,

                example: Some(Example { invocation: "without_date(\"Note 2024-06-15\")", result: "Note ", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::without_date) },
    FunctionRegistration { canonical: "ensure_leading", aliases: &["ensureleading"], catalog_order: 25, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "ensure_leading(var, prefix)",
                parameters: P_ANY2,
                returns: R_STRING_ERR,
                description: "Ensures the string form of a value starts with a prefix.",
                category: "String Mutations",
                order: 10,

                example: Some(Example { invocation: "ensure_leading(\"world\", \"hello \")", result: "hello world", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::ensure_leading) },
    FunctionRegistration { canonical: "ensure_trailing", aliases: &["ensuretrailing"], catalog_order: 26, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "ensure_trailing(var, postfix)",
                parameters: P_ANY2,
                returns: R_STRING_ERR,
                description: "Ensures the string form of a value ends with a postfix.",
                category: "String Mutations",
                order: 11,

                example: Some(Example { invocation: "ensure_trailing(\"hello\", \" world\")", result: "hello world", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::ensure_trailing) },
    FunctionRegistration { canonical: "replace", aliases: &[], catalog_order: 27, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "replace(x, find, replacement)",
                parameters: P_STRING3,
                returns: R_STRING_ERR,
                description: "Replaces every literal occurrence of a substring; empty find is a no-op.",
                category: "String Mutations",
                order: 12,

                example: Some(Example { invocation: "replace(\"a.b.c\", \".\", \"/\")", result: "a/b/c", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::replace) },
    FunctionRegistration { canonical: "replace_first", aliases: &["replacefirst"], catalog_order: 28, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "replace_first(x, find, replacement)",
                parameters: P_STRING3,
                returns: R_STRING_ERR,
                description: "Replaces the first literal occurrence of a substring; empty find is a no-op.",
                category: "String Mutations",
                order: 13,

                example: Some(Example { invocation: "replace_first(\"a.b.c\", \".\", \"/\")", result: "a/b.c", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::replace_first) },
    FunctionRegistration { canonical: "replace_last", aliases: &["replacelast"], catalog_order: 29, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "replace_last(x, find, replacement)",
                parameters: P_STRING3,
                returns: R_STRING_ERR,
                description: "Replaces the last literal occurrence of a substring; empty find is a no-op.",
                category: "String Mutations",
                order: 14,

                example: Some(Example { invocation: "replace_last(\"a.b.c\", \".\", \"/\")", result: "a.b/c", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::replace_last) },
];
