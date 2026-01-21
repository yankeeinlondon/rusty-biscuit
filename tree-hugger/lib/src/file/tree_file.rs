use std::path::{Path, PathBuf};

use biscuit_hash::xx_hash;
use tree_sitter::{Node, Parser, QueryCursor, StreamingIterator};

use crate::error::TreeHuggerError;
use crate::queries::{QueryKind, query_for};
use crate::shared::{
    CodeBlock, CodeRange, DiagnosticSeverity, FieldInfo, FunctionSignature, ImportSymbol,
    LintDiagnostic, ParameterInfo, ProgrammingLanguage, SymbolInfo, SymbolKind, SyntaxDiagnostic,
    TypeMetadata, VariantInfo, Visibility,
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

                // Extract metadata based on symbol kind
                let (signature, type_metadata, doc_comment) = if let Some(ctx) = context_node {
                    let sig = if kind.is_function() {
                        extract_signature(ctx, self.language, &self.source)
                    } else {
                        None
                    };
                    let type_meta = if kind.is_type() {
                        extract_type_metadata(ctx, self.language, &self.source)
                    } else {
                        None
                    };
                    let doc = extract_doc_comment(ctx, self.language, &self.source);
                    (sig, type_meta, doc)
                } else {
                    (None, None, None)
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
                        type_metadata,
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
    let visibility = extract_visibility(node, language, source);

    if parameters.is_empty() && return_type.is_none() && visibility.is_none() {
        return None;
    }

    Some(FunctionSignature {
        parameters,
        return_type,
        visibility,
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
        ProgrammingLanguage::Python | ProgrammingLanguage::Scala => "parameters",
        ProgrammingLanguage::Go | ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            "parameter_list"
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => "formal_parameters",
        ProgrammingLanguage::Php => "formal_parameters",
        ProgrammingLanguage::Java => "formal_parameters",
        ProgrammingLanguage::CSharp => "parameter_list",
        ProgrammingLanguage::Swift => return extract_swift_parameters(node, source),
        _ => return Vec::new(),
    };

    // For Go, methods have TWO parameter_list nodes:
    // 1. receiver (g *Greeter)
    // 2. actual parameters (name string)
    // We need to find the SECOND parameter_list for methods.
    //
    // For C/C++, parameters are inside function_declarator. The context node
    // may be either function_definition or function_declarator:
    // - If function_definition: look in function_declarator child
    // - If function_declarator: look directly for parameter_list
    let params_node = if language == ProgrammingLanguage::Go && node.kind() == "method_declaration" {
        find_nth_child_by_kind(node, params_node_kind, 1) // 0-indexed, so 1 = second
    } else if matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp) {
        // C/C++: Context may be function_declarator or function_definition
        if node.kind() == "function_declarator" {
            // Already at function_declarator, look for parameter_list directly
            find_child_by_kind(node, params_node_kind)
        } else {
            // At function_definition, look inside function_declarator
            find_child_by_kind(node, "function_declarator")
                .and_then(|fd| find_child_by_kind(fd, params_node_kind))
        }
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
        // Go needs special handling: a single parameter_declaration can define
        // multiple parameters (e.g., `a, b int` defines both `a` and `b`)
        if language == ProgrammingLanguage::Go {
            parameters.extend(extract_go_parameters(child, source));
        } else if let Some(param) = extract_single_parameter(child, language, source) {
            parameters.push(param);
        }
    }

    parameters
}

/// Extracts a single parameter from a parameter node.
///
/// Note: Go is handled separately in `extract_parameters` using `extract_go_parameters`
/// because Go allows multiple identifiers per declaration (e.g., `a, b int`).
/// Swift is handled separately using `extract_swift_parameters`.
fn extract_single_parameter(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ParameterInfo> {
    let kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => extract_rust_parameter(node, source),
        ProgrammingLanguage::Python => extract_python_parameter(node, source),
        ProgrammingLanguage::Scala => extract_scala_parameter(node, source),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            extract_js_parameter(node, kind, source)
        }
        ProgrammingLanguage::Php => extract_php_parameter(node, source),
        ProgrammingLanguage::Java => extract_java_parameter(node, source),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => extract_c_parameter(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_parameter(node, source),
        // Go is handled specially in extract_parameters
        // Swift is handled specially via extract_swift_parameters
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

    // Find the type annotation - could be various type node kinds
    let type_annotation = find_rust_type_node(node)
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

/// Extracts Go parameters from a parameter_declaration node.
///
/// Go allows multiple identifiers to share a type: `a, b int` creates two parameters.
/// This function returns all parameters from a single declaration.
fn extract_go_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let kind = node.kind();

    let is_variadic = kind == "variadic_parameter_declaration";
    if kind != "parameter_declaration" && kind != "variadic_parameter_declaration" {
        return Vec::new();
    }

    // Find the type annotation (shared by all identifiers in this declaration)
    let type_annotation = find_go_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Collect all identifiers in this declaration
    let mut cursor = node.walk();
    let mut params = Vec::new();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier"
            && let Ok(name) = child.utf8_text(source.as_bytes())
        {
            params.push(ParameterInfo {
                name: name.to_string(),
                type_annotation: type_annotation.clone(),
                default_value: None,
                is_variadic,
            });
        }
    }

    params
}

/// Finds a Go type node among the children.
fn find_go_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const GO_TYPE_KINDS: &[&str] = &[
        "type_identifier",
        "pointer_type",
        "slice_type",
        "array_type",
        "map_type",
        "channel_type",
        "function_type",
        "interface_type",
        "struct_type",
        "qualified_type",
        "generic_type",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| GO_TYPE_KINDS.contains(&child.kind()))
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
        ProgrammingLanguage::Php => extract_php_return_type(node, source),
        ProgrammingLanguage::Java => extract_java_return_type(node, source),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => extract_c_return_type(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_return_type(node, source),
        ProgrammingLanguage::Swift => extract_swift_return_type(node, source),
        ProgrammingLanguage::Scala => extract_scala_return_type(node, source),
        _ => None,
    }
}

