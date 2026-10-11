//! The module-scope name table the ECMAScript extractor consults: import
//! bindings, `require` bindings, declared names and reassigned names.

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use super::{Ctx, cjs};
use crate::shared::TextSpan;

/// What a bound name stands for in its source module.
pub(super) enum Kind {
    /// One named export of the module (`default` for a default import).
    Named(String),
    /// The whole module: a namespace import or `require` of a module.
    Whole,
}

pub(super) struct Binding {
    pub kind: Kind,
    pub source: String,
    /// `span` of the matching import record, for ES and TS imports.
    pub import_span: Option<TextSpan>,
    pub type_only: bool,
}

pub(super) enum Resolved<'t> {
    Binding(&'t Binding),
    /// A module-scope declaration of the same name hides the import.
    Shadowed,
    /// The name is assigned somewhere it is not shadowed.
    Rebound,
    Unbound,
}

#[derive(Default)]
pub(super) struct Table {
    es: HashMap<String, Binding>,
    cjs: HashMap<String, Binding>,
    locals: HashSet<String>,
    rebound: HashSet<String>,
    /// `let`/`var` names initialized from a bare identifier.
    mutable_copies: HashMap<String, String>,
}

const FUNCTIONS: &[&str] = &[
    "function_declaration",
    "function_expression",
    "function",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
];

pub(super) fn is_function(node: Node<'_>) -> bool {
    FUNCTIONS.contains(&node.kind())
}

impl Table {
    pub(super) fn build(ctx: &Ctx<'_>, root: Node<'_>) -> Self {
        let mut table = Table::default();
        let mut cursor = root.walk();
        let statements: Vec<Node<'_>> = root.children(&mut cursor).collect();
        for statement in &statements {
            if statement.kind() == "import_statement" {
                table.add_import(ctx, *statement);
            }
        }
        for statement in &statements {
            let mut names = Vec::new();
            declared_names(ctx, *statement, &mut names);
            table.locals.extend(names);
        }
        let require_free = !table.is_declared("require");
        for statement in &statements {
            table.add_declarations(ctx, *statement, require_free);
        }
        table
    }

