//! Typed descriptor catalog for expression functions.
//!
//! Each [`ExpressionFunctionDescriptor`] describes a single callable function
//! available in Darkmatter expressions. The catalog is a static, compile-time
//! constant — constructing or reading it performs no host probes, no I/O, and
//! no runtime context capture.
//!
//! Each descriptor now also carries a **typed signature** ([`ParamType`] per
//! parameter plus a [`ReturnType`]). The type vocabulary is the schema-plus
//! *data-type* domain ([`DataType`]) for parameters, and data types **plus the
//! `error` union member** (a [`ReturnType::fallible`] flag) for returns. This is
//! a **catalog-only** concern: [`DataType`] deliberately has **no** `error` or
//! function-type variant, and `error` is a return-position flag — never a
//! parameter type — so a frontmatter property can never be typed as a function
//! or as `error` (spec D7, schema-plus § Type domains).
use crate::catalog::{Described, Example, ExampleVerification};

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
#[derive(Debug, Clone, PartialEq)]
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


// Shared typed parameter lists, referenced by the descriptors below to keep
// each entry compact. Names are irrelevant here (they live in the signature);
// only the type shape is shared.
const P_ANY: &[ParamType] = &[ParamType::val(DataType::Any)];
const P_ANY2: &[ParamType] = &[ParamType::val(DataType::Any), ParamType::val(DataType::Any)];
const P_STRING: &[ParamType] = &[ParamType::val(DataType::String)];
const P_STRING2: &[ParamType] =
    &[ParamType::val(DataType::String), ParamType::val(DataType::String)];
const P_STRING3: &[ParamType] = &[
    ParamType::val(DataType::String),
    ParamType::val(DataType::String),
    ParamType::val(DataType::String),
];
const P_NUM: &[ParamType] = &[ParamType::val(DataType::Number)];
const P_NUM2: &[ParamType] = &[ParamType::val(DataType::Number), ParamType::val(DataType::Number)];
const P_LIST: &[ParamType] = &[ParamType::array(DataType::Any)];
const P_VARIADIC: &[ParamType] = &[ParamType::variadic(DataType::Any)];
const P_OBJ_STRING: &[ParamType] =
    &[ParamType::val(DataType::Object), ParamType::val(DataType::String)];
const P_NUM_CONV: &[ParamType] =
    &[ParamType::val(DataType::Any), ParamType::optional(DataType::Any)];
const P_ROUND: &[ParamType] =
    &[ParamType::val(DataType::Number), ParamType::optional(DataType::Number)];
const P_FILE: &[ParamType] = &[ParamType::val(DataType::File)];
const P_FILE_STRING: &[ParamType] =
    &[ParamType::val(DataType::File), ParamType::val(DataType::String)];
const P_FILE_OBJ: &[ParamType] =
    &[ParamType::val(DataType::File), ParamType::val(DataType::Object)];

// Shared typed returns.
const R_BOOL: ReturnType = ReturnType::plain(DataType::Boolean);
const R_BOOL_ERR: ReturnType = ReturnType::fallible(DataType::Boolean);
const R_NUM: ReturnType = ReturnType::plain(DataType::Number);
const R_NUM_ERR: ReturnType = ReturnType::fallible(DataType::Number);
const R_STRING_ERR: ReturnType = ReturnType::fallible(DataType::String);
const R_FILE_ERR: ReturnType = ReturnType::fallible(DataType::File);
const R_OBJ_ERR: ReturnType = ReturnType::fallible(DataType::Object);
const R_ANY_ERR: ReturnType = ReturnType::fallible(DataType::Any);

