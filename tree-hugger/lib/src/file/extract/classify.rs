//! Symbol-kind classification, container detection, node spans, and export
//! detection helpers carved out of `tree_file.rs`.

use std::cell::RefCell;
use std::collections::HashMap;

use super::*;
use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::count_shared_context_steps;

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
/// path from the name up to its declaration, and for membership the bodies
/// directly around it ([`is_direct_member`]), are inspected: a local is not a
/// field because its function sits in a class.
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
        // A `val`/`var` declared directly in a class, trait, object or enum
        // body is a field; one in a method or block stays a variable.
        (
            ProgrammingLanguage::Scala,
            "val_definition" | "var_definition" | "val_declaration" | "var_declaration",
        ) if kind == SymbolKind::Variable && is_direct_member(declaration) => {
            return SymbolKind::Field;
        }
        (_, "public_field_definition" | "property_definition" | "property_signature")
        | (ProgrammingLanguage::JavaScript, "field_definition") => {
            return SymbolKind::Property;
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
        SymbolKind::Function | SymbolKind::Method
            if is_constructor_declaration(name, declaration, language, source) =>
        {
            SymbolKind::Constructor
        }
        SymbolKind::Function if is_direct_member(declaration) => SymbolKind::Method,
        SymbolKind::Variable => refine_variable_kind(name, declaration, language),
        _ => kind,
    }
}

/// Whether a declaration captured as a function or method is its type's
/// constructor (JavaScript/TypeScript, Python, PHP, C++). Java, C# and Swift
/// constructors are captured as constructors by their queries and never
/// reach this.
///
/// Membership is established first, from the declaration's place in a type
/// body ([`is_direct_member`], except where an arm says otherwise); only then
/// may a rule read the name (`constructor`). A constructor in a function body
/// is still demoted afterward ([`refine_symbol_kind`]).
fn is_constructor_declaration(
    name: Node<'_>,
    declaration: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> bool {
    match language {
        // A class body is the membership test: it also covers a class
        // expression, which is not a member container. An object literal's
        // `constructor()` is an ordinary method.
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            declaration.kind() == "method_definition"
                && declaration
                    .indexed_parent()
                    .is_some_and(|parent| parent.kind() == "class_body")
                && node_text(name, source) == Some("constructor")
        }
        // A decorated `__init__` keeps its membership through the decorator.
        ProgrammingLanguage::Python => {
            declaration.kind() == "function_definition"
                && is_direct_member(declaration)
                && node_text(name, source) == Some("__init__")
        }
        // PHP method names are case-insensitive; the source spelling is kept.
        ProgrammingLanguage::Php => {
            declaration.kind() == "method_declaration"
                && is_direct_member(declaration)
                && node_text(name, source).is_some_and(|text| text.eq_ignore_ascii_case("__construct"))
        }
        // A class member with no return type whose function declarator
        // declares a plain identifier, looking through attributes and
        // parentheses only (`C [[deprecated]] ()`, `(C)()`): a destructor
        // (`destructor_name`), a conversion operator (`operator_cast`) and an
        // out-of-class `C::C()` (`qualified_identifier`) do not qualify, and
        // the name is never compared with the class's.
        ProgrammingLanguage::Cpp => {
            matches!(declaration.kind(), "declaration" | "field_declaration" | "function_definition")
                && declaration.child_by_field_name("type").is_none()
                && is_direct_member(declaration)
                && name.kind() == "identifier"
                && declared_function_named(declaration, name).is_some_and(|declared| {
                    declared.name == name
                        && declaration
                            .child_by_field_name("declarator")
                            .map(transparent_declarator)
                            == Some(declared.function)
                })
        }
        _ => false,
    }
}

