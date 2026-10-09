//! Declared type relations: the base types a declaration writes in its own
//! header (`extends`, `implements`, `unclassified_bases`), as written.
//!
//! A base keeps its complete written expression, outer whitespace trimmed
//! (`pkg.Base<T>`, `mix(Base)`, `for<'a> Fn(&'a u8)`). Alongside the text the
//! reader records the base's path ([`BasePath`]) when the expression is a
//! plain, possibly qualified and possibly instantiated name, so the per-file
//! resolver can look it up. C# and Scala base-constructor arguments are
//! excluded from that path but kept in the text. A dynamic call, reference
//! type or higher-ranked bound has no path and stays unresolved. Access and
//! `virtual` keywords on a C++ base specifier are not part of its text.
//!
//! Composition and inferred relations are never read: Go struct embeds,
//! PHP trait `use`, Perl `@ISA`/`use parent` and Lua metatables. Mixed lists
//! retain uncertain entries separately when syntax and local declarations
//! cannot establish their category.

use std::collections::HashSet;

use super::*;
use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::count_shared_context_steps;

/// A base type as written in a declaration's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrittenBase {
    /// The complete written expression, outer whitespace trimmed.
    pub(crate) text: String,
    /// Its path, when the expression is a name.
    pub(crate) path: Option<BasePath>,
    /// Swift evidence distinguishing header bases from extension additions.
    pub(crate) source_span: Option<TextSpan>,
    /// Restores written order when independently classified lists are merged.
    pub(crate) start_byte: usize,
    /// The spelling also names a built-in type that is no base (a Go
    /// predeclared non-interface type, a Swift standard raw-value type).
    /// The spelling alone proves nothing: the linker drops the base only
    /// when no binding visible at the declaration can give the path's first
    /// segment another meaning.
    pub(crate) builtin_spelling: bool,
}

/// A written name path, without generic arguments: `pkg.Base<T>` is
/// `["pkg", "Base"]` relative to the declaration's scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BasePath {
    pub(crate) anchor: PathAnchor,
    pub(crate) segments: Vec<String>,
    /// The number of generic arguments written on each segment (`0` when
    /// none are): `H<int>.Base` is `[1, 0]`. A C# name with a different
    /// argument count is a different type.
    pub(crate) arities: Vec<usize>,
}

/// Where a written path starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathAnchor {
    /// Looked up from the declaration's scope outward (by the language's
    /// visibility rule).
    Relative,
    /// From the file root only: Rust `crate::`, a PHP fully qualified
    /// `\NS\Base`, a C++ `::Base`, or C# `global::Base`.
    Root,
    /// Rust `::name`: a path through an external crate, which no
    /// declaration of this file can be.
    Extern,
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
    pub(crate) unclassified: Vec<WrittenBase>,
}

/// The bases written by the declaration whose captured name is `name`. The
/// declaration is the name's parent node (a Go `type_spec`, a C++
/// `class_specifier`), which is where every supported grammar writes its
/// base list.
pub(crate) fn declared_bases(name: Node<'_>, language: ProgrammingLanguage, source: &str) -> DeclaredBases {
    let Some(owner) = name.indexed_parent() else {
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
        ProgrammingLanguage::CSharp => csharp_bases(owner, source, &mut bases),
        ProgrammingLanguage::Swift => swift_bases(owner, source, &mut bases),
        ProgrammingLanguage::Scala => scala_bases(owner, source, &mut bases),
        _ => {}
    }
    for list in [&mut bases.extends, &mut bases.implements, &mut bases.unclassified] {
        dedupe(list);
    }
    bases
}

