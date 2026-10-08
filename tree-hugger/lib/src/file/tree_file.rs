use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::analysis::counters::{count_diagnostics_run, count_reference_capture};
use crate::analysis::declaration_names::{
    declared_name_tokens, is_scala_import_segment, scala_import_binds, scala_import_path_start, swift_import_selector,
};
use crate::error::TreeHuggerError;
use crate::file::extract::attributes::declaration_attributes;
use crate::file::extract::*;
use crate::file::query::{Capability, SymbolQuery, SymbolQueryResult};
use crate::file::source_file::SourceFile;
use crate::queries::{GrammarRef, QueryKind, format_rule_message, query_for, severity_for_rule};
use crate::rule_registry::{RuleRegistry, diagnostic_metadata_from_rule};
use crate::shared::{
    AnalysisPass, Attribute, CodeBlock, CodeRange, Diagnostic, DiagnosticSeverity, FileSymbolIndex,
    ImportSymbol, LintDiagnostic, ProgrammingLanguage, ReferenceEvidence, ReferencedSymbol, SourceContext, SymbolInfo,
    SymbolKind, SymbolKindData, SymbolKindV2, SymbolRecord, SyntaxDiagnostic,
    normalize_modifiers, stable_symbol_key, symbol_id_from_stable_key,
};

