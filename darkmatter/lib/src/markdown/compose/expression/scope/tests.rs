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