/// Extracts visibility modifier from a function/method node.
fn extract_visibility(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<Visibility> {
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            extract_ts_visibility(node, source)
        }
        ProgrammingLanguage::Java => extract_java_visibility(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_visibility(node, source),
        ProgrammingLanguage::Php => extract_php_visibility(node, source),
        ProgrammingLanguage::Rust => extract_rust_visibility(node, source),
        ProgrammingLanguage::Cpp => extract_cpp_visibility(node, source),
        ProgrammingLanguage::Swift => extract_swift_visibility(node, source),
        // Go, Python, Scala, C don't have visibility keywords (use naming conventions instead)
        _ => None,
    }
}

/// Extracts visibility from TypeScript/JavaScript method_definition.
///
/// TypeScript AST structure has accessibility_modifier as a child:
/// ```text
/// method_definition
///   accessibility_modifier (public/protected/private)
///   property_identifier
///   formal_parameters
///   ...
/// ```
fn extract_ts_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "accessibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            return match text {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
    }
    None
}

/// Extracts visibility from Java method_declaration.
///
/// Java AST structure has modifiers containing visibility:
/// ```text
/// method_declaration
///   modifiers
///     public/protected/private
///   type_identifier
///   identifier
///   ...
/// ```
fn extract_java_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let modifiers = find_child_by_kind(node, "modifiers")?;
    let mut cursor = modifiers.walk();
    for child in modifiers.children(&mut cursor) {
        let text = child.utf8_text(source.as_bytes()).ok()?;
        match text {
            "public" => return Some(Visibility::Public),
            "protected" => return Some(Visibility::Protected),
            "private" => return Some(Visibility::Private),
            _ => continue,
        }
    }
    None
}

/// Extracts visibility from C# method_declaration.
fn extract_csharp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            match text {
                "public" => return Some(Visibility::Public),
                "protected" => return Some(Visibility::Protected),
                "private" => return Some(Visibility::Private),
                "internal" => return Some(Visibility::Internal),
                _ => continue,
            }
        }
    }
    None
}

/// Extracts visibility from PHP method_declaration.
fn extract_php_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            return match text {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
    }
    None
}

/// Extracts visibility from Rust function_item (pub keyword).
fn extract_rust_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            if text.starts_with("pub") {
                return Some(Visibility::Public);
            }
        }
    }
    None
}

/// Extracts visibility from C++ method declarations.
///
/// C++ visibility is handled via access specifiers in the class (public:, private:, etc.)
/// not as part of the method itself. For now, we check for inline access specifiers.
fn extract_cpp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    // C++ uses access specifiers at the section level, not per-method
    // Check if there's an access_specifier sibling before this node
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if sibling.kind() == "access_specifier" {
            let text = sibling.utf8_text(source.as_bytes()).ok()?;
            return match text.trim_end_matches(':') {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
        // Stop if we hit another method or declaration
        if sibling.kind() == "function_definition"
            || sibling.kind() == "declaration"
            || sibling.kind() == "field_declaration"
        {
            break;
        }
        prev = sibling.prev_sibling();
    }
    None
}

/// Extracts visibility from Swift function declarations.
fn extract_swift_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                let text = modifier.utf8_text(source.as_bytes()).ok()?;
                match text {
                    "public" => return Some(Visibility::Public),
                    "internal" => return Some(Visibility::Internal),
                    "private" => return Some(Visibility::Private),
                    "fileprivate" => return Some(Visibility::Private),
                    _ => continue,
                }
            }
        }
    }
    None
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

// =============================================================================
// PHP extraction functions
// =============================================================================

/// Extracts a PHP parameter from a simple_parameter or property_promotion_parameter node.
///
/// PHP AST structure:
/// ```text
/// simple_parameter
///   primitive_type (string/int/etc.) or type_identifier (ClassName)
///   variable_name
///     $
///     name
///   = (optional, for defaults)
///   value (optional)
/// ```
fn extract_php_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();
    if kind != "simple_parameter" && kind != "property_promotion_parameter" && kind != "variadic_parameter" {
        return None;
    }

    let is_variadic = kind == "variadic_parameter";

    // Find the variable name
    let var_name = find_child_by_kind(node, "variable_name")?;
    let name_node = find_child_by_kind(var_name, "name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

    // Find the type annotation (primitive_type, type_identifier, union_type, etc.)
    let type_annotation = find_php_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a PHP type node among the children.
fn find_php_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const PHP_TYPE_KINDS: &[&str] = &[
        "primitive_type",
        "named_type",
        "optional_type",
        "union_type",
        "intersection_type",
        "type_list",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| PHP_TYPE_KINDS.contains(&child.kind()))
}

