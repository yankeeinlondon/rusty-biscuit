//! The library side of `ComposeOptions::with_setters`, for what only a host can
//! arrange: a data override beside a setter, and a preparation cache key.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::setters::{RawSetter, SetterPlan, TokenClass, classify_token};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use serde_json::json;
use tempfile::TempDir;

fn setter(token: &str) -> RawSetter {
    match classify_token(token) {
        TokenClass::Setter(setter) => setter,
        other => panic!("`{token}` is not a setter: {other:?}"),
    }
}

fn plan(tokens: &[&str]) -> SetterPlan {
    SetterPlan::new(tokens.iter().map(|token| setter(token))).expect("the plan settles")
}

fn compose(dir: &TempDir, document: &str, options: impl FnOnce(ComposeOptions) -> ComposeOptions) -> String {
    let path = dir.path().join("doc.md");
    std::fs::write(&path, document).unwrap();
    let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), "")).with_source_file(&path);
    let request = crate::request_support::request(options(base));
    let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
    composed.content().trim_end().to_string()
}

#[test]
fn a_setter_applies_after_set_overrides() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ a.b }} {{ a.c }}\n", |options| {
        options.with_set_overrides(json!({"a": {"b": "set", "c": "set"}})).with_setters(plan(&["a.b=setter"]))
    });
    assert_eq!(out, "setter set");
}

#[test]
fn a_data_override_keeps_the_key_it_names() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ a.b }} {{ c }} {{ d }}\n", |options| {
        options
            .with_data_overrides(json!({"a": {"b": "data"}, "c": "data"}))
            .with_setters(plan(&["a.b=setter", "c=setter", "d=setter"]))
    });
    assert_eq!(out, "data data setter");
}

#[test]
fn an_empty_plan_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ t }}\n", |options| options.with_setters(SetterPlan::default()));
    assert_eq!(out, "1");
}
