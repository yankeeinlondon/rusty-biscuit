//! Declared type relations: the base types a declaration writes in its own
//! header (`relations.extends` / `relations.implements`), as written.
//!
//! A base keeps its complete written expression, outer whitespace trimmed
//! (`pkg.Base<T>`, `mix(Base)`, `for<'a> Fn(&'a u8)`). Alongside the text the
//! reader records the base's path ([`BasePath`]) when the expression is a
//! plain, possibly qualified and possibly instantiated name, so the per-file
//! resolver can look it up; any other expression (a call, a reference type,
//! a higher-ranked bound) has no path and stays unresolved. Access and
//! `virtual` keywords on a C++ base specifier are not part of its text.
//!
//! Composition and inferred relations are never read: Go struct embeds,
//! PHP trait `use`, Perl `@ISA`/`use parent` and Lua metatables. C#, Swift
//! and Scala base lists mix inheritance and conformance and are not read
//! until their classification is decided.

use super::*;

/// A base type as written in a declaration's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrittenBase {
    /// The complete written expression, outer whitespace trimmed.
    pub(crate) text: String,
    /// Its path, when the expression is a name.
    pub(crate) path: Option<BasePath>,
}

/// A written name path, without generic arguments: `pkg.Base<T>` is
/// `["pkg", "Base"]` relative to the declaration's scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BasePath {
    pub(crate) anchor: PathAnchor,
    pub(crate) segments: Vec<String>,
}

/// Where a written path starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathAnchor {
    /// Looked up from the declaration's scope outward (by the language's
    /// visibility rule).
    Relative,
    /// From the file root only: Rust `crate::`, a PHP fully qualified
    /// `\NS\Base`.
    Root,
    /// Rust `self::`: the enclosing module only.
    SelfModule,
    /// Rust `super::` written this many times: that many modules out.
    Super(usize),
}

/// The bases a declaration writes, each list in source order.
#[derive(Debug, Default)]
pub(crate) struct DeclaredBases {
    pub(crate) extends: Vec<WrittenBase>,
    pub(crate) implements: Vec<WrittenBase>,
}

/// The bases written by the declaration whose captured name is `name`. The
/// declaration is the name's parent node (a Go `type_spec`, a C++
/// `class_specifier`), which is where every supported grammar writes its
/// base list.
pub(crate) fn declared_bases(name: Node<'_>, language: ProgrammingLanguage, source: &str) -> DeclaredBases {
    let Some(owner) = name.parent() else {
        return DeclaredBases::default();
    };
    let mut bases = DeclaredBases::default();
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            typescript_bases(owner, language, source, &mut bases)
        }
        ProgrammingLanguage::Java => java_bases(owner, language, source, &mut bases),
        ProgrammingLanguage::Php => php_bases(owner, language, source, &mut bases),
        ProgrammingLanguage::Python => python_bases(owner, language, source, &mut bases),
        ProgrammingLanguage::Go => go_bases(owner, language, source, &mut bases),
        ProgrammingLanguage::Cpp => cpp_bases(owner, language, source, &mut bases),
        ProgrammingLanguage::Rust => rust_supertraits(owner, language, source, &mut bases),
        // C#, Swift and Scala: mixed lists, classification undecided. The
        // other languages declare no bases in this feature's scope.
        _ => {}
    }
    bases
}

