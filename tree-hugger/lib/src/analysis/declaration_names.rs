//! Declaration-name sites of the languages without lexical binding rules.
//!
//! A reference capture on a token that declares a name is no reference,
//! whether or not the locals query reports that declaration as a record.
//! [`declaration_names`] lists those tokens by grammar shape, one decision
//! per node kind and language, so occurrence exclusion never depends on
//! which declarations became records. Rust and TS/JS read their binding
//! sites from [`super::scope::LanguageRules::binding_sites`] instead.
//!
//! A declared name written as a path names every component it declares, not
//! one span: [`declared_name_tokens`] turns a package, module or namespace
//! path (`package a.b`, `namespace A.B`, `namespace A::inline B`) into each
//! of its name tokens, and a C++ qualified type name (`union A::U {}`) into
//! its last one, whose qualifier stays a read. Both the reported records'
//! names (`definition_ranges`, every language, Rust and TS/JS included) and
//! this inventory go through it.
//!
//! The pattern readers are shared with the relation tree's Python and
//! Scala binding sites (`file/tree_file/relations/lexical.rs`).
//!
//! User contract: `tree-hugger/docs/references/language-support.md#declaration-names-in-the-other-languages`.

use tree_sitter::Node;

use crate::file::parents::IndexedParent as _;
use crate::file::extract::{next_syntax_sibling, prev_syntax_sibling};
use crate::shared::ProgrammingLanguage;

/// Pushes the name tokens `node` itself declares (its own fields and direct
/// children, plus the names of a pattern it introduces) for a language
/// without lexical rules. Called once per named node of the tree.
pub(crate) fn declaration_names<'tree>(
    node: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
    names: &mut Vec<Node<'tree>>,
) {
    match language {
        ProgrammingLanguage::C => c_type_specifier(node, names),
        ProgrammingLanguage::Java => java(node, names),
        ProgrammingLanguage::Scala => scala(node, source, names),
        ProgrammingLanguage::Go => go(node, names),
        ProgrammingLanguage::Python => python(node, source, names),
        ProgrammingLanguage::Cpp => cpp(node, names),
        ProgrammingLanguage::CSharp => csharp(node, names),
        ProgrammingLanguage::Swift => swift(node, names),
        ProgrammingLanguage::Php => php(node, names),
        ProgrammingLanguage::Perl => perl(node, names),
        ProgrammingLanguage::Lua => lua(node, names),
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => shell(node, source, names),
        // Rust and TS/JS have binding rules.
        _ => {}
    }
}

/// Name kinds that write a declared path: every component is a name the
/// declaration (re)opens, so each one is a declaration token (TS
/// `nested_identifier`, C++ `nested_namespace_specifier`, C#
/// `qualified_name`, Java `scoped_identifier`, Scala `package_identifier`,
/// Perl `package_name`, PHP `namespace_name`, a Swift module path
/// `identifier`). They reach [`declared_name_tokens`] only as a
/// declaration's own name, never as a qualified read.
const DECLARED_PATH_KINDS: [&str; 8] = [
    "nested_identifier",
    "nested_namespace_specifier",
    "qualified_name",
    "scoped_identifier",
    "package_identifier",
    "package_name",
    "namespace_name",
    "identifier",
];

