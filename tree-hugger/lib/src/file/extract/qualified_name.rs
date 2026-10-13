//! In-file qualified names: the enclosing named declarations of a symbol,
//! joined with the language's own separators, built from the file's source
//! alone. A file or naming-context prefix is a query input applied later and
//! never part of the name stored here (or of the ID derived from it).

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::*;
use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::count_shared_context_steps;

/// One extracted symbol, as the name builder sees it.
pub(crate) struct NamingInput<'tree> {
    pub name: &'tree str,
    pub kind: SymbolKind,
    pub name_node: Node<'tree>,
    pub declaration: Node<'tree>,
    pub anchor: Node<'tree>,
}

/// A symbol's in-file qualified name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedName {
    pub text: String,
    /// The outermost segment is a namespace or package the source declares.
    pub is_namespace_rooted: bool,
    /// The last segment is a namespace or package (PHP joins what follows
    /// with `\`).
    ends_in_namespace: bool,
}

/// Builds the qualified name of every symbol in `symbols`, in order.
///
/// A symbol's prefix is its nearest enclosing *naming* declaration: another
/// extracted function, type, enum variant, module or namespace (whose own
/// qualified name is reused), a Rust `impl` block (its self type's path as
/// written, see [`rust_impl_prefix`]), a named
/// node that is not extracted (a C# static constructor, a Java compact
/// record constructor), a Scala block package (`package p { … }`, a rooting
/// namespace segment), or a container the extractor knows but does not list
/// as a symbol (a Swift extension). The
/// walk ends at an unnamed owner (anonymous class, object literal, class
/// expression, Scala `given … with`), as the container search does, and at
/// the file root, where a file-level `package` or `namespace` statement
/// before the declaration roots the name.
pub(crate) fn qualified_names(
    symbols: &[NamingInput<'_>],
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<QualifiedName> {
    let mut owners: HashMap<usize, usize> = HashMap::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if is_naming_kind(symbol.kind) {
            for node in [symbol.declaration, symbol.anchor] {
                owners.entry(node.id()).or_insert(index);
            }
        }
    }
    // One climb per file, so the prefix walks stop at the root by ID rather
    // than asking every node for its parent's parent.
    let root_id = symbols.first().map_or(0, |symbol| {
        let mut root = symbol.anchor;
        while let Some(parent) = root.indexed_parent() {
            root = parent;
        }
        root.id()
    });
    let builder = Builder {
        symbols,
        owners,
        language,
        source,
        root_id,
        file_namespaces: OnceCell::new(),
    };
    let mut memo: Vec<Option<QualifiedName>> = vec![None; symbols.len()];
    (0..symbols.len())
        .map(|index| builder.qualified(index, &mut memo))
        .collect()
}

/// Kinds whose name qualifies the declarations inside them.
fn is_naming_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Type
            | SymbolKind::Class
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::EnumVariant
            | SymbolKind::Trait
            | SymbolKind::Module
            | SymbolKind::Namespace
    )
}

/// Declaration node kinds that state a namespace or package, rooting every
/// name inside or after them.
fn is_namespace_node_kind(kind: &str) -> bool {
    matches!(
        kind,
        "namespace_declaration"
            | "namespace_definition"
            | "file_scoped_namespace_declaration"
            | "package_declaration"
            | "package_clause"
            | "package_statement"
    )
}

/// What encloses a declaration, nearest first.
enum Prefix<'tree> {
    /// Another extracted symbol, by index.
    Symbol(usize),
    /// Named nodes that are not extracted symbols (outermost first); the
    /// walk continues from `node`.
    Named {
        segments: Vec<Segment>,
        node: Node<'tree>,
    },
    /// The file root, with the file-level namespace in force there.
    Root(Option<String>),
    /// An unnamed owner: nothing further qualifies the name.
    Unnamed,
}

struct Segment {
    text: String,
    is_namespace: bool,
}