/// Whether a Python assignment is a statement directly in a class body.
fn is_python_class_attribute_statement(assignment: Node<'_>) -> bool {
    assignment
        .indexed_parent()
        .filter(|statement| statement.kind() == "expression_statement")
        .and_then(|statement| statement.indexed_parent())
        .filter(|block| block.kind() == "block")
        .and_then(|block| block.indexed_parent())
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
        current = cursor.indexed_parent();
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
        || declaration.indexed_parent().is_some_and(|parent| {
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
        .indexed_parent()
        .is_some_and(|parent| parent.kind() == "const_declaration")
    {
        return SymbolKind::Constant;
    }

    SymbolKind::Variable
}

fn is_in_type_body(declaration: Node<'_>) -> bool {
    declaration.indexed_parent().is_some_and(|parent| {
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

/// Body nodes between a member and its container. A Java enum's members
/// after its constants sit in `enum_body_declarations` inside `enum_body`.
fn is_member_body_kind(kind: &str) -> bool {
    kind.ends_with("_body")
        || matches!(
            kind,
            "block"
                | "declaration_list"
                | "field_declaration_list"
                | "template_body"
                | "enum_body_declarations"
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
    while let Some(parent) = slot.indexed_parent()
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
    let mut current = member_slot(declaration).indexed_parent();
    while let Some(parent) = current {
        if is_member_container_kind(parent.kind()) {
            return true;
        }
        if !is_member_body_kind(parent.kind()) {
            return false;
        }
        current = parent.indexed_parent();
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
        && name.indexed_parent().is_some_and(|parent| {
            matches!(
                parent.kind(),
                "method_index_expression" | "dot_index_expression"
            )
        })
    {
        return false;
    }

    let mut current = member_slot(declaration).indexed_parent();
    while let Some(node) = current {
        if is_member_owner_kind(node.kind()) {
            return false;
        }
        if is_function_node_kind(node.kind()) {
            return true;
        }
        current = node.indexed_parent();
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
    /// found by name among the file's type declarations. `impl_item` is
    /// the Rust impl the declaration sits in, whose self type decides its
    /// container under Containment.
    SameFileType { impl_item: Option<Node<'tree>> },
    /// A Lua `M.f` / `M:m` / `M.x`: the table, found by name among the file's
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
/// level, before that level's parent, through `scopes` (pass one
/// [`SiblingScopes`] for every declaration of a file).
pub(crate) fn find_symbol_container<'tree>(
    declaration: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
    scopes: &SiblingScopes<'tree>,
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
            lookup: ContainerLookup::SameFileType { impl_item: None },
        });
    }

    let mut child = declaration;
    while let Some(parent) = child.indexed_parent() {
        if !is_sibling_scoped_container(child, language)
            && let Some(container) =
                preceding_sibling_container(child, parent, language, source, scopes)
        {
            return Some(container);
        }

        if language == ProgrammingLanguage::Rust && parent.kind() == "impl_item" {
            return impl_self_type_name(parent, source).map(|name| ContainerRef {
                name,
                lookup: ContainerLookup::SameFileType { impl_item: Some(parent) },
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
/// class expression or object literal, a C# anonymous object, a C/C++ `struct`/`class`/`union`
/// without a tag, and a Go `struct { … }` type that is not a `type_spec`'s
/// own (a field's type, a composite literal, an alias target). A declaration
/// inside one has no container rather than the named type around it.
///
/// A tagless C/C++ `struct`/`union` that declares no name of its own
/// (`union { int a; float b; };`) is an anonymous member: its fields are
/// members of the enclosing type, so it is not a boundary.
pub(crate) fn is_unnamed_type_owner(node: Node<'_>) -> bool {
    match node.kind() {
        "class" | "object" | "anonymous_class" | "anonymous_object_creation_expression" => true,
        "class_body" => node.indexed_parent().is_some_and(|parent| {
            matches!(parent.kind(), "object_creation_expression" | "enum_constant")
        }),
        "template_body" => node
            .indexed_parent()
            .is_some_and(|parent| parent.kind() == "instance_expression"),
        // A Scala `given` is not an extracted symbol, so its instance body
        // names no container.
        "with_template_body" => true,
        "struct_specifier" | "class_specifier" | "union_specifier" => {
            node.child_by_field_name("name").is_none()
                && anonymous_member_declaration(node).is_none()
        }
        "struct_type" => node.indexed_parent().is_none_or(|parent| parent.kind() != "type_spec"),
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
    specifier.indexed_parent().filter(|parent| {
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
        ProgrammingLanguage::Go => return go_type_container(node, source),
        // `module A {}` declares a namespace, as `namespace A {}` does; a
        // string-named ambient `declare module "x" {}` stays a module.
        ProgrammingLanguage::TypeScript
            if node.kind() == "module"
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| name.kind() != "string") =>
        {
            &[("module", SymbolKind::Namespace)]
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
            ("internal_module", SymbolKind::Namespace),
            ("module", SymbolKind::Module),
        ],
    };

    named_container_ref(node, source, kind_map)
}

/// A Go type declared by `node`, a `type_spec`, as the container of what its
/// body declares. The container node is the one the type's own symbol is
/// extracted at ([`context_for_name`]): the `type_declaration` of an
/// ungrouped `type S struct { … }`, the spec itself in a grouped
/// `type ( … )` block.
fn go_type_container<'tree>(node: Node<'tree>, source: &str) -> Option<ContainerRef<'tree>> {
    if node.kind() != "type_spec" {
        return None;
    }
    let name = node
        .child_by_field_name("name")?
        .utf8_text(source.as_bytes())
        .ok()?
        .to_string();
    let owner = node
        .indexed_parent()
        .filter(|declaration| {
            declaration.kind() == "type_declaration" && !is_grouped_go_type_declaration(*declaration)
        })
        .unwrap_or(node);
    Some(ContainerRef {
        name,
        lookup: ContainerLookup::Node {
            node: owner,
            fallback: SymbolKind::Type,
        },
    })
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

/// The sibling-scoped container in force at each child of a parent, by
/// node id, so the preceding siblings of a level are read once per file
/// rather than once per declaration below that level.
#[derive(Default)]
pub(crate) struct SiblingScopes<'tree> {
    /// Parent node id → child node id → the nearest sibling-scoped
    /// container before that child.
    by_parent: RefCell<HashMap<usize, HashMap<usize, Option<Node<'tree>>>>>,
}

impl<'tree> SiblingScopes<'tree> {
    /// The nearest sibling-scoped container before `child` among the
    /// children of `parent`.
    fn before(
        &self,
        child: Node<'tree>,
        parent: Node<'tree>,
        language: ProgrammingLanguage,
    ) -> Option<Node<'tree>> {
        count_shared_context_steps(1);
        let mut by_parent = self.by_parent.borrow_mut();
        let in_force = by_parent.entry(parent.id()).or_insert_with(|| {
            let mut in_force = HashMap::new();
            let mut nearest = None;
            let mut cursor = parent.walk();
            for sibling in parent.children(&mut cursor) {
                in_force.insert(sibling.id(), nearest);
                if is_sibling_scoped_container(sibling, language) {
                    nearest = Some(sibling);
                }
            }
            count_shared_context_steps(in_force.len());
            in_force
        });
        in_force.get(&child.id()).copied().flatten()
    }
}

fn preceding_sibling_container<'tree>(
    node: Node<'tree>,
    parent: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
    scopes: &SiblingScopes<'tree>,
) -> Option<ContainerRef<'tree>> {
    if !matches!(language, ProgrammingLanguage::CSharp | ProgrammingLanguage::Perl) {
        return None;
    }

    let candidate = scopes.before(node, parent, language)?;
    match language {
        ProgrammingLanguage::Perl => perl_package_container(candidate, source),
        _ => named_container_ref(
            candidate,
            source,
            &[("file_scoped_namespace_declaration", SymbolKind::Namespace)],
        ),
    }
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
    let Some(parent) = node.indexed_parent() else {
        return false;
    };

    if is_named_definition_node(parent, node) {
        return true;
    }

    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => match parent.kind() {
            // `(a = xs)`: the default in the `value` field is a read.
            "required_parameter" | "optional_parameter" => parent.child_by_field_name("value") != Some(node),
            "property_signature"
            | "method_signature"
            | "rest_pattern"
            | "shorthand_property_identifier_pattern"
            | "pair_pattern" => true,
            _ => false,
        },
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

    // Name fields only: `value` (Java `int k = xs`, TS `const k = xs`,
    // Rust `const K: i32 = XS`) holds the initializer, an expression read.
    ["name", "alias", "declarator", "parameter", "path"]
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

    // Skip context captures (e.g., function.context, type.context): they mark
    // a name's declaration node and name no symbol of their own.
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

/// A callable's body as syntax nodes ([`callable_body`]).
#[derive(Debug, Clone, Copy)]
pub(crate) enum CallableBody<'tree> {
    /// One node: the `body` field, or an expression body's expression.
    Node(Node<'tree>),
    /// A run of sibling children of the declaration, `first` to `last`
    /// inclusive (Python and Lua).
    Run { first: Node<'tree>, last: Node<'tree> },
    /// An empty Lua body: the empty span at this `end` token.
    EmptyAt(Node<'tree>),
}

impl CallableBody<'_> {
    /// The stored `source.body_span`, trailing whitespace removed.
    pub(crate) fn span(&self, source: &str) -> TextSpan {
        let extent = match *self {
            Self::Node(node) => text_span_for_node(node),
            Self::Run { first, last } => {
                let (first, last) = (text_span_for_node(first), text_span_for_node(last));
                TextSpan { end: last.end, end_byte: last.end_byte, ..first }
            }
            Self::EmptyAt(end) => {
                let end = text_span_for_node(end);
                TextSpan { end: end.start, end_byte: end.start_byte, ..end }
            }
        };
        without_trailing_whitespace(extent, source)
    }
}

/// The body of a function, method or constructor: what `source.body_span`
/// stores and `metrics.lines` and `metrics.cyclomatic` measure. It is the
/// declaration's own `body` field, never a type-body fallback, so a default
/// value, a return type or a nested callable is never taken for it; it is read from the declaration
/// because a C/C++ function's anchor is its `function_declarator`. `None`
/// without an implementation body (a prototype, an abstract or interface
/// method, a defaulted or deleted C++ member).
///
/// An expression body is its expression: C#'s `arrow_expression_clause`
/// holds the `=>`, which is left out (JS/TS arrows and Scala `=` already
/// keep the introducer outside the field). Python and Lua leave comments
/// before (Lua: also after) the body's `block` as children of the function,
/// so there the body runs from the first child after the header to the last
/// one before `end`, a comment on the header's own line excepted; Lua omits
/// the block of an empty body, which is then the empty span at `end`.
///
/// Its span ([`CallableBody::span`]) ends at its last non-whitespace byte:
/// tree-sitter-scala's braceless `indented_block` runs over the blank lines
/// up to the dedent and a Perl `block` over its line break, which are not
/// the body's lines.
pub(crate) fn callable_body(
    declaration: Node<'_>,
    language: ProgrammingLanguage,
) -> Option<CallableBody<'_>> {
    let header_end = match language {
        ProgrammingLanguage::Python => find_child_by_kind(declaration, ":")?,
        ProgrammingLanguage::Lua => declaration.child_by_field_name("parameters")?,
        _ => {
            let body = declaration.child_by_field_name("body")?;
            let expression = (body.kind() == "arrow_expression_clause")
                .then(|| {
                    let mut cursor = body.walk();
                    body.named_children(&mut cursor).find(|child| child.kind() != "comment")
                })
                .flatten();
            return Some(CallableBody::Node(expression.unwrap_or(body)));
        }
    };
    let header_row = header_end.end_position().row;
    let mut cursor = declaration.walk();
    let mut inside = declaration
        .children(&mut cursor)
        .skip_while(|child| child.id() != header_end.id())
        .skip(1)
        .take_while(|child| child.kind() != "end")
        .filter(|child| child.kind() != "comment" || child.start_position().row > header_row);
    match inside.next() {
        Some(first) => Some(CallableBody::Run { first, last: inside.last().unwrap_or(first) }),
        None => find_child_by_kind(declaration, "end").map(CallableBody::EmptyAt),
    }
}

/// `span` ending after its last non-whitespace byte, its end location
/// recomputed (one-based, byte columns as tree-sitter counts them).
fn without_trailing_whitespace(span: TextSpan, source: &str) -> TextSpan {
    let start = span.start_byte as usize;
    let Some(text) = source.get(start..span.end_byte as usize) else {
        return span;
    };
    let kept = text.trim_end_matches(|character: char| character.is_ascii_whitespace());
    if kept.len() == text.len() {
        return span;
    }
    let end_byte = start + kept.len();
    let end = match kept.rfind('\n') {
        Some(newline) => TextPoint {
            line: span.start.line + kept.matches('\n').count() as u32,
            column: (kept.len() - newline) as u32,
        },
        None => TextPoint { line: span.start.line, column: span.start.column + kept.len() as u32 },
    };
    TextSpan { end, end_byte: end_byte as u32, ..span }
}

/// The right-hand side of a type alias declaration as written (TypeScript,
/// Rust, Go and Scala `type A = …`, Swift `typealias A = …`), generic
/// parameters left unresolved. `None` for a declaration with no right-hand
/// side, such as an abstract Scala `type Element`.
pub(crate) fn extract_type_alias_target(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    let target = match (language, node.kind()) {
        (ProgrammingLanguage::Rust, "type_item")
        | (ProgrammingLanguage::Go, "type_alias")
        | (ProgrammingLanguage::Scala, "type_definition") => node.child_by_field_name("type"),
        // An ungrouped `type A = B` is its `type_declaration`
        // ([`context_for_name`]).
        (ProgrammingLanguage::Go, "type_declaration") => {
            find_child_by_kind(node, "type_alias")?.child_by_field_name("type")
        }
        // The alias and its target are both under the `name` field; the
        // target is the first named node after `=` that is not a comment.
        (ProgrammingLanguage::Swift, "typealias_declaration") => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .skip_while(|child| child.kind() != "=")
                .find(|child| child.is_named() && !child.is_extra())
        }
        (_, "type_alias_declaration") => node.child_by_field_name("value").or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "object_type" || child.kind().ends_with("type"))
        }),
        _ => None,
    };
    target
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
    while let Some(parent) = declaration.indexed_parent()
        && matches!(parent.kind(), "variable_declarator" | "lexical_declaration" | "variable_declaration")
    {
        declaration = parent;
    }
    declaration.indexed_parent().is_some_and(|parent| is_export_node(parent.kind()))
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

        current = cursor_node.indexed_parent();
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
        let Some(parent) = cursor.indexed_parent() else {
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
        find_symbol_container(declaration, language, source, &SiblingScopes::default()).map(|container| container.name)
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
        let container = find_symbol_container(block_sub, perl, source, &SiblingScopes::default()).expect("container");
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
        assert_eq!(extract_type_alias_target(meters, ProgrammingLanguage::Rust, source).as_deref(), Some("u32"));
        assert_eq!(extract_type_alias_target(pair, ProgrammingLanguage::Rust, source).as_deref(), Some("(T, T)"));

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

    /// The right-hand side as written, for every alias form the deferred
    /// kinds use, and nothing for a declaration without one.
    #[test]
    fn alias_targets_are_the_written_right_hand_side() {
        use ProgrammingLanguage as L;
        // `(language, file, source, declaration node kind, its text prefix,
        // target)`.
        type Case = (L, &'static str, &'static str, &'static str, &'static str, Option<&'static str>);
        let cases: [Case; 11] = [
            (L::Go, "a.go", "package p\n\ntype Alias = int\n", "type_alias", "Alias", Some("int")),
            (L::Go, "a.go", "package p\n\ntype Alias = int\n", "type_declaration", "type Alias", Some("int")),
            (L::Go, "a.go", "package p\n\ntype (\n\tGrouped = string\n\tDefined int\n)\n", "type_alias", "Grouped", Some("string")),
            (L::Go, "a.go", "package p\n\ntype GenAlias[T any] = []T\n", "type_alias", "GenAlias", Some("[]T")),
            (L::Go, "a.go", "package p\n\ntype Defined int\n", "type_declaration", "type Defined", None),
            (L::Swift, "a.swift", "typealias Alias = Int\n", "typealias_declaration", "typealias Alias", Some("Int")),
            (L::Swift, "a.swift", "public typealias G<T> = Array<T>\n", "typealias_declaration", "public typealias", Some("Array<T>")),
            (L::Scala, "a.scala", "object O {\n  type Alias = Int\n}\n", "type_definition", "type Alias", Some("Int")),
            (L::Scala, "a.scala", "object O {\n  type Gen[A] = List[A]\n}\n", "type_definition", "type Gen", Some("List[A]")),
            (L::Scala, "a.scala", "trait T {\n  type Element\n}\n", "type_definition", "type Element", None),
            // A C typedef's `type` field is only the base type of its declarator.
            (L::C, "a.c", "typedef int *IntPtr;\n", "type_definition", "typedef", None),
        ];
        for (language, file, source, kind, prefix, expected) in cases {
            let tree = parse(language, file, source);
            let node = find_node(tree.root_node(), kind, prefix, source)
                .unwrap_or_else(|| panic!("no {kind} at {prefix:?} in {source:?}"));
            assert_eq!(
                extract_type_alias_target(node, language, source).as_deref(),
                expected,
                "{language:?} {source:?}"
            );
        }
    }

    /// Declaration nodes of the deferred kinds are found from their names; a
    /// bare TypeScript enum member, which has no node of its own, is its own
    /// declaration, and a TypeScript `module` is a declaration rather than a
    /// file root.
    #[test]
    fn deferred_declarations_resolve_from_their_names() {
        use ProgrammingLanguage as L;
        // `(language, file, source, name node kind, name, declaration kind)`.
        type Case = (L, &'static str, &'static str, &'static str, &'static str, &'static str);
        let cases: [Case; 9] = [
            (L::TypeScript, "a.ts", "enum E { Red, Blue = 2 }\n", "property_identifier", "Red", "property_identifier"),
            (L::TypeScript, "a.ts", "enum E { Red, Blue = 2 }\n", "property_identifier", "Blue", "enum_assignment"),
            (L::TypeScript, "a.ts", "namespace Space { const inside = 1; }\n", "identifier", "Space", "internal_module"),
            (L::TypeScript, "a.ts", "module Mod { const inside = 1; }\n", "identifier", "Mod", "module"),
            (L::Go, "a.go", "package p\n\ntype Alias = int\n", "type_identifier", "Alias", "type_alias"),
            (L::Php, "a.php", "<?php\nenum Suit: string { case Hearts = 'h'; }\n", "name", "Hearts", "enum_case"),
            (L::Scala, "a.scala", "enum Color {\n  case Red, Blue\n  case Value(x: Int)\n}\n", "identifier", "Red", "simple_enum_case"),
            (L::Scala, "a.scala", "enum Color {\n  case Red, Blue\n  case Value(x: Int)\n}\n", "identifier", "Value", "full_enum_case"),
            // A parameter of a Scala enum case is still its own declaration.
            (L::Scala, "a.scala", "enum Color {\n  case Red, Blue\n  case Value(x: Int)\n}\n", "identifier", "x", "class_parameter"),
        ];
        for (language, file, source, name_kind, name, declaration_kind) in cases {
            let tree = parse(language, file, source);
            let name_node = find_node(tree.root_node(), name_kind, name, source)
                .unwrap_or_else(|| panic!("no {name_kind} {name:?} in {source:?}"));
            assert_eq!(
                resolve_declaration_node(name_node).kind(),
                declaration_kind,
                "{language:?} `{name}`"
            );
        }
    }

    /// A Go field's container is its type, found at the node the type's own
    /// symbol is extracted at; a field of an anonymous struct type has none.
    #[test]
    fn go_fields_are_contained_by_the_type_that_declares_them() {
        let source = "package p\n\ntype S struct {\n\tX, Y int\n\tanon struct{ Inner int }\n}\n\ntype (\n\tA struct{ P int }\n\tB struct{ Q int }\n)\n\ntype Al = struct{ R int }\n\nfunc f() {\n\tv := struct{ L int }{1}\n\t_ = v\n}\n";
        let tree = parse(ProgrammingLanguage::Go, "a.go", source);
        let root = tree.root_node();
        let field = |prefix: &str| {
            find_node(root, "field_declaration", prefix, source)
                .unwrap_or_else(|| panic!("no field {prefix:?}"))
        };
        let container = |declaration: Node<'_>| {
            find_symbol_container(declaration, ProgrammingLanguage::Go, source, &SiblingScopes::default()).map(|container| {
                let ContainerLookup::Node { node, fallback } = container.lookup else {
                    panic!("Go type containers are nodes");
                };
                assert_eq!(fallback, SymbolKind::Type);
                (container.name, node.kind(), source[node.byte_range()].to_string())
            })
        };

        let (name, kind, text) = container(field("X, Y")).expect("S");
        assert_eq!((name.as_str(), kind), ("S", "type_declaration"));
        assert!(text.starts_with("type S struct"));
        assert_eq!(container(field("anon")).map(|found| found.0).as_deref(), Some("S"));

        // In a group, each type is its own spec.
        assert_eq!(
            container(field("P int")).map(|(name, kind, _)| (name, kind)),
            Some(("A".to_string(), "type_spec"))
        );
        assert_eq!(
            container(field("Q int")).map(|(name, kind, _)| (name, kind)),
            Some(("B".to_string(), "type_spec"))
        );

        for anonymous in ["Inner int", "R int", "L int"] {
            assert!(container(field(anonymous)).is_none(), "{anonymous}");
        }

        // The type's own symbol is extracted at that same node.
        let grouped = find_node(root, "type_declaration", "type (", source).expect("group");
        let b_name = find_node(root, "type_identifier", "B", source).expect("B");
        assert_eq!(context_for_name(grouped, b_name, ProgrammingLanguage::Go).kind(), "type_spec");
        let single = find_node(root, "type_declaration", "type S", source).expect("S");
        let s_name = find_node(root, "type_identifier", "S", source).expect("S name");
        assert_eq!(context_for_name(single, s_name, ProgrammingLanguage::Go), single);
    }

    /// A TypeScript namespace contains its members, nested namespaces
    /// included, and is never its own container. A string-named ambient
    /// module stays a module.
    #[test]
    fn typescript_namespaces_contain_their_members() {
        let source = "namespace Space {\n  function inside() {}\n  namespace Inner {\n    function deep() {}\n  }\n}\nmodule Mod {\n  function inMod() {}\n}\ndeclare module \"x\" {\n  function ambient(): void;\n}\n";
        let tree = parse(ProgrammingLanguage::TypeScript, "a.ts", source);
        let root = tree.root_node();
        let container = |kind: &str, prefix: &str| {
            let node = find_node(root, kind, prefix, source).unwrap_or_else(|| panic!("no {prefix:?}"));
            find_symbol_container(node, ProgrammingLanguage::TypeScript, source, &SiblingScopes::default()).map(|container| {
                let ContainerLookup::Node { fallback, .. } = container.lookup else {
                    panic!("namespace containers are nodes");
                };
                (container.name, fallback)
            })
        };
        let namespace = |name: &str| Some((name.to_string(), SymbolKind::Namespace));

        assert_eq!(container("function_declaration", "function inside"), namespace("Space"));
        assert_eq!(container("internal_module", "namespace Inner"), namespace("Space"));
        assert_eq!(container("function_declaration", "function deep"), namespace("Inner"));
        assert_eq!(container("function_declaration", "function inMod"), namespace("Mod"));
        assert_eq!(container("internal_module", "namespace Space"), None);
        assert_eq!(container("module", "module Mod"), None);
        assert_eq!(
            container("function_signature", "function ambient"),
            Some(("\"x\"".to_string(), SymbolKind::Module))
        );
    }

    /// A namespace record qualifies its members once: adding it leaves every
    /// other name as it was without it.
    #[test]
    fn namespace_records_qualify_their_members_once() {
        use ProgrammingLanguage as L;
        // `(language, file, source, (namespace node kind, prefix), member
        // (node kind, prefix, name), expected names)`.
        type Case = (L, &'static str, &'static str, (&'static str, &'static str), (&'static str, &'static str, &'static str), [&'static str; 2]);
        let cases: [Case; 4] = [
            (L::TypeScript, "a.ts", "namespace Space {\n  function inside() {}\n}\n", ("internal_module", "namespace Space"), ("function_declaration", "function inside", "inside"), ["Space", "Space.inside"]),
            (L::TypeScript, "a.ts", "module Mod {\n  function inside() {}\n}\n", ("module", "module Mod"), ("function_declaration", "function inside", "inside"), ["Mod", "Mod.inside"]),
            (L::Php, "a.php", "<?php\nnamespace App;\nfunction inside() {}\n", ("namespace_definition", "namespace App"), ("function_definition", "function inside", "inside"), ["App", "App\\inside"]),
            (L::Php, "a.php", "<?php\nnamespace App\\Braced {\n  function inside() {}\n}\n", ("namespace_definition", "namespace App"), ("function_definition", "function inside", "inside"), ["App\\Braced", "App\\Braced\\inside"]),
        ];
        for (language, file, source, (namespace_kind, namespace_prefix), (member_kind, member_prefix, member_name), expected) in cases {
            let tree = parse(language, file, source);
            let root = tree.root_node();
            let namespace = find_node(root, namespace_kind, namespace_prefix, source).expect("namespace");
            let namespace_name = namespace.child_by_field_name("name").expect("namespace name");
            let member = find_node(root, member_kind, member_prefix, source).expect("member");
            let member_input = NamingInput {
                name: member_name,
                kind: SymbolKind::Function,
                name_node: member.child_by_field_name("name").expect("member name"),
                declaration: member,
                anchor: member,
            };
            let namespace_input = NamingInput {
                name: &source[namespace_name.byte_range()],
                kind: SymbolKind::Namespace,
                name_node: namespace_name,
                declaration: namespace,
                anchor: namespace,
            };

            let without = qualified_names(std::slice::from_ref(&member_input), language, source);
            let with = qualified_names(&[namespace_input, member_input], language, source);
            let names: Vec<&str> = with.iter().map(|name| name.text.as_str()).collect();
            assert_eq!(names, expected, "{source:?}");
            assert_eq!(with[1], without[0], "{source:?}: the member's name is unchanged");
        }
    }

    /// The constructor hook keeps the JavaScript/TypeScript rule: a class
    /// body's `constructor` (also a class expression's) is a constructor, an
    /// object literal's is a method.
    #[test]
    fn ecma_constructors_are_class_body_members_named_constructor() {
        let source = "class C { constructor() {} other() {} }\nconst K = class { constructor() {} };\nconst o = { constructor() {} };\nfunction f() { class Local { constructor() {} } }\n";
        let tree = parse(ProgrammingLanguage::TypeScript, "a.ts", source);
        let root = tree.root_node();
        let mut cursor = root.walk();
        let mut methods = Vec::new();
        collect_kind(root, "method_definition", &mut cursor, &mut methods);
        let kinds: Vec<(String, SymbolKind)> = methods
            .into_iter()
            .map(|method| {
                let name = method.child_by_field_name("name").expect("name");
                // A class's methods sit in its `class_body`; an object
                // literal's directly in the `object`.
                let owner = method
                    .indexed_parent()
                    .and_then(|parent| if parent.kind() == "class_body" { parent.indexed_parent() } else { Some(parent) })
                    .map(|owner| owner.kind())
                    .unwrap_or_default();
                (
                    format!("{owner}.{}", &source[name.byte_range()]),
                    refine_symbol_kind(SymbolKind::Method, name, method, ProgrammingLanguage::TypeScript, source),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            [
                ("class_declaration.constructor".to_string(), SymbolKind::Constructor),
                ("class_declaration.other".to_string(), SymbolKind::Method),
                ("class.constructor".to_string(), SymbolKind::Constructor),
                ("object.constructor".to_string(), SymbolKind::Method),
                ("class_declaration.constructor".to_string(), SymbolKind::Constructor),
            ]
        );
    }

    /// A Java enum's members after its constants are direct members.
    #[test]
    fn java_enum_body_declarations_hold_direct_members() {
        let source = "enum E { A; E() {} void m() {} }\n";
        let tree = parse(ProgrammingLanguage::Java, "E.java", source);
        let root = tree.root_node();
        for (kind, prefix) in [("constructor_declaration", "E()"), ("method_declaration", "void m")] {
            let declaration = find_node(root, kind, prefix, source).expect("member");
            assert!(is_direct_member(declaration), "{prefix}");
        }
    }

    fn collect_kind<'tree>(
        node: Node<'tree>,
        kind: &str,
        cursor: &mut tree_sitter::TreeCursor<'tree>,
        found: &mut Vec<Node<'tree>>,
    ) {
        if node.kind() == kind {
            found.push(node);
        }
        let children: Vec<Node<'tree>> = node.children(cursor).collect();
        for child in children {
            collect_kind(child, kind, cursor, found);
        }
    }
}