    /// Records assignments once the whole tree is scanned.
    pub(super) fn add_writes(&mut self, ctx: &Ctx<'_>, writes: &[Node<'_>]) {
        for write in writes {
            let target = match write.kind() {
                "update_expression" => write.child_by_field_name("argument"),
                _ => write.child_by_field_name("left"),
            };
            let Some(target) = target else { continue };
            let mut names = Vec::new();
            pattern_names(ctx, target, &mut names);
            for name in names {
                if !shadowed_below_program(ctx, *write, &name) {
                    self.rebound.insert(name);
                }
            }
        }
    }

    fn add_import(&mut self, ctx: &Ctx<'_>, statement: Node<'_>) {
        let source = statement.child_by_field_name("source").map(|node| ctx.string_value(node)).unwrap_or_default();
        let statement_type = has_anonymous(statement, "type");
        let mut cursor = statement.walk();
        for child in statement.children(&mut cursor) {
            match child.kind() {
                "import_clause" => {
                    let clause_type = statement_type || has_anonymous(child, "type");
                    let mut inner = child.walk();
                    for part in child.children(&mut inner) {
                        match part.kind() {
                            "identifier" => self.insert_es(ctx, part, Kind::Named("default".into()), &source, clause_type),
                            "namespace_import" => {
                                let mut ns = part.walk();
                                if let Some(ident) = part.children(&mut ns).find(|c| c.kind() == "identifier") {
                                    self.insert_es(ctx, ident, Kind::Whole, &source, clause_type);
                                }
                            }
                            "named_imports" => {
                                let mut specs = part.walk();
                                for spec in part.children(&mut specs).filter(|c| c.kind() == "import_specifier") {
                                    let Some(name) = spec.child_by_field_name("name") else { continue };
                                    let local = spec.child_by_field_name("alias").unwrap_or(name);
                                    let kind = Kind::Named(ctx.name_text(name));
                                    self.insert_es(ctx, local, kind, &source, clause_type || has_anonymous(spec, "type"));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "import_require_clause" => {
                    let mut inner = child.walk();
                    let ident = child.children(&mut inner).find(|c| c.kind() == "identifier");
                    let module = child.child_by_field_name("source").map(|node| ctx.string_value(node));
                    if let (Some(ident), Some(module)) = (ident, module) {
                        self.insert_es(ctx, ident, Kind::Whole, &module, statement_type);
                    }
                }
                _ => {}
            }
        }
    }

    fn insert_es(&mut self, ctx: &Ctx<'_>, local: Node<'_>, kind: Kind, source: &str, type_only: bool) {
        let binding = Binding {
            kind,
            source: source.to_owned(),
            import_span: ctx.import_span(local),
            type_only,
        };
        self.es.entry(ctx.text(local).to_owned()).or_insert(binding);
    }

    fn add_declarations(&mut self, ctx: &Ctx<'_>, statement: Node<'_>, require_free: bool) {
        let statement = unwrap_declaration(statement);
        if !matches!(statement.kind(), "lexical_declaration" | "variable_declaration") {
            return;
        }
        let mutable = statement.kind() == "variable_declaration" || ctx.text(statement).starts_with("let");
        let mut cursor = statement.walk();
        for declarator in statement.children(&mut cursor).filter(|c| c.kind() == "variable_declarator") {
            let (Some(name), Some(value)) =
                (declarator.child_by_field_name("name"), declarator.child_by_field_name("value"))
            else {
                continue;
            };
            if name.kind() == "identifier" && value.kind() == "identifier" && mutable {
                self.mutable_copies.insert(ctx.text(name).to_owned(), ctx.text(value).to_owned());
            }
            if !require_free {
                continue;
            }
            let Some(cjs::Require::Literal(source)) = cjs::require_call(ctx, value) else { continue };
            match name.kind() {
                "identifier" => {
                    self.locals.remove(ctx.text(name));
                    self.cjs.insert(
                        ctx.text(name).to_owned(),
                        Binding { kind: Kind::Whole, source, import_span: None, type_only: false },
                    );
                }
                "object_pattern" => {
                    let mut inner = name.walk();
                    for part in name.children(&mut inner) {
                        let (local, original) = match part.kind() {
                            "shorthand_property_identifier_pattern" => (part, part),
                            "pair_pattern" => match (part.child_by_field_name("value"), part.child_by_field_name("key")) {
                                (Some(local), Some(key)) if local.kind() == "identifier" => (local, key),
                                _ => continue,
                            },
                            _ => continue,
                        };
                        self.locals.remove(ctx.text(local));
                        self.cjs.insert(
                            ctx.text(local).to_owned(),
                            Binding {
                                kind: Kind::Named(ctx.name_text(original)),
                                source: source.clone(),
                                import_span: None,
                                type_only: false,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Whether a module-scope import, declaration or `require` binding
    /// carries `name`.
    pub(super) fn is_declared(&self, name: &str) -> bool {
        self.es.contains_key(name) || self.locals.contains(name) || self.cjs.contains_key(name)
    }

    pub(super) fn has_binding(&self, name: &str) -> bool {
        self.es.contains_key(name) || self.cjs.contains_key(name)
    }

    pub(super) fn resolve_es(&self, name: &str) -> Resolved<'_> {
        let Some(binding) = self.es.get(name) else { return Resolved::Unbound };
        if self.locals.contains(name) {
            Resolved::Shadowed
        } else if self.rebound.contains(name) {
            Resolved::Rebound
        } else {
            Resolved::Binding(binding)
        }
    }

    pub(super) fn resolve_any(&self, name: &str) -> Resolved<'_> {
        match self.cjs.get(name) {
            Some(_) if self.rebound.contains(name) => Resolved::Rebound,
            Some(binding) => Resolved::Binding(binding),
            None => self.resolve_es(name),
        }
    }

    /// For `let b = a` with `a` an import: the import `b` copies, when `b`
    /// is also reassigned.
    pub(super) fn rebound_copy_of_import(&self, name: &str) -> bool {
        self.mutable_copies
            .get(name)
            .is_some_and(|origin| self.es.contains_key(origin.as_str()) && self.rebound.contains(name))
    }
}

pub(super) fn has_anonymous(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| !child.is_named() && child.kind() == kind)
}

fn unwrap_declaration(statement: Node<'_>) -> Node<'_> {
    match statement.kind() {
        "export_statement" => statement.child_by_field_name("declaration").unwrap_or(statement),
        _ => statement,
    }
}

/// Names a statement declares in its own scope.
pub(super) fn declared_names(ctx: &Ctx<'_>, statement: Node<'_>, out: &mut Vec<String>) {
    let statement = unwrap_declaration(statement);
    match statement.kind() {
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = statement.walk();
            for declarator in statement.children(&mut cursor).filter(|c| c.kind() == "variable_declarator") {
                if let Some(name) = declarator.child_by_field_name("name") {
                    pattern_names(ctx, name, out);
                }
            }
        }
        "function_declaration"
        | "generator_function_declaration"
        | "class_declaration"
        | "abstract_class_declaration"
        | "enum_declaration"
        | "interface_declaration"
        | "type_alias_declaration"
        | "function_signature"
        | "import_alias" => {
            let name = statement.child_by_field_name("name").or_else(|| {
                let mut cursor = statement.walk();
                statement.children(&mut cursor).find(|c| c.kind() == "identifier")
            });
            if let Some(name) = name {
                out.push(ctx.text(name).to_owned());
            }
        }
        "internal_module" => {
            if let Some(name) = statement.child_by_field_name("name") {
                let mut first = name;
                while let Some(object) = first.child_by_field_name("object") {
                    first = object;
                }
                out.push(ctx.text(first).to_owned());
            }
        }
        "ambient_declaration" => {
            let mut cursor = statement.walk();
            for child in statement.children(&mut cursor) {
                declared_names(ctx, child, out);
            }
        }
        _ => {}
    }
}

/// The identifiers a binding or assignment pattern introduces.
pub(super) fn pattern_names(ctx: &Ctx<'_>, node: Node<'_>, out: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => out.push(ctx.text(node).to_owned()),
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                pattern_names(ctx, left, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                pattern_names(ctx, value, out);
            }
        }
        "required_parameter" | "optional_parameter" => {
            if let Some(pattern) = node.child_by_field_name("pattern") {
                pattern_names(ctx, pattern, out);
            }
        }
        "object_pattern" | "array_pattern" | "rest_pattern" | "formal_parameters" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                pattern_names(ctx, child, out);
            }
        }
        _ => {}
    }
}

/// Whether a scope between `node` and the program declares `name`.
pub(super) fn shadowed_below_program(ctx: &Ctx<'_>, node: Node<'_>, name: &str) -> bool {
    let mut current = node.parent();
    while let Some(scope) = current {
        if scope.kind() == "program" {
            return false;
        }
        let mut names = Vec::new();
        if is_function(scope) {
            if let Some(parameters) = scope.child_by_field_name("parameters") {
                pattern_names(ctx, parameters, &mut names);
            }
            if let Some(parameter) = scope.child_by_field_name("parameter") {
                pattern_names(ctx, parameter, &mut names);
            }
            if scope.kind() != "function_declaration"
                && let Some(own) = scope.child_by_field_name("name")
            {
                names.push(ctx.text(own).to_owned());
            }
        }
        match scope.kind() {
            "statement_block" => {
                let mut cursor = scope.walk();
                for child in scope.children(&mut cursor) {
                    declared_names(ctx, child, &mut names);
                }
            }
            "catch_clause" => {
                if let Some(parameter) = scope.child_by_field_name("parameter") {
                    pattern_names(ctx, parameter, &mut names);
                }
            }
            _ => {}
        }
        if names.iter().any(|declared| declared == name) {
            return true;
        }
        current = scope.parent();
    }
    false
}
