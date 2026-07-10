use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_ANY, P_NUM, P_NUM2, P_ROUND, R_BOOL, R_BOOL_ERR, R_NUM, R_NUM_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "is_string", aliases: &["isstring"], catalog_order: 0, descriptors: &[
        ExpressionFunctionDescriptor {
                signature: "is_string(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is a string.",
                category: "Type Predicates",
                order: 1,

                example: Some(Example { invocation: "is_string(\"hello\")", result: "true", verification: ExampleVerification::Executable }),
         },
    ], handler: FunctionHandler::Pure(super::is_string) },
    FunctionRegistration { canonical: "is_number", aliases: &["isnumber"], catalog_order: 1, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_number(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is a number.",
                category: "Type Predicates",
                order: 2,

                example: Some(Example { invocation: "is_number(42)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_number) },
    FunctionRegistration { canonical: "is_array", aliases: &["isarray"], catalog_order: 2, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_array(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is an array.",
                category: "Type Predicates",
                order: 3,

                example: Some(Example { invocation: "is_array(items)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_array) },
    FunctionRegistration { canonical: "is_null", aliases: &["isnull"], catalog_order: 3, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_null(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is null.",
                category: "Type Predicates",
                order: 4,

                example: Some(Example { invocation: "is_null(null)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_null) },
    FunctionRegistration { canonical: "is_object", aliases: &["isobject"], catalog_order: 4, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_object(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is an object.",
                category: "Type Predicates",
                order: 5,

                example: Some(Example { invocation: "is_object(obj)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_object) },
    FunctionRegistration { canonical: "is_empty", aliases: &["isempty"], catalog_order: 5, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_empty(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is null, empty string, empty array, or empty object.",
                category: "Type Predicates",
                order: 6,

                example: Some(Example { invocation: "is_empty(\"\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_empty_fn) },
    FunctionRegistration { canonical: "is_positive", aliases: &["ispositive"], catalog_order: 6, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_positive(val)",
                parameters: P_ANY,
                returns: R_BOOL_ERR,
                description: "Returns true when the coerced value is greater than zero.",
                category: "Type Predicates",
                order: 7,

                example: Some(Example { invocation: "is_positive(5)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_positive) },
    FunctionRegistration { canonical: "is_negative", aliases: &["isnegative"], catalog_order: 7, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_negative(val)",
                parameters: P_ANY,
                returns: R_BOOL_ERR,
                description: "Returns true when the coerced value is less than zero.",
                category: "Type Predicates",
                order: 8,

                example: Some(Example { invocation: "is_negative(-3)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_negative) },
    FunctionRegistration { canonical: "is_integer", aliases: &["isinteger"], catalog_order: 8, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_integer(val)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the value is a JSON number with no fractional component.",
                category: "Type Predicates",
                order: 9,

                example: Some(Example { invocation: "is_integer(7)", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_integer) },
    FunctionRegistration { canonical: "min", aliases: &[], catalog_order: 9, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "min(a, b)",
                parameters: P_NUM2,
                returns: R_NUM_ERR,
                description: "Returns the smaller of two numbers.",
                category: "Math",
                order: 1,

                example: Some(Example { invocation: "min(2, 5)", result: "2", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::min_fn) },
    FunctionRegistration { canonical: "max", aliases: &[], catalog_order: 10, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "max(a, b)",
                parameters: P_NUM2,
                returns: R_NUM_ERR,
                description: "Returns the larger of two numbers.",
                category: "Math",
                order: 2,

                example: Some(Example { invocation: "max(2, 5)", result: "5", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::max_fn) },
    FunctionRegistration { canonical: "abs", aliases: &[], catalog_order: 11, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "abs(x)",
                parameters: P_NUM,
                returns: R_NUM_ERR,
                description: "Returns the absolute value of a number.",
                category: "Math",
                order: 3,

                example: Some(Example { invocation: "abs(-3)", result: "3", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::abs_fn) },
    FunctionRegistration { canonical: "round", aliases: &[], catalog_order: 52, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "round(x, [default])",
                parameters: P_ROUND,
                returns: R_NUM,
                description: "Rounds a value to the nearest integer, with an optional default.",
                category: "Math",
                order: 4,

                example: Some(Example { invocation: "round(3.7)", result: "4", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::round_fn) },
];