/// Extracts return type from PHP function_definition or method_declaration.
///
/// PHP AST structure has return type after `:`:
/// ```text
/// function_definition
///   formal_parameters (...)
///   :
///   primitive_type  <-- return type
///   compound_statement { ... }
/// ```
fn extract_php_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_colon = false;

    for child in node.children(&mut cursor) {
        if child.kind() == ":" {
            found_colon = true;
            continue;
        }

        if found_colon {
            let kind = child.kind();
            // Stop at the function body
            if kind == "compound_statement" {
                return None;
            }
            // These are valid return type node kinds in PHP
            if matches!(
                kind,
                "primitive_type"
                    | "named_type"
                    | "optional_type"
                    | "union_type"
                    | "intersection_type"
            ) {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

// =============================================================================
// Java extraction functions
// =============================================================================

/// Extracts a Java parameter from a formal_parameter node.
///
/// Java AST structure:
/// ```text
/// formal_parameter
///   type_identifier (String) or integral_type (int)
///   identifier (name)
/// ```
fn extract_java_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();
    if kind != "formal_parameter" && kind != "spread_parameter" {
        return None;
    }

    let is_variadic = kind == "spread_parameter";

    // Find the identifier (parameter name)
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation
    let type_annotation = find_java_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None, // Java doesn't support default parameter values
        is_variadic,
    })
}

/// Extracts return type from Java method_declaration.
///
/// Java AST structure has return type before method name:
/// ```text
/// method_declaration
///   modifiers (public)
///   type_identifier  <-- return type
///   identifier (method name)
///   formal_parameters (...)
///   block { ... }
/// ```
fn extract_java_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // In Java, the return type comes before the method name
    // We need to find the type node that appears before the identifier
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Skip modifiers
        if kind == "modifiers" || kind == "marker_annotation" || kind == "annotation" {
            continue;
        }

        // If we hit the identifier, we've gone too far
        if kind == "identifier" {
            return None;
        }

        // These are valid return type node kinds in Java
        if matches!(
            kind,
            "type_identifier"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type"
                | "void_type"
                | "generic_type"
                | "array_type"
                | "scoped_type_identifier"
        ) {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }
    }

    None
}

// =============================================================================
// C/C++ extraction functions
// =============================================================================

/// Extracts a C/C++ parameter from a parameter_declaration node.
///
/// C/C++ AST structure:
/// ```text
/// parameter_declaration
///   primitive_type (int/char/etc.)
///   identifier or pointer_declarator
/// ```
fn extract_c_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter_declaration" && node.kind() != "variadic_parameter" {
        return None;
    }

    let is_variadic = node.kind() == "variadic_parameter";
    if is_variadic {
        return Some(ParameterInfo {
            name: "...".to_string(),
            type_annotation: None,
            default_value: None,
            is_variadic: true,
        });
    }

    // Find the identifier - could be direct child or inside pointer_declarator/reference_declarator
    let name = find_c_param_name(node, source)?;

    // Find the type annotation
    let type_annotation = find_c_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
        is_variadic: false,
    })
}

/// Finds the parameter name in a C/C++ parameter_declaration.
fn find_c_param_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        if kind == "identifier" {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }

        // Handle pointer declarator: *name
        if kind == "pointer_declarator" {
            if let Some(id) = find_child_by_kind(child, "identifier") {
                return id.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }

        // Handle reference declarator: &name
        if kind == "reference_declarator" {
            if let Some(id) = find_child_by_kind(child, "identifier") {
                return id.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Extracts return type from C/C++ function_definition.
///
/// C/C++ AST structure has return type as first child of function_definition:
/// ```text
/// function_definition
///   primitive_type  <-- return type (or qualified_identifier for std::string)
///   function_declarator  <-- This may be the context node
///     identifier
///     parameter_list (...)
///   compound_statement { ... }
/// ```
///
/// Note: The context node may be `function_declarator`, so we need to look
/// at the parent `function_definition` to find the return type.
fn extract_c_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // If the node is function_declarator, look at the parent
    let function_node = if node.kind() == "function_declarator" {
        node.parent()?
    } else {
        node
    };

    let mut cursor = function_node.walk();

    for child in function_node.children(&mut cursor) {
        let kind = child.kind();

        // Skip the function declarator and body
        if kind == "function_declarator" || kind == "compound_statement" {
            break;
        }

        // These are valid return type node kinds in C/C++
        if matches!(
            kind,
            "primitive_type"
                | "type_identifier"
                | "sized_type_specifier"
                | "qualified_identifier"
                | "template_type"
        ) {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }
    }

    None
}

// =============================================================================
// C# extraction functions
// =============================================================================

/// Extracts a C# parameter from a parameter node.
///
/// C# AST structure:
/// ```text
/// parameter
///   predefined_type (string/int/etc.) or type_identifier
///   identifier
/// ```
fn extract_csharp_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter" {
        return None;
    }

    // Check for params modifier (variadic)
    let is_variadic = find_child_by_kind(node, "parameter_modifier")
        .map(|m| m.utf8_text(source.as_bytes()).ok() == Some("params"))
        .unwrap_or(false);

    // Find the identifier (parameter name)
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation
    let type_annotation = find_csharp_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find default value
    let default_value = find_child_by_kind(node, "equals_value_clause")
        .and_then(|eq| eq.child(1))
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Extracts return type from C# method_declaration.
///
/// C# AST structure has return type before method name:
/// ```text
/// method_declaration
///   modifier (public)
///   predefined_type  <-- return type
///   identifier (method name)
///   parameter_list (...)
///   block { ... }
/// ```
fn extract_csharp_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Skip modifiers
        if kind == "modifier" {
            continue;
        }

        // If we hit the identifier, we've gone too far
        if kind == "identifier" {
            return None;
        }

        // These are valid return type node kinds in C#
        if matches!(
            kind,
            "predefined_type"
                | "generic_name"
                | "array_type"
                | "nullable_type"
                | "qualified_name"
        ) {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }

        // Handle identifier as type (e.g., custom class names)
        // Note: we need to check this after checking it's not the method name
    }

    None
}

// =============================================================================
// Swift extraction functions
// =============================================================================

