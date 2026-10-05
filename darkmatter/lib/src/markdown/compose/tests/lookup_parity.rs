//! Lookup parity inventory (R3): no `EvaluationLookup` resolves a bare
//! missing name from `ctx`.
//!
//! [`INVENTORY`] lists every implementation in this crate's sources. The scan
//! test fails when an implementation is added or removed without updating it,
//! so a new lookup is classified before it ships. Each production lookup is
//! then probed: a bare `today` is an absent document property even though
//! `ctx.today` resolves, and `env` is the namespace even when the document
//! holds an `env` key. Test fixtures answer from a fixed map and have no
//! namespace logic to fall back through, so they are listed but not probed.

use std::path::Path;

use serde_json::{Value, json};

use crate::markdown::compose::conditions::ShortcutLookup;
use crate::markdown::compose::context::effective_state::ResolvingLookup;
use crate::markdown::compose::expression::{
    BindingView, CtxLookup, EvaluationLookup, EvaluationSession, ResolutionContext,
    ResolvedBinding, ScopeId,
};
use crate::markdown::compose::frontmatter_interpolation::FrontmatterSeedState;
use crate::markdown::compose::inline::interpolation::deferrable_lookup_for_tests;
use crate::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};

/// How an implementation is covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coverage {
    /// Probed by [`every_production_lookup_keeps_bare_names_out_of_ctx`].
    Probed,
    /// A test fixture over a fixed map.
    Fixture,
}

/// `(type, file relative to src/, coverage)`.
const INVENTORY: &[(&str, &str, Coverage)] = &[
    ("EffectiveState", "markdown/compose/context/effective_state.rs", Coverage::Probed),
    ("ResolvingLookup", "markdown/compose/context/effective_state.rs", Coverage::Probed),
    ("FrontmatterSeedState", "markdown/compose/frontmatter_interpolation.rs", Coverage::Probed),
    ("ShortcutLookup", "markdown/compose/conditions.rs", Coverage::Probed),
    ("CtxLookup", "markdown/compose/expression/ctx.rs", Coverage::Probed),
    ("EvaluationSession", "markdown/compose/expression/binding.rs", Coverage::Probed),
    ("DeferrableLookup", "markdown/compose/inline/interpolation.rs", Coverage::Probed),
    ("TestLookup", "markdown/compose/expression/mod.rs", Coverage::Fixture),
    ("Nothing", "markdown/compose/expression/absence.rs", Coverage::Fixture),
    ("FixtureLookup", "markdown/compose/expression/semantics.rs", Coverage::Fixture),
    ("FsLookup", "markdown/compose/expression/catalog/mod.rs", Coverage::Fixture),
    ("FixtureLookup", "markdown/compose/expression/catalog/mod.rs", Coverage::Fixture),
    ("MapLookup", "markdown/compose/expression/catalog/mod.rs", Coverage::Fixture),
];