/// A base whose text is `node`'s source text.
pub(crate) fn written_base(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<WrittenBase> {
    let text = node.utf8_text(source.as_bytes()).ok()?.trim();
    (!text.is_empty()).then(|| WrittenBase {
        text: text.to_string(),
        path: base_path(node, language, source),
        source_span: (language == ProgrammingLanguage::Swift).then(|| text_span_for_node(node)),
        start_byte: node.start_byte(),
        builtin_spelling: false,
    })
}

/// The path of a written type or name expression, generic arguments
/// dropped; `None` for anything that is not a (qualified, instantiated)
/// name.
pub(crate) fn base_path(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<BasePath> {
    let mut segments = Vec::new();
    let mut arities = Vec::new();
    collect_segments(node, source, &mut segments, &mut arities)?;
    let mut anchor = PathAnchor::Relative;
    match language {
        ProgrammingLanguage::Rust if starts_at_global_scope(node) => anchor = PathAnchor::Extern,
        ProgrammingLanguage::Cpp if starts_at_global_scope(node) => anchor = PathAnchor::Root,
        ProgrammingLanguage::CSharp => {
            let mut first = node;
            while first.kind() == "qualified_name" {
                first = first.child_by_field_name("qualifier")?;
            }
            if first.kind() == "alias_qualified_name" {
                let alias = first.child_by_field_name("alias")?.utf8_text(source.as_bytes()).ok()?;
                if alias != "global" { return None; }
                anchor = PathAnchor::Root;
                segments.remove(0);
                arities.remove(0);
            }
        }
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
            arities.drain(..skip);
        }
        ProgrammingLanguage::Php
            if node.kind() == "qualified_name"
                && node.utf8_text(source.as_bytes()).is_ok_and(|text| text.trim_start().starts_with('\\')) =>
        {
            anchor = PathAnchor::Root;
        }
        _ => {}
    }
    (!segments.is_empty()).then_some(BasePath { anchor, segments, arities })
}

/// Whether a Rust or C++ path is written with a leading `::`: its leftmost
/// qualified part has no qualifier of its own and starts with that token.
fn starts_at_global_scope(node: Node<'_>) -> bool {
    let mut current = node;
    loop {
        match current.kind() {
            "generic_type" | "template_type" => {
                let Some(named) = current.child_by_field_name("type").or_else(|| current.child_by_field_name("name"))
                else {
                    return false;
                };
                current = named;
            }
            "scoped_type_identifier" | "scoped_identifier" | "qualified_identifier" => {
                match current.child_by_field_name("path").or_else(|| current.child_by_field_name("scope")) {
                    Some(qualifier) => current = qualifier,
                    None => return current.child(0).is_some_and(|first| first.kind() == "::"),
                }
            }
            _ => return false,
        }
    }
}

fn collect_segments(node: Node<'_>, source: &str, segments: &mut Vec<String>, arities: &mut Vec<usize>) -> Option<()> {
    match node.kind() {
        // A leaf name.
        "identifier" | "simple_identifier" | "type_identifier" | "name" | "property_identifier" | "namespace_identifier"
        | "package_identifier" | "crate" | "self" | "super" => {
            let text = node.utf8_text(source.as_bytes()).ok()?.trim();
            (!text.is_empty()).then(|| {
                segments.push(text.to_string());
                arities.push(0);
            })
        }
        // An instantiation: the path of the instantiated name, whose last
        // segment carries the written argument count.
        "generic_type" | "generic_name" | "template_type" | "subscript" => {
            let named = node
                .child_by_field_name("type")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(0))?;
            collect_segments(named, source, segments, arities)?;
            let mut cursor = node.walk();
            let written = node
                .named_children(&mut cursor)
                .find(|child| matches!(child.kind(), "type_arguments" | "type_argument_list" | "template_argument_list"))
                .map_or(0, |list| named_entries(list).len());
            if let Some(last) = arities.last_mut() {
                *last = written;
            }
            Some(())
        }
        // A qualified name: every named part in order, generic arguments
        // excluded.
        "scoped_type_identifier" | "scoped_identifier" | "member_expression" | "nested_type_identifier"
        | "qualified_name" | "alias_qualified_name" | "namespace_name" | "qualified_identifier" | "attribute"
        | "qualified_type" | "user_type" | "stable_type_identifier" | "stable_identifier" | "nested_identifier" => {
            let mut cursor = node.walk();
            let mut any = false;
            for child in node.named_children(&mut cursor) {
                if matches!(child.kind(), "type_arguments" | "type_argument_list" | "template_argument_list" | "comment") {
                    continue;
                }
                collect_segments(child, source, segments, arities)?;
                any = true;
            }
            any.then_some(())
        }
        _ => None,
    }
}

/// Adds a base; [`dedupe`] later keeps the first of equal written
/// expressions in each list.
fn push_unique(list: &mut Vec<WrittenBase>, base: Option<WrittenBase>) {
    list.extend(base);
}

/// Keeps the first of each written expression, in order: one membership
/// check per base, however many a list holds.
fn dedupe(list: &mut Vec<WrittenBase>) {
    count_shared_context_steps(list.len());
    let mut seen: HashSet<String> = HashSet::with_capacity(list.len());
    list.retain(|base| seen.insert(base.text.clone()));
}

