use std::path::{Path, PathBuf};

use biscuit_hash::xx_hash;
use tree_sitter::{Node, Parser, QueryCursor, StreamingIterator};

use crate::error::TreeHuggerError;
use crate::queries::{QueryKind, query_for};
use crate::shared::{
    CodeBlock, CodeRange, DiagnosticSeverity, FunctionSignature, ImportSymbol, LintDiagnostic,
    ParameterInfo, ProgrammingLanguage, SymbolInfo, SymbolKind, SyntaxDiagnostic,
};

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
        let file = path.as_ref().to_path_buf();
        let source = std::fs::read_to_string(&file).map_err(|source| TreeHuggerError::Io {
            path: file.clone(),
            source,
        })?;

        let (detected_language, tree_language) = match language {
            Some(language) => (language, language.tree_sitter_language()),
            None => ProgrammingLanguage::detect(&file)
                .ok_or_else(|| TreeHuggerError::UnsupportedLanguage { path: file.clone() })?,
        };

        let mut parser = Parser::new();
        parser
            .set_language(&tree_language)
            .map_err(|_| TreeHuggerError::UnsupportedLanguage { path: file.clone() })?;

        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| TreeHuggerError::ParseFailed { path: file.clone() })?;

        let hash = format!("{:x}", xx_hash(&source));

        Ok(Self {
            file,
            language: detected_language,
            hash,
            source,
            tree,
        })
    }

    /// Provides the list of symbols imported by this file.
    ///
    /// ## Returns
    /// Returns the imported symbols that tree-sitter can detect.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn imported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError> {
        let query = query_for(self.language, QueryKind::Imports)?;
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

                let name = capture
                    .node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();

                imports.push(ImportSymbol {
                    name,
                    range: range_for_node(capture.node),
                    language: self.language,
                    file: self.file.clone(),
                    source: None,
                });
            }

            matches.advance();
        }

        Ok(imports)
    }

    /// Provides the list of symbols exported by this file.
    ///
    /// ## Returns
    /// Returns exported symbols derived from top-level definitions.
    ///
    /// ## Errors
    /// Returns an error if query compilation fails.
    pub fn exported_symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError> {
        let symbols = self.symbol_nodes()?;
        let root = self.tree.root_node();

        Ok(symbols
            .into_iter()
            .filter(|(symbol, node)| is_exported_definition(symbol, *node, root))
            .map(|(symbol, _)| symbol)
            .collect())
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
        let symbols = self.symbol_nodes()?;
        let root = self.tree.root_node();

        Ok(symbols
            .into_iter()
            .filter(|(symbol, node)| !is_exported_definition(symbol, *node, root))
            .map(|(symbol, _)| symbol)
            .collect())
    }

    /// Provides lint diagnostics for this file.
    ///
    /// ## Returns
    /// Returns lint diagnostics when query patterns are available.
    pub fn lint_diagnostics(&self) -> Vec<LintDiagnostic> {
        Vec::new()
    }

    /// Provides syntax diagnostics for this file.
    ///
    /// ## Returns
    /// Returns syntax diagnostics derived from tree-sitter error nodes.
    pub fn syntax_diagnostics(&self) -> Vec<SyntaxDiagnostic> {
        let mut diagnostics = Vec::new();
        let root = self.tree.root_node();
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            if node.is_error() || node.is_missing() {
                let message = if node.is_missing() {
                    "Missing syntax node".to_string()
                } else {
                    format!("Syntax error: {}", node.kind())
                };

                diagnostics.push(SyntaxDiagnostic {
                    message,
                    range: range_for_node(node),
                    severity: DiagnosticSeverity::Error,
                });
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }

        diagnostics
    }

    /// Dead code blocks which are unreachable.
    ///
    /// ## Returns
    /// Returns an empty list until dead-code queries are defined.
    pub fn dead_code(&self) -> Vec<CodeBlock> {
        Vec::new()
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
            .symbol_nodes()?
            .into_iter()
            .map(|(symbol, _)| symbol)
            .collect())
    }

    fn symbol_nodes(&self) -> Result<Vec<(SymbolInfo, Node<'_>)>, TreeHuggerError> {
        let query = query_for(self.language, QueryKind::Locals)?;
        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        let mut symbols = Vec::new();

        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            // First pass: collect context nodes for this match
            let mut context_node: Option<Node<'_>> = None;
            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                if capture_name.ends_with(".context") {
                    context_node = Some(capture.node);
                    break;
                }
            }

            // Second pass: process symbol definitions
            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                if capture_name == "local.definition.import" {
                    continue;
                }

                let kind = match symbol_kind_from_capture(capture_name) {
                    Some(kind) => kind,
                    None => continue,
                };

                let name = capture
                    .node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();

                // Extract signature and doc comment if we have a context node
                let (signature, doc_comment) = if let Some(ctx) = context_node {
                    let sig = if kind.is_function() {
                        extract_signature(ctx, self.language, &self.source)
                    } else {
                        None
                    };
                    let doc = extract_doc_comment(ctx, self.language, &self.source);
                    (sig, doc)
                } else {
                    (None, None)
                };

                symbols.push((
                    SymbolInfo {
                        name,
                        kind,
                        range: range_for_node(capture.node),
                        language: self.language,
                        file: self.file.clone(),
                        doc_comment,
                        signature,
                    },
                    capture.node,
                ));
            }

            matches.advance();
        }

        Ok(symbols)
    }
}

