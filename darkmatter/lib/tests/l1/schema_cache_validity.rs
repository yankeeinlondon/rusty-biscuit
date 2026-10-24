//! Reusable schema work never serves a stale or foreign verdict.
//!
//! Every test runs on one long-lived [`DarkmatterSchemas`] (or several that
//! share the process caches) and asserts verdicts, positions, and the
//! compile-cache state, never elapsed time. Each fixture is its own temporary
//! repository, and every context is anchored there, never at the process
//! directory.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    CompileCache, DarkmatterSchemas, EmbeddedSchemaBundle, SchemaError, SchemaPhase, parse_yaml_schema,
};
use serde_json::json;

/// A temporary repository root (with `.git`), canonical so the paths
/// compared below are the ones resolution reports.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        Self { _dir: dir, root }
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn remove(&self, relative: &str) {
        std::fs::remove_file(self.root.join(relative)).expect("remove fixture");
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(&self.root, None, HashMap::new()).with_repository_root(&self.root)
    }

    fn schemas(&self) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context())
    }

    /// Writes `text` as `relative` and opens it, so the document carries its
    /// own folder.
    fn document(&self, relative: &str, text: &str) -> Markdown {
        let path = self.write(relative, text);
        Markdown::try_from(path.as_path()).expect("open document")
    }
}

fn valid(api: &DarkmatterSchemas, document: &Markdown) -> bool {
    api.validate(document).expect("the schema resolves").valid
}

/// A bundle whose files are test-built text.
fn bundle(name: &'static str, files: Vec<(&'static str, String)>) -> EmbeddedSchemaBundle {
    let files: Vec<(&'static str, &'static str)> =
        files.into_iter().map(|(file, text)| (file, &*Box::leak(text.into_boxed_str()))).collect();
    EmbeddedSchemaBundle::new(name, Box::leak(files.into_boxed_slice()))
}

/// A trigger matching any document with a `prompt`, typing `n` as `ty`.
fn trigger_typing_n(ty: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  n: {ty}\n")
}

// ── Distinct inputs keep distinct verdicts ───────────────────────────────────

#[test]
fn inline_schemas_differing_in_one_constraint_never_share_a_verdict() {
    let repo = Repo::new();
    let api = repo.schemas();
    let strict = repo.document("strict.md", "---\n$schema:\n  n: number(max(5))\nn: 7\n---\n");
    let loose = repo.document("loose.md", "---\n$schema:\n  n: number(max(10))\nn: 7\n---\n");

    assert!(!valid(&api, &strict));
    assert!(valid(&api, &loose));
    assert!(!valid(&api, &strict), "the looser schema's verdict is not reused");
}

#[test]
fn embedded_bundles_with_the_same_file_names_keep_their_own_verdicts() {
    let repo = Repo::new();
    let numbers = bundle("numbers", vec![("host.yaml", trigger_typing_n("number"))]);
    let strings = bundle("strings", vec![("host.yaml", trigger_typing_n("string"))]);
    let with = |bundle| {
        repo.schemas()
            .with_trigger_discovery()
            .and_then(|api| api.with_embedded_trigger_bundle(bundle))
            .expect("bundle loads")
    };
    let document = repo.document("doc.md", "---\nprompt: x\nn: hello\n---\n");
    let inline = repo.document("inline.md", "---\n$schema:\n  n: number\nn: hello\n---\n");

    assert!(!valid(&with(numbers), &document));
    assert!(valid(&with(strings), &document));
    assert!(!valid(&with(numbers), &document), "the other bundle's verdict is not reused");
    // An inline schema of the same shape as `numbers` is judged on its own.
    assert!(!valid(&with(strings), &inline));
    assert!(valid(&with(strings), &document));
}

#[test]
fn the_baseline_choice_is_part_of_every_verdict() {
    let repo = Repo::new();
    let baseline = |text: &str| {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).expect("baseline YAML");
        repo.schemas().with_baseline(parse_yaml_schema(&yaml).expect("baseline schema")).expect("baseline accepted")
    };
    let none = repo.schemas();
    let strict = baseline("title: string(required)\n");
    let loose = baseline("title: string\n");
    let document = repo.document("doc.md", "---\n$schema:\n  n: number\nn: 1\n---\n");
    let bare = repo.document("bare.md", "---\nn: 1\n---\n");

    assert!(valid(&none, &document));
    assert!(!valid(&strict, &document), "the baseline's required title applies");
    assert!(valid(&loose, &document));
    assert!(!valid(&strict, &document));
    assert!(valid(&none, &document));
    assert!(none.effective_for(&bare).expect("resolves").is_none(), "no schema without a baseline");
    assert!(!valid(&strict, &bare));
}

