//! Sequence descent (spec D6) through the frontmatter provider: typed schema
//! paths, Claudine's predicate typing (from `claudine/schemas/`), the per-pass shape memo, and
//! the hover/completion/navigation boundary at real list items versus
//! synthetic item positions.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::schemas::resolve::resolve_schema;
use lsp_types::{InitializeParams, Uri};

use super::*;
use crate::capabilities::ClientProfile;
use biscuit_file::FileResolutionContext;
use crate::config::{DmlsConfig, SchemaExtensionConfig};
use crate::graph::WorkspaceGraph;
use crate::overlay::{OverlayState, format_dotted};
use crate::source_map::{PositionEncoding, SourceMap};

use FmPathSegment::{Index, Key};

/// The Claudine schema files the `claudine.yaml` catalog's payload reads.
const CLAUDINE_BUNDLE: [(&str, &str); 4] = [
    ("claudine.yaml", include_str!("../../../../../claudine/schemas/claudine.yaml")),
    ("lifecycle.yaml", include_str!("../../../../../claudine/schemas/lifecycle.yaml")),
    ("action.yaml", include_str!("../../../../../claudine/schemas/action.yaml")),
    ("linking.yaml", include_str!("../../../../../claudine/schemas/linking.yaml")),
];

/// A temporary folder holding Claudine's catalog payload as a plain `$schema`
/// document beside the libraries it imports, and that document's path.
fn claudine_schema() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = crate::context::test_support::claudine_payload_extension(dir.path(), &CLAUDINE_BUNDLE);
    (dir, path)
}

/// The resolved shape (imports expanded) of the schema file at `path`.
fn resolved_root(path: &Path) -> SchemaShape {
    resolved_root_with_definitions(path).0
}

/// [`resolved_root`] with the recursive definitions its shape references.
fn resolved_root_with_definitions(path: &Path) -> (SchemaShape, Definitions) {
    let base = path.parent().unwrap();
    let resolved = resolve_schema(
        &Value::String(path.to_string_lossy().into_owned()),
        base,
        &FileResolutionContext::new(base),
    )
    .unwrap_or_else(|error| panic!("{} resolves: {error}", path.display()));
    match resolved.simplified {
        Some(SimplifiedSchema::Single(shape)) => (shape, Definitions(resolved.definitions)),
        other => panic!("{} is a single shape: {other:?}", path.display()),
    }
}

/// A resolved schema file's definition table, opened by an editor walk.
struct Definitions(Arc<darkmatter::markdown::schemas::simplified::DefinitionTable>);

impl DefinitionLookup for Definitions {
    fn lookup_definition(
        &self,
        key: &darkmatter::markdown::schemas::simplified::DefKey,
    ) -> Option<&darkmatter::markdown::schemas::simplified::Definition> {
        self.0.get(key)
    }
}

/// Claudine's catalog payload's resolved shape.
fn claudine_root() -> SchemaShape {
    claudine_root_with_definitions().0
}

/// [`claudine_root`] with the recursive lifecycle families it references.
fn claudine_root_with_definitions() -> (SchemaShape, Definitions) {
    let (_dir, path) = claudine_schema();
    resolved_root_with_definitions(&path)
}

/// Runs `f` with an overlay-backed context for `text` at `/w/prompts/doc.md`,
/// with Claudine's catalog payload active as an extension when `claudine` is
/// set.
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
    let _schemas = claudine.then(|| {
        let (dir, path) = claudine_schema();
        config.schema.extensions.insert(
            "claudine".to_string(),
            SchemaExtensionConfig { path, globs: vec!["**/*.md".to_string()] },
        );
        dir
    });
    let roots = [root.to_path_buf()];
    let state = OverlayState::default();
    let resolution = if root == Path::new("/w") {
        crate::context::test_support::resolution_for(path)
    } else {
        crate::context::test_support::resolution_in(root, path)
    };
    let overlay = state.for_document(&uri, text, path, &config, &roots, &resolution);
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
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
        resolution: &resolution,
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
    // A stack is `item[] | entry`, a recursive definition: an index descends
    // the array arm's item type, whose first inline-object arm is the
    // `communicate` category; a bare key descends the dictionary arm.
    let (root, definitions) = claudine_root_with_definitions();
    let item = nested_shape(&root, &[Key("initialize"), Index(0)], &definitions).expect("list item shape");
    for key in ["say", "notify", "info"] {
        assert!(item.properties.contains_key(key), "{key} on the list item");
    }
    assert!(!item.properties.contains_key("ensure_file"), "the item's first arm is a category, not the bundle");
    let bundle = nested_shape(&root, &[Key("failure")], &definitions).expect("bundle shape");
    assert!(is_expression_def(bundle.properties.get("when").expect("failure.when")));
    let nested = def_at_path(&root, &[Key("loop"), Key("gate"), Index(3), Key("say")], &definitions)
        .expect("loop.gate[3].say resolves");
    assert!(!is_expression_def(nested));
}

