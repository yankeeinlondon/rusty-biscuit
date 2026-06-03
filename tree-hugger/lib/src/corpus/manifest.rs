use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::CorpusTier;

/// A manifest describing the corpus under analysis.
///
/// The manifest pins repositories or fixture archives by commit SHA,
/// records license notes, selected paths, excluded directories, enabled
/// rules, and oracle tools. It serves as the configuration for corpus
/// harness runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusManifest {
    /// Version of the manifest format.
    pub version: String,
    /// Human-readable description of this corpus.
    pub description: String,
    /// The repositories or fixture sets in this corpus.
    pub items: Vec<CorpusItem>,
    /// Global rules enabled for all items (unless overridden).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_rules: Vec<String>,
    /// Global rules explicitly disabled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_rules: Vec<String>,
    /// Per-rule thresholds.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub thresholds: HashMap<String, super::Threshold>,
    /// Oracle tools used for comparison.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub oracles: Vec<OracleConfig>,
}

impl CorpusManifest {
    /// Creates a new empty manifest.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            version: "1".to_string(),
            description: description.into(),
            items: Vec::new(),
            enabled_rules: Vec::new(),
            disabled_rules: Vec::new(),
            thresholds: HashMap::new(),
            oracles: Vec::new(),
        }
    }

    /// Adds a corpus item.
    pub fn add_item(mut self, item: CorpusItem) -> Self {
        self.items.push(item);
        self
    }

    /// Sets the enabled rules.
    pub fn with_enabled_rules(mut self, rules: Vec<String>) -> Self {
        self.enabled_rules = rules;
        self
    }

    /// Sets the disabled rules.
    pub fn with_disabled_rules(mut self, rules: Vec<String>) -> Self {
        self.disabled_rules = rules;
        self
    }

    /// Sets a threshold for a rule.
    pub fn with_threshold(mut self, rule: impl Into<String>, threshold: super::Threshold) -> Self {
        self.thresholds.insert(rule.into(), threshold);
        self
    }

    /// Adds an oracle configuration.
    pub fn with_oracle(mut self, oracle: OracleConfig) -> Self {
        self.oracles.push(oracle);
        self
    }

    /// Returns all items included in a given tier.
    pub fn items_for_tier(&self, tier: CorpusTier) -> impl Iterator<Item = &CorpusItem> {
        self.items.iter().filter(move |item| {
            item.tiers.contains(&tier)
                || tier == CorpusTier::Benchmark
                || (tier == CorpusTier::Expanded && item.tiers.contains(&CorpusTier::Smoke))
        })
    }
}

/// A single repository or fixture set in the corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusItem {
    /// Human-readable name.
    pub name: String,
    /// Source URL or local path.
    pub source: String,
    /// Commit SHA, tag, or version pin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// License note (e.g., "MIT", "Apache-2.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Selected paths within the source (glob patterns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_paths: Vec<String>,
    /// Excluded directories or files (glob patterns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_paths: Vec<String>,
    /// Primary language(s) in this item.
    pub language: String,
    /// Which tiers include this item.
    pub tiers: Vec<CorpusTier>,
    /// Item-specific enabled rules (overrides globals).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_rules: Option<Vec<String>>,
    /// Item-specific disabled rules (overrides globals).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_rules: Option<Vec<String>>,
}

impl CorpusItem {
    /// Creates a new corpus item.
    pub fn new(name: impl Into<String>, source: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            revision: None,
            license: None,
            selected_paths: Vec::new(),
            excluded_paths: Vec::new(),
            language: language.into(),
            tiers: vec![CorpusTier::Smoke],
            enabled_rules: None,
            disabled_rules: None,
        }
    }

    /// Sets the revision pin.
    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    /// Sets the license note.
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Adds a selected path pattern.
    pub fn with_selected_path(mut self, path: impl Into<String>) -> Self {
        self.selected_paths.push(path.into());
        self
    }

    /// Adds an excluded path pattern.
    pub fn with_excluded_path(mut self, path: impl Into<String>) -> Self {
        self.excluded_paths.push(path.into());
        self
    }

    /// Sets the tiers this item belongs to.
    pub fn with_tiers(mut self, tiers: Vec<CorpusTier>) -> Self {
        self.tiers = tiers;
        self
    }
}

/// Configuration for an oracle tool used to validate corpus findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    /// Name of the oracle tool (e.g., "rustc", "clippy", "oxlint", "ruff").
    pub tool: String,
    /// Expected version or version range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Command-line arguments to pass to the oracle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Which languages this oracle applies to.
    pub languages: Vec<String>,
    /// Mapping from oracle rule IDs to Tree Hugger rule IDs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub rule_mapping: HashMap<String, String>,
}

impl OracleConfig {
    /// Creates a new oracle configuration.
    pub fn new(tool: impl Into<String>, languages: Vec<String>) -> Self {
        Self {
            tool: tool.into(),
            version: None,
            args: Vec::new(),
            languages,
            rule_mapping: HashMap::new(),
        }
    }

    /// Sets the expected version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Adds a rule mapping.
    pub fn with_rule_mapping(mut self, oracle_rule: impl Into<String>, our_rule: impl Into<String>) -> Self {
        self.rule_mapping.insert(oracle_rule.into(), our_rule.into());
        self
    }
}
