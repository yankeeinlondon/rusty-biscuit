//! Contextual syntax markers of the languages without lexical binding rules.
//!
//! A broad reference capture also takes identifier-shaped tokens that are
//! built-in syntax, not names being read: Java's inferred-type `var`, a Lua
//! variable attribute (`<const>`, `<close>`), PHP's class-relative `self`,
//! `static` and `parent`, Swift's compiler-configuration identifiers (a
//! precedence group's `associativity: left`, `#available(iOS 13, *)`,
//! `#if os(macOS)`), and the options of a shell declaration builtin
//! (`declare -i`), C#'s `global` in `global::X`. Named-argument and tuple-element labels (`f(x=v)`,
//! `(x: 1)`) are listed here too: a label names a parameter or element, not
//! a binding in view. Such a token reads no binding, so it is never an
//! occurrence and never a suggestion for a same-named declaration.
//! [`syntax_markers`] lists them by grammar role; a role whose slot also
//! holds real names (a Java type slot, a PHP class operand, a shell
//! argument) is narrowed by the one spelling the language reserves there.
//! The same spelling anywhere else (a Java variable named `var`, a PHP
//! method named `static`, a Lua local named `const`) stays an occurrence.
//!
//! User contract: `tree-hugger/docs/references/language-support.md#syntax-markers-in-the-other-languages`.

use tree_sitter::Node;

use crate::file::parents::IndexedParent as _;
use crate::shared::ProgrammingLanguage;

/// The marker tokens one walk of a file collects ([`syntax_markers`]).
#[derive(Default)]
pub(crate) struct SyntaxMarkers<'tree> {
    nodes: Vec<Node<'tree>>,
    /// Java `var` in an inferred-type slot ([`java_inferred_type`]).
    java_var: Vec<Node<'tree>>,
    /// The file declares a type named `var`, or imports one with an ordinary
    /// (not `static`) single-type import ([`java_declared_type_name`]), which
    /// Java 10 and later reject: the source predates `var` inference, so a
    /// `var` in a type slot names that type.
    java_var_type: bool,
}

impl<'tree> SyntaxMarkers<'tree> {
    /// Every marker token of the file.
    pub(crate) fn nodes(&self) -> impl Iterator<Item = Node<'tree>> + '_ {
        let java_var: &[Node<'tree>] = if self.java_var_type { &[] } else { &self.java_var };
        self.nodes.iter().chain(java_var).copied()
    }
}

/// Pushes the marker tokens `node` itself holds (its own fields and direct
/// children) for a language without lexical rules. Called once per named
/// node of the tree.
pub(crate) fn syntax_markers<'tree>(
    node: Node<'tree>,
    language: ProgrammingLanguage,
    source: &str,
    markers: &mut SyntaxMarkers<'tree>,
) {
    let text = |node: Node<'_>| source.get(node.start_byte()..node.end_byte()).unwrap_or_default();
    markers.nodes.extend(argument_label(node, language));
    match language {
        ProgrammingLanguage::Java => {
            markers.java_var.extend(java_inferred_type(node).filter(|slot| text(*slot) == "var"));
            markers.java_var_type |= java_declared_type_name(node).is_some_and(|name| text(name) == "var");
        }
        // `global::X.Y`: the `global` alias names the global namespace, no
        // binding; an extern alias (`A::T`) is an identifier and reads.
        ProgrammingLanguage::CSharp if node.kind() == "alias_qualified_name" => markers.nodes.extend(
            node.child_by_field_name("alias").filter(|alias| alias.child(0).is_some_and(|token| token.kind() == "global")),
        ),
        ProgrammingLanguage::Lua if node.kind() == "attribute" => {
            let mut cursor = node.walk();
            markers.nodes.extend(node.named_children(&mut cursor).filter(|child| child.kind() == "identifier"));
        }
        ProgrammingLanguage::Php => markers.nodes.extend(
            php_class_operand(node).filter(|name| {
                ["self", "static", "parent"].iter().any(|keyword| text(*name).eq_ignore_ascii_case(keyword))
            }),
        ),
        ProgrammingLanguage::Swift => {
            swift_markers(node, source, &mut markers.nodes);
            swift_element_labels(node, &mut markers.nodes);
        }
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh if node.kind() == "declaration_command" => {
            shell_declaration_options(node, source, &mut markers.nodes);
        }
        _ => {}
    }
}

