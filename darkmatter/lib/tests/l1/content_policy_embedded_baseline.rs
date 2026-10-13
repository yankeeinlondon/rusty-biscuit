//! The shipped runtime baseline's `content_policy` declaration, prepared from
//! the library's embedded schema documents (no schema file on disk).
//!
//! `content_policy_disk_baseline` holds the same `content_policy_fixtures`
//! declarations to the disk resolver; `base_schema_end_to_end` proves the
//! disk-resolved authored baseline equals the embedded one, typed and
//! lowered. The schema-mutation table runs against the embedded source in
//! the library's own `schemas::tests::embedded_sources`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, PropertyDef, SchemaOriginKind, SimplifiedSchema, TypeExpr,
    ValidationProblemKind, ValidationReport, darkmatter_base_json_schema_ref,
    darkmatter_base_schema,
};

use crate::content_policy_fixtures::{
    ACCEPTANCE_REJECTIONS, BASELINE_DECLARATION_LINE, DECLARATIONS, MIXED_POLICY_FRONTMATTER, Verdict,
    declaration, render,
};
use crate::request_support::context_at;

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

fn embedded_api(root: &Path) -> DarkmatterSchemas {
    DarkmatterSchemas::new(context_at(root))
        .with_darkmatter_baseline_json_schema()
        .expect("the embedded baseline attaches")
}

fn load(root: &Path, frontmatter: &str) -> Markdown {
    let path = root.join("doc.md");
    std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("write document");
    Markdown::try_from(path.as_path())
        .expect("the document loads")
        .with_source(ComposeSource::File(path))
}

fn validate(api: &DarkmatterSchemas, root: &Path, frontmatter: &str) -> ValidationReport {
    api.validate(&load(root, frontmatter)).expect("the schema prepares")
}

