//! Sequence descent (spec D6) through the frontmatter provider: typed schema
//! paths, the shipped Claudine predicate typing, the per-pass shape memo, and
//! the hover/completion/navigation boundary at real list items versus
//! synthetic item positions.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::schemas::resolve::resolve_schema;
use lsp_types::{InitializeParams, Uri};

use super::*;
use crate::capabilities::ClientProfile;
use crate::config::{DmlsConfig, SchemaExtensionConfig};
use crate::graph::WorkspaceGraph;
use crate::overlay::{OverlayState, format_dotted};
use crate::source_map::{PositionEncoding, SourceMap};

use FmPathSegment::{Index, Key};

fn claudine_schema_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas/claudine.yaml")
}

/// The shipped Claudine extension baseline's resolved shape (imports expanded).
fn claudine_root() -> SchemaShape {
    let path = claudine_schema_path();
    let resolved = resolve_schema(
        &Value::String(path.to_string_lossy().into_owned()),
        path.parent().unwrap(),
    )
    .expect("shipped Claudine schema resolves");
    match resolved.simplified {
        Some(SimplifiedSchema::Single(shape)) => shape,
        other => panic!("Claudine baseline is a single shape: {other:?}"),
    }
}

/// Runs `f` with an overlay-backed context for `text` at `/w/prompts/doc.md`,
/// with the shipped Claudine extension active when `claudine` is set.
fn with_ctx<R>(text: &str, claudine: bool, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let uri: Uri = "file:///w/prompts/doc.md".parse().unwrap();
    with_ctx_at(Path::new("/w/prompts/doc.md"), uri, Path::new("/w"), text, claudine, f)
}

/// Like [`with_ctx`] for a document at a real `path` under `root`.
fn with_ctx_at<R>(
    path: &Path,
    uri: Uri,
    root: &Path,
    text: &str,
    claudine: bool,
    f: impl FnOnce(&DocumentContext) -> R,
) -> R {
    let mut config = DmlsConfig::default();
    if claudine {
        config.schema.extensions.insert(
            "claudine".to_string(),
            SchemaExtensionConfig { path: claudine_schema_path(), globs: vec!["**/*.md".to_string()] },
        );
    }
    let roots = [root.to_path_buf()];
    let state = OverlayState::default();
    let overlay = state.for_document(&uri, text, path, &config, &roots);
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
    let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let ctx = DocumentContext {
        uri: &uri,
        path,
        text,
        source_map: &source_map,
        graph: &graph,
        doc_id: None,
        config: &config,
        profile: &profile,
        overlay: overlay.as_ref(),
    };
    f(&ctx)
}

fn is_expression_def(def: &PropertyDef) -> bool {
    expression_atom(def).is_some()
}

fn hover_text(hover: &Hover) -> &str {
    let HoverContents::Markup(MarkupContent { value, .. }) = &hover.contents else {
        panic!("markdown hover");
    };
    value
}

// ── Path semantics ──────────────────────────────────────────────────────────

#[test]
fn nested_shape_consumes_array_index_against_item_type() {
    let root = claudine_root();
    let item = nested_shape(&root, &[Key("initialize"), Key("stack"), Index(0)])
        .expect("stack item shape");
    for key in ["when", "action", "no_error"] {
        assert!(item.properties.contains_key(key), "{key} on the stack item");
    }
    let when = def_at_path(&root, &[Key("initialize"), Key("stack"), Index(0), Key("when")])
        .expect("initialize.stack[0].when resolves");
    assert!(is_expression_def(when));
    assert!(def_at_path(&root, &[Key("loop"), Key("stack"), Index(3), Key("when")]).is_some());
}