fn named_entries<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect()
}

/// Whether a C# declaration declares a value type: a `struct` or a
/// `record struct`. A `record` and a `record class` are classes, although
/// all three report the `Type` kind.
pub(crate) fn is_csharp_struct(declaration: Node<'_>) -> bool {
    match declaration.kind() {
        "struct_declaration" => true,
        "record_declaration" => {
            let mut cursor = declaration.walk();
            declaration.children(&mut cursor).any(|child| child.kind() == "struct")
        }
        _ => false,
    }
}

/// Whether a C# type declaration is written `partial`. The modifier alone
/// proves no shared identity: only `partial` declarations that also agree
/// on [`csharp_partial_shape`] and enclosing owner are fragments of one
/// type.
pub(crate) fn is_csharp_partial(declaration: Node<'_>, source: &str) -> bool {
    let mut cursor = declaration.walk();
    declaration
        .children(&mut cursor)
        .any(|child| child.kind() == "modifier" && source.get(child.start_byte()..child.end_byte()) == Some("partial"))
}

/// What a C# `partial` type declaration must share with another of the same
/// bound name and enclosing owner to be a fragment of the same type: its
/// category (class, struct, interface, record class, record struct; a
/// `record` and a `record class` are one category) and its generic arity.
/// `None` for a declaration that is not a `partial` type.
pub(crate) fn csharp_partial_shape(declaration: Node<'_>, source: &str) -> Option<(u8, usize)> {
    let category = match declaration.kind() {
        "class_declaration" => 0,
        "struct_declaration" => 1,
        "interface_declaration" => 2,
        "record_declaration" if is_csharp_struct(declaration) => 4,
        "record_declaration" => 3,
        _ => return None,
    };
    is_csharp_partial(declaration, source).then(|| (category, csharp_arity(declaration)))
}

/// The generic arity a C# type declaration writes: the length of its own
/// type parameter list.
pub(crate) fn csharp_arity(declaration: Node<'_>) -> usize {
    let mut cursor = declaration.walk();
    declaration
        .children(&mut cursor)
        .find(|child| child.kind() == "type_parameter_list")
        .map_or(0, |list| {
            let mut cursor = list.walk();
            list.named_children(&mut cursor).filter(|child| child.kind() == "type_parameter").count()
        })
}

fn csharp_bases(owner: Node<'_>, source: &str, bases: &mut DeclaredBases) {
    let Some(list) = find_child_by_kind(owner, "base_list") else { return };
    if owner.kind() == "enum_declaration" { return; }
    let is_interface = owner.kind() == "interface_declaration";
    let is_struct = is_csharp_struct(owner);
    let entries = named_entries(list);
    let mut position = 0;
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        index += 1;
        let invocation = entries.get(index).filter(|node| node.kind() == "argument_list");
        let base = if let Some(arguments) = invocation {
            index += 1;
            base_spanning(entry, arguments.end_byte(), ProgrammingLanguage::CSharp, source)
        } else {
            written_base(entry, ProgrammingLanguage::CSharp, source)
        };
        let list = if is_interface { &mut bases.extends }
            else if is_struct || position > 0 { &mut bases.implements }
            else if invocation.is_some() { &mut bases.extends }
            else { &mut bases.unclassified };
        push_unique(list, base);
        position += 1;
    }
}

/// Swift standard raw-value types an enum's first entry may name without
/// a case writing a raw value.
const SWIFT_STANDARD_RAW_TYPES: [&str; 14] = [
    "Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16", "UInt32", "UInt64", "String",
    "Character", "Float", "Double",
];

/// Whether `path` spells a Swift standard raw-value type: one of
/// [`SWIFT_STANDARD_RAW_TYPES`] or `Unicode.Scalar`, bare or qualified by
/// the `Swift` module.
fn is_swift_standard_raw_type(path: &BasePath) -> bool {
    let segments: Vec<&str> = path.segments.iter().map(String::as_str).collect();
    let unqualified = match segments.as_slice() {
        ["Swift", rest @ ..] => rest,
        all => all,
    };
    path.anchor == PathAnchor::Relative
        && match unqualified {
            [name] => SWIFT_STANDARD_RAW_TYPES.contains(name),
            ["Unicode", "Scalar"] => true,
            _ => false,
        }
}