/// The bare `type_identifier` written in a slot where Java infers the type
/// from `var`: a local variable, an enhanced `for` variable, a resource, an
/// explicitly typed lambda parameter and a record pattern component.
///
/// Since Java 10 `var` is a reserved type name (no class, interface or type
/// parameter may be called `var`), so a bare `var` in these slots is always
/// the inference marker. Source written for Java 9 or earlier could declare
/// a type `var`; such a file declares or imports it
/// ([`java_declared_type_name`]), and then no `var` is a marker. Slots where
/// the language never infers (fields, method parameters, return types) are
/// left alone, so a legacy `var` type there stays a read.
fn java_inferred_type(node: Node<'_>) -> Option<Node<'_>> {
    let slot = match node.kind() {
        "local_variable_declaration" | "enhanced_for_statement" | "resource" => node.child_by_field_name("type"),
        "formal_parameter"
            if node
                .indexed_parent()
                .and_then(|list| list.indexed_parent())
                .is_some_and(|owner| owner.kind() == "lambda_expression") =>
        {
            node.child_by_field_name("type")
        }
        "record_pattern_component" => node.named_child(0),
        _ => None,
    };
    slot.filter(|slot| slot.kind() == "type_identifier")
}

/// The type name a Java declaration introduces into the file: a class,
/// interface, enum, record or annotation type, a type parameter, or the last
/// segment of an ordinary single-type import. A static import (`import
/// static p.T.var;`) is no evidence, even though it could also bring in a
/// member type: it names fields and methods alike, and only established
/// evidence turns the inference markers of a whole file into type reads.
/// A `var` written as a qualifier of an import (`import p.var.T;`) or an
/// on-demand import names no type `var`.
fn java_declared_type_name(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "class_declaration" | "interface_declaration" | "enum_declaration" | "record_declaration"
        | "annotation_type_declaration" => node.child_by_field_name("name"),
        "type_parameter" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).find(|child| child.kind() == "type_identifier")
        }
        "import_declaration" => {
            let mut cursor = node.walk();
            if node.children(&mut cursor).any(|child| child.kind() == "static") {
                return None;
            }
            let mut children = node.named_children(&mut cursor);
            let path = children.find(|child| matches!(child.kind(), "identifier" | "scoped_identifier"))?;
            if children.any(|child| child.kind() == "asterisk") {
                return None;
            }
            path.child_by_field_name("name").or(Some(path))
        }
        _ => None,
    }
}

