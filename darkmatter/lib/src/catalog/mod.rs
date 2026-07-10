//! Shared descriptor framework for runtime-accessible metadata catalogs.
//!
//! The [`Described`] trait lets Darkmatter's static catalogs (context variables,
//! expression functions, side-effect capabilities) expose a uniform lookup and
//! suggestion surface. Consumers can perform exact lookups, fuzzy nearest-match
//! suggestions, and plain-text error enrichment without depending on any
//! specific catalog type.

/// One verified example for a described item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Example {
    /// Literal invocation shown to the user (e.g. `upper("hello")`).
    pub invocation: &'static str,
    /// Expected rendered result of the invocation.
    pub result: &'static str,
    /// Declares whether the example is machine-auditable, and if not, why.
    pub verification: ExampleVerification,
}

/// The verification intent of an [`Example`].
///
/// Distinguishes examples that an audit harness can execute and assert against
/// from those that are display-only. A display-only example is allowed but never
/// silent: it must carry a non-empty reason, so the opt-out is runtime-readable
/// metadata rather than a source comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExampleVerification {
    /// The invocation is executed and its rendered result is asserted to equal
    /// [`Example::result`].
    Executable,
    /// The result shape is illustrative only; live values are environment- or
    /// host-dependent, so equality is not asserted.
    TypeShapeOnly,
    /// The example is for display only and is never executed. The payload is a
    /// non-empty reason explaining why it cannot be machine-audited.
    DisplayOnly(&'static str),
}

/// Trait implemented by every static descriptor catalog in Darkmatter.
pub trait Described {
    /// Canonical lookup key (e.g. variable name or function signature).
    fn key(&self) -> &'static str;
    /// Short human-readable description.
    fn description(&self) -> &'static str;
    /// Logical grouping category.
    fn category(&self) -> &'static str;
    /// Stable display order within the category.
    fn order(&self) -> usize;
    /// Optional verified example.
    fn example(&self) -> Option<&Example>;
}

/// Exact lookup of a descriptor by key.
pub fn describe<'a, D: Described>(catalog: &'a [D], key: &str) -> Option<&'a D> {
    catalog.iter().find(|d| d.key() == key)
}

/// Fuzzy nearest-match suggestion.
///
/// Returns up to `max` descriptors sorted by Levenshtein distance to `key`,
/// tie-breaking by [`Described::order`]. `max == 0` yields an empty list.
/// Both the input key and catalog keys are normalized by stripping a leading
/// `ctx.` prefix and any parenthesized argument list before comparison.
///
/// A quality gate filters out candidates whose normalized distance is strictly
/// greater than `max(2, normalized_query_char_len / 3)`. This suppresses
/// confident-looking "did you mean" output for unrelated typos while still
/// catching common one- and two-character mistakes. As a result the returned
/// list may be shorter than `max`, including empty when nothing passes the gate.
pub fn suggest<'a, D: Described>(catalog: &'a [D], key: &str, max: usize) -> Vec<&'a D> {
    if max == 0 {
        return Vec::new();
    }

    let normalized = normalize_key(key);
    // Char count, not byte length: distances are computed over chars, and the
    // input may carry non-ASCII typo text even though catalog keys are ASCII.
    let threshold = (normalized.chars().count() / 3).max(2);
    let mut scored: Vec<(usize, usize, &'a D)> = catalog
        .iter()
        .map(|d| {
            let distance = levenshtein(&normalized, &normalize_key(d.key()));
            (distance, d.order(), d)
        })
        .filter(|(distance, _, _)| *distance <= threshold)
        .collect();

    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(max).map(|(_, _, d)| d).collect()
}