/// The name tokens of the declared name `name`: itself for a single token,
/// every component of a declared path ([`DECLARED_PATH_KINDS`]), the last
/// name of a C++ `qualified_identifier` (`A::U` in `union A::U {}` declares
/// `U`; `A` reads the enclosing scope).
pub(crate) fn declared_name_tokens(name: Node<'_>) -> Vec<Node<'_>> {
    if name.kind() == "qualified_identifier" {
        let mut last = name;
        while let Some(inner) = last.child_by_field_name("name") {
            last = inner;
        }
        return vec![last];
    }
    if !DECLARED_PATH_KINDS.contains(&name.kind()) || name.named_child_count() == 0 {
        return vec![name];
    }
    let mut tokens = Vec::new();
    let mut stack = vec![name];
    while let Some(node) = stack.pop() {
        if node.named_child_count() == 0 {
            tokens.push(node);
        } else {
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
    }
    tokens
}

/// A C/C++ `struct`, `union`, `enum` or `class` specifier's name when the
/// specifier declares it: it has a body, or it is written alone as a
/// forward or opaque declaration (`struct S;`, `enum class E : int;`, a
/// member `enum class E;`). An elaborated use (`struct S x;`, a parameter
/// `struct S *p`) reads, and so does a template specialization's
/// `S<int>`, which names the primary template.
fn c_type_specifier<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    if !matches!(node.kind(), "struct_specifier" | "union_specifier" | "enum_specifier" | "class_specifier") {
        return;
    }
    let declares = node.child_by_field_name("body").is_some()
        || node.indexed_parent().is_some_and(|parent| match parent.kind() {
            "declaration" | "field_declaration" => {
                parent.child_by_field_name("type").is_some_and(|ty| ty.id() == node.id())
                    && parent.child_by_field_name("declarator").is_none()
            }
            "translation_unit" | "declaration_list" | "field_declaration_list" | "template_declaration"
            | "linkage_specification" => true,
            _ => false,
        });
    if let Some(name) = node.child_by_field_name("name").filter(|name| declares && name.kind() != "template_type") {
        names.push(name);
    }
}

fn field<'tree>(node: Node<'tree>, name: &str, names: &mut Vec<Node<'tree>>) {
    names.extend(node.child_by_field_name(name));
}

/// The direct named children of `node` of one of `kinds`.
fn children_of<'tree>(node: Node<'tree>, kinds: &[&str]) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).filter(|child| kinds.contains(&child.kind())).collect()
}

/// The first direct named child of `kind`.
fn first_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    children_of(node, &[kind]).into_iter().next()
}

/// The direct child of `kind` written immediately before the anonymous
/// token `token` (C# `let j =`, `join k in`).
fn child_before<'tree>(node: Node<'tree>, kind: &str, token: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
    children
        .windows(2)
        .find(|pair| pair[0].kind() == kind && !pair[1].is_named() && pair[1].kind() == token)
        .map(|pair| pair[0])
}

fn has_token(node: Node<'_>, token: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| !child.is_named() && child.kind() == token)
}

fn java<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    match node.kind() {
        // `<T extends Bound>`: the name is the parameter's own
        // `type_identifier`; bounds sit inside `type_bound`.
        "type_parameter" => names.extend(first_child(node, "type_identifier")),
        // `instanceof P p` names its binder in `name`; the plain form has none.
        "compact_constructor_declaration"
        | "resource"
        | "annotation_type_declaration"
        | "annotation_type_element_declaration"
        | "module_declaration"
        | "instanceof_expression" => field(node, "name", names),
        // `package a.b;`: the path is an unfielded child after any annotations.
        "package_declaration" => names.extend(children_of(node, &["identifier", "scoped_identifier"])),
        "labeled_statement" | "record_pattern_component" | "type_pattern" => {
            names.extend(first_child(node, "identifier"));
        }
        _ => {}
    }
}

fn scala<'tree>(node: Node<'tree>, source: &str, names: &mut Vec<Node<'tree>>) {
    let mut bind = |name: Node<'tree>| names.push(name);
    match node.kind() {
        // `[X, Y] =>> F[X]` declares its parameters like a type parameter list.
        "type_parameters" | "type_lambda" => {
            let mut cursor = node.walk();
            names.extend(node.children_by_field_name("name", &mut cursor));
        }
        // `[T: Ordering as ord]` names the context parameter; a named
        // tuple type's `(x: Int)` names its element.
        "context_bound" | "name_and_type" => field(node, "name", names),
        "covariant_type_parameter" | "contravariant_type_parameter" | "type_definition" | "given_definition"
        | "package_clause" | "package_object" => {
            field(node, "name", names);
        }
        // `import a.{b => bb, c as cc}`: the alias binds; the imported name reads.
        "arrow_renamed_identifier" | "as_renamed_identifier" => {
            names.extend(node.child_by_field_name("alias").filter(|alias| alias.kind() != "wildcard"));
        }
        "val_definition" | "var_definition" | "case_clause" => {
            scala_pattern_names(node.child_by_field_name("pattern"), source, &mut bind);
        }
        "enumerator" if has_token(node, "<-") || has_token(node, "=") => {
            scala_pattern_names(node.named_child(0), source, &mut bind);
        }
        _ => {}
    }
}

