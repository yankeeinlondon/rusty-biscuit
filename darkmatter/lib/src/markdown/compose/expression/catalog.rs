//! Typed descriptor catalog for expression functions.
//!
//! Each [`ExpressionFunctionDescriptor`] describes a single callable function
//! available in Darkmatter expressions. The catalog is a static, compile-time
//! constant — constructing or reading it performs no host probes, no I/O, and
//! no runtime context capture.

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
}

/// All expression function descriptors, in display order.
pub const EXPRESSION_FUNCTION_DESCRIPTORS: &[ExpressionFunctionDescriptor] = &[
    // ── Type Predicates ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "is_string(x)",
        description: "Returns true when the value is a string.",
        category: "Type Predicates",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "is_number(x)",
        description: "Returns true when the value is a number.",
        category: "Type Predicates",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "is_array(x)",
        description: "Returns true when the value is an array.",
        category: "Type Predicates",
        order: 3,
    },
    ExpressionFunctionDescriptor {
        signature: "is_null(x)",
        description: "Returns true when the value is null.",
        category: "Type Predicates",
        order: 4,
    },
    ExpressionFunctionDescriptor {
        signature: "is_object(x)",
        description: "Returns true when the value is an object.",
        category: "Type Predicates",
        order: 5,
    },
    ExpressionFunctionDescriptor {
        signature: "is_empty(x)",
        description: "Returns true when the value is null, empty string, empty array, or empty object.",
        category: "Type Predicates",
        order: 6,
    },
    // ── Math ────────────────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "min(a, b)",
        description: "Returns the smaller of two numbers.",
        category: "Math",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "max(a, b)",
        description: "Returns the larger of two numbers.",
        category: "Math",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "abs(x)",
        description: "Returns the absolute value of a number.",
        category: "Math",
        order: 3,
    },
    // ── Collection ──────────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "first(x)",
        description: "Returns the first element of an array, or null when empty.",
        category: "Collection",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "last(x)",
        description: "Returns the last element of an array, or null when empty.",
        category: "Collection",
        order: 2,
    },
    // ── String Predicates ───────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "starts_with(x, find)",
        description: "Returns true when the string starts with the given prefix (case-sensitive).",
        category: "String Predicates",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "ends_with(x, find)",
        description: "Returns true when the string ends with the given suffix (case-sensitive).",
        category: "String Predicates",
        order: 2,
    },
    // ── String Mutations ────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "lower(x)",
        description: "Converts a string to lowercase.",
        category: "String Mutations",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "upper(x)",
        description: "Converts a string to uppercase.",
        category: "String Mutations",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "capitalize(x)",
        description: "Capitalizes the first character of a string.",
        category: "String Mutations",
        order: 3,
    },
    ExpressionFunctionDescriptor {
        signature: "kebab_case(x)",
        description: "Converts a string to kebab-case.",
        category: "String Mutations",
        order: 4,
    },
    ExpressionFunctionDescriptor {
        signature: "snake_case(x)",
        description: "Converts a string to snake_case.",
        category: "String Mutations",
        order: 5,
    },
    ExpressionFunctionDescriptor {
        signature: "camel_case(x)",
        description: "Converts a string to camelCase.",
        category: "String Mutations",
        order: 6,
    },
    ExpressionFunctionDescriptor {
        signature: "pascal_case(x)",
        description: "Converts a string to PascalCase.",
        category: "String Mutations",
        order: 7,
    },
    ExpressionFunctionDescriptor {
        signature: "title_case(x)",
        description: "Converts a string to Title Case.",
        category: "String Mutations",
        order: 8,
    },
    // ── Date Formatting ─────────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "date(iso, fmt)",
        description: "Reformats an ISO date/datetime string into a named human format.",
        category: "Date Formatting",
        order: 1,
    },
    // ── Date Validators (Strict) ────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "is_date(x)",
        description: "Returns true when the string is a valid ISO date (YYYY-MM-DD).",
        category: "Date Validators",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "is_date_utc(x)",
        description: "Same as is_date (the format itself is timezone-agnostic).",
        category: "Date Validators",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "is_date_time(x)",
        description: "Returns true when the string is a valid ISO datetime.",
        category: "Date Validators",
        order: 3,
    },
    ExpressionFunctionDescriptor {
        signature: "is_date_time_utc(x)",
        description: "Same parse contract as is_date_time.",
        category: "Date Validators",
        order: 4,
    },
    // ── Date Validators (Relative) ──────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "is_today(x)",
        description: "Returns true when the date/datetime is today (local).",
        category: "Date Validators",
        order: 5,
    },
    ExpressionFunctionDescriptor {
        signature: "is_today_utc(x)",
        description: "Returns true when the date/datetime is today (UTC).",
        category: "Date Validators",
        order: 6,
    },
    ExpressionFunctionDescriptor {
        signature: "is_yesterday(x)",
        description: "Returns true when the date/datetime is yesterday (local).",
        category: "Date Validators",
        order: 7,
    },
    ExpressionFunctionDescriptor {
        signature: "is_yesterday_utc(x)",
        description: "Returns true when the date/datetime is yesterday (UTC).",
        category: "Date Validators",
        order: 8,
    },
    ExpressionFunctionDescriptor {
        signature: "is_tomorrow(x)",
        description: "Returns true when the date/datetime is tomorrow (local).",
        category: "Date Validators",
        order: 9,
    },
    ExpressionFunctionDescriptor {
        signature: "is_tomorrow_utc(x)",
        description: "Returns true when the date/datetime is tomorrow (UTC).",
        category: "Date Validators",
        order: 10,
    },
    ExpressionFunctionDescriptor {
        signature: "is_this_month(x)",
        description: "Returns true when the date/datetime is in the current month (local).",
        category: "Date Validators",
        order: 11,
    },
    ExpressionFunctionDescriptor {
        signature: "is_this_month_utc(x)",
        description: "Returns true when the date/datetime is in the current month (UTC).",
        category: "Date Validators",
        order: 12,
    },
    ExpressionFunctionDescriptor {
        signature: "is_this_year(x)",
        description: "Returns true when the date/datetime is in the current year (local).",
        category: "Date Validators",
        order: 13,
    },
    ExpressionFunctionDescriptor {
        signature: "is_this_year_utc(x)",
        description: "Returns true when the date/datetime is in the current year (UTC).",
        category: "Date Validators",
        order: 14,
    },
    // ── Core Operators (in evaluate_function) ───────────────────────
    ExpressionFunctionDescriptor {
        signature: "and(...)",
        description: "Logical AND of all arguments. Short-circuits on first falsy value.",
        category: "Logical",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "or(...)",
        description: "Logical OR of all arguments. Short-circuits on first truthy value.",
        category: "Logical",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "has_key(obj, key)",
        description: "Returns true when the object contains the given key.",
        category: "Collection",
        order: 3,
    },
    ExpressionFunctionDescriptor {
        signature: "contains(haystack, needle)",
        description: "Returns true when haystack contains needle (array, object, or string).",
        category: "Collection",
        order: 4,
    },
    ExpressionFunctionDescriptor {
        signature: "length(x)",
        description: "Returns the length of a string, array, or object.",
        category: "Collection",
        order: 5,
    },
    ExpressionFunctionDescriptor {
        signature: "number(x, [default])",
        description: "Converts a value to a number, with an optional default.",
        category: "Type Conversion",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "round(x, [default])",
        description: "Rounds a value to the nearest integer, with an optional default.",
        category: "Math",
        order: 4,
    },
    // ── Filesystem Functions ────────────────────────────────────────
    ExpressionFunctionDescriptor {
        signature: "absolute(file)",
        description: "Resolves a file path to an absolute path.",
        category: "Filesystem",
        order: 1,
    },
    ExpressionFunctionDescriptor {
        signature: "relative(file)",
        description: "Returns a best-effort relative path from the document base directory.",
        category: "Filesystem",
        order: 2,
    },
    ExpressionFunctionDescriptor {
        signature: "file_exists(file)",
        description: "Returns true when the file exists (local or remote URL).",
        category: "Filesystem",
        order: 3,
    },
    ExpressionFunctionDescriptor {
        signature: "frontmatter(file)",
        description: "Reads the frontmatter of a Markdown file as an object.",
        category: "Filesystem",
        order: 4,
    },
    ExpressionFunctionDescriptor {
        signature: "frontmatter(file, prop)",
        description: "Reads a single frontmatter property from a Markdown file.",
        category: "Filesystem",
        order: 5,
    },
    ExpressionFunctionDescriptor {
        signature: "markdown_body_empty(file)",
        description: "Returns true when the Markdown body has only whitespace.",
        category: "Filesystem",
        order: 6,
    },
    ExpressionFunctionDescriptor {
        signature: "markdown_title(file)",
        description: "Returns the title from frontmatter or the first H1 heading.",
        category: "Filesystem",
        order: 7,
    },
    ExpressionFunctionDescriptor {
        signature: "validate_schema(file)",
        description: "Validates a Markdown document against its declared schema.",
        category: "Filesystem",
        order: 8,
    },
    ExpressionFunctionDescriptor {
        signature: "validate_schema(file, obj)",
        description: "Two-argument form accepted for forward compatibility.",
        category: "Filesystem",
        order: 9,
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
        LAZY_OPERATOR_NAMES, dispatchable_signatures,
    };
    use crate::markdown::compose::expression::{
        EvaluationLookup, ResolutionContext, evaluate, parse,
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
                err.as_deref()
                    .map(|e| !e.contains("Unknown function"))
                    .unwrap_or(true),
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
}
