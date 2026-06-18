//! Shared descriptor framework for runtime-accessible metadata catalogs.
//!
//! The [`Described`] trait lets Darkmatter's static catalogs (context variables,
//! expression functions, side-effect capabilities) expose a uniform lookup and
//! suggestion surface. Consumers can perform exact lookups, fuzzy nearest-match
//! suggestions, and plain-text error enrichment without depending on any
//! specific catalog type.

/// One verified example for a described item.
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    /// Literal invocation shown to the user (e.g. `upper("hello")`).
    pub invocation: &'static str,
    /// Expected rendered result of the invocation.
    pub result: &'static str,
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
pub fn suggest<'a, D: Described>(catalog: &'a [D], key: &str, max: usize) -> Vec<&'a D> {
    if max == 0 {
        return Vec::new();
    }

    let normalized = normalize_key(key);
    let mut scored: Vec<(usize, usize, &'a D)> = catalog
        .iter()
        .map(|d| {
            let distance = levenshtein(&normalized, &normalize_key(d.key()));
            (distance, d.order(), d)
        })
        .collect();

    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(max).map(|(_, _, d)| d).collect()
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

fn levenshtein(a: &str, b: &str) -> usize {
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
        let results = suggest(TEST_CATALOG, "uper", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, "upper(x)");
        assert_eq!(results[1].key, "lower(x)");
    }

    #[test]
    fn suggest_tie_breaks_by_order() {
        let results = suggest(TEST_CATALOG, "strng", 2);
        assert_eq!(results[0].order, 1);
        assert_eq!(results[1].order, 2);
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
}