/// Swift uses the same node for classes, structs, enums and extensions.
/// Enum raw-value types are storage declarations, not inheritance: an
/// explicit raw value proves one by syntax, a standard raw-type spelling
/// only once the linker finds no binding that shadows it.
fn swift_bases(owner: Node<'_>, source: &str, bases: &mut DeclaredBases) {
    let kind = declaration_keyword(owner, ProgrammingLanguage::Swift);
    let mut cursor = owner.walk();
    let entries = owner.named_children(&mut cursor).filter(|child| child.kind() == "inheritance_specifier");
    for (position, entry) in entries.enumerate() {
        let Some(target) = entry.named_child(0) else { continue };
        let raw_type_slot = kind.as_deref() == Some("enum") && position == 0;
        if raw_type_slot {
            // An explicit raw value establishes a raw-value enum even when
            // its raw type is a user-defined literal-convertible type.
            if owner.child_by_field_name("body").is_some_and(|body| {
                named_entries(body).iter().any(|case| case.kind() == "enum_entry" && {
                    let mut cursor = case.walk();
                    case.children(&mut cursor).any(|child| child.kind() == "=")
                })
            }) { continue; }
        }
        let list = match kind.as_deref() {
            Some("protocol") => &mut bases.extends,
            Some("struct" | "actor" | "extension") => &mut bases.implements,
            Some("class" | "enum") if position > 0 => &mut bases.implements,
            _ => &mut bases.unclassified,
        };
        let base = written_base(target, ProgrammingLanguage::Swift, source).map(|mut base| {
            base.builtin_spelling = raw_type_slot && base.path.as_ref().is_some_and(is_swift_standard_raw_type);
            base
        });
        push_unique(list, base);
    }
}

