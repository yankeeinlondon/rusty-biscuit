//! Symbol-kind classification, container detection, node spans, and export
//! detection helpers carved out of `tree_file.rs`.

use super::*;

pub(crate) fn is_block_like_dead_code_container(kind: &str) -> bool {
    kind.contains("block")
        || kind.contains("body")
        || kind.contains("compound")
        || kind == "statement_list"
        || kind == "source_file"
}

pub(crate) fn symbol_kind_priority(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Constructor => 105,
        SymbolKind::Method => 100,
        SymbolKind::Function => 90,
        SymbolKind::Constant => 85,
        SymbolKind::Property => 82,
        SymbolKind::Field => 80,
        SymbolKind::Parameter => 75,
        SymbolKind::Class
        | SymbolKind::Interface
        | SymbolKind::Enum
        | SymbolKind::EnumVariant
        | SymbolKind::Trait
        | SymbolKind::Type
        | SymbolKind::TypeAlias
        | SymbolKind::Module
        | SymbolKind::Namespace => 70,
        SymbolKind::Macro => 65,
        SymbolKind::Variable => 50,
        SymbolKind::Unknown => 0,
    }
}

/// Decides a capture's final kind from its declaration node, language and
/// name, so `symbols()`, `symbol_records()` and the CLI agree on every kind.
///
/// Rules keyed on the declaration node outrank the query capture. Only the
/// path from the name up to its declaration is inspected, never enclosing
/// bodies: a local is not a field because its function sits in a class.
///
/// A member kind, from a query capture or from a rule here, is kept only for
/// a declaration that belongs to a type; one in a function body is demoted
/// (see [`is_function_local_member`]).
pub(crate) fn refine_symbol_kind(
    kind: SymbolKind,
    name: Node<'_>,
    declaration: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> SymbolKind {
    let refined = classify_symbol_kind(kind, name, declaration, language, source);
    match refined {
        SymbolKind::Method | SymbolKind::Constructor
            if is_function_local_member(name, declaration, language) =>
        {
            SymbolKind::Function
        }
        SymbolKind::Field | SymbolKind::Property
            if is_function_local_member(name, declaration, language) =>
        {
            SymbolKind::Variable
        }
        _ => refined,
    }
}

fn classify_symbol_kind(
    kind: SymbolKind,
    name: Node<'_>,
    declaration: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> SymbolKind {
    match (language, declaration.kind()) {
        (_, "type_alias_declaration") | (ProgrammingLanguage::Rust, "type_item" | "associated_type") => {
            return SymbolKind::TypeAlias;
        }
        (ProgrammingLanguage::Rust, "enum_variant") => return SymbolKind::EnumVariant,
        // A function-typed C/C++ parameter (`void cb(int)`) is captured by the
        // function pattern.
        (
            ProgrammingLanguage::C | ProgrammingLanguage::Cpp,
            "parameter_declaration" | "optional_parameter_declaration",
        ) => return SymbolKind::Parameter,
        (ProgrammingLanguage::Rust, "mod_item") => return SymbolKind::Module,
        // A Rust `static` is a variable with static storage, not a constant.
        (ProgrammingLanguage::Rust, "static_item") => return SymbolKind::Variable,
        // A later target of a chained class-body assignment (`second` in
        // `first = second = 0`) is captured only by the generic assignment
        // pattern; its declaration is the class-body statement.
        (ProgrammingLanguage::Python, "assignment")
            if kind == SymbolKind::Variable && is_python_class_attribute_statement(declaration) =>
        {
            return SymbolKind::Field;
        }
        (_, "public_field_definition" | "property_definition" | "property_signature")
        | (ProgrammingLanguage::JavaScript, "field_definition") => {
            return SymbolKind::Property;
        }
        // An object literal's `constructor()` is an ordinary method.
        (_, "method_definition")
            if node_text(name, source) == Some("constructor")
                && declaration
                    .parent()
                    .is_some_and(|parent| parent.kind() == "class_body") =>
        {
            return SymbolKind::Constructor;
        }
        _ => {}
    }

    // The C/C++ queries capture a name behind an attribute or parentheses
    // (`int f [[deprecated]] (int)`, `int (f)(int)`) by its immediate parent,
    // which cannot tell a function from a variable or field.
    let kind = if matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp)
        && matches!(kind, SymbolKind::Variable | SymbolKind::Field)
        && declared_function_named(declaration, name).is_some()
    {
        SymbolKind::Function
    } else {
        kind
    };

    match kind {
        SymbolKind::Function if is_direct_member(declaration) => SymbolKind::Method,
        SymbolKind::Variable => refine_variable_kind(name, declaration, language),
        _ => kind,
    }
}

/// Whether a Python assignment is a statement directly in a class body.
fn is_python_class_attribute_statement(assignment: Node<'_>) -> bool {
    assignment
        .parent()
        .filter(|statement| statement.kind() == "expression_statement")
        .and_then(|statement| statement.parent())
        .filter(|block| block.kind() == "block")
        .and_then(|block| block.parent())
        .is_some_and(|owner| owner.kind() == "class_definition")
}

fn node_text<'src>(node: Node<'_>, source: &'src str) -> Option<&'src str> {
    node.utf8_text(source.as_bytes()).ok()
}

fn refine_variable_kind(
    name: Node<'_>,
    declaration: Node<'_>,
    language: ProgrammingLanguage,
) -> SymbolKind {
    let mut current = Some(name);
    while let Some(cursor) = current {
        match cursor.kind() {
            "self_parameter"
            | "parameter"
            | "parameter_declaration"
            | "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "required_parameter"
            | "optional_parameter"
            | "formal_parameter"
            | "formal_parameters"
            | "parameters"
            | "parameter_list" => return SymbolKind::Parameter,
            // tree-sitter-swift parses every `let`/`var` as a
            // `property_declaration`; only one in a type body is a field.
            "property_declaration"
                if language == ProgrammingLanguage::Swift && !is_in_type_body(cursor) =>
            {
                return SymbolKind::Variable;
            }
            "field_declaration"
            | "event_field_declaration"
            | "property_declaration"
            | "field_definition"
            | "property_initializer" => return SymbolKind::Field,
            "const_item" | "const_declaration" | "constant" => return SymbolKind::Constant,
            _ => {}
        }

        if cursor == declaration {
            break;
        }
        current = cursor.parent();
    }

    // A C++ variable template member (`template<class T> static T x;`) parses
    // as a plain `declaration` under its template header, not a
    // `field_declaration`.
    if language == ProgrammingLanguage::Cpp
        && declaration.kind() == "declaration"
        && is_direct_member(declaration)
    {
        return SymbolKind::Field;
    }

    // PHP `simple_parameter`, Java `catch_formal_parameter` and the like, or
    // a bare name directly in a parameter list.
    if declaration.kind().ends_with("_parameter")
        || declaration.parent().is_some_and(|parent| {
            matches!(
                parent.kind(),
                "parameters" | "formal_parameters" | "parameter_list"
            )
        })
    {
        return SymbolKind::Parameter;
    }
    // Go `const ( A = 1 )`: the keyword sits on the spec's parent.
    if declaration
        .parent()
        .is_some_and(|parent| parent.kind() == "const_declaration")
    {
        return SymbolKind::Constant;
    }

    SymbolKind::Variable
}

fn is_in_type_body(declaration: Node<'_>) -> bool {
    declaration.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "class_body" | "enum_class_body" | "protocol_body"
        )
    })
}

