//! Typed descriptor catalog for expression functions.
//!
//! Each [`ExpressionFunctionDescriptor`] describes a single callable function
//! available in Darkmatter expressions. The catalog is a static, compile-time
//! constant — constructing or reading it performs no host probes, no I/O, and
//! no runtime context capture.
use crate::catalog::{Described, Example};


/// Descriptor for a single expression function.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionFunctionDescriptor {

    /// Canonical snake_case signature (e.g., `is_string(x)`).
    pub signature: &'static str,
    /// Short description of the function's behavior.
    pub description: &'static str,
    /// Logical grouping category.
    pub category: &'static str,
    /// Stable display order within the category.
    pub order: usize,
    /// Optional verified example.
    pub example: Option<Example>,
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


/// All expression function descriptors, in display order.
pub const EXPRESSION_FUNCTION_DESCRIPTORS: &[ExpressionFunctionDescriptor] = &[
    // ── Type Predicates ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "is_string(x)",
        description: "Returns true when the value is a string.",
        category: "Type Predicates",
        order: 1,

        example: Some(Example { invocation: "is_string(\"hello\")", result: "true" }),
 },
    ExpressionFunctionDescriptor {

        signature: "is_number(x)",
        description: "Returns true when the value is a number.",
        category: "Type Predicates",
        order: 2,

        example: Some(Example { invocation: "is_number(42)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_array(x)",
        description: "Returns true when the value is an array.",
        category: "Type Predicates",
        order: 3,

        example: Some(Example { invocation: "is_array(items)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_null(x)",
        description: "Returns true when the value is null.",
        category: "Type Predicates",
        order: 4,

        example: Some(Example { invocation: "is_null(null)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_object(x)",
        description: "Returns true when the value is an object.",
        category: "Type Predicates",
        order: 5,

        example: Some(Example { invocation: "is_object(obj)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_empty(x)",
        description: "Returns true when the value is null, empty string, empty array, or empty object.",
        category: "Type Predicates",
        order: 6,

        example: Some(Example { invocation: "is_empty(\"\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_positive(val)",
        description: "Returns true when the coerced value is greater than zero.",
        category: "Type Predicates",
        order: 7,

        example: Some(Example { invocation: "is_positive(5)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_negative(val)",
        description: "Returns true when the coerced value is less than zero.",
        category: "Type Predicates",
        order: 8,

        example: Some(Example { invocation: "is_negative(-3)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_integer(val)",
        description: "Returns true when the value is a JSON number with no fractional component.",
        category: "Type Predicates",
        order: 9,

        example: Some(Example { invocation: "is_integer(7)", result: "true" }),

    },
    // ── Math ────────────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "min(a, b)",
        description: "Returns the smaller of two numbers.",
        category: "Math",
        order: 1,

        example: Some(Example { invocation: "min(2, 5)", result: "2" }),

    },
    ExpressionFunctionDescriptor {

        signature: "max(a, b)",
        description: "Returns the larger of two numbers.",
        category: "Math",
        order: 2,

        example: Some(Example { invocation: "max(2, 5)", result: "5" }),

    },
    ExpressionFunctionDescriptor {

        signature: "abs(x)",
        description: "Returns the absolute value of a number.",
        category: "Math",
        order: 3,

        example: Some(Example { invocation: "abs(-3)", result: "3" }),

    },
    // ── Collection ──────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "first(x)",
        description: "Returns the first element of an array, or null when empty.",
        category: "Collection",
        order: 1,

        example: Some(Example { invocation: "first(items)", result: "1" }),

    },
    ExpressionFunctionDescriptor {

        signature: "last(x)",
        description: "Returns the last element of an array, or null when empty.",
        category: "Collection",
        order: 2,

        example: Some(Example { invocation: "last(items)", result: "3" }),

    },
    // ── String Predicates ───────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "starts_with(x, find)",
        description: "Returns true when the string starts with the given prefix (case-sensitive).",
        category: "String Predicates",
        order: 1,

        example: Some(Example { invocation: "starts_with(\"hello\", \"he\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "ends_with(x, find)",
        description: "Returns true when the string ends with the given suffix (case-sensitive).",
        category: "String Predicates",
        order: 2,

        example: Some(Example { invocation: "ends_with(\"hello\", \"lo\")", result: "true" }),

    },
    // ── String Mutations ────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "lower(x)",
        description: "Converts a string to lowercase.",
        category: "String Mutations",
        order: 1,

        example: Some(Example { invocation: "lower(\"HELLO\")", result: "hello" }),

    },
    ExpressionFunctionDescriptor {

        signature: "upper(x)",
        description: "Converts a string to uppercase.",
        category: "String Mutations",
        order: 2,

        example: Some(Example { invocation: "upper(\"hello\")", result: "HELLO" }),

    },
    ExpressionFunctionDescriptor {

        signature: "capitalize(x)",
        description: "Capitalizes the first character of a string.",
        category: "String Mutations",
        order: 3,

        example: Some(Example { invocation: "capitalize(\"hello\")", result: "Hello" }),

    },
    ExpressionFunctionDescriptor {

        signature: "kebab_case(x)",
        description: "Converts a string to kebab-case.",
        category: "String Mutations",
        order: 4,

        example: Some(Example { invocation: "kebab_case(\"Hello World\")", result: "hello-world" }),

    },
    ExpressionFunctionDescriptor {

        signature: "snake_case(x)",
        description: "Converts a string to snake_case.",
        category: "String Mutations",
        order: 5,

        example: Some(Example { invocation: "snake_case(\"Hello World\")", result: "hello_world" }),

    },
    ExpressionFunctionDescriptor {

        signature: "camel_case(x)",
        description: "Converts a string to camelCase.",
        category: "String Mutations",
        order: 6,

        example: Some(Example { invocation: "camel_case(\"hello world\")", result: "helloWorld" }),

    },
    ExpressionFunctionDescriptor {

        signature: "pascal_case(x)",
        description: "Converts a string to PascalCase.",
        category: "String Mutations",
        order: 7,

        example: Some(Example { invocation: "pascal_case(\"hello world\")", result: "HelloWorld" }),

    },
    ExpressionFunctionDescriptor {

        signature: "title_case(x)",
        description: "Converts a string to Title Case.",
        category: "String Mutations",
        order: 8,

        example: Some(Example { invocation: "title_case(\"hello world\")", result: "Hello World" }),

    },
    ExpressionFunctionDescriptor {

        signature: "without_date(string)",
        description: "Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.",
        category: "String Mutations",
        order: 9,

        example: Some(Example { invocation: "without_date(\"Note 2024-06-15\")", result: "Note " }),

    },
    ExpressionFunctionDescriptor {

        signature: "ensure_leading(var, prefix)",
        description: "Ensures the string form of a value starts with a prefix.",
        category: "String Mutations",
        order: 10,

        example: Some(Example { invocation: "ensure_leading(\"world\", \"hello \")", result: "hello world" }),

    },
    ExpressionFunctionDescriptor {

        signature: "ensure_trailing(var, postfix)",
        description: "Ensures the string form of a value ends with a postfix.",
        category: "String Mutations",
        order: 11,

        example: Some(Example { invocation: "ensure_trailing(\"hello\", \" world\")", result: "hello world" }),

    },
    // ── Rendering ───────────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "terminal(string)",
        description: "Renders Prose markup to a terminal string with ANSI SGR sequences.",
        category: "Rendering",
        order: 1,

        example: Some(Example { invocation: "terminal(\"hello\")", result: "hello" }),

    },
    // ── Date Formatting ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "date(iso, fmt)",
        description: "Reformats an ISO date/datetime string into a named human format.",
        category: "Date Formatting",
        order: 1,

        example: Some(Example { invocation: "date(\"2024-06-15\", \"long\")", result: "Sat, June 15th, 2024" }),

    },
    // ── Date Validators (Strict) ────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "is_date(x)",
        description: "Returns true when the string is a valid ISO date (YYYY-MM-DD).",
        category: "Date Validators",
        order: 1,

        example: Some(Example { invocation: "is_date(\"2024-06-15\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_utc(x)",
        description: "Same as is_date (the format itself is timezone-agnostic).",
        category: "Date Validators",
        order: 2,

        example: Some(Example { invocation: "is_date_utc(\"2024-06-15\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_time(x)",
        description: "Returns true when the string is a valid ISO datetime.",
        category: "Date Validators",
        order: 3,

        example: Some(Example { invocation: "is_date_time(\"2024-06-15T12:30:00\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "is_date_time_utc(x)",
        description: "Same parse contract as is_date_time.",
        category: "Date Validators",
        order: 4,

        example: Some(Example { invocation: "is_date_time_utc(\"2024-06-15T12:30:00Z\")", result: "true" }),

    },
    // ── Date Validators (Relative) ──────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "is_today(x)",
        description: "Returns true when the date/datetime is today (local).",
        category: "Date Validators",
        order: 5,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_today_utc(x)",
        description: "Returns true when the date/datetime is today (UTC).",
        category: "Date Validators",
        order: 6,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_yesterday(x)",
        description: "Returns true when the date/datetime is yesterday (local).",
        category: "Date Validators",
        order: 7,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_yesterday_utc(x)",
        description: "Returns true when the date/datetime is yesterday (UTC).",
        category: "Date Validators",
        order: 8,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_tomorrow(x)",
        description: "Returns true when the date/datetime is tomorrow (local).",
        category: "Date Validators",
        order: 9,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_tomorrow_utc(x)",
        description: "Returns true when the date/datetime is tomorrow (UTC).",
        category: "Date Validators",
        order: 10,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_month(x)",
        description: "Returns true when the date/datetime is in the current month (local).",
        category: "Date Validators",
        order: 11,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_month_utc(x)",
        description: "Returns true when the date/datetime is in the current month (UTC).",
        category: "Date Validators",
        order: 12,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_year(x)",
        description: "Returns true when the date/datetime is in the current year (local).",
        category: "Date Validators",
        order: 13,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    ExpressionFunctionDescriptor {

        signature: "is_this_year_utc(x)",
        description: "Returns true when the date/datetime is in the current year (UTC).",
        category: "Date Validators",
        order: 14,

        example: None // display-only: Result depends on the current wall-clock date.,

    },
    // ── Core Operators (in evaluate_function) ───────────────────────
    ExpressionFunctionDescriptor {

        signature: "and(...)",
        description: "Logical AND of all arguments. Short-circuits on first falsy value.",
        category: "Logical",
        order: 1,

        example: Some(Example { invocation: "and(true, true)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "or(...)",
        description: "Logical OR of all arguments. Short-circuits on first truthy value.",
        category: "Logical",
        order: 2,

        example: Some(Example { invocation: "or(false, true)", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "has_key(obj, key)",
        description: "Returns true when the object contains the given key.",
        category: "Collection",
        order: 3,

        example: Some(Example { invocation: "has_key(obj, \"a\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "contains(haystack, needle)",
        description: "Returns true when haystack contains needle (array, object, or string).",
        category: "Collection",
        order: 4,

        example: Some(Example { invocation: "contains(\"hello\", \"ell\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "length(x)",
        description: "Returns the length of a string, array, or object.",
        category: "Collection",
        order: 5,

        example: Some(Example { invocation: "length(\"hello\")", result: "5" }),

    },
    ExpressionFunctionDescriptor {

        signature: "number(x, [default])",
        description: "Converts a value to a number, with an optional default.",
        category: "Type Conversion",
        order: 1,

        example: Some(Example { invocation: "number(\"42\")", result: "42" }),

    },
    ExpressionFunctionDescriptor {

        signature: "round(x, [default])",
        description: "Rounds a value to the nearest integer, with an optional default.",
        category: "Math",
        order: 4,

        example: Some(Example { invocation: "round(3.7)", result: "4" }),

    },
    // ── Filesystem Functions ────────────────────────────────────────
    ExpressionFunctionDescriptor {

        signature: "absolute(file)",
        description: "Resolves a file path to an absolute path.",
        category: "Filesystem",
        order: 1,

        example: None // display-only: Result depends on the absolute path of the resolution context, which is not portable.,

    },
    ExpressionFunctionDescriptor {

        signature: "relative(file)",
        description: "Returns a best-effort relative path from the document base directory.",
        category: "Filesystem",
        order: 2,

        example: Some(Example { invocation: "relative(\"fixture.md\")", result: "fixture.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_exists(file)",
        description: "Returns true when the file exists (local or remote URL).",
        category: "Filesystem",
        order: 3,

        example: Some(Example { invocation: "file_exists(\"fixture.md\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "frontmatter(file)",
        description: "Reads the frontmatter of a Markdown file as an object.",
        category: "Filesystem",
        order: 4,

        example: Some(Example { invocation: "frontmatter(\"fixture.md\")", result: "{\"title\":\"Fixture Title\"}" }),

    },
    ExpressionFunctionDescriptor {

        signature: "frontmatter(file, prop)",
        description: "Reads a single frontmatter property from a Markdown file.",
        category: "Filesystem",
        order: 5,

        example: Some(Example { invocation: "frontmatter(\"fixture.md\", \"title\")", result: "Fixture Title" }),

    },
    ExpressionFunctionDescriptor {

        signature: "markdown_body_empty(file)",
        description: "Returns true when the Markdown body has only whitespace.",
        category: "Filesystem",
        order: 6,

        example: Some(Example { invocation: "markdown_body_empty(\"fixture.md\")", result: "false" }),

    },
    ExpressionFunctionDescriptor {

        signature: "markdown_title(file)",
        description: "Returns the title from frontmatter or the first H1 heading.",
        category: "Filesystem",
        order: 7,

        example: Some(Example { invocation: "markdown_title(\"fixture.md\")", result: "Fixture Title" }),

    },
    ExpressionFunctionDescriptor {

        signature: "validate_schema(file)",
        description: "Validates a Markdown document against its declared schema.",
        category: "Filesystem",
        order: 8,

        example: Some(Example { invocation: "validate_schema(\"fixture.md\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "validate_schema(file, obj)",
        description: "Two-argument form accepted for forward compatibility.",
        category: "Filesystem",
        order: 9,

        example: None,

    },
    ExpressionFunctionDescriptor {

        signature: "is_indexed_file(file)",
        description: "Returns true when the filename stem matches the indexed grammar (base-NNN).",
        category: "Filesystem",
        order: 10,

        example: Some(Example { invocation: "is_indexed_file(\"review-1.md\")", result: "true" }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_index(file)",
        description: "Returns the parsed index suffix, or -1 when non-indexed.",
        category: "Filesystem",
        order: 11,

        example: Some(Example { invocation: "file_index(\"review-1.md\")", result: "1" }),

    },
    ExpressionFunctionDescriptor {

        signature: "increment_file_index(file)",
        description: "Increments the numeric index suffix, preserving zero-padding width.",
        category: "Filesystem",
        order: 12,

        example: Some(Example { invocation: "increment_file_index(\"review-1.md\")", result: "review-2.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "decrement_file_index(file)",
        description: "Decrements the numeric index suffix, clamped at 0.",
        category: "Filesystem",
        order: 13,

        example: Some(Example { invocation: "decrement_file_index(\"review-2.md\")", result: "review-1.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "basename(file)",
        description: "Returns the final path component including extension.",
        category: "Filesystem",
        order: 14,

        example: Some(Example { invocation: "basename(\"sub/note.md\")", result: "note.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "basename_without_index(file)",
        description: "Returns the basename with any indexed suffix removed from the stem.",
        category: "Filesystem",
        order: 15,

        example: Some(Example { invocation: "basename_without_index(\"review-1.md\")", result: "review.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "dir(file)",
        description: "Returns the directory portion of the display path.",
        category: "Filesystem",
        order: 16,

        example: Some(Example { invocation: "dir(\"sub/note.md\")", result: "sub" }),

    },
    ExpressionFunctionDescriptor {

        signature: "ext(file)",
        description: "Returns the final extension without the leading dot.",
        category: "Filesystem",
        order: 17,

        example: Some(Example { invocation: "ext(\"sub/note.md\")", result: "md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "parent_dir(file)",
        description: "Returns the directory segment immediately above the basename.",
        category: "Filesystem",
        order: 18,

        example: Some(Example { invocation: "parent_dir(\"sub/note.md\")", result: "sub" }),

    },
    ExpressionFunctionDescriptor {

        signature: "file_trailing(file)",
        description: "Returns the last directory segment plus the basename.",
        category: "Filesystem",
        order: 19,

        example: Some(Example { invocation: "file_trailing(\"sub/note.md\")", result: "sub/note.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "dir_leading(file)",
        description: "Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing).",
        category: "Filesystem",
        order: 20,

        example: Some(Example { invocation: "dir_leading(\"sub/note.md\")", result: "" }),

    },
    ExpressionFunctionDescriptor {

        signature: "join(left, right)",
        description: "Joins two path strings with normalized separators.",
        category: "Filesystem",
        order: 21,

        example: Some(Example { invocation: "join(\"sub\", \"note.md\")", result: "sub/note.md" }),

    },
    ExpressionFunctionDescriptor {

        signature: "link(file)",
        description: "Creates a Markdown link to a local file, using its relative path as the link text.",
        category: "Filesystem",
        order: 22,

        example: None // display-only: Result includes an absolute path, which is not portable.,

    },
    ExpressionFunctionDescriptor {

        signature: "link(target, desc)",
        description: "Creates a Markdown link to a local file or HTTP(S) URL with the given description.",
        category: "Filesystem",
        order: 23,

        example: None // display-only: Result includes an absolute destination path, which is not portable.,

    },
    ExpressionFunctionDescriptor {

        signature: "has_skill(name)",
        description: "Returns true when a skill directory exists in a user-scoped or local-scoped skill root.",
        category: "Context",
        order: 1,

        example: None // display-only: Depends on agent-specific skill roots outside the tempdir fixture.,

    },
    ExpressionFunctionDescriptor {

        signature: "has_local_skill(name)",
        description: "Returns true when a skill directory exists in a local-scoped skill root.",
        category: "Context",
        order: 2,

        example: None // display-only: Depends on agent-specific skill roots outside the tempdir fixture.,

    },
];

/// Returns all expression function descriptors in display order.
pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    EXPRESSION_FUNCTION_DESCRIPTORS
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
        evaluate(&expr, lookup).err()
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
            "terminal(string)",
            // Phase 4 — filesystem functions
            "is_indexed_file(file)",
            "file_index(file)",
            "increment_file_index(file)",
            "decrement_file_index(file)",
            "basename(file)",
            "basename_without_index(file)",
            "dir(file)",
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
        let (_dir, lookup) = make_fixture();
        let mut failures = Vec::new();
        for d in EXPRESSION_FUNCTION_DESCRIPTORS {
            let Some(example) = d.example() else { continue };
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

