//! The `content_policy: "policy[](min(1))@…"` baseline declaration, loaded
//! from disk through the normal resolver with the repository's layout: the
//! global catalog at `darkmatter/schemas/darkmatter.yaml`, whose `fm` partial
//! (`darkmatter/schemas/partials/fm.yaml`) imports Content Policy's editor
//! schema three directories up. A catalog loaded as a baseline file
//! contributes its resolved `fm`.
//!
//! These tests hold the shared `content_policy_fixtures` tables to the disk
//! resolver and to Content Policy's parser, so a second schema source can be
//! held to the same outcomes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, SchemaError, SchemaOriginKind, ValidationProblemKind, ValidationReport,
};

use crate::content_policy_fixtures::{
    BASELINE_PATH, CATALOG_PATH, DECLARATIONS, OTHER_PARTIAL_PATHS, INVALID_CONTROL, LOAD_FAILURES, MIXED_POLICY_FRONTMATTER,
    POLICY_SCHEMA_PATH, SCHEMA_MUTATIONS, SchemaExpect, SchemaFile, VALID_CONTROL, Verdict,
    baseline_with_policy, render,
};
use crate::request_support::context_at;
use crate::union_fixtures::content_policy_union_schema;

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// The shipped catalog's texts other than the `fm` partial and the policy
/// schema: the catalog itself and the [`OTHER_PARTIAL_PATHS`], in order.
struct Catalog<'a> {
    catalog: &'a str,
    others: [&'a str; 4],
}

/// Writes the catalog, its partials (`baseline` is the `fm` partial), and the
/// policy schema at their repository-relative paths below `root`.
fn write_schemas(root: &Path, catalog: &Catalog<'_>, baseline: &str, policy: &str) {
    let files = [(CATALOG_PATH, catalog.catalog), (BASELINE_PATH, baseline), (POLICY_SCHEMA_PATH, policy)]
        .into_iter()
        .chain(OTHER_PARTIAL_PATHS.into_iter().zip(catalog.others));
    for (relative, text) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("schema dir");
        std::fs::write(path, text).expect("write schema");
    }
}

fn baseline_api(root: &Path) -> Result<DarkmatterSchemas, SchemaError> {
    DarkmatterSchemas::new(context_at(root)).with_baseline_from_file(root.join(CATALOG_PATH))
}

macro_rules! shipped_catalog {
    () => {
        Catalog {
            catalog: include_str!("../../../schemas/darkmatter.yaml"),
            others: [
                include_str!("../../../schemas/partials/style.yaml"),
                include_str!("../../../schemas/partials/context-variables.yaml"),
                include_str!("../../../schemas/partials/error.yaml"),
                include_str!("../../../schemas/partials/timing.yaml"),
            ],
        }
    };
}

/// Loads `text` as `root/doc.md`. `None` when the document reader rejects it.
fn load(root: &Path, text: &str) -> Option<Markdown> {
    let path = root.join("doc.md");
    std::fs::write(&path, text).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).ok()?;
    Some(markdown.with_source(ComposeSource::File(path)))
}

fn document(frontmatter: &str) -> String {
    format!("---\n{frontmatter}---\nBody\n")
}

fn validate(api: &DarkmatterSchemas, root: &Path, frontmatter: &str) -> ValidationReport {
    let markdown = load(root, &document(frontmatter)).expect("the document loads");
    api.validate(&markdown).expect("the schema prepares")
}

