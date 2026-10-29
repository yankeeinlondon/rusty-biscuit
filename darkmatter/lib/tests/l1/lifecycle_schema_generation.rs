//! The generated lifecycle verb types in `darkmatter/schemas/partials/lifecycle.yaml`
//! match a fresh projection of Darkmatter's typed catalogs: every generated
//! type's parsed definition, every generated category and bundle key, and the
//! inlined catalog helper types. Regenerate with `just regen-lifecycle-schema`.
//!
//! The projection itself fails on missing side-effect metadata and on a new
//! name collision, and is deterministic.

use std::collections::{BTreeMap, BTreeSet};

use darkmatter::effects::effect_function_descriptors;
use darkmatter::lifecycle::schema_projection::{
    GeneratedRegion, LifecycleSchemaProjection, ProjectionCatalogs, ProjectionError, TEMPLATE_OPERAND, VerbOrigin,
    region_bounds, template_operand_type,
};
use darkmatter::markdown::compose::expression::{
    ExpressionFunctionDescriptor, expression_function_catalog, expression_function_descriptors,
};
use darkmatter::markdown::schemas::simplified::{
    PropertyAtom, PropertyDef, SimplifiedSchema, TypeExpr, parse_standalone_schema_document,
};

const PARTIAL: &str = "darkmatter/schemas/partials/lifecycle.yaml";

fn projection() -> LifecycleSchemaProjection {
    LifecycleSchemaProjection::shipped().expect("the shipped catalogs project")
}

/// The `types:` map of a lifecycle partial's text.
fn types_of(text: &str) -> indexmap::IndexMap<String, PropertyDef> {
    let document = parse_standalone_schema_document(text, PARTIAL)
        .expect("the partial parses")
        .expect("the partial is a schema document");
    match document.schema() {
        Some(SimplifiedSchema::Single(shape)) => shape.properties.clone(),
        other => panic!("`types:` is not one mapping: {other:?}"),
    }
}

/// The keys of category `name` and the type names each one references.
fn category_keys(types: &indexmap::IndexMap<String, PropertyDef>, name: &str) -> BTreeMap<String, Vec<String>> {
    let Some(PropertyDef::Single(PropertyAtom {
        ty: TypeExpr::InlineObject(shape),
        ..
    })) = types.get(name)
    else {
        panic!("`{name}` is not an object type");
    };
    shape
        .properties
        .iter()
        .map(|(key, def)| {
            let atoms = match def {
                PropertyDef::Single(atom) => std::slice::from_ref(atom),
                PropertyDef::Union(arms) => arms.as_slice(),
            };
            let names = atoms
                .iter()
                .map(|atom| match &atom.ty {
                    TypeExpr::Imported { name, .. } => name.clone(),
                    other => format!("{other:?}"),
                })
                .collect();
            (key.clone(), names)
        })
        .collect()
}

/// The checked-in file with only its generated regions replaced by a fresh
/// projection's: what `--write` would produce.
fn regenerated(checked_in: &str) -> String {
    projection().rewrite(checked_in).expect("the partial carries every region's markers")
}

#[test]
fn every_generated_type_matches_the_catalog_projection() {
    let checked_in = include_str!("../../../schemas/partials/lifecycle.yaml");
    let expected = types_of(&regenerated(checked_in));
    let actual = types_of(checked_in);
    let projection = projection();

    let generated: Vec<String> = std::iter::once(TEMPLATE_OPERAND.to_string())
        .chain(projection.helper_types.iter().map(|(name, _)| name.clone()))
        .chain(projection.verbs.iter().flat_map(|(_, verb)| {
            std::iter::once(verb.short_type_name()).chain(verb.long_type_name())
        }))
        .collect();
    assert!(generated.len() > 200, "{} generated types", generated.len());
    for name in &generated {
        assert_eq!(actual.get(name), expected.get(name), "`{name}` drifted; run `just regen-lifecycle-schema`");
    }
    assert_eq!(actual.get(TEMPLATE_OPERAND), Some(&template_operand_type()));
    for (_, verb) in &projection.verbs {
        assert_eq!(actual[&verb.short_type_name()], union_def(&verb.short), "{}", verb.name);
        if let (Some(name), Some(long)) = (verb.long_type_name(), &verb.long) {
            assert_eq!(actual[&name], union_def(long), "{}", verb.name);
        }
    }
    let actual_names: Vec<&String> = actual.keys().filter(|name| expected.contains_key(*name)).collect();
    assert_eq!(actual_names.len(), actual.len(), "the checked-in partial has types a regeneration drops");
}