fn scala_bases(owner: Node<'_>, source: &str, bases: &mut DeclaredBases) {
    let Some(clause) = find_child_by_kind(owner, "extends_clause") else { return };
    let entries = named_entries(clause);
    let mut position = 0;
    let mut index = 0;
    while index < entries.len() {
        let target = entries[index];
        index += 1;
        let mut end = target.end_byte();
        while index < entries.len() && entries[index].kind() == "arguments" {
            end = entries[index].end_byte();
            index += 1;
        }
        let list = if owner.kind() == "trait_definition" { &mut bases.extends }
            else if position > 0 { &mut bases.implements }
            else { &mut bases.unclassified };
        push_unique(list, base_spanning(target, end, ProgrammingLanguage::Scala, source));
        position += 1;
    }
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
        source_span: None,
        start_byte: value.start_byte(),
        builtin_spelling: false,
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

/// Go predeclared types that are not interfaces: unless a declaration
/// shadows the name, such an interface element is a type-set term, not an
/// embedded interface.
const GO_PREDECLARED_NON_INTERFACE: [&str; 19] = [
    "bool", "byte", "complex64", "complex128", "float32", "float64", "int", "int8", "int16", "int32",
    "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
];

/// Go interface elements that embed a named type (`Base`, `fmt.Stringer`,
/// `G[int]`). A union, a `~` term and any type literal are constraint terms,
/// and embedded struct fields are composition. A name spelled like a
/// predeclared non-interface type is kept with
/// [`WrittenBase::builtin_spelling`]: Go's universe names can be redeclared,
/// so only the linker, which sees the bindings, can drop it.
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
        if !matches!(term.kind(), "type_identifier" | "qualified_type" | "generic_type") {
            continue;
        }
        let base = written_base(*term, language, source).map(|mut base| {
            base.builtin_spelling =
                term.kind() == "type_identifier" && GO_PREDECLARED_NON_INTERFACE.contains(&base.text.as_str());
            base
        });
        push_unique(&mut bases.extends, base);
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

/// What one import statement binds in the scope that holds it, for the
/// relation resolver: a binding of a path's first segment decides that
/// segment's lookup there, and an imported name never resolves.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ImportBinding<'source> {
    /// The names bound, as written (alias, else the last imported
    /// segment; a Python `import a.b` binds `a`), and the name groups
    /// (`analysis::scope` `TYPE`/`VALUE` bits) they are bound in: a Swift
    /// `import func`/`let`/`var` binds a value, `import class`/`struct`/
    /// `enum`/`protocol`/`typealias` and a Java single-type import a type,
    /// every other import every group.
    Names(Vec<&'source str>, u8),
    /// A wildcard import, a C++ using-directive, a Go dot import or an
    /// import whose names cannot be read: any name may be bound, so no
    /// lookup through this scope can be proved.
    Opaque,
    /// A Swift whole-module import (`import Foundation`): like
    /// [`Self::Opaque`] for a lookup, but it names no single binding that
    /// could shadow a standard-library spelling.
    Module,
}

/// Every name group a binding can occupy.
const ANY_GROUP: u8 = crate::analysis::scope::ANY_CATEGORY | RELATION_NAMESPACE;

/// The group of a TypeScript namespace: the only binding a qualifier in a
/// type position (`implements ns.I`) looks up besides imports.
pub(crate) const RELATION_NAMESPACE: u8 = 8;

/// The binding `node` makes when it is an import statement of `language`
/// (Rust `use`, C++ `using` and namespace aliases, PHP `use`, Python
/// imports, JS/TS imports and TS `import x =`, C# `using` aliases, Swift
/// selective imports, every expression of a Scala `import`, Java single,
/// static and on-demand imports, Go package imports). A Java or Go import
/// binds a file-level name, so it decides a qualifier such as `Entry` in
/// `Entry.Base` before the file's own package name is tried.
///
/// An unaliased Go import binds the package name Go tools assume for its
/// path: the last element, without a `.vN` suffix, or the element before a
/// `/vN` major-version element.
///
/// An import whose bound names cannot all be read is
/// [`ImportBinding::Opaque`], never a shorter name list: a missed binding
/// would let a farther declaration claim the name.
pub(crate) fn import_binding<'source>(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &'source str,
) -> Option<ImportBinding<'source>> {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::trim).filter(|text| !text.is_empty());
    let mut names = Vec::new();
    let mut categories = ANY_GROUP;
    match (language, node.kind()) {
        (ProgrammingLanguage::Java, "import_declaration") => {
            let mut cursor = node.walk();
            let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
            if children.iter().any(|child| child.kind() == "asterisk") {
                return Some(ImportBinding::Opaque);
            }
            // A static import may name a member type as well as a field or
            // method; a single-type import names a type.
            if !children.iter().any(|child| child.kind() == "static") {
                categories = crate::analysis::scope::TYPE;
            }
            let bound = children.iter().find(|child| matches!(child.kind(), "scoped_identifier" | "identifier"))
                .and_then(|path| match path.kind() {
                    "scoped_identifier" => path.child_by_field_name("name"),
                    _ => Some(*path),
                })
                .and_then(text);
            let Some(bound) = bound else { return Some(ImportBinding::Opaque) };
            names.push(bound);
        }
        (ProgrammingLanguage::Go, "import_declaration") => {
            let mut specs = named_entries(node);
            if let Some(list) = specs.iter().position(|spec| spec.kind() == "import_spec_list") {
                let list = specs.remove(list);
                specs.extend(named_entries(list));
            }
            for spec in specs.into_iter().filter(|spec| spec.kind() == "import_spec") {
                match spec.child_by_field_name("name") {
                    Some(name) if name.kind() == "dot" => return Some(ImportBinding::Opaque),
                    Some(name) if name.kind() == "blank_identifier" => {}
                    Some(name) => names.extend(text(name)),
                    None => {
                        let path = spec
                            .child_by_field_name("path")
                            .and_then(|path| named_entries(path).into_iter().next())
                            .and_then(text);
                        let Some(bound) = path.and_then(go_package_name) else {
                            return Some(ImportBinding::Opaque);
                        };
                        names.push(bound);
                    }
                }
            }
        }
        (ProgrammingLanguage::CSharp, "using_directive") => {
            if let Some(alias) = node.child_by_field_name("name") {
                names.extend(text(alias));
            } else {
                return Some(ImportBinding::Opaque);
            }
        }
        (ProgrammingLanguage::Swift, "import_declaration") => {
            let selective = crate::analysis::declaration_names::swift_import_selector(node).map(|selector| match selector {
                "let" | "var" | "func" => crate::analysis::scope::VALUE,
                _ => crate::analysis::scope::TYPE,
            });
            let Some(group) = selective else { return Some(ImportBinding::Module) };
            categories = group;
            // A selective import whose name cannot be read binds an
            // unknown name: opaque, never absent.
            let Some(bound) = find_child_by_kind(node, "identifier")
                .and_then(|identifier| named_entries(identifier).last().copied())
                .and_then(text)
            else {
                return Some(ImportBinding::Opaque);
            };
            names.push(bound);
        }
        (ProgrammingLanguage::Scala, "import_declaration") => {
            if !scala_import_names(node, source, &mut names) {
                return Some(ImportBinding::Opaque);
            }
        }
        (ProgrammingLanguage::Rust, "use_declaration") => {
            let complete = node
                .child_by_field_name("argument")
                .is_some_and(|argument| rust_use_names(argument, None, source, &mut names));
            if !complete {
                return Some(ImportBinding::Opaque);
            }
        }
        (ProgrammingLanguage::Cpp, "using_declaration") => {
            let mut cursor = node.walk();
            if node.children(&mut cursor).any(|child| child.kind() == "namespace") {
                return Some(ImportBinding::Opaque);
            }
            let mut cursor = node.walk();
            let bound = node
                .named_children(&mut cursor)
                .find(|child| child.kind() != "comment")
                .and_then(last_segment)
                .and_then(text);
            let Some(bound) = bound else {
                return Some(ImportBinding::Opaque);
            };
            names.push(bound);
        }
        (ProgrammingLanguage::Cpp, "namespace_alias_definition") => {
            names.extend(node.child_by_field_name("name").and_then(text));
        }
        (ProgrammingLanguage::Php, "namespace_use_declaration") => {
            let mut cursor = node.walk();
            // `use function …` / `use const …` bind no class name.
            if node.children(&mut cursor).any(|child| matches!(child.kind(), "function" | "const")) {
                return None;
            }
            let mut clauses = named_entries(node);
            if let Some(group) = node.child_by_field_name("body") {
                clauses.extend(named_entries(group));
            }
            for clause in clauses.into_iter().filter(|clause| clause.kind() == "namespace_use_clause") {
                if clause.child_by_field_name("type").is_some() {
                    continue;
                }
                let bound = clause.child_by_field_name("alias").or_else(|| {
                    let mut cursor = clause.walk();
                    let named = clause.named_children(&mut cursor).find(|child| child.kind() != "comment")?;
                    // A qualified name's last segment is its last `name`.
                    if named.kind() == "qualified_name" {
                        named_entries(named).into_iter().rev().find(|part| part.kind() == "name")
                    } else {
                        Some(named)
                    }
                });
                names.extend(bound.and_then(text));
            }
        }
        (ProgrammingLanguage::Python, "import_statement" | "import_from_statement") => {
            let mut cursor = node.walk();
            for (index, child) in node.children(&mut cursor).enumerate() {
                if child.kind() == "wildcard_import" {
                    return Some(ImportBinding::Opaque);
                }
                if node.field_name_for_child(index as u32) != Some("name") {
                    continue;
                }
                let bound = match child.kind() {
                    "aliased_import" => child.child_by_field_name("alias"),
                    // `import a.b` binds `a`; `from m import b` binds `b`.
                    _ => child.named_child(0),
                };
                names.extend(bound.and_then(text));
            }
        }
        (ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript, "import_statement") => {
            let mut cursor = node.walk();
            let clause = node.named_children(&mut cursor).find(|child| child.kind() == "import_clause")?;
            for entry in named_entries(clause) {
                match entry.kind() {
                    "identifier" => names.extend(text(entry)),
                    "namespace_import" => {
                        let mut cursor = entry.walk();
                        let bound = entry.named_children(&mut cursor).find(|child| child.kind() == "identifier");
                        names.extend(bound.and_then(text));
                    }
                    "named_imports" => {
                        for specifier in named_entries(entry) {
                            let bound = specifier
                                .child_by_field_name("alias")
                                .or_else(|| specifier.child_by_field_name("name"));
                            names.extend(bound.and_then(text));
                        }
                    }
                    _ => {}
                }
            }
        }
        (ProgrammingLanguage::TypeScript, "import_alias") => {
            let mut cursor = node.walk();
            let bound = node.named_children(&mut cursor).find(|child| child.kind() == "identifier");
            names.extend(bound.and_then(text));
        }
        _ => return None,
    }
    Some(ImportBinding::Names(names, categories))
}