fn go<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    match node.kind() {
        "type_parameter_declaration" => {
            let mut cursor = node.walk();
            names.extend(node.children_by_field_name("name", &mut cursor));
        }
        "type_switch_statement" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                names.extend(children_of(alias, &["identifier"]));
            }
        }
        // `m := <-ch` declares; `m = <-ch` assigns.
        "receive_statement" if has_token(node, ":=") => {
            if let Some(left) = node.child_by_field_name("left") {
                names.extend(children_of(left, &["identifier"]));
            }
        }
        _ => {}
    }
}

fn python<'tree>(node: Node<'tree>, source: &str, names: &mut Vec<Node<'tree>>) {
    let mut ignored_writes = Vec::new();
    match node.kind() {
        "class_definition" | "function_definition" => {
            if let Some(list) = node.child_by_field_name("type_parameters") {
                python_type_parameters(list, names);
            }
        }
        "type_alias_statement" => {
            let declared = node
                .child_by_field_name("left")
                .and_then(|left| left.named_child(0));
            match declared {
                Some(name) if name.kind() == "identifier" => names.push(name),
                Some(generic) if generic.kind() == "generic_type" => {
                    names.extend(first_child(generic, "identifier"));
                    if let Some(list) = first_child(generic, "type_parameter") {
                        python_type_parameters(list, names);
                    }
                }
                _ => {}
            }
        }
        // `from __future__ import annotations, division as d`: like any
        // `from` import it binds the feature (or alias) name; the reader
        // reports no import for it.
        "future_import_statement" => {
            let mut cursor = node.walk();
            for name in node.children_by_field_name("name", &mut cursor) {
                match name.kind() {
                    "aliased_import" => field(name, "alias", names),
                    _ => names.extend(first_child(name, "identifier")),
                }
            }
        }
        "lambda_parameters" | "typed_parameter" => {
            names.extend(children_of(node, &["identifier"]));
            for splat in children_of(node, &["list_splat_pattern", "dictionary_splat_pattern"]) {
                names.extend(first_child(splat, "identifier"));
            }
        }
        "case_clause" => {
            for pattern in children_of(node, &["case_pattern"]) {
                python_captures(pattern, source, &mut |name| names.push(name));
            }
        }
        "assignment" | "for_statement" | "for_in_clause" => {
            python_targets(node.child_by_field_name("left"), &mut |name| names.push(name), &mut ignored_writes);
        }
        // `with … as target`, `except E as name`; a `case … as name` alias
        // is read with its pattern.
        "as_pattern" if !has_ancestor(node, "case_clause") => {
            python_targets(node.child_by_field_name("alias"), &mut |name| names.push(name), &mut ignored_writes);
        }
        _ => {}
    }
}

/// The names of a PEP 695 parameter list: `T`, the first type of a
/// bounded `T: Bound`, and the name of `*Ts`/`**P`.
fn python_type_parameters<'tree>(list: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    for parameter in children_of(list, &["type"]) {
        let Some(form) = parameter.named_child(0) else { continue };
        match form.kind() {
            "identifier" => names.push(form),
            "constrained_type" => {
                names.extend(first_child(form, "type").and_then(|name| first_child(name, "identifier")));
            }
            "splat_type" => names.extend(first_child(form, "identifier")),
            _ => {}
        }
    }
}

fn has_ancestor(node: Node<'_>, kind: &str) -> bool {
    let mut current = node.indexed_parent();
    while let Some(ancestor) = current {
        if ancestor.kind() == kind {
            return true;
        }
        current = ancestor.indexed_parent();
    }
    false
}

