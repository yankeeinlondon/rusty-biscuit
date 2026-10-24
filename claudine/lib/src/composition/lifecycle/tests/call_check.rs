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
    let error = reject_incompatible_calls(&value, &config, dummy_path(), Some(&schema)).expect_err("rejected");
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
    assert!(reject_incompatible_calls(&value, &config, dummy_path(), Some(&effective(dir.path(), fixed))).is_ok());
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
