//! Sequence descent (spec D6) through the frontmatter provider: typed schema
//! paths, Claudine's predicate typing (from `claudine/schemas/`), shape sharing through the
//! snapshot's analysis, and
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
use crate::work_counters::{WorkCounts, work_counts};
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
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let resolve = |path: &[FmPathSegment<'static>], memo: &mut ShapeMemo<'_, 'static>| memo.def_at(path).is_some();
        // Item 0 selects the note arm: `run` does not exist there.
        assert!(resolve(&[Key("steps"), Index(0), Key("text")], &mut memo));
        assert!(!resolve(&[Key("steps"), Index(0), Key("run")], &mut memo));
        // Item 1 selects the shell arm.
        let run = memo.def_at(&[Key("steps"), Index(1), Key("run")]).expect("shell run");
        assert!(is_expression_def(run));
        assert!(!resolve(&[Key("steps"), Index(1), Key("text")], &mut memo));
        // Item 2 has no discriminant: the merged-arm fallback keeps both keys.
        assert!(resolve(&[Key("steps"), Index(2), Key("run")], &mut memo));
        assert!(resolve(&[Key("steps"), Index(2), Key("text")], &mut memo));

        // Expression values reach the selected arm's list-item value only.
        let pointers: Vec<String> = expression_values(ctx)
            .iter()
            .map(|value| value.entry.pointer.clone())
            .collect();
        assert_eq!(pointers, vec!["/steps/1/run", "/steps/2/run"]);
    });
}

#[test]
fn sibling_items_share_their_selected_arm_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("types.yaml"),
        concat!(
            "$schema:\n",
            "  shell-step:\n",
            "    kind: literal(shell)\n",
            "    run: expression\n",
            "  count-step:\n",
            "    kind: literal(2)\n",
            "    count: number\n",
        ),
    )
    .unwrap();
    let text = concat!(
        "---\n",
        "$schema:\n",
        "  steps:\n",
        "    - \"shell-step[]@./types.yaml\"\n",
        "    - \"count-step[]@./types.yaml\"\n",
        "steps:\n",
        "  - kind: shell\n",
        "  - kind: 2\n",
        "  - kind: shell\n",
        "  - kind: '2'\n",
        "  - run: ctx.none\n",
        "  - kind: [shell]\n",
        "---\n\nbody\n",
    );
    let doc = dir.path().join("doc.md");
    let uri: Uri = url::Url::from_file_path(&doc).unwrap().as_str().parse().unwrap();
    with_ctx_at(&doc, uri, dir.path(), text, false, |ctx| {
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let mut item = |index: usize| {
            let shape = memo.shape(&[Key("steps"), Index(index)]).expect("item shape");
            (std::ptr::from_ref(shape), shape.properties.keys().cloned().collect::<Vec<_>>())
        };
        let before = work_counts();
        let (shell, shell_keys) = item(0);
        let (count, count_keys) = item(1);
        assert_eq!(item(2).0, shell, "the same selected arm is one shape");
        assert_ne!(shell, count, "a different arm is a different shape");
        assert_eq!((shell_keys, count_keys), (vec!["kind".to_string(), "run".into()], vec!["kind".to_string(), "count".into()]));
        // A quoted '2' is a string, so it selects no numeric arm: the merged
        // view, in arm then property order, shared by every unselected item.
        let (merged, merged_keys) = item(3);
        assert_eq!(merged_keys, ["kind", "run", "count"]);
        assert_eq!(item(4).0, merged);
        assert_eq!(item(5).0, merged);
        assert_ne!(merged, shell);
        assert_eq!(work_counts().since(before).merged_shapes, 1);
    });
}

