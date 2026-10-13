//! C visibility, and the file-scope linkage rule C++ shares.

use super::*;
use crate::file::parents::IndexedParent as _;

/// C visibility. Only file-scope functions and objects have an access level,
/// decided by [`linkage_visibility`]; types, tags, members, enumerators and
/// macros have no access modifier and report Unknown.
pub(super) fn extract_c_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    (has_linkage(ctx.kind) && is_namespace_scope(declaration))
        .then(|| linkage_visibility(declaration, ctx, false))
}

/// Functions and objects have linkage; nothing else does.
pub(super) fn has_linkage(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function | SymbolKind::Variable | SymbolKind::Constant
    )
}

/// Whether `declaration` sits at file scope, or at namespace scope in C++,
/// looking through `extern "C"` blocks, preprocessor conditionals and
/// template headers.
pub(super) fn is_namespace_scope(declaration: Node<'_>) -> bool {
    let mut current = declaration.indexed_parent();
    while let Some(node) = current {
        match node.kind() {
            "translation_unit" => return true,
            "declaration_list" => {
                if !node.indexed_parent().is_some_and(|parent| {
                    matches!(parent.kind(), "namespace_definition" | "linkage_specification")
                }) {
                    return false;
                }
            }
            "namespace_definition"
            | "linkage_specification"
            | "template_declaration"
            | "preproc_if"
            | "preproc_ifdef"
            | "preproc_else"
            | "preproc_elif"
            | "preproc_elifdef" => {}
            _ => return false,
        }
        current = node.indexed_parent();
    }
    false
}

/// The linkage rule for a file-scope (C) or namespace-scope (C++) function or
/// object: ModulePrivate when it is `static`, sits in an unnamed namespace
/// (C++), is a C++ object that is itself `const` (including `constexpr`)
/// without `extern` or `inline`, or is a function an earlier same-file
/// declaration made `static`; Public otherwise.
pub(super) fn linkage_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
    cpp: bool,
) -> Visibility {
    let source = ctx.source;
    let is_function = ctx.kind.is_function();
    let module_private = has_specifier(declaration, "storage_class_specifier", "static", source)
        || (cpp && in_unnamed_namespace(declaration))
        || (cpp
            && !is_function
            && is_const_object(declaration, ctx.name, source)
            && !has_specifier(declaration, "storage_class_specifier", "extern", source)
            && !has_specifier(declaration, "storage_class_specifier", "inline", source))
        || (is_function && has_earlier_static_declaration(declaration, ctx, cpp));

    if module_private {
        Visibility::ModulePrivate
    } else {
        Visibility::Public
    }
}

fn has_specifier(node: Node<'_>, kind: &str, keyword: &str, source: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        child.kind() == kind && child.utf8_text(source.as_bytes()).ok() == Some(keyword)
    })
}

fn in_unnamed_namespace(declaration: Node<'_>) -> bool {
    let mut current = declaration.indexed_parent();
    while let Some(node) = current {
        if node.kind() == "namespace_definition" && node.child_by_field_name("name").is_none() {
            return true;
        }
        current = node.indexed_parent();
    }
    false
}

/// Whether the object `name` declares is itself `const`. Its own type is the
/// declarator layer nearest the name, walking through parentheses (see
/// [`declarator_layers`]): `char **const p` and `int (*const p)(int)` are
/// const pointers, `char *const *p` and `const int (*p)[2]` are mutable
/// pointers. An array takes its element's constness (`char **const p[2]`).
/// With no pointer or array layer the declaration's own `const` decides.
/// A reference is never a const object, even `constexpr`; any other
/// `constexpr` object is const.
fn is_const_object(declaration: Node<'_>, name: Node<'_>, source: &str) -> bool {
    let Some(mut declarator) = child_on_path(declaration, name) else {
        return false;
    };
    if declarator.kind() == "init_declarator"
        && let Some(inner) = declarator.child_by_field_name("declarator")
    {
        declarator = inner;
    }
    let layers = declarator_layers(Some(declarator), Some(name), source);
    let own_type = layers.iter().find(|layer| !matches!(layer, Declarator::Array(_)));
    if matches!(own_type, Some(Declarator::Reference(_))) {
        return false;
    }
    if has_specifier(declaration, "type_qualifier", "constexpr", source) {
        return true;
    }
    match own_type {
        None => has_specifier(declaration, "type_qualifier", "const", source),
        Some(Declarator::Pointer(qualifiers)) => qualifiers.contains(&"const"),
        Some(_) => false,
    }
}

