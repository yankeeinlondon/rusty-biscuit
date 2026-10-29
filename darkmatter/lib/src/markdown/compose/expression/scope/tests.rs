use super::*;
use crate::markdown::compose::expression::functions::{
    accepts_call_arity, canonical_function_name, dispatchable_canonical_names, inspect_registration,
};

/// The built-in narrowing facts live on their registrations: exactly the
/// inspecting predicates declare one, each on its one argument.
#[test]
fn the_built_in_narrowing_facts_are_registered_on_one_argument_predicates() {
    let mut narrowing: Vec<&str> = dispatchable_canonical_names()
        .into_iter()
        .filter(|name| inspect_registration(name, |registration| registration.narrowing_fact().is_some()) == Some(true))
        .collect();
    narrowing.sort_unstable();
    assert_eq!(
        narrowing,
        ["file_exists", "is_array", "is_empty", "is_file_type", "is_null", "is_number", "is_object", "is_string"]
    );
    for name in narrowing {
        assert!(accepts_call_arity(name, 1), "{name} accepts one argument");
        let index = inspect_registration(name, |registration| registration.narrowing_fact_for(1).map(|fact| fact.index()));
        assert_eq!(index, Some(Some(0)), "{name}");
    }
}

#[test]
fn predicate_aliases_and_case_resolve_through_the_registry() {
    assert_eq!(canonical_function_name("isString"), Some("is_string"));
    assert_eq!(canonical_function_name("IS_NULL"), Some("is_null"));
    assert_eq!(canonical_function_name("fileExists"), Some("file_exists"));
    assert_eq!(canonical_function_name("AND"), Some("and"));
    assert_eq!(canonical_function_name("is_boolean"), None);
}

#[test]
fn subjects_overlap_through_ancestors_descendants_and_any_index() {
    let path = |text: &str| Subject::document_path(text).unwrap();
    assert!(path("a").overlaps(&path("a.b")));
    assert!(path("a.b").overlaps(&path("a")));
    assert!(!path("a.b").overlaps(&path("a.c")));
    assert!(!path("a").overlaps(&path("b")));
    let declared = DeclaredTypes::new(None);
    let subject = |text: &str| declared.subject(&crate::markdown::compose::expression::parse(text).unwrap()).unwrap();
    assert!(subject("a[0]").overlaps(&subject("a[-1]")));
    assert!(subject("a[0]").overlaps(&subject("a['k']")));
    assert!(subject("fm.a").overlaps(&path("a.b")));
}

mod lexical {
    use std::sync::Arc;

    use super::*;
    use crate::markdown::compose::expression::{BindingId, parse};
    use crate::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};

    fn schema() -> SimplifiedSchema {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str("person: string\nother: string\n").unwrap();
        parse_yaml_schema(&value).unwrap()
    }

    fn expr(text: &str) -> Expr {
        parse(text).unwrap()
    }

    fn present(scope: &TypeScope<'_>, text: &str) -> bool {
        scope.type_of_expr(&expr(text)).unwrap().guard_presence().is_some()
    }

    fn string_shape(scope: &TypeScope<'_>) -> Shape {
        scope.type_of_expr(&expr("person")).unwrap().shape().clone()
    }

    #[test]
    fn implicit_and_explicit_reads_are_one_local_subject_and_fm_is_a_document_subject() {
        let schema = schema();
        let declared = DeclaredTypes::new(Some(&schema));
        let outside = TypeScope::unguarded(&declared);
        let layer = LexicalTypes::new(vec![TypedLocal::new("person", string_shape(&outside))]);
        let scope = outside.within(layer);

        let implicit = scope.subject(&expr("person.name")).unwrap();
        let explicit = scope.subject(&expr("local.person.name")).unwrap();
        assert_eq!(implicit, explicit);
        assert!(matches!(implicit.binding(), BindingId::Local(_)));
        assert!(matches!(scope.subject(&expr("fm.person")).unwrap().binding(), BindingId::Document(_)));
        assert!(scope.resolves_to_local("person") && scope.resolves_to_local("local.person"));
        assert!(!scope.resolves_to_local("fm.person") && !scope.resolves_to_local("other"));
        assert!(scope.subject(&expr("local.nope")).is_none(), "no such local, no subject");
        assert!(matches!(
            outside.subject(&expr("person")).unwrap().binding(),
            BindingId::Document(_)
        ));
        assert!(outside.subject(&expr("local.person")).is_none(), "outside any scope `local` is empty");
    }

    #[test]
    fn facts_follow_the_binding_identity_not_the_spelling() {
        let schema = schema();
        let declared = DeclaredTypes::new(Some(&schema));
        let outside = TypeScope::unguarded(&declared);
        let shape = string_shape(&outside);
        let outer = LexicalTypes::new(vec![TypedLocal::new("person", shape.clone())]);
        let scope = outside.within(Arc::clone(&outer));
        let guarded = scope.assume(&expr("person"), Polarity::True);
        assert!(present(&guarded, "local.person"));
        assert!(!present(&guarded, "fm.person"), "a local fact never narrows the document property");
        assert!(!present(&scope, "person"));

        // A different spelling of the same original item shares its facts.
        let id = outer.id_of("person").unwrap();
        let aliased = guarded.within(outer.child(vec![TypedLocal::aliasing("p", id, shape.clone())]));
        assert!(present(&aliased, "p"));
        assert!(present(&aliased, "person"));

        // An inner binding of the same name is a new subject.
        let inner = guarded.within(outer.child(vec![TypedLocal::new("person", shape)]));
        assert!(!present(&inner, "person"));
        assert!(!present(&inner, "local.person"));

        // A fact about the document property does not narrow a local.
        let document_guard = outside.assume(&expr("fm.person"), Polarity::True);
        let local_again = document_guard.within(outer);
        assert!(present(&local_again, "fm.person") && !present(&local_again, "person"));

        // Leaving the layer restores the outer meaning.
        assert!(matches!(guarded.subject(&expr("person")).unwrap().binding(), BindingId::Local(_)));
        assert!(matches!(outside.subject(&expr("person")).unwrap().binding(), BindingId::Document(_)));
    }

    #[test]
    fn a_local_is_typed_by_its_layer_and_a_layer_owns_its_definitions() {
        let schema = schema();
        let declared = DeclaredTypes::new(Some(&schema));
        let outside = TypeScope::unguarded(&declared);
        let layer = LexicalTypes::new_defining(
            vec![TypedLocal::new("person", string_shape(&outside))],
            crate::markdown::schemas::DefinitionTable::default(),
        );
        let scope = outside.within(Arc::clone(&layer));
        assert!(std::ptr::eq(scope.definitions(), layer.definitions().unwrap()));
        assert!(std::ptr::eq(outside.definitions(), declared.definitions()));
        assert!(std::ptr::eq(scope.declared(), &declared), "the scope still borrows its declarations");
        let ty = scope.type_of_expr(&expr("person")).unwrap();
        assert!(matches!(ty.declared(), DeclaredType::Property(_)));
        assert!(matches!(
            scope.type_of_expr(&expr("person.name.deeper")).unwrap().declared(),
            DeclaredType::NoProjection
        ));
        assert_eq!(layer.names(), ["person"]);
    }
}