fn kind_name(kind: &ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

#[test]
fn the_authoring_example_and_a_policy_free_document_validate_against_the_baseline() {
    let shipped_baseline = include_str!("../../../schemas/partials/fm.yaml");
    let catalog = shipped_catalog!();
    let shipped_policy = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_schemas(&root, &catalog, &baseline_with_policy(shipped_baseline), &content_policy_union_schema(shipped_policy));
    let api = baseline_api(&root).expect("the baseline prepares");

    let mixed = validate(&api, &root, MIXED_POLICY_FRONTMATTER);
    assert!(mixed.valid, "{:#?}", mixed.problems);
    let plain = validate(&api, &root, "title: no policy\n");
    assert!(plain.valid, "{:#?}", plain.problems);

    let markdown = load(&root, &document(MIXED_POLICY_FRONTMATTER)).expect("loads");
    let effective = api.effective_for(&markdown).expect("prepares").expect("the baseline applies");
    assert_eq!(
        effective.origins.get("content_policy").map(|origin| origin.kind),
        Some(SchemaOriginKind::Baseline)
    );
    assert!(effective.declares_top_level_property("content_policy"));
}

/// Every shared declaration, in each presentation, reports exactly its
/// expected problems at the authored node.
#[test]
fn declarations_meet_their_baseline_expectations() {
    let shipped_baseline = include_str!("../../../schemas/partials/fm.yaml");
    let catalog = shipped_catalog!();
    let shipped_policy = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_schemas(&root, &catalog, &baseline_with_policy(shipped_baseline), &content_policy_union_schema(shipped_policy));
    let api = baseline_api(&root).expect("the baseline prepares");

    let names: BTreeSet<_> = DECLARATIONS.iter().map(|case| case.name).collect();
    assert_eq!(names.len(), DECLARATIONS.len(), "case names are unique");

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
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every shared declaration, in each presentation, meets its parser
/// expectation; an absent declaration is never parsed.
#[test]
fn declarations_meet_their_parser_expectations() {
    let mut failures = Vec::new();
    for case in DECLARATIONS {
        for rendered in render(case.declaration) {
            let label = format!("{} ({:?})", case.name, rendered.presentation);
            let frontmatter: serde_json::Value =
                serde_yaml_ng::from_str(&rendered.frontmatter).expect("the frontmatter parses");
            let declaration = frontmatter.get("content_policy");
            let verdict = declaration.map(|value| {
                match content_policy::Policy::from_declaration(value) {
                    Ok(_) => Verdict::Accept,
                    Err(_) => Verdict::Reject,
                }
            });
            if verdict != case.parser {
                failures.push(format!("{label}: expected {:?}, got {verdict:?}", case.parser));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Duplicate keys and malformed YAML fail at the document reader, before the
/// parser or the baseline can see a deduplicated or partial mapping.
#[test]
fn damaged_frontmatter_fails_document_loading() {
    let (_dir, root) = temp_root();
    for (name, frontmatter) in LOAD_FAILURES {
        assert!(load(&root, &document(frontmatter)).is_none(), "{name} loaded");
    }
    assert!(load(&root, &document(MIXED_POLICY_FRONTMATTER)).is_some(), "the control loads");
}

// ── schema mutations ───────────────────────────────────────────────────────

fn run_column(column: &str, shipped: &Catalog<'_>, shipped_baseline: &str, shipped_policy: &str) {
    let baseline = baseline_with_policy(shipped_baseline);
    let policy = content_policy_union_schema(shipped_policy);
    let (_dir, root) = temp_root();
    let cells: Vec<_> = SCHEMA_MUTATIONS.iter().filter(|m| m.column == column).collect();
    assert!(!cells.is_empty(), "column `{column}` has cells");
    assert_eq!(cells[0].expect, SchemaExpect::Control, "the first `{column}` row is its control");

    let mut failures = Vec::new();
    for cell in cells {
        let (catalog, baseline, policy) = match cell.file {
            SchemaFile::Catalog => (cell.edit.apply(shipped.catalog), baseline.clone(), policy.clone()),
            SchemaFile::Baseline => (shipped.catalog.to_string(), cell.edit.apply(&baseline), policy.clone()),
            SchemaFile::Policy => (shipped.catalog.to_string(), baseline.clone(), cell.edit.apply(&policy)),
        };
        let catalog = Catalog { catalog: &catalog, others: shipped.others };
        write_schemas(&root, &catalog, &baseline, &policy);
        let outcome = baseline_api(&root).and_then(|api| {
            let verdict = |frontmatter: &str| -> Result<bool, SchemaError> {
                let markdown = load(&root, &document(frontmatter)).expect("the control loads");
                Ok(api.validate(&markdown)?.valid)
            };
            Ok((
                verdict(VALID_CONTROL)?,
                verdict(INVALID_CONTROL)?,
                cell.probe.map(|(frontmatter, _)| verdict(frontmatter)).transpose()?,
            ))
        });
        let actual = match &outcome {
            Err(_) => SchemaExpect::PrepareFails,
            Ok((true, false, _)) => SchemaExpect::Control,
            Ok((true, true, _)) => SchemaExpect::Weakened,
            Ok((false, _, _)) => SchemaExpect::RejectsValid,
        };
        let probe_ok = match (cell.probe, &outcome) {
            (Some((_, expected)), Ok((_, _, Some(valid)))) => {
                (expected == Verdict::Accept) == *valid
            }
            (Some(_), _) => false,
            (None, _) => true,
        };
        if actual != cell.expect || !probe_ok {
            failures.push(format!(
                "{column} / {}: expected {:?} (probe {:?}), got {actual:?} from {outcome:?}",
                cell.cell, cell.expect, cell.probe
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

macro_rules! mutation_columns {
    ($($test:ident => $column:literal),+ $(,)?) => {
        $(
            #[test]
            fn $test() {
                run_column(
                    $column,
                    &shipped_catalog!(),
                    include_str!("../../../schemas/partials/fm.yaml"),
                    include_str!("../../../../content-policy/schemas/content-policy.yaml"),
                );
            }
        )+

        /// Every column in the shared table has a test above.
        #[test]
        fn every_mutation_column_has_a_test() {
            let tested: BTreeSet<&str> = [$($column),+].into_iter().collect();
            let declared: BTreeSet<&str> = SCHEMA_MUTATIONS.iter().map(|m| m.column).collect();
            assert_eq!(tested, declared);
        }
    };
}

mutation_columns! {
    baseline_envelope_cells => "$schema",
    policy_kind_cells => "kind",
    policy_types_cells => "types",
    baseline_declaration_cells => "$schema.content_policy",
    short_form_cells => "types.short_form",
    long_form_cells => "types.long_form",
    policy_definition_cells => "types.policy",
    long_form_rule_cells => "long_form.rule",
    long_form_action_cells => "long_form.action",
}

// ── precedence and definition isolation ────────────────────────────────────

/// Types named `policy`, `short_form`, and `long_form` that mean something
/// else: a document or trigger importing them must not see Content Policy's.
const SAME_NAMED_TYPES: &str = "kind: schema\ntypes:\n  short_form: number\n  long_form: \"{ note: string }\"\n  policy: [short_form, long_form]\n";

fn trigger_context(root: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root)
}

/// A matching trigger replaces the baseline's `content_policy`, and the
/// document's own declaration replaces the trigger's. Both import types whose
/// names match Content Policy's without disturbing the baseline's typing of
/// other documents.
#[test]
fn triggers_and_documents_replace_the_baseline_declaration_without_name_collisions() {
    let shipped_baseline = include_str!("../../../schemas/partials/fm.yaml");
    let catalog = shipped_catalog!();
    let shipped_policy = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_schemas(&root, &catalog, &baseline_with_policy(shipped_baseline), &content_policy_union_schema(shipped_policy));
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
    std::fs::write(root.join("schemas/same-names.yaml"), SAME_NAMED_TYPES).expect("types");
    std::fs::write(
        root.join("schemas/review.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  review: string(required)\n$schema: review.yaml\n",
    )
    .expect("trigger");
    std::fs::write(
        root.join("schemas/review.yaml"),
        "$schema:\n  content_policy: \"policy[]@./same-names.yaml\"\n",
    )
    .expect("trigger payload");

    let api = DarkmatterSchemas::new(trigger_context(&root))
        .with_baseline_from_file(root.join(CATALOG_PATH))
        .expect("the baseline prepares")
        .with_trigger_discovery()
        .expect("the trigger loads");
    let check = |frontmatter: &str| {
        let markdown = load(&root, &document(frontmatter)).expect("loads");
        let effective = api.effective_for(&markdown).expect("prepares").expect("applies");
        let report = api.validate(&markdown).expect("prepares");
        (effective.origins.get("content_policy").map(|o| o.kind), report.valid)
    };

    // Baseline: Content Policy's types.
    assert_eq!(check("content_policy: [\"ValidFor(3mo)\"]\n"), (Some(SchemaOriginKind::Baseline), true));
    assert_eq!(check("content_policy: [3]\n"), (Some(SchemaOriginKind::Baseline), false));
    assert_eq!(check("content_policy: []\n"), (Some(SchemaOriginKind::Baseline), false));

    // Trigger: its own same-named types, no minimum.
    let triggered = "review: yes\n";
    assert_eq!(
        check(&format!("{triggered}content_policy: [3, {{note: x}}]\n")),
        (Some(SchemaOriginKind::Trigger), true)
    );
    assert_eq!(
        check(&format!("{triggered}content_policy: [\"ValidFor(3mo)\"]\n")),
        (Some(SchemaOriginKind::Trigger), false)
    );
    assert_eq!(check(&format!("{triggered}content_policy: []\n")), (Some(SchemaOriginKind::Trigger), true));

    // Document: its own declaration wins over the trigger and the baseline,
    // even when it imports the same names.
    let declared = "$schema:\n  content_policy: string\n  local: \"policy@./schemas/same-names.yaml\"\n";
    assert_eq!(
        check(&format!("{triggered}{declared}content_policy: anything\nlocal: 4\n")),
        (Some(SchemaOriginKind::Document), true)
    );
    assert_eq!(
        check(&format!("{declared}content_policy: anything\nlocal: \"ValidFor(3mo)\"\n")),
        (Some(SchemaOriginKind::Document), false)
    );
    // A document importing the same names leaves the baseline's typing of an
    // undeclared `content_policy` intact.
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
