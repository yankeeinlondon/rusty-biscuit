use std::collections::HashMap;

use crate::queries::QueryKind;
use crate::shared::ProgrammingLanguage;

/// A suite of queries maintained by nvim-treesitter.
///
/// Tree Hugger currently vendors only `Locals`, but the inventory records
/// what captures are available from every upstream suite so future work can
/// reuse highlights, injections, folds, and indents where applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NvimQuerySuite {
    /// Scope and definition queries (`locals.scm`).
    Locals,
    /// Syntax highlighting queries (`highlights.scm`).
    Highlights,
    /// Language-injection queries (`injections.scm`).
    Injections,
    /// Code-folding queries (`folds.scm`).
    Folds,
    /// Indentation queries (`indents.scm`).
    Indents,
}

impl std::fmt::Display for NvimQuerySuite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locals => write!(f, "locals"),
            Self::Highlights => write!(f, "highlights"),
            Self::Injections => write!(f, "injections"),
            Self::Folds => write!(f, "folds"),
            Self::Indents => write!(f, "indents"),
        }
    }
}

/// A capture group discovered in an nvim-treesitter query suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureEntry {
    /// The capture name (e.g., `@function.call`).
    pub name: String,
    /// The node pattern the capture is attached to.
    pub pattern: String,
    /// Which suite this capture belongs to.
    pub suite: NvimQuerySuite,
    /// Whether Tree Hugger already reuses this capture.
    pub reused: bool,
    /// Notes about translation or mapping to Tree Hugger conventions.
    pub notes: Vec<String>,
}

/// Per-language inventory of upstream captures.
#[derive(Debug, Clone)]
pub struct LanguageInventory {
    /// Language this inventory describes.
    pub language: ProgrammingLanguage,
    /// All captures indexed by suite.
    pub captures: HashMap<NvimQuerySuite, Vec<CaptureEntry>>,
}

impl LanguageInventory {
    /// Creates an empty inventory for a language.
    pub fn new(language: ProgrammingLanguage) -> Self {
        Self {
            language,
            captures: HashMap::new(),
        }
    }

    /// Adds a capture entry to the inventory.
    pub fn add(&mut self, suite: NvimQuerySuite, entry: CaptureEntry) {
        self.captures.entry(suite).or_default().push(entry);
    }

    /// Returns captures for a specific suite.
    pub fn suite_captures(&self,
        suite: NvimQuerySuite,
    ) -> Option<&Vec<CaptureEntry>> {
        self.captures.get(&suite)
    }

    /// Returns the number of captured entries across all suites.
    pub fn total_captures(&self) -> usize {
        self.captures.values().map(|v| v.len()).sum()
    }

    /// Returns how many captures are marked as reused.
    pub fn reused_count(&self) -> usize {
        self.captures
            .values()
            .flat_map(|v| v.iter())
            .filter(|e| e.reused)
            .count()
    }
}

/// Global inventory of nvim-treesitter captures per language.
pub struct QueryInventory {
    /// Inventories keyed by language.
    pub languages: HashMap<ProgrammingLanguage, LanguageInventory>,
}

impl Default for QueryInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryInventory {
    /// Builds the inventory from known upstream capture conventions.
    ///
    /// This is a static snapshot derived from nvim-treesitter query files.
    /// It documents what captures exist and which ones Tree Hugger currently
    /// reuses (primarily `@local.definition.*` and `@local.reference.*`
    /// from `locals.scm`).
    pub fn new() -> Self {
        let mut languages = HashMap::new();

        for language in ProgrammingLanguage::all() {
            let mut inventory = LanguageInventory::new(language);
            populate_locals_captures(&mut inventory, language);
            populate_highlights_captures(&mut inventory, language);
            populate_injections_captures(&mut inventory, language);
            populate_folds_captures(&mut inventory, language);
            populate_indents_captures(&mut inventory, language);
            languages.insert(language, inventory);
        }

        Self { languages }
    }

    /// Returns the inventory for a language.
    pub fn for_language(&self,
        language: ProgrammingLanguage,
    ) -> Option<&LanguageInventory> {
        self.languages.get(&language)
    }

    /// Returns all capture entries for a language and suite.
    pub fn captures(
        &self,
        language: ProgrammingLanguage,
        suite: NvimQuerySuite,
    ) -> Option<&Vec<CaptureEntry>> {
        self.languages
            .get(&language)
            .and_then(|inv| inv.suite_captures(suite))
    }