#[test]
fn every_discriminant_shape_selects_per_item_within_one_snapshot() {
    // `kind` and `mode` are shared literal discriminants; `kind: shell` is a
    // duplicate literal (two arms declare it), so only `mode` selects those.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("types.yaml"),
        concat!(
            "$schema:\n",
            "  shell-step:\n",
            "    kind: literal(shell)\n",
            "    mode: literal(a)\n",
            "    run: expression\n",
            "  alias-step:\n",
            "    kind: literal(shell)\n",
            "    mode: literal(d)\n",
            "    alias: string\n",
            "  count-step:\n",
            "    kind: literal(2)\n",
            "    mode: literal(b)\n",
            "    count: number\n",
            "  flag-step:\n",
            "    kind: literal(true)\n",
            "    mode: literal(c)\n",
            "    on: string\n",
        ),
    )
    .unwrap();
    let cases: [(&str, Option<&str>); 12] = [
        ("mode: a", Some("run")),
        ("kind: 2", Some("count")),
        ("kind: 2, mode: b", Some("count")),
        ("kind: true", Some("on")),
        ("mode: d", Some("alias")),
        ("kind: 'true'", None),
        ("kind: '2'", None),
        ("kind: null", None),
        ("run: ctx.none", None),
        ("kind: other", None),
        ("kind: shell", None),
        ("kind: 2, mode: c", None),
    ];
    let mut text = String::from("---\n$schema:\n  steps:\n");
    for arm in ["shell", "alias", "count", "flag"] {
        text.push_str(&format!("    - \"{arm}-step[]@./types.yaml\"\n"));
    }
    text.push_str("steps:\n");
    for (mapping, _) in cases {
        text.push_str(&format!("  - {{ {mapping} }}\n"));
    }
    text.push_str("  - kind: true\n    \n");
    text.push_str("  - kind: '2'\n    \n");
    text.push_str("---\n\nbody\n");
    let doc = dir.path().join("doc.md");
    let uri: Uri = url::Url::from_file_path(&doc).unwrap().as_str().parse().unwrap();
    with_ctx_at(&doc, uri, dir.path(), &text, false, |ctx| {
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let before = work_counts();
        let mut merged: Option<*const SchemaShape> = None;
        let mut selected: BTreeMap<&str, *const SchemaShape> = BTreeMap::new();
        for (index, (mapping, own_key)) in cases.iter().enumerate() {
            let shape = memo.shape(&[Key("steps"), Index(index)]).expect("item shape");
            let keys: Vec<&str> = shape.properties.keys().map(String::as_str).collect();
            match own_key {
                Some(own) => {
                    assert_eq!(keys.len(), 3, "{mapping} selects one arm: {keys:?}");
                    assert!(keys.contains(own), "{mapping} selects the `{own}` arm: {keys:?}");
                    let first = *selected.entry(own).or_insert(std::ptr::from_ref(shape));
                    assert_eq!(first, std::ptr::from_ref(shape), "{mapping} shares its arm's shape");
                }
                None => {
                    // Every arm's keys, in arm then property order.
                    assert_eq!(keys, ["kind", "mode", "run", "alias", "count", "on"], "{mapping}");
                    let view = *merged.get_or_insert(std::ptr::from_ref(shape));
                    assert_eq!(view, std::ptr::from_ref(shape), "{mapping} shares the merged view");
                }
            }
        }
        assert_eq!(selected.len(), 4, "four distinct selected arms: {selected:?}");
        assert_eq!(work_counts().since(before).merged_shapes, 1, "one merged view for every unselected item");

        // Completion on the same snapshot offers each item its own arm.
        let labels = |line: &str| -> Vec<String> {
            let offset = text.find(line).unwrap() + line.len();
            completion(ctx, offset).into_iter().map(|item| item.label).collect()
        };
        let flag = labels("  - kind: true\n    ");
        assert!(flag.iter().any(|label| label == "on"), "{flag:?}");
        assert!(!flag.iter().any(|label| label == "count" || label == "run"), "{flag:?}");
        let unselected = labels("  - kind: '2'\n    ");
        for key in ["run", "alias", "count", "on"] {
            assert!(unselected.iter().any(|label| label == key), "merged-arm completion offers {key}: {unselected:?}");
        }
    });
}

