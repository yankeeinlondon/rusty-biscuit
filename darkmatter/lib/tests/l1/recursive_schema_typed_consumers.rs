//! Typed consumers (subtyping, projections, the call check, catalogs) on recursive references.
//!
//! Each consumer sees a recursive type as a named reference into the
//! resolution's definition table. These tests pin that every one of them
//! terminates and answers through the definition: the subtype relation, the
//! design-time call check (directly and as composition's precheck), nested
//! frontmatter lookups, and a global catalog's declaration.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, CallCheck, DeclaredTypes, TypeScope, check_expression_calls, parse_spanned,
};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::simplified::serialize_property_def;
use darkmatter::markdown::schemas::{
    PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, TypeExpr, is_definition_subtype_with_definitions,
    is_subtype_with_definitions, nested_property_shape, property_def_at_path, property_def_at_path_with_definitions,
};
use serde_json::json;
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

/// A self-recursive tree, a recursive union, and a mutually recursive pair of
/// unions.
const TYPES: &str = "kind: schema
types:
  node: \"{ label: string(required), children: node[](required) }\"
  data: [string, number, \"{ items: data[] }\"]
  branch: [string, \"{ leaves: leaf[] }\"]
  leaf: [number, \"{ up: branch }\"]
";

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).expect("write fixture");
}

/// Resolves `schema` (an inline `$schema` mapping) in `dir`, next to the
/// shared `types.yaml`.
fn resolve(dir: &Path, schema: &str) -> ResolvedSchema {
    write(dir, "types.yaml", TYPES);
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &FileResolutionContext::new(dir)).expect("fixture schema resolves")
}

fn root(resolved: &ResolvedSchema) -> &SchemaShape {
    match &resolved.simplified {
        Some(SimplifiedSchema::Single(shape)) => shape,
        other => panic!("expected a single shape, got {other:?}"),
    }
}

fn atom<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a PropertyAtom {
    match &root(resolved).properties[name] {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => panic!("`{name}` is a union"),
    }
}

mod subtyping {
    use super::*;

    const SCHEMA: &str = "tree: node(required)@./types.yaml
other: node@./types.yaml
value: data@./types.yaml
text: string
count: number
flag: boolean
b: branch@./types.yaml
l: leaf@./types.yaml
";

    /// Without the definition table `string ⊆ data` is unprovable, since
    /// `data` names its alternatives only through its body.
    #[test]
    fn a_recursive_union_is_compared_through_its_definition() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), SCHEMA);
        let defs = &resolved.definitions;
        let subtype = |a: &str, b: &str| is_subtype_with_definitions(atom(&resolved, a), atom(&resolved, b), defs);

        assert!(subtype("text", "value"), "string is one of data's alternatives");
        assert!(subtype("count", "value"), "number is one of data's alternatives");
        assert!(!subtype("flag", "value"), "boolean is not");
        assert!(!subtype("value", "text"), "data's object arm is not text");
        assert!(subtype("value", "value"));

        let (_, data) = defs.iter().find(|(key, _)| key.name == "data").expect("data is recursive");
        let value = &root(&resolved).properties["value"];
        assert!(is_definition_subtype_with_definitions(value, &data.body, defs), "data ⊆ its body");
        assert!(is_definition_subtype_with_definitions(&data.body, value, defs), "its body ⊆ data");
    }

    /// Two uses of one definition are one type whatever their use-site
    /// markers; a self- and a mutually recursive pair terminate with the
    /// relation's answers. Without the table every scalar here fails to
    /// reach `branch` and `leaf` through their bodies.
    #[test]
    fn self_and_mutually_recursive_references_terminate_with_identity_answers() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), SCHEMA);
        let defs = &resolved.definitions;
        let subtype = |a: &str, b: &str| is_subtype_with_definitions(atom(&resolved, a), atom(&resolved, b), defs);

        assert!(subtype("tree", "other"), "required and optional uses of node are one type");
        assert!(!subtype("tree", "text"));
        assert!(!subtype("text", "tree"));

        assert!(subtype("b", "b") && subtype("l", "l"));
        assert!(subtype("text", "b") && subtype("count", "l"));
        assert!(!subtype("count", "b") && !subtype("text", "l"));
        assert!(!subtype("b", "l") && !subtype("l", "b"));
        assert!(!subtype("b", "value") && !subtype("value", "b"));
    }
}

mod call_check {
    use super::*;

    const SCHEMA: &str = "tree: node(required)@./types.yaml\nvalue: data(required)@./types.yaml\n";