#[test]
fn swapping_two_matching_triggers_swaps_which_payload_wins() {
    let repo = Repo::new();
    let document = repo.document("doc.md", "---\nprompt: x\nn: hello\n---\n");
    let discover = || repo.schemas().with_trigger_discovery().expect("discovery");

    // Triggers in one root layer in file-name order; the later one wins `n`.
    repo.write("schemas/a.yaml", &trigger_typing_n("number"));
    repo.write("schemas/b.yaml", &trigger_typing_n("string"));
    assert!(valid(&discover(), &document));

    repo.write("schemas/a.yaml", &trigger_typing_n("string"));
    repo.write("schemas/b.yaml", &trigger_typing_n("number"));
    assert!(!valid(&discover(), &document), "the reordered layers are merged again");

    repo.write("schemas/a.yaml", &trigger_typing_n("number"));
    repo.write("schemas/b.yaml", &trigger_typing_n("string"));
    assert!(valid(&discover(), &document));
}

#[test]
fn launch_and_completion_projections_never_reuse_each_others_verdict() {
    let repo = Repo::new();
    let api = repo.schemas();
    let document = repo.document(
        "doc.md",
        "---\n$schema:\n  input: string(eager)\n  output: string(required)\ninput: ready\n---\n",
    );
    let effective = api.effective_for(&document).expect("resolves").expect("a schema");
    let instance = json!({ "input": "ready" });
    let at = |phase| effective.validate_for_phase(&instance, phase).expect("phase schema builds").valid;

    assert!(at(SchemaPhase::Launch));
    assert!(!at(SchemaPhase::Completion), "completion requires the output");
    assert!(at(SchemaPhase::Launch));
    assert!(!effective.validate(&instance).valid, "the unphased schema requires the output too");
    assert!(!at(SchemaPhase::Completion));
}

// ── One long-lived cache follows the files ───────────────────────────────────

#[test]
fn editing_a_transitive_import_changes_the_verdict_and_deleting_it_fails() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("schemas/doc.yaml", "$schema:\n  n: count@./types.yaml\n");
    repo.write("schemas/types.yaml", "kind: schema\ntypes:\n  count: number(max(5))\n");
    let document = repo.document("doc.md", "---\n$schema: ./schemas/doc.yaml\nn: 7\n---\n");

    assert!(!valid(&api, &document));
    repo.write("schemas/types.yaml", "kind: schema\ntypes:\n  count: number(max(100))\n");
    assert!(valid(&api, &document), "the edited import is read again");
    repo.write("schemas/types.yaml", "kind: schema\ntypes:\n  count: number(max(5))\n");
    assert!(!valid(&api, &document));

    repo.remove("schemas/types.yaml");
    assert!(api.validate(&document).is_err(), "a missing import never serves a cached verdict");
}

#[test]
fn a_nearer_import_candidate_added_then_removed_is_followed() {
    let repo = Repo::new();
    let api = repo.schemas();
    // An implicit path tries the document's folder before the repository root.
    repo.write("shared/types.yaml", "kind: schema\ntypes:\n  count: number(max(5))\n");
    let document = repo.document("docs/doc.md", "---\n$schema:\n  n: count@shared/types.yaml\nn: 7\n---\n");

    assert!(!valid(&api, &document));
    repo.write("docs/shared/types.yaml", "kind: schema\ntypes:\n  count: number(max(100))\n");
    assert!(valid(&api, &document), "the nearer candidate is selected once it exists");
    repo.remove("docs/shared/types.yaml");
    assert!(!valid(&api, &document), "the repository file is selected again");
}

// ── Shared validators, per-document results ──────────────────────────────────

