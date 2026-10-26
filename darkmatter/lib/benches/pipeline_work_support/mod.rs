//! Scenarios shared by the `pipeline_work` (Criterion timings) and
//! `pipeline_work_alloc` (work counts and allocations) benches of the
//! `2026-10-25-pipeline-work-reduction` measurement spike.
//!
//! Each [`Case`] names a spike item (`D1`…`D13`, `X1`…`X4`) or anchor (`A1`,
//! `A2`), a workload size, and whether every run starts from emptied process
//! caches. Its setup runs only when the case is selected, and returns the
//! closure a run executes. Every case goes through public library APIs; a
//! case that reproduces a caller's call sequence (D2's `md schema validate`,
//! D8's candidate construction) says so where it is built.

use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::path::{Path, PathBuf};

use darkmatter::lifecycle::host_profile::HostLifecycleProfile;
use darkmatter::lifecycle::{LifecycleSourceMap, lifecycle_call_checks, parse_lifecycle_config_with};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{
    DeclaredTypes, FunctionRegistration, TypeScope, one_based_position, parse,
};
use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, parse_authored};
use darkmatter::markdown::schemas::catalog::host_lifecycle_profiles;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::rewrite::rewrite_eager_file_values;
use darkmatter::markdown::schemas::{DarkmatterSchemas, reset_process_caches_for_bench, shape_from_json_schema};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

/// One measured operation at one workload size.
pub struct Case {
    /// The spike item or anchor, then a short variant: `D5/conditionals`.
    pub item: &'static str,
    /// The workload size, as the report names it.
    pub size: String,
    /// Empty the process caches before every run, outside the measurement.
    pub cold: bool,
    setup: Box<dyn FnOnce() -> Box<dyn FnMut()>>,
}

impl Case {
    fn new(item: &'static str, size: impl ToString, cold: bool, setup: impl FnOnce() -> Box<dyn FnMut()> + 'static) -> Self {
        Self { item, size: size.to_string(), cold, setup: Box::new(setup) }
    }

    /// `item/size`, the name both benches print.
    pub fn name(&self) -> String {
        format!("{}/{}", self.item, self.size)
    }

    /// Runs the setup and returns the operation one run executes.
    pub fn prepare(self) -> Box<dyn FnMut()> {
        (self.setup)()
    }
}

/// Empties the process caches before a cold run.
pub fn reset_caches() {
    reset_process_caches_for_bench();
}

/// Every case, in report order. Setup has not run yet.
pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    anchor_a1(&mut cases);
    anchor_a2(&mut cases);
    d1_eager_file_rewrite(&mut cases);
    d2_double_preparation(&mut cases);
    d4_position_preparation(&mut cases);
    d5_scope_join(&mut cases);
    d6_position_prefix_scan(&mut cases);
    d7_operand_sites(&mut cases);
    d8_placeholder_parses(&mut cases);
    d9_grammar_clones(&mut cases);
    d10_source_map(&mut cases);
    d11_rejected_candidates(&mut cases);
    d12_named_type_memo(&mut cases);
    d13_alphanumeric_checks(&mut cases);
    x3_recursive_translation(&mut cases);
    x4_wide_mapping_problems(&mut cases);
    cases
}

// ---------- shared fixtures ----------

/// A temporary directory with a canonical path, the first thing every
/// file-backed case builds.
fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// A context anchored in `root`, never the process directory, home, or
/// environment.
fn context_at(root: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::from_snapshot(root, None, HashMap::new()).with_repository_root(root)
}

/// The same inline schema `schema_validation.rs`'s 1,000-document corpus
/// uses, so A1 here is that bench's corpus.
const A1_SCHEMA_FRONTMATTER: &str = "$schema:\n  \
    title: 'string(required)'\n  \
    slug: 'string(pattern(^[a-z][a-z0-9-]*$);required)'\n  \
    views: 'number(min(0);required)'\n  \
    status: 'enum(draft, published, archived; required)'\n  \
    tags: 'string[]'\n";