/// Declaration nodes whose body makes a directly declared function a method.
fn is_member_container_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "class_definition"
            | "abstract_class_declaration"
            | "class_specifier"
            | "struct_specifier"
            | "union_specifier"
            | "struct_declaration"
            | "record_declaration"
            | "enum_declaration"
            | "interface_declaration"
            | "protocol_declaration"
            | "extension_declaration"
            | "trait_declaration"
            | "trait_definition"
            | "object_definition"
            // Scala `new T { … }`: an anonymous class.
            | "instance_expression"
            | "enum_definition"
            // Scala `given … with` instance body: an anonymous class.
            | "with_template_body"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "trait_item"
            | "impl_item"
    )
}

/// Body nodes between a member and its container.
fn is_member_body_kind(kind: &str) -> bool {
    kind.ends_with("_body")
        || matches!(
            kind,
            "block" | "declaration_list" | "field_declaration_list" | "template_body"
        )
}

/// Nodes that wrap a declaration without opening a scope of their own: a
/// C++ template header, a C, C++ or C# preprocessor conditional (the grammars
/// share these node kinds), a Python decorator
/// list, a Scala `extension` block (its methods belong to the type around
/// it, or to no type at top level). A member keeps its membership through any number of them, and so
/// does a namespace-scope or local declaration keep its own scope.
pub(crate) fn is_transparent_wrapper_kind(kind: &str) -> bool {
    matches!(
        kind,
        "template_declaration"
            | "decorated_definition"
            | "extension_definition"
            | "preproc_if"
            | "preproc_ifdef"
            | "preproc_else"
            | "preproc_elif"
            | "preproc_elifdef"
    )
}