#[test]
fn transparent_crossing_serves_key_only_authoring_ancestry() {
    // `then` exists only on the conditional arm of `loop.gate`'s array items,
    // never on its non-array `entry` (category or bundle) arm.
    let text = "---\nloop:\n  gate:\n    - when: ok\n      then: [stop]\n---\n\nbody\n";
    with_ctx(text, true, |ctx| {
        let keys = [Key("loop"), Key("gate"), Key("then")];
        let mut transparent = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
        assert!(transparent.def_at(&keys).is_some());
        let mut explicit = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        assert!(explicit.def_at(&keys).is_none());
        let indexed = [Key("loop"), Key("gate"), Index(0), Key("then")];
        assert!(explicit.def_at(&indexed).is_some());
        assert!(transparent.def_at(&indexed).is_none());
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
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
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
            let def = memo.def_at(&path).unwrap_or_else(|| panic!("{path:?} resolves"));
            assert!(is_expression_def(def), "{} is expression-typed", format_dotted(&path));
        }
        // Verb values are interpolated, not conditions.
        for path in [
            vec![Key("success"), Key("say")],
            vec![Key("failure"), Index(1), Key("say")],
            vec![Key("loop"), Key("say")],
        ] {
            let def = memo.def_at(&path);
            assert!(def.is_none_or(|def| !is_expression_def(def)), "{}", format_dotted(&path));
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
        let values = expression_values(ctx);
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
fn expression_pass_builds_each_shared_construction_once_per_snapshot() {
    let text = include_str!("../../../tests/fixtures/sequence_descent/implement-plan.md");
    with_ctx(text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        // The distinct ancestor paths the old per-path memo built a shape for.
        let mut distinct: HashSet<Vec<FmPathSegment<'_>>> = HashSet::new();
        let mut scalars = 0;
        for (index, entry) in ast.entries().iter().enumerate() {
            if entry.kind != FmValueKind::Scalar {
                continue;
            }
            scalars += 1;
            let path = ast.path_at(index);
            let Some(leaf_at) = path.iter().rposition(|segment| matches!(segment, Key(_))) else {
                continue;
            };
            for end in 1..=leaf_at {
                if matches!(path.get(end), Some(Key(_)) | None) {
                    distinct.insert(path[..end].to_vec());
                }
            }
        }
        assert!(ast.entries().iter().any(|entry| entry.depth >= 4), "fixture is sequence-deep");

        let before = work_counts();
        let values = expression_values(ctx);
        let first = work_counts().since(before);
        assert!(!values.is_empty(), "stack predicates are reached");
        assert_eq!(first.root_assemblies, 1, "{first:?}");
        assert_eq!((first.typing_walks, first.scalar_classifications), (1, scalars));
        let built = first.merged_shapes + first.item_syntheses;
        assert!(built > 0, "the fixture reaches merged item views: {first:?}");
        assert!(
            built < distinct.len(),
            "{built} constructions for {} distinct ancestor paths",
            distinct.len()
        );

        // A second pass on the same snapshot builds nothing.
        let before = work_counts();
        let again = expression_values(ctx);
        let second = work_counts().since(before);
        assert_eq!(again.len(), values.len());
        assert_eq!(
            (second.root_assemblies, second.merged_shapes, second.item_syntheses),
            (0, 0, 0),
            "{second:?}"
        );
        assert_eq!(
            (second.typing_walks, second.scalar_classifications),
            (0, 0),
            "the second pass reads the stored inventory: {second:?}"
        );
    });
}

/// The arena index of the entry at JSON `pointer`.
fn index_at(ast: &FrontmatterAst, pointer: &str) -> usize {
    ast.entries().iter().position(|entry| entry.pointer == pointer).unwrap_or_else(|| panic!("{pointer}"))
}

#[test]
fn point_typing_never_starts_the_inventory_and_the_inventory_reuses_it() {
    let mut text = repeated_lifecycle(5);
    text = text.replacen("---\nfailure:\n", "---\nmystery: x\nfailure:\n", 1);
    text = text.replacen("      - say: hi\n", "      - say: \"{{ err.message }}\"\n", 1);
    with_ctx(&text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        let scalars = ast.entries().iter().filter(|entry| entry.kind == FmValueKind::Scalar).count();
        let guard = &ast.entries()[index_at(ast, "/failure/0/when")];
        let say = &ast.entries()[index_at(ast, "/failure/0/then/0/say")];
        let unknown = index_at(ast, "/mystery");

        // Expression hover, then the stack hover of an untyped `{{ … }}`
        // span: each types the cursor's scalar alone.
        for (entry, label) in [(guard, "guard"), (say, "span")] {
            let before = work_counts();
            let hover = hover(ctx, entry.value_span.start + 1);
            let work = work_counts().since(before);
            assert!(hover.is_some(), "{label} hovers");
            assert_eq!(
                (work.typing_walks, work.stack_walks, work.scalar_classifications),
                (0, 0, 1),
                "{label}: {work:?}"
            );
        }

        // Negative outcomes are stored too: neither lookup is repeated.
        let before = work_counts();
        for _ in 0..2 {
            let unresolved = scalar_typing(ctx, unknown).expect("typed");
            assert!(!unresolved.file && matches!(unresolved.expression, ExpressionTyping::No));
            let plain = scalar_typing(ctx, index_at(ast, "/failure/1/then/0/say")).expect("typed");
            assert!(matches!(plain.expression, ExpressionTyping::No), "{plain:?}");
        }
        assert_eq!(work_counts().since(before).scalar_classifications, 2);

        // The inventory classifies only what no point request did, once.
        let before = work_counts();
        let values = expression_values(ctx);
        let first = work_counts().since(before);
        assert_eq!(values.len(), 5 * 2, "every guard is typed");
        assert_eq!((first.typing_walks, first.scalar_classifications), (1, scalars - 4), "{first:?}");
        let before = work_counts();
        assert_eq!(expression_values(ctx).len(), values.len());
        let _ = nav_targets(ctx);
        let _ = hover(ctx, guard.value_span.start + 1);
        let again = work_counts().since(before);
        assert_eq!((again.typing_walks, again.scalar_classifications), (0, 0), "{again:?}");
    });

    // Point and document access agree, each on its own fresh snapshot.
    let point: Vec<String> = with_ctx(&text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        (0..ast.entries().len())
            .filter(|&index| ast.entries()[index].kind == FmValueKind::Scalar)
            .map(|index| format!("{index}: {:?}", scalar_typing(ctx, index).expect("typed")))
            .collect()
    });
    let inventory: Vec<String> = with_ctx(&text, true, |ctx| {
        scalar_inventory(ctx).map(|(index, typing)| format!("{index}: {typing:?}")).collect()
    });
    assert_eq!(point, inventory);
    assert!(point.iter().any(|line| line.contains("Decoded")), "{point:#?}");
}