#[test]
fn nested_shape_fails_closed_on_mismatched_segments() {
    let root = claudine_root();
    // A key where the array item is required.
    assert!(nested_shape(&root, &[Key("initialize"), Key("stack")]).is_none());
    assert!(def_at_path(&root, &[Key("initialize"), Key("stack"), Key("when")]).is_none());
    // An index under a non-array property.
    assert!(nested_shape(&root, &[Key("initialize"), Index(0)]).is_none());
    // A leading index, and a nested index SimplifiedSchema cannot declare.
    assert!(nested_shape(&root, &[Index(0)]).is_none());
    assert!(nested_shape(&root, &[Key("initialize"), Key("stack"), Index(0), Index(0)]).is_none());
    // A key literally named `0` is not an index.
    assert!(nested_shape(&root, &[Key("initialize"), Key("stack"), Key("0")]).is_none());
}

#[test]
fn union_arms_are_chosen_by_array_crossing() {
    // `exit_expressions: exit-expression[] | exit-expression-layer`.
    let root = claudine_root();
    let layer = nested_shape(&root, &[Key("exit_expressions")]).expect("non-array layer arm");
    assert!(layer.properties.contains_key("mode"));
    let rule = nested_shape(&root, &[Key("exit_expressions"), Index(0)]).expect("array rule arm");
    assert!(rule.properties.contains_key("pattern"));
    assert!(!rule.properties.contains_key("mode"));
}

#[test]
fn context_aware_walk_selects_discriminated_arm_inside_array_items() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("types.yaml"),
        concat!(
            "$schema:\n",
            "  shell-step:\n",
            "    kind: literal(shell)\n",
            "    run: expression\n",
            "  note-step:\n",
            "    kind: literal(note)\n",
            "    text: string\n",
        ),
    )
    .unwrap();
    let text = concat!(
        "---\n",
        "$schema:\n",
        "  steps:\n",
        "    - \"shell-step[]@./types.yaml\"\n",
        "    - \"note-step[]@./types.yaml\"\n",
        "steps:\n",
        "  - kind: note\n",
        "    text: hello\n",
        "  - kind: shell\n",
        "    run: ctx.ok\n",
        "  - run: ctx.maybe\n",
        "---\n\nbody\n",
    );
    let doc = dir.path().join("doc.md");
    let uri: Uri = url::Url::from_file_path(&doc).unwrap().as_str().parse().unwrap();
    with_ctx_at(&doc, uri, dir.path(), text, false, |ctx| {
        let shape = known_shape(ctx);
        let mut memo = ShapeMemo::new(ArrayCrossing::Explicit);
        fn resolve<'p>(
            ctx: &DocumentContext,
            shape: &SchemaShape,
            path: &[FmPathSegment<'p>],
            memo: &mut ShapeMemo<'p>,
        ) -> Option<PropertyDef> {
            def_at_path_ctx(ctx, shape, path, memo).map(Cow::into_owned)
        }
        let resolve = |path: &[FmPathSegment<'static>], memo: &mut ShapeMemo<'static>| {
            resolve(ctx, &shape, path, memo)
        };
        // Item 0 selects the note arm: `run` does not exist there.
        assert!(resolve(&[Key("steps"), Index(0), Key("text")], &mut memo).is_some());
        assert!(resolve(&[Key("steps"), Index(0), Key("run")], &mut memo).is_none());
        // Item 1 selects the shell arm.
        let run = resolve(&[Key("steps"), Index(1), Key("run")], &mut memo).expect("shell run");
        assert!(is_expression_def(&run));
        assert!(resolve(&[Key("steps"), Index(1), Key("text")], &mut memo).is_none());
        // Item 2 has no discriminant: the merged-arm fallback keeps both keys.
        assert!(resolve(&[Key("steps"), Index(2), Key("run")], &mut memo).is_some());
        assert!(resolve(&[Key("steps"), Index(2), Key("text")], &mut memo).is_some());

        // Expression values reach the selected arm's list-item value only.
        let ast = overlay_ast(ctx).unwrap();
        let pointers: Vec<String> = expression_values(ctx, ast)
            .iter()
            .map(|value| value.entry.pointer.clone())
            .collect();
        assert_eq!(pointers, vec!["/steps/1/run", "/steps/2/run"]);
    });
}