/// A base whose text is `node`'s source text.
pub(crate) fn written_base(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<WrittenBase> {
    let text = node.utf8_text(source.as_bytes()).ok()?.trim();
    (!text.is_empty()).then(|| WrittenBase {
        text: text.to_string(),
        path: base_path(node, language, source),
    })
}

/// The path of a written type or name expression, generic arguments
/// dropped; `None` for anything that is not a (qualified, instantiated)
/// name.
pub(crate) fn base_path(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<BasePath> {
    let mut segments = Vec::new();
    collect_segments(node, source, &mut segments)?;
    let mut anchor = PathAnchor::Relative;
    match language {
        ProgrammingLanguage::Rust => {
            let skip = match segments.first().map(String::as_str) {
                Some("crate") => {
                    anchor = PathAnchor::Root;
                    1
                }
                Some("self") => {
                    anchor = PathAnchor::SelfModule;
                    1
                }
                _ => {
                    let supers = segments.iter().take_while(|segment| *segment == "super").count();
                    if supers > 0 {
                        anchor = PathAnchor::Super(supers);
                    }
                    supers
                }
            };
            segments.drain(..skip);
        }
        ProgrammingLanguage::Php
            if node.kind() == "qualified_name"
                && node.utf8_text(source.as_bytes()).is_ok_and(|text| text.trim_start().starts_with('\\')) =>
        {
            anchor = PathAnchor::Root;
        }
        _ => {}
    }
    (!segments.is_empty()).then_some(BasePath { anchor, segments })
}

fn collect_segments(node: Node<'_>, source: &str, segments: &mut Vec<String>) -> Option<()> {
    match node.kind() {
        // A leaf name.
        "identifier" | "type_identifier" | "name" | "property_identifier" | "namespace_identifier"
        | "package_identifier" | "crate" | "self" | "super" => {
            let text = node.utf8_text(source.as_bytes()).ok()?.trim();
            (!text.is_empty()).then(|| segments.push(text.to_string()))
        }
        // An instantiation: the path of the instantiated name.
        "generic_type" | "template_type" | "subscript" => {
            let named = node
                .child_by_field_name("type")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(0))?;
            collect_segments(named, source, segments)
        }
        // A qualified name: every named part in order, generic arguments
        // excluded.
        "scoped_type_identifier" | "scoped_identifier" | "member_expression" | "nested_type_identifier"
        | "qualified_name" | "namespace_name" | "qualified_identifier" | "attribute" | "qualified_type" => {
            let mut cursor = node.walk();
            let mut any = false;
            for child in node.named_children(&mut cursor) {
                if matches!(child.kind(), "type_arguments" | "template_argument_list" | "comment") {
                    continue;
                }
                collect_segments(child, source, segments)?;
                any = true;
            }
            any.then_some(())
        }
        _ => None,
    }
}

fn push_unique(list: &mut Vec<WrittenBase>, base: Option<WrittenBase>) {
    if let Some(base) = base
        && !list.iter().any(|existing| existing.text == base.text)
    {
        list.push(base);
    }
}

fn named_entries<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect()
}

/// TS/JS `class_heritage` (`extends_clause` with `value:` and its
/// `type_arguments:`, `implements_clause` entries; JS writes the expression
/// directly) and TS interface `extends_type_clause`.
fn typescript_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let mut cursor = owner.walk();
    for child in owner.named_children(&mut cursor) {
        match child.kind() {
            "class_heritage" => {
                let mut inner = child.walk();
                let clauses: Vec<Node<'_>> = child.named_children(&mut inner).collect();
                for clause in clauses {
                    match clause.kind() {
                        "extends_clause" => {
                            for base in extends_clause_bases(clause, language, source) {
                                push_unique(&mut bases.extends, Some(base));
                            }
                        }
                        "implements_clause" => {
                            for entry in named_entries(clause) {
                                push_unique(&mut bases.implements, written_base(entry, language, source));
                            }
                        }
                        "comment" => {}
                        // JavaScript: the heritage is the expression itself.
                        _ => push_unique(&mut bases.extends, written_base(clause, language, source)),
                    }
                }
            }
            "extends_type_clause" => {
                for entry in named_entries(child) {
                    push_unique(&mut bases.extends, written_base(entry, language, source));
                }
            }
            _ => {}
        }
    }
}

/// Each `value:` of a TS `extends_clause` with the `type_arguments:` that
/// follows it: the written text runs from the value to its arguments.
fn extends_clause_bases(clause: Node<'_>, language: ProgrammingLanguage, source: &str) -> Vec<WrittenBase> {
    let mut found = Vec::new();
    let mut cursor = clause.walk();
    let mut current: Option<(Node<'_>, usize)> = None;
    for (index, child) in clause.children(&mut cursor).enumerate() {
        match clause.field_name_for_child(index as u32) {
            Some("value") => {
                if let Some((value, end)) = current.take() {
                    found.extend(base_spanning(value, end, language, source));
                }
                current = Some((child, child.end_byte()));
            }
            Some("type_arguments") => {
                if let Some((_, end)) = current.as_mut() {
                    *end = child.end_byte();
                }
            }
            _ => {}
        }
    }
    if let Some((value, end)) = current {
        found.extend(base_spanning(value, end, language, source));
    }
    found
}

fn base_spanning(value: Node<'_>, end: usize, language: ProgrammingLanguage, source: &str) -> Option<WrittenBase> {
    let text = source.get(value.start_byte()..end)?.trim();
    (!text.is_empty()).then(|| WrittenBase {
        text: text.to_string(),
        path: base_path(value, language, source),
    })
}

/// Java `superclass` → extends; `super_interfaces` (class, enum, record) →
/// implements; interface `extends_interfaces` → extends.
fn java_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let mut cursor = owner.walk();
    for child in owner.named_children(&mut cursor) {
        let (list, entries) = match child.kind() {
            "superclass" => (&mut bases.extends, named_entries(child)),
            "super_interfaces" => (&mut bases.implements, type_list_entries(child)),
            "extends_interfaces" => (&mut bases.extends, type_list_entries(child)),
            _ => continue,
        };
        for entry in entries {
            push_unique(list, written_base(entry, language, source));
        }
    }
}