/// The node that occupies `declaration`'s place in its enclosing body or
/// scope: the declaration itself, or the outermost transparent wrapper
/// around it (see [`is_transparent_wrapper_kind`]).
///
/// Every "is this a member, and of what" decision starts from this node, so
/// kind, member `static`, storage `static` and access agree.
pub(crate) fn member_slot(declaration: Node<'_>) -> Node<'_> {
    let mut slot = declaration;
    while let Some(parent) = slot.parent()
        && is_transparent_wrapper_kind(parent.kind())
    {
        slot = parent;
    }
    slot
}

/// Whether `declaration` is declared directly in a class, struct, enum,
/// trait, interface, protocol, object, `impl` or extension body, rather than
/// inside a function.
pub(crate) fn is_direct_member(declaration: Node<'_>) -> bool {
    let mut current = member_slot(declaration).parent();
    while let Some(parent) = current {
        if is_member_container_kind(parent.kind()) {
            return true;
        }
        if !is_member_body_kind(parent.kind()) {
            return false;
        }
        current = parent.parent();
    }
    false
}

/// Nodes that own the members declared in them without being one of
/// [`is_member_container_kind`]'s declarations: type bodies (also of an
/// anonymous class), a class expression and an object literal.
pub(crate) fn is_member_owner_kind(kind: &str) -> bool {
    is_member_container_kind(kind)
        || matches!(
            kind,
            "class_body"
                | "enum_body"
                | "enum_class_body"
                | "interface_body"
                | "protocol_body"
                | "template_body"
                | "declaration_list"
                | "field_declaration_list"
                | "class"
                | "object"
        )
}

/// Whether a declaration given a member kind actually sits in a function
/// body, such as a C# local function, which some queries capture as a
/// method. The nearest enclosing function or member owner decides, so a
/// member of a class declared inside a function stays a member.
///
/// A Lua `function M:m()` or `function M.f()` names its table itself, so it
/// keeps its kind wherever it is written.
fn is_function_local_member(
    name: Node<'_>,
    declaration: Node<'_>,
    language: ProgrammingLanguage,
) -> bool {
    if language == ProgrammingLanguage::Lua
        && name.parent().is_some_and(|parent| {
            matches!(
                parent.kind(),
                "method_index_expression" | "dot_index_expression"
            )
        })
    {
        return false;
    }

    let mut current = member_slot(declaration).parent();
    while let Some(node) = current {
        if is_member_owner_kind(node.kind()) {
            return false;
        }
        if is_function_node_kind(node.kind()) {
            return true;
        }
        current = node.parent();
    }
    false
}

/// How a symbol's container is identified.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ContainerLookup<'tree> {
    /// The container is this declaration node; its kind is the kind extracted
    /// for that node, so containers and standalone symbols share one
    /// classifier. `fallback` applies when no symbol is extracted there.
    Node {
        node: Node<'tree>,
        fallback: SymbolKind,
    },
    /// A Rust `impl` member or a Go receiver method: the implemented type,
    /// found by name among the file's type declarations.
    SameFileType,
    /// A Lua `M.f` / `M:m`: the table, found by name among the file's
    /// item-scoped declarations.
    SameFileTable,
}

/// The nearest declaration containing a symbol.
#[derive(Debug, Clone)]
pub(crate) struct ContainerRef<'tree> {
    pub name: String,
    pub lookup: ContainerLookup<'tree>,
}