#[test]
fn documents_sharing_a_structural_schema_reuse_its_validator_with_their_own_positions() {
    let repo = Repo::new();
    let api = repo.schemas();
    let schema = "$schema:\n  shared_structural_n: number\n";
    let first = repo.document("first.md", &format!("---\n{schema}shared_structural_n: one\n---\n"));
    let second = repo.document("second.md", &format!("---\n{schema}title: t\nnote: n\nshared_structural_n: two\n---\n"));

    let (first_report, first_timings) = api.validate_timed(&first);
    let (second_report, second_timings) = api.validate_timed(&second);
    let line_of = |report: Result<darkmatter::markdown::schemas::ValidationReport, SchemaError>| {
        let report = report.expect("the schema resolves");
        assert!(!report.valid);
        report.problems.iter().map(|problem| problem.line).collect::<Vec<_>>()
    };

    assert_eq!(first_timings.compile_cache(), CompileCache::Miss);
    assert_eq!(second_timings.compile_cache(), CompileCache::Hit, "the structural validator is reused");
    assert_eq!(line_of(first_report), [Some(4)]);
    assert_eq!(line_of(second_report), [Some(6)], "positions come from the document being validated");
}

/// DMLS and composition give each document its own context
/// (`for_source`). The built-in baseline's lazy, syntax-only
/// `darkmatter-file-reference` format must not make that context part of
/// the validator's identity.
#[test]
fn documents_in_their_own_contexts_reuse_a_validator_over_the_baseline() {
    let repo = Repo::new();
    let schema = "$schema:\n  per_context_structural_n: number\n";
    let first_path = repo.write("a/first.md", &format!("---\n{schema}per_context_structural_n: one\n---\n"));
    let second_path = repo.write("b/second.md", &format!("---\n{schema}title: t\nper_context_structural_n: two\n---\n"));
    let open = |path: &PathBuf| Markdown::try_from(path.as_path()).expect("open document");
    let in_own_context = |path: &PathBuf| {
        DarkmatterSchemas::new(repo.context().for_source(path))
            .with_darkmatter_baseline_json_schema()
            .expect("the built-in baseline")
    };

    let (first_report, first_timings) = in_own_context(&first_path).validate_timed(&open(&first_path));
    let (second_report, second_timings) = in_own_context(&second_path).validate_timed(&open(&second_path));

    assert!(!first_report.expect("the schema resolves").valid);
    let second_report = second_report.expect("the schema resolves");
    assert!(!second_report.valid);
    assert_eq!(first_timings.compile_cache(), CompileCache::Miss);
    assert_eq!(second_timings.compile_cache(), CompileCache::Hit, "the baseline-merged validator is shared");
    assert_eq!(
        second_report.problems.iter().map(|problem| problem.line).collect::<Vec<_>>(),
        [Some(5)],
        "positions come from the document being validated"
    );
}

#[test]
fn an_invalid_document_reuses_the_validator_its_schema_already_compiled() {
    let repo = Repo::new();
    let api = repo.schemas();
    let schema = "$schema:\n  reused_by_conversion_n: number\n";
    let valid_document = repo.document("valid.md", &format!("---\n{schema}reused_by_conversion_n: 1\n---\n"));
    let invalid_document =
        repo.document("invalid.md", &format!("---\n{schema}reused_by_conversion_n: [1]\n---\n"));

    let (warm, _) = api.validate_timed(&valid_document);
    assert!(warm.expect("resolves").valid);
    let (report, timings) = api.validate_timed(&invalid_document);

    assert!(!report.expect("resolves").valid);
    assert_eq!(
        timings.compile_cache(),
        CompileCache::Hit,
        "judging a converted copy builds no second validator for the same schema"
    );
}

#[test]
fn file_values_are_judged_from_each_documents_own_folder_directly_and_through_a_reference() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/x.md", "# x\n");
    let direct = "$schema:\n  spec: file(eager)\nspec: ./x.md\n";
    let referenced = "$schema:\n  node: \"{ spec: file(eager), children: node[] }\"\n  tree: node(required)\n\
                      tree:\n  spec: ./x.md\n  children:\n    - spec: ./x.md\n      children: []\n";
    for (case, frontmatter) in [("direct", direct), ("through a reference", referenced)] {
        let beside = repo.document("a/doc.md", &format!("---\n{frontmatter}---\n"));
        let elsewhere = repo.document("b/doc.md", &format!("---\n{frontmatter}---\n"));

        assert!(valid(&api, &beside), "{case}: the file exists beside the document");
        assert!(!valid(&api, &elsewhere), "{case}: no file beside the second document");
        assert!(valid(&api, &beside), "{case}: the second document's verdict is not reused");
    }
}