/// Extracts Swift parameters from a function_declaration.
///
/// Swift has a unique AST structure where parameters are direct children
/// of function_declaration rather than in a separate parameters node:
/// ```text
/// function_declaration
///   func
///   simple_identifier
///   (
///   parameter
///     simple_identifier
///     :
///     user_type
///   ,
///   parameter
///     ...
///   )
///   ->
///   user_type
///   function_body { ... }
/// ```
fn extract_swift_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let mut parameters = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter" {
            if let Some(param) = extract_swift_single_parameter(child, source) {
                parameters.push(param);
            }
        }
    }

    parameters
}

/// Extracts a single Swift parameter.
fn extract_swift_single_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    // Swift parameters can have external and internal names
    // We want the internal name (second identifier) or the only identifier
    let identifiers: Vec<_> = {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|c| c.kind() == "simple_identifier")
            .collect()
    };

    // Use the last identifier as the parameter name (internal name)
    let name = identifiers
        .last()
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation (user_type, array_type, etc.)
    let type_annotation = find_swift_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Check for variadic (...)
    let is_variadic = {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|c| c.kind() == "...")
    };

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a Swift type node among the children.
fn find_swift_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const SWIFT_TYPE_KINDS: &[&str] = &[
        "user_type",
        "array_type",
        "dictionary_type",
        "optional_type",
        "tuple_type",
        "function_type",
        "metatype",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| SWIFT_TYPE_KINDS.contains(&child.kind()))
}

/// Extracts return type from Swift function_declaration.
///
/// Swift AST structure has return type after `->`:
/// ```text
/// function_declaration
///   ...parameters...
///   ->
///   user_type  <-- return type
///   function_body { ... }
/// ```
fn extract_swift_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        if found_arrow {
            let kind = child.kind();

            // Stop at the function body
            if kind == "function_body" {
                return None;
            }

            // These are valid return type node kinds in Swift
            if matches!(
                kind,
                "user_type"
                    | "array_type"
                    | "dictionary_type"
                    | "optional_type"
                    | "tuple_type"
                    | "function_type"
                    | "metatype"
            ) {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

// =============================================================================
// Scala extraction functions
// =============================================================================

/// Extracts return type from Scala function_definition.
///
/// Scala AST structure has return type after `:`:
/// ```text
/// function_definition
///   def
///   identifier
///   parameters (...)
///   :
///   type_identifier  <-- return type
///   =
///   block { ... }
/// ```
fn extract_scala_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_params = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "parameters" {
            found_params = true;
            continue;
        }

        if found_params && child.kind() == ":" {
            continue;
        }

        if found_params {
            let kind = child.kind();

            // Stop at equals or block
            if kind == "=" || kind == "block" {
                return None;
            }

            // These are valid return type node kinds in Scala
            if matches!(
                kind,
                "type_identifier"
                    | "generic_type"
                    | "tuple_type"
                    | "function_type"
                    | "compound_type"
            ) {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Extracts a Scala parameter from a parameter node.
///
/// Scala AST structure:
/// ```text
/// parameter
///   identifier (name)
///   :
///   type_identifier (type)
///   = (optional)
///   value (optional, for defaults)
/// ```
fn extract_scala_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter" {
        return None;
    }

    // Find the identifier (parameter name) - first identifier child
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation (type_identifier, generic_type, etc.)
    let type_annotation = find_scala_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Check for variadic (*) - Scala uses `name: Type*` for varargs
    let is_variadic = {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|c| c.kind() == "repeated_parameter_type")
    };

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a Scala type node among the children.
fn find_scala_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const SCALA_TYPE_KINDS: &[&str] = &[
        "type_identifier",
        "generic_type",
        "tuple_type",
        "function_type",
        "compound_type",
        "infix_type",
        "repeated_parameter_type",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| SCALA_TYPE_KINDS.contains(&child.kind()))
}

// =============================================================================
// Helper functions
// =============================================================================

/// Finds a default value after an `=` sign in a parameter.
fn find_default_value_after_equals(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_equals = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "=" {
            found_equals = true;
            continue;
        }

        if found_equals {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
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

/// Finds a Rust type node among the children of a parameter or other node.
///
/// Rust has many type node kinds: primitive_type, type_identifier, reference_type,
/// generic_type, tuple_type, array_type, function_type, etc.
fn find_rust_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const RUST_TYPE_KINDS: &[&str] = &[
        "primitive_type",
        "type_identifier",
        "reference_type",
        "generic_type",
        "scoped_type_identifier",
        "tuple_type",
        "array_type",
        "slice_type",
        "pointer_type",
        "function_type",
        "unit_type",
        "never_type",
        "bounded_type",
        "dynamic_type",
        "abstract_type",
        "macro_invocation", // For macro-generated types
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| RUST_TYPE_KINDS.contains(&child.kind()))
}

// ============================================================================
// Type metadata extraction
// ============================================================================

/// Extracts metadata from a type definition node (struct, enum, interface, etc.).
fn extract_type_metadata(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<TypeMetadata> {
    let node_kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => extract_rust_type_metadata(node, node_kind, source),
        ProgrammingLanguage::TypeScript => extract_typescript_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Go => extract_go_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Python => extract_python_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Java => extract_java_type_metadata(node, node_kind, source),
        ProgrammingLanguage::C => extract_c_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Cpp => extract_cpp_type_metadata(node, node_kind, source),
        ProgrammingLanguage::CSharp => extract_csharp_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Swift => extract_swift_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Scala => extract_scala_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Php => extract_php_type_metadata(node, node_kind, source),
        _ => None,
    }
}

/// Extracts type metadata from Rust struct_item or enum_item nodes.
fn extract_rust_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    // Try by field name first (the proper tree-sitter way)
    if let Some(type_params) = node.child_by_field_name("type_parameters") {
        metadata.type_parameters = extract_rust_type_parameters(type_params, source);
    } else if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        // Fallback to kind-based search
        metadata.type_parameters = extract_rust_type_parameters(type_params, source);
    }

    match node_kind {
        "struct_item" => {
            // Check for field_declaration_list (normal struct) or ordered_field_declaration_list (tuple struct)
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_rust_struct_fields(field_list, source);
            } else if let Some(tuple_fields) = find_child_by_kind(node, "ordered_field_declaration_list")
            {
                // Tuple struct: struct Point(i32, i32)
                metadata.fields = extract_rust_tuple_struct_fields(tuple_fields, source);
            }
        }
        "enum_item" => {
            if let Some(variant_list) = find_child_by_kind(node, "enum_variant_list") {
                metadata.variants = extract_rust_enum_variants(variant_list, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Rust type_parameters node.
fn extract_rust_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" | "lifetime" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
            // type_parameter is the wrapper node for a single generic type param
            "type_parameter" => {
                // The name field contains the type identifier
                if let Some(name) = child.child_by_field_name("name") {
                    if let Ok(text) = name.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                } else if let Some(ident) = find_child_by_kind(child, "type_identifier") {
                    if let Ok(text) = ident.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                }
            }
            "lifetime_parameter" => {
                if let Some(lifetime) = child.child_by_field_name("lifetime")
                    .or_else(|| find_child_by_kind(child, "lifetime"))
                {
                    if let Ok(text) = lifetime.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                }
            }
            "constrained_type_parameter" | "optional_type_parameter" => {
                // Get the type identifier from the constrained parameter
                if let Some(ident) = find_child_by_kind(child, "type_identifier") {
                    if let Ok(text) = ident.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    params
}

/// Extracts fields from a Rust field_declaration_list.
fn extract_rust_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        let name = find_child_by_kind(child, "field_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_rust_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let doc_comment = extract_doc_comment(child, ProgrammingLanguage::Rust, source);

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment,
            });
        }
    }

    fields
}

/// Extracts fields from a Rust tuple struct (ordered_field_declaration_list).
fn extract_rust_tuple_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();
    let mut index = 0;

    for child in node.children(&mut cursor) {
        // Look for type nodes directly as children
        if RUST_TYPE_KINDS.contains(&child.kind()) {
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());

            fields.push(FieldInfo {
                name: index.to_string(),
                type_annotation,
                doc_comment: None,
            });
            index += 1;
        }
    }

    fields
}