fn symbol_kind_from_capture(capture_name: &str) -> Option<SymbolKind> {
    let suffix = if let Some(rest) = capture_name.strip_prefix("local.definition.") {
        rest
    } else if capture_name == "local.definition" {
        return Some(SymbolKind::Variable);
    } else {
        return None;
    };

    // Skip context captures (e.g., function.context, type.context)
    // These are used for extracting signatures and doc comments in Phase 3
    if suffix.ends_with(".context") {
        return None;
    }

    match suffix {
        "function" => Some(SymbolKind::Function),
        "method" => Some(SymbolKind::Method),
        "type" => Some(SymbolKind::Type),
        "class" => Some(SymbolKind::Class),
        "interface" => Some(SymbolKind::Interface),
        "enum" => Some(SymbolKind::Enum),
        "trait" => Some(SymbolKind::Trait),
        "namespace" => Some(SymbolKind::Namespace),
        "module" => Some(SymbolKind::Module),
        "var" => Some(SymbolKind::Variable),
        "parameter" => Some(SymbolKind::Parameter),
        "field" => Some(SymbolKind::Field),
        "macro" => Some(SymbolKind::Macro),
        "const" | "constant" => Some(SymbolKind::Constant),
        "import" | "associated" => None,
        _ => Some(SymbolKind::Unknown),
    }
}

fn range_for_node(node: Node<'_>) -> CodeRange {
    let start = node.start_position();
    let end = node.end_position();

    CodeRange {
        start_line: start.row.saturating_add(1),
        start_column: start.column.saturating_add(1),
        end_line: end.row.saturating_add(1),
        end_column: end.column.saturating_add(1),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}

fn is_exported_definition(symbol: &SymbolInfo, node: Node<'_>, root: Node<'_>) -> bool {
    if matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Field) {
        return false;
    }

    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent == root {
            return true;
        }

        let parent_kind = parent.kind();

        if is_export_node(parent_kind) {
            return true;
        }

        current = parent;
    }

    false
}

fn is_export_node(kind: &str) -> bool {
    matches!(
        kind,
        "export_statement"
            | "export_declaration"
            | "export_specifier"
            | "named_exports"
            | "export_from_clause"
            | "export_default_declaration"
            | "public_field_definition"
            | "public_declaration"
    )
}

