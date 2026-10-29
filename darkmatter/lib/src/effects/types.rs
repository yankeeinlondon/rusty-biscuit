//! Typed parameter metadata for the side-effect capabilities.
//!
//! [`EFFECT_DESCRIPTORS`](super::EFFECT_DESCRIPTORS) carries untyped signature
//! strings; the parameter and return types live in the catalog-grammar file
//! `darkmatter/schemas/partials/mutations.yaml`, embedded here and loaded with
//! the same loader as the expression-function catalog. Effects are kept out
//! of `functions.yaml` so they never become callable in expressions. A unit
//! test holds the two in step: every descriptor signature has a typed overload
//! with the same verb and parameter names, and every typed overload has a
//! descriptor.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::markdown::compose::expression::catalog::load_catalog;
use crate::markdown::compose::expression::{ExpressionFunctionDescriptor, FunctionCatalog, FunctionRegistration};

/// The embedded file's repository path, used in load errors.
const MUTATIONS_PATH: &str = "darkmatter/schemas/partials/mutations.yaml";

const MUTATIONS_SOURCE: &str = include_str!("../../../schemas/partials/mutations.yaml");

/// The typed side-effect catalog, loaded once.
///
/// ## Panics
///
/// When the embedded `mutations.yaml` does not load; a unit test loads it.
pub fn effect_function_catalog() -> &'static FunctionCatalog {
    static CATALOG: LazyLock<FunctionCatalog> = LazyLock::new(|| {
        load_catalog(MUTATIONS_SOURCE, MUTATIONS_PATH, &[])
            .unwrap_or_else(|error| panic!("the embedded side-effect catalog does not load: {error}"))
    });
    &CATALOG
}

/// Every typed side-effect overload, in documentation order (category list
/// order, then verb, then authored overload order).
pub fn effect_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    &effect_function_catalog().descriptors
}