struct Builder<'a, 'tree> {
    symbols: &'a [NamingInput<'tree>],
    owners: HashMap<usize, usize>,
    language: ProgrammingLanguage,
    source: &'a str,
    /// The ID of the symbols' tree root.
    root_id: usize,
    /// The file-level namespace in force at each child of the root, by node
    /// id; built on first use (see [`file_namespace_index`]).
    file_namespaces: OnceCell<HashMap<usize, Option<Rc<str>>>>,
}

impl<'tree> Builder<'_, 'tree> {
    fn qualified(&self, index: usize, memo: &mut Vec<Option<QualifiedName>>) -> QualifiedName {
        if let Some(done) = &memo[index] {
            return done.clone();
        }
        let symbol = &self.symbols[index];
        let mut segments = vec![Segment {
            text: symbol.name.to_string(),
            is_namespace: is_namespace_node_kind(member_slot(symbol.declaration).kind()),
        }];
        // A Lua `M:m` / `M.f` names its table; a Go method its receiver type.
        let mut is_lua_method = false;
        if self.language == ProgrammingLanguage::Lua
            && let Some((table, is_method)) = lua_table_of_name(symbol.name_node)
            && let Ok(text) = table.utf8_text(self.source.as_bytes())
        {
            is_lua_method = is_method;
            segments.insert(0, Segment { text: text.to_string(), is_namespace: false });
        }
        if self.language == ProgrammingLanguage::Go
            && symbol.declaration.kind() == "method_declaration"
            && let Some(receiver) = go_receiver_type_name(symbol.declaration, self.source)
        {
            segments.insert(0, Segment { text: receiver, is_namespace: false });
        }

        let own_is_namespace = segments.last().is_some_and(|segment| segment.is_namespace);
        let mut node = member_slot(symbol.declaration);
        let base = loop {
            // A Perl package name is absolute wherever it is written.
            if own_is_namespace && self.language == ProgrammingLanguage::Perl {
                break None;
            }
            match self.prefix(index, node) {
                Prefix::Symbol(owner) => break Some(self.qualified(owner, memo)),
                Prefix::Named { segments: named, node: next } => {
                    segments.splice(0..0, named);
                    node = next;
                }
                // A file-level namespace statement is not inside itself.
                Prefix::Root(_) if own_is_namespace && segments.len() == 1 => break None,
                Prefix::Root(namespace) => {
                    break namespace.map(|text| QualifiedName {
                        text,
                        is_namespace_rooted: true,
                        ends_in_namespace: true,
                    });
                }
                Prefix::Unnamed => break None,
            }
        };
        let result = self.join(base, segments, is_lua_method);
        memo[index] = Some(result.clone());
        result
    }

    /// The nearest prefix above `from`, skipping nodes that stand for the
    /// symbol itself.
    fn prefix(&self, index: usize, from: Node<'tree>) -> Prefix<'tree> {
        let mut child = from;
        while let Some(node) = child.indexed_parent()
            && node.id() != self.root_id
        {
            match self.owners.get(&node.id()) {
                Some(&owner) if owner != index => return Prefix::Symbol(owner),
                Some(_) => {}
                None if self.language == ProgrammingLanguage::Rust && node.kind() == "impl_item" => {
                    if let Some(prefix) = rust_impl_prefix(node, self.source) {
                        return prefix;
                    }
                }
                None if is_unnamed_type_owner(node) => return Prefix::Unnamed,
                // Scala package clauses are never extracted; a block-form one
                // names everything in its body, nested blocks included.
                None if is_scala_block_package(node, self.language) => {
                    if let Some(text) = namespace_name(node, self.source) {
                        let segments = vec![Segment { text, is_namespace: true }];
                        return Prefix::Named { segments, node };
                    }
                }
                None => {
                    if let Some(text) = self.unextracted_name(node) {
                        let segments = vec![Segment { text, is_namespace: false }];
                        return Prefix::Named { segments, node };
                    }
                    if let Some(container) = ancestor_container(node, self.language, self.source) {
                        // An ungrouped Go `type_spec` names the type whose
                        // symbol is extracted at its `type_declaration`.
                        if let ContainerLookup::Node { node: container_node, .. } = container.lookup
                            && let Some(&owner) = self.owners.get(&container_node.id())
                            && owner != index
                        {
                            return Prefix::Symbol(owner);
                        }
                        let segments = vec![Segment {
                            text: container.name,
                            is_namespace: is_namespace_node_kind(node.kind()),
                        }];
                        return Prefix::Named { segments, node };
                    }
                }
            }
            child = node;
        }
        Prefix::Root(self.file_namespace_at(child))
    }

    /// The namespace or package a file-level statement at or before `top`
    /// (the root, or a child of it) puts it in, read from the per-file
    /// index rather than by walking back over `top`'s preceding siblings.
    fn file_namespace_at(&self, top: Node<'tree>) -> Option<String> {
        if !has_file_level_namespaces(self.language) {
            return None;
        }
        let Some(root) = top.indexed_parent() else {
            // The root has no siblings: only its own statement could apply.
            return file_level_namespace(top, self.language, self.source);
        };
        let index = self
            .file_namespaces
            .get_or_init(|| file_namespace_index(root, self.language, self.source));
        count_shared_context_steps(1);
        index.get(&top.id()).cloned().flatten().map(|name| name.to_string())
    }

    /// The name of a declaration that encloses others but is not extracted
    /// as a symbol: a named function-like node such as a C# static
    /// constructor. A TypeScript `namespace` (`internal_module`) is extracted
    /// and reaches this only if its capture is missing.
    fn unextracted_name(&self, node: Node<'tree>) -> Option<String> {
        let names_its_body = node.kind() == "internal_module" || is_function_node_kind(node.kind());
        if !names_its_body {
            return None;
        }
        let name = node.child_by_field_name("name")?;
        let text = name.utf8_text(self.source.as_bytes()).ok()?;
        (!text.is_empty()).then(|| text.to_string())
    }

    /// Appends `segments` to `base` (or starts from the first segment).
    fn join(&self, base: Option<QualifiedName>, segments: Vec<Segment>, is_lua_method: bool) -> QualifiedName {
        let mut segments = segments.into_iter();
        let mut name = base.unwrap_or_else(|| {
            let first = segments.next().expect("a symbol has a name segment");
            QualifiedName {
                text: first.text,
                is_namespace_rooted: first.is_namespace,
                ends_in_namespace: first.is_namespace,
            }
        });
        let mut rest = segments.peekable();
        while let Some(segment) = rest.next() {
            let is_last = rest.peek().is_none();
            name.text.push_str(separator(self.language, name.ends_in_namespace, is_lua_method && is_last));
            name.text.push_str(&segment.text);
            name.ends_in_namespace = segment.is_namespace;
        }
        name
    }
}

