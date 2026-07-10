use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_STRING, R_BOOL};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "has_skill", aliases: &["hasskill"], catalog_order: 77, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "has_skill(name)",
                parameters: P_STRING,
                returns: R_BOOL,
                description: "Returns true when a skill directory exists in a user-scoped or local-scoped skill root.",
                category: "Context",
                order: 1,
        
                example: Some(Example { invocation: "has_skill(\"darkmatter\")", result: "true", verification: ExampleVerification::DisplayOnly("depends on agent-specific skill roots outside the tempdir fixture") }),
        
            },
    ], handler: FunctionHandler::Context(super::has_skill_fn) },
    FunctionRegistration { canonical: "has_local_skill", aliases: &["haslocalskill"], catalog_order: 78, descriptors: &[
        ExpressionFunctionDescriptor {
        
                signature: "has_local_skill(name)",
                parameters: P_STRING,
                returns: R_BOOL,
                description: "Returns true when a skill directory exists in a local-scoped skill root.",
                category: "Context",
                order: 2,
        
                example: Some(Example { invocation: "has_local_skill(\"darkmatter\")", result: "true", verification: ExampleVerification::DisplayOnly("depends on agent-specific skill roots outside the tempdir fixture") }),
        
            },
    ], handler: FunctionHandler::Context(super::has_local_skill_fn) },
];