/// `schema_validation.rs::build_corpus`, with the inline schema optional so
/// the schema-free variant shares its documents.
fn a1_corpus(count: usize, with_schema: bool) -> Vec<Markdown> {
    (0..count)
        .map(|i| {
            let slug = if i % 10 == 0 { format!("INVALID slug {i}") } else { format!("post-{i}") };
            let status = ["draft", "published", "archived"][i % 3];
            let schema = if with_schema { A1_SCHEMA_FRONTMATTER } else { "" };
            Markdown::from(
                format!(
                    "---\n{schema}title: 'Post {i}'\nslug: '{slug}'\nviews: {i}\nstatus: {status}\ntags:\n  - rust\n  - markdown\n---\nBody of post {i}\n"
                )
                .as_str(),
            )
        })
        .collect()
}

/// `schemas::frontmatter_as_json`: the frontmatter as validation sees it.
fn frontmatter_json(document: &Markdown) -> Value {
    let mut object = serde_json::Map::new();
    for (key, value) in document.frontmatter().as_map() {
        if key != "$schema" {
            object.insert(key.clone(), value.clone());
        }
    }
    Value::Object(object)
}

/// The A1 configuration, without or with the shipped runtime baseline that
/// `md schema validate` applies by default.
fn a1_api(context: &biscuit_file::FileResolutionContext, baseline: bool) -> DarkmatterSchemas {
    let api = DarkmatterSchemas::new(context.clone());
    if baseline { api.with_darkmatter_baseline_json_schema().expect("the shipped baseline loads") } else { api }
}

/// A Markdown document's frontmatter as a JSON value, `$schema` included,
/// the way lifecycle preparation reads it.
fn frontmatter_value(text: &str) -> Value {
    let yaml = text.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---\n")).map(|(yaml, _)| yaml).expect("frontmatter");
    let parsed: YamlValue = serde_yaml_ng::from_str(yaml).expect("frontmatter YAML parses");
    serde_json::to_value(parsed).expect("frontmatter is JSON")
}

/// The frozen Claudine bundle `schema_validation.rs` validates against.
const CLAUDINE_SCHEMAS: [(&str, &str); 9] = [
    ("action.yaml", include_str!("../fixtures/claudine-schemas/action.yaml")),
    ("claudine.yaml", include_str!("../fixtures/claudine-schemas/claudine.yaml")),
    ("globals.yaml", include_str!("../fixtures/claudine-schemas/globals.yaml")),
    ("inline-compose.yaml", include_str!("../fixtures/claudine-schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../fixtures/claudine-schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../fixtures/claudine-schemas/linking.yaml")),
    ("review.yaml", include_str!("../fixtures/claudine-schemas/review.yaml")),
    ("sequence-types.yaml", include_str!("../fixtures/claudine-schemas/sequence-types.yaml")),
    ("sequence.yaml", include_str!("../fixtures/claudine-schemas/sequence.yaml")),
];

/// The A3 document: the repository's largest lifecycle prompt, frozen.
pub const A3_DOCUMENT: &str = include_str!("../fixtures/pipeline-work/implement-plan.md");

/// The host lifecycle artifact `claudine/schemas/globals.lifecycle.json`,
/// frozen beside the A3 document. It is keyed by the hash of `globals.yaml`,
/// which the frozen bundle shares with the live one, so it is fresh for it.
const CLAUDINE_LIFECYCLE_ARTIFACT: &str = include_str!("../fixtures/pipeline-work/globals.lifecycle.json");

/// Claudine's lifecycle host profile (its event names, verbs, and
/// globals), discovered from the frozen bundle and its lifecycle artifact
/// exactly as DMLS discovers it.
fn claudine_profile() -> std::sync::Arc<HostLifecycleProfile> {
    let repository = ClaudineRepository::new();
    std::fs::write(repository.root.join("schemas/globals.lifecycle.json"), CLAUDINE_LIFECYCLE_ARTIFACT).expect("write artifact");
    host_lifecycle_profiles(&repository.context())
        .into_iter()
        .find_map(|discovery| discovery.profile.ok())
        .expect("the frozen bundle's lifecycle artifact is fresh")
}

/// A temporary repository with the frozen bundle in its `schemas/` root, as
/// `schema_validation.rs`'s Claudine groups build it.
struct ClaudineRepository {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl ClaudineRepository {
    fn new() -> Self {
        let (dir, root) = temp_root();
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(root.join("schemas")).expect("schemas root");
        for (name, text) in CLAUDINE_SCHEMAS {
            std::fs::write(root.join("schemas").join(name), text).expect("write frozen schema");
        }
        Self { _dir: dir, root }
    }