/// The bare class `name` of a PHP type (`named_type`), of `new` and of the
/// right side of `instanceof`. PHP reserves `self`, `static` and `parent`
/// as class names (case-insensitively), so in these operands they denote
/// the class around the code; a method may still be named after them.
fn php_class_operand(node: Node<'_>) -> Option<Node<'_>> {
    let operand = match node.kind() {
        "named_type" | "object_creation_expression" => node.named_child(0),
        "binary_expression"
            if node
                .child_by_field_name("operator")
                .is_some_and(|operator| operator.kind() == "instanceof") =>
        {
            node.child_by_field_name("right")
        }
        _ => None,
    };
    operand.filter(|operand| operand.kind() == "name")
}

/// The label of a named argument `node` writes: Python `f(x=v)`
/// (`keyword_argument`), C# `f(x: v)` and `[A(x: v)]`, PHP `f(x: v)`, Swift
/// `f(x: v)` and the labels of a function reference (`f(x:)`), Scala
/// `f(x = v)`, a Go field key in a `struct{…}{X: v}` literal, and the Perl
/// word before a fat comma (`x => v`, which Perl quotes). A label names a parameter of the callee (or a hash key), not a
/// binding in view, so it is no read; the value after it stays one.
///
/// Scala writes a named argument as an `assignment_expression` directly in
/// the `arguments`; a parenthesized one (`f((x = 2))`) is an assignment and
/// keeps its target. A C# `X = v` in an attribute or object initializer
/// assigns a member, not a label, and stays an occurrence.
fn argument_label(node: Node<'_>, language: ProgrammingLanguage) -> Option<Node<'_>> {
    let label = match (language, node.kind()) {
        (ProgrammingLanguage::Python, "keyword_argument")
        | (ProgrammingLanguage::CSharp, "argument")
        | (ProgrammingLanguage::Php, "argument") => node.child_by_field_name("name"),
        (ProgrammingLanguage::CSharp, "attribute_argument") => {
            node.named_child(0).filter(|name| name.next_sibling().is_some_and(|colon| colon.kind() == ":"))
        }
        (ProgrammingLanguage::Swift, "value_argument_label") => node.named_child(0),
        (ProgrammingLanguage::Scala, "assignment_expression")
            if node.indexed_parent().is_some_and(|parent| parent.kind() == "arguments") =>
        {
            node.child_by_field_name("left")
        }
        (ProgrammingLanguage::Go, "keyed_element") if go_struct_literal_element(node) => {
            node.child_by_field_name("key").and_then(|key| key.named_child(0))
        }
        (ProgrammingLanguage::Perl, "call_expression_with_bareword")
            if node.next_sibling().is_some_and(|next| next.kind() == "fat_comma") =>
        {
            node.named_child(0)
        }
        _ => None,
    };
    label.filter(|label| matches!(label.kind(), "identifier" | "simple_identifier" | "name"))
}

/// Whether a Go `keyed_element` is an element of a composite literal whose
/// written type is a `struct` type literal, where its key is a field name.
/// Under a named type (`T{X: 1}`) the grammar cannot tell a struct field
/// from a map key or array index expression, which reads a binding, so
/// that key stays an occurrence.
fn go_struct_literal_element(element: Node<'_>) -> bool {
    element
        .indexed_parent()
        .filter(|body| body.kind() == "literal_value")
        .and_then(|body| body.indexed_parent())
        .filter(|literal| literal.kind() == "composite_literal")
        .and_then(|literal| literal.child_by_field_name("type"))
        .is_some_and(|written| written.kind() == "struct_type")
}

/// The element labels of a Swift tuple: in an expression `(x: v)` and a type
/// `(x: Int)` (the grammar's `name` field) and in a tuple pattern
/// `case (x: let a, y: _)`, where the label is a `simple_identifier` followed
/// by `:` and a `pattern`. A dictionary key (`[k: v]`) is followed by an
/// expression and stays a read.
fn swift_element_labels<'tree>(node: Node<'tree>, markers: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    match node.kind() {
        "tuple_expression" | "tuple_type_item" => markers.extend(
            node.children_by_field_name("name", &mut cursor)
                .filter(|name| name.kind() == "simple_identifier"),
        ),
        _ => {
            // Siblings read from one child list: each `next_sibling` would
            // be a descent from the root.
            let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
            for (idx, child) in children.iter().enumerate() {
                let after = &children[idx + 1..];
                if child.is_named()
                    && child.kind() == "simple_identifier"
                    && after.first().is_some_and(|colon| colon.kind() == ":")
                    && after.iter().find(|next| next.is_named()).is_some_and(|pattern| pattern.kind() == "pattern")
                {
                    markers.push(*child);
                }
            }
        }
    }
}

/// The Swift identifiers that configure the compiler rather than name a
/// declaration:
///
/// - a precedence group attribute's directive (`associativity`,
///   `assignment`, `higherThan`, `lowerThan`) and an `associativity` value
///   (`left`, `right`, `none`). The values of `higherThan` and `lowerThan`
///   name other precedence groups and stay reads; `assignment`'s value is a
///   `boolean_literal`, never captured.
/// - every identifier of a pre-Swift 3 operator body
///   (`{ associativity left precedence 140 }`).
/// - the platform names of `#available(iOS 13, *)`.
/// - the name of a built-in attribute ([`SWIFT_BUILTIN_ATTRIBUTES`]), every
///   identifier argument of the attributes whose arguments are all keywords
///   ([`SWIFT_MARKER_ARGUMENTS`]: `@available(iOS 13, deprecated)`,
///   `@objc(run:)`, `@convention(c)`), and an attribute argument's label
///   (`@derivative(of: f)`). Other arguments are expressions and stay reads
///   (`@derivative(of: f)`'s `f`, a property wrapper's argument).
/// - the operands of an `#if`/`#elseif` condition (`os(macOS)`, `DEBUG`,
///   `canImport(UIKit)`): platforms, build flags and modules, never an
///   in-file binding.
fn swift_markers<'tree>(node: Node<'tree>, source: &str, markers: &mut Vec<Node<'tree>>) {
    let text = |node: Node<'_>| source.get(node.start_byte()..node.end_byte()).unwrap_or_default();
    let mut cursor = node.walk();
    let mut identifiers = node.named_children(&mut cursor).filter(|child| child.kind() == "simple_identifier");
    match node.kind() {
        "precedence_group_attribute" => {
            let Some(directive) = identifiers.next() else {
                return;
            };
            markers.push(directive);
            if text(directive) == "associativity" {
                markers.extend(identifiers.next());
            }
        }
        "deprecated_operator_declaration_body" | "directive" => markers.extend(identifiers),
        "attribute" => {
            let builtin = node.named_child(0).and_then(swift_builtin_attribute(source));
            markers.extend(builtin);
            let all_arguments = builtin.is_some_and(|name| SWIFT_MARKER_ARGUMENTS.contains(&text(name)));
            markers.extend(identifiers.filter(|argument| {
                all_arguments || argument.next_sibling().is_some_and(|colon| colon.kind() == ":")
            }));
        }
        "availability_condition" => {
            let mut cursor = node.walk();
            let platforms = node.named_children(&mut cursor).filter(|child| child.kind() == "identifier");
            markers.extend(platforms.filter_map(|platform| platform.named_child(0)));
        }
        _ => {}
    }
}