/// The child of `ancestor` on the path down to `descendant`.
fn child_on_path<'tree>(ancestor: Node<'tree>, descendant: Node<'tree>) -> Option<Node<'tree>> {
    let mut node = descendant;
    while let Some(parent) = node.indexed_parent() {
        if parent == ancestor {
            return Some(node);
        }
        node = parent;
    }
    None
}

/// Whether an earlier declaration of the same function in this file is
/// `static`. In C the name alone identifies the function; in C++ the
/// namespace, the parameter types and, for a function template, the rest of
/// its signature must match as well (see [`FunctionIdentity`] and
/// [`TemplateSignature`]). Every
/// declarator of both declarations counts: `static int a(int), b(int);`
/// covers `b` too.
///
/// Each condition is one a redeclaration has to meet, so whatever the scan
/// cannot establish (a typedef'd spelling, a qualifier naming no namespace in
/// this file) makes it miss a `static`, never attach one to another function.
fn has_earlier_static_declaration(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
    cpp: bool,
) -> bool {
    let source = ctx.source;
    let Some(own_function) = declared_function_named(declaration, ctx.name) else {
        return false;
    };
    let index = ctx
        .memo
        .c_linkage
        .get_or_init(|| LinkageIndex::of(ctx.root, source, cpp));
    let Some(own) =
        FunctionIdentity::of(declaration, &own_function, source, &index.namespaces)
    else {
        return false;
    };
    count_shared_context_steps(1);
    index
        .static_functions
        .get(&own.linkage_key(cpp))
        .is_some_and(|earliest| *earliest < declaration.start_byte())
}

/// What the earlier-`static` rule reads from the whole file, collected once
/// per file into [`VisibilityMemo`]: the namespaces it declares (C++ only)
/// and the identity of every function a namespace-scope `static`
/// declaration declares. An earlier declaration's identity depends only on
/// what precedes it, never on the declaration it is compared with, so each
/// identity keeps only its earliest `static` declaration.
pub(super) struct LinkageIndex<'src> {
    /// Namespace path → the start of the earliest block declaring it.
    namespaces: Namespaces<'src>,
    /// Function identity ([`FunctionIdentity::linkage_key`]) → the start byte
    /// of its earliest `static` declaration.
    static_functions: HashMap<FunctionIdentity<'src>, usize>,
}

type Namespaces<'src> = HashMap<Vec<&'src str>, usize>;

impl<'src> LinkageIndex<'src> {
    fn of(root: Node<'_>, source: &'src str, cpp: bool) -> Self {
        let namespaces = if cpp { declared_namespaces(root, source) } else { HashMap::new() };
        let mut declarations = Vec::new();
        collect_scope_declarations(root, &mut declarations);
        count_shared_context_steps(declarations.len() + namespaces.len());
        let mut static_functions: HashMap<FunctionIdentity<'src>, usize> = HashMap::new();
        for candidate in declarations {
            if !has_specifier(candidate, "storage_class_specifier", "static", source) {
                continue;
            }
            for function in declared_functions(candidate) {
                if let Some(identity) =
                    FunctionIdentity::of(candidate, &function, source, &namespaces)
                {
                    // Declarations arrive in source order, so the first is the earliest.
                    static_functions
                        .entry(identity.linkage_key(cpp))
                        .or_insert(candidate.start_byte());
                }
            }
        }
        Self { namespaces, static_functions }
    }
}

/// Namespace-scope declarations and definitions, in source order, including
/// those under a template header.
fn collect_scope_declarations<'tree>(node: Node<'tree>, found: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "declaration" | "function_definition" => found.push(child),
            "namespace_definition" | "linkage_specification" | "declaration_list"
            | "template_declaration" | "preproc_if" | "preproc_ifdef" | "preproc_else"
            | "preproc_elif" | "preproc_elifdef" => collect_scope_declarations(child, found),
            _ => {}
        }
    }
}