#[test]
fn transparent_crossing_serves_key_only_authoring_ancestry() {
    let text = "---\ninitialize:\n  stack:\n    - when: ok\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let shape = known_shape(ctx);
        let keys = [Key("initialize"), Key("stack"), Key("when")];
        let mut transparent = ShapeMemo::new(ArrayCrossing::Transparent);
        assert!(def_at_path_ctx(ctx, &shape, &keys, &mut transparent).is_some());
        let mut explicit = ShapeMemo::new(ArrayCrossing::Explicit);
        assert!(def_at_path_ctx(ctx, &shape, &keys, &mut explicit).is_none());
        let indexed = [Key("initialize"), Key("stack"), Index(0), Key("when")];
        assert!(def_at_path_ctx(ctx, &shape, &indexed, &mut explicit).is_some());
        assert!(def_at_path_ctx(ctx, &shape, &indexed, &mut transparent).is_none());
    });
}

// ── Predicate schema ────────────────────────────────────────────────────────

#[test]
fn shipped_claudine_predicates_are_expression_typed() {
    let root = claudine_root();
    for path in [
        vec![Key("initialize"), Key("stack"), Index(0), Key("when")],
        vec![Key("failure"), Key("stack"), Index(2), Key("when")],
        vec![Key("loop"), Key("while")],
        vec![Key("loop"), Key("until")],
        vec![Key("loop"), Key("stack"), Index(0), Key("when")],
    ] {
        let def = def_at_path(&root, &path).unwrap_or_else(|| panic!("{path:?} resolves"));
        assert!(is_expression_def(def), "{} is expression-typed", format_dotted(&path));
    }
    // Communication fields stay strings: they are interpolated, not conditions.
    let say = def_at_path(&root, &[Key("success"), Key("say")]).unwrap();
    assert!(!is_expression_def(say));
}

#[test]
fn predicates_inside_stack_items_reach_expression_values_without_executing() {
    let text = concat!(
        "---\n",
        "initialize:\n",
        "  stack:\n",
        "    - when: \"$(touch /tmp/dmls-must-not-run) == 'x'\"\n",
        "      action: stop\n",
        "    - when: err.kind == 'timeout'\n",
        "      action: stop\n",
        "loop:\n",
        "  until: '{{ ctx.area }}'\n",
        "  say: done\n",
        "---\n\nbody\n",
    );
    with_ctx(text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        let values = expression_values(ctx, ast);
        let reached: Vec<(&str, &str)> = values
            .iter()
            .map(|value| (value.entry.dotted.as_str(), value.expression()))
            .collect();
        assert_eq!(
            reached,
            vec![
                ("initialize.stack[0].when", "$(touch /tmp/dmls-must-not-run) == 'x'"),
                ("initialize.stack[1].when", "err.kind == 'timeout'"),
                ("loop.until", "{{ ctx.area }}"),
            ]
        );
    });
}

// ── Shape memo ──────────────────────────────────────────────────────────────