/// List of Rust type node kinds (used for tuple struct field extraction).
const RUST_TYPE_KINDS: &[&str] = &[
    "primitive_type",
    "type_identifier",
    "reference_type",
    "generic_type",
    "scoped_type_identifier",
    "tuple_type",
    "array_type",
    "slice_type",
    "pointer_type",
    "function_type",
    "unit_type",
    "never_type",
    "bounded_type",
    "dynamic_type",
    "abstract_type",
    "macro_invocation",
];

/// Extracts variants from a Rust enum_variant_list.
fn extract_rust_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_variant" {
            continue;
        }

        let name = find_child_by_kind(child, "identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let doc_comment = extract_doc_comment(child, ProgrammingLanguage::Rust, source);

        if let Some(name) = name {
            let mut variant = VariantInfo::unit(&name);
            variant.doc_comment = doc_comment;

            // Check for tuple variant: Variant(Type1, Type2)
            if let Some(tuple_fields) = find_child_by_kind(child, "ordered_field_declaration_list") {
                variant.tuple_fields = extract_rust_variant_tuple_fields(tuple_fields, source);
            }

            // Check for struct variant: Variant { field: Type }
            if let Some(field_list) = find_child_by_kind(child, "field_declaration_list") {
                variant.struct_fields = extract_rust_struct_fields(field_list, source);
            }

            variants.push(variant);
        }
    }

    variants
}

/// Extracts tuple field types from an enum variant.
fn extract_rust_variant_tuple_fields(node: Node<'_>, source: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if RUST_TYPE_KINDS.contains(&child.kind()) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                fields.push(text.to_string());
            }
        }
    }

    fields
}

/// Extracts type metadata from TypeScript interface or type_alias_declaration.
fn extract_typescript_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_typescript_type_parameters(type_params, source);
    }

    match node_kind {
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "interface_body")
                .or_else(|| find_child_by_kind(node, "object_type"))
            {
                metadata.fields = extract_typescript_interface_fields(body, source);
            }
        }
        "type_alias_declaration" => {
            // Type aliases can be object types or other types
            if let Some(object_type) = find_child_by_kind(node, "object_type") {
                metadata.fields = extract_typescript_interface_fields(object_type, source);
            }
        }
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "class_body") {
                metadata.fields = extract_typescript_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_typescript_enum_variants(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from TypeScript type_parameters node.
fn extract_typescript_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter" {
            if let Some(name) = find_child_by_kind(child, "type_identifier") {
                if let Ok(text) = name.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
        }
    }

    params
}

/// Extracts fields from TypeScript interface_body or object_type.
fn extract_typescript_interface_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "property_signature" && kind != "public_field_definition" {
            continue;
        }

        let name = find_child_by_kind(child, "property_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_child_by_kind(child, "type_annotation")
            .and_then(|ta| ta.child(1)) // Skip the colon
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment: None,
            });
        }
    }

    fields
}

/// Extracts fields from TypeScript class_body.
fn extract_typescript_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "public_field_definition" && kind != "property_definition" {
            continue;
        }

        let name = find_child_by_kind(child, "property_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_child_by_kind(child, "type_annotation")
            .and_then(|ta| ta.child(1))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment: None,
            });
        }
    }

    fields
}

