//! Structured generic parameters (`type_info.generic_params`) and
//! declaration-level requirements (`type_info.constraints`), read from the
//! parameter list a declaration writes itself.
//!
//! Only the declaration's own list is read: a method never receives its
//! type's parameters and an implemented type never receives its `impl`
//! block's. Bound, default and requirement texts are the grammar nodes'
//! source text with outer whitespace trimmed; a bound list is split at its
//! grammar entries, never at a comma or `&` inside a nested type. The older
//! name lists in `type_metadata.rs` are separate readers with their own
//! (narrower) coverage and are not projected from this one.

use crate::shared::schema_v2::{GenericParam, GenericParamKind};

use super::*;

/// A declaration's written generic parameters and the requirements whose
/// subject is not one declared parameter (Rust `where T::Item: Copy`, Swift
/// `where U == Int`, a C++ `requires` clause).
#[derive(Debug, Default)]
pub(crate) struct DeclaredGenerics {
    pub(crate) params: Vec<GenericParam>,
    pub(crate) constraints: Vec<String>,
}

/// A parameter list longer than `GenericParam::position` can number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenericParamOverflow {
    pub(crate) count: usize,
}

/// One parameter as read, before its position is checked against `u16`.
struct Param {
    name: String,
    kind: GenericParamKind,
    constraints: Vec<String>,
    default: Option<String>,
    variance: Option<String>,
    /// Index in the written list; unnamed C++ parameters keep their slot.
    index: usize,
}

impl Param {
    fn new(name: &str, kind: GenericParamKind, index: usize) -> Self {
        Self {
            name: name.to_string(),
            kind,
            constraints: Vec::new(),
            default: None,
            variance: None,
            index,
        }
    }
}