    fn check(resolved: &ResolvedSchema, expression: &str) -> CallCheck {
        let declared = DeclaredTypes::new(resolved.simplified.as_ref()).with_definitions(&resolved.definitions);
        let expr = parse_spanned(expression).expect("expression parses");
        let mut checks = check_expression_calls(&expr, &TypeScope::unguarded(&declared));
        assert!(!checks.is_empty(), "{expression}: no call");
        checks.remove(0)
    }

    /// Without the table the call check cannot see through `node`, and every
    /// call it reaches is category 4.
    #[test]
    fn a_recursive_object_is_category_one_for_an_object_and_five_for_a_number() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), SCHEMA);

        let keys = check(&resolved, "keys(tree)");
        assert_eq!(keys.category(), ArgumentCategory::Compatible, "{keys:#?}");

        let min = check(&resolved, "min(tree, 5)");
        assert_eq!(min.category(), ArgumentCategory::Incompatible, "{min:#?}");
        let message = min.rejection_message();
        assert!(message.contains("is node, expected"), "the argument is named, not expanded: {message}");
    }

    /// `data` is text, a number, or an object: some values convert to a
    /// number and some cannot, which is category 3, never a hang or a guess.
    #[test]
    fn a_recursive_union_is_judged_by_its_alternatives() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), SCHEMA);

        assert_eq!(check(&resolved, "min(value, 5)").category(), ArgumentCategory::Uncertain);
        assert_eq!(check(&resolved, "length(value)").category(), ArgumentCategory::Compatible);
    }

    #[test]
    fn the_projected_type_of_a_recursive_property_is_its_named_reference() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), SCHEMA);
        let declared = DeclaredTypes::new(resolved.simplified.as_ref()).with_definitions(&resolved.definitions);

        let types = declared.top_level_types();
        let darkmatter::markdown::compose::expression::Shape::Arms { arms, nullable } = &types["tree"] else {
            panic!("tree has a type: {:?}", types["tree"]);
        };
        assert!(!nullable);
        assert!(matches!(&arms[..], [arm] if matches!(&arm.ty, TypeExpr::Ref(key) if key.name == "node")), "{arms:?}");
        assert!(!declared.definitions().is_empty());
    }
}

#[cfg(feature = "effects-instrumentation")]
mod composition {
    use std::sync::Arc;

    use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
    use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
    use darkmatter::markdown::{Markdown, MarkdownError};

    use super::*;

    const FRONTMATTER: &str = "$schema:
  tree: node(required)@./types.yaml
tree:
  label: root
  children:
    - label: a
      children: []
";

    fn compose(dir: &Path, body: &str) -> Result<Markdown, MarkdownError> {
        write(dir, "types.yaml", TYPES);
        let path = dir.join("doc.md");
        std::fs::write(&path, format!("---\n{FRONTMATTER}before: \"{{{{ effect() }}}}\"\n---\n{{{{ effect() }}}}\n{body}\n"))
            .unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
            .with_source_file(path.clone());
        Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request_at(dir, options))
            .map(|(composed, _)| composed)
    }

    fn effect() -> TestFunctionGuard {
        let registration = FunctionRegistration::builder("effect", "function(parameters(void); returns(boolean))")
            .build()
            .expect("the effect signature compiles");
        hooks::register(registration, Arc::new(|_| Ok(json!(true))))
    }

    /// Without the table the call is category 4, which composes, and `min`
    /// runs and fails at evaluation instead.
    #[test]
    fn a_category_five_recursive_argument_is_rejected_before_evaluation() {
        let dir = TempDir::new().unwrap();
        let _effect = effect();
        hooks::reset_handler_entries();

        let error = compose(dir.path(), "{{ min(tree, 5) }}").expect_err("min(tree, 5) is rejected");

        let check = error.rejected_call().unwrap_or_else(|| panic!("no call check: {error}"));
        assert_eq!(check.function, "min");
        assert!(check.is_rejected());
        assert_eq!(hooks::handler_entries("effect"), 0, "a sibling expression ran");
        assert_eq!(hooks::handler_entries("min"), 0, "min ran");
    }

    #[test]
    fn a_category_one_recursive_argument_composes() {
        let dir = TempDir::new().unwrap();
        let _effect = effect();

        let composed = compose(dir.path(), "{{ length(keys(tree)) }}").unwrap_or_else(|error| panic!("{error}"));

        assert!(composed.content().trim_end().ends_with('2'), "{}", composed.content());
    }
}