    fn document(&self, name: &str, text: &str) -> Markdown {
        let path = self.root.join(name);
        std::fs::write(&path, text).expect("write document");
        Markdown::try_from(path.as_path()).expect("open document")
    }

    fn context(&self) -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(&self.root, None, HashMap::new()).with_repository_root(&self.root)
    }
}

/// One validation as a fresh request: trigger discovery, then validation.
fn validate_claudine(context: &biscuit_file::FileResolutionContext, document: &Markdown) -> bool {
    DarkmatterSchemas::new(context.clone())
        .with_trigger_discovery()
        .expect("trigger discovery")
        .validate(document)
        .expect("validate must not fault")
        .valid
}

// ---------- anchors ----------

/// A1: validate the 1,000-document corpus, warm and cold, with and without
/// the runtime baseline `md schema validate` applies by default.
fn anchor_a1(cases: &mut Vec<Case>) {
    for (item, baseline) in [("A1/corpus_validate", false), ("A1/corpus_validate_baseline", true)] {
        for cold in [false, true] {
            let size = if cold { "1000-cold" } else { "1000-warm" };
            cases.push(Case::new(item, size, cold, move || {
                let corpus = a1_corpus(1_000, true);
                let context = biscuit_file::FileResolutionContext::new(std::env::current_dir().expect("process directory"));
                Box::new(move || {
                    let api = a1_api(&context, baseline);
                    for document in &corpus {
                        black_box(api.validate(document).expect("validate must not fault"));
                    }
                })
            }));
        }
    }
}

const NO_TRIGGER_MATCH: &str = "---\ntitle: x\n---\nbody\n";

const DOCUMENT_EVENTS: &str = r#"---
prompt: Summarize the change
agent: claude
start:
  - info: "starting"
  - say: "starting the review"
success:
  - effect: "crowd-applause"
  - say: "the build is green"
  - when: "fm.notify"
    then:
      - info: "notified"
failure:
  - warn: "giving up"
  - shell: "git status"
---
body
"#;

const NESTED_SEQUENCE_GROUP: &str = r#"---
start:
  - info: "sequence start"
success:
  - say: "all done"
sequence:
  - name: build
    shell: cargo build
    success:
      - info: "built"
  - name: review
    start:
      - info: "nested start"
    sequence:
      - name: lint
        prompt: ./lint.md
        failure:
          - warn: "lint failed"
  - name: fan-out
    group:
      name: checks
      tasks:
        - name: a
          prompt: ./a.md
        - name: b
          prompt: ./b.md
      success:
        - info: "group done"
      failure:
        - warn: "a check failed"
---
body
"#;