fn cpp<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    c_type_specifier(node, names);
    match node.kind() {
        "optional_type_parameter_declaration" | "namespace_alias_definition" | "namespace_definition" => {
            field(node, "name", names);
        }
        "type_parameter_declaration" | "variadic_type_parameter_declaration" => {
            names.extend(first_child(node, "type_identifier"));
        }
        "for_range_loop" => {
            names.extend(node.child_by_field_name("declarator").filter(|name| name.kind() == "identifier"));
        }
        "structured_binding_declarator" => names.extend(children_of(node, &["identifier"])),
        "lambda_capture_initializer" => field(node, "left", names),
        // `~C` declares the destructor when it is a function's declarator,
        // written inline or qualified (`C::~C`); an explicit `obj.~C()`
        // call names it.
        "destructor_name" => {
            let declarator = node.indexed_parent().and_then(|parent| match parent.kind() {
                "function_declarator" => Some(parent),
                "qualified_identifier" => parent.indexed_parent().filter(|outer| outer.kind() == "function_declarator"),
                _ => None,
            });
            if declarator.is_some() {
                names.extend(first_child(node, "identifier"));
            }
        }
        _ => {}
    }
}

fn csharp<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    match node.kind() {
        // A `static` constructor has no record; an event with accessors
        // (`event Action E { add {} remove {} }`, also `IFace.E`, whose
        // interface is an unfielded read) has none either.
        "catch_declaration" | "declaration_pattern" | "recursive_pattern" | "delegate_declaration"
        | "destructor_declaration" | "from_clause" | "tuple_element" | "constructor_declaration"
        | "event_declaration" | "extern_alias_directive" | "namespace_declaration"
        | "file_scoped_namespace_declaration" => field(node, "name", names),
        "labeled_statement" | "join_into_clause" | "query_continuation" => {
            names.extend(first_child(node, "identifier"));
        }
        "let_clause" => names.extend(child_before(node, "identifier", "=")),
        "join_clause" => names.extend(child_before(node, "identifier", "in")),
        _ => {}
    }
}

fn swift<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    {
        let mut cursor = node.walk();
        names.extend(node.children_by_field_name("bound_identifier", &mut cursor));
    }
    match node.kind() {
        // `func g(label local: Int)`: the external label is declared with
        // the parameter; `_` declares that it has none.
        "parameter" => field(node, "external_name", names),
        // The grammar fields both the name and the type as `name`.
        "lambda_parameter" => {
            field(node, "external_name", names);
            let mut cursor = node.walk();
            names.extend(
                node.children_by_field_name("name", &mut cursor)
                    .filter(|name| name.kind() == "simple_identifier"),
            );
        }
        // `<each T>` declares; `repeat each T` reads.
        "type_parameter_pack" if node.indexed_parent().is_some_and(|parent| parent.kind() == "type_parameter") => {
            names.extend(first_child(node, "user_type").and_then(|user| first_child(user, "type_identifier")));
        }
        // `[k = o]` declares `k`; `[weak self]` captures an existing name.
        "capture_list_item" if node.child_by_field_name("value").is_some() => field(node, "name", names),
        "computed_setter" | "willset_clause" | "didset_clause" | "enum_type_parameters" => {
            names.extend(children_of(node, &["simple_identifier"]));
        }
        "pattern" if swift_pattern_binds(node) => names.extend(children_of(node, &["simple_identifier"])),
        // The grammar leaves both names unfielded; a precedence group's
        // `higherThan: Q` sits inside its attributes and reads.
        "macro_declaration" | "precedence_group_declaration" => names.extend(first_child(node, "simple_identifier")),
        // `import Foo.Sub` imports the module path; a selective import
        // (`import struct Foo.Bar`) binds only `Bar`, which is its import
        // record, and `Foo` reads.
        "import_declaration" if swift_import_selector(node).is_none() => {
            names.extend(first_child(node, "identifier"));
        }
        _ => {}
    }
}