/// Fuzzy nearest-match suggestion over runtime strings.
///
/// The string sibling of [`suggest`]: it ranks plain `candidates` (e.g. sibling
/// filenames listed at render time) against `key` and returns up to `max` of
/// them, closest first. [`suggest`] cannot be reused for this because
/// [`Described::key`] is `&'static str` while filenames are runtime values.
///
/// The **same quality gate** as [`suggest`] applies — candidates whose distance
/// to `key` exceeds `max(2, key_char_len / 3)` are dropped — so the returned
/// list may be shorter than `max`, including empty when nothing is close enough.
/// Comparison is case-insensitive so a casing slip (`Spec.md` for `spec.md`)
/// still surfaces its near match. Ties break alphabetically for determinism
/// (runtime strings carry no [`Described::order`] to tie-break on).
pub fn suggest_strings<'a>(candidates: &'a [String], key: &str, max: usize) -> Vec<&'a str> {
    if max == 0 {
        return Vec::new();
    }

    let query = key.to_lowercase();
    let threshold = (query.chars().count() / 3).max(2);

    let mut scored: Vec<(usize, &'a str)> = candidates
        .iter()
        .map(|c| (levenshtein(&query, &c.to_lowercase()), c.as_str()))
        .filter(|(distance, _)| *distance <= threshold)
        .collect();

    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored.into_iter().take(max).map(|(_, c)| c).collect()
}

/// Plain-text formatter for error enrichment.
///
/// The output is plain text; terminal styling is the caller's responsibility.
pub fn describe_for_error<D: Described>(d: &D) -> String {
    let mut text = format!("{} — {}", d.key(), d.description());
    if let Some(example) = d.example() {
        text.push_str(&format!("\n  example: {} → {}", example.invocation, example.result));
    }
    text
}

