use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_STRING, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "terminal", aliases: &[], catalog_order: 30, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "terminal(string)",
                parameters: P_STRING,
                returns: R_STRING_ERR,
                description: "Renders Prose markup to a terminal string with ANSI SGR sequences.",
                category: "Rendering",
                order: 1,
        
                example: Some(Example { invocation: "terminal(\"hello\")", result: "hello", verification: ExampleVerification::Executable }),
        
            },
    ], handler: FunctionHandler::Pure(super::terminal) },
];
