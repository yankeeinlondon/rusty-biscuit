//! `ctx.recent_commits` and `recent_commits(count)` share one named type,
//! `recent-commits`, defined once in the function catalog. Neither declaration
//! may own the element type itself, so a change to the named type reaches
//! both consumers.

use darkmatter::markdown::compose::expression::{expression_function_catalog, function_descriptors};
use darkmatter::markdown::schemas::catalog::GlobalCatalog;
use darkmatter::markdown::schemas::{
    Constraint, PropertyAtom, PropertyDef, SimplifiedSchema, SimplifiedType, TypeExpr,
    parse_standalone_schema_document,
};

const SHARED: &str = "recent-commits";

fn single(def: &PropertyDef) -> &PropertyAtom {
    match def {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => panic!("expected a single atom: {def:?}"),
    }
}

/// The authored `types:` entries of a shipped partial, before any import is
/// resolved.
fn authored_types(source: &str, path: &str) -> darkmatter::markdown::schemas::SchemaShape {
    let document = parse_standalone_schema_document(source, path)
        .expect("the shipped partial parses")
        .expect("the shipped partial is a schema document");
    match document.schema() {
        Some(SimplifiedSchema::Single(shape)) => shape.clone(),
        other => panic!("{path}: expected a `types:` mapping, got {other:?}"),
    }
}

/// Whether `atom` names the shared type itself: a plain reference, not an
/// array of it and not an inline `string[]`.
fn references_shared(atom: &PropertyAtom, reference: &str) -> bool {
    !atom.is_array
        && matches!(&atom.ty, TypeExpr::Imported { name, reference: found } if name == SHARED && found == reference)
}

#[test]
fn ctx_recent_commits_references_the_catalog_named_type() {
    let types = authored_types(
        include_str!("../../../schemas/partials/context-variables.yaml"),
        "darkmatter/schemas/partials/context-variables.yaml",
    );
    let context = single(types.properties.get("context").expect("`context` type"));
    let TypeExpr::InlineObject(context) = &context.ty else { panic!("`context` is a shape: {context:?}") };
    let variable = single(context.properties.get("recent_commits").expect("ctx.recent_commits"));

    assert!(references_shared(variable, "darkmatter:functions"), "ctx.recent_commits owns its own type: {variable:?}");
    assert!(variable.constraints.contains(&Constraint::Generated), "{variable:?}");
    assert!(variable.constraints.contains(&Constraint::Required), "{variable:?}");
    let description = variable.description.as_deref().unwrap_or_default();
    assert!(description.contains("`recent_commits(count)`"), "{description}");
}

#[test]
fn recent_commits_returns_the_catalog_named_type() {
    let types = authored_types(
        include_str!("../../../schemas/partials/functions.yaml"),
        "darkmatter/schemas/partials/functions.yaml",
    );
    assert!(types.properties.contains_key(SHARED), "the catalog defines `{SHARED}`");

    let [descriptor] = function_descriptors("recent_commits") else { panic!("one `recent_commits` overload") };
    let returns = single(&descriptor.function_type.returns);
    assert!(references_shared(returns, "this"), "recent_commits owns its own return type: {returns:?}");
    assert!(descriptor.description.contains("`ctx.recent_commits`"), "{}", descriptor.description);
}

#[test]
fn the_shared_type_resolves_to_a_string_list_for_both_consumers() {
    let named = single(expression_function_catalog().named_type(SHARED).expect("`recent-commits` is a helper type"));
    assert!(named.is_array && named.ty == TypeExpr::Primitive(SimplifiedType::String), "{named:?}");

    let ctx = GlobalCatalog::darkmatter().global("ctx").and_then(|ctx| ctx.shape()).expect("ctx is a shape");
    let variable = single(ctx.properties.get("recent_commits").expect("ctx.recent_commits"));
    // The same shape the former inline `string[](generated; required)` had:
    // the use-site markers bind to the list, not to its items.
    assert!(variable.is_array && variable.ty == TypeExpr::Primitive(SimplifiedType::String), "{variable:?}");
    assert!(variable.constraints.is_empty(), "{variable:?}");
    assert_eq!(variable.array_constraints, [Constraint::Generated, Constraint::Required], "{variable:?}");
}