/// Every `impl … EvaluationLookup for <Type>` outside a comment, as
/// `(type, file relative to src/)`.
fn scanned_implementations() -> Vec<(String, String)> {
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let mut found = Vec::new();
    let mut pending = vec![src.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for line in text.lines() {
                let line = line.trim_start();
                if line.starts_with("//") || !line.starts_with("impl") {
                    continue;
                }
                let Some((_, rest)) = line.split_once("EvaluationLookup for ") else {
                    continue;
                };
                let name: String =
                    rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                let relative = path.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
                found.push((name, relative));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_inventory_lists_every_lookup_implementation() {
    let mut listed: Vec<(String, String)> = INVENTORY
        .iter()
        .map(|(name, file, _)| ((*name).to_string(), (*file).to_string()))
        .collect();
    listed.sort();
    assert_eq!(
        scanned_implementations(),
        listed,
        "an EvaluationLookup was added or removed: classify it in INVENTORY and, if it \
         resolves names itself, probe it below"
    );
}

const DOCUMENT_ENV: &str = "from-the-document";

fn context() -> ComposeContext {
    ComposeContext::fixed_for_testing()
}

fn state() -> EffectiveState {
    EffectiveStateBuilder::new()
        .with_frontmatter([("env".to_string(), json!(DOCUMENT_ENV))].into())
        .with_context(context())
        .build()
        .unwrap()
}

/// A bare name is never `ctx`; `ctx.<name>` is.
#[track_caller]
fn assert_bare_name_is_a_document_property(name: &str, lookup: &dyn EvaluationLookup) {
    assert_eq!(
        lookup.resolve("today").unwrap(),
        ResolvedBinding::Document { value: None },
        "{name}: a bare `today` must not read ctx.today"
    );
    assert!(
        matches!(lookup.resolve("ctx.today").unwrap(), ResolvedBinding::Namespace { value: Some(_) }),
        "{name}: ctx.today still resolves"
    );
}

/// `env` is the namespace, never the document's `env` key; `doc.env` is.
#[track_caller]
fn assert_reserved_roots_win(name: &str, lookup: &dyn EvaluationLookup) {
    let env = lookup.resolve("env").unwrap();
    assert!(
        matches!(&env, ResolvedBinding::Namespace { value } if *value != Some(json!(DOCUMENT_ENV))),
        "{name}: bare env read the document: {env:?}"
    );
    assert_eq!(
        lookup.resolve("doc.env").unwrap().into_value(),
        Some(json!(DOCUMENT_ENV)),
        "{name}: doc.env reads the document"
    );
}

#[test]
fn every_production_lookup_keeps_bare_names_out_of_ctx() {
    let state = state();
    let file_context = biscuit_file::FileResolutionContext::new(std::env::temp_dir());
    let resolution = ResolutionContext::new(file_context.clone());
    let environment = std::collections::HashMap::new();
    let data = json!({ "env": DOCUMENT_ENV });
    let view = std::sync::Arc::new(BindingView::builder(ScopeId::new("parity")).build().unwrap());

    for (name, _, coverage) in INVENTORY {
        if *coverage != Coverage::Probed {
            continue;
        }
        match *name {
            "EffectiveState" => {
                assert_bare_name_is_a_document_property(name, &state);
                assert_reserved_roots_win(name, &state);
            }
            "ResolvingLookup" => {
                let lookup = ResolvingLookup::new(&state, resolution.clone());
                assert_bare_name_is_a_document_property(name, &lookup);
                assert_reserved_roots_win(name, &lookup);
            }
            "DeferrableLookup" => {
                let lookup = deferrable_lookup_for_tests(&state, resolution.clone());
                assert_bare_name_is_a_document_property(name, &lookup);
                assert_reserved_roots_win(name, &lookup);
            }
            "EvaluationSession" => {
                let session = EvaluationSession::associate(view.clone(), &state, []).unwrap();
                assert_bare_name_is_a_document_property(name, &session);
                assert_reserved_roots_win(name, &session);
            }
            "FrontmatterSeedState" => {
                let seed = FrontmatterSeedState::new(
                    [("env".to_string(), json!(DOCUMENT_ENV))].into(),
                    context(),
                );
                assert_bare_name_is_a_document_property(name, &seed);
                assert_reserved_roots_win(name, &seed);
            }
            "ShortcutLookup" => {
                let lookup = ShortcutLookup::new(&data, &file_context);
                assert_bare_name_is_a_document_property(name, &lookup);
                assert_reserved_roots_win(name, &lookup);
            }
            "CtxLookup" => {
                // Context-only: there is no document to read, so only the
                // fallback half applies.
                let lookup = CtxLookup::new(Path::new("."), &environment);
                assert_bare_name_is_a_document_property(name, &lookup);
            }
            other => panic!("{other} is marked Probed but has no probe"),
        }
    }
}

/// The probes above would pass vacuously if `today` were not a context key.
#[test]
fn the_probe_name_is_a_captured_context_key() {
    assert_eq!(context().get("today"), Some(&Value::String("2024-06-15".to_string())));
}
