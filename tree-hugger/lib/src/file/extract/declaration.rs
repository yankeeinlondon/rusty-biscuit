//! The declaration node a captured symbol name belongs to, and the facts read
//! from that node once at extraction: whether member or module access applies
//! to it, and its declaration text.

use super::*;
use crate::file::parents::IndexedParent as _;
use crate::shared::SymbolScope;

/// What visibility and static readers know about a declaration besides its node.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeclarationContext<'ctx, 'tree> {
    pub language: ProgrammingLanguage,
    pub source: &'tree str,
    /// The captured name. One declaration can introduce several names
    /// (`int a, *b;`, Swift `case east, west`), so a rule that depends on
    /// which name starts from here.
    pub name: Node<'tree>,
    /// The file's root node, for same-file lookups (a Rust trait's level, an
    /// earlier C/C++ `static` prototype, a Scala `package` clause).
    pub root: Node<'tree>,
    /// JavaScript/TypeScript only: the file is a module, so an unexported
    /// top-level declaration is ModulePrivate rather than a global. Computed
    /// once per file by [`is_ecma_module_file`].
    pub is_ecma_module: bool,
    /// The final kind, after classification.
    pub kind: SymbolKind,
    /// The declaration's [`symbol_scope`]: `Local` (a parameter, or inside a
    /// function body) is where member and module access do not apply.
    pub scope: SymbolScope,
    /// File facts shared by every declaration of the file, so a reader never
    /// re-scans the file per declaration.
    pub memo: &'ctx VisibilityMemo<'tree>,
}

impl DeclarationContext<'_, '_> {
    pub(crate) fn is_local(&self) -> bool {
        self.scope == SymbolScope::Local
    }
}

/// Node kinds of function-like constructs whose body makes the declarations
/// inside it local.
const FUNCTION_NODE_KINDS: &[&str] = &[
    "function_item",
    "function_signature_item",
    "closure_expression",
    "function_declaration",
    "function_definition",
    "function_expression",
    "function",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
    "method_declaration",
    "method_signature",
    "abstract_method_signature",
    "function_signature",
    "constructor_declaration",
    "compact_constructor_declaration",
    "destructor_declaration",
    "operator_declaration",
    "conversion_operator_declaration",
    "accessor_declaration",
    "local_function_statement",
    "lambda",
    "lambda_expression",
    "lambda_literal",
    "anonymous_function",
    "anonymous_function_creation_expression",
    "func_literal",
    "init_declaration",
    "deinit_declaration",
    "protocol_function_declaration",
];

/// Exact node kinds that declare a name but match none of the suffixes in
/// [`is_declaration_kind`].
const EXACT_DECLARATION_KINDS: &[&str] = &[
    "declaration",
    "parameter",
    "self_parameter",
    "closure_parameters",
    "enum_variant",
    "enum_entry",
    "enumerator",
    "enum_constant",
    // PHP `case A;`, Scala `case A, B` and `case C(x: Int)`, TypeScript
    // `A = 1` (a bare TypeScript member has no node of its own: its
    // declaration is the name).
    "enum_case",
    "simple_enum_case",
    "full_enum_case",
    "enum_assignment",
    // Go `type A = B`.
    "type_alias",
    // TypeScript `namespace A {}` and `module A {}`.
    "internal_module",
    "module",
    "associated_type",
    "assignment",
    "assignment_statement",
    "variable_assignment",
    "let_condition",
    "package_statement",
];

pub(crate) fn is_function_node_kind(kind: &str) -> bool {
    FUNCTION_NODE_KINDS.contains(&kind)
}

fn is_declaration_kind(kind: &str) -> bool {
    EXACT_DECLARATION_KINDS.contains(&kind)
        || [
            "_declaration",
            "_definition",
            "_item",
            "_signature",
            "_specifier",
            "_spec",
            "_parameter",
            "_clause",
        ]
        .iter()
        .any(|suffix| kind.ends_with(suffix))
}

/// A node at which the search for a name's declaration stops: a body, block
/// or file root. Crossing it would attribute the name to the enclosing
/// declaration.
///
/// Python's root is a `module` node, a boundary as every root is; a
/// TypeScript `module A {}` is a declaration, so `module` is not listed.
fn is_scope_boundary(node: Node<'_>) -> bool {
    is_scope_boundary_kind(node.kind()) || node.indexed_parent().is_none()
}

/// [`is_scope_boundary`] by kind alone; the root is a boundary whatever its
/// kind.
fn is_scope_boundary_kind(kind: &str) -> bool {
    is_block_like_dead_code_container(kind)
        || matches!(
            kind,
            "program"
                | "translation_unit"
                | "compilation_unit"
                | "chunk"
                | "declaration_list"
                | "field_declaration_list"
                | "enum_variant_list"
                | "template_body"
                | "statements"
                // A bare name in a parameter list is its own declaration.
                | "parameters"
                | "formal_parameters"
                | "parameter_list"
        )
        || (is_function_node_kind(kind) && !is_declaration_kind(kind))
}