#[test]
fn match_globs_are_judged_from_each_documents_own_folder_directly_and_through_a_named_type() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    // In a root union `match()` rules an arm out; the other arm accepts
    // nothing here, so the verdict is the glob's.
    let direct = "$schema:\n  - spec: \"file(match('./notes/*.md'))\"\n  - spec: number\n";
    let referenced = "$schema:\n  - note: \"file(match('./notes/*.md'))\"\n    spec: note\n  - spec: number\n";
    for (case, schema) in [("direct", direct), ("through a named type", referenced)] {
        let inside = repo.document("a/doc.md", &format!("---\n{schema}spec: ./notes/x.md\n---\n"));
        let outside = repo.document("b/doc.md", &format!("---\n{schema}spec: ../a/notes/x.md\n---\n"));

        assert!(valid(&api, &inside), "{case}: the file is under the document's `./notes`");
        assert!(!valid(&api, &outside), "{case}: `./notes` is judged from the second document's folder");
        assert!(valid(&api, &inside), "{case}: the second document's verdict is not reused");
    }
}


// ── `match()` reached through a recursive reference ──────────────────────────

/// A recursive type whose file alternative declares a glob.
const NODE: &str = "\"file(match('./notes/*.md')) | { children: node[] }\"";

/// Validates the same `schema` from `a/doc.md` (whose `./notes` holds the
/// file) and then from `b/doc.md` (whose `./notes` does not) on one
/// long-lived cache. The first must pass and the second fail, and the second
/// must build its own validator: the glob makes the verdict depend on the
/// document's folder, so a structural (context-free) hit would be wrong.
fn assert_match_judged_from_each_folder(case: &str, repo: &Repo, schema: &str) {
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    let inside = repo.document("a/doc.md", &format!("---\n{schema}spec: ./notes/x.md\n---\n"));
    let outside = repo.document("b/doc.md", &format!("---\n{schema}spec: ../a/notes/x.md\n---\n"));

    let (inside_report, inside_timings) = api.validate_timed(&inside);
    let (outside_report, outside_timings) = api.validate_timed(&outside);

    let inside_report = inside_report.expect("the schema resolves");
    assert!(inside_report.valid, "{case}: the file is under `a/notes`: {:#?}", inside_report.problems);
    assert!(!outside_report.expect("the schema resolves").valid, "{case}: `./notes` is judged from `b`");
    assert_eq!(inside_timings.compile_cache(), CompileCache::Miss, "{case}");
    assert_eq!(
        outside_timings.compile_cache(),
        CompileCache::Miss,
        "{case}: the glob keeps each document's context in the validator's identity"
    );
    assert!(valid(&api, &inside), "{case}: the second document's verdict is not reused");
}

#[test]
fn match_below_a_recursive_reference_in_an_inline_root_union_arm_is_enforced() {
    let repo = Repo::new();
    let schema = format!("$schema:\n  - node: {NODE}\n    spec: node(required)\n  - spec: number\n");
    assert_match_judged_from_each_folder("inline arm", &repo, &schema);

    // The glob belongs to the type, so a file nested in the tree is judged too.
    let api = repo.schemas();
    let nested = |folder: &str, value: &str| {
        repo.document(
            &format!("{folder}/nested.md"),
            &format!("---\n{schema}spec:\n  children:\n    - children:\n        - {value}\n---\n"),
        )
    };
    assert!(valid(&api, &nested("a", "./notes/x.md")));
    assert!(!valid(&api, &nested("b", "../a/notes/x.md")), "a nested file outside `b/notes` rules the arm out");
}

#[test]
fn match_below_a_recursive_reference_in_a_referenced_simplified_arm_is_enforced() {
    let repo = Repo::new();
    repo.write("schemas/node-arm.yaml", &format!("$schema:\n  node: {NODE}\n  spec: node(required)\n"));
    let schema = "$schema:\n  - ../schemas/node-arm.yaml\n  - spec: number\n";
    assert_match_judged_from_each_folder("referenced arm", &repo, schema);
}

