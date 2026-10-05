use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::analysis::counters::count_diagnostics_run;
use crate::error::TreeHuggerError;
use crate::file::extract::*;
use crate::file::query::{SymbolQuery, SymbolQueryResult};
use crate::file::source_file::SourceFile;
use crate::queries::{GrammarRef, QueryKind, format_rule_message, query_for, severity_for_rule};
use crate::rule_registry::{RuleRegistry, diagnostic_metadata_from_rule};
use crate::shared::{
    AnalysisPass, CodeBlock, CodeRange, Diagnostic, DiagnosticSeverity, FileSymbolIndex,
    ImportSymbol, LintDiagnostic, ProgrammingLanguage, ReferencedSymbol, SourceContext, SymbolInfo,
    SymbolKind, SymbolKindData, SymbolKindV2, SymbolRecord, SymbolRef, SyntaxDiagnostic,
    stable_symbol_key, symbol_id_from_stable_key,
};

mod definitions;

pub(crate) use definitions::{ExtractedSymbol, MetadataSelection, RecordFacts};

/// Represents a parsed source file backed by tree-sitter.
#[derive(Debug, Clone)]
pub struct TreeFile {
    /// Absolute path to the file on disk.
    pub file: PathBuf,
    /// The detected programming language for the file.
    pub language: ProgrammingLanguage,
    /// A deterministic hash of the file contents.
    pub hash: String,
    source: String,
    tree: tree_sitter::Tree,
    /// The grammar used to parse `tree`; queries must compile against it.
    grammar: tree_sitter::Language,
    /// Stable identifier for the grammar variant, used to key query caching.
    grammar_id: &'static str,
    /// Whether to enable experimental semantic rules.
    pub experimental_semantics: bool,
}