/// Extracts variants from TypeScript enum_body.
fn extract_typescript_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_assignment" && child.kind() != "property_identifier" {
            continue;
        }

        let name = if child.kind() == "enum_assignment" {
            find_child_by_kind(child, "property_identifier")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string())
        } else {
            child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string())
        };

        if let Some(name) = name {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}

/// Extracts type metadata from Go type_spec nodes.
fn extract_go_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    if node_kind != "type_spec" && node_kind != "type_declaration" {
        return None;
    }

    // For type_declaration, find the type_spec inside
    let type_spec = if node_kind == "type_declaration" {
        find_child_by_kind(node, "type_spec")?
    } else {
        node
    };

    let mut metadata = TypeMetadata::new();

    // Extract type parameters if present
    if let Some(type_params) = find_child_by_kind(type_spec, "type_parameter_list") {
        metadata.type_parameters = extract_go_type_parameters(type_params, source);
    }

    // Check for struct type
    if let Some(struct_type) = find_child_by_kind(type_spec, "struct_type") {
        if let Some(field_list) = find_child_by_kind(struct_type, "field_declaration_list") {
            metadata.fields = extract_go_struct_fields(field_list, source);
        }
    }

    // Check for interface type
    if let Some(interface_type) = find_child_by_kind(type_spec, "interface_type") {
        metadata.fields = extract_go_interface_methods(interface_type, source);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts type parameters from Go type_parameter_list.
fn extract_go_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter_declaration" {
            // Get all identifiers in this declaration
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "identifier" {
                    if let Ok(text) = inner.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                }
            }
        }
    }

    params
}

/// Extracts fields from Go struct field_declaration_list.
fn extract_go_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Go allows multiple identifiers per field declaration: `a, b int`
        let type_annotation = find_go_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier" {
                if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation: type_annotation.clone(),
                        doc_comment: None,
                    });
                }
            }
        }
    }

    fields
}

/// Extracts method signatures from Go interface_type.
fn extract_go_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "method_spec" {
            let name = find_child_by_kind(child, "field_identifier")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            // Get the full method signature as the "type"
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());

            if let Some(name) = name {
                fields.push(FieldInfo {
                    name,
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts type metadata from Python class_definition.
fn extract_python_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    if node_kind != "class_definition" {
        return None;
    }

    let mut metadata = TypeMetadata::new();

    // Check for class body
    if let Some(body) = find_child_by_kind(node, "block") {
        metadata.fields = extract_python_class_fields(body, source);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from Python class body.
fn extract_python_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Annotated assignment: name: str = "value"
            "expression_statement" => {
                if let Some(assignment) = find_child_by_kind(child, "assignment") {
                    // Check for type annotation
                    if let Some(type_node) = find_child_by_kind(assignment, "type") {
                        let name = assignment
                            .child(0)
                            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                            .map(|s| s.to_string());

                        let type_annotation = type_node
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());

                        if let Some(name) = name {
                            fields.push(FieldInfo {
                                name,
                                type_annotation,
                                doc_comment: None,
                            });
                        }
                    }
                }
            }
            // Typed assignment without value: name: str
            "typed_assignment_statement" => {
                let name = find_child_by_kind(child, "identifier")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                let type_annotation = find_child_by_kind(child, "type")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                if let Some(name) = name {
                    fields.push(FieldInfo {
                        name,
                        type_annotation,
                        doc_comment: None,
                    });
                }
            }
            _ => {}
        }
    }

    fields
}

// ============================================================================
// Java type metadata extraction
// ============================================================================

/// Extracts type metadata from Java class, enum, record, or interface declarations.
fn extract_java_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_java_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "class_body") {
                metadata.fields = extract_java_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_java_enum_variants(body, source);
            }
        }
        "record_declaration" => {
            // Record components are in formal_parameters
            if let Some(params) = find_child_by_kind(node, "formal_parameters") {
                metadata.fields = extract_java_record_components(params, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "interface_body") {
                metadata.fields = extract_java_interface_methods(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Java type_parameters node.
fn extract_java_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter" {
            if let Some(ident) = find_child_by_kind(child, "type_identifier") {
                if let Ok(text) = ident.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
        }
    }

    params
}

/// Extracts fields from Java class_body.
fn extract_java_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type
        let type_annotation = find_java_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get all variable declarators (handles `int x, y;`)
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "variable_declarator" {
                if let Some(name_node) = find_child_by_kind(inner, "identifier") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        fields.push(FieldInfo {
                            name: name.to_string(),
                            type_annotation: type_annotation.clone(),
                            doc_comment: None,
                        });
                    }
                }
            }
        }
    }

    fields
}

/// Finds a type node in a Java declaration.
fn find_java_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| JAVA_TYPE_KINDS.contains(&child.kind()))
}

/// List of Java type node kinds.
const JAVA_TYPE_KINDS: &[&str] = &[
    "integral_type",
    "floating_point_type",
    "boolean_type",
    "void_type",
    "type_identifier",
    "scoped_type_identifier",
    "generic_type",
    "array_type",
];

/// Extracts variants from Java enum_body.
fn extract_java_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_constant" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                variants.push(VariantInfo::unit(name));
            }
        }
    }

    variants
}