/// Finds the nearest container of `declaration`. The search starts at the
/// declaration's parent, so a declaration is never its own container.
///
/// Sibling-scoped containers (a C# file-scoped namespace, a Perl
/// `package Foo;` statement) are found among the preceding siblings at each
/// level, before that level's parent.
pub(crate) fn find_symbol_container<'tree>(
    declaration: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ContainerRef<'tree>> {
    // A Perl package name is absolute: `package B` inside or after
    // `package A` is `B`, not a member of `A`.
    if language == ProgrammingLanguage::Perl && declaration.kind() == "package_statement" {
        return None;
    }

    // A Go method's container is its receiver's type; what is declared
    // inside the method (receiver, parameters, locals) has none, as in any
    // Go function.
    if language == ProgrammingLanguage::Go && declaration.kind() == "method_declaration" {
        return go_receiver_type_name(declaration, source).map(|name| ContainerRef {
            name,
            lookup: ContainerLookup::SameFileType,
        });
    }

    let mut child = declaration;
    while let Some(parent) = child.parent() {
        if !is_sibling_scoped_container(child, language)
            && let Some(container) = preceding_sibling_container(child, language, source)
        {
            return Some(container);
        }

        if language == ProgrammingLanguage::Rust && parent.kind() == "impl_item" {
            return impl_self_type_name(parent, source).map(|name| ContainerRef {
                name,
                lookup: ContainerLookup::SameFileType,
            });
        }

        if is_unnamed_type_owner(parent) {
            return None;
        }
        if let Some(container) = ancestor_container(parent, language, source) {
            return Some(container);
        }

        child = parent;
    }

    None
}

/// The base type a Go method's receiver names (`func (p *Point[T]) M()` →
/// `Point`).
pub(crate) fn go_receiver_type_name(method: Node<'_>, source: &str) -> Option<String> {
    let receiver = method.child_by_field_name("receiver")?;
    let mut cursor = receiver.walk();
    let parameter = receiver
        .named_children(&mut cursor)
        .find(|child| child.kind() == "parameter_declaration")?;
    let mut node = parameter.child_by_field_name("type")?;
    loop {
        node = match node.kind() {
            "pointer_type" => node.named_child(0)?,
            "generic_type" => node.child_by_field_name("type")?,
            "type_identifier" => {
                return node.utf8_text(source.as_bytes()).ok().map(str::to_string);
            }
            _ => return None,
        };
    }
}

/// A type that owns the declarations in it but has no name to report: an
/// anonymous class (Java, also an enum constant's body; PHP; Scala
/// `new T { … }`, also a `given … with` instance), a JavaScript/TypeScript
/// class expression or object literal, a C# anonymous object, and a C/C++ `struct`/`class`/`union`
/// without a tag. A declaration inside one has no container rather than the
/// named type around it.
///
/// A tagless C/C++ `struct`/`union` that declares no member of its own
/// (`union { int a; float b; };`) is an anonymous member: its fields are
/// members of the enclosing type, so it is not a boundary.
pub(crate) fn is_unnamed_type_owner(node: Node<'_>) -> bool {
    match node.kind() {
        "class" | "object" | "anonymous_class" | "anonymous_object_creation_expression" => true,
        "class_body" => node.parent().is_some_and(|parent| {
            matches!(parent.kind(), "object_creation_expression" | "enum_constant")
        }),
        "template_body" => node
            .parent()
            .is_some_and(|parent| parent.kind() == "instance_expression"),
        // A Scala `given` is not an extracted symbol, so its instance body
        // names no container.
        "with_template_body" => true,
        "struct_specifier" | "class_specifier" | "union_specifier" => {
            node.child_by_field_name("name").is_none()
                && anonymous_member_declaration(node).is_none()
        }
        _ => false,
    }
}

