//! Typed descriptor catalog for expression functions.
//!
//! Each [`ExpressionFunctionDescriptor`] describes a single callable function
//! available in Darkmatter expressions. The public catalog is projected once
//! from the embedded authored schema; reading it performs no host probes, no
//! filesystem I/O, and no runtime context capture.
//!
//! Each descriptor now also carries a **typed signature** ([`ParamType`] per
//! parameter plus a [`ReturnType`]). The type vocabulary is the schema-plus
//! *data-type* domain ([`DataType`]) for parameters, and data types **plus the
//! `error` union member** (a [`ReturnType::fallible`] flag) for returns. This is
//! a **catalog-only** concern: [`DataType`] deliberately has **no** `error` or
//! function-type variant, and `error` is a return-position flag — never a
//! parameter type — so a frontmatter property can never be typed as a function
//! or as `error` (spec D7, schema-plus § Type domains).
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::catalog::{Described, Example, ExampleVerification};

pub(crate) mod ast;
pub(crate) mod parser;

use ast::{CatalogVerification, ExpressionFunctionCatalog};
use parser::{CatalogParseError, parse_expression_function_catalog};

/// A data type usable in a function **parameter** or (non-`error`) **return**
/// position.
///
/// Mirrors the schema-plus *data-type* domain — the `SimplifiedType` keyword set
/// plus `any`. It intentionally carries **no** `error` variant and **no**
/// function type: those belong to the return and catalog domains respectively,
/// keeping the frontmatter validator's type set untouched by function typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// `string`.
    String,
    /// `number`.
    Number,
    /// `number(integer)` — an integer-constrained number.
    Integer,
    /// `boolean`.
    Boolean,
    /// `date`.
    Date,
    /// `datetime`.
    DateTime,
    /// `time`.
    Time,
    /// `object`.
    Object,
    /// `file` reference.
    File,
    /// `url`.
    Url,
    /// `email`.
    Email,
    /// `yaml` content-format string.
    Yaml,
    /// `json` content-format string.
    Json,
    /// `any`.
    Any,
}

impl DataType {
    /// Parses a canonical function-catalog type keyword.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        use crate::markdown::schemas::SimplifiedType;

        if keyword == "number(integer)" {
            return Some(Self::Integer);
        }
        Some(match SimplifiedType::from_keyword(keyword)? {
            SimplifiedType::String => Self::String,
            SimplifiedType::Number => Self::Number,
            SimplifiedType::Boolean => Self::Boolean,
            SimplifiedType::Date => Self::Date,
            SimplifiedType::DateTime => Self::DateTime,
            SimplifiedType::Time => Self::Time,
            SimplifiedType::Object => Self::Object,
            SimplifiedType::File => Self::File,
            SimplifiedType::Url => Self::Url,
            SimplifiedType::Email => Self::Email,
            SimplifiedType::Yaml => Self::Yaml,
            SimplifiedType::Json => Self::Json,
            SimplifiedType::Any => Self::Any,
            SimplifiedType::NumberLike
            | SimplifiedType::Boolish
            | SimplifiedType::Enum
            | SimplifiedType::Literal
            | SimplifiedType::Expression
            | SimplifiedType::TypeDefinition
            | SimplifiedType::Schema => {
                return None;
            }
        })
    }

    /// Canonical type keyword (`string`, `datetime`, `any`, …).
    pub const fn as_keyword(self) -> &'static str {
        match self {
            DataType::String => "string",
            DataType::Number => "number",
            DataType::Integer => "number(integer)",
            DataType::Boolean => "boolean",
            DataType::Date => "date",
            DataType::DateTime => "datetime",
            DataType::Time => "time",
            DataType::Object => "object",
            DataType::File => "file",
            DataType::Url => "url",
            DataType::Email => "email",
            DataType::Yaml => "yaml",
            DataType::Json => "json",
            DataType::Any => "any",
        }
    }
}

/// One typed function parameter.
///
/// Parameter names live in [`ExpressionFunctionDescriptor::signature`]; this
/// carries only the *type* shape so common parameter lists can be shared as
/// `const`s across descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamType {
    /// The parameter's data type.
    pub ty: DataType,
    /// Whether the parameter is an array of `ty` (`ty[]`).
    pub array: bool,
    /// Whether the parameter is optional (`[name]` in the signature).
    pub optional: bool,
    /// Whether the parameter is variadic (`...`).
    pub variadic: bool,
}