/// The keyword of a Swift selective import (`import struct Foo.Bar`):
/// `typealias`, `struct`, `class`, `enum`, `protocol`, `let`, `var` or
/// `func`; `None` for a whole-module import.
pub(crate) fn swift_import_selector(import: Node<'_>) -> Option<&'static str> {
    const SELECTORS: [&str; 8] = ["typealias", "struct", "class", "enum", "protocol", "let", "var", "func"];
    let mut cursor = import.walk();
    import
        .children(&mut cursor)
        .filter(|child| !child.is_named())
        .find_map(|child| SELECTORS.iter().copied().find(|selector| *selector == child.kind()))
}

/// The node kinds a Scala import path segment is written as: the flat
/// children of an `import_declaration` between its `.` tokens, an ordinary
/// or backticked name or a symbolic one (`+` in `import a.+.C` or
/// `import a.b.+`).
pub(crate) const SCALA_IMPORT_SEGMENT_KINDS: &[&str] = &["identifier", "operator_identifier"];

/// Whether `node` is a segment of a Scala import path: a
/// [`SCALA_IMPORT_SEGMENT_KINDS`] child of an `import_declaration`.
pub(crate) fn is_scala_import_segment(node: Node<'_>) -> bool {
    SCALA_IMPORT_SEGMENT_KINDS.contains(&node.kind())
        && node.indexed_parent().is_some_and(|parent| parent.kind() == "import_declaration")
}

/// The first segment of the Scala import path `segment` belongs to
/// (`a` for `b` or `C` in `import a.b.C`, `c` for `d` in `import a.b, c.d`),
/// or `None` when `segment` is no import path segment. The pinned grammar
/// writes every path as flat segment/`.` children of the
/// `import_declaration`, so a path runs back to the previous `,` or the
/// `import` keyword; a comment between segments is part of neither and
/// never ends the path.
pub(crate) fn scala_import_path_start(segment: Node<'_>) -> Option<Node<'_>> {
    if !is_scala_import_segment(segment) {
        return None;
    }
    let mut first = segment;
    let mut current = prev_syntax_sibling(segment);
    while let Some(previous) =
        current.filter(|previous| previous.kind() == "." || SCALA_IMPORT_SEGMENT_KINDS.contains(&previous.kind()))
    {
        if previous.kind() != "." {
            first = previous;
        }
        current = prev_syntax_sibling(previous);
    }
    Some(first)
}

/// Whether the Scala import path segment `segment` is a name its import
/// binds: the last segment of a `,`-separated expression (a segment
/// followed by `.` and an unbraced rename, `b` in `a.b.C as D`, is a
/// qualifier: the rename binds), or the last
/// segment of a path that opens a wildcard (`a.b.*`, `a.b._`, `a.b.given`)
/// or a selector group holding a wildcard or a `given` selector
/// (`a.b.{_, given T}`). Every other segment is a qualifier. Comments
/// between the segment and what follows it are skipped, so they never
/// decide the binding.
pub(crate) fn scala_import_binds(segment: Node<'_>) -> bool {
    if !is_scala_import_segment(segment) {
        return false;
    }
    let Some(next) = next_syntax_sibling(segment) else { return true };
    match next.kind() {
        "," => true,
        "." => next_syntax_sibling(next).is_some_and(|opened| match opened.kind() {
            "namespace_wildcard" => true,
            "namespace_selectors" => {
                let mut cursor = opened.walk();
                opened.children(&mut cursor).any(|selector| matches!(selector.kind(), "namespace_wildcard" | "given"))
            }
            _ => false,
        }),
        _ => false,
    }
}

/// Whether a bare name in this Swift pattern binds rather than matches an
/// existing value: some pattern around it carries `let`/`var`, or the
/// pattern is a declaration's name, a `for` item, a `catch` pattern, or an
/// `if`/`guard`/`while case` pattern written after `let`/`var`.
fn swift_pattern_binds(pattern: Node<'_>) -> bool {
    let mut top = pattern;
    let mut current = Some(pattern);
    while let Some(node) = current.filter(|node| node.kind() == "pattern") {
        if first_child(node, "value_binding_pattern").is_some() {
            return true;
        }
        top = node;
        current = node.indexed_parent();
    }
    let Some(parent) = top.indexed_parent() else { return false };
    let field_of_top = (0..parent.child_count())
        .find(|&index| parent.child(index as u32).is_some_and(|child| child.id() == top.id()))
        .and_then(|index| parent.field_name_for_child(index as u32));
    match (parent.kind(), field_of_top) {
        ("property_declaration", Some("name")) | ("for_statement", Some("item")) | (_, Some("error")) => true,
        ("if_statement" | "guard_statement" | "while_statement", _) => {
            let mut sibling = top.prev_sibling();
            while let Some(node) = sibling {
                match node.kind() {
                    "value_binding_pattern" => return true,
                    "case" | "," => return false,
                    _ => sibling = node.prev_sibling(),
                }
            }
            false
        }
        _ => false,
    }
}

fn php<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    // A promoted parameter may be written by reference (`&$p`).
    let variable_name = |variable: Option<Node<'tree>>| {
        variable
            .and_then(|variable| if variable.kind() == "by_ref" { first_child(variable, "variable_name") } else { Some(variable) })
            .and_then(|variable| first_child(variable, "name"))
    };
    match node.kind() {
        "catch_clause" | "static_variable_declaration" | "property_promotion_parameter" | "variadic_parameter" => {
            names.extend(variable_name(node.child_by_field_name("name")));
        }
        "namespace_definition" => field(node, "name", names),
        // `[$a, 'k' => $b, $k => $c] = …` and `list(…)`: every element
        // variable is a target, except a key written before `=>`.
        "list_literal" => {
            let mut cursor = node.walk();
            let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
            for (index, child) in children.iter().enumerate() {
                let is_key = children.get(index + 1).is_some_and(|next| next.kind() == "=>");
                let target = match child.kind() {
                    "variable_name" if !is_key => Some(*child),
                    "by_ref" => first_child(*child, "variable_name"),
                    _ => None,
                };
                names.extend(variable_name(target));
            }
        }
        _ => {}
    }
}