#[test]
fn match_below_a_recursive_reference_in_a_referenced_root_union_file_is_enforced() {
    // The two files resolve to the same schema, so each case gets its own
    // repository rather than reusing the other's validators.
    let inline_arm = Repo::new();
    inline_arm.write(
        "schemas/node-union.yaml",
        &format!("$schema:\n  - node: {NODE}\n    spec: node(required)\n  - spec: number\n"),
    );
    assert_match_judged_from_each_folder("inline arm of the file", &inline_arm, "$schema: ../schemas/node-union.yaml\n");

    let file_arm = Repo::new();
    file_arm.write("schemas/node-arm.yaml", &format!("$schema:\n  node: {NODE}\n  spec: node(required)\n"));
    file_arm.write("schemas/file-arm-union.yaml", "$schema:\n  - ./node-arm.yaml\n  - spec: number\n");
    assert_match_judged_from_each_folder("file arm of the file", &file_arm, "$schema: ../schemas/file-arm-union.yaml\n");
}

#[test]
fn match_below_a_recursive_reference_beside_a_raw_arm_is_enforced() {
    let repo = Repo::new();
    repo.write(
        "schemas/number-arm.json",
        &json!({"type": "object", "properties": {"spec": {"type": "number"}}, "required": ["spec"]}).to_string(),
    );
    let schema = format!("$schema:\n  - node: {NODE}\n    spec: node(required)\n  - ../schemas/number-arm.json\n");
    assert_match_judged_from_each_folder("beside a raw arm", &repo, &schema);
}

#[test]
fn match_on_a_direct_file_property_of_a_root_union_arm_is_enforced() {
    let repo = Repo::new();
    let schema = "$schema:\n  - spec: \"file(match('./notes/*.md'))\"\n  - spec: number\n";
    assert_match_judged_from_each_folder("direct", &repo, schema);
}

#[test]
fn match_through_an_acyclic_named_file_type_is_enforced() {
    let repo = Repo::new();
    let schema = "$schema:\n  - note: \"file(match('./notes/*.md'))\"\n    spec: note(required)\n  - spec: number\n";
    assert_match_judged_from_each_folder("acyclic named type", &repo, schema);
}

// ── Acyclic arms keep their suggestion-only `match()` ───────────────────────

/// An arm's `(name, type)` lines.
type ArmLines = Vec<(&'static str, &'static str)>;

/// The six acyclic shapes whose `match()` sits below an arm property without
/// being that property's own `file` type: each arm's `(name, type)` lines and
/// a `spec` value naming `a/notes/x.md` from `b/`. None recurses, so each
/// lowers exactly as before recursive types existed: the glob only suggests.
fn suggestion_only_arms() -> [(&'static str, ArmLines, serde_json::Value); 6] {
    let file = "../a/notes/x.md";
    [
        ("property union", vec![("spec", "\"file(match('./notes/*.md')) | number\"")], json!(file)),
        (
            "acyclic name bound to the union",
            vec![("choice", "\"file(match('./notes/*.md')) | number\""), ("spec", "choice")],
            json!(file),
        ),
        ("inline object", vec![("spec", "\"{ path: file(match('./notes/*.md')) }\"")], json!({ "path": file })),
        (
            "pattern dictionary",
            vec![("spec", "\"{ <pattern::^n>: file(match('./notes/*.md')) }\"")],
            json!({ "n1": file }),
        ),
        ("tuple slot", vec![("spec", "\"tuple([file(match('./notes/*.md')), number])\"")], json!([file, 1])),
        ("list of a union", vec![("spec", "\"(file(match('./notes/*.md')) | number)[]\"")], json!([file])),
    ]
}

/// The arm's lines as the body of a `$schema` mapping (`indent` spaces) or
/// of a root-union list item.
fn arm_yaml(lines: &[(&str, &str)], first: &str, indent: &str) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(index, (name, ty))| format!("{}{name}: {ty}\n", if index == 0 { first } else { indent }))
        .collect()
}