impl ParamType {
    /// A required scalar parameter of type `ty`.
    pub const fn val(ty: DataType) -> Self {
        Self { ty, array: false, optional: false, variadic: false }
    }
    /// A required array parameter (`ty[]`).
    pub const fn array(ty: DataType) -> Self {
        Self { ty, array: true, optional: false, variadic: false }
    }
    /// An optional scalar parameter.
    pub const fn optional(ty: DataType) -> Self {
        Self { ty, array: false, optional: true, variadic: false }
    }
    /// A variadic scalar parameter.
    pub const fn variadic(ty: DataType) -> Self {
        Self { ty, array: false, optional: false, variadic: true }
    }
}

/// A function's typed return.
///
/// A fallible function returns `<success> | error`, modeled by
/// [`ReturnType::fallible`] set to `true` (mirrors Rust `Result<T, error>`). The
/// `error` type is a **return-position-only** anchor — it is never a
/// [`DataType`] and so can never appear in a parameter or a frontmatter property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnType {
    /// The success data type.
    pub ty: DataType,
    /// Whether the success value is an array (`ty[]`).
    pub array: bool,
    /// Whether the function is fallible (adds the `| error` union member).
    pub fallible: bool,
}

impl ReturnType {
    /// An infallible scalar return of type `ty`.
    pub const fn plain(ty: DataType) -> Self {
        Self { ty, array: false, fallible: false }
    }
    /// A fallible scalar return (`ty | error`).
    pub const fn fallible(ty: DataType) -> Self {
        Self { ty, array: false, fallible: true }
    }
}

/// Descriptor for a single expression function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpressionFunctionDescriptor {

    /// Canonical snake_case signature (e.g., `is_string(x)`). Preserved verbatim
    /// as the display key for `md schema about`, the generated doc table, and
    /// DMLS descriptor consumers.
    pub signature: &'static str,
    /// Short description of the function's behavior.
    pub description: &'static str,
    /// Logical grouping category.
    pub category: &'static str,
    /// Stable display order within the category.
    pub order: usize,
    /// Typed parameter list, in signature order (data types only; names come
    /// from [`Self::signature`]).
    pub parameters: &'static [ParamType],
    /// Typed return, including `error` union membership for fallible functions.
    pub returns: ReturnType,
    /// Optional verified example.
    pub example: Option<Example>,
}