/// What makes two C++ declarations name the same function, read from the
/// syntax tree so that spelling which changes neither the namespace nor the
/// parameter types is ignored: parameter names at any nesting level,
/// comments, default arguments, parentheses around a name, and
/// `namespace a::b {}` versus `namespace a { namespace b {} }`. Type names
/// are compared as written, so a typedef and its target, or `long` and
/// `long int`, differ.
#[derive(PartialEq, Eq, Hash)]
struct FunctionIdentity<'src> {
    name: &'src str,
    /// The one namespace the declared name belongs to, as flat segments.
    namespace: Vec<&'src str>,
    parameters: Vec<ParameterType<'src>>,
    /// Present only for a function template, whose signature goes beyond
    /// its parameters: a template and a non-template function with the same
    /// parameters are different functions.
    template: Option<TemplateSignature<'src>>,
}

/// The parts of a function template's signature besides its name,
/// namespace and parameter types ([temp.over.link]): the template header,
/// the return type in both its leading and trailing spelling, and the
/// `requires` clauses after the header and after the parameter list. Each
/// is compared as written, so `requires A<T>` placed after the header and
/// after the parameters differ, as do `auto f() -> int` and `int f()`; the
/// exception specification is not part of it.
#[derive(Debug, PartialEq, Eq, Hash)]
struct TemplateSignature<'src> {
    /// The `template<…>` parameter tokens, or `None` for an abbreviated
    /// template (one with an `auto` parameter and no header).
    header: Option<Vec<&'src str>>,
    header_constraint: Option<Vec<&'src str>>,
    /// The specifiers and the declarators wrapped around the function.
    return_type: ParameterType<'src>,
    trailing_return: Option<ParameterType<'src>>,
    trailing_constraint: Option<Vec<&'src str>>,
}

impl<'src> FunctionIdentity<'src> {
    /// The identity of `function`, or `None` when its namespace cannot be
    /// established from this file. An unqualified name belongs to exactly its
    /// enclosing namespace. A qualified `b::f` inside `namespace a` belongs to
    /// `a::b` when this file declares `a::b` before it, else to `b` when this
    /// file declares `b`, as the compiler looks the first qualifier up from
    /// the nearest enclosing namespace outward.
    fn of(
        declaration: Node<'_>,
        function: &DeclaredFunction<'_>,
        source: &'src str,
        namespaces: &Namespaces<'src>,
    ) -> Option<Self> {
        let mut scope = Vec::new();
        let mut absolute = false;
        let mut name = function.name;
        while name.kind() == "qualified_identifier" {
            match name.child_by_field_name("scope") {
                Some(segment) => scope.push(node_text(segment, source)),
                None => absolute = true,
            }
            name = name.child_by_field_name("name")?;
        }

        let enclosing = enclosing_namespaces(declaration, source);
        let namespace = match scope.first() {
            _ if absolute => scope,
            None => enclosing,
            Some(first) => (0..=enclosing.len()).rev().find_map(|depth| {
                let looked_up = [&enclosing[..depth], &[*first]].concat();
                count_shared_context_steps(1);
                namespaces
                    .get(&looked_up)
                    .is_some_and(|start| *start < declaration.start_byte())
                    .then(|| [&enclosing[..depth], scope.as_slice()].concat())
            })?,
        };
        let parameters = function
            .function
            .child_by_field_name("parameters")
            .map(|parameters| parameter_types(parameters, source))
            .unwrap_or_default();
        let template = TemplateSignature::of(declaration, function.function, source);
        Some(Self { name: node_text(name, source), namespace, parameters, template })
    }

    /// The part of this identity that decides whether two declarations
    /// declare the same function: all of it in C++, the name alone in C.
    fn linkage_key(self, cpp: bool) -> Self {
        if cpp {
            self
        } else {
            Self { name: self.name, namespace: Vec::new(), parameters: Vec::new(), template: None }
        }
    }
}

