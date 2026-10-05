//! The declaration node a captured symbol name belongs to, and the facts read
//! from that node once at extraction: whether member or module access applies
//! to it, and its declaration text.

use super::*;

/// What visibility and static readers know about a declaration besides its node.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeclarationContext<'tree> {
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
    /// Inside a function body (a local variable, or a function nested in
    /// another), where member and module access do not apply.
    pub is_local: bool,
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
fn is_scope_boundary(node: Node<'_>) -> bool {
    let kind = node.kind();
    node.parent().is_none()
        || is_block_like_dead_code_container(kind)
        || matches!(
            kind,
            "program"
                | "module"
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
    let mut current = name.parent();
    while let Some(node) = current {
        if is_scope_boundary(node) {
            break;
        }
        if is_declaration_kind(node.kind()) {
            return climb_declaration_wrapper(node);
        }
        current = node.parent();
    }

    name.parent()
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
    while let Some(parent) = current.parent() {
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

/// C# keeps access and storage modifiers on the statement that wraps a
/// `variable_declaration`, so that statement is the declaration. Every target
/// of a Python chained assignment (`first = second = 0`) is declared by the
/// outermost assignment, the statement.
fn climb_declaration_wrapper(node: Node<'_>) -> Node<'_> {
    if node.kind() == "assignment" {
        let mut outermost = node;
        while let Some(parent) = outermost
            .parent()
            .filter(|parent| parent.kind() == "assignment")
        {
            outermost = parent;
        }
        return outermost;
    }
    if node.kind() == "variable_declaration"
        && let Some(parent) = node.parent()
        && matches!(
            parent.kind(),
            "field_declaration" | "event_field_declaration" | "local_declaration_statement"
        )
    {
        return parent;
    }
    node
}

/// Whether `declaration` sits inside a function-like construct or a
/// block-valued initializer or accessor, where member and module access do
/// not apply. The nearest function, local scope or type owner decides, so a
/// member of a class declared in a function (a Java local or anonymous class)
/// keeps its access.
pub(crate) fn is_local_declaration(declaration: Node<'_>, language: ProgrammingLanguage) -> bool {
    let mut current = member_slot(declaration).parent();
    while let Some(node) = current {
        if is_member_owner_kind(node.kind()) {
            return false;
        }
        if is_function_node_kind(node.kind()) || is_local_scope_kind(node.kind(), language) {
            return true;
        }
        current = node.parent();
    }
    false
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

/// Whether a C/C++ symbol is a `typedef` name.
pub(crate) fn is_c_typedef_name(language: ProgrammingLanguage, kind: SymbolKind, declaration: Node<'_>) -> bool {
    matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp)
        && kind == SymbolKind::TypeAlias
        && declaration.kind() == "type_definition"
}