/// For a C/C++ anonymous member (`union { int a; float b; };`, a tagless
/// `struct`/`union` that declares no name of its own), the
/// `field_declaration` that places it in the enclosing type. Its members are
/// members of that type, with that declaration's access.
pub(crate) fn anonymous_member_declaration(specifier: Node<'_>) -> Option<Node<'_>> {
    if !matches!(specifier.kind(), "struct_specifier" | "class_specifier" | "union_specifier")
        || specifier.child_by_field_name("name").is_some()
    {
        return None;
    }
    specifier.parent().filter(|parent| {
        parent.kind() == "field_declaration" && parent.child_by_field_name("declarator").is_none()
    })
}

pub(crate) fn ancestor_container<'tree>(
    node: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ContainerRef<'tree>> {
    let kind_map: &[(&str, SymbolKind)] = match language {
        ProgrammingLanguage::Rust => &[
            ("trait_item", SymbolKind::Trait),
            ("struct_item", SymbolKind::Type),
            ("union_item", SymbolKind::Type),
            ("enum_item", SymbolKind::Enum),
            ("mod_item", SymbolKind::Module),
        ],
        // tree-sitter-swift parses every type kind, extensions included, as
        // `class_declaration`; standalone Swift types report `Type`.
        ProgrammingLanguage::Swift => &[
            ("class_declaration", SymbolKind::Type),
            ("protocol_declaration", SymbolKind::Interface),
        ],
        ProgrammingLanguage::Perl => {
            return match node.kind() {
                "package_statement" => perl_package_container(node, source),
                "function_definition" => named_container_ref(
                    node,
                    source,
                    &[("function_definition", SymbolKind::Function)],
                ),
                _ => None,
            };
        }
        _ => &[
            ("class_declaration", SymbolKind::Class),
            ("class_definition", SymbolKind::Class),
            ("abstract_class_declaration", SymbolKind::Class),
            ("class_specifier", SymbolKind::Class),
            ("struct_declaration", SymbolKind::Type),
            ("struct_specifier", SymbolKind::Type),
            ("union_specifier", SymbolKind::Type),
            ("enum_declaration", SymbolKind::Enum),
            ("enum_specifier", SymbolKind::Enum),
            ("interface_declaration", SymbolKind::Interface),
            ("protocol_declaration", SymbolKind::Interface),
            ("trait_declaration", SymbolKind::Trait),
            ("trait_definition", SymbolKind::Trait),
            ("object_definition", SymbolKind::Module),
            ("enum_definition", SymbolKind::Enum),
            ("record_declaration", SymbolKind::Type),
            ("namespace_declaration", SymbolKind::Namespace),
            ("namespace_definition", SymbolKind::Namespace),
            ("module", SymbolKind::Module),
            ("type_declaration", SymbolKind::Type),
        ],
    };

    named_container_ref(node, source, kind_map)
}

fn named_container_ref<'tree>(
    node: Node<'tree>,
    source: &str,
    kind_map: &[(&str, SymbolKind)],
) -> Option<ContainerRef<'tree>> {
    named_container(node, source, kind_map).map(|(name, fallback)| ContainerRef {
        name,
        lookup: ContainerLookup::Node { node, fallback },
    })
}

/// Whether `node` is a container that scopes its following siblings rather
/// than its children.
fn is_sibling_scoped_container(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::CSharp => node.kind() == "file_scoped_namespace_declaration",
        ProgrammingLanguage::Perl => {
            node.kind() == "package_statement" && find_child_by_kind(node, "block").is_none()
        }
        _ => false,
    }
}

fn preceding_sibling_container<'tree>(
    node: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ContainerRef<'tree>> {
    if !matches!(language, ProgrammingLanguage::CSharp | ProgrammingLanguage::Perl) {
        return None;
    }

    let mut sibling = node.prev_sibling();
    while let Some(candidate) = sibling {
        if is_sibling_scoped_container(candidate, language) {
            return match language {
                ProgrammingLanguage::Perl => perl_package_container(candidate, source),
                _ => named_container_ref(
                    candidate,
                    source,
                    &[("file_scoped_namespace_declaration", SymbolKind::Namespace)],
                ),
            };
        }
        sibling = candidate.prev_sibling();
    }
    None
}