/// Extracts record components from Java formal_parameters.
fn extract_java_record_components(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "formal_parameter" {
            continue;
        }

        let type_annotation = find_java_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts method signatures from Java interface_body.
fn extract_java_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                // Get the full method signature as the "type"
                let type_annotation = child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

// ============================================================================
// C type metadata extraction
// ============================================================================

/// Extracts type metadata from C struct or enum declarations.
fn extract_c_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "struct_specifier" => {
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_c_struct_fields(field_list, source);
            }
        }
        "enum_specifier" => {
            if let Some(enumerator_list) = find_child_by_kind(node, "enumerator_list") {
                metadata.variants = extract_c_enum_variants(enumerator_list, source);
            }
        }
        "type_definition" => {
            // For typedef struct { ... } Name; we look for struct_specifier inside
            if let Some(struct_spec) = find_child_by_kind(node, "struct_specifier") {
                if let Some(field_list) = find_child_by_kind(struct_spec, "field_declaration_list") {
                    metadata.fields = extract_c_struct_fields(field_list, source);
                }
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from C field_declaration_list.
fn extract_c_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type (first type-like child)
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get field identifiers
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier" {
                if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation: type_annotation.clone(),
                        doc_comment: None,
                    });
                }
            }
        }
    }

    fields
}

/// Finds a type node in a C declaration.
fn find_c_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| C_TYPE_KINDS.contains(&child.kind()))
}

/// List of C type node kinds.
const C_TYPE_KINDS: &[&str] = &[
    "primitive_type",
    "type_identifier",
    "sized_type_specifier",
    "struct_specifier",
    "enum_specifier",
    "union_specifier",
    // C++ types
    "qualified_identifier",
    "template_type",
];

/// Extracts variants from C enumerator_list.
fn extract_c_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enumerator" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                variants.push(VariantInfo::unit(name));
            }
        }
    }

    variants
}

// ============================================================================
// C++ type metadata extraction
// ============================================================================

/// Extracts type metadata from C++ class, struct, or enum declarations.
fn extract_cpp_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "class_specifier" | "struct_specifier" => {
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_cpp_class_fields(field_list, source);
            }
        }
        "enum_specifier" => {
            if let Some(enumerator_list) = find_child_by_kind(node, "enumerator_list") {
                metadata.variants = extract_c_enum_variants(enumerator_list, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from C++ field_declaration_list.
fn extract_cpp_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Skip method declarations (they have function_declarator)
        if find_child_by_kind(child, "function_declarator").is_some() {
            continue;
        }

        // Get the type
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get field identifiers
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier" {
                if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation: type_annotation.clone(),
                        doc_comment: None,
                    });
                }
            }
        }
    }

    fields
}

// ============================================================================
// C# type metadata extraction
// ============================================================================

/// Extracts type metadata from C# class, struct, enum, interface, or record declarations.
fn extract_csharp_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameter_list") {
        metadata.type_parameters = extract_csharp_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" | "struct_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_csharp_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_member_declaration_list") {
                metadata.variants = extract_csharp_enum_variants(body, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_csharp_interface_methods(body, source);
            }
        }
        "record_declaration" => {
            // Record parameters are in parameter_list
            if let Some(params) = find_child_by_kind(node, "parameter_list") {
                metadata.fields = extract_csharp_record_parameters(params, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from C# type_parameter_list.
fn extract_csharp_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter" {
            if let Some(ident) = find_child_by_kind(child, "identifier") {
                if let Ok(text) = ident.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
        }
    }

    params
}

/// Extracts fields from C# declaration_list (class/struct body).
fn extract_csharp_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type from variable_declaration
        let type_annotation = find_child_by_kind(child, "variable_declaration")
            .and_then(|vd| find_csharp_type_node(vd))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get variable declarators
        if let Some(var_decl) = find_child_by_kind(child, "variable_declaration") {
            let mut inner_cursor = var_decl.walk();
            for inner in var_decl.children(&mut inner_cursor) {
                if inner.kind() == "variable_declarator" {
                    if let Some(ident) = find_child_by_kind(inner, "identifier") {
                        if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                            fields.push(FieldInfo {
                                name: name.to_string(),
                                type_annotation: type_annotation.clone(),
                                doc_comment: None,
                            });
                        }
                    }
                }
            }
        }
    }

    fields
}

/// Finds a type node in a C# variable_declaration.
fn find_csharp_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| CSHARP_TYPE_KINDS.contains(&child.kind()))
}

/// List of C# type node kinds.
const CSHARP_TYPE_KINDS: &[&str] = &[
    "predefined_type",
    "identifier",
    "qualified_name",
    "generic_name",
    "array_type",
    "nullable_type",
    "tuple_type",
];

/// Extracts variants from C# enum_member_declaration_list.
fn extract_csharp_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_member_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                variants.push(VariantInfo::unit(name));
            }
        }
    }

    variants
}

/// Extracts method signatures from C# interface declaration_list.
fn extract_csharp_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                // Get the full method signature as the "type"
                let type_annotation = child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts parameters from C# record parameter_list.
fn extract_csharp_record_parameters(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }

        let type_annotation = find_csharp_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name_node) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

// ============================================================================
// Swift type metadata extraction
// ============================================================================

/// Extracts type metadata from Swift class, struct, enum, or protocol declarations.
fn extract_swift_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_swift_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" => {
            // Swift uses class_declaration for struct, class, and enum
            // Check for class_body to determine actual type
            if let Some(body) = find_child_by_kind(node, "class_body") {
                // Check if this is an enum by looking for enum_entry nodes
                let mut cursor = body.walk();
                let has_enum_entries = body
                    .children(&mut cursor)
                    .any(|child| child.kind() == "enum_entry");

                if has_enum_entries {
                    metadata.variants = extract_swift_enum_cases(body, source);
                } else {
                    metadata.fields = extract_swift_class_fields(body, source);
                }
            }
        }
        "protocol_declaration" => {
            if let Some(body) = find_child_by_kind(node, "protocol_body") {
                metadata.fields = extract_swift_protocol_methods(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Swift type_parameters.
fn extract_swift_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter" {
            if let Some(ident) = find_child_by_kind(child, "type_identifier") {
                if let Ok(text) = ident.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
        }
    }

    params
}

/// Extracts fields from Swift class_body.
fn extract_swift_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        // Get pattern (contains the name)
        if let Some(pattern) = find_child_by_kind(child, "pattern") {
            if let Some(ident) = find_child_by_kind(pattern, "simple_identifier") {
                if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                    // Get type annotation
                    let type_annotation = find_child_by_kind(child, "type_annotation")
                        .and_then(|ta| ta.child(1)) // Skip colon
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation,
                        doc_comment: None,
                    });
                }
            }
        }
    }

    fields
}

