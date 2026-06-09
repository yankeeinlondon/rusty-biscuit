use std::collections::HashMap;

/// Status of a predicate or directive when translated from Neovim to Tree
/// Hugger's tree-sitter Rust runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    /// Works identically in both runtimes.
    Native,
    /// Requires a named post-processing hook because the Rust runtime does not
    /// support it directly.
    RequiresHook,
    /// Not supported; consumers must not silently ignore it.
    Unsupported,
}

impl std::fmt::Display for CompatibilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "native"),
            Self::RequiresHook => write!(f, "requires-hook"),
            Self::Unsupported => write!(f, "unsupported"),
        }
    }
}

/// Describes how a single predicate or directive behaves in Tree Hugger.
#[derive(Debug, Clone)]
pub struct PredicateCompatibility {
    /// The predicate/directive name without the `#` prefix.
    pub name: String,
    /// Whether this is a predicate or a directive.
    pub kind: PredicateKind,
    /// Compatibility status in the Rust runtime.
    pub status: CompatibilityStatus,
    /// Description of what the predicate does upstream.
    pub upstream_description: String,
    /// Name of the post-processing hook, if `status` is `RequiresHook`.
    pub hook_name: Option<String>,
    /// Human-readable guidance for query authors.
    pub guidance: String,
}

/// Distinguishes predicates from directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateKind {
    Predicate,
    Directive,
}

impl std::fmt::Display for PredicateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Predicate => write!(f, "predicate"),
            Self::Directive => write!(f, "directive"),
        }
    }
}

/// Registry of known predicates and directives.
#[derive(Debug, Clone, Default)]
pub struct CompatibilityRegistry {
    entries: HashMap<String, PredicateCompatibility>,
}