/// A2: the three `schema_validation.rs` Claudine documents, plus the frozen
/// A3 document, each validated against the frozen bundle cold and warm.
fn anchor_a2(cases: &mut Vec<Case>) {
    let documents: [(&str, &'static str); 4] = [
        ("no_trigger_match", NO_TRIGGER_MATCH),
        ("document_events", DOCUMENT_EVENTS),
        ("nested_sequence_group", NESTED_SEQUENCE_GROUP),
        ("implement_plan", A3_DOCUMENT),
    ];
    for (name, text) in documents {
        for cold in [false, true] {
            let size = format!("{name}-{}", if cold { "cold" } else { "warm" });
            cases.push(Case::new("A2/claudine_bundle", size, cold, move || {
                let repository = ClaudineRepository::new();
                let context = repository.context();
                let document = repository.document("doc.md", text);
                Box::new(move || {
                    let _keep = &repository;
                    black_box(validate_claudine(&context, &document));
                })
            }));
        }
    }
}

// ---------- items ----------

/// Two eager-file object arms told apart by a literal tag.
const TAGGED_FILE_TYPES: &str = "kind: schema\ntypes:\n  \
    tagged:\n    \
    - \"{ kind: literal(doc; required), path: file(eager; required) }\"\n    \
    - \"{ kind: literal(spec; required), spec: file(eager; required) }\"\n";

/// D1: the eager-file rewrite of a list whose items are a union of two
/// eligible object arms. Isolated (`rewrite_eager_file_values` over the
/// lowered schema) at 10/100/1,000 items, and the A1 variant: 1,000
/// documents with a three-item file list, each prepared, validated, and
/// normalized as composition's schema stage does, with the normalization
/// alone as its own case.
fn d1_eager_file_rewrite(cases: &mut Vec<Case>) {
    for items in [10usize, 100, 1_000] {
        cases.push(Case::new("D1/rewrite_isolated", items, false, move || {
            let (dir, root) = temp_root();
            std::fs::write(root.join("objects.yaml"), TAGGED_FILE_TYPES).expect("write types");
            std::fs::write(root.join("a.md"), "# A\n").expect("write a.md");
            std::fs::write(root.join("b.md"), "# B\n").expect("write b.md");
            let context = context_at(&root);
            let document = Markdown::from("---\n$schema:\n  files: \"tagged[]@./objects.yaml\"\n---\n");
            let effective = DarkmatterSchemas::new(context.clone())
                .effective_for(&document)
                .expect("schema resolves")
                .expect("declared schema");
            let list: Vec<Value> = (0..items)
                .map(|i| if i % 2 == 0 { json!({ "kind": "doc", "path": "./a.md" }) } else { json!({ "kind": "spec", "spec": "./b.md" }) })
                .collect();
            let instance = json!({ "files": list });
            let pending = HashSet::new();
            Box::new(move || {
                let _keep = &dir;
                let outcome = rewrite_eager_file_values(&effective.json_schema, &instance, &root, None, &pending, &context);
                assert!(outcome.changed, "every `./` path is rewritten");
                black_box(outcome);
            })
        }));
    }
    for (variant, normalize_only) in [("D1/a1_variant_prepare_validate_normalize", false), ("D1/a1_variant_normalize_only", true)] {
        cases.push(Case::new(variant, "1000x3", false, move || {
            let (dir, root) = temp_root();
            std::fs::write(root.join("objects.yaml"), TAGGED_FILE_TYPES).expect("write types");
            std::fs::write(root.join("a.md"), "# A\n").expect("write a.md");
            std::fs::write(root.join("b.md"), "# B\n").expect("write b.md");
            let context = context_at(&root);
            let corpus: Vec<Markdown> = (0..1_000)
                .map(|i| {
                    Markdown::from(
                        format!(
                            "---\n{A1_SCHEMA_FRONTMATTER}  files: \"tagged[]@./objects.yaml\"\ntitle: 'Post {i}'\nslug: 'post-{i}'\nviews: {i}\nstatus: draft\nfiles:\n  - {{ kind: doc, path: ./a.md }}\n  - {{ kind: spec, spec: ./b.md }}\n  - {{ kind: doc, path: ./a.md }}\n---\nBody {i}\n"
                        )
                        .as_str(),
                    )
                })
                .collect();
            let api = DarkmatterSchemas::new(context);
            let prepared: Vec<_> = corpus
                .iter()
                .map(|document| (api.effective_for(document).expect("resolves").expect("declared"), frontmatter_json(document)))
                .collect();
            let pending = HashSet::new();
            Box::new(move || {
                let _keep = &dir;
                if normalize_only {
                    for (effective, frontmatter) in &prepared {
                        black_box(effective.normalize_frontmatter(frontmatter, &pending));
                    }
                } else {
                    for document in &corpus {
                        let effective = api.effective_for(document).expect("resolves").expect("declared");
                        let frontmatter = frontmatter_json(document);
                        black_box(effective.validate(&frontmatter));
                        black_box(effective.normalize_frontmatter(&frontmatter, &pending));
                    }
                }
            })
        }));
    }
}

/// D2: `md schema validate`'s `validate_one` after assignments: `validate`
/// and then `effective_for` only to learn whether a schema applied. The
/// control is `validate` alone; the difference is the second preparation.
/// Reproduces the CLI's library calls; the corpus is outside a repository,
/// so `validate_one` would run no trigger discovery either.
fn d2_double_preparation(cases: &mut Vec<Case>) {
    let variants = [
        ("D2/validate_only", false, false),
        ("D2/validate_then_presence", false, true),
        ("D2/baseline_validate_only", true, false),
        ("D2/baseline_validate_then_presence", true, true),
    ];
    for (item, baseline, presence) in variants {
        for documents in [10usize, 100, 1_000] {
            cases.push(Case::new(item, documents, false, move || {
                let corpus = a1_corpus(documents, true);
                let context = biscuit_file::FileResolutionContext::new(std::env::current_dir().expect("process directory"));
                let api = a1_api(&context, baseline);
                Box::new(move || {
                    for document in &corpus {
                        let report = api.validate(document).expect("validate must not fault");
                        if presence {
                            black_box(api.effective_for(document).expect("resolves").is_none());
                        }
                        black_box(report);
                    }
                })
            }));
        }
    }
}

/// D4: source positions are prepared before the effective schema is known.
/// `validate` (positions first) against a control that prepares the schema
/// and validates without positions, on the A1 corpus (90% valid) and on a
/// schema-free corpus with and without the runtime baseline. The
/// difference is position preparation (and, for the 10% invalid A1
/// documents, problem location, which is needed work).
fn d4_position_preparation(cases: &mut Vec<Case>) {
    // (item, corpus has an inline schema, runtime baseline, prepare positions)
    let variants = [
        ("D4/a1_validate", true, false, true),
        ("D4/a1_no_positions_control", true, false, false),
        ("D4/schema_free_validate", false, false, true),
        ("D4/schema_free_no_positions_control", false, false, false),
        ("D4/schema_free_baseline_validate", false, true, true),
        ("D4/schema_free_baseline_no_positions_control", false, true, false),
    ];
    for (item, with_schema, baseline, with_positions) in variants {
        cases.push(Case::new(item, 1_000, false, move || {
            let corpus = a1_corpus(1_000, with_schema);
            let context = biscuit_file::FileResolutionContext::new(std::env::current_dir().expect("process directory"));
            let api = a1_api(&context, baseline);
            Box::new(move || {
                for document in &corpus {
                    if with_positions {
                        black_box(api.validate(document).expect("validate must not fault"));
                    } else {
                        let frontmatter = frontmatter_json(document);
                        if let Some(effective) = api.effective_for(document).expect("resolves") {
                            black_box(effective.validate(&frontmatter));
                        }
                    }
                }
            })
        }));
    }
}

/// A frontmatter whose `start` stack holds `count` conditionals, each
/// guarding on its own optional string, so every conditional adds one fact
/// to the scope its successors join over.
fn conditional_stack(count: usize) -> String {
    let mut text = String::from("---\n$schema:\n");
    for i in 0..count {
        text.push_str(&format!("  f{i}: string\n"));
    }
    text.push_str("start:\n");
    for i in 0..count {
        text.push_str(&format!("  - when: \"f{i}\"\n    then:\n      - info: \"f{i} is set\"\n"));
    }
    text.push_str("---\nbody\n");
    text
}

/// D5: the passive lifecycle call check over a stack of 10/100/1,000
/// conditionals.
fn d5_scope_join(cases: &mut Vec<Case>) {
    for count in [10usize, 100, 1_000] {
        cases.push(Case::new("D5/conditionals", count, false, move || {
            let (dir, root) = temp_root();
            let text = conditional_stack(count);
            let path = root.join("doc.md");
            std::fs::write(&path, &text).expect("write document");
            let document = Markdown::try_from(path.as_path()).expect("open document");
            let effective = DarkmatterSchemas::new(context_at(&root))
                .effective_for(&document)
                .expect("schema resolves")
                .expect("declared schema");
            let value = frontmatter_value(&text);
            let profile = claudine_profile();
            let config =
                parse_lifecycle_config_with(&value, &path, &profile.parse_options(None)).expect("the lifecycle parses");
            Box::new(move || {
                let _keep = &dir;
                black_box(lifecycle_call_checks(&value, &config, profile.registry(), Some(&effective)));
            })
        }));
    }
}

/// D6: locating 10/100/1,000 findings in the last thousand lines of a
/// 10,000-line source, one `one_based_position` call each.
fn d6_position_prefix_scan(cases: &mut Vec<Case>) {
    for findings in [10usize, 100, 1_000] {
        cases.push(Case::new("D6/late_findings", findings, false, move || {
            let text: String = (0..10_000).map(|i| format!("line {i:05}: an ordinary authored line of frontmatter text\n")).collect();
            let line_starts: Vec<usize> = std::iter::once(0).chain(text.match_indices('\n').map(|(index, _)| index + 1)).collect();
            let offsets: Vec<usize> = (0..findings).map(|j| line_starts[9_000 + j * 1_000 / findings] + 7).collect();
            Box::new(move || {
                for offset in &offsets {
                    black_box(one_based_position(&text, *offset));
                }
            })
        }));
    }
}

/// D7: a Claudine document whose `start` stack holds 10/100/1,000 actions,
/// validated warm against the frozen bundle; operand discovery records one
/// site per action.
fn d7_operand_sites(cases: &mut Vec<Case>) {
    for actions in [10usize, 100, 1_000] {
        cases.push(Case::new("D7/start_actions", actions, false, move || {
            let repository = ClaudineRepository::new();
            let context = repository.context();
            let mut text = String::from("---\nprompt: hi\nstart:\n");
            for i in 0..actions {
                text.push_str(&format!("  - info: \"step {i}\"\n"));
            }
            text.push_str("---\nbody\n");
            let document = repository.document("doc.md", &text);
            validate_claudine(&context, &document);
            Box::new(move || {
                let _keep = &repository;
                black_box(validate_claudine(&context, &document));
            })
        }));
    }
}

/// D8: one command template with 1/10/50 holes, through the passes
/// candidate construction runs over it: grammar (`parse_authored`), type
/// projection, and the late-root lookup per placeholder. Reproduces
/// `CandidateScope::build`'s public pieces; the guard lookup it adds for a
/// late placeholder is not reached (no hole reads a late root).
fn d8_placeholder_parses(cases: &mut Vec<Case>) {
    for holes in [1usize, 10, 50] {
        cases.push(Case::new("D8/template_holes", holes, false, move || {
            let command: String =
                std::iter::once("echo".to_string()).chain((0..holes).map(|i| format!("{{{{ fm.a{i} }}}}"))).collect::<Vec<_>>().join(" ");
            let roots = DeferredShellRoots::new(["seq"]);
            let derived = HashSet::new();
            Box::new(move || {
                let declared = DeclaredTypes::for_schema(None);
                let scope = TypeScope::unguarded(&declared);
                let mut template = parse_authored(&command).expect("a valid template");
                template.project_types(&scope);
                for placeholder in template.placeholders() {
                    black_box(roots.late_roots_of(&placeholder.expression, &derived));
                }
                black_box(template);
            })
        }));
    }
    // One parse of a short expression, for converting D8's parse counts into
    // time and bytes.
    cases.push(Case::new("D8/single_parse", 1, false, || {
        Box::new(|| {
            black_box(parse("fm.a1").expect("parses"));
        })
    }));
}

/// Builds a D9 frontmatter from its payload size in kilobytes.
type PayloadBuilder = fn(usize) -> Value;

/// `count` plain `info` actions of about 100 bytes each.
fn info_actions(count: usize) -> Vec<Value> {
    (0..count).map(|i| json!({ "info": format!("{i:06} {}", "x".repeat(88)) })).collect()
}

/// D9: lifecycle parsing of a stack of 1/10/100 KB placed under a loop's
/// `gate` (copied twice by `parse_loop_concerns`) and, as the control, under
/// `start`; and one guarded compact item whose sibling verb carries 1/10/100
/// KB (cloned to remove `when`), against the same item unguarded.
fn d9_grammar_clones(cases: &mut Vec<Case>) {
    for kilobytes in [1usize, 10, 100] {
        let variants: [(&'static str, PayloadBuilder); 4] = [
            ("D9/gate_stack", |kb| json!({ "loop": { "until": "done", "gate": info_actions(kb * 10) } })),
            ("D9/start_stack_control", |kb| json!({ "start": info_actions(kb * 10) })),
            ("D9/guarded_item", |kb| json!({ "start": [{ "when": "flag", "info": "x".repeat(kb * 1_000) }] })),
            ("D9/unguarded_item_control", |kb| json!({ "start": [{ "info": "x".repeat(kb * 1_000) }] })),
        ];
        for (item, build) in variants {
            cases.push(Case::new(item, format!("{kilobytes}KB"), false, move || {
                let value = build(kilobytes);
                let profile = claudine_profile();
                Box::new(move || {
                    let options = profile.parse_options(None);
                    black_box(parse_lifecycle_config_with(&value, Path::new("doc.md"), &options).expect("the lifecycle parses"));
                })
            }));
        }
    }
}

/// The A3 document scaled `factor` times: every lifecycle stack (and the
/// loop gate) repeats its items. Comments are dropped.
pub fn scaled_a3(factor: usize) -> String {
    if factor == 1 {
        return A3_DOCUMENT.to_string();
    }
    let (yaml, body) = A3_DOCUMENT.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---\n")).expect("frontmatter");
    let mut value: YamlValue = serde_yaml_ng::from_str(yaml).expect("YAML");
    let repeat = |stack: &mut YamlValue| {
        if let YamlValue::Sequence(items) = stack {
            let original = items.clone();
            for _ in 1..factor {
                items.extend(original.iter().cloned());
            }
        }
    };
    if let YamlValue::Mapping(map) = &mut value {
        for event in ["initialize", "start", "success"] {
            if let Some(stack) = map.get_mut(event) {
                repeat(stack);
            }
        }
        if let Some(YamlValue::Mapping(block)) = map.get_mut("loop")
            && let Some(gate) = block.get_mut("gate")
        {
            repeat(gate);
        }
    }
    format!("---\n{}---\n{body}", serde_yaml_ng::to_string(&value).expect("YAML serializes"))
}

/// D10: one `LifecycleSourceMap` build over the A3 document's frontmatter,
/// at 1× and 10×; A3 reports how many builds one edit loop makes.
fn d10_source_map(cases: &mut Vec<Case>) {
    for factor in [1usize, 10] {
        cases.push(Case::new("D10/source_map_build", format!("a3x{factor}"), false, move || {
            let value = frontmatter_value(&scaled_a3(factor));
            let profile = claudine_profile();
            Box::new(move || {
                black_box(LifecycleSourceMap::from_frontmatter_in(&value, profile.registry()));
            })
        }));
        // The lifecycle parse DMLS and preparation run on the same document,
        // for D9's share of A3.
        cases.push(Case::new("D9/a3_parse", format!("a3x{factor}"), false, move || {
            let value = frontmatter_value(&scaled_a3(factor));
            let profile = claudine_profile();
            Box::new(move || {
                let options = profile.parse_options(None);
                black_box(parse_lifecycle_config_with(&value, Path::new("doc.md"), &options).expect("the A3 lifecycle parses"));
            })
        }));
    }
}

/// Rejecting overload types, in the order they are added.
const REJECTING_TYPES: [&str; 8] = ["string", "number", "boolean", "null", "string[]", "number[]", "boolean[]", "literal(x)"];

/// D11: a successful call whose object argument (1/10/100 KB) is rejected
/// by 1/4/8 overloads before the last, `unknown`, accepts it.
fn d11_rejected_candidates(cases: &mut Vec<Case>) {
    for rejected in [1usize, 4, 8] {
        for kilobytes in [1usize, 10, 100] {
            cases.push(Case::new("D11/successful_bind", format!("{rejected}rej-{kilobytes}KB"), false, move || {
                let signatures: Vec<String> = REJECTING_TYPES[..rejected]
                    .iter()
                    .chain(std::iter::once(&"unknown"))
                    .map(|ty| format!("function(parameters(v: {ty}); returns(boolean))"))
                    .collect();
                let registration = FunctionRegistration::builder("f", signatures.join(" | ")).build().expect("the signatures compile");
                let object: serde_json::Map<String, Value> =
                    (0..kilobytes * 10).map(|i| (format!("k{i:04}"), Value::String("v".repeat(90)))).collect();
                let arguments = [Value::Object(object)];
                Box::new(move || {
                    black_box(registration.bind_call(&arguments).expect("the `unknown` overload binds"));
                })
            }));
        }
    }
}

/// D12: an edit-and-reuse loop over 10 and 300 imported files: each run
/// edits one file (new content, so a new revision) and resolves a schema
/// importing every file. Past 256 revisions the memo empties completely.
fn d12_named_type_memo(cases: &mut Vec<Case>) {
    for files in [10usize, 300] {
        cases.push(Case::new("D12/edit_and_reuse", files, false, move || {
            let (dir, root) = temp_root();
            let mut wiring = serde_yaml_ng::Mapping::new();
            for i in 0..files {
                std::fs::write(root.join(format!("t{i}.yaml")), format!("kind: schema\ntypes:\n  t{i}: string\n")).expect("write types");
                wiring.insert(format!("p{i}").into(), format!("t{i}@./t{i}.yaml").into());
            }
            let wiring = YamlValue::Mapping(wiring);
            let context = context_at(&root);
            resolve_yaml_schema(&wiring, &root, &context).expect("resolves");
            let mut edit = 0usize;
            Box::new(move || {
                edit += 1;
                std::fs::write(root.join("t0.yaml"), format!("kind: schema\ntypes:\n  t0: string\n# edit {edit}\n")).expect("edit types");
                black_box(resolve_yaml_schema(&wiring, &root, &context).expect("resolves"));
                let _keep = &dir;
            })
        }));
    }
}

/// D13: resolving a schema with 1/10/100 imports of one suggested
/// `alphanumeric` type, cold.
fn d13_alphanumeric_checks(cases: &mut Vec<Case>) {
    for imports in [1usize, 10, 100] {
        cases.push(Case::new("D13/suggested_imports", imports, true, move || {
            let (dir, root) = temp_root();
            std::fs::write(root.join("types.yaml"), "kind: schema\ntypes:\n  slug: \"alphanumeric(suggest(ab, abc))\"\n").expect("write types");
            let mut wiring = serde_yaml_ng::Mapping::new();
            for i in 0..imports {
                wiring.insert(format!("p{i}").into(), "slug@./types.yaml".into());
            }
            let wiring = YamlValue::Mapping(wiring);
            let context = context_at(&root);
            Box::new(move || {
                let _keep = &dir;
                black_box(resolve_yaml_schema(&wiring, &root, &context).expect("resolves"));
            })
        }));
    }
}

/// X3: the conversion translation of the frozen recursive lifecycle
/// schema, once.
fn x3_recursive_translation(cases: &mut Vec<Case>) {
    cases.push(Case::new("X3/recursive_translation", "frozen-recursive", true, || {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recursive_schema_frozen");
        let mut wiring = serde_yaml_ng::Mapping::new();
        for event in ["initialize", "start", "blocked", "success", "failure", "finalize"] {
            wiring.insert(event.into(), "stack@./lifecycle-recursive.yaml".into());
        }
        wiring.insert("loop".into(), "loop@./lifecycle-recursive.yaml".into());
        let context = biscuit_file::FileResolutionContext::new(&dir);
        let json = resolve_yaml_schema(&YamlValue::Mapping(wiring), &dir, &context).expect("resolves").json_schema;
        let definitions = json.get("$defs").and_then(Value::as_object).map_or(0, serde_json::Map::len);
        eprintln!("X3: the frozen recursive schema has {definitions} `$defs` entries");
        Box::new(move || {
            black_box(shape_from_json_schema(&json));
        })
    }));
}

/// X4: one document whose 1,000 top-level properties are all invalid,
/// validated with positions.
fn x4_wide_mapping_problems(cases: &mut Vec<Case>) {
    cases.push(Case::new("X4/wide_invalid_mapping", 1_000, false, || {
        let mut text = String::from("---\n$schema:\n");
        for i in 0..1_000 {
            text.push_str(&format!("  p{i}: number\n"));
        }
        for i in 0..1_000 {
            text.push_str(&format!("p{i}: not-a-number\n"));
        }
        text.push_str("---\nbody\n");
        let (dir, root) = temp_root();
        let path = root.join("doc.md");
        std::fs::write(&path, &text).expect("write document");
        let document = Markdown::try_from(path.as_path()).expect("open document");
        let api = DarkmatterSchemas::new(context_at(&root));
        let report = api.validate(&document).expect("validates");
        assert_eq!(report.problems.len(), 1_000, "one problem per property");
        Box::new(move || {
            let _keep = &dir;
            black_box(api.validate(&document).expect("validates"));
        })
    }));
}