/// Extracts function signature from a function/method node.
fn extract_signature(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<FunctionSignature> {
    let parameters = extract_parameters(node, language, source);
    let return_type = extract_return_type(node, language, source);

    if parameters.is_empty() && return_type.is_none() {
        return None;
    }

    Some(FunctionSignature {
        parameters,
        return_type,
    })
}

/// Extracts parameters from a function node.
fn extract_parameters(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<ParameterInfo> {
    let params_node_kind = match language {
        ProgrammingLanguage::Rust => "parameters",
        ProgrammingLanguage::Python => "parameters",
        ProgrammingLanguage::Go => "parameter_list",
        ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::TypeScript => "formal_parameters",
        _ => return Vec::new(),
    };

    // For Go, methods have TWO parameter_list nodes:
    // 1. receiver (g *Greeter)
    // 2. actual parameters (name string)
    // We need to find the SECOND parameter_list for methods.
    let params_node = if language == ProgrammingLanguage::Go && node.kind() == "method_declaration" {
        find_nth_child_by_kind(node, params_node_kind, 1) // 0-indexed, so 1 = second
    } else {
        find_child_by_kind(node, params_node_kind)
    };

    let params_node = match params_node {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        if let Some(param) = extract_single_parameter(child, language, source) {
            parameters.push(param);
        }
    }

    parameters
}

/// Extracts a single parameter from a parameter node.
fn extract_single_parameter(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ParameterInfo> {
    let kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => extract_rust_parameter(node, source),
        ProgrammingLanguage::Python => extract_python_parameter(node, source),
        ProgrammingLanguage::Go => extract_go_parameter(node, source),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            extract_js_parameter(node, kind, source)
        }
        _ => None,
    }
}

fn extract_rust_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    if kind == "self_parameter" {
        return Some(ParameterInfo::new("self"));
    }

    if kind != "parameter" {
        return None;
    }

    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    let type_annotation = find_child_by_kind(node, "type_identifier")
        .or_else(|| find_child_by_kind(node, "reference_type"))
        .or_else(|| find_child_by_kind(node, "generic_type"))
        .or_else(|| find_child_by_kind(node, "scoped_type_identifier"))
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
        is_variadic: false,
    })
}

fn extract_python_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    // Check if this typed_parameter contains a splat pattern (variadic)
    let has_list_splat = find_child_by_kind(node, "list_splat_pattern").is_some();
    let has_dict_splat = find_child_by_kind(node, "dictionary_splat_pattern").is_some();
    let is_splat = has_list_splat || has_dict_splat;

    // Handle different parameter types
    let (name_node, type_node, default_node, is_variadic) = match kind {
        "identifier" => (Some(node), None, None, false),
        "typed_parameter" => {
            // typed_parameter can contain a splat pattern: *names: str
            let name = if let Some(splat) = find_child_by_kind(node, "list_splat_pattern") {
                find_child_by_kind(splat, "identifier")
            } else if let Some(splat) = find_child_by_kind(node, "dictionary_splat_pattern") {
                find_child_by_kind(splat, "identifier")
            } else {
                find_child_by_kind(node, "identifier")
            };
            (name, find_child_by_kind(node, "type"), None, is_splat)
        }
        "default_parameter" => (
            find_child_by_kind(node, "identifier"),
            None,
            node.child_by_field_name("value"),
            false,
        ),
        "typed_default_parameter" => (
            find_child_by_kind(node, "identifier"),
            find_child_by_kind(node, "type"),
            node.child_by_field_name("value"),
            false,
        ),
        "list_splat_pattern" => (find_child_by_kind(node, "identifier"), None, None, true),
        "dictionary_splat_pattern" => (find_child_by_kind(node, "identifier"), None, None, true),
        _ => return None,
    };

    let name = name_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    let type_annotation = type_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let default_value = default_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

fn extract_go_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    let is_variadic = kind == "variadic_parameter_declaration";
    let node_kind = if is_variadic {
        "variadic_parameter_declaration"
    } else if kind == "parameter_declaration" {
        "parameter_declaration"
    } else {
        return None;
    };

    if node.kind() != node_kind {
        return None;
    }

    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    let type_annotation = find_child_by_kind(node, "type_identifier")
        .or_else(|| find_child_by_kind(node, "pointer_type"))
        .or_else(|| find_child_by_kind(node, "slice_type"))
        .or_else(|| find_child_by_kind(node, "array_type"))
        .or_else(|| find_child_by_kind(node, "map_type"))
        .or_else(|| find_child_by_kind(node, "channel_type"))
        .or_else(|| find_child_by_kind(node, "function_type"))
        .or_else(|| find_child_by_kind(node, "interface_type"))
        .or_else(|| find_child_by_kind(node, "struct_type"))
        .or_else(|| find_child_by_kind(node, "qualified_type"))
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
        is_variadic,
    })
}

