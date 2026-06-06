use std::collections::HashMap;
use std::fmt;

use crate::shared::{
    DiagnosticCategory, DiagnosticConfidence, DiagnosticMetadata, DiagnosticSeverity,
    DiagnosticSource, ProgrammingLanguage,
};

/// Metadata for a single lint or semantic rule.
///
/// Rule metadata is registered by the built-in registry and validated at
/// startup. It includes enough information for consumers to understand rule
/// identity, category, confidence, source, and effective severity.
#[derive(Debug, Clone)]
pub struct RuleMetadata {
    /// Unique rule identifier (e.g., "unwrap-call", "undefined-symbol").
    pub id: String,
    /// Semantic version of the rule (e.g., "1.0.0").
    pub version: String,
    /// Human-readable title.
    pub title: String,
    /// Category for policy grouping.
    pub category: DiagnosticCategory,
    /// Default severity when no policy overrides it.
    pub default_severity: DiagnosticSeverity,
    /// Confidence level in findings from this rule.
    pub confidence: DiagnosticConfidence,
    /// Which languages this rule supports.
    pub languages: Vec<ProgrammingLanguage>,
    /// Whether this rule is enabled by default.
    pub enabled_by_default: bool,
    /// Whether this rule requires `--experimental-semantics`.
    pub requires_experimental_semantics: bool,
    /// Example code that triggers this rule.
    pub examples: Vec<RuleExample>,
    /// Known limitations or caveats.
    pub caveats: Vec<String>,
    /// Whether this rule needs project context to be accurate.
    pub needs_project_context: bool,
    /// Aliases for this rule ID (e.g., older names).
    pub aliases: Vec<String>,
}

/// An example showing a rule trigger and (optionally) a fix.
#[derive(Debug, Clone)]
pub struct RuleExample {
    /// Description of what the example demonstrates.
    pub description: String,
    /// Code that triggers the rule.
    pub code: String,
    /// Fixed version, if applicable.
    pub fixed: Option<String>,
    /// Language of the example code.
    pub language: ProgrammingLanguage,
}