/// The prefix a Rust `impl` block gives its members: the self type's path as
/// written (`impl right::Repeated` → `right::Repeated`), so a qualified self
/// type keeps the module that tells same-named types apart. The path is read
/// relative to the enclosing module: each leading `super` continues the walk
/// above one more enclosing `mod`, a leading `crate` roots it at the file,
/// and `self` is dropped.
fn rust_impl_prefix<'tree>(impl_item: Node<'tree>, source: &str) -> Option<Prefix<'tree>> {
    let path = impl_self_type_path(impl_item, source)?;
    let mut node = impl_item;
    let mut written = path.as_slice();
    while let Some((first, rest)) = written.split_first()
        && !rest.is_empty()
    {
        match first.as_str() {
            "crate" => {
                while let Some(parent) = node.indexed_parent() {
                    node = parent;
                }
            }
            "super" => {
                let mut ancestor = node.indexed_parent();
                while let Some(candidate) = ancestor
                    && candidate.kind() != "mod_item"
                {
                    ancestor = candidate.indexed_parent();
                }
                node = match ancestor {
                    Some(module) => module,
                    None => {
                        let mut root = node;
                        while let Some(parent) = root.indexed_parent() {
                            root = parent;
                        }
                        root
                    }
                };
            }
            "self" => {}
            _ => break,
        }
        written = rest;
    }
    let segments = written
        .iter()
        .map(|text| Segment { text: text.clone(), is_namespace: false })
        .collect();
    Some(Prefix::Named { segments, node })
}