/// The package name an unaliased Go import path is assumed to bind, `None`
/// when it is not an identifier.
fn go_package_name(path: &str) -> Option<&str> {
    let is_major_version = |element: &str| {
        element.strip_prefix('v').is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
    };
    let mut elements = path.rsplit('/');
    let mut last = elements.next()?;
    if is_major_version(last) {
        last = elements.next()?;
    }
    if let Some((stem, suffix)) = last.rsplit_once('.')
        && is_major_version(suffix)
    {
        last = stem;
    }
    let mut chars = last.chars();
    let starts = chars.next().is_some_and(|first| first == '_' || first.is_alphabetic());
    (starts && chars.all(|rest| rest == '_' || rest.is_alphanumeric())).then_some(last)
}

/// Collects the names a Scala `import` binds; `false` when any of its
/// expressions may bind names it does not spell (a wildcard, a `given`
/// selector) or cannot be read.
///
/// The clause's expressions are flat siblings separated by `,` tokens: each
/// is a `path` ending in nothing (its last segment is bound), a wildcard,
/// a selector group, or an unbraced `as` rename. A rename binds its alias;
/// an exclusion (`Other => _`, `Other as _`) binds nothing.
fn scala_import_names<'source>(node: Node<'_>, source: &'source str, names: &mut Vec<&'source str>) -> bool {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::trim).filter(|text| !text.is_empty());
    let renamed = |selector: Node<'_>, names: &mut Vec<&'source str>| match selector.child_by_field_name("alias") {
        Some(alias) if alias.kind() == "wildcard" => true,
        Some(alias) => text(alias).map(|alias| names.push(alias)).is_some(),
        None => false,
    };
    // The last `path` segment of the expression being read, until a suffix
    // (selectors, wildcard, rename) says the path is not itself bound.
    let mut bare_path: Option<Node<'_>> = None;
    let mut cursor = node.walk();
    for (index, child) in node.children(&mut cursor).enumerate() {
        if node.field_name_for_child(index as u32) == Some("path") {
            if child.is_named() {
                bare_path = Some(child);
            }
            continue;
        }
        match child.kind() {
            "import" | "." | "comment" | "block_comment" => {}
            "," => {
                if let Some(path) = bare_path.take() {
                    match text(path) {
                        Some(bound) => names.push(bound),
                        None => return false,
                    }
                }
            }
            "as_renamed_identifier" => {
                bare_path = None;
                if !renamed(child, names) {
                    return false;
                }
            }
            "namespace_selectors" => {
                bare_path = None;
                for selector in named_entries(child) {
                    let complete = match selector.kind() {
                        "identifier" | "operator_identifier" => text(selector).map(|bound| names.push(bound)).is_some(),
                        "arrow_renamed_identifier" | "as_renamed_identifier" => renamed(selector, names),
                        "comment" | "block_comment" => true,
                        // A wildcard, or a `given` selector, imports names
                        // the clause does not spell.
                        _ => false,
                    };
                    if !complete {
                        return false;
                    }
                }
            }
            // `namespace_wildcard`, and anything this reader does not know.
            _ => return false,
        }
    }
    match bare_path {
        Some(path) => text(path).map(|bound| names.push(bound)).is_some(),
        None => true,
    }
}