/// Extracts enum cases from Swift class_body (for enums).
fn extract_swift_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        // Try multiple possible node types for Swift enum cases
        let kind = child.kind();
        if kind == "enum_entry" || kind == "enum_case_pattern" {
            if let Some(ident) = find_child_by_kind(child, "simple_identifier") {
                if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                    variants.push(VariantInfo::unit(name));
                }
            }
        } else if kind == "switch_entry" {
            // Swift switch/case patterns
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "simple_identifier" {
                    if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                        variants.push(VariantInfo::unit(name));
                    }
                }
            }
        }
    }

    variants
}

/// Extracts method requirements from Swift protocol_body.
fn extract_swift_protocol_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "protocol_function_declaration" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "simple_identifier") {
            if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                // Get the full method signature
                let type_annotation = child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

// ============================================================================
// Scala type metadata extraction
// ============================================================================

/// Extracts type metadata from Scala class, trait, object, or enum definitions.
fn extract_scala_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_scala_type_parameters(type_params, source);
    }

    match node_kind {
        "class_definition" => {
            // Class parameters are in class_parameters
            if let Some(params) = find_child_by_kind(node, "class_parameters") {
                metadata.fields = extract_scala_class_parameters(params, source);
            }
        }
        "trait_definition" => {
            // Traits can have method declarations in template_body
            if let Some(body) = find_child_by_kind(node, "template_body") {
                metadata.fields = extract_scala_trait_methods(body, source);
            }
        }
        "object_definition" => {
            // Objects can have members in template_body
            if let Some(body) = find_child_by_kind(node, "template_body") {
                metadata.fields = extract_scala_object_members(body, source);
            }
        }
        "enum_definition" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_scala_enum_cases(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Scala type_parameters.
fn extract_scala_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        // Look for identifiers inside type parameters
        if child.kind() == "identifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                params.push(text.to_string());
            }
        }
    }

    params
}

/// Extracts class parameters from Scala class_parameters.
fn extract_scala_class_parameters(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "class_parameter" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                // Get type annotation (after colon)
                let type_annotation = find_child_by_kind(child, "type_identifier")
                    .or_else(|| find_child_by_kind(child, "generic_type"))
                    .or_else(|| find_child_by_kind(child, "compound_type"))
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts method declarations from Scala trait template_body.
fn extract_scala_trait_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "function_declaration" && kind != "function_definition" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "identifier") {
            if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                // Get the full method signature
                let type_annotation = child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts members from Scala object template_body.
fn extract_scala_object_members(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "function_definition" && kind != "val_definition" && kind != "var_definition" {
            continue;
        }

        // For function definitions
        if kind == "function_definition" {
            if let Some(ident) = find_child_by_kind(child, "identifier") {
                if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                    let type_annotation = child
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.trim().to_string());

                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation,
                        doc_comment: None,
                    });
                }
            }
        }
    }

    fields
}

/// Extracts enum cases from Scala enum_body.
fn extract_scala_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "simple_enum_case" && child.kind() != "enum_case_definitions" {
            continue;
        }

        if child.kind() == "simple_enum_case" {
            if let Some(ident) = find_child_by_kind(child, "identifier") {
                if let Ok(name) = ident.utf8_text(source.as_bytes()) {
                    variants.push(VariantInfo::unit(name));
                }
            }
        } else {
            // enum_case_definitions can contain multiple cases
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "identifier" {
                    if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                        variants.push(VariantInfo::unit(name));
                    }
                }
            }
        }
    }

    variants
}

// ============================================================================
// PHP type metadata extraction
// ============================================================================

/// Extracts type metadata from PHP class, interface, trait, or enum declarations.
fn extract_php_type_metadata(node: Node<'_>, node_kind: &str, source: &str) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_class_fields(body, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_interface_methods(body, source);
            }
        }
        "trait_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_declaration_list") {
                metadata.variants = extract_php_enum_cases(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from PHP class declaration_list.
fn extract_php_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        // Get type if present
        let type_annotation = find_child_by_kind(child, "named_type")
            .or_else(|| find_child_by_kind(child, "primitive_type"))
            .or_else(|| find_child_by_kind(child, "optional_type"))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get property elements
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "property_element" {
                if let Some(var_name) = find_child_by_kind(inner, "variable_name") {
                    if let Some(name_node) = find_child_by_kind(var_name, "name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            fields.push(FieldInfo {
                                name: name.to_string(),
                                type_annotation: type_annotation.clone(),
                                doc_comment: None,
                            });
                        }
                    }
                }
            }
        }
    }

    fields
}

/// Extracts method signatures from PHP interface declaration_list.
fn extract_php_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "name") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                // Get the full method signature
                let type_annotation = child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());

                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation,
                    doc_comment: None,
                });
            }
        }
    }

    fields
}

/// Extracts enum cases from PHP enum_declaration_list.
fn extract_php_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_case" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "name") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                variants.push(VariantInfo::unit(name));
            }
        }
    }

    variants
}