fn type_list_entries(clause: Node<'_>) -> Vec<Node<'_>> {
    named_entries(clause)
        .into_iter()
        .flat_map(|entry| if entry.kind() == "type_list" { named_entries(entry) } else { vec![entry] })
        .collect()
}

/// PHP `base_clause` → extends (class and interface);
/// `class_interface_clause` → implements (class and enum). A trait `use`
/// in the body is composition.
fn php_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let mut cursor = owner.walk();
    for child in owner.named_children(&mut cursor) {
        let list = match child.kind() {
            "base_clause" => &mut bases.extends,
            "class_interface_clause" => &mut bases.implements,
            _ => continue,
        };
        for entry in named_entries(child) {
            push_unique(list, written_base(entry, language, source));
        }
    }
}

/// Python positional `superclasses` entries; keyword arguments
/// (`metaclass=`) and `**` unpacking are not bases. A `*bases` unpacking
/// is kept as written, unresolved.
fn python_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let Some(arguments) = owner.child_by_field_name("superclasses") else {
        return;
    };
    for entry in named_entries(arguments) {
        if matches!(entry.kind(), "keyword_argument" | "dictionary_splat") {
            continue;
        }
        push_unique(&mut bases.extends, written_base(entry, language, source));
    }
}

/// Go predeclared types that are not interfaces: as an interface element
/// they are type-set terms, not embedded interfaces.
const GO_PREDECLARED_NON_INTERFACE: [&str; 19] = [
    "bool", "byte", "complex64", "complex128", "float32", "float64", "int", "int8", "int16", "int32",
    "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
];

/// Go interface elements that embed a named type (`Base`, `fmt.Stringer`,
/// `G[int]`). A union, a `~` term, a predeclared non-interface type and any
/// type literal are constraint terms, and embedded struct fields are
/// composition.
fn go_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let Some(interface) = owner.child_by_field_name("type").filter(|node| node.kind() == "interface_type") else {
        return;
    };
    for element in named_entries(interface) {
        if element.kind() != "type_elem" {
            continue;
        }
        let terms = named_entries(element);
        let [term] = terms.as_slice() else {
            continue;
        };
        let is_named = match term.kind() {
            "type_identifier" => term
                .utf8_text(source.as_bytes())
                .is_ok_and(|text| !GO_PREDECLARED_NON_INTERFACE.contains(&text.trim())),
            "qualified_type" | "generic_type" => true,
            _ => false,
        };
        if is_named {
            push_unique(&mut bases.extends, written_base(*term, language, source));
        }
    }
}

/// C++ `base_class_clause` entries: the type after any access specifier and
/// `virtual`, so neither is part of the text.
fn cpp_bases(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    let mut cursor = owner.walk();
    let Some(clause) = owner.named_children(&mut cursor).find(|child| child.kind() == "base_class_clause") else {
        return;
    };
    for entry in named_entries(clause) {
        if entry.kind() == "access_specifier" {
            continue;
        }
        push_unique(&mut bases.extends, written_base(entry, language, source));
    }
}

/// Rust supertraits: the trait's `bounds` entries, minus lifetimes (a
/// constraint, not a base) and `?Sized` relaxations.
fn rust_supertraits(owner: Node<'_>, language: ProgrammingLanguage, source: &str, bases: &mut DeclaredBases) {
    if owner.kind() != "trait_item" {
        return;
    }
    let Some(bounds) = owner.child_by_field_name("bounds") else {
        return;
    };
    for entry in named_entries(bounds) {
        if matches!(entry.kind(), "lifetime" | "removed_trait_bound") {
            continue;
        }
        push_unique(&mut bases.extends, written_base(entry, language, source));
    }
}