#[test]
fn definition_types_only_the_cursor_scalar() {
    let text = "---\n$schema:\n  refs: file[]\n  note: string\nnote: ./c.md\nrefs:\n  - ./a.md\n  - ./b.md\n---\n\nbody\n";
    with_ctx(text, false, |ctx| {
        let offset = text.find("./b.md").unwrap() + 1;
        let before = work_counts();
        let found = definition(ctx, offset);
        let work = work_counts().since(before);
        assert_eq!((work.typing_walks, work.scalar_classifications), (0, 1), "{work:?}");
        // Document links see the same target, through the inventory.
        let expected: Vec<Uri> = nav_targets(ctx)
            .into_iter()
            .filter(|(span, _)| span.contains(&offset))
            .filter_map(|(_, path)| file_path_to_uri(&path))
            .collect();
        assert_eq!(found.into_iter().map(|location| location.uri).collect::<Vec<_>>(), expected);
        assert_eq!(nav_targets(ctx).len(), 2, "both list items, never the string-typed `note`");
        // A cursor on a string-typed value has no target.
        assert!(definition(ctx, text.find("./c.md").unwrap() + 1).is_empty());
    });
}

/// A lifecycle document with `items` identical nested guard items.
fn repeated_lifecycle(items: usize) -> String {
    let mut text = String::from("---\nfailure:\n");
    for index in 0..items {
        text.push_str(&format!(
            "  - when: flag_{index}\n    then:\n      - say: hi\n      - when: deeper\n        then: [stop]\n"
        ));
    }
    text.push_str("---\n\nbody\n");
    text
}