    /// Returns true if the inventory has any captures for the language.
    pub fn has_language(&self,
        language: ProgrammingLanguage,
    ) -> bool {
        self.languages.contains_key(&language)
    }
}

fn populate_locals_captures(inventory: &mut LanguageInventory, _language: ProgrammingLanguage) {
    // Tree Hugger reuses locals.scm captures from nvim-treesitter.
    // The key mappings are:
    //   @local.definition.*  -> symbol definitions
    //   @local.reference.*   -> identifier usages
    //   @local.scope        -> scope boundaries
    let reused = vec![
        ("local.definition", "scope/definition captures"),
        ("local.definition.function", "function definitions"),
        ("local.definition.method", "method definitions"),
        ("local.definition.type", "type definitions"),
        ("local.definition.enum", "enum definitions"),
        ("local.definition.trait", "trait definitions"),
        ("local.definition.interface", "interface definitions"),
        ("local.definition.class", "class definitions"),
        ("local.definition.field", "field/variant definitions"),
        ("local.definition.var", "variable definitions"),
        ("local.definition.parameter", "parameter definitions"),
        ("local.definition.import", "import definitions"),
        ("local.definition.macro", "macro definitions"),
        ("local.definition.namespace", "module/namespace definitions"),
        ("local.reference", "identifier references"),
        ("local.scope", "scope boundaries"),
    ];

    for (name, desc) in reused {
        inventory.add(
            NvimQuerySuite::Locals,
            CaptureEntry {
                name: name.to_string(),
                pattern: desc.to_string(),
                suite: NvimQuerySuite::Locals,
                reused: true,
                notes: vec!["Mapped to Tree Hugger symbol extraction".to_string()],
            },
        );
    }
}

fn populate_highlights_captures(
    inventory: &mut LanguageInventory,
    _language: ProgrammingLanguage,
) {
    // Highlights captures are not currently reused by Tree Hugger, but
    // they are inventoried for future use (e.g., syntax highlighting in
    // terminal output).
    let not_reused = vec![
        ("attribute", "decorators/attributes"),
        ("comment", "comments"),
        ("constant", "constants"),
        ("constant.builtin", "built-in constants"),
        ("constructor", "constructors"),
        ("function", "functions"),
        ("function.builtin", "built-in functions"),
        ("function.call", "function calls"),
        ("function.macro", "macro invocations"),
        ("keyword", "keywords"),
        ("keyword.function", "function keywords"),
        ("keyword.return", "return keywords"),
        ("label", "labels"),
        ("method", "methods"),
        ("method.call", "method calls"),
        ("namespace", "namespaces/modules"),
        ("number", "numeric literals"),
        ("operator", "operators"),
        ("parameter", "parameters"),
        ("property", "properties"),
        ("punctuation.delimiter", "delimiters"),
        ("punctuation.bracket", "brackets"),
        ("string", "string literals"),
        ("string.special", "special strings"),
        ("type", "type names"),
        ("type.builtin", "built-in types"),
        ("variable", "variables"),
        ("variable.builtin", "built-in variables"),
        ("variable.parameter", "parameters"),
    ];

    for (name, desc) in not_reused {
        inventory.add(
            NvimQuerySuite::Highlights,
            CaptureEntry {
                name: name.to_string(),
                pattern: desc.to_string(),
                suite: NvimQuerySuite::Highlights,
                reused: false,
                notes: vec![
                    "Not yet reused by Tree Hugger".to_string(),
                ],
            },
        );
    }
}

fn populate_injections_captures(
    inventory: &mut LanguageInventory,
    _language: ProgrammingLanguage,
) {
    inventory.add(
        NvimQuerySuite::Injections,
        CaptureEntry {
            name: "injection.content".to_string(),
            pattern: "injected language content".to_string(),
            suite: NvimQuerySuite::Injections,
            reused: false,
            notes: vec!["Not yet reused by Tree Hugger".to_string()],
        },
    );
    inventory.add(
        NvimQuerySuite::Injections,
        CaptureEntry {
            name: "injection.language".to_string(),
            pattern: "language identifier".to_string(),
            suite: NvimQuerySuite::Injections,
            reused: false,
            notes: vec!["Not yet reused by Tree Hugger".to_string()],
        },
    );
}