/// For each acyclic shape, the root union `schema(index, arm)` assembles is
/// lowered with no `x-darkmatter-match` anywhere, and a document in `b/`
/// naming `a/notes/x.md` (outside `b/notes`) is valid. When `phases`, the
/// Launch and Completion re-lowerings accept that value too.
fn assert_acyclic_arms_only_suggest(route: &str, phases: bool, schema: impl Fn(&Repo, usize, &str) -> String) {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    for (index, (shape, lines, value)) in suggestion_only_arms().into_iter().enumerate() {
        let case = format!("{route}, {shape}");
        let schema = schema(&repo, index, &arm_yaml(&lines, "  - ", "    "));
        let document = repo.document(&format!("b/{index}.md"), &format!("---\n{schema}spec: {value}\n---\n"));
        let effective = api.effective_for(&document).expect("resolves").expect("a schema");
        let lowered = effective.json_schema.to_string();
        assert!(lowered.contains("darkmatter-file-reference"), "{case}: the file arm applies: {lowered}");
        assert!(!lowered.contains("x-darkmatter-match"), "{case}: an acyclic glob only suggests: {lowered}");
        let report = api.validate(&document).expect("the schema resolves");
        assert!(report.valid, "{case}: an existing file outside the glob is accepted: {:#?}", report.problems);
        if phases {
            for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
                let verdict = effective.validate_for_phase(&json!({ "spec": value }), phase).expect("phase schema");
                assert!(verdict.valid, "{case}, {phase:?}: {:#?}", verdict.problems);
            }
        }
    }
}

/// The arm, as the body of its own simplified schema file.
fn arm_file(repo: &Repo, index: usize, arm: &str) -> String {
    let body = arm.replacen("  - ", "  ", 1).replace("\n    ", "\n  ");
    repo.write(&format!("schemas/arm-{index}.yaml"), &format!("$schema:\n{body}"));
    format!("arm-{index}.yaml")
}

#[test]
fn acyclic_match_in_an_inline_document_arm_only_suggests() {
    assert_acyclic_arms_only_suggest("inline arm", true, |_, _, arm| format!("$schema:\n{arm}  - spec: boolean\n"));
}

#[test]
fn acyclic_match_in_a_referenced_simplified_arm_only_suggests() {
    assert_acyclic_arms_only_suggest("referenced arm", true, |repo, index, arm| {
        format!("$schema:\n  - ../schemas/{}\n  - spec: boolean\n", arm_file(repo, index, arm))
    });
}

#[test]
fn acyclic_match_in_an_inline_arm_of_a_referenced_union_file_only_suggests() {
    assert_acyclic_arms_only_suggest("inline arm of a union file", true, |repo, index, arm| {
        repo.write(&format!("schemas/union-{index}.yaml"), &format!("$schema:\n{arm}  - spec: boolean\n"));
        format!("$schema: ../schemas/union-{index}.yaml\n")
    });
}

#[test]
fn acyclic_match_in_a_file_arm_of_a_referenced_union_file_only_suggests() {
    assert_acyclic_arms_only_suggest("file arm of a union file", true, |repo, index, arm| {
        let arm = arm_file(repo, index, arm);
        repo.write(&format!("schemas/union-{index}.yaml"), &format!("$schema:\n  - ./{arm}\n  - spec: boolean\n"));
        format!("$schema: ../schemas/union-{index}.yaml\n")
    });
}

#[test]
fn acyclic_match_beside_a_raw_arm_only_suggests() {
    assert_acyclic_arms_only_suggest("beside a raw arm", false, |repo, _, arm| {
        repo.write(
            "schemas/boolean-arm.json",
            &json!({"type": "object", "properties": {"spec": {"type": "boolean"}}, "required": ["spec"]}).to_string(),
        );
        format!("$schema:\n{arm}  - ../schemas/boolean-arm.json\n")
    });
}