fn extract_js_parameter(node: Node<'_>, kind: &str, source: &str) -> Option<ParameterInfo> {
    // Check if this is a rest/variadic parameter
    let is_rest = kind == "rest_pattern"
        || find_child_by_kind(node, "rest_pattern").is_some();

    let name = if kind == "rest_pattern" {
        find_child_by_kind(node, "identifier")
    } else if kind == "identifier" {
        Some(node)
    } else if kind == "assignment_pattern" {
        node.child_by_field_name("left")
    } else if kind == "required_parameter" || kind == "optional_parameter" {
        // For required_parameter containing rest_pattern: ...names
        if let Some(rest) = find_child_by_kind(node, "rest_pattern") {
            find_child_by_kind(rest, "identifier")
        } else {
            find_child_by_kind(node, "identifier")
        }
    } else {
        return None;
    };

    let name = name
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // For TypeScript, try to get type annotation
    let type_annotation = find_child_by_kind(node, "type_annotation")
        .and_then(|ta| ta.child(1)) // Skip the colon
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let default_value = if kind == "assignment_pattern" {
        node.child_by_field_name("right")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic: is_rest,
    })
}

/// Extracts return type from a function node.
fn extract_return_type(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    match language {
        ProgrammingLanguage::Rust => extract_rust_return_type(node, source),
        ProgrammingLanguage::Python => extract_python_return_type(node, source),
        ProgrammingLanguage::Go => extract_go_return_type(node, source),
        ProgrammingLanguage::TypeScript => extract_typescript_return_type(node, source),
        ProgrammingLanguage::JavaScript => None, // JavaScript has no type annotations
        _ => None,
    }
}

/// Extracts return type from Rust function_item.
///
/// Rust AST structure has the return type as a direct child after `->`:
/// ```text
/// function_item
///   parameters (...)
///   ->
///   type_identifier  <-- this is the return type
///   block { ... }
/// ```
///
/// If no explicit return type is present, returns `()` (unit type) since
/// Rust functions without a return annotation implicitly return unit.
fn extract_rust_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        // Once we find the arrow, the next type-like node is the return type
        if found_arrow {
            let kind = child.kind();
            // Skip the block - that's the function body, not the return type
            if kind == "block" {
                return None;
            }
            // These are all valid return type node kinds in Rust
            if matches!(
                kind,
                "type_identifier"
                    | "primitive_type"
                    | "reference_type"
                    | "generic_type"
                    | "scoped_type_identifier"
                    | "tuple_type"
                    | "array_type"
                    | "pointer_type"
                    | "function_type"
                    | "unit_type"
                    | "never_type"
                    | "bounded_type"
                    | "dynamic_type"
                    | "abstract_type"
            ) {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    // No explicit return type means the function returns () (unit type)
    Some("()".to_string())
}

/// Extracts return type from Python function_definition.
///
/// Python AST structure has the return type after `->`:
/// ```text
/// function_definition
///   parameters (...)
///   ->
///   type  <-- this is the return type (contains identifier like "str")
///   :
///   block: ...
/// ```
fn extract_python_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        if found_arrow {
            let kind = child.kind();
            // The colon and block come after the type
            if kind == ":" || kind == "block" {
                return None;
            }
            // The type node contains the actual return type
            if kind == "type" {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Extracts return type from Go function_declaration.
fn extract_go_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // Go return type is in result field or simple_type
    node.child_by_field_name("result")
        .or_else(|| {
            // Find the second parameter_list (return types)
            let mut count = 0;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    count += 1;
                    if count == 2 {
                        return Some(child);
                    }
                }
            }
            None
        })
        .or_else(|| {
            // Look for type identifier after parameters
            let mut found_params = false;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    found_params = true;
                    continue;
                }
                if found_params && child.kind() != "block" {
                    return Some(child);
                }
            }
            None
        })
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Extracts return type from TypeScript function_declaration.
///
/// TypeScript AST uses `type_annotation` for the return type:
/// ```text
/// function_declaration
///   formal_parameters (...)
///   type_annotation
///     :
///     predefined_type  <-- this is the return type
///   statement_block { ... }
/// ```
fn extract_typescript_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // Find type_annotation that is a direct child (not inside parameters)
    let mut cursor = node.walk();
    let mut found_params = false;

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Track when we've passed the formal_parameters
        if kind == "formal_parameters" {
            found_params = true;
            continue;
        }

        // Look for type_annotation after parameters but before body
        if found_params && kind == "type_annotation" {
            // The type_annotation contains ": type", we want just the type
            let mut ta_cursor = child.walk();
            for ta_child in child.children(&mut ta_cursor) {
                if ta_child.kind() != ":" {
                    return ta_child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                }
            }
        }

        // Stop if we've reached the function body
        if kind == "statement_block" {
            break;
        }
    }

    None
}