mod lookups {
    use super::*;

    /// Without the table a lookup stops at the first reference.
    #[test]
    fn a_nested_lookup_follows_one_definition_per_segment_at_depth_four() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), "tree: node(required)@./types.yaml\n");
        let shape = root(&resolved);
        let path = ["tree", "children", "children", "children", "label"];

        let label = property_def_at_path_with_definitions(shape, &json!({}), &path, &resolved.definitions)
            .expect("the label four levels down resolves");
        assert_eq!(serialize_property_def(&label), "string(required)");
        let children = property_def_at_path_with_definitions(shape, &json!({}), &path[..4], &resolved.definitions)
            .expect("the children list resolves");
        assert_eq!(serialize_property_def(&children), "node[](required)", "a back-reference keeps its name");

        assert!(property_def_at_path(shape, &json!({}), &path).is_none());
        assert!(nested_property_shape(shape, &json!({}), &path[..1]).is_none());
    }
}

mod catalogs {
    use super::*;

    const CATALOG: &str = "kind: schema\n$schema:\n  tree: node@./types.yaml -> the document tree\n";

    /// Before the table was kept, the reference was misread as a union and
    /// the catalog failed to load.
    #[test]
    fn a_recursive_global_loads_and_describes_itself_by_name() {
        let catalog = GlobalCatalog::from_documents(
            "catalog.yaml",
            &[("catalog.yaml", CATALOG), ("types.yaml", TYPES)],
            &[GlobalRequirement::shape("tree")],
        )
        .unwrap_or_else(|error| panic!("{error}"));

        let tree = catalog.global("tree").expect("tree is declared");
        assert_eq!(serialize_property_def(tree.definition()), "node -> the document tree");
        assert_eq!(tree.description(), Some("the document tree"));
        let shape = tree.shape().expect("a recursive object global has members");
        assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["label", "children"]);
        assert_eq!(serialize_property_def(&shape.properties["children"]), "node[](required)");
        assert_eq!(tree.definitions().len(), 1);
    }

    #[test]
    fn a_recursive_global_that_is_not_an_object_is_refused() {
        let catalog = "kind: schema\n$schema:\n  tree: data@./types.yaml\n";
        let error = GlobalCatalog::from_documents(
            "catalog.yaml",
            &[("catalog.yaml", catalog), ("types.yaml", TYPES)],
            &[GlobalRequirement::shape("tree")],
        )
        .expect_err("a recursive union is not an object shape");
        assert_eq!(error.field, "$schema.tree");
    }
}

mod shell_template_holes {
    use std::sync::Arc;

    use darkmatter::markdown::compose::expression::{ExpressionError, Polarity, parse};
    use darkmatter::markdown::compose::shell_expansion::template::{
        CommandTemplate, HoleError, RejectReason, fill_template, parse_authored,
    };
    use darkmatter::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
    use darkmatter::markdown::{Markdown, MarkdownError};
    use serde_json::Value;

    use super::*;

    /// A program no `PATH` holds: a value the hole accepts reaches target
    /// resolution, which refuses it; a value it refuses never gets there.
    const MISSING: &str = "dm-recursive-hole-missing-program";

    /// `command` typed against `resolved` with its definitions, on the true
    /// side of `guard` when there is one.
    fn typed(resolved: &ResolvedSchema, command: &str, guard: Option<&str>) -> CommandTemplate {
        let declared = DeclaredTypes::new(resolved.simplified.as_ref()).with_definitions(&resolved.definitions);
        let site = TypeScope::unguarded(&declared);
        let site = match guard {
            Some(guard) => site.assume(&parse(guard).unwrap(), Polarity::True),
            None => site,
        };
        let mut template = parse_authored(command).unwrap();
        template.project_types(&site);
        template
    }

    fn values(values: Value) -> impl Fn(&str) -> Result<Value, ExpressionError> {
        move |expression: &str| Ok(values.get(expression).cloned().unwrap_or(Value::Null))
    }