/// The separator a language writes between a qualifier and the name after
/// it.
fn separator(language: ProgrammingLanguage, after_namespace: bool, is_lua_method: bool) -> &'static str {
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp | ProgrammingLanguage::Perl => "::",
        ProgrammingLanguage::Php if after_namespace => "\\",
        ProgrammingLanguage::Php => "::",
        ProgrammingLanguage::Lua if is_lua_method => ":",
        _ => ".",
    }
}

/// Whether `language` has a file-level `package`/`namespace` statement
/// form ([`file_level_namespace`]); a file in any other language needs no
/// namespace index.
fn has_file_level_namespaces(language: ProgrammingLanguage) -> bool {
    matches!(
        language,
        ProgrammingLanguage::Java
            | ProgrammingLanguage::Go
            | ProgrammingLanguage::CSharp
            | ProgrammingLanguage::Scala
            | ProgrammingLanguage::Php
            | ProgrammingLanguage::Perl
    )
}

/// The namespace or package in force at each child of `root`, by node id,
/// from one pass over the children: the nearest Java `package`, Go package
/// clause, C# file-scoped namespace, PHP `namespace X;` or Perl `package X;`
/// at or before the child (a later one replaces an earlier one), or every
/// Scala package clause at or before it, outermost first (`package outer`
/// then `package inner` is `outer.inner`: Scala clauses nest).
pub(crate) fn file_namespace_index(
    root: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> HashMap<usize, Option<Rc<str>>> {
    let mut index = HashMap::new();
    let mut in_force: Option<Rc<str>> = None;
    let mut scala_clauses = String::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if let Some(name) = file_level_namespace(child, language, source) {
            in_force = Some(if language == ProgrammingLanguage::Scala {
                if !scala_clauses.is_empty() {
                    scala_clauses.push('.');
                }
                scala_clauses.push_str(&name);
                scala_clauses.as_str().into()
            } else {
                name.into()
            });
        }
        index.insert(child.id(), in_force.clone());
    }
    count_shared_context_steps(index.len());
    index
}

/// Whether `node` is a Scala `package p { … }` clause: it scopes its body,
/// not the file statements after it (see [`file_level_namespace`]).
fn is_scala_block_package(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    language == ProgrammingLanguage::Scala
        && node.kind() == "package_clause"
        && node.child_by_field_name("body").is_some()
}

fn file_level_namespace(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> Option<String> {
    let statement = match (language, node.kind()) {
        (ProgrammingLanguage::Java, "package_declaration")
        | (ProgrammingLanguage::Go, "package_clause")
        | (ProgrammingLanguage::CSharp, "file_scoped_namespace_declaration") => node,
        (ProgrammingLanguage::Scala, "package_clause") if node.child_by_field_name("body").is_none() => node,
        (ProgrammingLanguage::Php, "namespace_definition") if node.child_by_field_name("body").is_none() => node,
        (ProgrammingLanguage::Perl, "package_statement") if find_child_by_kind(node, "block").is_none() => node,
        _ => return None,
    };
    namespace_name(statement, source)
}

/// The written name of a namespace or package statement.
pub(crate) fn namespace_name(statement: Node<'_>, source: &str) -> Option<String> {
    let name = statement.child_by_field_name("name").or_else(|| {
        let mut cursor = statement.walk();
        statement.named_children(&mut cursor).find(|child| {
            matches!(
                child.kind(),
                "scoped_identifier"
                    | "identifier"
                    | "package_identifier"
                    | "package_name"
                    | "namespace_name"
                    | "qualified_name"
            )
        })
    })?;
    let text = name.utf8_text(source.as_bytes()).ok()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}