#[test]
fn identical_items_do_not_multiply_shared_constructions() {
    let built = |items: usize| {
        let text = repeated_lifecycle(items);
        with_ctx(&text, true, |ctx| {
            let before = work_counts();
            let values = expression_values(ctx);
            assert_eq!(values.len(), items * 2, "every guard is typed");
            work_counts().since(before)
        })
    };
    let (small, large) = (built(10), built(20));
    assert!(small.merged_shapes > 0, "{small:?}");
    assert_eq!(
        (small.root_assemblies, small.merged_shapes, small.item_syntheses),
        (large.root_assemblies, large.merged_shapes, large.item_syntheses),
        "doubling identical items doubles no construction"
    );
    assert!(large.scalar_classifications > small.scalar_classifications, "classification is per entry");
}

#[test]
fn recursive_depths_and_same_arm_items_share_one_view() {
    let text = concat!(
        "---\n",
        "failure:\n",
        "  - when: a\n",
        "    then:\n",
        "      - when: b\n",
        "        then:\n",
        "          - when: c\n",
        "            then:\n",
        "              - when: d\n",
        "                then: [stop]\n",
        "  - when: e\n",
        "    then: [stop]\n",
        "---\n\nbody\n",
    );
    with_ctx(text, true, |ctx| {
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let depth = |memo: &mut ShapeMemo<'_, 'static>, path: &[FmPathSegment<'static>]| {
            std::ptr::from_ref(memo.shape(path).unwrap_or_else(|| panic!("{path:?} resolves")))
        };
        let first = depth(&mut memo, &[Key("failure"), Index(0)]);
        let sibling = depth(&mut memo, &[Key("failure"), Index(1)]);
        assert_eq!(first, sibling, "sibling items share one view");
        let before = work_counts();
        let two = depth(&mut memo, &[Key("failure"), Index(0), Key("then"), Index(0)]);
        let three = depth(&mut memo, &[Key("failure"), Index(0), Key("then"), Index(0), Key("then"), Index(0)]);
        let four = depth(
            &mut memo,
            &[Key("failure"), Index(0), Key("then"), Index(0), Key("then"), Index(0), Key("then"), Index(0)],
        );
        assert_eq!(two, three, "a recursive depth returns to the same view");
        assert_eq!(three, four);
        assert!(work_counts().since(before).merged_shapes <= 1, "one view serves every depth");
        // A fresh pass on the same snapshot reuses the views.
        let mut next = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let before = work_counts();
        assert_eq!(depth(&mut next, &[Key("failure"), Index(1)]), first);
        assert_eq!(work_counts().since(before).merged_shapes, 0);
    });
}

#[test]
fn item_definitions_are_synthesized_once_for_every_item() {
    let mut text = String::from("---\n$schema:\n  tags: string[]\ntags:\n");
    for index in 0..50 {
        text.push_str(&format!("  - tag{index}\n"));
    }
    text.push_str("---\n\nbody\n");
    with_ctx(&text, false, |ctx| {
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
        let before = work_counts();
        let first = memo.def_at(&[Key("tags"), Index(0)]).expect("item definition");
        let last = memo.def_at(&[Key("tags"), Index(49)]).expect("item definition");
        assert!(std::ptr::eq(first, last));
        let atom = primary_atom(first).expect("one atom");
        assert!(!atom.is_array && atom.array_constraints.is_empty(), "the item, not the array");
        expression_values(ctx);
        assert_eq!(work_counts().since(before).item_syntheses, 1);
    });
}