fn union_def(arms: &[PropertyAtom]) -> PropertyDef {
    match arms {
        [atom] => PropertyDef::Single(atom.clone()),
        _ => PropertyDef::Union(arms.to_vec()),
    }
}

#[test]
fn inlined_helper_types_equal_the_catalog_bodies() {
    let checked_in = include_str!("../../../schemas/partials/lifecycle.yaml");
    let actual = types_of(checked_in);
    let projection = projection();

    let names: Vec<&str> = projection.helper_types.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["agentic-cli", "ip-address"]);
    for name in names {
        let catalog = expression_function_catalog().named_type(name).expect("a catalog helper type");
        assert_eq!(actual.get(name), Some(catalog), "the inlined `{name}` drifted from functions.yaml");
    }
}

#[test]
fn categories_and_bundles_name_every_generated_verb_with_its_projected_forms() {
    let checked_in = include_str!("../../../schemas/partials/lifecycle.yaml");
    let actual = types_of(checked_in);
    let projection = projection();

    for (category, short_only) in [("expression", false), ("bundle", true), ("initialize-bundle", true)] {
        let keys = category_keys(&actual, category);
        for (_, verb) in &projection.verbs {
            let mut expected = vec![verb.short_type_name()];
            if !short_only {
                expected.extend(verb.long_type_name());
            }
            assert_eq!(keys.get(&verb.name), Some(&expected), "`{category}.{}`", verb.name);
        }
        assert_eq!(keys.get("set"), Some(&vec!["set-short".to_string()]), "`{category}.set` stays lifecycle `set`");
    }
}

#[test]
fn the_generated_regions_are_byte_identical_to_a_regeneration() {
    let checked_in = include_str!("../../../schemas/partials/lifecycle.yaml");
    let lines: Vec<&str> = checked_in.split_inclusive('\n').collect();
    let projection = projection();

    for region in GeneratedRegion::ALL {
        let (begin, end) = region_bounds(&lines, region).expect("the partial carries the region's markers");
        assert_eq!(
            lines[begin + 1..end].concat(),
            projection.render_region(region),
            "the `{}` region drifted; run `just regen-lifecycle-schema`",
            region.name()
        );
    }
    assert_eq!(regenerated(checked_in), checked_in);
}

#[test]
fn the_generated_verbs_are_value_accepting_catalog_functions_and_side_effects_but_set() {
    let projection = projection();
    let mut expected: Vec<(VerbOrigin, &str)> = effect_function_descriptors()
        .iter()
        .map(|descriptor| (VerbOrigin::SideEffect, &*descriptor.name))
        .filter(|(_, name)| *name != "set")
        .chain(expression_function_descriptors().iter().filter(|descriptor| descriptor.accepts_value_arguments()).map(|descriptor| (VerbOrigin::ExpressionFunction, &*descriptor.name)))
        .collect();
    expected.sort_unstable();
    expected.dedup();

    let actual: Vec<(VerbOrigin, &str)> =
        projection.verbs.iter().map(|(origin, verb)| (*origin, verb.name.as_str())).collect();

    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 129);
    let recorded: Vec<(&str, VerbOrigin)> =
        projection.collisions.iter().map(|collision| (collision.verb, collision.catalog)).collect();
    assert_eq!(recorded, [("set", VerbOrigin::SideEffect), ("max", VerbOrigin::ExpressionFunction)]);
    assert_eq!(projection.callback_exclusions.iter().map(String::as_str).collect::<BTreeSet<_>>(), BTreeSet::from(["map", "filter"]));
}

