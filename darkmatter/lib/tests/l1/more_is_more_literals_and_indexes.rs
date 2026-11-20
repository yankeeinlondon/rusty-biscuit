use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, SpannedExprKind, SpannedObjectEntry, evaluate, parse, parse_spanned,
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
    let SpannedObjectEntry::Field { key, .. } = &entries[0] else {
        panic!("expected a written field, got {:?}", entries[0]);
    };
    assert_eq!(key.value, "state");
    assert_eq!(&source[key.span.clone()], "state");
    assert!(matches!(entries[1].value().kind, SpannedExprKind::ArrayLiteral(_)));

    let dir = tempfile::tempdir().unwrap();
    let lookup = Lookup {
        context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path())),
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
        context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path())),
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
        context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path())),
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

#[test]
fn object_spread_merges_in_written_order_and_rejects_a_non_object() {
    let dir = tempfile::tempdir().unwrap();
    let lookup = Lookup {
        context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path())),
    };
    // The last entry to supply a key wins; key order follows first insertion.
    assert_eq!(
        eval("{ a: 1, ...{ a: 2, b: 2 }, c: 3, ...{ c: 4 } }", &lookup),
        json!({"a": 2, "b": 2, "c": 4})
    );
    assert_eq!(eval("{ ...{ a: 1 }, a: 9 }", &lookup), json!({"a": 9}));
    // A null spread contributes nothing; an object from a variable is copied.
    assert_eq!(eval("{ ...null, limit: ctx.limit, ...{} }", &lookup), json!({"limit": 3}));
    for (source, found) in [("{ ...1 }", "number"), ("{ ...'s' }", "string"), ("{ ...[1] }", "array"), ("{ ...true }", "boolean")] {
        let error = evaluate(&parse(source).unwrap(), &lookup).unwrap_err();
        assert!(
            matches!(&error, ExpressionError::InvalidSpreadOperand { found: typed, .. } if *typed == found),
            "{source}: {error:?}"
        );
        assert_eq!(error.to_string(), format!("object spread requires an object or null, found {found}"));
    }
}