/// Registry of all known rule metadata.
///
/// The registry is built at initialization from built-in rule definitions.
/// It supports lookup by rule ID and validation of rule sets.
#[derive(Debug, Clone)]
pub struct RuleRegistry {
    rules: HashMap<String, RuleMetadata>,
    alias_map: HashMap<String, String>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleRegistry {
    /// Creates a new registry populated with built-in rules.
    pub fn new() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
            alias_map: HashMap::new(),
        };
        registry.register_builtin_rules();
        registry
    }

    /// Looks up rule metadata by ID.
    pub fn get(&self, rule_id: &str) -> Option<&RuleMetadata> {
        self.rules.get(rule_id).or_else(|| {
            self.alias_map
                .get(rule_id)
                .and_then(|canonical| self.rules.get(canonical))
        })
    }

    /// Returns metadata for a rule, or default metadata if unknown.
    pub fn get_or_default(&self, rule_id: &str) -> RuleMetadata {
        self.get(rule_id).cloned().unwrap_or_else(|| {
            RuleMetadata {
                id: rule_id.to_string(),
                version: "0.0.0".to_string(),
                title: format!("Lint rule: {rule_id}"),
                category: DiagnosticCategory::Suspicious,
                default_severity: DiagnosticSeverity::Warning,
                confidence: DiagnosticConfidence::Medium,
                languages: ProgrammingLanguage::all(),
                enabled_by_default: true,
                requires_experimental_semantics: false,
                examples: Vec::new(),
                caveats: Vec::new(),
                needs_project_context: false,
                aliases: Vec::new(),
            }
        })
    }

    /// Returns true if the rule is known (including aliases).
    pub fn has_rule(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id) || self.alias_map.contains_key(rule_id)
    }

    /// Returns all registered rule IDs.
    pub fn rule_ids(&self) -> impl Iterator<Item = &String> {
        self.rules.keys()
    }

    /// Returns rules in a given category.
    pub fn rules_in_category(&self, category: DiagnosticCategory) -> Vec<&RuleMetadata> {
        self.rules
            .values()
            .filter(|r| r.category == category)
            .collect()
    }

    /// Returns rules enabled by default.
    pub fn default_enabled_rules(&self) -> Vec<&RuleMetadata> {
        self.rules
            .values()
            .filter(|r| r.enabled_by_default)
            .collect()
    }

    /// Validates the registry for consistency.
    ///
    /// Checks for:
    /// - Duplicate rule IDs
    /// - Dangling aliases (alias pointing to non-existent rule)
    /// - Unknown categories
    /// - Undocumented default-on rules (rules with no examples)
    pub fn validate(&self) -> Result<(), Vec<RuleRegistryError>> {
        let mut errors = Vec::new();

        // Check for dangling aliases
        for (alias, canonical) in &self.alias_map {
            if !self.rules.contains_key(canonical) {
                errors.push(RuleRegistryError::DanglingAlias {
                    alias: alias.clone(),
                    target: canonical.clone(),
                });
            }
        }

        // Check for undocumented default-on rules
        for rule in self.rules.values() {
            if rule.enabled_by_default && rule.examples.is_empty() {
                errors.push(RuleRegistryError::UndocumentedDefaultRule {
                    rule_id: rule.id.clone(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Registers a rule in the registry.
    fn register_rule(&mut self, rule: RuleMetadata) {
        for alias in &rule.aliases {
            self.alias_map.insert(alias.clone(), rule.id.clone());
        }
        self.rules.insert(rule.id.clone(), rule);
    }

    /// Populates the registry with built-in rules.
    fn register_builtin_rules(&mut self) {
        // Syntax rules
        self.register_rule(RuleMetadata {
            id: "invalid-syntax".to_string(),
            version: "1.0.0".to_string(),
            title: "Invalid syntax".to_string(),
            category: DiagnosticCategory::Correctness,
            default_severity: DiagnosticSeverity::Error,
            confidence: DiagnosticConfidence::High,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "Missing semicolon".to_string(),
                code: "let x = 1".to_string(),
                fixed: Some("let x = 1;".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec!["Syntax errors may cascade and produce secondary diagnostics.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        // Pattern-based rules (mature, high-confidence)
        self.register_rule(RuleMetadata {
            id: "unwrap-call".to_string(),
            version: "1.0.0".to_string(),
            title: "Explicit unwrap() call".to_string(),
            // Restriction, not Suspicious: an explicit unwrap() is a deliberate
            // "panic on None/Err" choice, not an anomaly. Off by default; opt in
            // with `--warn unwrap-call` / `--deny unwrap-call`.
            category: DiagnosticCategory::Restriction,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Rust],
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "Unwrap on Option".to_string(),
                code: "let x = Some(1);\nlet _ = x.unwrap();".to_string(),
                fixed: Some("let x = Some(1);\nlet _ = x.unwrap_or(0);".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec!["unwrap() is idiomatic in tests and quick prototypes.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "expect-call".to_string(),
            version: "1.0.0".to_string(),
            title: "Explicit expect() call".to_string(),
            // Restriction, not Suspicious: expect() is a deliberate panic with a
            // documented reason. Off by default; opt in with `--warn`/`--deny`.
            category: DiagnosticCategory::Restriction,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Rust],
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "Expect on Result".to_string(),
                code: "let x: Result<i32, _> = Ok(1);\nlet _ = x.expect(\"should work\");".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec!["expect() with a descriptive message is acceptable in main binaries.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "dbg-macro".to_string(),
            version: "1.0.0".to_string(),
            title: "Debug macro dbg!() call".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Rust],
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "dbg! macro in production code".to_string(),
                code: "dbg!(value);".to_string(),
                fixed: Some("println!(\"{:?}\", value);".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec!["dbg! is fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "eval-call".to_string(),
            version: "1.0.0".to_string(),
            title: "Use of eval()".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::JavaScript, ProgrammingLanguage::TypeScript, ProgrammingLanguage::Php],
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "eval with string".to_string(),
                code: "eval('console.log(1)');".to_string(),
                fixed: None,
                language: ProgrammingLanguage::JavaScript,
            }],
            caveats: vec!["Sometimes used in build tools and bundlers.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "exec-call".to_string(),
            version: "1.0.0".to_string(),
            title: "Use of exec()".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Python, ProgrammingLanguage::Php],
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "exec with string".to_string(),
                code: "exec('x = 1')".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Python,
            }],
            caveats: vec!["Sometimes used in dynamic code generation.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "debugger-statement".to_string(),
            version: "1.0.0".to_string(),
            title: "Debugger statement found".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::JavaScript, ProgrammingLanguage::TypeScript],
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "debugger statement in source".to_string(),
                code: "function foo() {\n  debugger;\n}".to_string(),
                fixed: Some("function foo() {\n  // debugger removed\n}".to_string()),
                language: ProgrammingLanguage::JavaScript,
            }],
            caveats: vec!["Fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "breakpoint-call".to_string(),
            version: "1.0.0".to_string(),
            title: "Breakpoint call found".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Python],
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "breakpoint() in source".to_string(),
                code: "def foo():\n    breakpoint()\n    return 1".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Python,
            }],
            caveats: vec!["Fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        // Debug artifact rules (default-off)
        self.register_rule(RuleMetadata {
            id: "console-log".to_string(),
            version: "1.0.0".to_string(),
            title: "console.log() call found".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::JavaScript, ProgrammingLanguage::TypeScript],
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "console.log in production code".to_string(),
                code: "console.log('debug');".to_string(),
                fixed: None,
                language: ProgrammingLanguage::JavaScript,
            }],
            caveats: vec!["Fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "print-call".to_string(),
            version: "1.0.0".to_string(),
            title: "print() call found".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Python],
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "print in production code".to_string(),
                code: "print('debug')".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Python,
            }],
            caveats: vec!["Fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "fmt-println".to_string(),
            version: "1.0.0".to_string(),
            title: "fmt.Println() call found".to_string(),
            category: DiagnosticCategory::Suspicious,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::High,
            languages: vec![ProgrammingLanguage::Go],
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "fmt.Println in production code".to_string(),
                code: "fmt.Println(\"debug\")".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Go,
            }],
            caveats: vec!["Fine during active development.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        // Semantic rules (experimental, gated)
        self.register_rule(RuleMetadata {
            id: "undefined-symbol".to_string(),
            version: "1.0.0".to_string(),
            title: "Reference to undefined symbol".to_string(),
            category: DiagnosticCategory::Correctness,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: true,
            examples: vec![RuleExample {
                description: "Reference to unknown variable".to_string(),
                code: "fn main() {\n    let _ = missing_value;\n}".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec![
                "False positives for symbols from external crates, macros, and generated code.".to_string(),
                "Does not understand module boundaries or re-exports.".to_string(),
            ],
            needs_project_context: true,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "unused-symbol".to_string(),
            version: "1.0.0".to_string(),
            title: "Symbol defined but never used".to_string(),
            category: DiagnosticCategory::Style,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: true,
            examples: vec![RuleExample {
                description: "Unused local variable".to_string(),
                code: "fn main() {\n    let unused = 1;\n}".to_string(),
                fixed: Some("fn main() {\n    // variable removed\n}".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec![
                "False positives for public API surfaces and symbols used via reflection.".to_string(),
            ],
            needs_project_context: true,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "unused-import".to_string(),
            version: "1.0.0".to_string(),
            title: "Imported symbol is never used".to_string(),
            category: DiagnosticCategory::Style,
            default_severity: DiagnosticSeverity::Warning,
            // Without trait/type resolution a syntactic scan cannot see imports
            // used only through trait methods (`Write` for `.write_all()`) or
            // macros, so it has an irreducible false-positive rate. Low
            // confidence, off by default; opt in with `--warn unused-import`.
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "Unused import".to_string(),
                code: "use std::fs;\n\nfn main() {}".to_string(),
                fixed: Some("fn main() {}".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec![
                "Flags imports used only via trait methods or macros, or for side effects, as false positives.".to_string(),
            ],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "undefined-module".to_string(),
            version: "1.0.0".to_string(),
            title: "Reference to undefined module or namespace".to_string(),
            category: DiagnosticCategory::Correctness,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: true,
            examples: vec![RuleExample {
                description: "Unknown module qualifier".to_string(),
                code: "fn main() {\n    let _ = unknown_module::function();\n}".to_string(),
                fixed: None,
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec![
                "False positives for external crate modules and re-exports.".to_string(),
            ],
            needs_project_context: true,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "dead-code".to_string(),
            version: "1.0.0".to_string(),
            title: "Unreachable code after unconditional exit".to_string(),
            category: DiagnosticCategory::Correctness,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Medium,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: true,
            requires_experimental_semantics: false,
            examples: vec![RuleExample {
                description: "Code after return".to_string(),
                code: "fn demo() {\n    return;\n    let x = 1;\n}".to_string(),
                fixed: Some("fn demo() {\n    return;\n}".to_string()),
                language: ProgrammingLanguage::Rust,
            }],
            caveats: vec!["May miss unreachable code inside complex control flow.".to_string()],
            needs_project_context: false,
            aliases: vec!["unreachable-code".to_string()],
        });

        // Legacy/deprecated rules
        self.register_rule(RuleMetadata {
            id: "unused-variable".to_string(),
            version: "0.1.0".to_string(),
            title: "Potentially unused variable".to_string(),
            category: DiagnosticCategory::Style,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: true,
            examples: Vec::new(),
            caveats: vec!["Deprecated: use unused-symbol instead.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "shadowed-variable".to_string(),
            version: "0.1.0".to_string(),
            title: "Variable shadows outer binding".to_string(),
            category: DiagnosticCategory::Style,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Low,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: true,
            examples: Vec::new(),
            caveats: vec!["Deprecated: not actively maintained.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });

        self.register_rule(RuleMetadata {
            id: "deprecated-syntax".to_string(),
            version: "0.1.0".to_string(),
            title: "Deprecated syntax".to_string(),
            category: DiagnosticCategory::Style,
            default_severity: DiagnosticSeverity::Warning,
            confidence: DiagnosticConfidence::Medium,
            languages: ProgrammingLanguage::all(),
            enabled_by_default: false,
            requires_experimental_semantics: false,
            examples: Vec::new(),
            caveats: vec!["This rule is not actively maintained.".to_string()],
            needs_project_context: false,
            aliases: Vec::new(),
        });
    }
}

/// Errors that can occur during rule registry validation.
#[derive(Debug, Clone)]
pub enum RuleRegistryError {
    /// An alias points to a rule that does not exist.
    DanglingAlias { alias: String, target: String },
    /// A rule is enabled by default but has no examples.
    UndocumentedDefaultRule { rule_id: String },
    /// A rule ID is duplicated.
    DuplicateRuleId { rule_id: String },
    /// An unknown category was used.
    UnknownCategory { category: String },
}

impl fmt::Display for RuleRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DanglingAlias { alias, target } => {
                write!(formatter, "Alias '{alias}' points to non-existent rule '{target}'")
            }
            Self::UndocumentedDefaultRule { rule_id } => {
                write!(formatter, "Rule '{rule_id}' is enabled by default but has no examples")
            }
            Self::DuplicateRuleId { rule_id } => {
                write!(formatter, "Duplicate rule ID: '{rule_id}'")
            }
            Self::UnknownCategory { category } => {
                write!(formatter, "Unknown category: '{category}'")
            }
        }
    }
}

impl std::error::Error for RuleRegistryError {}

/// Applies CLI policy overrides to rule metadata.
///
/// Given a rule's metadata and policy selectors, returns the effective
/// severity after applying `--deny`, `--warn`, `--allow`, and `--strict`.
pub fn apply_policy(
    metadata: &RuleMetadata,
    deny_rules: &[RuleSelector],
    warn_rules: &[RuleSelector],
    allow_rules: &[RuleSelector],
    strict: bool,
) -> DiagnosticSeverity {
    let base = metadata.default_severity;

    // Check most specific selectors first (rule ID), then category
    for selector in allow_rules {
        if selector.matches(metadata) {
            return DiagnosticSeverity::Info;
        }
    }

    for selector in warn_rules {
        if selector.matches(metadata) {
            return DiagnosticSeverity::Warning;
        }
    }

    for selector in deny_rules {
        if selector.matches(metadata) {
            return DiagnosticSeverity::Error;
        }
    }

    // --strict promotes warnings to errors
    if strict && base == DiagnosticSeverity::Warning {
        return DiagnosticSeverity::Error;
    }

    base
}

/// Selector for targeting rules by ID or category.
#[derive(Debug, Clone)]
pub enum RuleSelector {
    /// Match a specific rule ID.
    Rule(String),
    /// Match all rules in a category.
    Category(DiagnosticCategory),
    /// Match all rules.
    All,
}

impl RuleSelector {
    /// Parses a selector from a string.
    ///
    /// - `"all"` matches all rules.
    /// - `"category:name"` matches a category.
    /// - Any other string matches a rule ID.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("all") {
            return Some(Self::All);
        }
        if let Some(rest) = s.strip_prefix("category:") {
            let category = rest.trim().parse().ok()?;
            return Some(Self::Category(category));
        }
        Some(Self::Rule(s.to_string()))
    }

    /// Checks if this selector matches a rule.
    pub fn matches(&self, metadata: &RuleMetadata) -> bool {
        match self {
            Self::All => true,
            Self::Rule(id) => &metadata.id == id || metadata.aliases.contains(id),
            Self::Category(category) => &metadata.category == category,
        }
    }
}

/// Creates diagnostic metadata from rule metadata and policy.
pub fn diagnostic_metadata_from_rule(
    rule_id: &str,
    registry: &RuleRegistry,
    deny_rules: &[RuleSelector],
    warn_rules: &[RuleSelector],
    allow_rules: &[RuleSelector],
    strict: bool,
    experimental_semantics: bool,
) -> DiagnosticMetadata {
    let rule = registry.get_or_default(rule_id);
    let effective_severity = apply_policy(&rule, deny_rules, warn_rules, allow_rules, strict);

    let is_enabled = if rule.requires_experimental_semantics {
        experimental_semantics && rule.enabled_by_default
    } else {
        rule.enabled_by_default
    };

    DiagnosticMetadata {
        category: rule.category,
        confidence: rule.confidence,
        source: diagnostic_source_for_rule(&rule),
        default_severity: rule.default_severity,
        effective_severity,
        is_enabled_by_default: is_enabled,
        requires_experimental_semantics: rule.requires_experimental_semantics,
    }
}

fn diagnostic_source_for_rule(rule: &RuleMetadata) -> DiagnosticSource {
    match rule.id.as_str() {
        "invalid-syntax" => DiagnosticSource::SyntaxParser,
        "undefined-symbol" | "unused-symbol" | "unused-import" | "dead-code"
        | "undefined-module" => DiagnosticSource::SemanticAnalysis,
        _ => DiagnosticSource::TreeSitterQuery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_builtin_rules() {
        let registry = RuleRegistry::new();
        assert!(registry.has_rule("unwrap-call"));
        assert!(registry.has_rule("undefined-symbol"));
    }

    #[test]
    fn registry_lookup_by_alias() {
        let registry = RuleRegistry::new();
        assert!(registry.get("dead-code").is_some());
        assert_eq!(registry.get("unreachable-code").unwrap().id, "dead-code");
    }

    #[test]
    fn experimental_rules_disabled_by_default() {
        let registry = RuleRegistry::new();
        let undefined = registry.get("undefined-symbol").unwrap();
        assert!(!undefined.enabled_by_default);
        assert!(undefined.requires_experimental_semantics);
    }

    #[test]
    fn syntax_rule_is_error() {
        let registry = RuleRegistry::new();
        let syntax = registry.get("invalid-syntax").unwrap();
        assert_eq!(syntax.default_severity, DiagnosticSeverity::Error);
        assert_eq!(syntax.category, DiagnosticCategory::Correctness);
    }

    #[test]
    fn strict_promotes_warning_to_error() {
        let registry = RuleRegistry::new();
        let unwrap = registry.get("unwrap-call").unwrap();
        let severity = apply_policy(unwrap, &[], &[], &[], true);
        assert_eq!(severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn allow_demotes_to_info() {
        let registry = RuleRegistry::new();
        let unwrap = registry.get("unwrap-call").unwrap();
        let severity = apply_policy(unwrap, &[], &[], &[RuleSelector::Rule("unwrap-call".to_string())], false);
        assert_eq!(severity, DiagnosticSeverity::Info);
    }

    #[test]
    fn deny_promotes_to_error() {
        let registry = RuleRegistry::new();
        let unwrap = registry.get("unwrap-call").unwrap();
        let severity = apply_policy(unwrap, &[RuleSelector::Rule("unwrap-call".to_string())], &[], &[], false);
        assert_eq!(severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn category_selector_matches() {
        let registry = RuleRegistry::new();
        let unwrap = registry.get("unwrap-call").unwrap();
        let selector = RuleSelector::Category(DiagnosticCategory::Restriction);
        assert!(selector.matches(unwrap));
    }

    #[test]
    fn rule_selector_parses() {
        assert!(matches!(RuleSelector::parse("all").unwrap(), RuleSelector::All));
        assert!(matches!(RuleSelector::parse("category:suspicious").unwrap(), RuleSelector::Category(DiagnosticCategory::Suspicious)));
        assert!(matches!(RuleSelector::parse("unwrap-call").unwrap(), RuleSelector::Rule(_)));
    }
}