fn kind_name(kind: &ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

#[test]
fn the_runtime_baseline_declares_the_policy_list() {
    let shipped = include_str!("../../../docs/schemas/darkmatter.yaml");
    assert_eq!(shipped.matches(BASELINE_DECLARATION_LINE).count(), 1);
}

/// The embedded baseline prepares, lowers, and compiles; the authoring
/// example and a policy-free document are valid, typed by the baseline, and
/// nothing embedded becomes a filesystem dependency.
#[test]
fn the_authoring_example_validates_against_the_embedded_baseline() {
    let (_dir, root) = temp_root();
    let api = embedded_api(&root);

    let mixed = validate(&api, &root, MIXED_POLICY_FRONTMATTER);
    assert!(mixed.valid, "{:#?}", mixed.problems);
    let plain = validate(&api, &root, "title: no policy\n");
    assert!(plain.valid, "{:#?}", plain.problems);

    let effective = api
        .effective_for(&load(&root, MIXED_POLICY_FRONTMATTER))
        .expect("prepares")
        .expect("the baseline applies");
    assert_eq!(
        effective.origins.get("content_policy").map(|origin| origin.kind),
        Some(SchemaOriginKind::Baseline)
    );
    assert!(effective.declares_top_level_property("content_policy"));
    assert!(
        effective.dependencies().is_empty(),
        "embedded documents are not watched: {:?}",
        effective.dependencies()
    );
}

/// Every shared declaration, in each presentation, reports exactly its
/// expected problems at the authored node, against `content_policy` as the
/// baseline types it.
#[test]
fn declarations_meet_their_baseline_expectations_through_the_embedded_baseline() {
    let (_dir, root) = temp_root();
    let api = embedded_api(&root);

    let mut failures = Vec::new();
    for case in DECLARATIONS {
        for rendered in render(case.declaration) {
            let label = format!("{} ({:?})", case.name, rendered.presentation);
            let report = validate(&api, &root, &rendered.frontmatter);
            let actual: BTreeSet<_> = report
                .problems
                .iter()
                .map(|p| (p.path.clone(), kind_name(&p.kind), p.line, p.column))
                .collect();
            let expected: BTreeSet<_> = case
                .problems
                .iter()
                .map(|p| {
                    let (line, column) = rendered.start(p.at);
                    (p.path.to_string(), p.kind, Some(line), Some(column))
                })
                .collect();
            let verdict = if report.valid { Verdict::Accept } else { Verdict::Reject };
            if verdict != case.schema || actual != expected {
                failures.push(format!(
                    "{label}: expected {:?} {expected:?}, got {verdict:?} {actual:?}",
                    case.schema
                ));
            }
            if !report.problems.is_empty() {
                let effective = api
                    .effective_for(&load(&root, &rendered.frontmatter))
                    .expect("prepares")
                    .expect("the baseline applies");
                let origin = effective.origins.get("content_policy").map(|origin| origin.kind);
                if origin != Some(SchemaOriginKind::Baseline) {
                    failures.push(format!("{label}: content_policy typed by {origin:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The typed baseline carries Content Policy's definitions inline: a
/// non-empty list whose items are the compact rule patterns (with their
/// descriptions and the shipped suggestions) or the closed long form with a
/// required `rule` union and a required `action` enum.
#[test]
fn the_typed_baseline_carries_the_policy_definitions() {
    let policy = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let SimplifiedSchema::Single(shape) = darkmatter_base_schema() else {
        panic!("the baseline is a single shape");
    };
    let PropertyDef::Single(list) = &shape.properties["content_policy"] else {
        panic!("content_policy is one list atom");
    };
    assert!(list.is_array);
    assert!(!list.constraints.contains(&Constraint::Required), "the list is optional");
    let TypeExpr::Union(arms) = &list.ty else {
        panic!("list items are a union: {list:?}");
    };

    let compact: Vec<_> = arms
        .iter()
        .filter(|arm| !matches!(arm.ty, TypeExpr::InlineObject(_)))
        .collect();
    let descriptions: Vec<_> = compact.iter().map(|arm| arm.description.as_deref().unwrap_or("")).collect();
    for description in &descriptions {
        assert!(policy.contains(&format!("-> {description}'")), "`{description}` is a shipped description");
    }
    assert_eq!(descriptions.len(), policy.matches("    - 'string(pattern(").count());
    let suggestions: Vec<_> = compact
        .iter()
        .flat_map(|arm| arm.constraints.iter())
        .filter_map(|constraint| match constraint {
            Constraint::Suggest(candidates) => Some(candidates.iter().map(|c| c.decoded.clone()).collect::<Vec<_>>()),
            _ => None,
        })
        .collect();
    assert_eq!(
        suggestions,
        [[
            "Evergreen",
            "TimeSensitive",
            "ValidFor(3mo)",
            "ValidFor(3mo, @last_updated)",
            "ValidFor(3mo, 2026-09-28)",
            "ValidUntil(2027-01-01)",
            "FileChanged(src/config.rs, @config_fingerprint)",
        ]]
    );

    let long_forms: Vec<_> = arms
        .iter()
        .filter_map(|arm| match &arm.ty {
            TypeExpr::InlineObject(shape) => Some(shape),
            _ => None,
        })
        .collect();
    let [long_form] = long_forms.as_slice() else {
        panic!("exactly one long-form arm: {long_forms:?}");
    };
    let PropertyDef::Single(rule) = &long_form.properties["rule"] else {
        panic!("rule is one atom");
    };
    assert!(rule.constraints.contains(&Constraint::Required));
    assert!(matches!(&rule.ty, TypeExpr::Union(rule_arms) if rule_arms.len() == compact.len()));
    let PropertyDef::Single(action) = &long_form.properties["action"] else {
        panic!("action is one atom");
    };
    assert!(action.constraints.contains(&Constraint::Required));
    assert!(action.constraints.contains(&Constraint::Members(
        ["refresh", "archive", "remove"].map(String::from).to_vec()
    )));

    let lowered = &darkmatter_base_json_schema_ref()["properties"]["content_policy"];
    assert!(lowered.to_string().contains("\"minItems\":1"), "{lowered}");
}

/// The baseline is prepared and lowered once per process: every accessor and
/// builder shares one allocation. Two requests rooted outside the checkout,
/// each holding only its test document, accept the authoring example and
/// reject the specification's invalid policies at their instance paths; the
/// second load reuses the first's resolved baseline.
#[test]
fn the_embedded_baseline_is_prepared_once_per_process() {
    let checkout = biscuit_test_harness::manifest_dir!()
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(|checkout| biscuit_file::canonicalize_simplified(checkout).expect("canonical checkout"));
    let first = darkmatter_base_json_schema_ref();

    for _load in 0..2 {
        let (_dir, root) = temp_root();
        if let Some(checkout) = &checkout {
            assert!(!root.starts_with(checkout), "{} is inside {}", root.display(), checkout.display());
        }
        let api = embedded_api(&root);
        let mixed = validate(&api, &root, MIXED_POLICY_FRONTMATTER);
        assert!(mixed.valid, "{:#?}", mixed.problems);
        for name in ACCEPTANCE_REJECTIONS {
            let case = declaration(name);
            let rendered = render(case.declaration).remove(0);
            let report = validate(&api, &root, &rendered.frontmatter);
            let paths: Vec<_> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
            let expected: Vec<_> = case.problems.iter().map(|problem| problem.path).collect();
            assert!(!report.valid, "{name}");
            assert_eq!(paths, expected, "{name}");
        }
        let entries: Vec<_> = std::fs::read_dir(&root)
            .expect("list the request root")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(entries, ["doc.md"], "only the test document exists");
        assert!(std::ptr::eq(first, darkmatter_base_json_schema_ref()));
    }
    assert_eq!(darkmatter_base_schema(), darkmatter_base_schema());
}

/// Types named `policy`, `short_form`, and `long_form` that mean something
/// else: a document or trigger importing them must not see Content Policy's.
const SAME_NAMED_TYPES: &str = "kind: schema\ntypes:\n  short_form: number\n  long_form: \"{ note: string }\"\n  policy: [short_form, long_form]\n";

/// A matching trigger and a document replace the embedded baseline's
/// `content_policy` with their own same-named types, and a document importing
/// those names for another property leaves the baseline's typing intact.
#[test]
fn triggers_and_documents_override_the_embedded_declaration_without_name_collisions() {
    let (_dir, root) = temp_root();
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
    std::fs::write(root.join("schemas/same-names.yaml"), SAME_NAMED_TYPES).expect("types");
    std::fs::write(
        root.join("schemas/review.trigger.yaml"),
        "kind: trigger-schema\nmatch:\n  review: string(required)\n$schema: review.yaml\n",
    )
    .expect("trigger");
    std::fs::write(
        root.join("schemas/review.yaml"),
        "$schema:\n  content_policy: \"policy[]@./same-names.yaml\"\n",
    )
    .expect("trigger payload");
    let context = biscuit_file::FileResolutionContext::from_snapshot(&root, None, std::collections::HashMap::new())
        .with_repository_root(&root);
    let api = DarkmatterSchemas::new(context)
        .with_darkmatter_baseline_json_schema()
        .expect("the baseline attaches")
        .with_trigger_discovery()
        .expect("the trigger loads");
    let check = |frontmatter: &str| {
        let markdown = load(&root, frontmatter);
        let effective = api.effective_for(&markdown).expect("prepares").expect("applies");
        let report = api.validate(&markdown).expect("prepares");
        (effective.origins.get("content_policy").map(|o| o.kind), report.valid)
    };

    assert_eq!(check("content_policy: [\"ValidFor(3mo)\"]\n"), (Some(SchemaOriginKind::Baseline), true));
    assert_eq!(check("content_policy: [3]\n"), (Some(SchemaOriginKind::Baseline), false));

    let triggered = "review: yes\n";
    assert_eq!(
        check(&format!("{triggered}content_policy: [3, {{note: x}}]\n")),
        (Some(SchemaOriginKind::Trigger), true)
    );
    assert_eq!(
        check(&format!("{triggered}content_policy: [\"ValidFor(3mo)\"]\n")),
        (Some(SchemaOriginKind::Trigger), false)
    );

    let declared = "$schema:\n  content_policy: string\n  local: \"policy@./schemas/same-names.yaml\"\n";
    assert_eq!(
        check(&format!("{triggered}{declared}content_policy: anything\nlocal: 4\n")),
        (Some(SchemaOriginKind::Document), true)
    );
    let local_only = "$schema:\n  local: \"policy@./schemas/same-names.yaml\"\n";
    assert_eq!(
        check(&format!("{local_only}local: 4\ncontent_policy: [\"ValidFor(3mo)\"]\n")),
        (Some(SchemaOriginKind::Baseline), true)
    );
    assert_eq!(
        check(&format!("{local_only}local: 4\ncontent_policy: [4]\n")),
        (Some(SchemaOriginKind::Baseline), false)
    );
}