fn normalize_key(key: &str) -> String {
    let key = key.strip_prefix("ctx.").unwrap_or(key);
    key.split('(')
        .next()
        .unwrap_or(key)
        .trim()
        .to_lowercase()
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (curr[j - 1] + 1)
                .min(prev[j] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestDescriptor {
        key: &'static str,
        description: &'static str,
        category: &'static str,
        order: usize,
        example: Option<Example>,
    }

    impl Described for TestDescriptor {
        fn key(&self) -> &'static str {
            self.key
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

    const UPPER_EXAMPLE: Example = Example {
        invocation: r#"upper("hello")"#,
        result: "HELLO",
        verification: ExampleVerification::Executable,
    };

    static TEST_CATALOG: &[TestDescriptor] = &[
        TestDescriptor {
            key: "upper(x)",
            description: "Converts a string to uppercase.",
            category: "String",
            order: 1,
            example: Some(UPPER_EXAMPLE),
        },
        TestDescriptor {
            key: "lower(x)",
            description: "Converts a string to lowercase.",
            category: "String",
            order: 2,
            example: None,
        },
        TestDescriptor {
            key: "today",
            description: "Local date in ISO-8601 format.",
            category: "Date",
            order: 3,
            example: None,
        },
    ];

    #[test]
    fn describe_returns_exact_match() {
        let found = describe(TEST_CATALOG, "upper(x)").expect("upper should be found");
        assert_eq!(found.key, "upper(x)");
    }

    #[test]
    fn describe_returns_none_for_miss() {
        assert!(describe(TEST_CATALOG, "not_there").is_none());
    }

    #[test]
    fn describe_requires_exact_signature() {
        assert!(describe(TEST_CATALOG, "upper").is_none());
        assert!(describe(TEST_CATALOG, "today").is_some());
    }

    #[test]
    fn suggest_ranks_by_distance() {
        // "uper" (4 chars) -> threshold max(2, 4/3) = 2. Only `upper(x)` (distance
        // 1) clears the quality gate; `lower(x)` (distance 3) is now correctly
        // filtered out as an unrelated match, so a single result is returned.
        let results = suggest(TEST_CATALOG, "uper", 2);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "upper(x)");
    }

    #[test]
    fn suggest_tie_breaks_by_order() {
        // "oper" sits at distance 2 from both `upper(x)` (order 1) and `lower(x)`
        // (order 2) — equal distance, within the threshold-2 gate — so the
        // deterministic order tie-break decides their ranking. (The previous
        // "strng" input fell outside the gate for every entry once it was added.)
        let results = suggest(TEST_CATALOG, "oper", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].order, 1);
        assert_eq!(results[1].order, 2);
    }

    #[test]
    fn suggest_omits_unrelated_typo() {
        // A long string unrelated to any catalog key must not produce a
        // confident-looking suggestion; the gate yields an empty list rather
        // than the first `max` entries.
        let results = suggest(TEST_CATALOG, "xxxxxxxxxxxx", 3);
        assert!(results.is_empty(), "unrelated typo should yield no suggestion");
    }

    #[test]
    fn suggest_keeps_common_short_typo() {
        // A common one-character mistake still surfaces the close match: "uper"
        // (distance 1) is well within the threshold-2 gate for `upper(x)`.
        let results = suggest(TEST_CATALOG, "uper", 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "upper(x)");
    }

    #[test]
    fn suggest_threshold_boundary_includes_and_excludes() {
        // "upwe" (4 chars) -> threshold max(2, 4/3) = 2.
        //   `upper(x)` distance == 2 (== threshold)      -> INCLUDED
        //   `lower(x)` distance == 3 (== threshold + 1)   -> EXCLUDED
        //   `today`    distance == 5                       -> EXCLUDED
        // A single call proves both edges of the gate by membership.
        let results = suggest(TEST_CATALOG, "upwe", 3);
        assert_eq!(results.len(), 1, "only the at-threshold candidate passes");
        assert_eq!(results[0].key, "upper(x)");
    }

    #[test]
    fn suggest_max_zero_returns_empty() {
        let results = suggest(TEST_CATALOG, "upper", 0);
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_strips_parentheses_from_input() {
        let with_parens = suggest(TEST_CATALOG, "upper(x)", 1);
        let without_parens = suggest(TEST_CATALOG, "upper", 1);
        assert_eq!(with_parens.len(), 1);
        assert_eq!(without_parens.len(), 1);
        assert_eq!(with_parens[0].key, without_parens[0].key);
    }

    #[test]
    fn suggest_strips_ctx_prefix() {
        let with_prefix = suggest(TEST_CATALOG, "ctx.today", 1);
        let without_prefix = suggest(TEST_CATALOG, "today", 1);
        assert_eq!(with_prefix.len(), 1);
        assert_eq!(without_prefix.len(), 1);
        assert_eq!(with_prefix[0].key, without_prefix[0].key);
    }

    #[test]
    fn suggest_strings_ranks_by_distance() {
        let candidates = vec![
            "spec.md".to_string(),
            "specs.md".to_string(),
            "readme.md".to_string(),
        ];
        // "spec.md" (7 chars) -> threshold max(2, 7/3) = 2. The exact match has
        // distance 0 and ranks first; "specs.md" (distance 1) clears the gate;
        // "readme.md" (distance 5) is filtered out.
        let results = suggest_strings(&candidates, "spec.md", 3);
        assert_eq!(results, vec!["spec.md", "specs.md"]);
    }

    #[test]
    fn suggest_strings_near_name_passes_gate() {
        // The calibration case from the design: a missing `specs.md` next to a
        // real `spec.md` (distance 1) must surface the near match.
        let candidates = vec!["spec.md".to_string(), "plan.md".to_string()];
        let results = suggest_strings(&candidates, "specs.md", 1);
        assert_eq!(results, vec!["spec.md"]);
    }

    #[test]
    fn suggest_strings_is_case_insensitive() {
        let candidates = vec!["Spec.md".to_string()];
        let results = suggest_strings(&candidates, "spec.md", 1);
        assert_eq!(results, vec!["Spec.md"]);
    }

    #[test]
    fn suggest_strings_omits_unrelated() {
        let candidates = vec!["completely-different-name.txt".to_string()];
        let results = suggest_strings(&candidates, "spec.md", 3);
        assert!(results.is_empty(), "unrelated candidate must not be suggested");
    }

    #[test]
    fn suggest_strings_tie_breaks_alphabetically() {
        // "abc.md" and "bbc.md" are both distance 1 from "xbc.md"; with no
        // `order` to consult, the alphabetical tie-break decides.
        let candidates = vec!["bbc.md".to_string(), "abc.md".to_string()];
        let results = suggest_strings(&candidates, "xbc.md", 2);
        assert_eq!(results, vec!["abc.md", "bbc.md"]);
    }

    #[test]
    fn suggest_strings_max_zero_and_empty_candidates() {
        let candidates = vec!["spec.md".to_string()];
        assert!(suggest_strings(&candidates, "spec.md", 0).is_empty());
        assert!(suggest_strings(&[], "spec.md", 3).is_empty());
    }

    #[test]
    fn describe_for_error_includes_key_and_description() {
        let d = &TEST_CATALOG[0];
        let text = describe_for_error(d);
        assert!(text.contains("upper(x)"));
        assert!(text.contains("Converts a string to uppercase."));
    }

    #[test]
    fn describe_for_error_includes_example_when_present() {
        let d = &TEST_CATALOG[0];
        let text = describe_for_error(d);
        assert!(text.contains(r#"upper("hello")"#));
        assert!(text.contains("HELLO"));
    }

    #[test]
    fn describe_for_error_omits_example_when_absent() {
        let d = &TEST_CATALOG[1];
        let text = describe_for_error(d);
        assert!(!text.contains("example:"));
    }

    /// `describe_for_error` still renders `invocation → result`, independent of
    /// the new verification intent on the example.
    #[test]
    fn describe_for_error_renders_invocation_to_result_regardless_of_verification() {
        let display_only = TestDescriptor {
            key: "now()",
            description: "Current time.",
            category: "Date",
            order: 1,
            example: Some(Example {
                invocation: "now()",
                result: "10:30",
                verification: ExampleVerification::DisplayOnly("wall-clock dependent"),
            }),
        };
        let text = describe_for_error(&display_only);
        assert!(text.contains("now()"));
        assert!(text.contains("10:30"));
        assert!(text.contains("→"));
    }

    /// Generic invariant: every `DisplayOnly` example in EVERY shipped catalog
    /// carries a non-empty reason string. A silent (empty-reason) opt-out is a
    /// documentation hole, so the audit fails the build rather than shipping it.
    #[test]
    fn every_display_only_example_carries_a_non_empty_reason() {
        fn check<D: Described>(catalog: &[D], label: &str, failures: &mut Vec<String>) {
            for d in catalog {
                if let Some(Example {
                    verification: ExampleVerification::DisplayOnly(reason),
                    ..
                }) = d.example()
                    && reason.trim().is_empty()
                {
                    failures.push(format!("{label}: `{}` has an empty DisplayOnly reason", d.key()));
                }
            }
        }

        let mut failures = Vec::new();
        check(
            &crate::markdown::compose::context::catalog::CONTEXT_VARIABLE_DESCRIPTORS,
            "context",
            &mut failures,
        );
        check(
            crate::markdown::compose::expression::expression_function_descriptors(),
            "expression",
            &mut failures,
        );
        check(crate::effects::catalog::EFFECT_DESCRIPTORS, "effects", &mut failures);

        use crate::markdown::compose::expression::semantics::*;
        check(OPERATOR_DESCRIPTORS, "semantics::operator", &mut failures);
        check(TRUTHINESS_DESCRIPTORS, "semantics::truthiness", &mut failures);
        check(MODE_DESCRIPTORS, "semantics::mode", &mut failures);
        check(NULL_PROPAGATION_DESCRIPTORS, "semantics::null_propagation", &mut failures);
        check(UNARY_OPERATOR_DESCRIPTORS, "semantics::unary", &mut failures);
        check(COMPARISON_OPERATOR_DESCRIPTORS, "semantics::comparison", &mut failures);
        check(ARITHMETIC_OPERATOR_DESCRIPTORS, "semantics::arithmetic", &mut failures);
        check(VARIABLE_ACCESS_DESCRIPTORS, "semantics::variable_access", &mut failures);

        assert!(
            failures.is_empty(),
            "display-only examples with empty reasons: {failures:?}"
        );
    }
}