#[test]
fn nested_shape_fails_closed_on_mismatched_segments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("types.yaml");
    std::fs::write(
        &path,
        "$schema:\n  steps: \"{ when: expression, say: string }[]\"\n  opts:\n    level: string\n",
    )
    .unwrap();
    let root = resolved_root(&path);
    assert!(def_at_path(&root, &[Key("steps"), Index(0), Key("when")], &NoDefinitions).is_some());
    // A key where the array item is required (`steps` is list-only).
    assert!(nested_shape(&root, &[Key("steps")], &NoDefinitions).is_none());
    assert!(def_at_path(&root, &[Key("steps"), Key("when")], &NoDefinitions).is_none());
    // An index under a non-array property.
    assert!(nested_shape(&root, &[Key("opts"), Index(0)], &NoDefinitions).is_none());
    // A leading index, and a nested index SimplifiedSchema cannot declare.
    assert!(nested_shape(&root, &[Index(0)], &NoDefinitions).is_none());
    assert!(nested_shape(&root, &[Key("steps"), Index(0), Index(0)], &NoDefinitions).is_none());
    // A key literally named `0` is not an index.
    assert!(nested_shape(&root, &[Key("steps"), Key("0")], &NoDefinitions).is_none());
}

#[test]
fn union_arms_are_chosen_by_array_crossing() {
    // `exit_expressions` is a rule array or a `{ mode, rules }` layer.
    let root = claudine_root();
    let layer = nested_shape(&root, &[Key("exit_expressions")], &NoDefinitions).expect("non-array layer arm");
    assert!(layer.properties.contains_key("mode"));
    let rule = nested_shape(&root, &[Key("exit_expressions"), Index(0)], &NoDefinitions).expect("array rule arm");
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
    // `then` exists only on the conditional arm of `loop.gate`'s array items,
    // never on its non-array `entry` (category or bundle) arm.
    let text = "---\nloop:\n  gate:\n    - when: ok\n      then: [stop]\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let shape = known_shape(ctx);
        let keys = [Key("loop"), Key("gate"), Key("then")];
        let mut transparent = ShapeMemo::new(ArrayCrossing::Transparent);
        assert!(def_at_path_ctx(ctx, &shape, &keys, &mut transparent).is_some());
        let mut explicit = ShapeMemo::new(ArrayCrossing::Explicit);
        assert!(def_at_path_ctx(ctx, &shape, &keys, &mut explicit).is_none());
        let indexed = [Key("loop"), Key("gate"), Index(0), Key("then")];
        assert!(def_at_path_ctx(ctx, &shape, &indexed, &mut explicit).is_some());
        assert!(def_at_path_ctx(ctx, &shape, &indexed, &mut transparent).is_none());
    });
}

// ── Predicate schema ────────────────────────────────────────────────────────