#[test]
fn the_root_is_assembled_once_and_shared() {
    with_ctx(STACK_DOC, true, |ctx| {
        let before = work_counts();
        let root = known_shape(ctx);
        assert!(std::ptr::eq(root, known_shape(ctx)));
        let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
        assert!(memo.def_at(&[Key("initialize")]).is_some());
        assert!(hover(ctx, STACK_DOC.find("when:").unwrap() + 1).is_some());
        assert_eq!(work_counts().since(before).root_assemblies, 1);
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
        let targets = nav_targets(ctx);
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

#[test]
fn every_scalar_representation_types_alike_through_point_and_inventory() {
    let text = concat!(
        "---\n",
        "$schema:\n",
        "  plain: expression\n",
        "  single: expression\n",
        "  double: expression\n",
        "  block: expression\n",
        "  tagged: expression\n",
        "  mixed:\n    - number\n    - expression\n",
        "plain: a == 1\n",
        "single: 'b == 2'\n",
        "double: \"c == 3\"\n",
        "block: |\n",
        "  d == 4\n",
        "tagged: !!str \"e == 5\"\n",
        "mixed: f > 0\n",
        "---\n\nbody\n",
    );
    let expected = [
        ("/plain", "a == 1", true),
        ("/single", "b == 2", true),
        ("/double", "c == 3", true),
        ("/block", "d == 4\n", false),
        ("/tagged", "e == 5", false),
        ("/mixed", "f > 0", true),
    ];
    with_ctx(text, false, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        // Point typing first, before any inventory exists.
        let pointed: Vec<(String, String)> = expected
            .iter()
            .map(|(pointer, _, _)| {
                let index = index_at(ast, pointer);
                let ExpressionTyping::Decoded { text, .. } = &scalar_typing(ctx, index).expect("scalar").expression else {
                    panic!("{pointer} is expression-typed");
                };
                (pointer.to_string(), text.clone())
            })
            .collect();
        let wanted: Vec<(String, String)> =
            expected.iter().map(|(pointer, decoded, _)| (pointer.to_string(), decoded.to_string())).collect();
        assert_eq!(pointed, wanted);

        // The inventory on the same snapshot holds the same values, and an
        // exactly projected value maps back to its own authored bytes.
        let before = work_counts();
        let values = expression_values(ctx);
        let scalars = ast.entries().iter().filter(|entry| entry.kind == FmValueKind::Scalar).count();
        assert_eq!(
            work_counts().since(before).scalar_classifications,
            scalars - expected.len(),
            "the inventory types only the scalars no point request did"
        );
        let inventoried: Vec<(String, String)> =
            values.iter().map(|value| (value.entry.pointer.clone(), value.expression().to_string())).collect();
        assert_eq!(inventoried, wanted);
        for (value, (pointer, decoded, exact)) in values.iter().zip(expected) {
            assert_eq!(value.is_exact(), exact, "{pointer}");
            if exact {
                assert_eq!(&text[value.expression_span()], decoded, "{pointer}");
            } else {
                assert!(value.entry.value_span.contains(&value.expression_span().start), "{pointer}");
            }
        }
    });
}

// ── Bounded work on a large lifecycle document ──────────────────────────────

/// A lifecycle document with `items` identical failure handlers, each a guard
/// expression, a `{{ … }}` span, a file verb, and a plain verb, beside as many
/// file references and one key no schema declares.
fn lifecycle_document(items: usize) -> String {
    let mut text = String::from("---\n$schema:\n  refs: file[]\nmystery: x\nrefs:\n");
    for index in 0..items {
        text.push_str(&format!("  - ./ref-{index}.md\n"));
    }
    text.push_str("failure:\n");
    for index in 0..items {
        text.push_str(&format!(
            "  - when: file_exists('plan-{index}.md')\n    then:\n      - say: \"{{{{ err.message }}}}\"\n      - ensure_file: ./out-{index}.md\n      - warn: careful\n"
        ));
    }
    text.push_str("---\n\nbody\n");
    text
}

/// The byte offsets of each cold point request inside item `item` of a
/// [`lifecycle_document`].
struct PointRequests {
    expression: usize,
    verb: usize,
    outside: usize,
    completion: usize,
    definition: usize,
}

impl PointRequests {
    fn at(text: &str, item: usize) -> Self {
        let start = text.find(&format!("file_exists('plan-{item}.md')")).expect("item");
        let local = |needle: &str| start + text[start..].find(needle).unwrap_or_else(|| panic!("{needle}"));
        Self {
            expression: start + 2,
            verb: local("ensure_file") + 2,
            outside: local("then:") + 1,
            completion: local("warn:") + 2,
            definition: text.find(&format!("./ref-{item}.md")).expect("reference") + 2,
        }
    }
}

/// The work one cold point request on item `item` of an `items`-long
/// [`lifecycle_document`] does on a fresh snapshot, after checking that it
/// answers.
fn cold_point_work(items: usize, item: usize, request: fn(&PointRequests, &DocumentContext) -> bool) -> WorkCounts {
    let text = lifecycle_document(items);
    let at = PointRequests::at(&text, item);
    with_ctx(&text, true, |ctx| {
        let before = work_counts();
        assert!(request(&at, ctx), "the request answers");
        work_counts().since(before)
    })
}

/// Asserts that a cold point request on a 200-item document types only its
/// cursor's scalar and builds only the shapes on its path, the same work it
/// does on a 20-item document.
fn assert_cold_point_request_is_bounded(request: fn(&PointRequests, &DocumentContext) -> bool) {
    // The deepest cursor path: failure → [item] → then → [verb] → leaf, so at
    // most these many distinct schema positions can be built for it.
    const PATH_POSITIONS: usize = 5;
    let large = cold_point_work(200, 100, request);
    assert_eq!((large.typing_walks, large.stack_walks), (0, 0), "{large:?}");
    assert!(large.scalar_classifications <= 1, "only the cursor scalar is typed: {large:?}");
    assert!(large.root_assemblies <= 1, "{large:?}");
    assert!(large.merged_shapes + large.item_syntheses <= PATH_POSITIONS, "{large:?}");
    let small = cold_point_work(20, 10, request);
    let typing = |work: WorkCounts| {
        (work.root_assemblies, work.merged_shapes, work.item_syntheses, work.scalar_classifications)
    };
    assert_eq!(typing(small), typing(large), "independent of the item count");
}

/// One test per cold point request, so nextest spreads them across cores.
macro_rules! cold_point_request_tests {
    ($($name:ident: $request:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_cold_point_request_is_bounded($request);
        }
    )*};
}

cold_point_request_tests! {
    cold_expression_hover_types_only_the_cursor: |at, ctx| {
        crate::providers::ProviderRegistry::with_substrate().hover(ctx, at.expression).is_some()
    };
    cold_verb_hover_types_only_the_cursor: |at, ctx| {
        crate::providers::ProviderRegistry::with_substrate().hover(ctx, at.verb).is_some()
    };
    cold_key_hover_types_no_scalar_walk: |at, ctx| {
        crate::providers::ProviderRegistry::with_substrate().hover(ctx, at.outside).is_some()
    };
    cold_completion_types_only_the_cursor: |at, ctx| {
        !crate::providers::ProviderRegistry::with_substrate().completion(ctx, at.completion).is_empty()
    };
    cold_definition_types_only_the_cursor: |at, ctx| {
        !crate::providers::ProviderRegistry::with_substrate().definition(ctx, at.definition).is_empty()
    };
}

/// Diagnostics through every provider over a fresh snapshot of an
/// `items`-long [`lifecycle_document`], with the work it did.
fn diagnostics_work(items: usize) -> (usize, WorkCounts) {
    let text = lifecycle_document(items);
    with_ctx(&text, true, |ctx| {
        let registry = crate::providers::ProviderRegistry::with_substrate();
        let before = work_counts();
        let diagnostics = registry.diagnostics(ctx);
        (diagnostics.len(), work_counts().since(before))
    })
}

#[test]
fn diagnostics_constructions_do_not_scale_with_identical_items() {
    let (small_found, small) = diagnostics_work(100);
    let (large_found, large) = diagnostics_work(200);
    assert_eq!(small_found, large_found, "identical items add no diagnostic");
    assert_eq!((small.typing_walks, small.stack_walks), (1, 1), "{small:?}");
    assert_eq!((large.typing_walks, large.stack_walks), (1, 1), "{large:?}");
    assert_eq!(
        (small.root_assemblies, small.merged_shapes, small.item_syntheses),
        (large.root_assemblies, large.merged_shapes, large.item_syntheses),
        "doubling identical items builds nothing more"
    );
    // Each item adds five scalars (four in its handler, one reference), each
    // classified once.
    assert_eq!(large.scalar_classifications - small.scalar_classifications, 100 * 5, "{small:?} {large:?}");
}

#[test]
fn diagnostics_then_point_requests_reuse_one_snapshot() {
    let text = lifecycle_document(200);
    let at = PointRequests::at(&text, 100);
    let registry = crate::providers::ProviderRegistry::with_substrate();
    let answers = |ctx: &DocumentContext| {
        format!(
            "{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
            registry.hover(ctx, at.expression),
            registry.hover(ctx, at.verb),
            registry.hover(ctx, at.outside),
            registry.completion(ctx, at.completion),
            registry.definition(ctx, at.definition),
        )
    };
    let cold = with_ctx(&text, true, |ctx| answers(ctx));
    with_ctx(&text, true, |ctx| {
        let ast = overlay_ast(ctx).unwrap();
        // A guard expression, a plain verb value, and an undeclared key.
        let probes = [
            index_at(ast, "/failure/100/when"),
            index_at(ast, "/failure/100/then/2/warn"),
            index_at(ast, "/mystery"),
        ];
        let typed = |ctx: &DocumentContext| -> Vec<String> {
            probes.iter().map(|&index| format!("{:?}", scalar_typing(ctx, index).expect("scalar"))).collect()
        };
        let before_diagnostics = typed(ctx);
        assert!(before_diagnostics[0].contains("Decoded"), "{before_diagnostics:?}");
        assert!(before_diagnostics[1].contains("No") && before_diagnostics[2].contains("No"), "{before_diagnostics:?}");

        let before = work_counts();
        let first = registry.diagnostics(ctx);
        let work = work_counts().since(before);
        assert_eq!(work.typing_walks, 1, "{work:?}");

        // A repeated publication reuses the inventory.
        let before = work_counts();
        assert_eq!(registry.diagnostics(ctx), first);
        let again = work_counts().since(before);
        assert_eq!(
            (again.typing_walks, again.scalar_classifications, again.root_assemblies, again.merged_shapes, again.item_syntheses),
            (0, 0, 0, 0, 0),
            "{again:?}"
        );

        // Point requests after diagnostics walk nothing and answer as the
        // cold requests did.
        let before = work_counts();
        assert_eq!(answers(ctx), cold);
        let after = work_counts().since(before);
        assert_eq!((after.typing_walks, after.stack_walks, after.scalar_classifications), (0, 0, 0), "{after:?}");

        // Point typings taken before the inventory existed are what it holds.
        assert_eq!(typed(ctx), before_diagnostics);
        let inventory: BTreeMap<usize, String> =
            scalar_inventory(ctx).map(|(index, typing)| (index, format!("{typing:?}"))).collect();
        let held: Vec<String> = probes.iter().map(|index| inventory[index].clone()).collect();
        assert_eq!(held, before_diagnostics);
    });
}