impl<'src> TemplateSignature<'src> {
    /// The template signature of `function` in `declaration`, or `None`
    /// when it is not a function template: it neither sits under a
    /// `template<…>` header nor has a parameter whose type is a placeholder
    /// (`auto`, `Concept auto`), which makes it an abbreviated template.
    fn of(declaration: Node<'_>, function: Node<'_>, source: &'src str) -> Option<Self> {
        let header = declaration
            .indexed_parent()
            .filter(|parent| parent.kind() == "template_declaration");
        if header.is_none() && !has_placeholder_parameter(function) {
            return None;
        }
        let header_tokens = header.map(|header| {
            let mut tokens = Vec::new();
            if let Some(parameters) = header.child_by_field_name("parameters") {
                leaf_tokens(parameters, source, &mut tokens);
            }
            tokens
        });
        let trailing_return = child_of_kind(function, "trailing_return_type")
            .and_then(first_named_child)
            .map(|descriptor| parameter_type(descriptor, source));
        Some(Self {
            header: header_tokens,
            header_constraint: header.and_then(|header| constraint_tokens(header, source)),
            return_type: return_type(declaration, function, source),
            trailing_return,
            trailing_constraint: constraint_tokens(function, source),
        })
    }
}

/// Whether one of `function`'s own parameters has a placeholder type.
fn has_placeholder_parameter(function: Node<'_>) -> bool {
    let Some(parameters) = function.child_by_field_name("parameters") else {
        return false;
    };
    let mut cursor = parameters.walk();
    parameters.named_children(&mut cursor).any(|parameter| {
        parameter
            .child_by_field_name("type")
            .is_some_and(|parameter_type| parameter_type.kind() == "placeholder_type_specifier")
    })
}

/// The tokens of `node`'s own `requires` clause, `requires` included.
fn constraint_tokens<'src>(node: Node<'_>, source: &'src str) -> Option<Vec<&'src str>> {
    child_of_kind(node, "requires_clause").map(|clause| {
        let mut tokens = Vec::new();
        leaf_tokens(clause, source, &mut tokens);
        tokens
    })
}

fn child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| child.kind() == kind)
}

/// The leading return type of `function` in `declaration`: the `const`/`volatile`
/// qualifiers and type among the declaration's specifiers, and the
/// declarators wrapped around `function` (`int (*f(int))(long)` returns a
/// pointer to a function taking `long`).
fn return_type<'src>(declaration: Node<'_>, function: Node<'_>, source: &'src str) -> ParameterType<'src> {
    let mut qualifiers = Vec::new();
    let mut cursor = declaration.walk();
    for child in declaration.children(&mut cursor) {
        if child.kind() == "type_qualifier"
            && let text @ ("const" | "volatile") = node_text(child, source)
        {
            qualifiers.push(text);
        }
    }
    qualifiers.sort_unstable();
    let mut specifiers = Vec::new();
    if let Some(type_node) = declaration.child_by_field_name("type") {
        leaf_tokens(type_node, source, &mut specifiers);
    }
    let declarators = declarator_layers(child_on_path(declaration, function), Some(function), source);
    ParameterType { qualifiers, specifiers, declarators }
}

/// The namespace segments enclosing `declaration`, outermost first. An
/// unnamed namespace contributes an empty segment.
fn enclosing_namespaces<'src>(declaration: Node<'_>, source: &'src str) -> Vec<&'src str> {
    let mut blocks = Vec::new();
    let mut current = declaration.indexed_parent();
    while let Some(node) = current {
        if node.kind() == "namespace_definition" {
            blocks.push(own_namespace_segments(node, source));
        }
        current = node.indexed_parent();
    }
    blocks.into_iter().rev().flatten().collect()
}

fn own_namespace_segments<'src>(namespace: Node<'_>, source: &'src str) -> Vec<&'src str> {
    let mut segments = Vec::new();
    match namespace.child_by_field_name("name") {
        Some(name) => namespace_segments(name, source, &mut segments),
        None => segments.push(""),
    }
    segments
}