fn perl_package_container<'tree>(node: Node<'tree>, source: &str) -> Option<ContainerRef<'tree>> {
    let name = find_child_by_kind(node, "package_name")?
        .utf8_text(source.as_bytes())
        .ok()?
        .to_string();
    Some(ContainerRef {
        name,
        lookup: ContainerLookup::Node {
            node,
            fallback: SymbolKind::Module,
        },
    })
}

pub(crate) fn named_container(
    node: Node<'_>,
    source: &str,
    kind_map: &[(&str, SymbolKind)],
) -> Option<(String, SymbolKind)> {
    let container_kind = kind_map
        .iter()
        .find_map(|(kind_name, mapped_kind)| (*kind_name == node.kind()).then_some(*mapped_kind))?;

    // Common field names used by tree-sitter grammars for declaration identifiers.
    let name = ["name", "declarator", "identifier", "type", "path"]
        .iter()
        .find_map(|field| {
            node.child_by_field_name(field)
                .and_then(|value| value.utf8_text(source.as_bytes()).ok())
                .map(str::to_string)
        })
        .or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find_map(|child| match child.kind() {
                    "identifier" | "type_identifier" | "field_identifier" => {
                        child.utf8_text(source.as_bytes()).ok().map(str::to_string)
                    }
                    _ => None,
                })
        })?;

    Some((name, container_kind))
}

pub(crate) fn is_definition_like_reference(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };

    if is_named_definition_node(parent, node) {
        return true;
    }

    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => matches!(
            parent.kind(),
            "property_signature"
                | "method_signature"
                | "required_parameter"
                | "optional_parameter"
                | "rest_pattern"
                | "shorthand_property_identifier_pattern"
                | "pair_pattern"
        ),
        ProgrammingLanguage::Rust => match parent.kind() {
            // In `value: Type` / `field: Type`, the binding name (pattern/name)
            // is definition-like, but the `type` child is a genuine reference —
            // excluding it makes type-only imports look unused. Skip only the
            // non-type child.
            "parameter" | "field_declaration" => parent.child_by_field_name("type") != Some(node),
            "self_parameter" | "use_as_clause" => true,
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn is_named_definition_node(parent: Node<'_>, node: Node<'_>) -> bool {
    let parent_kind = parent.kind();
    if !matches!(
        parent_kind,
        "function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "method_definition"
            | "property_signature"
            | "method_signature"
            | "public_field_definition"
            | "variable_declarator"
            | "import_specifier"
            | "namespace_import"
            | "required_parameter"
            | "optional_parameter"
            | "formal_parameter"
            | "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "mod_item"
            | "const_item"
            | "static_item"
            | "type_item"
            | "field_declaration"
            | "macro_definition"
    ) {
        return false;
    }

    ["name", "alias", "declarator", "value", "parameter", "path"]
        .iter()
        .filter_map(|field| parent.child_by_field_name(field))
        .any(|candidate| candidate.id() == node.id())
}

pub(crate) fn symbol_kind_from_capture(capture_name: &str) -> Option<SymbolKind> {
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
        "constructor" => Some(SymbolKind::Constructor),
        "property" => Some(SymbolKind::Property),
        "type_alias" => Some(SymbolKind::TypeAlias),
        "enum_variant" => Some(SymbolKind::EnumVariant),
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

pub(crate) fn range_for_node(node: Node<'_>) -> CodeRange {
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

pub(crate) fn text_span_from_code_range(range: &CodeRange) -> TextSpan {
    TextSpan {
        start: TextPoint {
            line: range.start_line as u32,
            column: range.start_column as u32,
        },
        end: TextPoint {
            line: range.end_line as u32,
            column: range.end_column as u32,
        },
        start_byte: range.start_byte as u32,
        end_byte: range.end_byte as u32,
    }
}

pub(crate) fn text_span_for_node(node: Node<'_>) -> TextSpan {
    text_span_from_code_range(&range_for_node(node))
}

pub(crate) fn body_span_for_declaration(node: Node<'_>) -> Option<TextSpan> {
    body_node_for_declaration(node).map(text_span_for_node)
}

pub(crate) fn body_node_for_declaration(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("body")
        .or_else(|| find_child_by_kind(node, "class_body"))
        .or_else(|| find_child_by_kind(node, "enum_body"))
        .or_else(|| find_child_by_kind(node, "interface_body"))
        .or_else(|| find_child_by_kind(node, "object_type"))
}

/// The right-hand side of a type alias declaration (TypeScript `type A = …`,
/// Rust `type A = …`).
pub(crate) fn extract_type_alias_target(node: Node<'_>, source: &str) -> Option<String> {
    if node.kind() == "type_item" {
        return node
            .child_by_field_name("type")
            .and_then(|target| target.utf8_text(source.as_bytes()).ok())
            .map(str::to_string);
    }
    if node.kind() != "type_alias_declaration" {
        return None;
    }

    node.child_by_field_name("value")
        .or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "object_type" || child.kind().ends_with("type"))
        })
        .and_then(|target| target.utf8_text(source.as_bytes()).ok())
        .map(str::to_string)
}

