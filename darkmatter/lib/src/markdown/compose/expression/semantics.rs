//! Typed descriptor catalogs for expression semantics.
//!
//! These catalogs expose the literal semantics of the Darkmatter expression
//! language — precedence, truthiness, parse modes, null propagation, unary and
//! comparison operators, arithmetic rules, and variable access — as static,
//! runtime-readable metadata. They are anchored to the parser and evaluator so
//! documentation surfaces such as `claudine context --expressions` can render
//! from a single source of truth rather than duplicating literal rows.

use crate::catalog::{Described, Example, ExampleVerification};

/// One precedence level in the expression grammar.
#[derive(Debug, Clone, PartialEq)]
pub struct OperatorDescriptor {
    /// Human-readable precedence name.
    pub name: &'static str,
    /// Operator tokens or forms at this level.
    pub operators: &'static str,
    /// Short description of what the level covers.
    pub description: &'static str,
    /// Stable display order (highest precedence first).
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for OperatorDescriptor {
    fn key(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Operator Precedence"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Precedence levels from highest (primary) to lowest (ternary).
pub const OPERATOR_DESCRIPTORS: &[OperatorDescriptor] = &[
    OperatorDescriptor {
        name: "Primary / member access",
        operators: "literals, variables, function calls, `foo.bar`, `foo[0]`, `(expr)`",
        description: "Tightest grouping: values and postfix access.",
        order: 1,
        example: Some(Example {
            invocation: "1 + 2 * 3",
            result: "7",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Unary",
        operators: "`!`, `-`",
        description: "Boolean negation and numeric negation.",
        order: 2,
        example: Some(Example {
            invocation: "!true",
            result: "false",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Multiplicative",
        operators: "`*`, `/`, `%`",
        description: "Left-associative multiplication, division, and remainder.",
        order: 3,
        example: Some(Example {
            invocation: "10 / 2 * 3",
            result: "15",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Additive",
        operators: "`+`, `-`",
        description: "Left-associative addition and subtraction; `+` coerces mixed numeric strings and numbers, and otherwise concatenates strings.",
        order: 4,
        example: Some(Example {
            invocation: "\"count: \" + 5",
            result: "count: 5",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Comparison",
        operators: "`==`, `!=`, `>`, `>=`, `<`, `<=`",
        description: "Equality compares scalar string forms; ordering coerces to numbers.",
        order: 5,
        example: Some(Example {
            invocation: "1 < 2",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Logical AND",
        operators: "`&&`",
        description: "Logical AND in both interpolation and condition mode.",
        order: 6,
        example: Some(Example {
            invocation: "true && false",
            result: "false",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Logical OR / Fallback",
        operators: "`||`",
        description: "Fallback in interpolation mode; logical OR in condition mode.",
        order: 7,
        example: Some(Example {
            invocation: "false || \"default\"",
            result: "default",
            verification: ExampleVerification::Executable,
        }),
    },
    OperatorDescriptor {
        name: "Ternary",
        operators: "`? :`",
        description: "Right-associative conditional expression.",
        order: 8,
        example: Some(Example {
            invocation: "true ? \"yes\" : \"no\"",
            result: "yes",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A truthiness rule: which values evaluate to falsy.
#[derive(Debug, Clone, PartialEq)]
pub struct TruthinessDescriptor {
    /// Literal form shown to the user.
    pub form: &'static str,
    /// Whether the form is falsy.
    pub is_falsy: bool,
    /// Short explanation.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for TruthinessDescriptor {
    fn key(&self) -> &'static str {
        self.form
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Truthiness"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Truthiness catalog: falsy values and the catch-all truthy rule.
pub const TRUTHINESS_DESCRIPTORS: &[TruthinessDescriptor] = &[
    TruthinessDescriptor {
        form: "`null` / missing",
        is_falsy: true,
        description: "Missing or undefined values are falsy.",
        order: 1,
        example: Some(Example {
            invocation: "!missing",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`false`",
        is_falsy: true,
        description: "Boolean false is falsy.",
        order: 2,
        example: Some(Example {
            invocation: "!false",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`0`",
        is_falsy: true,
        description: "Integer zero is falsy.",
        order: 3,
        example: Some(Example {
            invocation: "!0",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`0.0`",
        is_falsy: true,
        description: "Floating-point zero is falsy.",
        order: 4,
        example: Some(Example {
            invocation: "!0.0",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`\"\"` (empty string)",
        is_falsy: true,
        description: "The empty string is falsy.",
        order: 5,
        example: Some(Example {
            invocation: "!\"\"",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`[]` (empty array)",
        is_falsy: true,
        description: "An empty array is falsy.",
        order: 6,
        example: Some(Example {
            invocation: "!empty_array",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "`{}` (empty object)",
        is_falsy: true,
        description: "An empty object is falsy.",
        order: 7,
        example: Some(Example {
            invocation: "!empty_object",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    TruthinessDescriptor {
        form: "any other value",
        is_falsy: false,
        description: "All non-falsy values are truthy.",
        order: 8,
        example: Some(Example {
            invocation: "!\"hello\"",
            result: "false",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A parse-mode difference.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeDescriptor {
    /// Surface syntax (e.g. `{{ ... }}`).
    pub surface: &'static str,
    /// Meaning of `||` on that surface.
    pub pipe_meaning: &'static str,
    /// Short description.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for ModeDescriptor {
    fn key(&self) -> &'static str {
        self.surface
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Interpolation vs. Condition Mode"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Parse-mode catalog.
pub const MODE_DESCRIPTORS: &[ModeDescriptor] = &[
    ModeDescriptor {
        surface: "`{{ ... }}` (interpolation)",
        pipe_meaning: "fallback, first truthy value wins",
        description: "Interpolation surfaces treat `||` as fallback sugar.",
        order: 1,
        example: Some(Example {
            invocation: "\"a\" || \"b\"",
            result: "a",
            verification: ExampleVerification::Executable,
        }),
    },
    ModeDescriptor {
        surface: "`when=\"...\"` (condition)",
        pipe_meaning: "logical OR, returns a boolean",
        description: "Condition surfaces treat `||` as logical OR.",
        order: 2,
        example: Some(Example {
            invocation: "false || true",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A null-propagation rule.
#[derive(Debug, Clone, PartialEq)]
pub struct NullPropagationDescriptor {
    /// Operation or surface form.
    pub operation: &'static str,
    /// Behavior when null or missing is involved.
    pub behavior: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for NullPropagationDescriptor {
    fn key(&self) -> &'static str {
        self.operation
    }
    fn description(&self) -> &'static str {
        self.behavior
    }
    fn category(&self) -> &'static str {
        "Null Propagation"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Null-propagation catalog.
pub const NULL_PROPAGATION_DESCRIPTORS: &[NullPropagationDescriptor] = &[
    NullPropagationDescriptor {
        operation: "Dot access on `null` / missing path",
        behavior: "`null`",
        order: 1,
        example: Some(Example {
            invocation: "null.foo",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    NullPropagationDescriptor {
        operation: "Numeric dot access (`foo.0`)",
        behavior: "parse error / unsupported",
        order: 2,
        example: Some(Example {
            invocation: "foo.0",
            result: "parse error",
            verification: ExampleVerification::DisplayOnly("parse error, not an evaluable result"),
        }),
    },
    NullPropagationDescriptor {
        operation: "Bracket out-of-bounds",
        behavior: "`null`",
        order: 3,
        example: Some(Example {
            invocation: "numbers[9]",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    NullPropagationDescriptor {
        operation: "Bracket on `null` base",
        behavior: "`null`",
        order: 4,
        example: Some(Example {
            invocation: "null[0]",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    NullPropagationDescriptor {
        operation: "Bracket key on non-collection",
        behavior: "`null`",
        order: 5,
        example: Some(Example {
            invocation: "\"text\"[\"key\"]",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    NullPropagationDescriptor {
        operation: "Negative index from the end",
        behavior: "element or `null` if outside range",
        order: 6,
        example: Some(Example {
            invocation: "numbers[-1]",
            result: "3",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A unary operator rule.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOperatorDescriptor {
    /// Syntax form.
    pub syntax: &'static str,
    /// Description of the operator's behavior.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for UnaryOperatorDescriptor {
    fn key(&self) -> &'static str {
        self.syntax
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Unary Operators"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Unary-operator catalog.
pub const UNARY_OPERATOR_DESCRIPTORS: &[UnaryOperatorDescriptor] = &[
    UnaryOperatorDescriptor {
        syntax: "`!x`",
        description: "boolean negation (`!truthy ⇒ false`, `!falsy ⇒ true`)",
        order: 1,
        example: Some(Example {
            invocation: "!\"\"",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    UnaryOperatorDescriptor {
        syntax: "`-x`",
        description: "numeric negation; `-null` is `null`; `-\"hi\"` is an error",
        order: 2,
        example: Some(Example {
            invocation: "-7",
            result: "-7",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A comparison-operator rule.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonOperatorDescriptor {
    /// Syntax form.
    pub syntax: &'static str,
    /// Description of the operator's behavior.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for ComparisonOperatorDescriptor {
    fn key(&self) -> &'static str {
        self.syntax
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Comparison Operators"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Comparison-operator catalog.
pub const COMPARISON_OPERATOR_DESCRIPTORS: &[ComparisonOperatorDescriptor] = &[
    ComparisonOperatorDescriptor {
        syntax: "`==` and `!=`",
        description: "compare scalar string representations",
        order: 1,
        example: Some(Example {
            invocation: "\"5\" == 5",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    ComparisonOperatorDescriptor {
        syntax: "`>`, `>=`, `<`, `<=`",
        description: "coerce both sides to numbers",
        order: 2,
        example: Some(Example {
            invocation: "\"10\" > 2",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
    ComparisonOperatorDescriptor {
        syntax: "both sides missing",
        description: "`a == b` is false and `a != b` is also false",
        order: 3,
        example: Some(Example {
            invocation: "missing_a == missing_b",
            result: "false",
            verification: ExampleVerification::Executable,
        }),
    },
    ComparisonOperatorDescriptor {
        syntax: "one side defined, one missing",
        description: "`==` is false and `!=` is true",
        order: 4,
        example: Some(Example {
            invocation: "\"x\" != missing",
            result: "true",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// An arithmetic-operator rule.
#[derive(Debug, Clone, PartialEq)]
pub struct ArithmeticOperatorDescriptor {
    /// Syntax form.
    pub syntax: &'static str,
    /// Description of the operator's behavior.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for ArithmeticOperatorDescriptor {
    fn key(&self) -> &'static str {
        self.syntax
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Arithmetic Operators"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Arithmetic-operator catalog.
pub const ARITHMETIC_OPERATOR_DESCRIPTORS: &[ArithmeticOperatorDescriptor] = &[
    ArithmeticOperatorDescriptor {
        syntax: "`+`, `-`, `*`, `/`, `%`",
        description: "require numeric operands",
        order: 1,
        example: Some(Example {
            invocation: "7 % 3",
            result: "1",
            verification: ExampleVerification::Executable,
        }),
    },
    ArithmeticOperatorDescriptor {
        syntax: "`+`",
        description: "coerces a numeric string paired with a number; otherwise concatenates strings",
        order: 2,
        example: Some(Example {
            invocation: "\"a\" + \"b\"",
            result: "ab",
            verification: ExampleVerification::Executable,
        }),
    },
    ArithmeticOperatorDescriptor {
        syntax: "division by zero",
        description: "`x / 0` and `x % 0` raise an error",
        order: 3,
        example: Some(Example {
            invocation: "5 / 0",
            result: "error",
            verification: ExampleVerification::DisplayOnly("error path, not an evaluable result"),
        }),
    },
    ArithmeticOperatorDescriptor {
        syntax: "`%`",
        description: "follows C-style semantics: the sign of `a % b` follows the sign of `a`",
        order: 4,
        example: Some(Example {
            invocation: "-7 % 3",
            result: "-1",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// A variable-access rule.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableAccessDescriptor {
    /// Surface form.
    pub form: &'static str,
    /// Description of the access form.
    pub description: &'static str,
    /// Stable display order.
    pub order: usize,
    /// Optional verified example expression and result.
    pub example: Option<Example>,
}

impl Described for VariableAccessDescriptor {
    fn key(&self) -> &'static str {
        self.form
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        "Variable Access"
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

/// Variable-access catalog.
pub const VARIABLE_ACCESS_DESCRIPTORS: &[VariableAccessDescriptor] = &[
    VariableAccessDescriptor {
        form: "simple keys",
        description: "`draft`",
        order: 1,
        example: Some(Example {
            invocation: "draft",
            result: "hello",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "nested keys",
        description: "`user.role`",
        order: 2,
        example: Some(Example {
            invocation: "user.role",
            result: "admin",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "context variables",
        description: "`ctx.today`, `ctx.repo`, `ctx.current_package`",
        order: 3,
        example: Some(Example {
            invocation: "ctx.repo",
            result: "rusty-biscuit",
            verification: ExampleVerification::DisplayOnly("context values depend on the host environment"),
        }),
    },
    VariableAccessDescriptor {
        form: "environment keys",
        description: "`env.AGENT`, `env.HOME`",
        order: 4,
        example: Some(Example {
            invocation: "env.AGENT",
            result: "opencode",
            verification: ExampleVerification::DisplayOnly("env values depend on the host environment"),
        }),
    },
    VariableAccessDescriptor {
        form: "dot access on missing path",
        description: "a non-existent path resolves to `null` (no error)",
        order: 5,
        example: Some(Example {
            invocation: "user.missing",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "dot access on `null` base",
        description: "`null.foo` returns `null`",
        order: 6,
        example: Some(Example {
            invocation: "null.foo",
            result: "null",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "numeric dot access",
        description: "`foo.0` is not supported — use bracket syntax",
        order: 7,
        example: Some(Example {
            invocation: "foo.0",
            result: "parse error",
            verification: ExampleVerification::DisplayOnly("parse error, not an evaluable result"),
        }),
    },
    VariableAccessDescriptor {
        form: "bracket arrays",
        description: "`foo[0]`, `foo[-1]` (negative indexes count from the end)",
        order: 8,
        example: Some(Example {
            invocation: "numbers[-1]",
            result: "3",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "bracket objects",
        description: "`foo[\"key\"]` (string keys only)",
        order: 9,
        example: Some(Example {
            invocation: "config[\"name\"]",
            result: "dark",
            verification: ExampleVerification::Executable,
        }),
    },
    VariableAccessDescriptor {
        form: "chained access",
        description: "`items[-1].name`, `config[\"key\"][0]`",
        order: 10,
        example: Some(Example {
            invocation: "items[0].name",
            result: "first",
            verification: ExampleVerification::Executable,
        }),
    },
];

/// Returns all operator-precedence descriptors.
pub fn operator_descriptors() -> &'static [OperatorDescriptor] {
    OPERATOR_DESCRIPTORS
}

/// Returns all truthiness descriptors.
pub fn truthiness_descriptors() -> &'static [TruthinessDescriptor] {
    TRUTHINESS_DESCRIPTORS
}

/// Returns all parse-mode descriptors.
pub fn mode_descriptors() -> &'static [ModeDescriptor] {
    MODE_DESCRIPTORS
}

/// Returns all null-propagation descriptors.
pub fn null_propagation_descriptors() -> &'static [NullPropagationDescriptor] {
    NULL_PROPAGATION_DESCRIPTORS
}

/// Returns all unary-operator descriptors.
pub fn unary_operator_descriptors() -> &'static [UnaryOperatorDescriptor] {
    UNARY_OPERATOR_DESCRIPTORS
}

/// Returns all comparison-operator descriptors.
pub fn comparison_operator_descriptors() -> &'static [ComparisonOperatorDescriptor] {
    COMPARISON_OPERATOR_DESCRIPTORS
}

/// Returns all arithmetic-operator descriptors.
pub fn arithmetic_operator_descriptors() -> &'static [ArithmeticOperatorDescriptor] {
    ARITHMETIC_OPERATOR_DESCRIPTORS
}

/// Returns all variable-access descriptors.
pub fn variable_access_descriptors() -> &'static [VariableAccessDescriptor] {
    VARIABLE_ACCESS_DESCRIPTORS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::{
        evaluate, parse, EvaluationLookup, ResolutionContext,
    };
    use serde_json::Value;
    use std::collections::HashMap;

    struct FixtureLookup {
        ctx: ResolutionContext,
        data: HashMap<String, Value>,
    }

    impl EvaluationLookup for FixtureLookup {
        fn get(&self, path: &str) -> Option<Value> {
            let mut segments = path.split('.');
            let first = segments.next()?;
            let mut current = self.data.get(first)?.clone();
            for segment in segments {
                match current {
                    Value::Object(mut map) => {
                        current = map.remove(segment).unwrap_or(Value::Null);
                    }
                    _ => return Some(Value::Null),
                }
            }
            Some(current)
        }
        fn resolution_context(&self) -> Option<ResolutionContext> {
            Some(self.ctx.clone())
        }
    }

    fn make_fixture() -> (tempfile::TempDir, FixtureLookup) {
        let dir = tempfile::TempDir::new().unwrap();
        let mut data = HashMap::new();
        data.insert("draft".to_string(), serde_json::json!("hello"));
        data.insert("user".to_string(), serde_json::json!({"role": "admin", "missing": null}));
        data.insert("items".to_string(), serde_json::json!([{"name": "first"}, {"name": "second"}, {"name": "third"}]));
        data.insert("numbers".to_string(), serde_json::json!([1, 2, 3]));
        data.insert("config".to_string(), serde_json::json!({"name": "dark"}));
        data.insert("empty_array".to_string(), serde_json::json!([]));
        data.insert("empty_object".to_string(), serde_json::json!({}));
        let ctx = ResolutionContext::new(dir.path().to_path_buf());
        let lookup = FixtureLookup { ctx, data };
        (dir, lookup)
    }

    fn render_value(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    fn evaluate_example(invocation: &str, lookup: &FixtureLookup) -> Result<String, String> {
        let expr = parse(invocation).map_err(|e| e.message)?;
        let value = evaluate(&expr, lookup).map_err(|error| error.to_string())?;
        Ok(render_value(&value))
    }

    /// Every cataloged precedence example must evaluate to its declared result.
    #[test]
    fn operator_precedence_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in OPERATOR_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.name, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.name, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "operator precedence examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Truthiness examples must evaluate to their declared results.
    #[test]
    fn truthiness_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in TRUTHINESS_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.form, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.form, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "truthiness examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Mode examples must evaluate to their declared results.
    #[test]
    fn mode_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in MODE_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.surface, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.surface, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "mode examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Null-propagation examples must evaluate to their declared results.
    #[test]
    fn null_propagation_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in NULL_PROPAGATION_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.operation, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.operation, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "null propagation examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Unary-operator examples must evaluate to their declared results.
    #[test]
    fn unary_operator_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in UNARY_OPERATOR_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.syntax, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.syntax, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "unary operator examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Comparison-operator examples must evaluate to their declared results.
    #[test]
    fn comparison_operator_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in COMPARISON_OPERATOR_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.syntax, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.syntax, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "comparison operator examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Arithmetic-operator examples must evaluate to their declared results.
    #[test]
    fn arithmetic_operator_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in ARITHMETIC_OPERATOR_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.syntax, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.syntax, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "arithmetic operator examples did not evaluate correctly: {failures:?}"
        );
    }

    /// Variable-access examples must evaluate to their declared results.
    #[test]
    fn variable_access_examples_evaluate_correctly() {
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for desc in VARIABLE_ACCESS_DESCRIPTORS {
            let Some(example) = desc.example() else { continue };
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            match evaluate_example(example.invocation, &lookup) {
                Ok(result) if result == example.result => {}
                Ok(result) => failures.push((desc.form, example.invocation, result, example.result)),
                Err(err) => failures.push((desc.form, example.invocation, err, example.result)),
            }
        }
        assert!(
            failures.is_empty(),
            "variable access examples did not evaluate correctly: {failures:?}"
        );
    }

    /// The operator-precedence catalog must stay in sync with the parser's own
    /// precedence table.
    #[test]
    fn operator_precedence_matches_parser() {
        use crate::markdown::compose::expression::parser::PRECEDENCE_TABLE;

        assert_eq!(
            OPERATOR_DESCRIPTORS.len(),
            PRECEDENCE_TABLE.len(),
            "operator descriptor count must match parser precedence table count"
        );

        for (descriptor, (parser_name, _)) in OPERATOR_DESCRIPTORS.iter().zip(PRECEDENCE_TABLE.iter()) {
            assert_eq!(
                descriptor.name, *parser_name,
                "operator descriptor name `{}` must match parser precedence table name `{}`",
                descriptor.name, parser_name
            );
        }
    }
}