#[test]
fn expression_pass_materializes_each_distinct_ancestor_shape_once() {
    let text = include_str!("../../../tests/fixtures/sequence_descent/implement-plan.md");
    with_ctx(text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        // The distinct step prefixes a memoized pass may materialize, and the
        // per-entry ancestor levels an unmemoized pass would.
        let mut distinct: HashSet<Vec<FmPathSegment<'_>>> = HashSet::new();
        let mut unmemoized = 0;
        for (index, entry) in ast.entries().iter().enumerate() {
            if entry.kind != FmValueKind::Scalar {
                continue;
            }
            let path = ast.path_at(index);
            let Some(leaf_at) = path.iter().rposition(|segment| matches!(segment, Key(_))) else {
                continue;
            };
            for end in 1..=leaf_at {
                if matches!(path.get(end), Some(Key(_)) | None) {
                    distinct.insert(path[..end].to_vec());
                    unmemoized += 1;
                }
            }
        }
        assert!(ast.entries().iter().any(|entry| entry.depth >= 4), "fixture is sequence-deep");

        SHAPE_MATERIALIZATIONS.with(|count| count.set(0));
        let values = expression_values(ctx, ast);
        let materialized = SHAPE_MATERIALIZATIONS.with(std::cell::Cell::get);
        assert!(!values.is_empty(), "stack predicates are reached");
        assert!(
            materialized <= distinct.len(),
            "{materialized} shape materializations for {} distinct ancestor paths",
            distinct.len()
        );
        assert!(materialized < unmemoized, "{materialized} vs {unmemoized} without the memo");
    });
}

// ── Capability boundaries ───────────────────────────────────────────────────

const STACK_DOC: &str = concat!(
    "---\n",
    "initialize:\n",
    "  stack:\n",
    "    - when: ctx.today\n",
    "      action: stop\n",
    "---\n\nbody\n",
);

#[test]
fn hover_works_at_list_item_key_and_value() {
    with_ctx(STACK_DOC, true, |ctx| {
        let key_offset = STACK_DOC.find("when:").unwrap() + 1;
        let key_hover = hover(ctx, key_offset).expect("hover on a list-item key");
        assert!(hover_text(&key_hover).contains("Type: **expression**"), "{}", hover_text(&key_hover));
        let when_start = STACK_DOC.find("when").unwrap();
        assert_eq!(key_hover.range, ctx.source_map.byte_range_to_lsp(when_start..when_start + 4));

        let value_offset = STACK_DOC.find("ctx.today").unwrap() + 5;
        let value_hover = hover(ctx, value_offset).expect("hover on a list-item expression value");
        let today = expressions::ctx_descriptor("today").unwrap();
        assert!(hover_text(&value_hover).contains(&expressions::format_ctx_hover_block(today)));
    });
}

#[test]
fn hover_on_scalar_list_item_describes_the_item_type() {
    let text = "---\n$schema:\n  tags: string[]\ntags:\n  - alpha\n---\n\nbody\n";
    with_ctx(text, false, |ctx| {
        let offset = text.find("alpha").unwrap() + 2;
        let hover = hover(ctx, offset).expect("hover on a scalar item");
        assert!(hover_text(&hover).contains("**`tags[0]`**"), "{}", hover_text(&hover));
        assert!(hover_text(&hover).contains("Type: **string**"), "{}", hover_text(&hover));
        let start = text.find("alpha").unwrap();
        assert_eq!(hover.range, ctx.source_map.byte_range_to_lsp(start..start + 5));
    });
}

#[test]
fn hover_is_silent_on_synthetic_item_positions() {
    with_ctx(STACK_DOC, true, |ctx| {
        // The `- ` marker of the stack item.
        let marker = STACK_DOC.find("- when").unwrap();
        assert!(hover(ctx, marker).is_none(), "{:?}", hover(ctx, marker));
    });
    let text = "---\n$schema:\n  tags: string[]\ntags:\n  - alpha\n---\n\nbody\n";
    with_ctx(text, false, |ctx| {
        let marker = text.find("- alpha").unwrap();
        assert!(hover(ctx, marker).is_none());
    });
}

#[test]
fn completion_works_inside_list_items() {
    // A key being authored on a blank line inside an existing item mapping.
    let text = "---\ninitialize:\n  stack:\n    - when: ok\n      \n      no_error: true\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let offset = text.find("ok\n").unwrap() + 3 + 6;
        let items = completion(ctx, offset);
        assert!(items.iter().any(|item| item.label == "action"), "{items:#?}");
    });
    // A list-item predicate value offers the expression catalog.
    let text = "---\ninitialize:\n  stack:\n    - when: ctx.\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let offset = text.find("ctx.").unwrap() + 4;
        let items = completion(ctx, offset);
        assert!(items.iter().any(|item| item.label == "ctx.packages"), "{items:#?}");
    });
}