pub(crate) fn is_exported_definition(symbol: &SymbolInfo, node: Node<'_>, root: Node<'_>) -> bool {
    if matches!(
        symbol.kind,
        SymbolKind::Parameter
            | SymbolKind::Field
            | SymbolKind::Property
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::EnumVariant
    ) {
        return false;
    }

    if symbol.language == ProgrammingLanguage::Rust {
        return is_rust_public_definition(node);
    }

    if !is_top_level_definition(node, root) {
        return false;
    }

    // `export const a = 1, f = () => …;` exports each declarator, and a
    // function bound to one is anchored at the declarator's value.
    let mut declaration = node;
    while let Some(parent) = declaration.parent()
        && matches!(parent.kind(), "variable_declarator" | "lexical_declaration" | "variable_declaration")
    {
        declaration = parent;
    }
    declaration.parent().is_some_and(|parent| is_export_node(parent.kind()))
}

pub(crate) fn is_rust_public_definition(node: Node<'_>) -> bool {
    let mut current = Some(node);

    while let Some(cursor_node) = current {
        let mut cursor = cursor_node.walk();
        for child in cursor_node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return true;
            }
        }

        if matches!(
            cursor_node.kind(),
            "function_item"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "mod_item"
                | "const_item"
                | "static_item"
                | "type_item"
                | "macro_definition"
                | "impl_item"
        ) {
            return false;
        }

        current = cursor_node.parent();
    }

    false
}

pub(crate) fn is_export_node(kind: &str) -> bool {
    matches!(
        kind,
        "export_statement"
            | "export_declaration"
            | "export_default_declaration"
            | "public_declaration"
    )
}