    /// `data` is text, a number, or an object holding more `data`. The
    /// validator is checked at fill time, after the scope is gone, so it owns
    /// the definitions; without them every value of `data` is refused. An
    /// object is still never a hole's value.
    #[test]
    fn a_recursive_union_hole_fills_with_a_scalar_alternative_and_refuses_others() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), "value: data(required)@./types.yaml\n");
        let template = typed(&resolved, "tool {{ value }}", None);
        assert_eq!(template.placeholders().next().unwrap().typing.declared_text(), "data(required)");

        for (value, text) in [(json!(3), "3"), (json!("a b"), "a b")] {
            let filled = fill_template(&template, &values(json!({ "value": value })), None)
                .unwrap_or_else(|error| panic!("{value}: {error:?}"));
            assert_eq!(filled.segments[0].args, [text]);
        }
        for value in [json!(true), json!({ "items": [1] })] {
            let error = fill_template(&template, &values(json!({ "value": value })), None).unwrap_err();
            assert!(matches!(error, HoleError::Rejected { .. }), "{value}: {error:?}");
        }
    }

    /// A guard keeps the reference arm; the narrowed type owns the
    /// definitions too, so a value it admits passes the refinement.
    #[test]
    fn a_guard_narrowed_recursive_hole_judges_the_reference_by_its_body() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), "value: data@./types.yaml\n");
        let template = typed(&resolved, "tool {{ value }}", Some("is_number(value)"));
        assert!(template.placeholders().next().unwrap().typing.refinement.is_some());

        let filled = fill_template(&template, &values(json!({ "value": 3 })), None).unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(filled.segments[0].args, ["3"]);
        let error = fill_template(&template, &values(json!({ "value": false })), None).unwrap_err();
        assert!(
            matches!(error, HoleError::Rejected { reason: RejectReason::DeclaredType, .. }),
            "{error:?}"
        );
    }

    struct ApproveShape;

    impl ShellApprovalHandler for ApproveShape {
        fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowTemplatePersist)
        }
    }

    /// A scalar alternative of the recursive union fills a document's hole,
    /// so the command reaches its target (missing, so composition fails
    /// there). A value no alternative accepts never reaches the hole in a
    /// document: frontmatter validation refuses it first, so the refusal is
    /// pinned at the template API above.
    #[test]
    fn a_document_hole_typed_by_a_recursive_union_fills_with_a_scalar() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "types.yaml", TYPES);
        let path = dir.path().join("doc.md");
        let body = format!("---\n$schema:\n  value: data(required)@./types.yaml\nvalue: 3\n---\n::shell {MISSING} {{{{ value }}}}\n");
        std::fs::write(&path, body).unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
            .with_source_file(path.clone())
            .with_shell_policy_root(dir.path())
            .with_shell_working_directory(dir.path())
            .with_shell_approval_handler(Arc::new(ApproveShape));

        let error = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request_at(dir.path(), options))
            .map(|_| ())
            .expect_err("the program is missing");

        assert!(
            matches!(&error, MarkdownError::ShellExpansion(inner) if matches!(**inner, ShellExpansionError::TemplateTarget { .. })),
            "3 is a `data` and fills the hole: {error:?}"
        );
    }
}

mod shell_template_preflight {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots;
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};

    use super::*;

    /// Preflight types a document's holes on its own and validates a late
    /// value against that typing, so its declared layer needs the
    /// definitions as composition's does: without them `3` is refused.
    #[test]
    fn a_late_recursive_union_value_passes_preflight_validation() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "types.yaml", TYPES);
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "---\n$schema:\n  value: data@./types.yaml\n---\n::shell tool {{ value }}\n").unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let preflight = |value: serde_json::Value| {
            let options = ComposeOptions::new()
                .with_context(ComposeContext::capture_for_content(dir.path(), ""))
                .with_source_file(&path)
                .with_deferred_shell_roots(DeferredShellRoots::new(["value"]))
                .with_deferred_shell_value_validation(true)
                .with_data_overrides(json!({ "value": value }));
            markdown.compose_preflight(&crate::request_support::request_at(dir.path(), options))
        };

        preflight(json!(3)).unwrap_or_else(|error| panic!("3 is a `data`: {error}"));
    }
}

mod scoped_types {
    use darkmatter::markdown::compose::expression::parse;

    use super::*;

    /// A shape keeps a recursive alternative as its named reference and has
    /// no table to judge it by, so it admits a value through it rather than
    /// refusing every value of the type.
    #[test]
    fn a_recursive_reference_arm_admits_a_value_of_its_type() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve(dir.path(), "value: data(required)@./types.yaml\n");
        let declared = DeclaredTypes::new(resolved.simplified.as_ref()).with_definitions(&resolved.definitions);

        let ty = TypeScope::unguarded(&declared).type_of_expr(&parse("value").unwrap()).expect("value is typed");

        assert!(ty.admits(&json!(3)), "{ty:?}");
        assert!(!ty.admits(&serde_json::Value::Null), "a required reference is present");
    }
}