/// Every namespace the file declares, with the start of the earliest
/// declaring `namespace` block: `namespace a::b {}` declares both `a` and
/// `a::b`.
fn declared_namespaces<'src>(root: Node<'_>, source: &'src str) -> Namespaces<'src> {
    let mut found: Namespaces<'src> = HashMap::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "namespace_definition" => {
                    let outer = enclosing_namespaces(child, source);
                    let own = own_namespace_segments(child, source);
                    for depth in 1..=own.len() {
                        let start = child.start_byte();
                        found
                            .entry([outer.as_slice(), &own[..depth]].concat())
                            .and_modify(|earliest| *earliest = (*earliest).min(start))
                            .or_insert(start);
                    }
                    pending.push(child);
                }
                "linkage_specification" | "declaration_list" | "template_declaration"
                | "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif"
                | "preproc_elifdef" => pending.push(child),
                _ => {}
            }
        }
    }
    found
}

/// The identifiers of a namespace name, so `a::inline b` yields `a`, `b`.
fn namespace_segments<'src>(name: Node<'_>, source: &'src str, segments: &mut Vec<&'src str>) {
    if name.kind() == "namespace_identifier" {
        segments.push(node_text(name, source));
        return;
    }
    let mut cursor = name.walk();
    for child in name.named_children(&mut cursor) {
        namespace_segments(child, source, segments);
    }
}

/// One parameter's type as C++ compares it.
#[derive(Debug, PartialEq, Eq, Hash)]
struct ParameterType<'src> {
    /// `const`/`volatile` written among the specifiers, in sorted order so
    /// `const int` and `int const` agree.
    qualifiers: Vec<&'src str>,
    /// The other specifier tokens: the type name, `struct`, and so on.
    specifiers: Vec<&'src str>,
    /// Type constructors from the one nearest the (possibly absent) name
    /// outward, so the first is the parameter's top-level type.
    declarators: Vec<Declarator<'src>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Declarator<'src> {
    Pointer(Vec<&'src str>),
    Reference(&'src str),
    Array(Vec<&'src str>),
    /// Parameter types and the tokens after them (`noexcept`, `const`, …).
    Function(Vec<ParameterType<'src>>, Vec<&'src str>),
    Pack,
    /// A declarator form not modeled above, compared by kind and tokens.
    Other(&'static str, Vec<&'src str>),
}

/// The parameter types of a `parameter_list`, adjusted as C++ adjusts them
/// before comparing functions: a top-level `const`/`volatile` is dropped, an
/// array becomes a pointer, a function becomes a pointer to it, and `(void)`
/// is an empty list. A C-style `...` tail counts as a parameter.
fn parameter_types<'src>(parameters: Node<'_>, source: &'src str) -> Vec<ParameterType<'src>> {
    let mut cursor = parameters.walk();
    let types: Vec<_> = parameters
        .children(&mut cursor)
        .filter_map(|child| match child.kind() {
            "(" | ")" | "," | "comment" => None,
            "..." => Some(ParameterType {
                qualifiers: Vec::new(),
                specifiers: vec!["..."],
                declarators: Vec::new(),
            }),
            _ => Some(parameter_type(child, source)),
        })
        .collect();
    let declares_nothing = matches!(
        types.as_slice(),
        [only] if only.qualifiers.is_empty() && only.declarators.is_empty() && only.specifiers == ["void"]
    );
    if declares_nothing {
        return Vec::new();
    }
    types.into_iter().map(ParameterType::adjusted).collect()
}

fn parameter_type<'src>(parameter: Node<'_>, source: &'src str) -> ParameterType<'src> {
    let declarator = parameter.child_by_field_name("declarator");
    let default_value = parameter.child_by_field_name("default_value");
    let mut qualifiers = Vec::new();
    let mut specifiers = Vec::new();
    let mut cursor = parameter.walk();
    for child in parameter.children(&mut cursor) {
        if Some(child) == declarator || Some(child) == default_value {
            continue;
        }
        match child.kind() {
            "comment" | "=" => {}
            "type_qualifier" => qualifiers.push(node_text(child, source)),
            _ => leaf_tokens(child, source, &mut specifiers),
        }
    }
    qualifiers.sort_unstable();

    let declarators = declarator_layers(declarator, None, source);

    ParameterType { qualifiers, specifiers, declarators }
}

/// The type constructors from `start` down to the declared name (a plain or
/// qualified identifier, or `stop`), nearest the name first, so the first is
/// the top-level type. Parentheses and attributes add no layer.
fn declarator_layers<'src>(
    start: Option<Node<'_>>,
    stop: Option<Node<'_>>,
    source: &'src str,
) -> Vec<Declarator<'src>> {
    let mut declarators = Vec::new();
    let mut next = start;
    while let Some(node) = next {
        next = None;
        if Some(node) == stop {
            break;
        }
        match node.kind() {
            // The declared name: nothing of the type is left inside it.
            "identifier" | "qualified_identifier" => {}
            "parenthesized_declarator" | "abstract_parenthesized_declarator"
            | "attributed_declarator" => {
                next = first_named_child(node);
            }
            "pointer_declarator" | "abstract_pointer_declarator" => {
                next = node.child_by_field_name("declarator");
                let mut pointer_qualifiers = tokens_except(node, next, source);
                pointer_qualifiers.retain(|token| *token != "*");
                pointer_qualifiers.sort_unstable();
                declarators.push(Declarator::Pointer(pointer_qualifiers));
            }
            "reference_declarator" | "abstract_reference_declarator" => {
                next = first_named_child(node);
                let operator = node.child(0).map_or("&", |token| node_text(token, source));
                declarators.push(Declarator::Reference(operator));
            }
            "array_declarator" | "abstract_array_declarator" => {
                next = node.child_by_field_name("declarator");
                declarators.push(Declarator::Array(tokens_except(node, next, source)));
            }
            "function_declarator" | "abstract_function_declarator" => {
                next = node.child_by_field_name("declarator");
                let parameters = node.child_by_field_name("parameters");
                let mut trailing = Vec::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if Some(child) != next && Some(child) != parameters {
                        leaf_tokens(child, source, &mut trailing);
                    }
                }
                let parameters = parameters
                    .map(|parameters| parameter_types(parameters, source))
                    .unwrap_or_default();
                declarators.push(Declarator::Function(parameters, trailing));
            }
            "variadic_declarator" => {
                next = first_named_child(node);
                declarators.push(Declarator::Pack);
            }
            kind => {
                let mut tokens = Vec::new();
                leaf_tokens(node, source, &mut tokens);
                declarators.push(Declarator::Other(kind, tokens));
            }
        }
    }
    declarators.reverse();
    declarators
}

