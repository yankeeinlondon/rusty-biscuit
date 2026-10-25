//! The design-time call check on Claudine's lifecycle surfaces, and the host
//! catalog typing Claudine's globals.

use super::*;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{
    DeclaredTypes, Shape, TypeScope, check_expression_calls, parse, parse_spanned,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};

fn frontmatter(text: &str) -> serde_json::Value {
    let md = Markdown::from(text);
    serde_json::Value::Object(md.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

fn effective(dir: &Path, text: &str) -> EffectiveSchema {
    DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir))
        .effective_for(&Markdown::from(text))
        .expect("the schema resolves")
        .expect("an effective schema")
}

fn shape_text(shape: &Shape) -> String {
    darkmatter::markdown::compose::expression::ArgumentType::Typed(shape.clone()).to_string()
}

/// `min(flag, 5)` in a Claudine lifecycle action is rejected with the same
/// verdict the body call gets.
#[test]
fn a_lifecycle_call_is_rejected_like_the_body_call() {
    let dir = tempfile::tempdir().unwrap();
    let text = "---\n$schema:\n  flag: boolean(required)\nflag: true\nsuccess:\n  - say: \"{{ min(flag, 5) }}\"\n---\nbody\n";
    let value = frontmatter(text);
    let config = parse_lifecycle_config(&value, dummy_path()).expect("the lifecycle parses");
    let schema = effective(dir.path(), text);
    let error = reject_incompatible_calls(&value, &config, dummy_path(), None, Some(&schema)).expect_err("rejected");
    let CompositionError::LifecycleCallRejected { property, source, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(property, "success[0].say.text");
    let declared = DeclaredTypes::for_schema(Some(&schema));
    let body = check_expression_calls(&parse_spanned("min(flag, 5)").unwrap(), &TypeScope::unguarded(&declared)).remove(0);
    assert_eq!(source.to_string(), body.rejection_message());

    let fixed = "---\n$schema:\n  flag: boolean(required)\nflag: true\nsuccess:\n  - say: \"{{ flag ? 'yes' : 'no' }}\"\n---\nbody\n";
    let value = frontmatter(fixed);
    let config = parse_lifecycle_config(&value, dummy_path()).expect("the lifecycle parses");
    assert!(reject_incompatible_calls(&value, &config, dummy_path(), None, Some(&effective(dir.path(), fixed))).is_ok());
}

/// Claudine's globals are typed by its host catalog: the registry carries
/// it, and a sequence site that declares `seq` reads it.
#[test]
fn claudine_globals_take_their_host_catalog_types() {
    let declared = shell_template::lifecycle_declared_types(None, claudine_registry());
    let scope = TypeScope::unguarded(&declared);
    let count = scope.type_of_expr(&parse("loop.count").unwrap()).unwrap();
    assert_eq!(shape_text(count.shape()), "number(integer;min(1)) | null");

    let roots = ["seq"];
    let sequence = DeclaredTypes::new(None)
        .with_host_globals(&roots)
        .with_global_catalog(crate::composition::schema::globals::claudine_globals());
    let scope = TypeScope::unguarded(&sequence);
    let index = scope.type_of_expr(&parse("seq.index").unwrap()).unwrap();
    assert_eq!(shape_text(index.shape()), "number(integer;min(1)) | null");
}

/// Under Claudine's registry a guard narrows the lifecycle sites it selects,
/// a `set` ends the fact, and a host global shadowing a document property
/// is a separate subject: a guard on `fm.seq` narrows the document property
/// and never the host's `seq`.
#[test]
fn a_guard_narrows_until_a_write_and_never_reaches_a_shadowing_host_global() {
    use darkmatter::markdown::compose::expression::{ArgumentCategory, Polarity};

    let dir = tempfile::tempdir().unwrap();
    let text = "---\n$schema:\n  seq: number\nsuccess:\n  - when: fm.seq\n    then:\n      - say: \"{{ min(fm.seq, 5) }}\"\n      - set:\n          seq: 3\n      - say: \"{{ min(fm.seq, 6) }}\"\n---\nbody\n";
    let value = frontmatter(text);
    let config = parse_lifecycle_config(&value, dummy_path()).expect("the lifecycle parses");
    let schema = effective(dir.path(), text);
    let found: Vec<(String, ArgumentCategory)> =
        darkmatter::lifecycle::lifecycle_call_checks(&value, &config, claudine_registry(), Some(&schema))
            .into_iter()
            .flat_map(|surface| {
                let property = surface.property.to_string();
                surface.checks.into_iter().map(move |found| (property.clone(), found.check.category()))
            })
            .collect();
    assert_eq!(found, [("success[0].then[2].say.text".to_string(), ArgumentCategory::NullableMatch)]);
    assert!(reject_incompatible_calls(&value, &config, dummy_path(), None, Some(&schema)).is_ok());

    // At a sequence site `seq` is the host's global, shadowing the property.
    let roots = ["seq"];
    let sequence = DeclaredTypes::for_schema(Some(&schema))
        .with_host_globals(&roots)
        .with_global_catalog(crate::composition::schema::globals::claudine_globals());
    let site = TypeScope::unguarded(&sequence);
    let guarded = site.assume(&parse("fm.seq").unwrap(), Polarity::True);
    let read = |scope: &TypeScope<'_>, path: &str| shape_text(scope.type_of_expr(&parse(path).unwrap()).unwrap().shape());
    assert_eq!(read(&guarded, "fm.seq"), "number");
    assert_eq!(read(&site, "fm.seq"), "number | null");
    assert_eq!(read(&guarded, "seq.index"), read(&site, "seq.index"), "the host global is not the guarded property");
    assert_ne!(read(&site, "seq.index"), "unknown");
}

/// A category 3 lifecycle call is not rejected: Claudine receives its shared
/// warning, located at the argument in the authored text, to report with the
/// composition's own warnings.
#[test]
fn a_category_3_lifecycle_call_returns_its_located_warning() {
    let dir = tempfile::tempdir().unwrap();
    let text = "---\n$schema:\n  label: string\nlabel: \"7\"\nsuccess:\n  - when: min(label, 5) > 1\n    then:\n      - say: done\n---\nBody\n";
    let value = frontmatter(text);
    let config = parse_lifecycle_config(&value, dummy_path()).expect("the lifecycle parses");
    let schema = effective(dir.path(), text);
    let warnings = reject_incompatible_calls(&value, &config, dummy_path(), Some(text), Some(&schema))
        .expect("a category 3 call is not rejected");
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    let diagnostic = warnings[0].diagnostic.as_ref().expect("the shared record");
    assert_eq!(diagnostic.code, darkmatter::markdown::compose::expression::UNCERTAIN_CALL_CODE);
    let source = diagnostic.source.as_ref().expect("located");
    // `label` in `  - when: min(label, 5) > 1`, line 6.
    assert_eq!((source.line, source.column), (6, 15));
}