fn populate_folds_captures(
    inventory: &mut LanguageInventory,
    _language: ProgrammingLanguage,
) {
    inventory.add(
        NvimQuerySuite::Folds,
        CaptureEntry {
            name: "fold".to_string(),
            pattern: "foldable region".to_string(),
            suite: NvimQuerySuite::Folds,
            reused: false,
            notes: vec!["Not yet reused by Tree Hugger".to_string()],
        },
    );
}

fn populate_indents_captures(
    inventory: &mut LanguageInventory,
    _language: ProgrammingLanguage,
) {
    let indents = vec![
        ("indent.begin", "indentation increase"),
        ("indent.end", "indentation decrease"),
        ("indent.auto", "automatic indentation"),
        ("indent.dedent", "dedent trigger"),
        ("indent.branch", "branch point"),
        ("indent.ignore", "ignore indentation"),
        ("indent.align", "alignment anchor"),
        ("indent.zero", "zero indentation"),
    ];

    for (name, desc) in indents {
        inventory.add(
            NvimQuerySuite::Indents,
            CaptureEntry {
                name: name.to_string(),
                pattern: desc.to_string(),
                suite: NvimQuerySuite::Indents,
                reused: false,
                notes: vec!["Not yet reused by Tree Hugger".to_string()],
            },
        );
    }
}

/// Returns the nvim-treesitter query suite that corresponds to a Tree Hugger
/// query kind, if any.
pub fn suite_for_kind(kind: QueryKind) -> Option<NvimQuerySuite> {
    match kind {
        QueryKind::Locals => Some(NvimQuerySuite::Locals),
        _ => None,
    }
}

/// Lists all captures that Tree Hugger currently reuses from nvim-treesitter.
pub fn reused_captures() -> Vec<CaptureEntry> {
    let inventory = QueryInventory::new();
    let mut reused = Vec::new();

    for lang_inventory in inventory.languages.values() {
        for entries in lang_inventory.captures.values() {
            for entry in entries {
                if entry.reused {
                    reused.push(entry.clone());
                }
            }
        }
    }

    reused
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_has_all_languages() {
        let inventory = QueryInventory::new();
        for language in ProgrammingLanguage::all() {
            assert!(
                inventory.has_language(language),
                "inventory should include {language}"
            );
        }
    }

    #[test]
    fn locals_suite_has_reused_captures() {
        let inventory = QueryInventory::new();
        let rust = inventory
            .for_language(ProgrammingLanguage::Rust)
            .expect("rust should be present");
        let locals = rust
            .suite_captures(NvimQuerySuite::Locals)
            .expect("locals should be present");
        assert!(!locals.is_empty());
        assert!(locals.iter().any(|c| c.reused));
    }

    #[test]
    fn highlights_suite_is_not_reused() {
        let inventory = QueryInventory::new();
        let rust = inventory
            .for_language(ProgrammingLanguage::Rust)
            .expect("rust should be present");
        let highlights = rust
            .suite_captures(NvimQuerySuite::Highlights)
            .expect("highlights should be present");
        assert!(!highlights.is_empty());
        assert!(!highlights.iter().any(|c| c.reused));
    }

    #[test]
    fn suite_for_kind_locals() {
        assert_eq!(
            suite_for_kind(QueryKind::Locals),
            Some(NvimQuerySuite::Locals)
        );
    }

    #[test]
    fn suite_for_kind_lint_is_none() {
        assert!(suite_for_kind(QueryKind::Lint).is_none());
    }

    #[test]
    fn reused_captures_list_is_non_empty() {
        let captures = reused_captures();
        assert!(!captures.is_empty());
        assert!(captures.iter().all(|c| c.reused));
    }

    #[test]
    fn language_inventory_counts() {
        let mut inv = LanguageInventory::new(ProgrammingLanguage::Rust);
        inv.add(
            NvimQuerySuite::Locals,
            CaptureEntry {
                name: "test".to_string(),
                pattern: "pattern".to_string(),
                suite: NvimQuerySuite::Locals,
                reused: true,
                notes: Vec::new(),
            },
        );
        assert_eq!(inv.total_captures(), 1);
        assert_eq!(inv.reused_count(), 1);
    }
}