fn perl<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    const VARIABLES: [&str; 3] = ["scalar_variable", "array_variable", "hash_variable"];
    match node.kind() {
        // `for my $x (…)` / `foreach our $x (…)`: the variable after the
        // scope declares; `for $x (…)` reuses (localizes) an existing one.
        "for_statement_2" => {
            let mut cursor = node.walk();
            let children: Vec<Node<'tree>> = node.named_children(&mut cursor).collect();
            names.extend(
                children
                    .windows(2)
                    .find(|pair| pair[0].kind() == "scope" && VARIABLES.contains(&pair[1].kind()))
                    .map(|pair| pair[1]),
            );
        }
        "function_signature" => names.extend(children_of(node, &VARIABLES)),
        "package_statement" => names.extend(first_child(node, "package_name")),
        _ => {}
    }
}

fn lua<'tree>(node: Node<'tree>, names: &mut Vec<Node<'tree>>) {
    match node.kind() {
        // `{ field = v }` declares the key; `{ [key] = v }` computes it.
        "field" if node.child(0).is_some_and(|first| first.kind() != "[") => field(node, "name", names),
        "label_statement" => names.extend(first_child(node, "identifier")),
        _ => {}
    }
}

/// The names a Python assignment target binds; an attribute whose object
/// is a dotted name is a qualified write (`writes`), any other attribute
/// and a subscript bind nothing.
pub(crate) fn python_targets<'a>(target: Option<Node<'a>>, bind: &mut impl FnMut(Node<'a>), writes: &mut Vec<Node<'a>>) {
    let Some(target) = target else { return };
    let mut stack = vec![target];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier" => bind(node),
            "attribute" if is_dotted_name(node.child_by_field_name("object")) => writes.push(node),
            "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list" | "expression_list"
            | "parenthesized_expression" | "list_splat_pattern" | "list_splat" | "as_pattern_target" => {
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
            _ => {}
        }
    }
}