/// The last name of a C++ (possibly qualified) name.
fn last_segment(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "qualified_identifier" => last_segment(node.child_by_field_name("name")?),
        _ => Some(node),
    }
}

/// Collects the names a Rust use tree binds; `false` when it holds a glob.
/// `parent` is the last segment of the enclosing list's path, which a
/// `self` entry binds.
fn rust_use_names<'source>(
    tree: Node<'_>,
    parent: Option<&'source str>,
    source: &'source str,
    names: &mut Vec<&'source str>,
) -> bool {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::trim).filter(|text| !text.is_empty());
    match tree.kind() {
        "use_wildcard" => return false,
        "use_as_clause" => names.extend(tree.child_by_field_name("alias").and_then(text)),
        "self" => names.extend(parent),
        "identifier" | "crate" | "super" => names.extend(text(tree)),
        "scoped_identifier" => names.extend(tree.child_by_field_name("name").and_then(text)),
        "scoped_use_list" => {
            let last = tree.child_by_field_name("path").and_then(|path| match path.kind() {
                "scoped_identifier" => path.child_by_field_name("name").and_then(text),
                _ => text(path),
            });
            if let Some(list) = tree.child_by_field_name("list") {
                return rust_use_names(list, last, source, names);
            }
        }
        "use_list" => {
            let mut complete = true;
            for entry in named_entries(tree) {
                complete &= rust_use_names(entry, parent, source, names);
            }
            return complete;
        }
        _ => {}
    }
    true
}