impl ExpressionFunctionDescriptor {
    /// Renders the fully typed signature, e.g. `as_csv(list: any[]) -> string | error`.
    ///
    /// Parameter names are read from [`Self::signature`]; their types come from
    /// [`Self::parameters`]. Optional parameters render as `[name: type]`,
    /// variadic parameters as `...type`. A fallible return appends `| error`.
    pub fn typed_signature(&self) -> String {
        let name = self.signature.split('(').next().unwrap_or(self.signature);
        let inner = self
            .signature
            .split_once('(')
            .and_then(|(_, rest)| rest.rsplit_once(')'))
            .map(|(params, _)| params)
            .unwrap_or("");
        let raw_names: Vec<&str> = if inner.trim().is_empty() {
            Vec::new()
        } else {
            inner.split(',').map(str::trim).collect()
        };

        let mut parts = Vec::with_capacity(self.parameters.len());
        for (i, p) in self.parameters.iter().enumerate() {
            let mut ty = p.ty.as_keyword().to_string();
            if p.array {
                ty.push_str("[]");
            }
            if p.variadic {
                parts.push(format!("...{ty}"));
                continue;
            }
            let raw = raw_names.get(i).copied().unwrap_or("");
            let base = raw.trim_start_matches('[').trim_end_matches(']');
            if p.optional {
                parts.push(format!("[{base}: {ty}]"));
            } else {
                parts.push(format!("{base}: {ty}"));
            }
        }

        let mut ret = self.returns.ty.as_keyword().to_string();
        if self.returns.array {
            ret.push_str("[]");
        }
        if self.returns.fallible {
            ret.push_str(" | error");
        }
        format!("{name}({}) -> {ret}", parts.join(", "))
    }
}
impl Described for ExpressionFunctionDescriptor {
    fn key(&self) -> &'static str {
        self.signature
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        self.category
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn project_descriptors(
    catalog: ExpressionFunctionCatalog,
) -> Vec<ExpressionFunctionDescriptor> {
    let mut functions = catalog.functions;
    functions.sort_by_key(|function| function.order);

    let mut category_orders = HashMap::<String, usize>::new();
    let mut descriptors = Vec::new();
    for function in functions {
        let category_order = category_orders.entry(function.category.clone()).or_default();
        let category = leak(function.category);
        let description = leak(function.description);
        for overload in function.overloads {
            *category_order += 1;
            let parameter_names = overload
                .parameters
                .iter()
                .map(|parameter| {
                    if parameter.variadic {
                        "...".to_string()
                    } else if parameter.optional {
                        format!("[{}]", parameter.name)
                    } else {
                        parameter.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let signature = leak(format!("{}({parameter_names})", function.name));
            let parameters = overload
                .parameters
                .into_iter()
                .map(|parameter| ParamType {
                    ty: parameter.ty,
                    array: parameter.array,
                    optional: parameter.optional,
                    variadic: parameter.variadic,
                })
                .collect::<Vec<_>>();
            let parameters = Box::leak(parameters.into_boxed_slice());
            let verification = match overload.example.verification {
                CatalogVerification::Executable => ExampleVerification::Executable,
                CatalogVerification::DisplayOnly(reason) => {
                    ExampleVerification::DisplayOnly(leak(reason))
                }
            };
            descriptors.push(ExpressionFunctionDescriptor {
                signature,
                description,
                category,
                order: *category_order,
                parameters,
                returns: ReturnType {
                    ty: overload.returns.ty,
                    array: overload.returns.array,
                    fallible: overload.returns.fallible,
                },
                example: Some(Example {
                    invocation: leak(overload.example.expression),
                    result: leak(overload.example.result),
                    verification,
                }),
            });
        }
    }
    descriptors
}

pub(crate) fn expression_function_catalog() -> &'static ExpressionFunctionCatalog {
    static CATALOG: LazyLock<ExpressionFunctionCatalog> = LazyLock::new(|| {
        parse_expression_function_catalog(include_str!(
            "../../../../../../docs/schemas/expression-functions.yaml"
        ))
        .expect("embedded expression-function catalog must parse")
    });
    &CATALOG
}

#[allow(dead_code)]
pub(crate) fn try_parse_catalog(
    yaml: &str,
) -> Result<ExpressionFunctionCatalog, CatalogParseError> {
    parse_expression_function_catalog(yaml)
}

/// Returns all expression function descriptors in display order.
pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    static DESCRIPTORS: LazyLock<Vec<ExpressionFunctionDescriptor>> = LazyLock::new(|| {
        project_descriptors(expression_function_catalog().clone())
    });
    &DESCRIPTORS
}

/// Generates a Markdown function-reference table from the expression catalog.
///
/// The output is a single table with `Category`, `Function`, `Description`, and
/// `Example` columns, suitable for embedding in `darkmatter-expressions.md`.
/// Only machine-executed (`Executable`) examples populate the example cell;
/// display-only examples are illustrative metadata, not verified results, so
/// their cell is left empty.
pub fn generate_expression_function_table() -> String {
    let mut out = String::new();
    out.push_str("| Category | Function | Description | Example |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for d in expression_function_descriptors() {
        let example = match d.example() {
            Some(ex) if ex.verification == ExampleVerification::Executable => {
                format!("`{}` ⇒ `{}`", ex.invocation, ex.result)
            }
            _ => String::new(),
        };
        let description = d.description().replace('|', "\\|");
        out.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            d.category(),
            d.key(),
            description,
            example
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::functions::{
        dispatchable_signatures, lazy_operator_names,
    };
    use crate::markdown::compose::expression::{
        evaluate, parse, EvaluationLookup, ResolutionContext,
    };
    use serde_json::Value;
    use std::collections::HashSet;

    #[test]
    fn malformed_catalog_is_rejected_before_projection() {
        // The fixture carries a minimal `$schema` so it clears structural
        // validation and reaches the function-domain `EmptyCatalog` check.
        let error = try_parse_catalog(
            "kind: expression-function-catalog\n$schema:\n  kind: string(required)\n  functions: string[](required)\nfunctions: []\n",
        )
        .expect_err("an empty catalog must not reach descriptor projection");
        assert!(error.to_string().contains("function"));
    }

    /// `try_parse_catalog` returns the **owned** AST, never a leaking
    /// `&'static` descriptor projection. The spec's lifetime decision permits
    /// leaking only the one validated embedded catalog (via
    /// [`expression_function_descriptors`]); the fixture path must stay owned
    /// so repeated valid-fixture parsing cannot accumulate process-lifetime
    /// allocations. The explicit owned type annotation below is the
    /// structural proof; each iteration's catalog then drops in place.
    #[test]
    fn try_parse_catalog_returns_owned_catalog_without_leaking() {
        use super::ast::ExpressionFunctionCatalog;

        const FIXTURE: &str = r#"kind: expression-function-catalog
$schema:
  kind: string(required)
  functions: "{ name: string(not-empty; required), category: string(not-empty; required), order: number(integer; required), description: string(not-empty; required), overloads: { parameters: { name: string(not-empty; required), type: string(not-empty; required), array: boolean, optional: boolean, variadic: boolean }[], returns: { type: string(not-empty; required), array: boolean, fallible: boolean }, example: { expression: string(not-empty; required), result: string(required), verification: enum(executable, display-only; required), reason: string(not-empty) } }[](min(1); required) }[](min(1); required)"
functions:
  - name: ping
    category: Test
    order: 1
    description: Returns its argument.
    overloads:
      - parameters:
          - { name: value, type: string }
        returns: { type: string }
        example: { expression: 'ping("x")', result: x, verification: executable }
"#;

        let catalog: ExpressionFunctionCatalog =
            try_parse_catalog(FIXTURE).expect("valid fixture must parse as owned catalog");
        assert_eq!(catalog.functions.len(), 1);
        assert_eq!(catalog.functions[0].name, "ping");

        // Repeatable valid-fixture parsing with no static promotion: each
        // owned catalog drops at the end of the iteration.
        for _ in 0..3 {
            let _owned: ExpressionFunctionCatalog =
                try_parse_catalog(FIXTURE).expect("valid fixture must parse repeatedly");
        }
    }

    /// The number of arguments to pass when exercising a signature: the count
    /// of comma-separated parameters, with a variadic `...` exercised at two
    /// arguments and optional `[param]` placeholders counted as present.
    fn signature_call_arity(signature: &str) -> usize {
        let inner = signature
            .split_once('(')
            .and_then(|(_, rest)| rest.rsplit_once(')'))
            .map(|(params, _)| params.trim())
            .unwrap_or("");
        if inner.is_empty() {
            return 0;
        }
        if inner.contains("...") {
            return 2;
        }
        inner.split(',').filter(|p| !p.trim().is_empty()).count()
    }

    /// Whether `message` is an arity (wrong-argument-count) error rather than a
    /// type/domain error.
    ///
    /// Arity errors come from `require_args` and the variadic count guards and
    /// read "… requires N argument(s)" / "… requires 1 or 2 arguments" / "…
    /// requires at least 1 argument". Type errors also contain "requires …
    /// argument" but name the rejected domain ("numeric"/"string"/"array"), so
    /// those are excluded.
    fn is_arity_error(message: &str) -> bool {
        let m = message.to_lowercase();
        m.contains("requires")
            && m.contains("argument")
            && !m.contains("numeric")
            && !m.contains("string argument")
            && !m.contains("array argument")
    }

    /// A lookup that supplies a [`ResolutionContext`] so the filesystem
    /// dispatch surface (`dispatch_fs`) is reachable — without one,
    /// `evaluate_function` skips `dispatch_fs` and every fs function would look
    /// "unknown" even though it is dispatchable.
    struct FsLookup {
        ctx: ResolutionContext,
    }

    impl EvaluationLookup for FsLookup {
        fn get(&self, _path: &str) -> Option<Value> {
            None
        }
        fn resolution_context(&self) -> Option<ResolutionContext> {
            Some(self.ctx.clone())
        }
    }

    /// Evaluate `name(0, 0, …)` with `arity` arguments through the real parse +
    /// evaluate pipeline and return the error string, if any. A recognized
    /// function either succeeds or fails with an argument/type error; only an
    /// *unrecognized* name yields `Unknown function: …`.
    fn dispatch_error_arity(name: &str, arity: usize, lookup: &FsLookup) -> Option<String> {
        let args = vec!["0"; arity].join(", ");
        let expr = parse(&format!("{name}({args})")).expect("descriptor signature must parse");
        evaluate(&expr, lookup).err().map(|error| error.to_string())
    }

    /// Convenience: exercise a name with two arguments.
    fn dispatch_error(name: &str, lookup: &FsLookup) -> Option<String> {
        dispatch_error_arity(name, 2, lookup)
    }

    /// Exact, bidirectional parity between descriptor *signatures* and the
    /// runtime *signature* surface — overload for overload, not merely name for
    /// name.
    ///
    /// The runtime side is [`dispatchable_signatures`], which enumerates the
    /// per-registration `signatures` of [`dispatch`]/[`dispatch_fs`] plus the
    /// lazy logical operators. Comparing full signatures (with arity) rather
    /// than collapsed names means set equality fails in *both* directions for
    /// overloads too:
    ///
    /// - adding or removing a callable overload (e.g. a second
    ///   `frontmatter(file, prop)` registration signature) without the matching
    ///   descriptor, and
    /// - adding or removing a descriptor overload without the matching callable
    ///   signature.
    #[test]
    fn descriptor_signature_set_equals_dispatchable_signature_set() {
        let descriptors: HashSet<&str> = expression_function_descriptors()
            .iter()
            .map(|d| d.signature)
            .collect();
        let runtime: HashSet<&str> = dispatchable_signatures().into_iter().collect();

        let missing_descriptors: Vec<_> = runtime.difference(&descriptors).collect();
        let extra_descriptors: Vec<_> = descriptors.difference(&runtime).collect();

        assert!(
            missing_descriptors.is_empty(),
            "dispatchable signatures without descriptors: {missing_descriptors:?}"
        );
        assert!(
            extra_descriptors.is_empty(),
            "descriptor signatures without a dispatchable signature: {extra_descriptors:?}"
        );
    }

    /// The lazy logical operators must genuinely resolve at runtime, so their
    /// presence in the lazy registrations (and the parity set above) is real.
    #[test]
    fn lazy_operators_are_dispatchable() {
        let lookup = FsLookup {
            ctx: ResolutionContext::new(std::env::temp_dir()),
        };
        for name in lazy_operator_names() {
            let err = dispatch_error(name, &lookup);
            assert!(
                err.as_deref().map(|e| !e.contains("Unknown function")).unwrap_or(true),
                "lazy operator `{name}` must dispatch; got error: {err:?}"
            );
        }
    }

    /// Every descriptor overload must be dispatchable at its declared arity.
    ///
    /// Complements the set-equality test above with an end-to-end proof that is
    /// arity-aware: each descriptor signature is parsed for its argument count
    /// and exercised through the actual `evaluate` → `evaluate_function` →
    /// `dispatch_fs`/`dispatch` pipeline at *that* arity. A descriptor whose
    /// handler was removed yields `Unknown function`; a bogus overload arity the
    /// handler rejects (e.g. a spurious three-argument `frontmatter`) yields an
    /// arity error. Either fails here — so the declared signatures are bound to
    /// what the runtime genuinely accepts, not just to a dispatchable name.
    #[test]
    fn every_descriptor_overload_is_dispatchable_at_its_declared_arity() {
        let lookup = FsLookup {
            ctx: ResolutionContext::new(std::env::temp_dir()),
        };

        let mut failures = Vec::new();
        for desc in expression_function_descriptors() {
            let name = desc.signature.split('(').next().unwrap();
            let arity = signature_call_arity(desc.signature);
            if let Some(err) = dispatch_error_arity(name, arity, &lookup)
                && (err.contains("Unknown function") || is_arity_error(&err))
            {
                failures.push((desc.signature, err));
            }
        }

        assert!(
            failures.is_empty(),
            "descriptor overloads the evaluator does not accept at their declared arity: {failures:?}"
        );
    }

    /// Anchor for the recognition test: a name with no runtime arm must be
    /// rejected as `Unknown function`, proving the assertion above is real.
    #[test]
    fn unknown_function_is_rejected() {
        let lookup = FsLookup {
            ctx: ResolutionContext::new(std::env::temp_dir()),
        };
        let err = dispatch_error("definitely_not_a_real_function", &lookup)
            .expect("an unknown function must error");
        assert!(
            err.contains("Unknown function"),
            "unknown name must report `Unknown function`; got: {err}"
        );
    }

    #[test]
    fn descriptor_traversal_order_is_deterministic() {
        let sigs: Vec<&str> = expression_function_descriptors()
            .iter()
            .map(|d| d.signature)
            .collect();
        let sigs_again: Vec<&str> = expression_function_descriptors()
            .iter()
            .map(|d| d.signature)
            .collect();
        assert_eq!(sigs, sigs_again);
    }

    #[test]
    fn descriptor_signatures_are_unique() {
        let mut seen = HashSet::new();
        for d in expression_function_descriptors() {
            assert!(
                seen.insert(d.signature),
                "Duplicate descriptor signature: {}",
                d.signature
            );
        }
    }

    #[test]
    fn catalog_access_performs_no_capture() {
        let _ = expression_function_descriptors();
    }

    /// Every expression descriptor that carries an example must declare it
    /// `Executable` or `DisplayOnly` — never `TypeShapeOnly`. Expression
    /// functions are deterministic enough to either be executed or to carry a
    /// documented opt-out reason; a "type shape only" example would be an
    /// un-audited, un-explained middle ground.
    #[test]
    fn every_expression_example_is_executable_or_display_only() {
        use crate::catalog::ExampleVerification;
        let mut offenders = Vec::new();
        for d in expression_function_descriptors() {
            if let Some(example) = d.example()
                && matches!(example.verification, ExampleVerification::TypeShapeOnly)
            {
                offenders.push(d.signature);
            }
        }
        assert!(
            offenders.is_empty(),
            "expression descriptors must not use TypeShapeOnly: {offenders:?}"
        );
    }

    /// The generated function table in `darkmatter-expressions.md` must match
    /// the catalog output exactly.
    #[test]
    fn narrative_doc_function_table_matches_catalog() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let doc_path = manifest_dir
            .join("../../darkmatter/docs/topics/darkmatter-expressions.md");
        let content = std::fs::read_to_string(&doc_path)
            .expect("darkmatter-expressions.md should be readable");

        const START: &str = "<!-- BEGIN GENERATED FUNCTION TABLE -->";
        const END: &str = "<!-- END GENERATED FUNCTION TABLE -->";

        let start = content.find(START).expect("start marker should exist") + START.len();
        let end = content.find(END).expect("end marker should exist");

        let doc_table = content[start..end].trim();
        let generated = generate_expression_function_table().trim().to_string();

        assert_eq!(
            doc_table, generated,
            "function table in darkmatter-expressions.md does not match generated output"
        );
    }

    /// Claudine anti-drift: every function added by this feature must remain
    /// present in the exported expression catalog (`claudine context --expressions`).
    ///
    /// Asserts against the shared [`expression_function_descriptors()`] so the
    /// check stays in sync with the runtime surface and does not duplicate a
    /// Claudine-only list.
    #[test]
    fn feature_functions_are_present_in_exported_expression_catalog() {
        let expected = [
            // Phase 3 — pure functions
            "is_positive(val)",
            "is_negative(val)",
            "is_integer(val)",
            "without_date(string)",
            "ensure_leading(var, prefix)",
            "ensure_trailing(var, postfix)",
            "replace(x, find, replacement)",
            "replace_first(x, find, replacement)",
            "replace_last(x, find, replacement)",
            "terminal(string)",
            // Phase 4 — filesystem functions
            "is_indexed_file(file)",
            "file_index(file)",
            "increment_file_index(file)",
            "decrement_file_index(file)",
            "basename(file)",
            "basename_without_index(file)",
            "dirname(file)",
            "ext(file)",
            "parent_dir(file)",
            "file_trailing(file)",
            "dir_leading(file)",
            "join(left, right)",
            // Phase 5 — link and skill functions
            "link(file)",
            "link(target, desc)",
            "has_skill(name)",
            "has_local_skill(name)",
        ];

        let descriptor_sigs: std::collections::HashSet<&str,
        > = expression_function_descriptors()
            .iter()
            .map(|d| d.signature)
            .collect();

        let missing: Vec<&str> = expected
            .iter()
            .copied()
            .filter(|sig| !descriptor_sigs.contains(sig))
            .collect();

        assert!(
            missing.is_empty(),
            "Feature function signatures missing from exported catalog: {missing:?}"
        );
    }
}

#[cfg(test)]
mod phase2_tests {
    use super::*;
    use crate::markdown::compose::expression::{evaluate, parse, EvaluationLookup, ResolutionContext};
    use serde_json::Value;

    struct FixtureLookup {
        ctx: ResolutionContext,
        data: std::collections::HashMap<String, Value>,
    }

    impl EvaluationLookup for FixtureLookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.data.get(path).cloned()
        }
        fn resolution_context(&self) -> Option<ResolutionContext> {
            Some(self.ctx.clone())
        }
    }