/// The generic parameters `declaration` declares for the symbol named by
/// `name` (the captured name node: a Go `type` group declares several
/// specs).
///
/// ## Errors
///
/// [`GenericParamOverflow`] when a parameter's position does not fit in
/// `u16`; positions are never truncated.
pub(crate) fn declared_generics(
    declaration: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Result<DeclaredGenerics, GenericParamOverflow> {
    let mut constraints = Vec::new();
    let params = match language {
        ProgrammingLanguage::Rust => rust_generics(declaration, source, &mut constraints),
        ProgrammingLanguage::TypeScript => typescript_generics(declaration, source),
        ProgrammingLanguage::Go => go_generics(declaration, name, source),
        ProgrammingLanguage::Python => python_generics(declaration, source),
        ProgrammingLanguage::Java => java_generics(declaration, source),
        ProgrammingLanguage::CSharp => csharp_generics(declaration, source, &mut constraints),
        ProgrammingLanguage::Cpp => cpp_generics(declaration, name, source, &mut constraints),
        ProgrammingLanguage::Swift => swift_generics(declaration, source, &mut constraints),
        ProgrammingLanguage::Scala => scala_generics(declaration, source),
        // No declared generic parameters: JavaScript, C, PHP, Perl, Bash,
        // Zsh and Lua.
        _ => Vec::new(),
    };
    let params = params
        .into_iter()
        .map(|param| {
            let position = u16::try_from(param.index).map_err(|_| GenericParamOverflow {
                count: param.index + 1,
            })?;
            Ok(GenericParam {
                name: param.name,
                kind: param.kind,
                constraints: param.constraints,
                default: param.default,
                variance: param.variance,
                position,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DeclaredGenerics {
        params,
        constraints,
    })
}

fn text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn field_text(node: Node<'_>, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field).map(|child| text(child, source))
}

fn is_comment(node: Node<'_>) -> bool {
    node.kind().contains("comment")
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !is_comment(*child))
        .collect()
}

/// Each named entry of a bound list node (Rust `trait_bounds`, Java
/// `type_bound`), as written.
fn entries(list: Node<'_>, source: &str) -> Vec<String> {
    named_children(list)
        .into_iter()
        .map(|entry| text(entry, source))
        .collect()
}

/// Appends `bounds` to the declared parameter `subject` names, or returns
/// `false` when no parameter has that name.
fn attach(params: &mut [Param], subject: &str, bounds: Vec<String>) -> bool {
    match params.iter_mut().find(|param| param.name == subject) {
        Some(param) => {
            param.constraints.extend(bounds);
            true
        }
        None => false,
    }
}

// ============================================================================
// Rust
// ============================================================================

fn rust_generics(declaration: Node<'_>, source: &str, constraints: &mut Vec<String>) -> Vec<Param> {
    let mut params = Vec::new();
    if let Some(list) = declaration.child_by_field_name("type_parameters") {
        for (index, entry) in named_children(list).into_iter().enumerate() {
            let kind = match entry.kind() {
                "lifetime_parameter" => GenericParamKind::Lifetime,
                "type_parameter" => GenericParamKind::Type,
                "const_parameter" => GenericParamKind::Const,
                _ => continue,
            };
            let Some(name) = field_text(entry, "name", source) else {
                continue;
            };
            let mut param = Param::new(&name, kind, index);
            if let Some(bounds) = entry.child_by_field_name("bounds") {
                param.constraints = entries(bounds, source);
            }
            // A const parameter's `type:` is its declared type, not a bound.
            param.default = match kind {
                GenericParamKind::Const => field_text(entry, "value", source),
                _ => field_text(entry, "default_type", source),
            };
            params.push(param);
        }
    }
    if let Some(clause) = find_child_by_kind(declaration, "where_clause") {
        for predicate in named_children(clause) {
            if predicate.kind() != "where_predicate" {
                continue;
            }
            let subject = predicate.child_by_field_name("left");
            let bounds = predicate.child_by_field_name("bounds");
            let attached = match (subject, bounds) {
                // Only a bare parameter name (`T`, `'a`) is that parameter;
                // `T::Item`, `Vec<T>` and `for<'c> &'c T` are not.
                (Some(subject), Some(bounds))
                    if matches!(subject.kind(), "type_identifier" | "lifetime") =>
                {
                    attach(&mut params, &text(subject, source), entries(bounds, source))
                }
                _ => false,
            };
            if !attached {
                constraints.push(text(predicate, source));
            }
        }
    }
    params
}

// ============================================================================
// TypeScript
// ============================================================================

fn typescript_generics(declaration: Node<'_>, source: &str) -> Vec<Param> {
    let Some(list) = declaration.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    for (index, entry) in named_children(list).into_iter().enumerate() {
        let Some(name) = field_text(entry, "name", source) else {
            continue;
        };
        let mut param = Param::new(&name, GenericParamKind::Type, index);
        // `constraint` and `default_type` include their `extends` / `=`
        // token; the entry is their one named child.
        let inner = |field| {
            entry
                .child_by_field_name(field)
                .and_then(|node| named_children(node).into_iter().next())
                .map(|node| text(node, source))
        };
        param.constraints.extend(inner("constraint"));
        param.default = inner("value");
        params.push(param);
    }
    params
}

// ============================================================================
// Go
// ============================================================================

fn go_generics(declaration: Node<'_>, name: Node<'_>, source: &str) -> Vec<Param> {
    // A `type` declaration (or group) holds one spec per name.
    let owner = match name.parent() {
        Some(spec) if matches!(spec.kind(), "type_spec" | "type_alias") => spec,
        _ => declaration,
    };
    let Some(list) = owner.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    let mut index = 0;
    for declaration in named_children(list) {
        // `K, V any`: every name shares the one constraint, kept whole
        // (unions and `~` terms included).
        let constraint = field_text(declaration, "type", source);
        let mut cursor = declaration.walk();
        for name in declaration.children_by_field_name("name", &mut cursor) {
            let mut param = Param::new(&text(name, source), GenericParamKind::Type, index);
            param.constraints.extend(constraint.clone());
            params.push(param);
            index += 1;
        }
    }
    params
}

// ============================================================================
// Python
// ============================================================================

fn python_generics(declaration: Node<'_>, source: &str) -> Vec<Param> {
    let Some(list) = declaration.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    for (index, entry) in named_children(list).into_iter().enumerate() {
        // Each entry is a `type` wrapping a bare name, a `constrained_type`
        // (`T: int`, two `type` children) or a `splat_type` (`*Ts`, `**P`,
        // named without the stars).
        let Some(inner) = named_children(entry).into_iter().next() else {
            continue;
        };
        let (name, bound) = match inner.kind() {
            "identifier" => (Some(inner), None),
            "constrained_type" => {
                let parts = named_children(inner);
                (
                    parts.first().and_then(|part| named_children(*part).into_iter().next()),
                    parts.get(1).copied(),
                )
            }
            "splat_type" => (named_children(inner).into_iter().next(), None),
            _ => (None, None),
        };
        let Some(name) = name else {
            continue;
        };
        let mut param = Param::new(&text(name, source), GenericParamKind::Type, index);
        param.constraints.extend(bound.map(|bound| text(bound, source)));
        params.push(param);
    }
    params
}

// ============================================================================
// Java
// ============================================================================

fn java_generics(declaration: Node<'_>, source: &str) -> Vec<Param> {
    let Some(list) = declaration.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    for (index, entry) in named_children(list).into_iter().enumerate() {
        let Some(name) = find_child_by_kind(entry, "type_identifier") else {
            continue;
        };
        let mut param = Param::new(&text(name, source), GenericParamKind::Type, index);
        if let Some(bound) = find_child_by_kind(entry, "type_bound") {
            param.constraints = entries(bound, source);
        }
        params.push(param);
    }
    params
}

// ============================================================================
// C#
// ============================================================================

fn csharp_generics(declaration: Node<'_>, source: &str, constraints: &mut Vec<String>) -> Vec<Param> {
    // The list is fielded on interfaces and methods, unfielded on classes.
    let Some(list) = declaration
        .child_by_field_name("type_parameters")
        .or_else(|| find_child_by_kind(declaration, "type_parameter_list"))
    else {
        return Vec::new();
    };
    let mut params = Vec::new();
    for (index, entry) in named_children(list).into_iter().enumerate() {
        let Some(name) = field_text(entry, "name", source) else {
            continue;
        };
        let mut param = Param::new(&name, GenericParamKind::Type, index);
        let mut cursor = entry.walk();
        param.variance = entry
            .children(&mut cursor)
            .find(|token| matches!(token.kind(), "in" | "out"))
            .map(|token| token.kind().to_string());
        params.push(param);
    }
    let mut cursor = declaration.walk();
    let clauses: Vec<Node<'_>> = declaration
        .children(&mut cursor)
        .filter(|child| child.kind() == "type_parameter_constraints_clause")
        .collect();
    for clause in clauses {
        let parts = named_children(clause);
        let Some(subject) = parts.first() else {
            continue;
        };
        let bounds = parts[1..]
            .iter()
            .filter(|part| part.kind() == "type_parameter_constraint")
            .map(|part| text(*part, source))
            .collect();
        if !attach(&mut params, &text(*subject, source), bounds) {
            // The clause without its `where`, as written.
            constraints.push(source[subject.start_byte()..clause.end_byte()].trim().to_string());
        }
    }
    params
}

// ============================================================================
// C++
// ============================================================================

fn cpp_generics(
    declaration: Node<'_>,
    name: Node<'_>,
    source: &str,
    constraints: &mut Vec<String>,
) -> Vec<Param> {
    // Only the template that directly wraps the declaration: an enclosing
    // class template's list belongs to the class, and in
    // `template <class T> template <class U> void A<T>::f(U)` the inner one
    // is the member's.
    let Some(template) = declaration
        .parent()
        .filter(|parent| parent.kind() == "template_declaration")
    else {
        return Vec::new();
    };
    let mut params = Vec::new();
    if let Some(list) = template.child_by_field_name("parameters") {
        for (index, entry) in named_children(list).into_iter().enumerate() {
            if let Some(param) = cpp_param(entry, index, source) {
                params.push(param);
            }
        }
    }
    if let Some(clause) = find_child_by_kind(template, "requires_clause") {
        constraints.extend(field_text(clause, "constraint", source));
    }
    // A trailing `requires` after a function's parameters.
    if let Some(function) = declared_function_named(declaration, name)
        && let Some(clause) = find_child_by_kind(function.function, "requires_clause")
    {
        constraints.extend(field_text(clause, "constraint", source));
    }
    params
}

/// One template parameter; `None` for an unnamed one (`typename`), whose
/// slot still counts toward later positions.
///
/// A non-type parameter is `Const` and its declared type is no constraint.
/// The pinned grammar parses a concept-constrained type parameter
/// (`template <std::integral T>`) exactly like a non-type parameter of a
/// named type, so it is read as `Const` too.
fn cpp_param(entry: Node<'_>, index: usize, source: &str) -> Option<Param> {
    fn type_name(node: Node<'_>) -> Option<Node<'_>> {
        node.child_by_field_name("name")
            .or_else(|| find_child_by_kind(node, "type_identifier"))
    }
    let (name, kind) = match entry.kind() {
        "type_parameter_declaration"
        | "optional_type_parameter_declaration"
        | "variadic_type_parameter_declaration" => (type_name(entry)?, GenericParamKind::Type),
        // `template <class> class TT`: the name sits on the trailing
        // `class TT`, after the parameter's own parameter list.
        "template_template_parameter_declaration" => {
            let declared = named_children(entry)
                .into_iter()
                .rfind(|child| child.kind().ends_with("type_parameter_declaration"))?;
            (type_name(declared)?, GenericParamKind::Type)
        }
        "parameter_declaration" | "optional_parameter_declaration" | "variadic_parameter_declaration" => {
            (declarator_name(entry.child_by_field_name("declarator")?)?, GenericParamKind::Const)
        }
        _ => return None,
    };
    let mut param = Param::new(&text(name, source), kind, index);
    param.default = field_text(entry, "default_type", source)
        .or_else(|| field_text(entry, "default_value", source));
    Some(param)
}

// ============================================================================
// Swift
// ============================================================================

fn swift_generics(declaration: Node<'_>, source: &str, constraints: &mut Vec<String>) -> Vec<Param> {
    let mut params = Vec::new();
    if let Some(list) = find_child_by_kind(declaration, "type_parameters") {
        for (index, entry) in named_children(list).into_iter().enumerate() {
            let Some(name) = find_child_by_kind(entry, "type_identifier") else {
                continue;
            };
            let mut param = Param::new(&text(name, source), GenericParamKind::Type, index);
            // The pinned grammar puts the inline bound under field `name`.
            param.constraints.extend(field_text(entry, "name", source));
            params.push(param);
        }
    }
    if let Some(clause) = find_child_by_kind(declaration, "type_constraints") {
        for constraint in named_children(clause) {
            if constraint.kind() != "type_constraint" {
                continue;
            }
            // `T: P` adds a bound to `T`; a same-type requirement
            // (`U == Int`) or a nested subject (`T.Element: P`) is the
            // declaration's.
            let attached = named_children(constraint)
                .into_iter()
                .next()
                .filter(|requirement| requirement.kind() == "inheritance_constraint")
                .and_then(|requirement| {
                    let subject = requirement.child_by_field_name("constrained_type")?;
                    let bound = requirement.child_by_field_name("name")?;
                    Some(attach(&mut params, &text(subject, source), vec![text(bound, source)]))
                })
                .unwrap_or(false);
            if !attached {
                constraints.push(text(constraint, source));
            }
        }
    }
    params
}

// ============================================================================
// Scala
// ============================================================================

fn scala_generics(declaration: Node<'_>, source: &str) -> Vec<Param> {
    // Found by kind: a function's list sits under field `parameters`, beside
    // its value parameters.
    let Some(list) = find_child_by_kind(declaration, "type_parameters") else {
        return Vec::new();
    };
    // An invariant parameter is flat in the list (`name`, then its `bound`
    // fields); a variant one is wrapped with the same fields inside.
    let mut params: Vec<Param> = Vec::new();
    let mut pending_bound = None;
    for (field, child) in fielded_children(list) {
        match (field, child.kind()) {
            (_, "covariant_type_parameter" | "contravariant_type_parameter") => {
                let Some(name) = child.child_by_field_name("name") else {
                    continue;
                };
                let mut param = Param::new(&text(name, source), GenericParamKind::Type, params.len());
                param.variance = Some(if child.kind() == "covariant_type_parameter" { "+" } else { "-" }.to_string());
                let mut start = None;
                for (field, bound) in fielded_children(child) {
                    if field == Some("bound") {
                        param.constraints.extend(scala_bound(bound, &mut start, source));
                    }
                }
                params.push(param);
            }
            (Some("name"), _) => {
                params.push(Param::new(&text(child, source), GenericParamKind::Type, params.len()));
            }
            (Some("bound"), _) => {
                if let Some(bound) = scala_bound(child, &mut pending_bound, source)
                    && let Some(param) = params.last_mut()
                {
                    param.constraints.push(bound);
                }
            }
            _ => {}
        }
    }
    params
}

/// A Scala bound with its operator (`<: T`, `>: T`, `: Ctx`): one list holds
/// upper, lower and context bounds. A context bound's `:` is a separate
/// token, also fielded `bound`, that `start` remembers for the next bound.
fn scala_bound(node: Node<'_>, start: &mut Option<usize>, source: &str) -> Option<String> {
    if !node.is_named() {
        *start = Some(node.start_byte());
        return None;
    }
    let from = start.take().unwrap_or(node.start_byte());
    Some(source[from..node.end_byte()].trim().to_string())
}

fn fielded_children(node: Node<'_>) -> Vec<(Option<&'static str>, Node<'_>)> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .enumerate()
        .map(|(index, child)| (node.field_name_for_child(index as u32), child))
        .collect()
}