#[test]
fn shipped_claudine_predicates_are_expression_typed() {
    // A conditional is one arm of a stack item's union, so the predicates
    // resolve through the context-aware walk that selects it.
    let text = concat!(
        "---\n",
        "initialize:\n",
        "  - when: a\n",
        "    then: [stop]\n",
        "failure:\n",
        "  - when: b\n",
        "    then:\n",
        "      - say: x\n",
        "      - when: c\n",
        "        then: [stop]\n",
        "    else:\n",
        "      - when: d\n",
        "        then: [stop]\n",
        "  - say: y\n",
        "  - when: e\n",
        "    then: [stop]\n",
        "success:\n",
        "  say: done\n",
        "  when: f\n",
        "loop:\n",
        "  while: g\n",
        "  until: h\n",
        "  say: z\n",
        "  gate:\n",
        "    - when: i\n",
        "      then: [stop]\n",
        "---\n\nbody\n",
    );
    with_ctx(text, true, |ctx| {
        let shape = known_shape(ctx);
        let mut memo = ShapeMemo::new(ArrayCrossing::Explicit);
        for path in [
            vec![Key("initialize"), Index(0), Key("when")],
            vec![Key("failure"), Index(2), Key("when")],
            vec![Key("failure"), Index(0), Key("then"), Index(1), Key("when")],
            vec![Key("failure"), Index(0), Key("else"), Index(0), Key("when")],
            vec![Key("success"), Key("when")],
            vec![Key("loop"), Key("while")],
            vec![Key("loop"), Key("until")],
            vec![Key("loop"), Key("gate"), Index(0), Key("when")],
        ] {
            let def = def_at_path_ctx(ctx, &shape, &path, &mut memo)
                .unwrap_or_else(|| panic!("{path:?} resolves"));
            assert!(is_expression_def(&def), "{} is expression-typed", format_dotted(&path));
        }
        // Verb values are interpolated, not conditions.
        for path in [
            vec![Key("success"), Key("say")],
            vec![Key("failure"), Index(1), Key("say")],
            vec![Key("loop"), Key("say")],
        ] {
            let def = def_at_path_ctx(ctx, &shape, &path, &mut memo);
            assert!(def.is_none_or(|def| !is_expression_def(&def)), "{}", format_dotted(&path));
        }
    });
}

#[test]
fn predicates_inside_stack_items_reach_expression_values_without_executing() {
    let text = concat!(
        "---\n",
        "initialize:\n",
        "  - when: \"$(touch /tmp/dmls-must-not-run) == 'x'\"\n",
        "    then: [stop]\n",
        "  - when: err.kind == 'timeout'\n",
        "    then:\n",
        "      - when: nested_flag\n",
        "        then: [stop]\n",
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
                ("initialize[0].when", "$(touch /tmp/dmls-must-not-run) == 'x'"),
                ("initialize[1].when", "err.kind == 'timeout'"),
                ("initialize[1].then[0].when", "nested_flag"),
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
    "  - when: ctx.today\n",
    "    then: [stop]\n",
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
    let text = "---\ninitialize:\n  - when: ok\n    \n    no_error: true\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let offset = text.find("ok\n").unwrap() + 3 + 4;
        let items = completion(ctx, offset);
        assert!(items.iter().any(|item| item.label == "then"), "{items:#?}");
    });
    // A list-item predicate value offers the expression catalog.
    let text = "---\ninitialize:\n  - when: ctx.\n---\n\nbody\n";
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
        "  - when: '1 +'\n",
        "    then: [stop]\n",
        "  - when: nope_root\n",
        "    then: [stop]\n",
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
        assert_eq!(malformed[0].range.start.line, 2);
        assert_eq!(malformed[0].severity, Some(lsp_types::DiagnosticSeverity::ERROR));
        let unknown: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| code(d) == crate::diagnostics::codes::code::EXPRESSION_UNDECLARED_PROPERTY)
            .collect();
        assert_eq!(unknown.len(), 1, "{diagnostics:#?}");
        assert_eq!(unknown[0].range.start.line, 4);
        assert!(unknown[0].message.contains("nope_root"));
        assert_eq!(unknown[0].severity, Some(lsp_types::DiagnosticSeverity::WARNING));
        // The generic format problem for the malformed value stays suppressed.
        assert!(
            diagnostics.iter().all(|d| d.source.as_deref() != Some(crate::diagnostics::codes::source::SCHEMA)),
            "{diagnostics:#?}"
        );
    });
}