/// All expression function descriptors, in display order.
pub const EXPRESSION_FUNCTION_DESCRIPTORS: &[ExpressionFunctionDescriptor] = &[
    // ── Type Predicates ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "is_string(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is a string.",
        category: "Type Predicates",
        order: 1,

        example: Some(Example { invocation: "is_string(\"hello\")", result: "true", verification: ExampleVerification::Executable }),
 },
    ExpressionFunctionDescriptor {

        signature: "is_number(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is a number.",
        category: "Type Predicates",
        order: 2,

        example: Some(Example { invocation: "is_number(42)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_array(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is an array.",
        category: "Type Predicates",
        order: 3,

        example: Some(Example { invocation: "is_array(items)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_null(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is null.",
        category: "Type Predicates",
        order: 4,

        example: Some(Example { invocation: "is_null(null)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_object(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is an object.",
        category: "Type Predicates",
        order: 5,

        example: Some(Example { invocation: "is_object(obj)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_empty(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is null, empty string, empty array, or empty object.",
        category: "Type Predicates",
        order: 6,

        example: Some(Example { invocation: "is_empty(\"\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_positive(val)",
        parameters: P_ANY,
        returns: R_BOOL_ERR,
        description: "Returns true when the coerced value is greater than zero.",
        category: "Type Predicates",
        order: 7,

        example: Some(Example { invocation: "is_positive(5)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_negative(val)",
        parameters: P_ANY,
        returns: R_BOOL_ERR,
        description: "Returns true when the coerced value is less than zero.",
        category: "Type Predicates",
        order: 8,

        example: Some(Example { invocation: "is_negative(-3)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_integer(val)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the value is a JSON number with no fractional component.",
        category: "Type Predicates",
        order: 9,

        example: Some(Example { invocation: "is_integer(7)", result: "true", verification: ExampleVerification::Executable }),

    },
    // ── Math ────────────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "min(a, b)",
        parameters: P_NUM2,
        returns: R_NUM_ERR,
        description: "Returns the smaller of two numbers.",
        category: "Math",
        order: 1,

        example: Some(Example { invocation: "min(2, 5)", result: "2", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "max(a, b)",
        parameters: P_NUM2,
        returns: R_NUM_ERR,
        description: "Returns the larger of two numbers.",
        category: "Math",
        order: 2,

        example: Some(Example { invocation: "max(2, 5)", result: "5", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "abs(x)",
        parameters: P_NUM,
        returns: R_NUM_ERR,
        description: "Returns the absolute value of a number.",
        category: "Math",
        order: 3,

        example: Some(Example { invocation: "abs(-3)", result: "3", verification: ExampleVerification::Executable }),

    },
    // ── Collection ──────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "first(x)",
        parameters: P_LIST,
        returns: R_ANY_ERR,
        description: "Returns the first element of an array, or null when empty.",
        category: "Collection",
        order: 1,

        example: Some(Example { invocation: "first(items)", result: "1", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "last(x)",
        parameters: P_LIST,
        returns: R_ANY_ERR,
        description: "Returns the last element of an array, or null when empty.",
        category: "Collection",
        order: 2,

        example: Some(Example { invocation: "last(items)", result: "3", verification: ExampleVerification::Executable }),

    },
    // ── String Predicates ───────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "starts_with(x, find)",
        parameters: P_STRING2,
        returns: R_BOOL_ERR,
        description: "Returns true when the string starts with the given prefix (case-sensitive).",
        category: "String Predicates",
        order: 1,

        example: Some(Example { invocation: "starts_with(\"hello\", \"he\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "ends_with(x, find)",
        parameters: P_STRING2,
        returns: R_BOOL_ERR,
        description: "Returns true when the string ends with the given suffix (case-sensitive).",
        category: "String Predicates",
        order: 2,

        example: Some(Example { invocation: "ends_with(\"hello\", \"lo\")", result: "true", verification: ExampleVerification::Executable }),

    },
    // ── String Mutations ────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "lower(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to lowercase.",
        category: "String Mutations",
        order: 1,

        example: Some(Example { invocation: "lower(\"HELLO\")", result: "hello", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "upper(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to uppercase.",
        category: "String Mutations",
        order: 2,

        example: Some(Example { invocation: "upper(\"hello\")", result: "HELLO", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "capitalize(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Capitalizes the first character of a string.",
        category: "String Mutations",
        order: 3,

        example: Some(Example { invocation: "capitalize(\"hello\")", result: "Hello", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "kebab_case(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to kebab-case.",
        category: "String Mutations",
        order: 4,

        example: Some(Example { invocation: "kebab_case(\"Hello World\")", result: "hello-world", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "snake_case(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to snake_case.",
        category: "String Mutations",
        order: 5,

        example: Some(Example { invocation: "snake_case(\"Hello World\")", result: "hello_world", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "camel_case(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to camelCase.",
        category: "String Mutations",
        order: 6,

        example: Some(Example { invocation: "camel_case(\"hello world\")", result: "helloWorld", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "pascal_case(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to PascalCase.",
        category: "String Mutations",
        order: 7,

        example: Some(Example { invocation: "pascal_case(\"hello world\")", result: "HelloWorld", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "title_case(x)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Converts a string to Title Case.",
        category: "String Mutations",
        order: 8,

        example: Some(Example { invocation: "title_case(\"hello world\")", result: "Hello World", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "without_date(string)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.",
        category: "String Mutations",
        order: 9,

        example: Some(Example { invocation: "without_date(\"Note 2024-06-15\")", result: "Note ", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "ensure_leading(var, prefix)",
        parameters: P_ANY2,
        returns: R_STRING_ERR,
        description: "Ensures the string form of a value starts with a prefix.",
        category: "String Mutations",
        order: 10,

        example: Some(Example { invocation: "ensure_leading(\"world\", \"hello \")", result: "hello world", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "ensure_trailing(var, postfix)",
        parameters: P_ANY2,
        returns: R_STRING_ERR,
        description: "Ensures the string form of a value ends with a postfix.",
        category: "String Mutations",
        order: 11,

        example: Some(Example { invocation: "ensure_trailing(\"hello\", \" world\")", result: "hello world", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "replace(x, find, replacement)",
        parameters: P_STRING3,
        returns: R_STRING_ERR,
        description: "Replaces every literal occurrence of a substring; empty find is a no-op.",
        category: "String Mutations",
        order: 12,

        example: Some(Example { invocation: "replace(\"a.b.c\", \".\", \"/\")", result: "a/b/c", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "replace_first(x, find, replacement)",
        parameters: P_STRING3,
        returns: R_STRING_ERR,
        description: "Replaces the first literal occurrence of a substring; empty find is a no-op.",
        category: "String Mutations",
        order: 13,

        example: Some(Example { invocation: "replace_first(\"a.b.c\", \".\", \"/\")", result: "a/b.c", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "replace_last(x, find, replacement)",
        parameters: P_STRING3,
        returns: R_STRING_ERR,
        description: "Replaces the last literal occurrence of a substring; empty find is a no-op.",
        category: "String Mutations",
        order: 14,

        example: Some(Example { invocation: "replace_last(\"a.b.c\", \".\", \"/\")", result: "a.b/c", verification: ExampleVerification::Executable }),

    },
    // ── Rendering ───────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "terminal(string)",
        parameters: P_STRING,
        returns: R_STRING_ERR,
        description: "Renders Prose markup to a terminal string with ANSI SGR sequences.",
        category: "Rendering",
        order: 1,

        example: Some(Example { invocation: "terminal(\"hello\")", result: "hello", verification: ExampleVerification::Executable }),

    },
    // ── Date Formatting ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "date(iso, fmt)",
        parameters: P_STRING2,
        returns: R_STRING_ERR,
        description: "Reformats an ISO date/datetime string into a named human format.",
        category: "Date Formatting",
        order: 1,

        example: Some(Example { invocation: "date(\"2024-06-15\", \"long\")", result: "Sat, June 15th, 2024", verification: ExampleVerification::Executable }),

    },
    // ── Date Validators (Strict) ────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "is_date(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the string is a valid ISO date (YYYY-MM-DD).",
        category: "Date Validators",
        order: 1,

        example: Some(Example { invocation: "is_date(\"2024-06-15\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Same as is_date (the format itself is timezone-agnostic).",
        category: "Date Validators",
        order: 2,

        example: Some(Example { invocation: "is_date_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_time(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the string is a valid ISO datetime.",
        category: "Date Validators",
        order: 3,

        example: Some(Example { invocation: "is_date_time(\"2024-06-15T12:30:00\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_time_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Same parse contract as is_date_time.",
        category: "Date Validators",
        order: 4,

        example: Some(Example { invocation: "is_date_time_utc(\"2024-06-15T12:30:00Z\")", result: "true", verification: ExampleVerification::Executable }),

    },
    // ── Date Validators (Relative) ──────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "is_today(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is today (local).",
        category: "Date Validators",
        order: 5,

        example: Some(Example { invocation: "is_today(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_today_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is today (UTC).",
        category: "Date Validators",
        order: 6,

        example: Some(Example { invocation: "is_today_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_yesterday(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is yesterday (local).",
        category: "Date Validators",
        order: 7,

        example: Some(Example { invocation: "is_yesterday(\"2024-06-14\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_yesterday_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is yesterday (UTC).",
        category: "Date Validators",
        order: 8,

        example: Some(Example { invocation: "is_yesterday_utc(\"2024-06-14\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_tomorrow(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is tomorrow (local).",
        category: "Date Validators",
        order: 9,

        example: Some(Example { invocation: "is_tomorrow(\"2024-06-16\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_tomorrow_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is tomorrow (UTC).",
        category: "Date Validators",
        order: 10,

        example: Some(Example { invocation: "is_tomorrow_utc(\"2024-06-16\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_month(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is in the current month (local).",
        category: "Date Validators",
        order: 11,

        example: Some(Example { invocation: "is_this_month(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_month_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is in the current month (UTC).",
        category: "Date Validators",
        order: 12,

        example: Some(Example { invocation: "is_this_month_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_year(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is in the current year (local).",
        category: "Date Validators",
        order: 13,

        example: Some(Example { invocation: "is_this_year(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_year_utc(x)",
        parameters: P_ANY,
        returns: R_BOOL,
        description: "Returns true when the date/datetime is in the current year (UTC).",
        category: "Date Validators",
        order: 14,

        example: Some(Example { invocation: "is_this_year_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

    },
    // ── Core Operators (in evaluate_function) ───────────────────────
    ExpressionFunctionDescriptor {

        signature: "and(...)",
        parameters: P_VARIADIC,
        returns: R_BOOL,
        description: "Logical AND of all arguments. Short-circuits on first falsy value.",
        category: "Logical",
        order: 1,

        example: Some(Example { invocation: "and(true, true)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "or(...)",
        parameters: P_VARIADIC,
        returns: R_BOOL,
        description: "Logical OR of all arguments. Short-circuits on first truthy value.",
        category: "Logical",
        order: 2,

        example: Some(Example { invocation: "or(false, true)", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "has_key(obj, key)",
        parameters: P_OBJ_STRING,
        returns: R_BOOL_ERR,
        description: "Returns true when the object contains the given key.",
        category: "Collection",
        order: 3,

        example: Some(Example { invocation: "has_key(obj, \"a\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "contains(haystack, needle)",
        parameters: P_ANY2,
        returns: R_BOOL_ERR,
        description: "Returns true when haystack contains needle (array, object, or string).",
        category: "Collection",
        order: 4,

        example: Some(Example { invocation: "contains(\"hello\", \"ell\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "length(x)",
        parameters: P_ANY,
        returns: R_NUM_ERR,
        description: "Returns the length of a string, array, or object.",
        category: "Collection",
        order: 5,

        example: Some(Example { invocation: "length(\"hello\")", result: "5", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "number(x, [default])",
        parameters: P_NUM_CONV,
        returns: R_NUM_ERR,
        description: "Converts a value to a number, with an optional default.",
        category: "Type Conversion",
        order: 1,

        example: Some(Example { invocation: "number(\"42\")", result: "42", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "round(x, [default])",
        parameters: P_ROUND,
        returns: R_NUM,
        description: "Rounds a value to the nearest integer, with an optional default.",
        category: "Math",
        order: 4,

        example: Some(Example { invocation: "round(3.7)", result: "4", verification: ExampleVerification::Executable }),

    },
    // ── Filesystem Functions ────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "absolute(file)",
        parameters: P_FILE,
        returns: R_FILE_ERR,
        description: "Resolves a file path to an absolute path.",
        category: "Filesystem",
        order: 1,

        example: Some(Example { invocation: "absolute(\"fixture.md\")", result: "/path/to/fixture.md", verification: ExampleVerification::DisplayOnly("resolves to an absolute path of the resolution context, which is not portable") }),

    },
    ExpressionFunctionDescriptor {

        signature: "relative(file)",
        parameters: P_FILE,
        returns: R_FILE_ERR,
        description: "Returns a best-effort relative path from the document base directory.",
        category: "Filesystem",
        order: 2,

        example: Some(Example { invocation: "relative(\"fixture.md\")", result: "fixture.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_exists(file)",
        parameters: P_FILE,
        returns: R_BOOL_ERR,
        description: "Returns true when the file exists (local or remote URL).",
        category: "Filesystem",
        order: 3,

        example: Some(Example { invocation: "file_exists(\"fixture.md\")", result: "true", verification: ExampleVerification::Executable }),

    },
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
    ExpressionFunctionDescriptor {

        signature: "markdown_body_empty(file)",
        parameters: P_FILE,
        returns: R_BOOL_ERR,
        description: "Returns true when the Markdown body has only whitespace.",
        category: "Filesystem",
        order: 6,

        example: Some(Example { invocation: "markdown_body_empty(\"fixture.md\")", result: "false", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "markdown_title(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the title from frontmatter or the first H1 heading.",
        category: "Filesystem",
        order: 7,

        example: Some(Example { invocation: "markdown_title(\"fixture.md\")", result: "Fixture Title", verification: ExampleVerification::Executable }),

    },
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
    ExpressionFunctionDescriptor {

        signature: "is_indexed_file(file)",
        parameters: P_FILE,
        returns: R_BOOL_ERR,
        description: "Returns true when the filename stem matches the indexed grammar (base-NNN).",
        category: "Filesystem",
        order: 10,

        example: Some(Example { invocation: "is_indexed_file(\"review-1.md\")", result: "true", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_index(file)",
        parameters: P_FILE,
        returns: R_NUM_ERR,
        description: "Returns the parsed index suffix, or -1 when non-indexed.",
        category: "Filesystem",
        order: 11,

        example: Some(Example { invocation: "file_index(\"review-1.md\")", result: "1", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "increment_file_index(file)",
        parameters: P_FILE,
        returns: R_FILE_ERR,
        description: "Increments the numeric index suffix, preserving zero-padding width.",
        category: "Filesystem",
        order: 12,

        example: Some(Example { invocation: "increment_file_index(\"review-1.md\")", result: "review-2.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "decrement_file_index(file)",
        parameters: P_FILE,
        returns: R_FILE_ERR,
        description: "Decrements the numeric index suffix, clamped at 0.",
        category: "Filesystem",
        order: 13,

        example: Some(Example { invocation: "decrement_file_index(\"review-2.md\")", result: "review-1.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "basename(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the final path component including extension.",
        category: "Filesystem",
        order: 14,

        example: Some(Example { invocation: "basename(\"sub/note.md\")", result: "note.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "basename_without_index(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the basename with any indexed suffix removed from the stem.",
        category: "Filesystem",
        order: 15,

        example: Some(Example { invocation: "basename_without_index(\"review-1.md\")", result: "review.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "dirname(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the directory portion of the display path.",
        category: "Filesystem",
        order: 16,

        example: Some(Example { invocation: "dirname(\"sub/note.md\")", result: "sub", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "ext(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the final extension without the leading dot.",
        category: "Filesystem",
        order: 17,

        example: Some(Example { invocation: "ext(\"sub/note.md\")", result: "md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "parent_dir(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the directory segment immediately above the basename.",
        category: "Filesystem",
        order: 18,

        example: Some(Example { invocation: "parent_dir(\"sub/note.md\")", result: "sub", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_trailing(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the last directory segment plus the basename.",
        category: "Filesystem",
        order: 19,

        example: Some(Example { invocation: "file_trailing(\"sub/note.md\")", result: "sub/note.md", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "dir_leading(file)",
        parameters: P_FILE,
        returns: R_STRING_ERR,
        description: "Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing).",
        category: "Filesystem",
        order: 20,

        example: Some(Example { invocation: "dir_leading(\"sub/note.md\")", result: "", verification: ExampleVerification::Executable }),

    },
    ExpressionFunctionDescriptor {

        signature: "join(left, right)",
        parameters: P_STRING2,
        returns: R_STRING_ERR,
        description: "Joins two path strings with normalized separators.",
        category: "Filesystem",
        order: 21,

        example: Some(Example { invocation: "join(\"sub\", \"note.md\")", result: "sub/note.md", verification: ExampleVerification::Executable }),

    },
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
    ExpressionFunctionDescriptor {

        signature: "has_command(cmd)",
        parameters: P_STRING,
        returns: R_BOOL,
        description: "Returns true when the command is found on PATH or is an existing executable absolute path.",
        category: "Filesystem",
        order: 24,

        example: None,

    },
    ExpressionFunctionDescriptor {

        signature: "has_skill(name)",
        parameters: P_STRING,
        returns: R_BOOL,
        description: "Returns true when a skill directory exists in a user-scoped or local-scoped skill root.",
        category: "Context",
        order: 1,

        example: Some(Example { invocation: "has_skill(\"darkmatter\")", result: "true", verification: ExampleVerification::DisplayOnly("depends on agent-specific skill roots outside the tempdir fixture") }),

    },
    ExpressionFunctionDescriptor {

        signature: "has_local_skill(name)",
        parameters: P_STRING,
        returns: R_BOOL,
        description: "Returns true when a skill directory exists in a local-scoped skill root.",
        category: "Context",
        order: 2,

        example: Some(Example { invocation: "has_local_skill(\"darkmatter\")", result: "true", verification: ExampleVerification::DisplayOnly("depends on agent-specific skill roots outside the tempdir fixture") }),

    },
    // ── List Formatting ─────────────────────────────────────────────
    // Each takes an array and renders it to a string. Bare-array interpolation
    // is line-separated by default (Phase 5), so `as_line_separated` is the
    // explicit, no-op form. Multi-line / whitespace-sensitive renderings carry
    // `DisplayOnly` examples so the generated single-line doc table stays intact;
    // they are verified by the dedicated unit tests and the schema-plus example
    // files (`features/2026-07-08-single-sourcing-schema/examples/`).
    ExpressionFunctionDescriptor {
        signature: "as_line_separated(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Joins a list into a newline-separated string (the default bare-array rendering).",
        category: "List Formatting",
        order: 1,
        example: Some(Example { invocation: "as_line_separated(items)", result: "1\n2\n3", verification: ExampleVerification::DisplayOnly("multi-line output; verified via example file") }),
    },
    ExpressionFunctionDescriptor {
        signature: "as_csv(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Joins a list into a comma-separated string.",
        category: "List Formatting",
        order: 2,
        example: Some(Example { invocation: "as_csv(items)", result: "1, 2, 3", verification: ExampleVerification::Executable }),
    },
    ExpressionFunctionDescriptor {
        signature: "as_tsv(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Joins a list into a tab-separated string.",
        category: "List Formatting",
        order: 3,
        example: Some(Example { invocation: "as_tsv(items)", result: "1\t2\t3", verification: ExampleVerification::DisplayOnly("tab-delimited output; verified via example file") }),
    },
    ExpressionFunctionDescriptor {
        signature: "as_space_separated(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Joins a list into a space-separated string.",
        category: "List Formatting",
        order: 4,
        example: Some(Example { invocation: "as_space_separated(items)", result: "1 2 3", verification: ExampleVerification::Executable }),
    },
    ExpressionFunctionDescriptor {
        signature: "as_unordered_list(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
        category: "List Formatting",
        order: 5,
        example: Some(Example { invocation: "as_unordered_list(items)", result: "- 1\n- 2\n- 3", verification: ExampleVerification::DisplayOnly("multi-line Markdown list; verified via example file") }),
    },
    ExpressionFunctionDescriptor {
        signature: "as_ordered_list(list)",
        parameters: P_LIST,
        returns: R_STRING_ERR,
        description: "Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists.",
        category: "List Formatting",
        order: 6,
        example: Some(Example { invocation: "as_ordered_list(items)", result: "1. 1\n2. 2\n3. 3", verification: ExampleVerification::DisplayOnly("multi-line Markdown list; verified via example file") }),
    },
];

/// Returns all expression function descriptors in display order.
pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    EXPRESSION_FUNCTION_DESCRIPTORS
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
    for d in EXPRESSION_FUNCTION_DESCRIPTORS {
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
        dispatchable_signatures, LAZY_OPERATOR_NAMES,
    };
    use crate::markdown::compose::expression::{
        evaluate, parse, EvaluationLookup, ResolutionContext,
    };
    use serde_json::Value;
    use std::collections::HashSet;

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
        let descriptors: HashSet<&str> = EXPRESSION_FUNCTION_DESCRIPTORS
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
    /// presence in `LAZY_OPERATOR_NAMES` (and the parity set above) is real.
    #[test]
    fn lazy_operators_are_dispatchable() {
        let lookup = FsLookup {
            ctx: ResolutionContext::new(std::env::temp_dir()),
        };
        for name in LAZY_OPERATOR_NAMES {
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
        for desc in EXPRESSION_FUNCTION_DESCRIPTORS {
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
        let sigs: Vec<&str> = EXPRESSION_FUNCTION_DESCRIPTORS
            .iter()
            .map(|d| d.signature)
            .collect();
        let sigs_again: Vec<&str> = EXPRESSION_FUNCTION_DESCRIPTORS
            .iter()
            .map(|d| d.signature)
            .collect();
        assert_eq!(sigs, sigs_again);
    }

    #[test]
    fn descriptor_signatures_are_unique() {
        let mut seen = HashSet::new();
        for d in EXPRESSION_FUNCTION_DESCRIPTORS {
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
        for d in EXPRESSION_FUNCTION_DESCRIPTORS {
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
    /// Asserts against the shared [`EXPRESSION_FUNCTION_DESCRIPTORS`] so the
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
        > = EXPRESSION_FUNCTION_DESCRIPTORS
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
        for d in EXPRESSION_FUNCTION_DESCRIPTORS {
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
        for d in EXPRESSION_FUNCTION_DESCRIPTORS {
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
            let d = EXPRESSION_FUNCTION_DESCRIPTORS
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
        let csv = EXPRESSION_FUNCTION_DESCRIPTORS
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