    fn make_fixture() -> (tempfile::TempDir, FixtureLookup) {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("fixture.md"),
            "---\ntitle: Fixture Title\n---\n# Fixture\n\nBody\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("note.md"), "plain\n").unwrap();
        std::fs::write(dir.path().join("review-1.md"), "").unwrap();
        std::fs::write(dir.path().join("review-2.md"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "").unwrap();
        let ctx = ResolutionContext::new(dir.path().to_path_buf());
        let mut data = std::collections::HashMap::new();
        data.insert("items".to_string(), serde_json::json!([1, 2, 3]));
        data.insert("obj".to_string(), serde_json::json!({"a": 1}));
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

    #[test]
    fn every_example_evaluates_to_its_declared_result() {
        use crate::catalog::ExampleVerification;
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for d in expression_function_descriptors() {
            let Some(example) = d.example() else { continue };
            // Only `Executable` examples are asserted to evaluate to their
            // declared result; display-only examples are illustrative and not
            // run.
            if example.verification != ExampleVerification::Executable {
                continue;
            }
            let expr = match parse(example.invocation) {
                Ok(e) => e,
                Err(err) => {
                    failures.push((d.signature, format!("parse error: {}", err.message)));
                    continue;
                }
            };
            let result = match evaluate(&expr, &lookup) {
                Ok(v) => render_value(&v),
                Err(err) => {
                    failures.push((d.signature, format!("eval error: {}", err)));
                    continue;
                }
            };
            if result != example.result {
                failures.push((
                    d.signature,
                    format!("got {:?}, expected {:?}", result, example.result),
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "expression examples did not evaluate to declared results: {failures:?}"
        );
    }
}

#[cfg(test)]
mod typed_signature_tests {
    use super::*;
    use crate::markdown::schemas::SimplifiedType;

    /// Every descriptor's typed signature renders and always names a return
    /// type; a fallible return carries the `| error` union member.
    #[test]
    fn every_descriptor_has_a_typed_signature() {
        for d in expression_function_descriptors() {
            let typed = d.typed_signature();
            assert!(
                typed.starts_with(d.signature.split('(').next().unwrap()),
                "typed signature `{typed}` should start with `{}`",
                d.signature
            );
            assert!(typed.contains(" -> "), "typed signature must show a return: {typed}");
            assert_eq!(
                d.returns.fallible,
                typed.contains("| error"),
                "`| error` presence must match the fallible flag for {}",
                d.signature
            );
        }
    }

    /// The six D4 list formatters are all present, take a single `any[]`, and
    /// return `string | error`.
    #[test]
    fn list_formatting_functions_are_typed() {
        let expected = [
            "as_line_separated(list)",
            "as_csv(list)",
            "as_tsv(list)",
            "as_space_separated(list)",
            "as_unordered_list(list)",
            "as_ordered_list(list)",
        ];
        for signature in expected {
            let d = expression_function_descriptors()
                .iter()
                .find(|d| d.signature == signature)
                .unwrap_or_else(|| panic!("missing list formatter: {signature}"));
            assert_eq!(d.category, "List Formatting");
            assert_eq!(d.parameters.len(), 1);
            assert_eq!(d.parameters[0].ty, DataType::Any);
            assert!(d.parameters[0].array, "list parameter must be an array");
            assert_eq!(d.returns.ty, DataType::String);
            assert!(d.returns.fallible, "list formatters are fallible");
        }
        let csv = expression_function_descriptors()
            .iter()
            .find(|d| d.signature == "as_csv(list)")
            .unwrap();
        assert_eq!(csv.typed_signature(), "as_csv(list: any[]) -> string | error");
    }

    /// `error` is a **return-position** anchor only: no function parameter may be
    /// typed as `error`, and `DataType` (the parameter/data domain) has no
    /// `error` keyword. `SimplifiedType` (the frontmatter validator) likewise
    /// knows neither `error` nor a function type, so a frontmatter property can
    /// never be typed as a function (spec D7, "catalog-only" typing).
    #[test]
    fn error_and_functions_never_leak_into_the_data_or_frontmatter_type_domains() {
        // No DataType keyword is `error` or a function signature.
        for dt in [
            DataType::String,
            DataType::Number,
            DataType::Integer,
            DataType::Boolean,
            DataType::Date,
            DataType::DateTime,
            DataType::Time,
            DataType::Object,
            DataType::File,
            DataType::Url,
            DataType::Email,
            DataType::Yaml,
            DataType::Json,
            DataType::Any,
        ] {
            let keyword = dt.as_keyword();
            assert_ne!(keyword, "error");
            assert!(!keyword.contains("->"), "no data type is a function signature");
        }
        // The frontmatter validator's type set knows neither.
        assert!(SimplifiedType::from_keyword("error").is_none());
        assert!(SimplifiedType::from_keyword("function").is_none());
    }
}

#[cfg(test)]
mod list_formatting_example_files {
    use crate::markdown::compose::expression::{EvaluationLookup, evaluate, parse};
    use serde_json::Value;
    use std::collections::HashMap;

    struct MapLookup {
        data: HashMap<String, Value>,
    }

    impl EvaluationLookup for MapLookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.data.get(path).cloned()
        }
    }

    fn render(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// Every schema-plus example file for a list formatter evaluates its
    /// `invocation` (with `parameters` bound as initial values) to the declared
    /// `returns` string — the verified-example requirement (spec E3 / task 7).
    #[test]
    fn example_files_evaluate_to_their_declared_returns() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest_dir
            .join("../features/_completed/2026-07-08-single-sourcing-schema/examples");
        let files = [
            "as_line_separated.yaml",
            "as_csv.yaml",
            "as_tsv.yaml",
            "as_space_separated.yaml",
            "as_unordered_list.yaml",
            "as_ordered_list.yaml",
        ];
        for file in files {
            let path = dir.join(file);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {file}: {e}"));
            let doc: Value = serde_yaml_ng::from_str(&text)
                .unwrap_or_else(|e| panic!("parse {file}: {e}"));

            let invocation = doc
                .get("invocation")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{file} missing string invocation"));
            let expected = doc
                .get("returns")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{file} missing string returns"));

            let mut data = HashMap::new();
            if let Some(params) = doc.get("parameters").and_then(Value::as_array) {
                for entry in params {
                    if let Some(obj) = entry.as_object() {
                        for (key, value) in obj {
                            data.insert(key.clone(), value.clone());
                        }
                    }
                }
            }

            let lookup = MapLookup { data };
            let expr = parse(invocation)
                .unwrap_or_else(|e| panic!("{file} parse `{invocation}`: {}", e.message));
            let got = render(
                &evaluate(&expr, &lookup)
                    .unwrap_or_else(|e| panic!("{file} eval `{invocation}`: {e}")),
            );
            assert_eq!(got, expected, "example {file} did not match its declared returns");
        }
    }
}