/// Whether a Python expression is a plain dotted name (`a`, `a.b.c`): no
/// call, subscript or other expression a write's owner path could pass
/// through.
fn is_dotted_name(node: Option<Node<'_>>) -> bool {
    let mut current = node;
    while let Some(node) = current {
        match node.kind() {
            "identifier" => return true,
            "attribute" => current = node.child_by_field_name("object"),
            _ => return false,
        }
    }
    false
}

/// The capture names of a Python `case` pattern: a bare name (one-part
/// dotted name), an `as` alias, a star or double-star name; a dotted name,
/// a class pattern's class and a keyword pattern's keyword bind nothing.
pub(crate) fn python_captures<'a>(pattern: Node<'a>, source: &str, bind: &mut impl FnMut(Node<'a>)) {
    let mut stack = vec![pattern];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        let children: Vec<Node<'a>> = node.named_children(&mut cursor).collect();
        match node.kind() {
            "dotted_name" => {
                let binds = node.indexed_parent().is_some_and(|parent| matches!(parent.kind(), "case_pattern" | "keyword_pattern"))
                    && children.len() == 1
                    && source.get(node.start_byte()..node.end_byte()) != Some("_");
                if binds {
                    bind(children[0]);
                }
            }
            "class_pattern" => stack.extend(children.into_iter().filter(|child| child.kind() != "dotted_name")),
            "keyword_pattern" => stack.extend(children.into_iter().skip(1)),
            "as_pattern" | "splat_pattern" => {
                for child in children {
                    if child.kind() == "identifier" {
                        bind(child);
                    } else {
                        stack.push(child);
                    }
                }
            }
            _ => stack.extend(children),
        }
    }
}

/// The variable names a Scala pattern binds. Scala decides by the lexical
/// form, not by lookup: a simple identifier pattern is a variable when it
/// is a `varid` (first character a lower-case letter or `_`; a backquoted
/// or upper-case one is a stable identifier pattern, a reference). A
/// binder's name (`x @ p`) binds whatever its spelling: Scala 2.13's
/// grammar writes it `varid`, scalac binds any identifier, and binding it
/// can only leave a base unresolved, never resolve it wrongly.
pub(crate) fn scala_pattern_names<'a>(pattern: Option<Node<'a>>, source: &str, bind: &mut impl FnMut(Node<'a>)) {
    let Some(pattern) = pattern else { return };
    let mut stack = vec![pattern];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier" => {
                let is_varid = source
                    .get(node.start_byte()..node.end_byte())
                    .and_then(|text| text.chars().next())
                    .is_some_and(|first| first == '_' || first.is_lowercase());
                if is_varid {
                    bind(node);
                }
            }
            "stable_identifier" | "type_identifier" | "wildcard" => {}
            _ => {
                let mut cursor = node.walk();
                for (index, child) in node.children(&mut cursor).enumerate() {
                    let field = node.field_name_for_child(index as u32);
                    if node.kind() == "capture_pattern" && field == Some("name") && child.kind() == "identifier" {
                        bind(child);
                    } else if child.is_named() && !matches!(field, Some("type" | "operator")) {
                        stack.push(child);
                    }
                }
            }
        }
    }
}

/// The operands of a shell declaration builtin after its options
/// (`local x`, `typeset +x y`, `export c`, `declare -f g`, zsh `integer i`):
/// each names the variable or function being declared, a declaration token
/// even though no record reports it (a record needs `x=value`). Bash lexes
/// an identifier operand as an uncaptured `variable_name`; what is left as
/// a `word` there (`my-func`, an operand after `--`) and every zsh operand
/// is listed here. Expansions (`local $v`) stay reads.
fn shell<'tree>(node: Node<'tree>, source: &str, names: &mut Vec<Node<'tree>>) {
    if node.kind() != "declaration_command" {
        return;
    }
    let mut cursor = node.walk();
    names.extend(
        node.named_children(&mut cursor)
            .skip(super::syntax_markers::shell_option_count(node, source))
            .filter(|operand| operand.kind() == "word"),
    );
}