impl ParameterType<'_> {
    fn adjusted(mut self) -> Self {
        let is_cv = |token: &&str| matches!(*token, "const" | "volatile");
        match self.declarators.first_mut() {
            None => self.qualifiers.retain(|token| !is_cv(token)),
            Some(Declarator::Pointer(qualifiers)) => qualifiers.retain(|token| !is_cv(token)),
            Some(top @ Declarator::Array(_)) => *top = Declarator::Pointer(Vec::new()),
            Some(Declarator::Function(..)) => self.declarators.insert(0, Declarator::Pointer(Vec::new())),
            Some(_) => {}
        }
        self
    }
}

fn first_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| child.kind() != "comment")
}

/// The tokens of `node` outside its child `skip`, without comments.
fn tokens_except<'src>(node: Node<'_>, skip: Option<Node<'_>>, source: &'src str) -> Vec<&'src str> {
    let mut tokens = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if Some(child) != skip {
            leaf_tokens(child, source, &mut tokens);
        }
    }
    tokens
}

/// Every leaf token under `node` in source order, skipping comments.
fn leaf_tokens<'src>(node: Node<'_>, source: &'src str, tokens: &mut Vec<&'src str>) {
    if node.kind() == "comment" {
        return;
    }
    if node.child_count() == 0 {
        let text = node_text(node, source);
        if !text.is_empty() {
            tokens.push(text);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        leaf_tokens(child, source, tokens);
    }
}

fn node_text<'src>(node: Node<'_>, source: &'src str) -> &'src str {
    source.get(node.byte_range()).unwrap_or_default()
}