impl CompatibilityRegistry {
    /// Creates a registry populated with all known predicates/directives.
    pub fn new() -> Self {
        let mut registry = Self {
            entries: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Looks up a predicate by name (without `#` prefix).
    pub fn get(&self, name: &str) -> Option<&PredicateCompatibility> {
        self.entries.get(name)
    }

    /// Returns all entries.
    pub fn all(&self) -> &HashMap<String, PredicateCompatibility> {
        &self.entries
    }

    /// Returns all unsupported predicates.
    pub fn unsupported(&self) -> Vec<&PredicateCompatibility> {
        self.entries
            .values()
            .filter(|e| e.status == CompatibilityStatus::Unsupported)
            .collect()
    }

    /// Returns all predicates that require a hook.
    pub fn requires_hook(&self) -> Vec<&PredicateCompatibility> {
        self.entries
            .values()
            .filter(|e| e.status == CompatibilityStatus::RequiresHook)
            .collect()
    }

    fn register(&mut self, entry: PredicateCompatibility) {
        self.entries.insert(entry.name.clone(), entry);
    }

    fn register_builtins(&mut self) {
        // --- Native predicates (tree-sitter Rust supports these) ---
        self.register(PredicateCompatibility {
            name: "eq".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "String equality".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "match".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "Regex match".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "lua-match".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "Lua pattern match".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "any-of".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "Match any of a set of strings".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "contains".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "String contains substring".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "any-contains".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::Native,
            upstream_description: "Any capture contains substring".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });

        // --- Directives ---
        self.register(PredicateCompatibility {
            name: "set".to_string(),
            kind: PredicateKind::Directive,
            status: CompatibilityStatus::Native,
            upstream_description: "Set arbitrary properties on a match".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "offset".to_string(),
            kind: PredicateKind::Directive,
            status: CompatibilityStatus::Native,
            upstream_description: "Offset capture range".to_string(),
            hook_name: None,
            guidance: "Use directly; fully supported.".to_string(),
        });

        // --- RequiresHook (Neovim-specific, needs translation) ---
        self.register(PredicateCompatibility {
            name: "has-parent".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::RequiresHook,
            upstream_description: "Check if node has a parent of given type".to_string(),
            hook_name: Some("postprocess_has_parent".to_string()),
            guidance: "Not supported natively. Use a post-query filter or rewrite the pattern to test the parent explicitly.".to_string(),
        });
        self.register(PredicateCompatibility {
            name: "is".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::RequiresHook,
            upstream_description: "Neovim-specific type test".to_string(),
            hook_name: Some("postprocess_is".to_string()),
            guidance: "Neovim-specific. Rewrite using standard predicates or post-process matches."
                .to_string(),
        });
        self.register(PredicateCompatibility {
            name: "has-ancestor".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::RequiresHook,
            upstream_description: "Check if node has an ancestor of given type".to_string(),
            hook_name: Some("postprocess_has_ancestor".to_string()),
            guidance: "Not supported natively. Use a post-query traversal or rewrite the pattern."
                .to_string(),
        });
        self.register(PredicateCompatibility {
            name: "not-has-ancestor".to_string(),
            kind: PredicateKind::Predicate,
            status: CompatibilityStatus::RequiresHook,
            upstream_description: "Check that node does not have an ancestor of given type"
                .to_string(),
            hook_name: Some("postprocess_not_has_ancestor".to_string()),
            guidance: "Not supported natively. Use a post-query traversal or rewrite the pattern."
                .to_string(),
        });

        // --- Unsupported (must not be silently dropped) ---
        self.register(PredicateCompatibility {
            name: "make-range".to_string(),
            kind: PredicateKind::Directive,
            status: CompatibilityStatus::Unsupported,
            upstream_description: "Create a range from two captures".to_string(),
            hook_name: None,
            guidance: "Not supported. Remove from query or restructure to avoid range creation."
                .to_string(),
        });
        self.register(PredicateCompatibility {
            name: "strip".to_string(),
            kind: PredicateKind::Directive,
            status: CompatibilityStatus::Unsupported,
            upstream_description: "Strip whitespace from capture text".to_string(),
            hook_name: None,
            guidance: "Not supported. Post-process captured text in Rust code instead.".to_string(),
        });
    }
}

/// Checks if a query text contains unsupported predicates.
///
/// Returns the list of unsupported predicate names found.
/// Extracts a predicate/directive name from `(#name? ...)` or `(#name! ...)`.
fn extract_predicate_name(line: &str, start: usize) -> Option<&str> {
    let rest = &line[start + 2..];
    // Predicates end with `?`, directives end with `!`
    let end = rest.find('?').or_else(|| rest.find('!'))?;
    let name = &rest[..end];
    let clean = name.trim();
    if clean.is_empty() { None } else { Some(clean) }
}

/// Checks if a query text contains unsupported predicates.
///
/// Returns the list of unsupported predicate names found.
pub fn find_unsupported_predicates(query_text: &str) -> Vec<String> {
    let registry = CompatibilityRegistry::new();
    let mut unsupported = Vec::new();

    for line in query_text.lines() {
        if let Some(start) = line.find("(#")
            && let Some(clean) = extract_predicate_name(line, start)
            && let Some(entry) = registry.get(clean)
            && entry.status == CompatibilityStatus::Unsupported
        {
            unsupported.push(clean.to_string());
        }
    }

    unsupported.dedup();
    unsupported
}

/// Checks if a query text contains predicates that require hooks.
///
/// Returns the list of hook-requiring predicate names and their hook names.
pub fn find_hook_predicates(query_text: &str) -> Vec<(String, String)> {
    let registry = CompatibilityRegistry::new();
    let mut hooks = Vec::new();

    for line in query_text.lines() {
        if let Some(start) = line.find("(#")
            && let Some(clean) = extract_predicate_name(line, start)
            && let Some(entry) = registry.get(clean)
            && entry.status == CompatibilityStatus::RequiresHook
            && let Some(hook) = &entry.hook_name
        {
            hooks.push((clean.to_string(), hook.clone()));
        }
    }

    hooks.dedup();
    hooks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_is_native() {
        let registry = CompatibilityRegistry::new();
        let eq = registry.get("eq").expect("eq? should be registered");
        assert_eq!(eq.status, CompatibilityStatus::Native);
        assert_eq!(eq.kind, PredicateKind::Predicate);
    }

    #[test]
    fn has_parent_requires_hook() {
        let registry = CompatibilityRegistry::new();
        let pred = registry
            .get("has-parent")
            .expect("has-parent? should be registered");
        assert_eq!(pred.status, CompatibilityStatus::RequiresHook);
        assert!(pred.hook_name.is_some());
    }

    #[test]
    fn make_range_is_unsupported() {
        let registry = CompatibilityRegistry::new();
        let pred = registry
            .get("make-range")
            .expect("make-range! should be registered");
        assert_eq!(pred.status, CompatibilityStatus::Unsupported);
        assert_eq!(pred.kind, PredicateKind::Directive);
    }

    #[test]
    fn finds_unsupported_in_query_text() {
        let text = r#"
(function_declaration
  (#make-range! "start" "end"))
"#;
        let found = find_unsupported_predicates(text);
        assert!(found.contains(&"make-range".to_string()));
    }

    #[test]
    fn finds_hook_predicates_in_query_text() {
        let text = r#"
(function_declaration
  (#has-parent? "program"))
"#;
        let found = find_hook_predicates(text);
        assert!(found.iter().any(|(name, _)| name == "has-parent"));
    }

    #[test]
    fn no_unsupported_in_simple_query() {
        let text = r#"
(call_expression
  function: (field_expression
    field: (field_identifier) @_method)
  (#eq? @_method "unwrap"))
"#;
        let found = find_unsupported_predicates(text);
        assert!(found.is_empty());
    }

    #[test]
    fn unsupported_list_is_nonempty() {
        let registry = CompatibilityRegistry::new();
        let unsupported = registry.unsupported();
        assert!(!unsupported.is_empty());
    }

    #[test]
    fn requires_hook_list_is_nonempty() {
        let registry = CompatibilityRegistry::new();
        let hooks = registry.requires_hook();
        assert!(!hooks.is_empty());
    }

    #[test]
    fn status_display() {
        assert_eq!(CompatibilityStatus::Native.to_string(), "native");
        assert_eq!(
            CompatibilityStatus::RequiresHook.to_string(),
            "requires-hook"
        );
        assert_eq!(CompatibilityStatus::Unsupported.to_string(), "unsupported");
    }
}