/// Extracts doc comment from preceding sibling nodes.
///
/// Doc comments can appear:
/// 1. As direct preceding siblings (Rust `///`, Go `//`)
/// 2. As siblings of a parent wrapper (TypeScript exported functions)
fn extract_doc_comment(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    // First try: look at direct preceding siblings
    if let Some(comments) = collect_doc_comments(node, language, source) {
        return Some(comments);
    }

    // Second try: look at parent's preceding siblings (for exported functions, etc.)
    // This handles cases like:
    //   comment  <-- doc comment is here
    //   export_statement
    //     function_declaration  <-- but we're looking from here
    if let Some(parent) = node.parent() {
        let parent_kind = parent.kind();
        // Only traverse up for known wrapper patterns
        if matches!(
            parent_kind,
            "export_statement"
                | "export_declaration"
                | "decorated_definition"  // Python decorators
                | "public_declaration"
        ) {
            return collect_doc_comments(parent, language, source);
        }
    }

    None
}

/// Collects doc comments from preceding siblings of the given node.
fn collect_doc_comments(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling();

    while let Some(sibling) = prev {
        if is_doc_comment_node(sibling.kind(), language) {
            if let Ok(text) = sibling.utf8_text(source.as_bytes()) {
                comments.push(text.to_string());
            }
            prev = sibling.prev_sibling();
        } else if sibling.kind() == "comment" || sibling.kind() == "line_comment" {
            // Only include adjacent comments
            prev = sibling.prev_sibling();
        } else {
            break;
        }
    }

    if comments.is_empty() {
        return None;
    }

    // Reverse to get comments in order
    comments.reverse();

    // Clean and join the comments
    let cleaned: Vec<String> = comments
        .iter()
        .map(|c| clean_doc_comment(c, language))
        .collect();

    Some(cleaned.join("\n"))
}

/// Checks if a node kind is a doc comment for the given language.
fn is_doc_comment_node(kind: &str, language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Rust => kind == "line_comment",
        ProgrammingLanguage::Python => kind == "expression_statement", // docstrings
        ProgrammingLanguage::Go => kind == "comment",
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            kind == "comment" || kind == "jsx_text"
        }
        ProgrammingLanguage::Java
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Scala => kind == "comment" || kind == "block_comment",
        _ => kind == "comment",
    }
}

/// Cleans doc comment prefixes based on language conventions.
fn clean_doc_comment(comment: &str, language: ProgrammingLanguage) -> String {
    let trimmed = comment.trim();

    match language {
        ProgrammingLanguage::Rust => {
            // Handle /// and //! doc comments
            trimmed
                .strip_prefix("///")
                .or_else(|| trimmed.strip_prefix("//!"))
                .map(|s| s.trim())
                .unwrap_or(trimmed)
                .to_string()
        }
        ProgrammingLanguage::Python => {
            // Handle docstrings (""" ... """ or ''' ... ''')
            let s = trimmed
                .strip_prefix("\"\"\"")
                .and_then(|s| s.strip_suffix("\"\"\""))
                .or_else(|| {
                    trimmed
                        .strip_prefix("'''")
                        .and_then(|s| s.strip_suffix("'''"))
                })
                .unwrap_or(trimmed);
            s.trim().to_string()
        }
        ProgrammingLanguage::Go => {
            // Handle // comments
            trimmed
                .strip_prefix("//")
                .map(|s| s.trim())
                .unwrap_or(trimmed)
                .to_string()
        }
        ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Scala => {
            // Handle JSDoc-style /** ... */ and // comments
            if trimmed.starts_with("/**") && trimmed.ends_with("*/") {
                clean_jsdoc_comment(trimmed)
            } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                trimmed[2..trimmed.len() - 2].trim().to_string()
            } else {
                trimmed
                    .strip_prefix("//")
                    .map(|s| s.trim())
                    .unwrap_or(trimmed)
                    .to_string()
            }
        }
        _ => trimmed.to_string(),
    }
}

/// Cleans JSDoc-style block comments.
fn clean_jsdoc_comment(comment: &str) -> String {
    let content = &comment[3..comment.len() - 2]; // Remove /** and */
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("*")
                .map(|s| s.trim())
                .unwrap_or(trimmed)
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Finds the first child node with the given kind.
fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|child| child.kind() == kind)
}

/// Finds the nth child node with the given kind (0-indexed).
fn find_nth_child_by_kind<'a>(node: Node<'a>, kind: &str, n: usize) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .nth(n)
}
