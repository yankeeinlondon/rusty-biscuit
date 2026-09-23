use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ResolutionContext, SpannedExprKind, evaluate, parse, parse_spanned,
};
use serde_json::{Value, json};
use std::fs;

struct Lookup {
    context: ResolutionContext,
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        (path == "ctx.limit").then(|| json!(3))
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(self.context.clone())
    }
}

fn eval(source: &str, lookup: &Lookup) -> Value {
    evaluate(&parse(source).unwrap(), lookup).unwrap()
}

#[test]
fn object_and_array_literals_preserve_spans_and_evaluate_computed_values() {
    let source = r#"{ state: "open", limits: [1, ctx.limit], nested: { ok: true } }"#;
    let parsed = parse_spanned(source).unwrap();
    assert_eq!(parsed.span, 0..source.len());
    let SpannedExprKind::ObjectLiteral(entries) = &parsed.kind else {
        panic!("expected object literal");
    };
    assert_eq!(entries[0].0.value, "state");
    assert_eq!(&source[entries[0].0.span.clone()], "state");
    assert!(matches!(entries[1].1.kind, SpannedExprKind::ArrayLiteral(_)));

    let dir = tempfile::tempdir().unwrap();
    let lookup = Lookup {
        context: ResolutionContext::new(dir.path().to_path_buf()),
    };
    assert_eq!(
        evaluate(&parsed.erase(), &lookup).unwrap(),
        json!({"state": "open", "limits": [1, 3], "nested": {"ok": true}})
    );
}

#[test]
fn literals_reject_duplicate_invalid_and_trailing_keys_without_breaking_indexing() {
    for invalid in [
        "{ a: 1, a: 2 }",
        "{ 1: true }",
        "{ a true }",
        "{ a: 1, }",
        "[1, 2,]",
    ] {
        assert!(parse(invalid).is_err(), "{invalid} must be rejected");
    }

    let dir = tempfile::tempdir().unwrap();
    let lookup = Lookup {
        context: ResolutionContext::new(dir.path().to_path_buf()),
    };
    assert_eq!(eval(r#"["zero", "one"][1]"#, &lookup), json!("one"));
    assert_eq!(eval(r#"{ key: [4, 7] }.key[0]"#, &lookup), json!(4));
}

#[test]
fn indexed_file_endpoints_cover_family_ordering_isolation_and_fallbacks() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "foo.md",
        "foo-002.md",
        "foo-10.md",
        "foo-3.txt",
        "food-99.md",
        "foo-bar.md",
        "lone-2.md",
    ] {
        fs::write(dir.path().join(name), "").unwrap();
    }
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/foo-99.md"), "").unwrap();
    let lookup = Lookup {
        context: ResolutionContext::new(dir.path().to_path_buf()),
    };

    assert_eq!(eval(r#"find_first_index("foo-002.md")"#, &lookup), json!("foo.md"));
    assert_eq!(eval(r#"find_last_index("foo.md")"#, &lookup), json!("foo-10.md"));
    assert_eq!(eval(r#"find_last_index("lone-2.md")"#, &lookup), json!("lone-2.md"));
    assert_eq!(eval(r#"find_first_index("missing-4.md")"#, &lookup), json!("missing-4.md"));
    assert_eq!(eval("find_first_index(null)", &lookup), Value::Null);
    assert!(eval_error(r#"find_last_index("https://example.com/foo.md")"#, &lookup));
}

fn eval_error(source: &str, lookup: &Lookup) -> bool {
    evaluate(&parse(source).unwrap(), lookup).is_err()
}