/// Finds the declaration that introduces `name`, the node a query captured as
/// a symbol name.
///
/// This is the nearest declaration-like ancestor within the name's own
/// statement. When none is found before a body or block, the name's parent is
/// used, and the name itself when the parent is such a boundary.
pub(crate) fn resolve_declaration_node(name: Node<'_>) -> Node<'_> {
    let name_parent = name.indexed_parent();
    let mut current = name_parent;
    while let Some(node) = current {
        let kind = node.kind();
        if is_scope_boundary_kind(kind) {
            break;
        }
        // The root is a boundary too; its parent is the next step otherwise.
        let parent = node.indexed_parent();
        if parent.is_none() {
            break;
        }
        if is_declaration_kind(kind) {
            return climb_declaration_wrapper(node);
        }
        current = parent;
    }

    name_parent
        .filter(|parent| !is_scope_boundary(*parent))
        .unwrap_or(name)
}

/// The declaration a query's context capture stands for. C and C++ function
/// patterns capture the `function_declarator`, but storage class, return type
/// and the access section belong to the enclosing definition or declaration.
/// A `function_declarator` inside a parameter list stands for that parameter
/// (`void cb(int)` declares a parameter of function type).
pub(crate) fn declaration_for_context(context: Node<'_>) -> Node<'_> {
    if context.kind() != "function_declarator" {
        return context;
    }

    let mut current = context;
    while let Some(parent) = current.indexed_parent() {
        match parent.kind() {
            kind if kind.ends_with("_declarator") => current = parent,
            "function_definition"
            | "declaration"
            | "field_declaration"
            | "parameter_declaration"
            | "optional_parameter_declaration" => return parent,
            _ => break,
        }
    }
    context
}

/// The context capture that stands for `name` alone. A grouped Go
/// `type ( A …; B … )` block is the context of every type it declares, so
/// each name's own `type_spec` or `type_alias` stands in for it: the types of
/// a group never share a span, header, field list or container identity. An
/// ungrouped `type A …` keeps its `type_declaration`, which holds the `type`
/// keyword its header starts with.
pub(crate) fn context_for_name<'tree>(
    context: Node<'tree>,
    name: Node<'tree>,
    language: ProgrammingLanguage,
) -> Node<'tree> {
    if language == ProgrammingLanguage::Go
        && is_grouped_go_type_declaration(context)
        && let Some(spec) = name
            .indexed_parent()
            .filter(|spec| matches!(spec.kind(), "type_spec" | "type_alias") && spec.indexed_parent() == Some(context))
    {
        return spec;
    }
    context
}

/// A Go `type ( … )` block, which can declare several types.
pub(crate) fn is_grouped_go_type_declaration(node: Node<'_>) -> bool {
    node.kind() == "type_declaration" && find_child_by_kind(node, "(").is_some()
}

/// C# keeps access and storage modifiers on the statement that wraps a
/// `variable_declaration`, so that statement is the declaration. Every target
/// of a Python chained assignment (`first = second = 0`) is declared by the
/// outermost assignment, the statement.
fn climb_declaration_wrapper(node: Node<'_>) -> Node<'_> {
    if node.kind() == "assignment" {
        let mut outermost = node;
        while let Some(parent) = outermost
            .indexed_parent()
            .filter(|parent| parent.kind() == "assignment")
        {
            outermost = parent;
        }
        return outermost;
    }
    if node.kind() == "variable_declaration"
        && let Some(parent) = node.indexed_parent()
        && matches!(
            parent.kind(),
            "field_declaration" | "event_field_declaration" | "local_declaration_statement"
        )
    {
        return parent;
    }
    node
}

/// Bodies that are not functions but whose declarations are still local: a
/// Scala block expression (a `val`, `var` or alias `given` initializer, a
/// `match` arm), a Rust block expression (a `const` or `static`
/// initializer), a Java initializer block, and a Swift computed property,
/// accessor or observer body.
///
/// Python, Lua, Perl and the shells reuse `block` for class bodies or for
/// statements whose definitions stay global, so they have no entry here.
pub(crate) fn is_local_scope_kind(kind: &str, language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Scala => {
            matches!(kind, "block" | "indented_block" | "case_block")
        }
        ProgrammingLanguage::Rust | ProgrammingLanguage::Java => kind == "block",
        ProgrammingLanguage::Swift => matches!(
            kind,
            "computed_property"
                | "computed_getter"
                | "computed_setter"
                | "computed_modify"
                | "willset_didset_block"
        ),
        _ => false,
    }
}

/// The declaration's source text from its start up to, but not including, its
/// body, with trailing whitespace trimmed; the whole text when it has no body.
///
/// Leading modifiers, attributes and annotations are included when the
/// grammar parses them as part of the declaration node; doc comments never
/// are.
pub(crate) fn declaration_text(declaration: Node<'_>, source: &str) -> Option<String> {
    let end = body_node_for_declaration(declaration)
        .map(|body| body.start_byte())
        .filter(|start| *start >= declaration.start_byte() && *start <= declaration.end_byte())
        .unwrap_or_else(|| declaration.end_byte());
    let text = source.get(declaration.start_byte()..end)?.trim_end();
    (!text.is_empty()).then(|| text.to_string())
}

/// The context a C/C++ function or method pattern captures, for a function
/// name the query reached by a pattern that captures none (a name behind an
/// attribute or parentheses, a member function behind return declarators):
/// a member function's declaration, as the class-body patterns capture, or a
/// free function's own `function_declarator`, as the function pattern does.
pub(crate) fn implied_c_function_context<'tree>(
    language: ProgrammingLanguage,
    kind: SymbolKind,
    name: Node<'tree>,
    declaration: Node<'tree>,
) -> Option<Node<'tree>> {
    if !matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp) {
        return None;
    }
    match kind {
        SymbolKind::Method | SymbolKind::Constructor => Some(declaration),
        SymbolKind::Function => declared_function_named(declaration, name).map(|function| function.function),
        _ => None,
    }
}