pub(crate) fn is_top_level_definition(node: Node<'_>, root: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(cursor) = current {
        let Some(parent) = cursor.parent() else {
            break;
        };

        if parent == root {
            return true;
        }

        if matches!(
            parent.kind(),
            "class_body"
                | "interface_body"
                | "object_type"
                | "statement_block"
                | "field_declaration_list"
                | "enum_body"
        ) {
            return false;
        }

        current = Some(parent);
    }

    false
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn parse(language: ProgrammingLanguage, file_name: &str, source: &str) -> tree_sitter::Tree {
        let (grammar, _) = language.grammar_for_path(Path::new(file_name));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).expect("grammar");
        parser.parse(source, None).expect("parse")
    }

    /// The first node of `kind` whose text starts with `prefix`, in document order.
    fn find_node<'tree>(node: Node<'tree>, kind: &str, prefix: &str, source: &str) -> Option<Node<'tree>> {
        if node.kind() == kind && source[node.byte_range()].starts_with(prefix) {
            return Some(node);
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
        children
            .into_iter()
            .find_map(|child| find_node(child, kind, prefix, source))
    }

    fn container_name(declaration: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<String> {
        find_symbol_container(declaration, language, source).map(|container| container.name)
    }

    /// The Perl rules ahead of the Perl query: a `package Foo;` statement
    /// scopes the declarations after it, a block package its contents, and a
    /// sub the variables inside it.
    #[test]
    fn perl_packages_and_subs_contain_the_declarations_they_scope() {
        let source = include_str!("../../../tests/fixtures/symbols/perl.pl");
        let tree = parse(ProgrammingLanguage::Perl, "fixture.pl", source);
        let root = tree.root_node();
        let perl = ProgrammingLanguage::Perl;
        let node = |kind: &str, prefix: &str| {
            find_node(root, kind, prefix, source)
                .unwrap_or_else(|| panic!("no {kind} starting with {prefix:?}"))
        };

        let cases = [
            (node("function_definition", "sub greet"), Some("Fixture::Greeter")),
            (node("function_definition", "sub other_package_sub"), Some("Fixture::Other")),
            (node("function_definition", "sub block_package_sub"), Some("Fixture::Block")),
            // After the block form closes, the earlier statement form resumes.
            (node("function_definition", "sub after_block_sub"), Some("Fixture::Other")),
            (node("variable_declaration", "our $package_level_our"), Some("Fixture::Greeter")),
            (node("variable_declaration", "our $block_package_our"), Some("Fixture::Block")),
            (node("variable_declaration", "my $joined_name"), Some("greet")),
            (node("variable_declaration", "my $block_local"), Some("block_package_sub")),
            // A package statement is not contained by the package before it.
            (node("package_statement", "package Fixture::Other"), None),
        ];
        for (declaration, expected) in cases {
            assert_eq!(
                container_name(declaration, perl, source).as_deref(),
                expected,
                "container of {:?}",
                source[declaration.byte_range()].lines().next().unwrap_or_default()
            );
        }

        let block_sub = node("function_definition", "sub block_package_sub");
        let container = find_symbol_container(block_sub, perl, source).expect("container");
        assert!(matches!(
            container.lookup,
            ContainerLookup::Node { fallback: SymbolKind::Module, .. }
        ));
    }

    #[test]
    fn perl_list_declaration_text_covers_every_name() {
        let source = include_str!("../../../tests/fixtures/symbols/perl.pl");
        let tree = parse(ProgrammingLanguage::Perl, "fixture.pl", source);
        let root = tree.root_node();
        for name in ["$first_name", "$last_name"] {
            let variable = find_node(root, "scalar_variable", name, source).expect("list name");
            let declaration = resolve_declaration_node(variable);
            assert_eq!(
                declaration_text(declaration, source).as_deref(),
                Some("my ($first_name, $last_name)"),
                "{name}"
            );
        }
    }

    #[test]
    fn rust_type_alias_target_is_the_right_hand_side() {
        let source = "pub type Meters = u32;\ntype Pair<T> = (T, T);\n";
        let tree = parse(ProgrammingLanguage::Rust, "alias.rs", source);
        let root = tree.root_node();
        let meters = find_node(root, "type_item", "pub type Meters", source).expect("Meters");
        let pair = find_node(root, "type_item", "type Pair", source).expect("Pair");
        assert_eq!(extract_type_alias_target(meters, source).as_deref(), Some("u32"));
        assert_eq!(extract_type_alias_target(pair, source).as_deref(), Some("(T, T)"));

        let name = meters.child_by_field_name("name").expect("name");
        assert_eq!(
            refine_symbol_kind(SymbolKind::Type, name, meters, ProgrammingLanguage::Rust, source),
            SymbolKind::TypeAlias
        );
    }

    #[test]
    fn capture_names_cover_every_extracted_kind() {
        for (capture, kind) in [
            ("local.definition.constructor", SymbolKind::Constructor),
            ("local.definition.property", SymbolKind::Property),
            ("local.definition.type_alias", SymbolKind::TypeAlias),
            ("local.definition.enum_variant", SymbolKind::EnumVariant),
            ("local.definition.method", SymbolKind::Method),
        ] {
            assert_eq!(symbol_kind_from_capture(capture), Some(kind), "{capture}");
        }
        assert_eq!(symbol_kind_from_capture("local.definition.property.context"), None);
    }

    #[test]
    fn rust_static_item_is_a_variable() {
        let source = "pub static LIMIT: u32 = 3;\n";
        let tree = parse(ProgrammingLanguage::Rust, "static.rs", source);
        let item = find_node(tree.root_node(), "static_item", "pub static", source).expect("static");
        let name = item.child_by_field_name("name").expect("name");
        assert_eq!(
            refine_symbol_kind(SymbolKind::Constant, name, item, ProgrammingLanguage::Rust, source),
            SymbolKind::Variable
        );
    }
}