#[test]
fn completion_never_offers_synthetic_item_indices() {
    let text = "---\n$schema:\n  tags: string[]\ntags:\n  - alpha\n  - beta\n  - \n---\n\nbody\n";
    with_ctx(text, false, |ctx| {
        let offset = text.find("  - \n").unwrap() + 4;
        let items = completion(ctx, offset);
        assert!(
            items.iter().all(|item| !["0", "1", "2"].contains(&item.label.as_str())),
            "{items:#?}"
        );
    });
    with_ctx(STACK_DOC, true, |ctx| {
        let marker = STACK_DOC.find("- when").unwrap() + 1;
        let items = completion(ctx, marker);
        assert!(items.iter().all(|item| item.label != "0"), "{items:#?}");
    });
}

#[test]
fn navigation_works_at_list_item_file_values_only() {
    let text = "---\n$schema:\n  refs: file[]\nrefs:\n  - ./a.md\n  - ./b.md\n---\n\nbody\n";
    with_ctx(text, false, |ctx| {
        // `nav_targets` backs both definition and document links; the virtual
        // `/w` root has no file URI on Windows, so assert the resolved paths.
        let ast = overlay_ast(ctx).unwrap();
        let targets = nav_targets(ctx, ast);
        let resolved: Vec<(&str, &Path)> = targets
            .iter()
            .map(|(span, path)| (&text[span.clone()], path.as_path()))
            .collect();
        assert_eq!(resolved.len(), 2, "{targets:#?}");
        assert_eq!(resolved[1].0, "./b.md");
        assert!(resolved[1].1.ends_with(Path::new("prompts").join("b.md")), "{targets:#?}");

        let marker = text.find("- ./b.md").unwrap();
        assert!(definition(ctx, marker).is_empty());
        let key = text.find("refs:\n").unwrap() + 1;
        assert!(definition(ctx, key).is_empty());
    });
}

#[test]
fn expression_diagnostics_newly_reach_list_items() {
    // Descent widens the existing producers into list items, at the severity
    // ladder's frontmatter severities.
    let text = concat!(
        "---\n",
        "initialize:\n",
        "  stack:\n",
        "    - when: '1 +'\n",
        "      action: stop\n",
        "    - when: nope_root\n",
        "      action: stop\n",
        "---\n\nbody\n",
    );
    with_ctx(text, true, |ctx| {
        let diagnostics = crate::diagnostics::frontmatter::diagnostics(ctx);
        let code = |diagnostic: &Diagnostic| match &diagnostic.code {
            Some(lsp_types::NumberOrString::String(code)) => code.clone(),
            _ => String::new(),
        };
        let malformed: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| code(d) == crate::diagnostics::codes::code::EXPRESSION_MALFORMED)
            .collect();
        assert_eq!(malformed.len(), 1, "{diagnostics:#?}");
        assert_eq!(malformed[0].range.start.line, 3);
        assert_eq!(malformed[0].severity, Some(lsp_types::DiagnosticSeverity::ERROR));
        let unknown: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| code(d) == crate::diagnostics::codes::code::EXPRESSION_UNKNOWN_IDENTIFIER)
            .collect();
        assert_eq!(unknown.len(), 1, "{diagnostics:#?}");
        assert_eq!(unknown[0].range.start.line, 5);
        assert!(unknown[0].message.contains("nope_root"));
        assert_eq!(unknown[0].severity, Some(lsp_types::DiagnosticSeverity::WARNING));
        // The generic format problem for the malformed value stays suppressed.
        assert!(
            diagnostics.iter().all(|d| d.source.as_deref() != Some(crate::diagnostics::codes::source::SCHEMA)),
            "{diagnostics:#?}"
        );
    });
}