/// The binding registration for the side-effect `verb` (exact, case-sensitive
/// name), compiled once from every typed overload of that verb. `None` for a
/// verb with no typed entry.
///
/// ## Panics
///
/// When a typed entry does not compile into a registration; a unit test
/// builds every one.
pub fn effect_registration(verb: &str) -> Option<&'static FunctionRegistration> {
    static REGISTRATIONS: LazyLock<HashMap<&'static str, FunctionRegistration>> = LazyLock::new(|| {
        let catalog = effect_function_catalog();
        let mut overloads: HashMap<&'static str, Vec<_>> = HashMap::new();
        for descriptor in &catalog.descriptors {
            overloads
                .entry(&*descriptor.name)
                .or_default()
                .push(descriptor.function_type.clone());
        }
        let resolver = catalog.type_resolver();
        overloads
            .into_iter()
            .map(|(name, types)| {
                let registration = FunctionRegistration::builder_from_types(name, types)
                    .build_with(&resolver)
                    .unwrap_or_else(|error| panic!("the side-effect `{name}` does not compile: {error}"));
                (name, registration)
            })
            .collect()
    });
    REGISTRATIONS.get(verb)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::effects::EFFECT_DESCRIPTORS;

    /// A verb plus its ordered parameter names: the identity both tables share.
    type SignatureKey = (String, Vec<String>);

    /// Splits a descriptor signature such as `ensure_file(file, content)`.
    fn descriptor_key(signature: &str) -> SignatureKey {
        let (verb, rest) = signature
            .split_once('(')
            .unwrap_or_else(|| panic!("descriptor signature `{signature}` has no parameter list"));
        let parameters = rest
            .strip_suffix(')')
            .unwrap_or_else(|| panic!("descriptor signature `{signature}` is not closed"))
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
        (verb.trim().to_string(), parameters)
    }

    fn typed_key(descriptor: &ExpressionFunctionDescriptor) -> SignatureKey {
        let parameters = descriptor.parameters.iter().map(|parameter| parameter.name.to_string()).collect();
        (descriptor.name.to_string(), parameters)
    }

    fn render((verb, parameters): &SignatureKey) -> String {
        format!("{verb}({})", parameters.join(", "))
    }

    /// The coverage gaps between `catalog` and the effect descriptors, as
    /// `(descriptor signatures with no typed overload, typed overloads with
    /// no descriptor)`. Both empty means full coverage.
    fn coverage_gaps(catalog: &FunctionCatalog) -> (Vec<String>, Vec<String>) {
        let described: BTreeSet<SignatureKey> =
            EFFECT_DESCRIPTORS.iter().map(|descriptor| descriptor_key(descriptor.signature)).collect();
        let typed: BTreeSet<SignatureKey> = catalog.descriptors.iter().map(typed_key).collect();
        (
            described.difference(&typed).map(render).collect(),
            typed.difference(&described).map(render).collect(),
        )
    }

    #[test]
    fn every_effect_descriptor_has_exactly_one_typed_overload() {
        let (untyped, undescribed) = coverage_gaps(effect_function_catalog());

        assert!(untyped.is_empty(), "effect signatures without a typed mutations.yaml entry: {untyped:?}");
        assert!(undescribed.is_empty(), "typed mutations.yaml entries without an effect descriptor: {undescribed:?}");
        assert_eq!(effect_function_descriptors().len(), EFFECT_DESCRIPTORS.len());
    }

    #[test]
    fn coverage_check_reports_a_removed_typed_entry() {
        let without_append_line = MUTATIONS_SOURCE.replace(
            "    append_line: |-
        function(
            parameters(file: string, text: string | object | unknown[] | null);
            returns(string);
            category(File & Directory);
            fallible;
        ) -> Appends text and a newline to a file. Returns the absolute path.
",
            "",
        );
        assert_ne!(without_append_line, MUTATIONS_SOURCE, "the fixture must remove the `append_line` entry");
        let catalog = load_catalog(&without_append_line, "fixture/mutations.yaml", &[]).expect("fixture loads");

        let (untyped, undescribed) = coverage_gaps(&catalog);

        assert_eq!(untyped, ["append_line(file, text)"]);
        assert!(undescribed.is_empty(), "{undescribed:?}");
    }

    #[test]
    fn coverage_check_reports_a_typed_overload_with_no_descriptor() {
        let extra = MUTATIONS_SOURCE.replace(
            "            ) -> Creates an empty file if missing. Returns the absolute path.\n",
            "            ) -> Creates an empty file if missing. Returns the absolute path.\n        - |-\n            function(\n                parameters(file: string, content: string, mode: string);\n                returns(string);\n                category(File & Directory);\n                fallible;\n            ) -> Not a real overload.\n",
        );
        assert_ne!(extra, MUTATIONS_SOURCE, "the fixture must add an `ensure_file` overload");
        let catalog = load_catalog(&extra, "fixture/mutations.yaml", &[]).expect("fixture loads");

        let (untyped, undescribed) = coverage_gaps(&catalog);

        assert!(untyped.is_empty(), "{untyped:?}");
        assert_eq!(undescribed, ["ensure_file(file, content, mode)"]);
    }

    #[test]
    fn typed_categories_match_the_effect_descriptor_categories() {
        for descriptor in EFFECT_DESCRIPTORS {
            let key = descriptor_key(descriptor.signature);
            let typed = effect_function_descriptors()
                .iter()
                .find(|typed| typed_key(typed) == key)
                .unwrap_or_else(|| panic!("no typed overload for `{}`", descriptor.signature));
            assert_eq!(&*typed.category, descriptor.category, "{}", descriptor.signature);
        }
    }

    #[test]
    fn every_typed_effect_is_fallible() {
        for descriptor in effect_function_descriptors() {
            assert!(descriptor.fallible, "`{}` must be fallible: every engine method returns a Result", descriptor.signature);
        }
    }

    #[test]
    fn every_effect_verb_compiles_into_a_registration_with_its_overloads() {
        let mut verbs: Vec<&str> = EFFECT_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.signature.split_once('(').map_or(descriptor.signature, |(verb, _)| verb))
            .collect();
        verbs.dedup();

        for verb in verbs {
            let registration = effect_registration(verb).unwrap_or_else(|| panic!("no registration for `{verb}`"));
            let expected = EFFECT_DESCRIPTORS
                .iter()
                .filter(|descriptor| descriptor.signature.starts_with(&format!("{verb}(")))
                .count();
            assert_eq!(registration.name(), verb);
            assert_eq!(registration.overloads().len(), expected, "{verb}");
        }
        assert!(effect_registration("not_an_effect").is_none());
        assert!(effect_registration("SET").is_none(), "verb lookup is exact");
    }

    fn binds(verb: &str, args: &[serde_json::Value]) -> bool {
        effect_registration(verb).expect("registered").bind_call(args).is_ok()
    }

    #[test]
    fn scalar_text_parameters_admit_what_scalar_string_reads_as_text() {
        use serde_json::json;

        // Dispatch reads these through `scalar_string`, so a number or boolean
        // reaches the engine as its display text.
        assert!(binds("set_frontmatter", &[json!("d.md"), json!(7), json!(null)]));
        assert!(binds("delete_frontmatter", &[json!("d.md"), json!(true)]));
        assert!(binds("http_post", &[json!("https://example.com"), json!(1)]));
        // Paths and property names admit no null or composite value.
        assert!(!binds("ensure_dir", &[json!(null)]));
        assert!(!binds("set_frontmatter", &[json!("d.md"), json!({"a": 1}), json!(1)]));
    }

    #[test]
    fn payload_parameters_admit_every_value_dispatch_writes_today() {
        use serde_json::json;

        for payload in [json!("x"), json!(4), json!(false), json!(null), json!([1, null]), json!({"k": 1})] {
            assert!(binds("ensure_file", &[json!("f.txt"), payload.clone()]), "content {payload}");
            assert!(binds("append_line", &[json!("f.txt"), payload.clone()]), "text {payload}");
            assert!(binds("append_jsonl", &[json!("f.jsonl"), payload.clone()]), "obj {payload}");
            assert!(binds("http_post", &[json!("https://example.com"), payload.clone()]), "body {payload}");
            assert!(binds("append_frontmatter", &[json!("d.md"), json!("tags"), payload.clone()]), "value {payload}");
            assert!(binds("set", &[json!("k"), payload.clone()]), "value {payload}");
        }
    }

    #[test]
    fn merge_frontmatter_admits_only_an_object_like_the_engine() {
        use serde_json::json;

        assert!(binds("merge_frontmatter", &[json!("d.md"), json!({"owner": "ken"})]));
        for rejected in [json!(null), json!("x"), json!([1])] {
            assert!(!binds("merge_frontmatter", &[json!("d.md"), rejected.clone()]), "{rejected}");
        }
    }

    #[test]
    fn ensure_file_selects_its_overload_by_arity() {
        use serde_json::json;

        let registration = effect_registration("ensure_file").expect("registered");

        assert_eq!(registration.bind_call(&[json!("f.txt")]).expect("one argument binds").overload(), 0);
        assert_eq!(registration.bind_call(&[json!("f.txt"), json!("x")]).expect("two arguments bind").overload(), 1);
        assert!(registration.bind_call(&[]).is_err());
    }
}