#[test]
fn variadic_verbs_have_no_long_form() {
    let projection = projection();
    let positional_only: Vec<&str> = projection
        .verbs
        .iter()
        .filter(|(_, verb)| verb.long.is_none())
        .map(|(_, verb)| verb.name.as_str())
        .collect();

    assert_eq!(positional_only, ["and", "or"]);
}

#[test]
fn a_side_effect_without_a_typed_entry_fails_the_projection() {
    let mut catalogs = ProjectionCatalogs::shipped();
    let without_append_line: Vec<ExpressionFunctionDescriptor> =
        effect_function_descriptors().iter().filter(|descriptor| &*descriptor.name != "append_line").cloned().collect();
    catalogs.side_effects = &without_append_line;

    let error = LifecycleSchemaProjection::project(&catalogs).expect_err("a side effect lost its types");

    assert_eq!(
        error,
        ProjectionError::MissingEffectType {
            signature: "append_line(file, text)".to_string()
        }
    );
}

#[test]
fn a_removed_overload_fails_the_projection() {
    let mut catalogs = ProjectionCatalogs::shipped();
    let without_overload: Vec<ExpressionFunctionDescriptor> = effect_function_descriptors()
        .iter()
        .filter(|descriptor| !(&*descriptor.name == "ensure_file" && descriptor.parameters.len() == 2))
        .cloned()
        .collect();
    catalogs.side_effects = &without_overload;

    let error = LifecycleSchemaProjection::project(&catalogs).expect_err("an overload lost its types");

    assert_eq!(
        error,
        ProjectionError::MissingEffectType {
            signature: "ensure_file(file, content)".to_string()
        }
    );
}

/// The shipped expression functions plus one renamed copy of `abs`.
fn with_function_named(name: &'static str) -> Vec<ExpressionFunctionDescriptor> {
    let mut descriptors = expression_function_descriptors().to_vec();
    let mut renamed = descriptors.iter().find(|descriptor| &*descriptor.name == "abs").expect("abs").clone();
    renamed.name = name.into();
    descriptors.push(renamed);
    descriptors
}

#[test]
fn a_new_collision_with_a_hand_verb_or_reserved_key_fails_the_projection() {
    for (name, owner) in [
        ("stop", "a lifecycle control verb"),
        ("info", "a communication channel"),
        ("shell", "the `shell` action"),
        ("set", "lifecycle `set`"),
        ("when", "a reserved action-map key"),
        ("no_error", "a reserved action-map key"),
    ] {
        let descriptors = with_function_named(name);
        let mut catalogs = ProjectionCatalogs::shipped();
        catalogs.expression_functions = &descriptors;

        let error = LifecycleSchemaProjection::project(&catalogs).expect_err(name);

        assert_eq!(
            error,
            ProjectionError::Collision {
                verb: name.to_string(),
                owner: owner.to_string()
            }
        );
    }
}

#[test]
fn a_function_named_like_a_side_effect_fails_the_projection() {
    let descriptors = with_function_named("ensure_dir");
    let mut catalogs = ProjectionCatalogs::shipped();
    catalogs.expression_functions = &descriptors;

    let error = LifecycleSchemaProjection::project(&catalogs).expect_err("ambiguous verb");

    assert!(matches!(error, ProjectionError::Collision { ref verb, .. } if verb == "ensure_dir"), "{error:?}");
}

#[test]
fn projecting_twice_renders_identical_regions() {
    let first = projection();
    let second = projection();

    assert_eq!(first, second);
    for region in GeneratedRegion::ALL {
        assert_eq!(first.render_region(region), second.render_region(region), "{}", region.name());
    }
}