/// A recursive reference nested in an arm's acyclic inline object still
/// enforces through the arm's copy, while a sibling file in that same object
/// only suggests.
#[test]
fn a_reference_inside_an_acyclic_inline_object_is_enforced_but_its_file_sibling_is_not() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    let schema = format!(
        "$schema:\n  - node: {NODE}\n    spec: \"{{ inner: node, path: file(match('./notes/*.md')) }}\"\n  - spec: number\n"
    );
    let at = |name: &str, folder: &str, value: serde_json::Value| {
        valid(&api, &repo.document(&format!("{folder}/{name}.md"), &format!("---\n{schema}spec: {value}\n---\n")))
    };
    assert!(at("inner", "a", json!({ "inner": "./notes/x.md" })));
    assert!(!at("inner", "b", json!({ "inner": "../a/notes/x.md" })), "the reference is judged through its copy");
    assert!(!at("nested", "b", json!({ "inner": { "children": ["../a/notes/x.md"] } })));
    assert!(at("path", "b", json!({ "path": "../a/notes/x.md" })), "the acyclic sibling only suggests");
}

#[test]
fn an_explicit_match_keyword_behind_a_raw_local_reference_is_enforced() {
    let repo = Repo::new();
    repo.write(
        "schemas/raw-node.json",
        &json!({
            "type": "object",
            "properties": {"spec": {"$ref": "#/$defs/node"}},
            "required": ["spec"],
            "$defs": {"node": {"anyOf": [
                {"type": "string", "x-darkmatter-match": ["./notes/*.md"]},
                {"type": "object", "properties": {"children": {"type": "array", "items": {"$ref": "#/$defs/node"}}}},
            ]}},
        })
        .to_string(),
    );
    assert_match_judged_from_each_folder("raw reference", &repo, "$schema: ../schemas/raw-node.json\n");
}

#[test]
fn phase_projections_of_a_root_union_keep_match_below_a_recursive_reference() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    let schema = format!("$schema:\n  - node: {NODE}\n    spec: node(required)\n  - spec: number\n");
    let effective_in = |folder: &str| {
        let document = repo.document(&format!("{folder}/doc.md"), &format!("---\n{schema}---\n"));
        api.effective_for(&document).expect("resolves").expect("a schema")
    };
    let (inside, outside) = (effective_in("a"), effective_in("b"));

    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let at = |effective: &darkmatter::markdown::schemas::EffectiveSchema, value: &str| {
            effective.validate_for_phase(&json!({ "spec": value }), phase).expect("phase schema builds").valid
        };
        assert!(at(&inside, "./notes/x.md"), "{phase:?}");
        assert!(!at(&outside, "../a/notes/x.md"), "{phase:?}: the re-lowered arms keep the glob");
    }
}

#[test]
fn match_below_a_reference_keeps_context_and_leaves_shared_uses_suggestion_only() {
    let repo = Repo::new();
    let api = repo.schemas();
    repo.write("a/notes/x.md", "# x\n");
    let union = format!("$schema:\n  - node: {NODE}\n    spec: node(required)\n  - spec: number\n");
    let first = repo.document("b/first.md", &format!("---\n{union}spec: ../a/notes/x.md\n---\n"));
    let second = repo.document("b/second.md", &format!("---\n{union}spec: ../a/notes/x.md\n---\n"));

    let (first_report, first_timings) = api.validate_timed(&first);
    let (second_report, second_timings) = api.validate_timed(&second);

    assert!(!first_report.expect("resolves").valid);
    assert!(!second_report.expect("resolves").valid);
    assert_eq!(first_timings.compile_cache(), CompileCache::Miss);
    assert_eq!(second_timings.compile_cache(), CompileCache::Hit, "one folder's documents share its validator");

    // The arm enforces through its own copy of the definition; the shared
    // definition itself stays the suggestion-only body other uses lower to.
    let effective = api.effective_for(&first).expect("resolves").expect("a schema");
    let defs = effective.json_schema["$defs"].as_object().expect("union root $defs");
    assert!(!defs["node"].to_string().contains("x-darkmatter-match"), "{:#}", defs["node"]);
    assert!(defs.values().any(|body| body.to_string().contains("x-darkmatter-match")), "{defs:#?}");

    // A single schema only suggests: the same type outside a root union
    // rejects no file.
    let single = format!("$schema:\n  node: {NODE}\n  spec: node(required)\n");
    let suggested = repo.document("b/single.md", &format!("---\n{single}spec: ../a/notes/x.md\n---\n"));
    assert!(valid(&api, &suggested));
}