/// The non-type attributes the Swift compiler declares (its `DeclAttr.def`
/// and `TypeAttr.def`), as of Swift 5.10, the language the pinned
/// tree-sitter-swift 0.7.1 grammar parses (it has no Swift 6 `sending`).
/// The compiler matches these spellings before looking up a custom
/// attribute, so `@objc` never names a declaration in view; every
/// `_`-prefixed attribute is built in as well. Spellings that also name a
/// real type stay reads: `@Sendable` is the `Sendable` protocol's name and
/// `@MainActor` a global actor type.
const SWIFT_BUILTIN_ATTRIBUTES: &[&str] = &[
    "GKInspectable",
    "IBAction",
    "IBDesignable",
    "IBInspectable",
    "IBOutlet",
    "IBSegueAction",
    "LLDBDebuggerFunction",
    "NSApplicationMain",
    "NSCopying",
    "NSManaged",
    "UIApplicationMain",
    "attached",
    "autoclosure",
    "available",
    "backDeployed",
    "convention",
    "derivative",
    "differentiable",
    "discardableResult",
    "dynamicCallable",
    "dynamicMemberLookup",
    "escaping",
    "exclusivity",
    "freestanding",
    "frozen",
    "globalActor",
    "inlinable",
    "inline",
    "main",
    "noDerivative",
    "nonobjc",
    "objc",
    "objcMembers",
    "preconcurrency",
    "propertyWrapper",
    "requires_stored_property_inits",
    "resultBuilder",
    "retroactive",
    "specialize",
    "storageRestrictions",
    "testable",
    "transpose",
    "unchecked",
    "unknown",
    "usableFromInline",
    "warn_unqualified_access",
];

/// The built-in attributes whose identifier arguments are all keywords or
/// selector pieces, never expressions: `@available(iOS 13, *)`,
/// `@objc(run:with:)`, `@convention(c)`, `@inline(never)`,
/// `@exclusivity(unchecked)`.
const SWIFT_MARKER_ARGUMENTS: &[&str] = &["available", "convention", "exclusivity", "inline", "objc"];

/// The `type_identifier` of an attribute name (`user_type`) that is one
/// segment spelled as a built-in attribute ([`SWIFT_BUILTIN_ATTRIBUTES`] or
/// `_`-prefixed).
fn swift_builtin_attribute(source: &str) -> impl Fn(Node<'_>) -> Option<Node<'_>> + '_ {
    move |name| {
        if name.kind() != "user_type" || name.named_child_count() != 1 {
            return None;
        }
        name.named_child(0).filter(|segment| {
            let spelling = source.get(segment.start_byte()..segment.end_byte()).unwrap_or_default();
            segment.kind() == "type_identifier"
                && (spelling.starts_with('_') || SWIFT_BUILTIN_ATTRIBUTES.contains(&spelling))
        })
    }
}

/// The options of a shell declaration builtin (`declare -i`, `local -r`,
/// `export -n`, `typeset +x`): the leading `word` arguments that start with
/// `-` or `+`, through a closing `--`, by the builtins' own option rule. A
/// later word is an operand naming a variable or function, a declaration
/// name (`super::declaration_names`), never a marker or a read.
fn shell_declaration_options<'tree>(node: Node<'tree>, source: &str, markers: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    markers.extend(node.named_children(&mut cursor).take(shell_option_count(node, source)));
}

/// How many of `declaration`'s leading named children are options (see
/// [`shell_declaration_options`]); the operands follow them.
pub(crate) fn shell_option_count(declaration: Node<'_>, source: &str) -> usize {
    let mut cursor = declaration.walk();
    let mut count = 0;
    for argument in declaration.named_children(&mut cursor) {
        let word = source.get(argument.start_byte()..argument.end_byte()).unwrap_or_default();
        if argument.kind() != "word" || word.len() < 2 || !(word.starts_with('-') || word.starts_with('+')) {
            break;
        }
        count += 1;
        if word == "--" {
            break;
        }
    }
    count
}