mod definitions;
mod relations;

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

                // A Go path captured beside its alias.
                if self.should_skip_import(node, self.language) {
                    continue;
                }
                let node = swift_selective_import_name(node, self.language).unwrap_or(node);

                let mut name = node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();

                // A captured Go import path names its package by its last segment.
                if self.language == ProgrammingLanguage::Go && matches!(node.kind(), "interpreted_string_literal" | "raw_string_literal") {
                    let path = go_import_path(&name);
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

        // Two patterns may capture one token (a Rust `use a::{self, *}`, a
        // Scala `{_, given}`): it is one import. Matches arrive per pattern,
        // so position is restored here.
        imports.sort_by_key(|import| (import.range.start_byte, import.range.end_byte));
        imports.dedup_by_key(|import| (import.range.start_byte, import.range.end_byte));
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
        let Some(captures) = self.reference_captures()? else {
            return Ok(Vec::new());
        };
        Ok(self.references_from_captures(&captures, definitions, imports))
    }

    /// [`Self::references_from`] over the captures of one
    /// [`Self::reference_captures`] run.
    pub(crate) fn references_from_captures(
        &self,
        captures: &[ReferenceCapture<'_>],
        definitions: &[ExtractedSymbol<'_>],
        imports: &[ImportSymbol],
    ) -> Vec<ReferencedSymbol> {
        let definition_ranges = definition_ranges(definitions, imports);
        captures
            .iter()
            .filter(|capture| capture.is_reference)
            .filter_map(|capture| self.referenced_symbol(capture.node, &definition_ranges))
            .collect()
    }

    /// The `@reference` usage `node` names, unless it is a definition's or
    /// import's own name, definition-like, or empty.
    fn referenced_symbol(
        &self,
        node: Node<'_>,
        definition_ranges: &HashSet<(usize, usize)>,
    ) -> Option<ReferencedSymbol> {
        let range = range_for_node(node);
        if definition_ranges.contains(&(range.start_byte, range.end_byte))
            || is_definition_like_reference(node, self.language)
            || crate::analysis::occurrences::is_ecma_export_name(node, self.language)
        {
            return None;
        }
        let name = node
            .utf8_text(self.source.as_bytes())
            .map(str::to_string)
            .unwrap_or_default();
        if name.is_empty() {
            return None;
        }
        let (is_qualified, qualifier) = self.detect_qualified_reference(node);
        Some(ReferencedSymbol {
            name,
            range,
            language: self.language,
            file: self.file.clone(),
            is_qualified,
            qualifier,
        })
    }

    /// Runs the references query once: one capture per captured node span,
    /// in first-match order, then the member callees behind transparent
    /// wrappers that no pattern reaches
    /// ([`wrapped_member_callees`](crate::analysis::occurrences::wrapped_member_callees)).
    /// `None` when the language's query has no patterns.
    pub(crate) fn reference_captures(
        &self,
    ) -> Result<Option<Vec<ReferenceCapture<'_>>>, TreeHuggerError> {
        let query = query_for(self.grammar_ref(), QueryKind::References)?;
        if query.pattern_count() == 0 {
            return Ok(None);
        }

        count_reference_capture();
        let mut cursor = QueryCursor::new();
        let capture_names = query.capture_names();
        let mut captures: Vec<ReferenceCapture<'_>> = Vec::new();
        let mut by_range: HashMap<(usize, usize), usize> = HashMap::new();
        let mut matches = cursor.matches(query.as_ref(), self.tree.root_node(), self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                let (is_reference, is_member) = match capture_names.get(capture.index as usize) {
                    Some(&"reference") => (true, false),
                    Some(&"reference.member") => (false, true),
                    Some(&"reference.pattern") => (false, false),
                    _ => continue,
                };
                let node = capture.node;
                let range = (node.start_byte(), node.end_byte());
                match by_range.get(&range) {
                    Some(&idx) => {
                        captures[idx].is_reference |= is_reference;
                        captures[idx].is_member |= is_member;
                    }
                    None => {
                        by_range.insert(range, captures.len());
                        captures.push(ReferenceCapture { node, is_reference, is_member });
                    }
                }
            }
            matches.advance();
        }
        for node in crate::analysis::occurrences::wrapped_member_callees(self.tree.root_node(), self.language) {
            let range = (node.start_byte(), node.end_byte());
            match by_range.get(&range) {
                Some(&idx) => captures[idx].is_member = true,
                None => {
                    by_range.insert(range, captures.len());
                    captures.push(ReferenceCapture { node, is_reference: false, is_member: true });
                }
            }
        }

        Ok(Some(captures))
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
    /// Go: an aliased import (a blank `_` included) is captured at both its
    /// alias and its path, and the path is dropped. Scala: every flat path
    /// segment is captured, since whether one binds depends on what follows
    /// it past any comment, and only [`scala_import_binds`] segments are
    /// kept. Every other query captures exactly the name each import brings
    /// in.
    fn should_skip_import(&self, node: Node, language: ProgrammingLanguage) -> bool {
        match language {
            ProgrammingLanguage::Scala => is_scala_import_segment(node) && !scala_import_binds(node),
            // A dot import (`. "a/fmt"`) has no alias capture, so its path
            // is kept.
            ProgrammingLanguage::Go => {
                matches!(node.kind(), "interpreted_string_literal" | "raw_string_literal")
                    && find_ancestor_by_kind(node, "import_spec")
                        .and_then(|import_spec| import_spec.child_by_field_name("name"))
                        .is_some_and(|name| matches!(name.kind(), "package_identifier" | "blank_identifier"))
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
            ProgrammingLanguage::Php => self.extract_php_import_metadata(node, name),
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
                find_ancestor_by_kinds(node, &["import_statement", "import_alias"])
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
    /// `import * as ns from "module"`, `import foo from "module"`, and
    /// TypeScript `import foo = require("module")` and `import Foo = N.S`.
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

        let parent = node.parent();
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        // TypeScript `import q = require("m");` names its module beside the
        // binding; `import R = N.S;` aliases an entity path, kept as the
        // original name.
        if let Some(clause) = parent.filter(|parent| parent.kind() == "import_require_clause") {
            let module = clause
                .child_by_field_name("source")
                .and_then(text)
                .map(|module| module.trim_matches('"').trim_matches('\'').to_string());
            return (module, None, None);
        }
        // The target is the entity after `=`, whatever comments surround
        // it, spelled without them: its last segment is the original name
        // the recursion rule compares.
        if let Some(alias) = parent.filter(|parent| parent.kind() == "import_alias") {
            let mut cursor = alias.walk();
            let target = alias
                .children(&mut cursor)
                .find(|child| child.kind() == "=")
                .and_then(next_named_syntax_sibling)
                .map(|target| tokens_without_extras(target, &self.source));
            return (None, target, Some(name.to_string()));
        }

        // Check if this is an aliased import (import_specifier with name and alias)
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

    /// Extracts import metadata for Python. `source` is the module path as
    /// written (`a.b` for `import a.b`, whose name is `a`; `m` for
    /// `from m import x`; `__future__` for a future import); an alias gives
    /// `alias` and `original_name`, the module path for `import a.b as c`
    /// and the imported name for `from m import x as y`. An on-demand
    /// `from a.b import *` is named by `b` (or the relative prefix `.`) with
    /// `alias` that name, `original_name` `*` and `source` `a.b`.
    fn extract_python_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        let Some(statement) =
            find_ancestor_by_kinds(node, &["import_statement", "import_from_statement", "future_import_statement"])
        else {
            return (None, None, None);
        };
        let module = || match statement.kind() {
            "import_from_statement" => statement.child_by_field_name("module_name").and_then(text),
            "future_import_statement" => Some("__future__".to_string()),
            _ => None,
        };
        let parent = node.parent();
        if let Some(aliased) = parent.filter(|parent| parent.kind() == "aliased_import") {
            let target = aliased.child_by_field_name("name").and_then(text);
            return match statement.kind() {
                "import_statement" => (target.clone(), target, Some(name.to_string())),
                _ => (module(), target, Some(name.to_string())),
            };
        }
        let is_wildcard = {
            let mut cursor = statement.walk();
            statement.children(&mut cursor).any(|child| child.kind() == "wildcard_import")
        };
        if is_wildcard && statement.kind() == "import_from_statement" {
            return (module(), Some("*".to_string()), Some(name.to_string()));
        }
        match statement.kind() {
            "import_statement" => (parent.and_then(text), None, None),
            _ => (module(), None, None),
        }
    }

    /// Extracts import metadata for Rust. `source` is the path the bound
    /// name is imported from (`std::io` for `use std::io::Read`, `a::g` for
    /// `h` in `use a::{g::h}`, none for `use a;`); an alias gives `alias`
    /// and `original_name`, the aliased path's last segment (`e` in
    /// `e as f`, `c` in `b::c::{self as cc}`, `y` in `extern crate y as
    /// z`). A glob is named by its path's last segment, with `alias` that
    /// name, `original_name` `*` and `source` the whole path (`a::b` for
    /// `use a::b::*`).
    fn extract_rust_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        let parent = node.parent();
        if let Some(declaration) = parent.filter(|parent| parent.kind() == "extern_crate_declaration") {
            let original = declaration
                .child_by_field_name("alias")
                .is_some_and(|alias| alias.id() == node.id())
                .then(|| declaration.child_by_field_name("name").and_then(text))
                .flatten();
            let alias = original.as_ref().map(|_| name.to_string());
            return (None, original, alias);
        }
        if let Some(clause) = parent.filter(|parent| parent.kind() == "use_as_clause") {
            let Some(original) = clause.child_by_field_name("path").and_then(|path| self.rust_use_segment(path)) else {
                return (None, None, Some(name.to_string()));
            };
            return (self.rust_use_prefix(original), text(original), Some(name.to_string()));
        }
        if is_rust_glob_path(node) {
            let path = match self.rust_use_prefix(node) {
                Some(prefix) => format!("{prefix}::{name}"),
                None => name.to_string(),
            };
            return (Some(path), Some("*".to_string()), Some(name.to_string()));
        }
        (self.rust_use_prefix(node), None, None)
    }

    /// The last segment of a `use` path: the node itself, a scoped path's
    /// name, or for `self` its list's path's last segment.
    fn rust_use_segment<'tree>(&self, path: Node<'tree>) -> Option<Node<'tree>> {
        match path.kind() {
            "scoped_identifier" => path.child_by_field_name("name"),
            "self" => {
                let entry = path.parent().filter(|parent| parent.kind() == "use_as_clause").unwrap_or(path);
                let list = entry.parent().filter(|parent| parent.kind() == "use_list")?;
                let owner = list.parent().filter(|parent| parent.kind() == "scoped_use_list")?;
                owner.child_by_field_name("path").and_then(|path| self.rust_use_segment(path))
            }
            _ => Some(path),
        }
    }

    /// The path written before the `use` segment `segment`, across the
    /// lists around it (`a::g` for `h` in `use a::{g::h}`); `None` when
    /// nothing is written before it.
    fn rust_use_prefix(&self, segment: Node) -> Option<String> {
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok();
        let mut parts = Vec::new();
        let mut current = segment;
        while let Some(parent) = current.parent() {
            match parent.kind() {
                "scoped_identifier"
                    if parent.child_by_field_name("name").is_some_and(|name| name.id() == current.id()) =>
                {
                    parts.extend(parent.child_by_field_name("path").and_then(text));
                }
                "use_list" => {
                    if let Some(owner) = parent.parent().filter(|owner| owner.kind() == "scoped_use_list") {
                        parts.extend(owner.child_by_field_name("path").and_then(text));
                        current = owner;
                        continue;
                    }
                }
                "use_declaration" => break,
                _ => {}
            }
            current = parent;
        }
        parts.reverse();
        (!parts.is_empty()).then(|| parts.join("::"))
    }

    /// Extracts import metadata for Go: `source` is the path; an alias
    /// (`f "a/fmt"`, a blank `_ "a/fmt"`) gives `alias` and the path's last
    /// segment as `original_name`; a dot import (`. "a/fmt"`), named by the
    /// path's last segment, is on demand: `alias` that name and
    /// `original_name` `*`.
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
                source = Some(go_import_path(path_text).to_string());
            }

            // Check if there's an alias (name field)
            if let Some(name_node) = spec.child_by_field_name("name") {
                let alias_text = name_node
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or_default();
                if name_node.kind() == "dot" {
                    alias = Some(name.to_string());
                    original_name = Some("*".to_string());
                } else if alias_text == name {
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
    /// On-demand paths, including a single identifier, retain the complete source
    /// and represent `*` under the path's final identifier.
    fn extract_java_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut source = None;
        let mut original_name = None;
        let mut alias = None;

        let import_decl = find_ancestor_by_kind(node, "import_declaration");
        if let Some(decl) = import_decl {
            let is_wildcard = decl
                .children(&mut decl.walk())
                .any(|c| c.kind() == "asterisk");

            for child in decl.children(&mut decl.walk()) {
                if matches!(child.kind(), "scoped_identifier" | "identifier") {
                    let full_path = child.utf8_text(self.source.as_bytes()).unwrap_or_default();

                    if is_wildcard {
                        source = Some(full_path.to_string());
                        original_name = Some("*".to_string());
                        alias = Some(
                            node.utf8_text(self.source.as_bytes())
                                .unwrap_or_default()
                                .to_string(),
                        );
                    } else {
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

    /// Extracts import metadata for C#: `source` is the directive's path or
    /// alias target as written (`using A.B;` → `A.B`, `using E = F.G;` →
    /// `F.G`); an alias directive also gives `alias` and, when its target
    /// is a name, `original_name` (its final segment, `G`).
    fn extract_csharp_import_metadata(
        &self,
        node: Node,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let Some(directive) = find_ancestor_by_kind(node, "using_directive") else {
            return (None, None, None);
        };
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        let alias_name = directive.child_by_field_name("name");
        let mut cursor = directive.walk();
        let Some(target) = directive
            .named_children(&mut cursor)
            .find(|child| Some(child.id()) != alias_name.map(|name| name.id()) && !child.is_extra())
        else {
            return (None, None, None);
        };
        let source = text(target);
        let Some(alias_name) = alias_name else {
            return (source, None, None);
        };
        let original = csharp_final_name(target).and_then(text);
        (source, original, text(alias_name))
    }

    /// Extracts import metadata for PHP: `source` is the clause's path as
    /// written, with a group's prefix (`A\B` for `B` in `use A\{B};`);
    /// an alias gives `alias` and `original_name`, the path's last segment
    /// (`B` in `use A\B as C;`).
    fn extract_php_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        let Some(clause) = find_ancestor_by_kind(node, "namespace_use_clause") else {
            return (None, None, None);
        };
        let alias_node = clause.child_by_field_name("alias");
        let mut cursor = clause.walk();
        let path = clause.named_children(&mut cursor).find(|child| {
            matches!(child.kind(), "name" | "qualified_name") && Some(child.id()) != alias_node.map(|alias| alias.id())
        });
        let prefix = clause
            .parent()
            .filter(|parent| parent.kind() == "namespace_use_group")
            .and_then(|group| group.parent())
            .and_then(|declaration| find_child_by_kind(declaration, "namespace_name"))
            .and_then(text);
        let source = path.and_then(text).map(|path| match prefix {
            Some(prefix) => format!("{prefix}\\{path}"),
            None => path,
        });
        if alias_node.is_none() {
            return (source, None, None);
        }
        // A qualified name's last segment is its last `name` child.
        let original = path
            .and_then(|path| match path.kind() {
                "qualified_name" => {
                    let mut cursor = path.walk();
                    path.named_children(&mut cursor).filter(|part| part.kind() == "name").last()
                }
                _ => Some(path),
            })
            .and_then(text);
        (source, original, Some(name.to_string()))
    }

    /// Extracts import metadata for Scala. `source` is the bound name's path
    /// as written (`a.b.C`), or for a rename/selector its path and original name
    /// (`a.D` for `E` in `import a.{D => E}`); a rename gives `alias` and
    /// `original_name` (`D`). An on-demand import (`a.b.*`, `a.b._`,
    /// `a.{_, given T}`) is named by its path's last segment, with `alias`
    /// that name, `original_name` `*` (or `given` when it opens only given
    /// instances) and `source` the path (`a.b`).
    fn extract_scala_import_metadata(
        &self,
        node: Node,
        name: &str,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let text = |node: Node| node.utf8_text(self.source.as_bytes()).ok().map(str::to_string);
        let Some(parent) = node.parent() else {
            return (None, None, None);
        };
        let selector_source = |selectors: Node, selected: &str| {
            let path = prev_named_syntax_sibling(selectors).and_then(|last| self.scala_import_path(last))?;
            Some(format!("{path}.{selected}"))
        };
        match parent.kind() {
            "arrow_renamed_identifier" | "as_renamed_identifier" => {
                let original = parent.child_by_field_name("name").and_then(text);
                let source = parent.parent().zip(original.as_deref()).and_then(|(context, original)| {
                    if context.kind() == "namespace_selectors" {
                        selector_source(context, original)
                    } else {
                        selector_source(parent, original)
                    }
                });
                (source, original, Some(name.to_string()))
            }
            "namespace_selectors" => (selector_source(parent, name), None, None),
            _ => {
                let path = self.scala_import_path(node);
                let opened = next_named_syntax_sibling(node).and_then(|next| match next.kind() {
                    "namespace_wildcard" => Some(scala_wildcard(next.utf8_text(self.source.as_bytes()).unwrap_or_default())),
                    "namespace_selectors" => {
                        let mut cursor = next.walk();
                        let mut kinds = next.children(&mut cursor).filter_map(|selector| match selector.kind() {
                            "namespace_wildcard" => {
                                Some(scala_wildcard(selector.utf8_text(self.source.as_bytes()).unwrap_or_default()))
                            }
                            "given" => Some("given"),
                            _ => None,
                        });
                        let first = kinds.next()?;
                        Some(if first == "*" || kinds.any(|kind| kind == "*") { "*" } else { first })
                    }
                    _ => None,
                });
                match opened {
                    Some(opened) => (path, Some(opened.to_string()), Some(name.to_string())),
                    None => (path, None, None),
                }
            }
        }
    }

    /// The written path of the Scala import segment `segment`: from the
    /// first segment of its `,`-separated expression through `segment`.
    fn scala_import_path(&self, segment: Node) -> Option<String> {
        let start = scala_import_path_start(segment).unwrap_or(segment);
        self.source.get(start.start_byte()..segment.end_byte()).map(str::to_string)
    }

    /// Extracts import metadata for Swift: the `source` is the path as
    /// written (`Foundation`, `Foo.Sub`, `Foo.Bar` in `import struct
    /// Foo.Bar`, whose name is `Bar`).
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
        let defined_names: std::collections::HashSet<String> = definitions
            .iter()
            .map(|extracted| crate::analysis::name_identity::key(self.language, &extracted.symbol.name).into_owned())
            .collect();
        let imported_names: std::collections::HashSet<String> =
            imports.iter().map(|i| crate::analysis::name_identity::key(self.language, i.alias.as_deref().unwrap_or(&i.name)).into_owned()).collect();
        let exported_names: std::collections::HashSet<String> = self
            .exports_from(definitions)
            .map(|symbol| crate::analysis::name_identity::key(self.language, &symbol.name).into_owned())
            .collect();
        let referenced_names: std::collections::HashSet<String> =
            references.iter().map(|r| crate::analysis::name_identity::key(self.language, &r.name).into_owned()).collect();

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
        defined_names: &std::collections::HashSet<String>,
        imported_names: &std::collections::HashSet<String>,
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
            if defined_names.contains(crate::analysis::name_identity::key(self.language, name).as_ref()) {
                continue;
            }

            // Skip if imported
            if imported_names.contains(crate::analysis::name_identity::key(self.language, name).as_ref()) {
                continue;
            }

            // Skip if it's a language builtin
            if is_builtin(self.language, crate::analysis::name_identity::key(self.language, name).as_ref()) {
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
        exported_names: &std::collections::HashSet<String>,
        referenced_names: &std::collections::HashSet<String>,
    ) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for definition in definitions {
            let name = definition.name.as_str();

            // Skip if exported
            if exported_names.contains(crate::analysis::name_identity::key(self.language, name).as_ref()) {
                continue;
            }

            // Skip if referenced
            if referenced_names.contains(crate::analysis::name_identity::key(self.language, name).as_ref()) {
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
        referenced_names: &std::collections::HashSet<String>,
    ) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for import in imports {
            let name = import.name.as_str();

            // Skip if referenced
            if referenced_names.contains(crate::analysis::name_identity::key(self.language, name).as_ref()) {
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

            // An on-demand import (`import a.b.*`, `use a::*`, `from a
            // import *`) binds no name of its own to use; a TS/JS
            // `import * as ns` binds `ns`.
            let is_ecma = matches!(import.language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript);
            if !is_ecma
                && (import.original_name.as_deref() == Some("*")
                    || (import.language == ProgrammingLanguage::Scala && import.original_name.as_deref() == Some("given")))
            {
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
        defined_names: &std::collections::HashSet<String>,
        imported_names: &std::collections::HashSet<String>,
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
            if defined_names.contains(crate::analysis::name_identity::key(self.language, qualifier).as_ref()) {
                continue;
            }

            // Skip if the qualifier is imported
            if imported_names.contains(crate::analysis::name_identity::key(self.language, qualifier).as_ref()) {
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
        self.records_from(self.definitions(&selection)?, &selection, RecordFacts::default())
    }

    /// Converts definitions to parse-pass records, in the same order.
    ///
    /// Every definition yields a record with its identity, ID, spans and
    /// container; the header texts, keyword, type-alias target, generic
    /// parameters, attributes, written modifiers, declared bases and the
    /// `facts` asked for are filled only for kinds `selection` wants. Base
    /// targets resolve against every record, wanted or not.
    ///
    /// ## Errors
    ///
    /// [`TreeHuggerError::TooManyGenericParameters`] when a selected
    /// declaration's generic parameter positions do not fit in `u16`.
    pub(crate) fn records_from(
        &self,
        definitions: Vec<ExtractedSymbol<'_>>,
        selection: &MetadataSelection,
        facts: RecordFacts,
    ) -> Result<Vec<SymbolRecord>, TreeHuggerError> {
        let root = self.tree.root_node();
        let mut ordinals: HashMap<(SymbolKindV2, String), u32> = HashMap::new();
        let mut records = Vec::new();
        let mut relation_inputs = Vec::new();
        // Inner `#![cfg(test)]` state per file root and module body, shared
        // by every declaration below them.
        let mut inner_cfg_test = HashMap::new();
        for ExtractedSymbol {
            symbol,
            anchor,
            declaration,
            name,
            container_impl,
            container_extension,
        } in definitions
        {
            let wanted = selection.wants(symbol.kind);
            // `metrics.lines` and `metrics.cyclomatic` measure this body, so
            // they never disagree with the stored span.
            let callable = symbol
                .kind
                .is_function()
                .then(|| callable_body(declaration, self.language))
                .flatten();
            let body_span = if symbol.kind.is_function() {
                callable.map(|body| body.span(&self.source))
            } else {
                body_span_for_declaration(anchor)
            };
            let metrics = (wanted && facts.metrics).then(|| {
                symbol_metrics(
                    symbol.kind,
                    declaration,
                    name,
                    symbol.doc_comment.as_deref(),
                    callable,
                    self.language,
                    &self.source,
                )
            });
            // Parsed once per selected declaration: the record's
            // `attributes` and both flag readers take this list rather than
            // reading the tree again.
            let attributes = if wanted {
                declaration_attributes(declaration, name, self.language, &self.source)
            } else {
                Vec::new()
            };
            let flags = (wanted && facts.flags).then(|| {
                let is_deprecated =
                    is_deprecated(&attributes, symbol.doc_comment.as_deref(), self.language);
                // The file-wide rules (`is_test_file`) read the path and
                // the package layout around it, which the cache does not
                // key on; `SymbolQueryResult::finish` applies them after
                // lookup.
                let is_test = has_test_attribute(&attributes, self.language)
                    || (self.language == ProgrammingLanguage::Rust
                        && is_in_rust_cfg_test(declaration, &self.source, &mut inner_cfg_test));
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
            record.source.body_span = body_span;
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
                let generics = declared_generics(declaration, name, self.language, &self.source)
                    .map_err(|overflow| TreeHuggerError::TooManyGenericParameters {
                        path: self.file.clone(),
                        symbol: record.identity.name.clone(),
                        count: overflow.count,
                    })?;
                record.type_info.generic_params = generics.params;
                record.type_info.constraints = generics.constraints;
                record.attributes = attributes.iter().map(Attribute::from).collect();
                record
                    .modifiers
                    .extend(declaration_modifiers(declaration, name, self.language));
                if record.kind == SymbolKindV2::TypeAlias {
                    record.kind_data = SymbolKindData::TypeAlias(TypeAliasData {
                        aliased_type_text: extract_type_alias_target(declaration, self.language, &self.source),
                        aliased_type_ast: None,
                        is_recursive: false,
                    });
                }
            }

            // The one point where a record's modifiers are put in order.
            normalize_modifiers(&mut record.modifiers);

            relation_inputs.push(relations::RelationInput {
                declaration,
                anchor,
                name,
                wanted,
                container_impl,
                container_extension,
            });
            records.push(record);
        }

        relations::link_relations(
            &mut records,
            &relation_inputs,
            root,
            self.language,
            &self.source,
            facts.containment,
        );

        Ok(records)
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

    /// Every reference in this file as an occurrence (owner, kind, target
    /// and confidence), with the coverage of each owning region, over the
    /// whole file. Rust, TypeScript and JavaScript resolve names through
    /// their lexical scopes; every other language reports occurrences that
    /// are never `Scoped` and `Limited` coverage.
    ///
    /// Owner, target and candidate IDs are those of [`Self::symbol_index_v2`].
    ///
    /// ## Errors
    /// Returns an error if a query fails to compile or a record cannot be
    /// built.
    pub fn reference_evidence(&self) -> Result<ReferenceEvidence, TreeHuggerError> {
        let query = SymbolQuery::new()
            .kinds([])
            .capabilities([Capability::Occurrences]);
        let index = crate::analysis::run_query(self, &query)?.index;
        Ok(ReferenceEvidence {
            occurrences: index.occurrences,
            reference_coverage: index.reference_coverage,
        })
    }

    /// Runs `query`: plans the passes its capabilities need and returns the
    /// selected records that pass its predicates, with what was computed and
    /// qualified names prefixed per its naming inputs.
    ///
    /// ## Errors
    /// Returns an error if any planned pass fails.
    pub fn query(&self, query: &SymbolQuery) -> Result<SymbolQueryResult, TreeHuggerError> {
        Ok(crate::analysis::run_query(self, query)?.finish(query))
    }
}

/// The identifier a C# type name ends in (`G` in `F.G`, `B` in `A.B<C>`,
/// `T` in `global::T`), or `None` for a type that is no name (`int*`, a
/// tuple, a predefined type).
fn csharp_final_name(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" => Some(node),
        "generic_name" => find_child_by_kind(node, "identifier"),
        "qualified_name" | "alias_qualified_name" => {
            node.child_by_field_name("name").and_then(csharp_final_name)
        }
        _ => None,
    }
}

/// One node the references query captured.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferenceCapture<'tree> {
    pub(crate) node: Node<'tree>,
    /// Captured as `@reference`: an identifier usage, read by the
    /// diagnostics and by reference occurrences.
    pub(crate) is_reference: bool,
    /// Captured as `@reference.member`: a method-call or constructor member
    /// name, read only by reference occurrences. A capture that is neither
    /// (`@reference.pattern`, a Rust struct-pattern field shorthand) is read
    /// only by reference occurrences, as a plain name.
    pub(crate) is_member: bool,
}

/// The name a Swift selective import (`import struct Foo.Bar`) binds: the
/// last segment of its captured path. The path stays the import's `source`;
/// a whole-module import (`import Foo.Sub`) is named by its whole path.
fn swift_selective_import_name(path: Node<'_>, language: ProgrammingLanguage) -> Option<Node<'_>> {
    if language != ProgrammingLanguage::Swift || path.kind() != "identifier" {
        return None;
    }
    path.parent().and_then(swift_import_selector)?;
    let mut cursor = path.walk();
    path.named_children(&mut cursor).filter(|segment| segment.kind() == "simple_identifier").last()
}

/// Whether the Rust import token `node` names the path a glob opens: the
/// path's last segment in `use a::b::*;` or `use a::{*};`, never in a list
/// that also holds `self` (`use a::{self, *};` imports `a` itself).
fn is_rust_glob_path(node: Node<'_>) -> bool {
    let path = match node.parent() {
        Some(parent)
            if parent.kind() == "scoped_identifier"
                && parent.child_by_field_name("name").is_some_and(|name| name.id() == node.id()) =>
        {
            parent
        }
        _ => node,
    };
    let Some(parent) = path.parent() else { return false };
    match parent.kind() {
        "use_wildcard" => true,
        "scoped_use_list" => {
            let Some(list) = parent.child_by_field_name("list") else { return false };
            let mut cursor = list.walk();
            let entries: Vec<Node<'_>> = list.named_children(&mut cursor).collect();
            !entries.iter().any(|entry| entry.kind() == "self")
                && entries.iter().any(|entry| entry.kind() == "use_wildcard" && entry.named_child_count() == 0)
        }
        _ => false,
    }
}

/// A Go import path without its quotes: `"a/b"` or the raw `` `a/b` ``.
pub(crate) fn go_import_path(literal: &str) -> &str {
    ['"', '`']
        .iter()
        .find_map(|quote| literal.strip_prefix(*quote).and_then(|path| path.strip_suffix(*quote)))
        .unwrap_or(literal)
}

/// The `original_name` of a Scala on-demand import: `given` when it opens
/// only given instances, else `*` (`*` and Scala 2's `_` alike).
fn scala_wildcard(written: &str) -> &'static str {
    if written == "given" { "given" } else { "*" }
}

/// The name spans of `definitions` and `imports`, which are never
/// references: each record's name span plus, for a declared path
/// (`namespace A.B`) or a C++ qualified type name, every name token it
/// declares ([`declared_name_tokens`]), since a reference capture takes the
/// tokens one by one.
pub(crate) fn definition_ranges(
    definitions: &[ExtractedSymbol<'_>],
    imports: &[ImportSymbol],
) -> HashSet<(usize, usize)> {
    definitions
        .iter()
        .flat_map(|extracted| {
            std::iter::once((extracted.symbol.range.start_byte, extracted.symbol.range.end_byte)).chain(
                declared_name_tokens(extracted.name)
                    .into_iter()
                    .map(|token| (token.start_byte(), token.end_byte())),
            )
        })
        .chain(imports.iter().map(|import| (import.range.start_byte, import.range.end_byte)))
        .collect()
}