impl TreeFile {
    /// Creates a new `TreeFile` by reading and parsing the file on disk.
    ///
    /// ## Returns
    /// Returns a parsed `TreeFile` ready for symbol and diagnostic queries.
    ///
    /// ## Errors
    /// Returns an error if the file cannot be read, parsed, or is unsupported.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, TreeHuggerError> {
        Self::with_language(path, None)
    }

    /// Creates a new `TreeFile`, overriding language detection when provided.
    ///
    /// ## Returns
    /// Returns the parsed `TreeFile` for the provided path.
    ///
    /// ## Errors
    /// Returns an error if the file cannot be read, parsed, or is unsupported.
    pub fn with_language<P: AsRef<Path>>(
        path: P,
        language: Option<ProgrammingLanguage>,
    ) -> Result<Self, TreeHuggerError> {
        SourceFile::read_with_language(path, language)?.parse()
    }

    /// Assembles a parsed file from [`SourceFile::parse`]'s parts.
    pub(crate) fn from_parts(
        file: PathBuf,
        language: ProgrammingLanguage,
        hash: String,
        source: String,
        tree: tree_sitter::Tree,
        grammar: tree_sitter::Language,
        grammar_id: &'static str,
    ) -> Self {
        Self {
            file,
            language,
            hash,
            source,
            tree,
            grammar,
            grammar_id,
            experimental_semantics: false,
        }
    }

    /// Returns the source text of the parsed file.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the root node of the parsed syntax tree.
    ///
    /// Exposed within the crate for analyses that must walk the concrete
    /// syntax tree directly (e.g. control-flow nesting depth) rather than the
    /// extracted symbol records.
    pub(crate) fn root_node(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// Returns the byte ranges of all comment nodes detected by the comments
    /// query for this file's language.
    ///
    /// ## Returns
    /// Returns a list of `(start_byte, end_byte)` ranges covering comment nodes.
    /// Returns an empty vector if no comments query is available.
    ///
    /// ## Errors
    /// Returns an error if the comments query fails to compile.
    pub fn comment_ranges(&self) -> Result<Vec<(usize, usize)>, TreeHuggerError> {
        let query = query_for(self.grammar_ref(), QueryKind::Comments)?;
        if query.pattern_count() == 0 {
            return Ok(Vec::new());
        }

        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let mut ranges = Vec::new();
        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                ranges.push((capture.node.start_byte(), capture.node.end_byte()));
            }
            matches.advance();
        }

        Ok(ranges)
    }

    /// Returns a reference to the grammar this file was parsed with for query
    /// compilation.
    fn grammar_ref(&self) -> GrammarRef<'_> {
        GrammarRef {
            language: self.language,
            grammar: &self.grammar,
            id: self.grammar_id,
        }
    }

    fn diagnostic_metadata(&self, rule_id: &str) -> crate::shared::DiagnosticMetadata {
        diagnostic_metadata_from_rule(
            rule_id,
            &RuleRegistry::new(),
            &[],
            &[],
            &[],
            false,
            self.experimental_semantics,
        )
    }

    /// Provides the list of symbols imported by this file.
    ///
    /// ## Returns
    /// Returns the imported symbols that tree-sitter can detect.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn imported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError> {
        let query = query_for(self.grammar_ref(), QueryKind::Imports)?;
        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        let mut imports = Vec::new();

        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                if capture_name != "local.definition.import" {
                    continue;
                }

                let node = capture.node;

                // Skip imports that should be filtered out (e.g., package declarations,
                // intermediate path components in Java/C#)
                if self.should_skip_import(node, self.language) {
                    continue;
                }

                let mut name = node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();

                // For Go, if the captured node is a string literal (import path),
                // extract the package name from the path
                if self.language == ProgrammingLanguage::Go && name.starts_with('"') {
                    let path = name.trim_matches('"');
                    name = path.rsplit('/').next().unwrap_or(path).to_string();
                }

                // Extract import metadata by traversing the AST
                let (source, original_name, alias) =
                    self.extract_import_metadata(node, &name, self.language);
                let statement_range = self.import_statement_range(node, self.language);
                let is_reexport = self.import_is_reexport(node, self.language);

                imports.push(ImportSymbol {
                    name,
                    original_name,
                    alias,
                    range: range_for_node(node),
                    statement_range,
                    language: self.language,
                    file: self.file.clone(),
                    source,
                    is_reexport,
                });
            }

            matches.advance();
        }

        Ok(imports)
    }

    /// Provides the list of symbol references in this file.
    ///
    /// References are identifier usages (not definitions) that refer to
    /// symbols defined elsewhere (locally, imported, or builtin).
    ///
    /// ## Returns
    /// Returns all identifier references detected by tree-sitter queries.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn referenced_symbols(&self) -> Result<Vec<ReferencedSymbol>, TreeHuggerError> {
        let definitions = self.definitions(&MetadataSelection::ALL)?;
        let imports = self.imported_symbols()?;
        self.references_from(&definitions, &imports)
    }

    /// The identifier usages of this file that are not one of `definitions`'
    /// or `imports`' own names.
    pub(crate) fn references_from(
        &self,
        definitions: &[ExtractedSymbol<'_>],
        imports: &[ImportSymbol],
    ) -> Result<Vec<ReferencedSymbol>, TreeHuggerError> {
        let query = query_for(self.grammar_ref(), QueryKind::References)?;

        // Empty query means no reference rules defined for this language
        if query.pattern_count() == 0 {
            return Ok(Vec::new());
        }

        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        let mut references = Vec::new();
        let mut seen_ranges = HashSet::new();
        let mut definition_ranges = HashSet::new();

        for extracted in definitions {
            let range = &extracted.symbol.range;
            definition_ranges.insert((range.start_byte, range.end_byte));
        }

        for import in imports {
            definition_ranges.insert((import.range.start_byte, import.range.end_byte));
        }

        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                // Only process @reference captures
                if capture_name != "reference" {
                    continue;
                }

                let node = capture.node;
                let range = range_for_node(node);

                // Deduplicate based on byte range (same node captured multiple times)
                let range_key = (range.start_byte, range.end_byte);
                if !seen_ranges.insert(range_key) {
                    continue;
                }

                if definition_ranges.contains(&range_key)
                    || is_definition_like_reference(node, self.language)
                {
                    continue;
                }

                let name = node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();

                // Skip empty names
                if name.is_empty() {
                    continue;
                }

                // Detect if this is a qualified reference
                let (is_qualified, qualifier) = self.detect_qualified_reference(node);

                references.push(ReferencedSymbol {
                    name,
                    range,
                    language: self.language,
                    file: self.file.clone(),
                    is_qualified,
                    qualifier,
                });
            }

            matches.advance();
        }

        Ok(references)
    }

    /// Detects if a node is part of a qualified reference.
    ///
    /// Returns (is_qualified, qualifier) where qualifier is the prefix
    /// (e.g., `foo` in `foo.bar` or `module` in `module::symbol`).
    fn detect_qualified_reference(&self, node: Node) -> (bool, Option<String>) {
        let parent = match node.parent() {
            Some(p) => p,
            None => return (false, None),
        };

        // Check for various qualified access patterns by language
        match self.language {
            ProgrammingLanguage::Rust => {
                // Check for scoped_identifier (module::symbol)
                if parent.kind() == "scoped_identifier"
                    && let Some(path_node) = parent.child_by_field_name("path")
                    && path_node.id() != node.id()
                {
                    let qualifier = path_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(String::from);
                    return (true, qualifier);
                }
                // Check for field_expression (object.field)
                if parent.kind() == "field_expression"
                    && let Some(value_node) = parent.child_by_field_name("value")
                    && value_node.id() != node.id()
                {
                    let qualifier = value_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(String::from);
                    return (true, qualifier);
                }
            }
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                // Check for member_expression (object.property)
                if parent.kind() == "member_expression"
                    && let Some(object_node) = parent.child_by_field_name("object")
                    && object_node.id() != node.id()
                {
                    let qualifier = object_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(String::from);
                    return (true, qualifier);
                }
            }
            ProgrammingLanguage::Python => {
                // Check for attribute (object.attr)
                if parent.kind() == "attribute"
                    && let Some(object_node) = parent.child_by_field_name("object")
                    && object_node.id() != node.id()
                {
                    let qualifier = object_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(String::from);
                    return (true, qualifier);
                }
            }
            ProgrammingLanguage::Go => {
                // Check for selector_expression (package.Symbol)
                if parent.kind() == "selector_expression"
                    && let Some(operand_node) = parent.child_by_field_name("operand")
                    && operand_node.id() != node.id()
                {
                    let qualifier = operand_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(String::from);
                    return (true, qualifier);
                }
            }
            _ => {
                // Generic check for field access patterns
                let kind = parent.kind();
                if kind.contains("member") || kind.contains("field") || kind.contains("selector") {
                    // Try to find the object/base child
                    let child_count = parent.named_child_count();
                    for i in 0..child_count {
                        if let Some(child) = parent.named_child(i as u32) {
                            if child.id() != node.id() {
                                let qualifier = child
                                    .utf8_text(self.source.as_bytes())
                                    .ok()
                                    .map(String::from);
                                return (true, qualifier);
                            }
                            break;
                        }
                    }
                }
            }
        }

        (false, None)
    }

    /// Determines if an import capture should be skipped.
    ///
    /// This filters out:
    /// - Package/namespace declarations (Java, C#)
    /// - Intermediate path components (only keep the final imported symbol)
    /// - Duplicate captures for aliased imports (Go)
    fn should_skip_import(&self, node: Node, language: ProgrammingLanguage) -> bool {
        match language {
            ProgrammingLanguage::Go => {
                // For Go, if this is a path capture (string literal) but the import_spec
                // has a real alias (not blank identifier "_"), skip the path capture
                if node.kind() == "interpreted_string_literal"
                    && let Some(import_spec) = find_ancestor_by_kind(node, "import_spec")
                    && let Some(name_node) = import_spec.child_by_field_name("name")
                {
                    // Only skip if the alias is a real package_identifier, not blank "_"
                    if name_node.kind() == "package_identifier" {
                        return true; // Skip path when there's a real alias
                    }
                }
                false
            }
            ProgrammingLanguage::Java => {
                // Skip if inside package_declaration
                if find_ancestor_by_kind(node, "package_declaration").is_some() {
                    return true;
                }
                // For Java imports, only keep the final identifier in the path
                // We need to find the outermost scoped_identifier and check if this node
                // is its rightmost identifier
                if let Some(import_decl) = find_ancestor_by_kind(node, "import_declaration") {
                    // Find the direct scoped_identifier child of import_declaration
                    for child in import_decl.children(&mut import_decl.walk()) {
                        if child.kind() == "scoped_identifier" {
                            // Get the rightmost identifier (the "name" field)
                            if let Some(name_node) = child.child_by_field_name("name") {
                                // Only keep this import if it's the rightmost identifier
                                return name_node.id() != node.id();
                            }
                        }
                    }
                }
                false
            }
            ProgrammingLanguage::CSharp => {
                // Skip if inside namespace_declaration (not using_directive)
                if find_ancestor_by_kind(node, "namespace_declaration").is_some()
                    && find_ancestor_by_kind(node, "using_directive").is_none()
                {
                    return true;
                }
                // For C# using directives, only keep the final identifier
                if let Some(using_decl) = find_ancestor_by_kind(node, "using_directive") {
                    // Find the direct qualified_name child
                    for child in using_decl.children(&mut using_decl.walk()) {
                        if child.kind() == "qualified_name"
                            && let Some(name_node) = child.child_by_field_name("name")
                        {
                            return name_node.id() != node.id();
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Extracts import metadata by traversing the AST from the captured import node.
    ///
    /// Returns (source, original_name, alias).
    fn extract_import_metadata(
        &self,
        node: Node,
        name: &str,
        language: ProgrammingLanguage,
    ) -> (Option<String>, Option<String>, Option<String>) {
        match language {
            ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
                self.extract_ecma_import_metadata(node, name)
            }
            ProgrammingLanguage::Python => self.extract_python_import_metadata(node, name),
            ProgrammingLanguage::Rust => self.extract_rust_import_metadata(node, name),
            ProgrammingLanguage::Go => self.extract_go_import_metadata(node, name),
            ProgrammingLanguage::Java => self.extract_java_import_metadata(node),
            ProgrammingLanguage::CSharp => self.extract_csharp_import_metadata(node),
            ProgrammingLanguage::Php => self.extract_php_import_metadata(node),
            ProgrammingLanguage::Scala => self.extract_scala_import_metadata(node, name),
            ProgrammingLanguage::Swift => self.extract_swift_import_metadata(node),
            _ => (None, None, None),
        }
    }

    /// Returns the range of the full import statement for grouping.
    fn import_statement_range(
        &self,
        node: Node,
        language: ProgrammingLanguage,
    ) -> Option<CodeRange> {
        let statement = match language {
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                find_ancestor_by_kind(node, "import_statement")
            }
            ProgrammingLanguage::Python => {
                find_ancestor_by_kinds(node, &["import_from_statement", "import_statement"])
            }
            ProgrammingLanguage::Rust => find_ancestor_by_kind(node, "use_declaration"),
            ProgrammingLanguage::Go => find_ancestor_by_kind(node, "import_declaration"),
            ProgrammingLanguage::Java => find_ancestor_by_kind(node, "import_declaration"),
            ProgrammingLanguage::CSharp => find_ancestor_by_kind(node, "using_directive"),
            ProgrammingLanguage::Php => find_ancestor_by_kind(node, "namespace_use_declaration"),
            ProgrammingLanguage::Scala => find_ancestor_by_kind(node, "import_declaration"),
            ProgrammingLanguage::Swift => find_ancestor_by_kind(node, "import_declaration"),
            _ => None,
        };

        statement.map(range_for_node)
    }

    /// Reports whether an import re-exports its symbol (part of the public API).
    ///
    /// Only languages with explicit re-export syntax are detected; others return
    /// `false`. Rust: a `use` declaration carrying a `visibility_modifier`
    /// (`pub use`, `pub(crate) use`, …) is a re-export.
    fn import_is_reexport(&self, node: Node, language: ProgrammingLanguage) -> bool {
        match language {
            ProgrammingLanguage::Rust => find_ancestor_by_kind(node, "use_declaration")
                .map(|stmt| {
                    let mut cursor = stmt.walk();
                    stmt.children(&mut cursor)
                        .any(|child| child.kind() == "visibility_modifier")
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Extracts import metadata for JavaScript/TypeScript.
    /// Handles: `import { foo } from "module"`, `import { foo as bar } from "module"`,
    /// `import * as ns from "module"`, `import foo from "module"`.
    fn extract_ecma_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut original_name = None;
        let mut alias = None;

        // Find the import_statement ancestor
        let import_stmt = find_ancestor_by_kind(node, "import_statement");
        if let Some(stmt) = import_stmt {
            // Find the source (string node)
            source = stmt
                .child_by_field_name("source")
                .and_then(|n| n.utf8_text(self.source.as_bytes()).ok())
                .map(|s| s.trim_matches('"').trim_matches('\'').to_string());
        }

        // Check if this is an aliased import (import_specifier with name and alias)
        let parent = node.parent();
        if let Some(p) = parent {
            if p.kind() == "import_specifier" {
                // import { original as alias } from "module"
                if let Some(name_node) = p.child_by_field_name("name")
                    && let Some(alias_node) = p.child_by_field_name("alias")
                {
                    let orig = name_node
                        .utf8_text(self.source.as_bytes())
                        .unwrap_or_default()
                        .to_string();
                    let al = alias_node
                        .utf8_text(self.source.as_bytes())
                        .unwrap_or_default()
                        .to_string();

                    // The captured node is the alias (what's defined locally)
                    if name == al {
                        original_name = Some(orig);
                        alias = Some(al);
                    }
                }
            } else if p.kind() == "namespace_import" {
                // import * as ns from "module"
                alias = Some(name.to_string());
                original_name = Some("*".to_string());
            }
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for Python.
    /// Handles: `import os`, `from typing import Optional`, `from x import y as z`.
    fn extract_python_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut original_name = None;
        let mut alias = None;

        // Find import_statement or import_from_statement ancestor
        let import_stmt = find_ancestor_by_kind(node, "import_statement")
            .or_else(|| find_ancestor_by_kind(node, "import_from_statement"));

        // Check for aliased import (aliased_import) - do this first to get the correct module name
        let parent = node.parent();
        if let Some(p) = parent
            && p.kind() == "aliased_import"
        {
            // This handles both:
            // - `import X as Y` (name field is module, alias field is local name)
            // - `from M import X as Y` (name field is original symbol, alias field is local name)
            if let Some(name_node) = p.child_by_field_name("name") {
                let orig = name_node
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or_default();
                if let Some(alias_node) = p.child_by_field_name("alias") {
                    let al = alias_node
                        .utf8_text(self.source.as_bytes())
                        .unwrap_or_default();
                    if name == al {
                        original_name = Some(extract_dotted_name(orig));
                        alias = Some(al.to_string());

                        // For `import X as Y`, the source is X (the module name)
                        if let Some(stmt) = &import_stmt
                            && stmt.kind() == "import_statement"
                        {
                            source = Some(orig.to_string());
                        }
                    }
                }
            }
        }

        // Set source if not already set by aliased import handling
        if source.is_none()
            && let Some(stmt) = import_stmt
        {
            if stmt.kind() == "import_from_statement" {
                // from X import Y - extract X as the source
                if let Some(module_node) = stmt.child_by_field_name("module_name") {
                    source = module_node
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                }
            } else {
                // import X - the module is the source
                // For `import os`, the source is "os" (same as name)
                source = Some(name.to_string());
            }
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for Rust.
    /// Handles: `use std::io`, `use std::io::{Read, Write}`, `use foo as bar`.
    fn extract_rust_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut original_name = None;
        let mut alias = None;

        // Find use_declaration ancestor
        let use_decl = find_ancestor_by_kind(node, "use_declaration");
        if let Some(decl) = use_decl {
            // Extract the full use path
            if let Some(arg) = decl.child_by_field_name("argument") {
                source = self.extract_rust_use_path(arg, name);
            }
        }

        // Check for use_as_clause (use foo as bar)
        let parent = node.parent();
        if let Some(p) = parent
            && p.kind() == "use_as_clause"
        {
            // The captured node is the alias
            if let Some(path_node) = p.child_by_field_name("path") {
                let orig = path_node
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or_default()
                    .to_string();
                original_name = Some(orig);
                alias = Some(name.to_string());
            }
        }

        (source, original_name, alias)
    }

    /// Extracts the full path for a Rust use statement.
    fn extract_rust_use_path(&self, node: Node, _target_name: &str) -> Option<String> {
        // Get the full path text
        let text = node.utf8_text(self.source.as_bytes()).ok()?;

        // Extract module path (everything before the last ::)
        // For `std::io::Read`, the source is `std::io`
        // For `std::process::{Child, Command}`, the source is `std::process`
        if let Some(last_sep) = text.rfind("::") {
            let path = &text[..last_sep];
            // Remove any use_list braces
            let cleaned = path.trim_end_matches('{').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }

        // For simple imports like `use foo`, the name is the source
        Some(text.to_string())
    }

    /// Extracts import metadata for Go.
    /// Handles: `import "fmt"`, `import alias "fmt"`.
    fn extract_go_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut alias = None;
        let mut original_name = None;

        // Find import_spec ancestor
        let import_spec = find_ancestor_by_kind(node, "import_spec");
        if let Some(spec) = import_spec {
            // Get the path (string literal)
            if let Some(path_node) = spec.child_by_field_name("path") {
                let path_text = path_node
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or_default();
                source = Some(path_text.trim_matches('"').to_string());
            }

            // Check if there's an alias (name field)
            if let Some(name_node) = spec.child_by_field_name("name") {
                let alias_text = name_node
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or_default();
                if alias_text == name {
                    alias = Some(name.to_string());
                    // Original name is derived from the path
                    if let Some(src) = &source {
                        original_name = src.rsplit('/').next().map(|s| s.to_string());
                    }
                }
            }
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for Java.
    /// Handles: `import com.example.Foo`, `import static java.lang.Math.PI`, `import java.io.*`.
    fn extract_java_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut original_name = None;
        let mut alias = None;

        // Find import_declaration ancestor
        let import_decl = find_ancestor_by_kind(node, "import_declaration");
        if let Some(decl) = import_decl {
            // Check if this is a wildcard import (has asterisk child)
            let is_wildcard = decl
                .children(&mut decl.walk())
                .any(|c| c.kind() == "asterisk");

            // Get the full import path from scoped_identifier
            for child in decl.children(&mut decl.walk()) {
                if child.kind() == "scoped_identifier" {
                    let full_path = child.utf8_text(self.source.as_bytes()).unwrap_or_default();

                    if is_wildcard {
                        // For wildcard imports like `import java.io.*`
                        // The source is the entire path, and the import represents "*"
                        source = Some(full_path.to_string());
                        original_name = Some("*".to_string());
                        alias = Some(
                            node.utf8_text(self.source.as_bytes())
                                .unwrap_or_default()
                                .to_string(),
                        );
                    } else {
                        // Extract package path (everything before the last dot)
                        // For `java.util.List`, source is `java.util`
                        if let Some(last_dot) = full_path.rfind('.') {
                            source = Some(full_path[..last_dot].to_string());
                        } else {
                            source = Some(full_path.to_string());
                        }
                    }
                    break;
                }
            }
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for C#.
    /// Handles: `using System.IO`.
    fn extract_csharp_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;

        // Find using_directive ancestor
        let using_decl = find_ancestor_by_kind(node, "using_directive");
        if let Some(decl) = using_decl {
            // Get the namespace
            for child in decl.children(&mut decl.walk()) {
                if child.kind() == "qualified_name" || child.kind() == "identifier" {
                    source = child
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    break;
                }
            }
        }

        (source, None, None)
    }

    /// Extracts import metadata for PHP.
    /// Handles: `use App\Models\User`, `use App\Models\User as UserModel`.
    fn extract_php_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut alias = None;
        let mut original_name = None;

        // Find namespace_use_clause ancestor
        let use_clause = find_ancestor_by_kind(node, "namespace_use_clause");
        if let Some(clause) = use_clause {
            // Get the qualified name
            for child in clause.children(&mut clause.walk()) {
                if child.kind() == "qualified_name" || child.kind() == "name" {
                    source = child
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                }
                if child.kind() == "namespace_aliasing_clause" {
                    // Has an alias
                    if let Some(alias_node) = child.child_by_field_name("alias") {
                        alias = alias_node
                            .utf8_text(self.source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());
                        // Original name is the last part of the qualified name
                        if let Some(src) = &source {
                            original_name = src.rsplit('\\').next().map(|s| s.to_string());
                        }
                    }
                }
            }
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for Scala.
    /// Handles: `import scala.io._`, `import java.util.{List => JList}`.
    fn extract_scala_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut alias = None;
        let mut original_name = None;

        // Find import_declaration ancestor
        let import_decl = find_ancestor_by_kind(node, "import_declaration");
        if let Some(decl) = import_decl {
            // Extract the full import path
            let import_text = decl.utf8_text(self.source.as_bytes()).unwrap_or_default();
            // Parse out the package path (skip "import " prefix)
            if import_text.contains('.') && import_text.len() > 7 {
                source = Some(import_text[7..].trim().to_string());
            }
        }

        // Check for renamed import (import_selectors with rename)
        let parent = node.parent();
        if let Some(p) = parent
            && p.kind() == "renamed_identifier"
            && let Some(name_node) = p.child(0)
        {
            let orig = name_node
                .utf8_text(self.source.as_bytes())
                .unwrap_or_default()
                .to_string();
            original_name = Some(orig);
            alias = Some(name.to_string());
        }

        (source, original_name, alias)
    }

    /// Extracts import metadata for Swift.
    /// Handles: `import Foundation`, `import UIKit`.
    fn extract_swift_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;

        // Find import_declaration ancestor
        let import_decl = find_ancestor_by_kind(node, "import_declaration");
        if let Some(decl) = import_decl {
            // Get the module identifier
            for child in decl.children(&mut decl.walk()) {
                if child.kind() == "identifier" {
                    source = child
                        .utf8_text(self.source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    break;
                }
            }
        }

        (source, None, None)
    }

    /// Provides the list of symbols exported by this file.
    ///
    /// ## Returns
    /// Returns exported symbols derived from top-level definitions.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn exported_symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError> {
        let root = self.tree.root_node();

        Ok(self
            .definitions(&MetadataSelection::ALL)?
            .into_iter()
            .filter(|extracted| is_exported_definition(&extracted.symbol, extracted.anchor, root))
            .map(|extracted| extracted.symbol)
            .collect())
    }

    /// The exported subset of `definitions`, in order.
    pub(crate) fn exports_from<'a>(
        &self,
        definitions: &'a [ExtractedSymbol<'_>],
    ) -> impl Iterator<Item = &'a SymbolInfo> {
        let root = self.tree.root_node();
        definitions
            .iter()
            .filter(move |extracted| is_exported_definition(&extracted.symbol, extracted.anchor, root))
            .map(|extracted| &extracted.symbol)
    }

    /// Provides the list of symbols re-exported by this file.
    ///
    /// ## Returns
    /// Returns an empty list until re-export capture support is added.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn reexported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError> {
        Ok(Vec::new())
    }

    /// Provides symbols defined in this file but not exported.
    ///
    /// ## Returns
    /// Returns locally scoped symbols.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn local_symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError> {
        let root = self.tree.root_node();

        Ok(self
            .definitions(&MetadataSelection::ALL)?
            .into_iter()
            .filter(|extracted| !is_exported_definition(&extracted.symbol, extracted.anchor, root))
            .map(|extracted| extracted.symbol)
            .collect())
    }

    /// Provides lint diagnostics for this file.
    ///
    /// Combines pattern-based and semantic lint checks:
    /// - Pattern checks: Query-based rules from lint.scm files
    /// - Semantic checks: undefined-symbol, unused-symbol, unused-import
    ///
    /// Respects ignore directives in comments:
    /// - `// tree-hugger-ignore: rule1, rule2` - Ignore specific rules on next line
    /// - `// tree-hugger-ignore` - Ignore all rules on next line
    /// - `// tree-hugger-ignore-file: rule1` - Ignore rule for entire file
    ///
    /// ## Returns
    /// Returns all lint diagnostics found by pattern and semantic analysis,
    /// filtered by ignore directives.
    ///
    /// ## Notes
    /// This is an infallible, best-effort wrapper: each analysis stage that
    /// fails contributes no diagnostics rather than aborting the others.
    /// Callers that must distinguish "clean file" from "analyzer could not run"
    /// should use [`try_lint_diagnostics`] instead.
    ///
    /// [`try_lint_diagnostics`]: Self::try_lint_diagnostics
    pub fn lint_diagnostics(&self) -> Vec<LintDiagnostic> {
        count_diagnostics_run();
        let mut diagnostics = Vec::new();
        diagnostics.extend(self.run_pattern_diagnostics().unwrap_or_default());
        let semantic = self.definitions(&MetadataSelection::ALL).and_then(|definitions| {
            let imports = self.imported_symbols()?;
            let references = self.references_from(&definitions, &imports)?;
            Ok(self.run_semantic_diagnostics(&definitions, &imports, &references))
        });
        diagnostics.extend(semantic.unwrap_or_default());
        self.apply_ignore_directives(diagnostics)
    }

    /// Returns all lint diagnostics found by pattern and semantic analysis,
    /// filtered by ignore directives.
    ///
    /// ## Returns
    /// Returns pattern-based and semantic lint diagnostics for the file.
    ///
    /// ## Errors
    /// Returns an error if a lint/reference query fails to compile or symbol
    /// extraction fails. An empty list means the file is clean, never that the
    /// analyzer failed to run.
    pub fn try_lint_diagnostics(&self) -> Result<Vec<LintDiagnostic>, TreeHuggerError> {
        let definitions = self.definitions(&MetadataSelection::ALL)?;
        let imports = self.imported_symbols()?;
        let references = self.references_from(&definitions, &imports)?;
        self.lint_diagnostics_from(&definitions, &imports, &references)
    }

    /// Lint diagnostics from already extracted definitions, imports and
    /// references, filtered by ignore directives.
    pub(crate) fn lint_diagnostics_from(
        &self,
        definitions: &[ExtractedSymbol<'_>],
        imports: &[ImportSymbol],
        references: &[ReferencedSymbol],
    ) -> Result<Vec<LintDiagnostic>, TreeHuggerError> {
        count_diagnostics_run();
        let mut diagnostics = self.run_pattern_diagnostics()?;
        diagnostics.extend(self.run_semantic_diagnostics(definitions, imports, references));
        Ok(self.apply_ignore_directives(diagnostics))
    }

    /// Removes diagnostics suppressed by in-source ignore directives.
    fn apply_ignore_directives(&self, mut diagnostics: Vec<LintDiagnostic>) -> Vec<LintDiagnostic> {
        use crate::ignore_directives::IgnoreDirectives;

        // Parse ignore directives using tree-sitter (avoids false positives from strings)
        let ignores =
            IgnoreDirectives::parse_with_grammar(&self.source, &self.tree, self.grammar_ref());
        if ignores.has_directives() {
            diagnostics.retain(|d| !ignores.should_ignore(d.range.start_line, d.rule.as_deref()));
        }

        diagnostics
    }

    /// Runs pattern-based lint checks from lint.scm query files.
    fn run_pattern_diagnostics(&self) -> Result<Vec<LintDiagnostic>, TreeHuggerError> {
        let query = query_for(self.grammar_ref(), QueryKind::Lint)?;

        // Empty query means no lint rules defined for this language
        if query.pattern_count() == 0 {
            return Ok(Vec::new());
        }

        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        let mut diagnostics = Vec::new();

        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                // Only process captures with "diagnostic." prefix
                if let Some(rule_id) = capture_name.strip_prefix("diagnostic.") {
                    let node = capture.node;
                    let range = range_for_node(node);
                    let context = self.build_source_context(&node);

                    diagnostics.push(LintDiagnostic {
                        message: format_rule_message(rule_id),
                        range,
                        severity: severity_for_rule(rule_id),
                        rule: Some(rule_id.to_string()),
                        context: Some(context),
                        metadata: Some(self.diagnostic_metadata(rule_id)),
                    });
                }
            }

            matches.advance();
        }

        Ok(diagnostics)
    }

    /// Runs semantic lint checks for undefined/unused symbols.
    fn run_semantic_diagnostics(
        &self,
        definitions: &[ExtractedSymbol<'_>],
        imports: &[ImportSymbol],
        references: &[ReferencedSymbol],
    ) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Build lookup sets
        let defined_names: std::collections::HashSet<&str> = definitions
            .iter()
            .map(|extracted| extracted.symbol.name.as_str())
            .collect();
        let imported_names: std::collections::HashSet<&str> =
            imports.iter().map(|i| i.name.as_str()).collect();
        let exported_names: std::collections::HashSet<&str> = self
            .exports_from(definitions)
            .map(|symbol| symbol.name.as_str())
            .collect();
        let referenced_names: std::collections::HashSet<&str> =
            references.iter().map(|r| r.name.as_str()).collect();

        // Check for undefined symbols (experimental)
        if self.experimental_semantics {
            diagnostics.extend(self.check_undefined_symbols(
                references,
                &defined_names,
                &imported_names,
            ));
        }

        // Check for unused symbols (experimental)
        if self.experimental_semantics {
            diagnostics.extend(self.check_unused_symbols(
                definitions.iter().map(|extracted| &extracted.symbol),
                &exported_names,
                &referenced_names,
            ));
        }

        // Check for unused imports (stable)
        diagnostics.extend(self.check_unused_imports(imports, &referenced_names));

        // Check for qualified references with undefined qualifiers (experimental)
        if self.experimental_semantics {
            diagnostics.extend(self.check_qualified_references(
                references,
                &defined_names,
                &imported_names,
            ));
        }

        // Check for dead code
        diagnostics.extend(self.check_dead_code());

        diagnostics
    }

    /// Checks for code that follows unconditional exit statements.
    fn check_dead_code(&self) -> Vec<LintDiagnostic> {
        self.dead_code_ranges()
            .into_iter()
            .map(|range| {
                let context = self.build_source_context_from_range(&range);
                LintDiagnostic {
                    message: "Unreachable code after unconditional exit".to_string(),
                    range,
                    severity: severity_for_rule("dead-code"),
                    rule: Some("dead-code".to_string()),
                    context: Some(context),
                    metadata: Some(self.diagnostic_metadata("dead-code")),
                }
            })
            .collect()
    }

    /// Collects the source ranges of code that follows unconditional exit
    /// statements, deduplicated and ordered by position.
    fn dead_code_ranges(&self) -> Vec<CodeRange> {
        use crate::dead_code::{find_dead_code_after, is_terminal_statement};

        let mut ranges = Vec::new();
        let root = self.tree.root_node();
        let mut stack = vec![root];
        let mut seen_dead: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        // Walk the entire AST looking for terminal statements
        while let Some(node) = stack.pop() {
            if is_block_like_dead_code_container(node.kind()) {
                let mut saw_terminal = false;
                let child_count = node.named_child_count();
                for index in 0..child_count {
                    let Some(child) = node.named_child(index as u32) else {
                        continue;
                    };

                    if saw_terminal {
                        let range = range_for_node(child);
                        if seen_dead.insert((range.start_byte, range.end_byte)) {
                            ranges.push(range);
                        }
                        continue;
                    }

                    if is_terminal_statement(child, self.language, &self.source) {
                        saw_terminal = true;
                    }
                }
            }

            if is_terminal_statement(node, self.language, &self.source) {
                // Find dead code after this terminal
                for dead_node in find_dead_code_after(node, self.language) {
                    let range = range_for_node(dead_node);
                    if seen_dead.insert((range.start_byte, range.end_byte)) {
                        ranges.push(range);
                    }
                }
            }

            // Continue traversing
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }

        ranges.sort_by_key(|range| (range.start_byte, range.end_byte));
        ranges
    }

    /// Checks for references to symbols that are not defined, imported, or builtin.
    fn check_undefined_symbols(
        &self,
        references: &[ReferencedSymbol],
        defined_names: &std::collections::HashSet<&str>,
        imported_names: &std::collections::HashSet<&str>,
    ) -> Vec<LintDiagnostic> {
        use crate::builtins::is_builtin;

        let mut diagnostics = Vec::new();
        let mut seen_undefined: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        for reference in references {
            let name = reference.name.as_str();

            // Skip if the reference is qualified (e.g., foo.bar - we only check 'foo')
            if reference.is_qualified {
                continue;
            }

            // Skip if defined locally
            if defined_names.contains(name) {
                continue;
            }

            // Skip if imported
            if imported_names.contains(name) {
                continue;
            }

            // Skip if it's a language builtin
            if is_builtin(self.language, name) {
                continue;
            }

            // Skip common patterns that are likely valid:
            // - Single character identifiers (often loop variables, etc.)
            // - Identifiers starting with uppercase (often type references from other modules)
            // - Identifiers starting with underscore (intentionally unused)
            if name.len() == 1 || name.starts_with('_') {
                continue;
            }

            // Avoid duplicate diagnostics for the same location
            let location = (reference.range.start_byte, reference.range.end_byte);
            if !seen_undefined.insert(location) {
                continue;
            }

            let context = self.build_source_context_from_range(&reference.range);

            diagnostics.push(LintDiagnostic {
                message: format!("Reference to undefined symbol '{}'", name),
                range: reference.range.clone(),
                severity: severity_for_rule("undefined-symbol"),
                rule: Some("undefined-symbol".to_string()),
                context: Some(context),
                metadata: Some(self.diagnostic_metadata("undefined-symbol")),
            });
        }

        diagnostics
    }

    /// Checks for definitions that are neither exported nor referenced.
    fn check_unused_symbols<'a>(
        &self,
        definitions: impl Iterator<Item = &'a SymbolInfo>,
        exported_names: &std::collections::HashSet<&str>,
        referenced_names: &std::collections::HashSet<&str>,
    ) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for definition in definitions {
            let name = definition.name.as_str();

            // Skip if exported
            if exported_names.contains(name) {
                continue;
            }

            // Skip if referenced
            if referenced_names.contains(name) {
                continue;
            }

            // Skip symbols starting with underscore (intentionally unused)
            if name.starts_with('_') {
                continue;
            }

            if definition.kind == SymbolKind::Function && name == "main" {
                continue;
            }

            // Skip certain symbol kinds that are typically used implicitly
            // (e.g., trait implementations, macros)
            if matches!(definition.kind, SymbolKind::Trait | SymbolKind::Macro) {
                continue;
            }

            let context = self.build_source_context_from_range(&definition.range);

            diagnostics.push(LintDiagnostic {
                message: format!("Symbol '{}' is defined but never used", name),
                range: definition.range.clone(),
                severity: severity_for_rule("unused-symbol"),
                rule: Some("unused-symbol".to_string()),
                context: Some(context),
                metadata: Some(self.diagnostic_metadata("unused-symbol")),
            });
        }

        diagnostics
    }

    /// Checks for imports that are never referenced.
    fn check_unused_imports(
        &self,
        imports: &[ImportSymbol],
        referenced_names: &std::collections::HashSet<&str>,
    ) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for import in imports {
            let name = import.name.as_str();

            // Skip if referenced
            if referenced_names.contains(name) {
                continue;
            }

            // Skip re-exports (e.g. Rust `pub use`): exporting the symbol is the
            // use, so a missing local reference is expected, not a defect.
            if import.is_reexport {
                continue;
            }

            // Skip namespace imports (import * as x) - check for common patterns
            if name == "*" || name.contains("*") {
                continue;
            }

            // Skip imports starting with underscore (intentionally unused)
            if name.starts_with('_') {
                continue;
            }

            let context = self.build_source_context_from_range(&import.range);

            diagnostics.push(LintDiagnostic {
                message: format!("Imported symbol '{}' is never used", name),
                range: import.range.clone(),
                severity: severity_for_rule("unused-import"),
                rule: Some("unused-import".to_string()),
                context: Some(context),
                metadata: Some(self.diagnostic_metadata("unused-import")),
            });
        }

        diagnostics
    }

    /// Checks for qualified references with undefined qualifiers.
    ///
    /// Reports warnings for qualified references (e.g., `unknown_module::function()`)
    /// where the qualifier is not defined in the file's imports or declarations.
    ///
    /// The following qualifiers are skipped (not reported):
    /// - `self`, `this`, `super`, `Self` - language-specific keywords
    /// - Single-letter identifiers - likely generic type parameters
    fn check_qualified_references(
        &self,
        references: &[ReferencedSymbol],
        defined_names: &std::collections::HashSet<&str>,
        imported_names: &std::collections::HashSet<&str>,
    ) -> Vec<LintDiagnostic> {
        use crate::builtins::is_builtin;

        // Qualifiers to skip (language keywords and common patterns)
        const SKIP_QUALIFIERS: &[&str] = &["self", "this", "super", "Self"];

        let mut diagnostics = Vec::new();
        let mut seen_undefined: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        for reference in references {
            // Only check qualified references that have a qualifier
            let qualifier = match &reference.qualifier {
                Some(q) => q.as_str(),
                None => continue,
            };

            // Skip language keywords
            if SKIP_QUALIFIERS.contains(&qualifier) {
                continue;
            }

            // Skip single-letter qualifiers (likely generic type params like T, U, etc.)
            if qualifier.len() == 1 {
                continue;
            }

            // Skip if the qualifier is defined locally
            if defined_names.contains(qualifier) {
                continue;
            }

            // Skip if the qualifier is imported
            if imported_names.contains(qualifier) {
                continue;
            }

            // Skip if it's a language builtin
            if is_builtin(self.language, qualifier) {
                continue;
            }

            // Avoid duplicate diagnostics for the same location
            let location = (reference.range.start_byte, reference.range.end_byte);
            if !seen_undefined.insert(location) {
                continue;
            }

            // Build context from range
            let context = self.build_source_context_from_range(&reference.range);

            diagnostics.push(LintDiagnostic {
                message: format!("Reference to undefined module or namespace '{}'", qualifier),
                range: reference.range.clone(),
                severity: severity_for_rule("undefined-module"),
                rule: Some("undefined-module".to_string()),
                context: Some(context),
                metadata: Some(self.diagnostic_metadata("undefined-module")),
            });
        }

        diagnostics
    }

    /// Builds source context from a `CodeRange` to enable visual diagnostic display.
    ///
    /// This is the primary helper for creating `SourceContext` instances. It handles
    /// the conversion from 1-indexed `CodeRange` values to 0-indexed source positions.
    fn build_source_context_from_range(&self, range: &CodeRange) -> SourceContext {
        // CodeRange uses 1-indexed lines/columns, convert to 0-indexed for nth()
        let line_index = range.start_line.saturating_sub(1);
        let underline_column = range.start_column.saturating_sub(1);

        // Get the line containing the range start
        let line_text = self
            .source
            .lines()
            .nth(line_index)
            .unwrap_or("")
            .to_string();

        // Calculate underline length (handle multi-line by capping to end of first line)
        let underline_length = if range.start_line == range.end_line {
            range.end_column.saturating_sub(range.start_column).max(1)
        } else {
            // Multi-line: underline to end of first line
            line_text.len().saturating_sub(underline_column).max(1)
        };

        SourceContext {
            line_text,
            underline_column,
            underline_length,
        }
    }

    /// Builds source context for a node to enable visual diagnostic display.
    ///
    /// Delegates to `build_source_context_from_range` after converting the node to a range.
    fn build_source_context(&self, node: &Node<'_>) -> SourceContext {
        let range = range_for_node(*node);
        self.build_source_context_from_range(&range)
    }

    /// Provides syntax diagnostics for this file.
    ///
    /// ## Returns
    /// Returns syntax diagnostics derived from tree-sitter error nodes.
    pub fn syntax_diagnostics(&self) -> Vec<SyntaxDiagnostic> {
        let root = self.tree.root_node();

        // Fast path: if the tree has no errors, skip traversal entirely
        if !root.has_error() {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            if node.is_error() || node.is_missing() {
                // For missing nodes, node.kind() returns what was expected (e.g., ";" or "identifier")
                let message = if node.is_missing() {
                    format!("Missing expected: {}", node.kind())
                } else {
                    format!("Syntax error: {}", node.kind())
                };

                let range = range_for_node(node);
                let context = self.build_source_context_from_range(&range);

                diagnostics.push(SyntaxDiagnostic {
                    message,
                    range,
                    severity: DiagnosticSeverity::Error,
                    context: Some(context),
                    metadata: Some(self.diagnostic_metadata("invalid-syntax")),
                });
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }

        diagnostics
    }

    /// Provides all diagnostics for this file in a unified format.
    ///
    /// Combines lint diagnostics (pattern-based and semantic) with syntax
    /// diagnostics into a single collection. Each diagnostic includes a
    /// `kind` field indicating its source (`Lint`, `Semantic`, or `Syntax`).
    ///
    /// ## Returns
    /// Returns all diagnostics from lint and syntax analysis.
    ///
    /// ## Examples
    /// ```ignore
    /// let tree_file = TreeFile::new("example.rs")?;
    /// for diagnostic in tree_file.diagnostics() {
    ///     println!("[{}] {}: {}", diagnostic.kind, diagnostic.severity, diagnostic.message);
    /// }
    /// ```
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.try_diagnostics().unwrap_or_default()
    }

    /// Provides all diagnostics for this file in a unified format, surfacing
    /// analyzer failures instead of hiding them.
    ///
    /// ## Returns
    /// Returns all diagnostics from lint and syntax analysis.
    ///
    /// ## Errors
    /// Returns an error if a lint/reference query fails to compile or symbol
    /// extraction fails. An empty list means the file is clean, never that the
    /// analyzer failed to run.
    pub fn try_diagnostics(&self) -> Result<Vec<Diagnostic>, TreeHuggerError> {
        let definitions = self.definitions(&MetadataSelection::ALL)?;
        let imports = self.imported_symbols()?;
        let references = self.references_from(&definitions, &imports)?;
        self.diagnostics_from(&definitions, &imports, &references)
    }

    /// Lint and syntax diagnostics from already extracted definitions,
    /// imports and references.
    pub(crate) fn diagnostics_from(
        &self,
        definitions: &[ExtractedSymbol<'_>],
        imports: &[ImportSymbol],
        references: &[ReferencedSymbol],
    ) -> Result<Vec<Diagnostic>, TreeHuggerError> {
        let mut diagnostics: Vec<Diagnostic> = self
            .lint_diagnostics_from(definitions, imports, references)?
            .into_iter()
            .map(Diagnostic::from_lint)
            .collect();
        diagnostics.extend(self.syntax_diagnostics().into_iter().map(Diagnostic::from_syntax));
        Ok(diagnostics)
    }

    /// Dead code blocks which are unreachable.
    ///
    /// Detects statements that follow an unconditional exit (e.g. `return`,
    /// `break`, `throw`, `panic!`) within the same block. This is the same
    /// analysis surfaced by the `dead-code` lint rule, exposed here as
    /// [`CodeBlock`] values with source snippets.
    ///
    /// ## Returns
    /// Returns the unreachable code blocks, ordered by source position.
    pub fn dead_code(&self) -> Vec<CodeBlock> {
        self.dead_code_ranges()
            .into_iter()
            .map(|range| {
                let snippet = self
                    .source
                    .get(range.start_byte..range.end_byte)
                    .map(str::to_string);
                CodeBlock { range, snippet }
            })
            .collect()
    }

    /// Provides all symbol definitions detected in the file.
    ///
    /// ## Returns
    /// Returns symbol definitions for the file.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError> {
        Ok(self
            .definitions(&MetadataSelection::ALL)?
            .into_iter()
            .map(|extracted| extracted.symbol)
            .collect())
    }

    /// Provides v2 schema symbol records for this file from the parse pass.
    ///
    /// ## Returns
    /// Returns symbol records in Symbol Schema v2 format.
    ///
    /// ## Errors
    /// Returns an error if symbol extraction or query compilation fails.
    pub fn symbol_records(&self) -> Result<Vec<SymbolRecord>, TreeHuggerError> {
        let selection = MetadataSelection::ALL;
        Ok(self.records_from(self.definitions(&selection)?, &selection, RecordFacts::default()))
    }

    /// Converts definitions to parse-pass records, in the same order.
    ///
    /// Every definition yields a record with its identity, ID, spans and
    /// container; the header texts, keyword, type-alias target, implemented
    /// trait and the `facts` asked for are filled only for kinds `selection`
    /// wants.
    pub(crate) fn records_from(
        &self,
        definitions: Vec<ExtractedSymbol<'_>>,
        selection: &MetadataSelection,
        facts: RecordFacts,
    ) -> Vec<SymbolRecord> {
        let root = self.tree.root_node();
        let mut ordinals: HashMap<(SymbolKindV2, String), u32> = HashMap::new();
        let mut records = Vec::new();
        let mut implemented_traits = Vec::new();
        let is_test_file = facts.flags && is_test_file(&self.file, self.language);
        for ExtractedSymbol {
            symbol,
            anchor,
            declaration,
            name,
        } in definitions
        {
            let wanted = selection.wants(symbol.kind);
            let metrics = (wanted && facts.metrics).then(|| {
                symbol_metrics(
                    symbol.kind,
                    declaration,
                    name,
                    symbol.doc_comment.as_deref(),
                    self.language,
                    &self.source,
                )
            });
            let flags = (wanted && facts.flags).then(|| {
                let is_deprecated = is_deprecated(
                    declaration,
                    symbol.doc_comment.as_deref(),
                    self.language,
                    &self.source,
                );
                let is_test = is_test_file
                    || has_test_attribute(declaration, self.language, &self.source)
                    || (self.language == ProgrammingLanguage::Rust
                        && is_in_rust_cfg_test(declaration, &self.source));
                (is_test, is_deprecated)
            });
            let name_span = text_span_from_code_range(&symbol.range);
            let name_bytes = symbol.range.start_byte..symbol.range.end_byte;
            let is_exported = is_exported_definition(&symbol, anchor, root);
            let mut record: SymbolRecord = symbol.into();

            // Same-named declarations of one kind and qualified name (C++
            // overloads, repeated locals) are told apart by their order.
            let ordinal_key = (record.kind, record.identity.qualified_name.clone());
            let ordinal_slot = ordinals.entry(ordinal_key).or_insert(0);
            let sibling_ordinal = *ordinal_slot;
            *ordinal_slot += 1;

            record.source.declaration_span = text_span_for_node(anchor);
            record.source.name_span = Some(name_span);
            record.source.body_span = body_span_for_declaration(anchor);
            record.visibility.is_exported = is_exported;
            if let Some((is_test, is_deprecated)) = flags {
                record.visibility.is_test = is_test;
                record.visibility.is_deprecated = is_deprecated;
            }
            record.metrics = metrics;
            record.identity.stable_key = stable_symbol_key(
                record.language,
                &record.source.file_path,
                record.kind,
                &record.identity.qualified_name,
                sibling_ordinal,
            );
            record.id = symbol_id_from_stable_key(&record.identity.stable_key);

            if wanted {
                record.source.declaration_text = declaration_text(declaration, &self.source);
                record.source.signature_text =
                    signature_text(declaration, name_bytes, self.language, &self.source);
                record.identity.keyword = declaration_keyword(declaration, self.language);
                if record.kind == SymbolKindV2::TypeAlias {
                    record.kind_data = SymbolKindData::TypeAlias(TypeAliasData {
                        aliased_type_text: extract_type_alias_target(declaration, &self.source),
                        aliased_type_ast: None,
                        is_recursive: false,
                    });
                }
            }

            implemented_traits.push(
                (wanted && self.language == ProgrammingLanguage::Rust)
                    .then(|| impl_of_member(declaration))
                    .flatten()
                    .and_then(|impl_item| impl_trait_name(impl_item, &self.source)),
            );
            records.push(record);
        }

        // A member of `impl Trait for Type` records the trait; `id` only when
        // exactly one trait of that name is declared in this file.
        for (idx, trait_name) in implemented_traits.into_iter().enumerate() {
            let Some(trait_name) = trait_name else {
                continue;
            };
            let mut same_file_traits = records.iter().filter(|candidate| {
                candidate.kind == SymbolKindV2::Trait && candidate.identity.name == trait_name
            });
            let id = match (same_file_traits.next(), same_file_traits.next()) {
                (Some(trait_record), None) => Some(trait_record.id.clone()),
                _ => None,
            };
            records[idx].relations.implements.push(SymbolRef {
                id,
                name: trait_name,
                qualified_name: None,
            });
        }

        records
    }

    /// Runs the full v2 analysis pipeline and returns a per-file symbol index.
    ///
    /// This executes the parse, bind, semantic, docs and diagnostics passes:
    /// [`SymbolQuery::complete_index`] over every kind.
    ///
    /// ## Returns
    /// Returns the staged `FileSymbolIndex` output for the file.
    ///
    /// ## Errors
    /// Returns an error if any extraction pass fails.
    pub fn symbol_index_v2(&self) -> Result<FileSymbolIndex, TreeHuggerError> {
        Ok(self.query(&SymbolQuery::complete_index())?.index)
    }

    /// Runs selected analysis passes, after their prerequisites, and returns
    /// a v2 symbol index of every kind.
    ///
    /// ## Returns
    /// Returns `FileSymbolIndex` populated through the requested pass level.
    /// Diagnostics are included only when [`AnalysisPass::Diagnostics`] is
    /// requested.
    ///
    /// ## Errors
    /// Returns an error if any selected pass fails.
    pub fn symbol_index_with_passes(
        &self,
        passes: &[AnalysisPass],
    ) -> Result<FileSymbolIndex, TreeHuggerError> {
        crate::analysis::analyze_file(self, passes)
    }

    /// Runs `query`: plans the passes its capabilities need and returns the
    /// selected records with what was computed.
    ///
    /// ## Errors
    /// Returns an error if any planned pass fails.
    pub fn query(&self, query: &SymbolQuery) -> Result<SymbolQueryResult, TreeHuggerError> {
        crate::analysis::run_query(self, query)
    }
}
