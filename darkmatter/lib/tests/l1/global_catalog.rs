//! Darkmatter's global catalog, `darkmatter/schemas/darkmatter.yaml`, through
//! its public boundary ([`GlobalCatalog`]).
//!
//! The catalog declares every global's shape once; the runtime baseline is
//! its resolved `fm`, the `ctx.*` descriptors project its `ctx`, and the
//! runtime `err`/`timing` values have exactly its members. The input
//! robustness matrix at the end edits one field of the shipped files per
//! test, from an unedited positive control.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::lifecycle::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticSnapshot, LifecycleErrorInfo, LifecycleTiming};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::context::context_variable_descriptors;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::catalog::{
    CatalogError, CatalogErrorReason, DARKMATTER_CATALOG_PATH, DARKMATTER_GLOBALS, GlobalCatalog,
};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, PropertyDef, SchemaError, SchemaOriginKind, SimplifiedType, TypeExpr,
    darkmatter_base_json_schema, darkmatter_base_schema, to_json_schema,
};

use crate::request_support::context_at;

/// The catalog bundle's repository-relative paths, in [`Texts`] order.
const PATHS: [&str; 7] = [
    DARKMATTER_CATALOG_PATH,
    "darkmatter/schemas/partials/fm.yaml",
    "darkmatter/schemas/partials/style.yaml",
    "darkmatter/schemas/partials/context-variables.yaml",
    "darkmatter/schemas/partials/error.yaml",
    "darkmatter/schemas/partials/timing.yaml",
    "content-policy/schemas/content-policy.yaml",
];

/// The shipped bundle's texts, read by each test with `include_str!`.
macro_rules! shipped {
    () => {
        Texts([
            include_str!("../../../schemas/darkmatter.yaml").to_string(),
            include_str!("../../../schemas/partials/fm.yaml").to_string(),
            include_str!("../../../schemas/partials/style.yaml").to_string(),
            include_str!("../../../schemas/partials/context-variables.yaml").to_string(),
            include_str!("../../../schemas/partials/error.yaml").to_string(),
            include_str!("../../../schemas/partials/timing.yaml").to_string(),
            include_str!("../../../../content-policy/schemas/content-policy.yaml").to_string(),
        ])
    };
}

/// One text per [`PATHS`] entry.
struct Texts([String; 7]);

impl Texts {
    fn documents(&self) -> Vec<(&str, &str)> {
        PATHS.iter().copied().zip(self.0.iter().map(String::as_str)).collect()
    }

    fn load(&self) -> Result<GlobalCatalog, CatalogError> {
        GlobalCatalog::from_documents(DARKMATTER_CATALOG_PATH, &self.documents(), DARKMATTER_GLOBALS)
    }

    fn write_below(&self, root: &Path) {
        for (path, text) in self.documents() {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("schema dir");
            std::fs::write(target, text).expect("write schema");
        }
    }
}

fn shape_keys(catalog: &GlobalCatalog, global: &str) -> Vec<String> {
    catalog
        .global(global)
        .and_then(|declaration| declaration.shape())
        .unwrap_or_else(|| panic!("`{global}` is an object shape"))
        .properties
        .keys()
        .cloned()
        .collect()
}

fn single(def: &PropertyDef) -> &darkmatter::markdown::schemas::PropertyAtom {
    match def {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => panic!("expected a single atom: {def:?}"),
    }
}

// ── resolution and parity ──────────────────────────────────────────────────

#[test]
fn the_shipped_files_load_from_texts_alone_and_equal_the_embedded_catalog() {
    let texts = shipped!();
    let loaded = texts.load().expect("the shipped catalog loads");
    assert_eq!(&loaded, GlobalCatalog::darkmatter());
    let names: Vec<_> = loaded.globals().map(|global| global.name().to_string()).collect();
    assert_eq!(names, ["fm", "ctx", "current", "env", "err", "timing"]);
    assert_eq!(loaded.description(), Some("the shapes of every Darkmatter global"));
}

#[test]
fn imports_resolve_transitively_through_every_partial() {
    let catalog = GlobalCatalog::darkmatter();
    let fm = catalog.global("fm").expect("fm");
    // `fm` → `style@./style.yaml` → its nested `page` shape.
    let style = single(fm.member("style").expect("style"));
    let TypeExpr::InlineObject(style) = &style.ty else { panic!("style is a shape: {style:?}") };
    assert!(style.properties.contains_key("page") && style.properties.contains_key("markdown"));
    // `fm` → `context@./context-variables.yaml`, the same type as the `ctx` global.
    let fm_ctx = single(fm.member("ctx").expect("fm.ctx"));
    let TypeExpr::InlineObject(fm_ctx) = &fm_ctx.ty else { panic!("fm.ctx is a shape") };
    assert_eq!(Some(fm_ctx), catalog.global("ctx").and_then(|ctx| ctx.shape()));
    // `fm` → `policy@../../../content-policy/…`, climbing out of the area.
    let policy = single(fm.member("content_policy").expect("content_policy"));
    assert!(policy.is_array && matches!(policy.ty, TypeExpr::Union(_)), "{policy:?}");
}

/// The embedded (generated) catalog equals the one loaded from the authored
/// files on disk, global by global, and the disk load is anchored at the
/// catalog's own directory, not the request's.
#[test]
fn every_global_has_generated_and_loaded_parity() {
    let dir = tempfile::tempdir().expect("temp dir");
    let elsewhere = tempfile::tempdir().expect("unrelated anchor");
    shipped!().write_below(dir.path());
    let loaded = GlobalCatalog::from_file(
        &dir.path().join(DARKMATTER_CATALOG_PATH),
        DARKMATTER_GLOBALS,
        &context_at(elsewhere.path()),
    )
    .expect("the disk catalog loads");
    let embedded = GlobalCatalog::darkmatter();
    for requirement in DARKMATTER_GLOBALS {
        assert_eq!(loaded.global(&requirement.name), embedded.global(&requirement.name), "{}", requirement.name);
    }
    assert_eq!(&loaded, embedded);
}

#[test]
fn the_runtime_baseline_is_the_catalogs_fm_definition() {
    let baseline = GlobalCatalog::darkmatter().baseline().expect("fm");
    assert_eq!(baseline, darkmatter_base_schema());
    assert_eq!(to_json_schema(&baseline).expect("lowers"), darkmatter_base_json_schema());
}

#[test]
fn ctx_and_current_share_one_shape_projected_by_the_context_descriptors() {
    let catalog = GlobalCatalog::darkmatter();
    let ctx = catalog.global("ctx").and_then(|ctx| ctx.shape()).expect("ctx");
    assert_eq!(Some(ctx), catalog.global("current").and_then(|current| current.shape()));
    let descriptors: Vec<_> = context_variable_descriptors().iter().map(|descriptor| descriptor.name).collect();
    let keys: Vec<_> = ctx.properties.keys().map(String::as_str).collect();
    assert_eq!(descriptors, keys);
    let env = single(catalog.global("env").expect("env").definition());
    assert_eq!(env.ty, TypeExpr::Primitive(SimplifiedType::Object));
}

#[test]
fn timing_declares_the_runtime_fields_as_optional_integers() {
    let catalog = GlobalCatalog::darkmatter();
    let runtime = LifecycleTiming { document_ms: Some(1), total_ms: Some(2), step_ms: Some(3) }.to_value();
    let runtime: BTreeSet<_> = runtime.as_object().expect("object").keys().cloned().collect();
    assert_eq!(shape_keys(catalog, "timing").into_iter().collect::<BTreeSet<_>>(), runtime);
    for member in catalog.global("timing").and_then(|timing| timing.shape()).expect("timing").properties.values() {
        assert!(!member.is_required(), "{member:?}");
        let atom = single(member);
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Number));
        assert!(atom.constraints.contains(&Constraint::Integer), "{atom:?}");
    }
    // Unmeasured fields are absent, read as null.
    assert_eq!(LifecycleTiming::default().to_value(), serde_json::json!({}));
}

#[test]
fn err_declares_exactly_the_runtime_fields() {
    let snapshot = DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: "document".into(),
        code: "document.example".into(),
        disposition: "correctable".into(),
        origin: "author".into(),
        severity: "error".into(),
        detail: serde_json::json!({}),
        message: "example".into(),
        cause: None,
        frontmatter_excerpt: None,
    };
    let value = LifecycleErrorInfo::from_snapshot("document", "document.example".into(), snapshot).to_value();
    let runtime: BTreeSet<_> = value.as_object().expect("object").keys().cloned().collect();
    let declared: BTreeSet<_> = shape_keys(GlobalCatalog::darkmatter(), "err").into_iter().collect();
    assert_eq!(declared, runtime);
    let err = GlobalCatalog::darkmatter().global("err").expect("err");
    let always: BTreeSet<_> = ["kind", "variant", "msg"].into_iter().map(String::from).collect();
    let required: BTreeSet<_> = err
        .shape()
        .expect("shape")
        .properties
        .iter()
        .filter(|(_, def)| def.is_required())
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(required, always, "only the always-present fields are required");
}

/// An authored `ctx:` key is the document's `fm.ctx`; the `ctx` global is the
/// captured context, and neither changes the catalog's declaration.
#[test]
fn an_authored_fm_ctx_stays_independent_of_the_ctx_global() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("doc.md");
    std::fs::write(&path, "---\nctx:\n  note: authored\n---\n[{{ fm.ctx.note }}] [{{ fm.ctx.today == ctx.today }}]\n")
        .expect("write");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("loads")
        .compose_with(&crate::request_support::request(options))
        .expect("composes");
    assert_eq!(composed.content().trim_end(), "[authored] [false]");
    assert!(!shape_keys(GlobalCatalog::darkmatter(), "ctx").contains(&"note".to_string()));
}

// ── never a document schema ────────────────────────────────────────────────

/// Copies the catalog and partials into a repository's `schemas/` root, then
/// reaches the catalog through every discovery path a document has: a bare
/// name, a relative path, a trigger walk of the root, and a `$schema` naming a
/// partial's directory. None applies the catalog's globals as properties.
#[test]
fn the_catalog_never_applies_as_a_document_schema_when_rediscovered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    let texts = shipped!();
    // `darkmatter/schemas/…` lands below `root/darkmatter`; the root's own
    // `schemas/` gets a flat copy of the catalog and its partials.
    texts.write_below(&root);
    std::fs::create_dir_all(root.join("schemas/partials")).expect("schemas");
    for (path, text) in texts.documents() {
        if let Some(rest) = path.strip_prefix("darkmatter/schemas/") {
            std::fs::write(root.join("schemas").join(rest), text).expect("copy");
        }
    }
    let context = biscuit_file::FileResolutionContext::from_snapshot(&root, None, Default::default())
        .with_repository_root(&root);
    let api = DarkmatterSchemas::new(context.clone())
        .with_darkmatter_baseline_json_schema()
        .expect("baseline")
        .with_trigger_discovery()
        .expect("the root's catalog and partials are not triggers");
    let check = |frontmatter: &str| {
        let doc = root.join("doc.md");
        std::fs::write(&doc, format!("---\n{frontmatter}---\nBody\n")).expect("doc");
        let markdown = Markdown::try_from(doc.as_path())
            .expect("loads")
            .with_source(darkmatter::markdown::compose::ComposeSource::File(doc));
        api.effective_for(&markdown)
    };

    // No `$schema`: the trigger walk contributes nothing; only the baseline applies.
    let effective = check("title: plain\n").expect("prepares").expect("baseline");
    assert!(effective.origins.values().all(|origin| origin.kind == SchemaOriginKind::Baseline));
    assert!(!effective.declares_top_level_property("fm") && !effective.declares_top_level_property("timing"));

    for reference in ["darkmatter.yaml", "./schemas/darkmatter.yaml", "./darkmatter/schemas/darkmatter.yaml"] {
        let Err(error) = check(&format!("$schema: {reference}\n")) else {
            panic!("`{reference}` applied the catalog as a document schema");
        };
        assert!(format!("{error:?}").contains("global catalog"), "{reference}: {error:?}");
    }
    // A catalog directory is not a schema file.
    assert!(check("$schema: ./schemas/\n").is_err());
    // Explicit registration is the one way in.
    let registered = DarkmatterSchemas::new(context)
        .with_baseline_from_file(root.join(DARKMATTER_CATALOG_PATH))
        .expect("an explicitly registered catalog contributes its `fm`");
    let report = registered.validate(&Markdown::from("---\ncontent_policy: []\n---\nBody\n")).expect("validates");
    assert!(!report.valid, "the registered catalog's `fm` types `content_policy`");
}

#[test]
fn an_unloadable_catalog_registered_as_the_baseline_fails_naming_its_field() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut texts = shipped!();
    texts.0[0] = texts.0[0].replace("    env: object", "    env: null #");
    texts.write_below(dir.path());
    let error = DarkmatterSchemas::new(context_at(dir.path()))
        .with_baseline_from_file(dir.path().join(DARKMATTER_CATALOG_PATH))
        .err()
        .expect("the catalog does not load");
    assert!(matches!(&error, SchemaError::Baseline { message, .. } if message.contains("`$schema.env` is null")), "{error:?}");
}

/// The old separately embedded baseline under `docs/schemas/` is gone, and no
/// Darkmatter source or test names or includes it.
#[test]
fn the_old_embedded_baseline_is_deleted_and_never_included() {
    let area = biscuit_test_harness::manifest_dir!().join("..");
    let old = ["docs", "schemas", "darkmatter.yaml"].join("/");
    assert!(!area.join(&old).exists());
    let needle = format!("darkmatter/{old}");
    let include = format!("{old}\")");
    let mut offenders = Vec::new();
    let mut pending: Vec<_> = ["lib/src", "lib/tests", "cli/src", "cli/tests", "dmls/src", "dmls/tests"]
        .iter()
        .map(|dir| area.join(dir))
        .collect();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("readable source tree") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("source");
                let names_old = text.lines().any(|line| {
                    (line.contains(&needle) || line.contains(&include)) && !line.contains("is_none()")
                });
                if names_old {
                    offenders.push(path);
                }
            }
        }
    }
    assert!(offenders.is_empty(), "{offenders:#?}");
}

// ── input robustness matrix ────────────────────────────────────────────────

/// The plan's global-catalog matrix: every (column, row) cell.
const COLUMNS: [&str; 8] = ["kind", "$schema", "fm", "ctx", "current", "env", "err", "timing"];
const ROWS: [&str; 9] =
    ["control", "absent", "null", "wrong_type", "one_wrong", "every_wrong", "empty", "duplicate", "trailing"];

/// Which file a cell edits.
#[derive(Clone, Copy)]
enum File {
    Catalog = 0,
    Fm = 1,
    Error = 4,
    Timing = 5,
}

/// The expected public outcome: the loaded globals, or a failure at `field`.
#[derive(Clone, Copy, Debug)]
enum Expect {
    Loads,
    Missing(&'static str),
    Null(&'static str),
    WrongType(&'static str),
    Empty(&'static str),
    /// The document is not valid YAML; nothing names a field.
    Yaml,
    /// The declaration does not parse or resolve.
    Resolve(&'static str),
}

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` occurs once");
    text.replacen(from, to, 1)
}

fn tail(text: &str, from: &str, to: &str) -> String {
    let start = text.find(from).unwrap_or_else(|| panic!("`{from}` occurs"));
    format!("{}{to}", &text[..start])
}

/// The context type's members, as an inline block under `name`, with every
/// line passed through `member`.
fn context_block(texts: &Texts, name: &str, member: impl Fn(&str) -> String) -> String {
    let partial = &texts.0[3];
    let body = &partial[partial.find("    context:\n").expect("context type") + "    context:\n".len()..];
    let members: String =
        body.lines().filter(|line| !line.trim().is_empty()).map(|line| format!("{}\n", member(line))).collect();
    format!("    {name}:\n{members}")
}

const CTX_LINE: &str = "    ctx: context@./partials/context-variables.yaml -> context captured once at the start of the document's lifecycle\n";
const CURRENT_LINE: &str = "    current: context@./partials/context-variables.yaml -> the same context, materialized lazily when an event reads it\n";

fn check_cell(column: &str, row: &str, file: File, edit: impl Fn(&Texts, &str) -> String, expect: Expect) {
    assert!(COLUMNS.contains(&column) && ROWS.contains(&row), "{column}/{row} is a matrix cell");
    let mut texts = shipped!();
    let index = file as usize;
    let edited = edit(&texts, &texts.0[index]);
    if row != "control" {
        assert_ne!(edited, texts.0[index], "{column}/{row} edits its file");
    }
    texts.0[index] = edited;
    let outcome = texts.load();
    let failure = |error: &CatalogError| (error.field.clone(), format!("{:?}", error.reason));
    match (expect, &outcome) {
        (Expect::Loads, Ok(catalog)) => {
            // Public results: the baseline and every global match the runtime.
            assert_eq!(catalog.baseline(), Some(darkmatter_base_schema()));
            assert_eq!(catalog, GlobalCatalog::darkmatter());
        }
        (Expect::Missing(field), Err(error)) => {
            assert_eq!(error.field, field, "{column}/{row}: {error}");
            assert!(matches!(error.reason, CatalogErrorReason::Missing), "{column}/{row}: {error}");
        }
        (Expect::Null(field), Err(error)) => {
            assert_eq!(error.field, field, "{column}/{row}: {error}");
            assert!(matches!(error.reason, CatalogErrorReason::Null), "{column}/{row}: {error}");
        }
        (Expect::WrongType(field), Err(error)) => {
            assert_eq!(error.field, field, "{column}/{row}: {error}");
            assert!(matches!(error.reason, CatalogErrorReason::WrongType { .. }), "{column}/{row}: {error}");
        }
        (Expect::Empty(field), Err(error)) => {
            assert_eq!(error.field, field, "{column}/{row}: {error}");
            assert!(matches!(error.reason, CatalogErrorReason::Empty), "{column}/{row}: {error}");
        }
        (Expect::Yaml, Err(error)) => {
            assert!(matches!(error.reason, CatalogErrorReason::Yaml(_)), "{column}/{row}: {error}");
        }
        (Expect::Resolve(field), Err(error)) => {
            assert_eq!(error.field, field, "{column}/{row}: {error}");
            assert!(matches!(error.reason, CatalogErrorReason::Resolve(_)), "{column}/{row}: {error}");
        }
        (expect, outcome) => panic!(
            "{column}/{row}: expected {expect:?}, got {:?}",
            outcome.as_ref().map(|_| "a loaded catalog").map_err(failure)
        ),
    }
}

macro_rules! matrix {
    ($($test:ident: $column:literal / $row:literal, $file:ident, $edit:expr => $expect:expr;)+) => {
        $(
            #[test]
            fn $test() {
                check_cell($column, $row, File::$file, $edit, $expect);
            }
        )+

        /// The generated tests are exactly the plan's cells.
        #[test]
        fn the_matrix_tests_cover_every_global_catalog_cell_once() {
            let generated: Vec<(&str, &str)> = vec![$(($column, $row)),+];
            let unique: BTreeSet<_> = generated.iter().copied().collect();
            assert_eq!(unique.len(), generated.len(), "a cell is generated twice");
            let table: BTreeSet<_> =
                COLUMNS.iter().flat_map(|column| ROWS.iter().map(move |row| (*column, *row))).collect();
            assert_eq!(unique, table);
        }
    };
}

matrix! {
    // ── kind ──
    kind_control: "kind" / "control", Catalog, |_, t| t.to_string() => Expect::Loads;
    kind_absent: "kind" / "absent", Catalog, |_, t| replace_once(t, "kind: schema\n", "") => Expect::Missing("kind");
    kind_null: "kind" / "null", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: null\n") => Expect::Null("kind");
    kind_wrong_type: "kind" / "wrong_type", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: 42\n") => Expect::WrongType("kind");
    // N/A for a scalar: a collection substituted there fails.
    kind_one_wrong: "kind" / "one_wrong", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: [schema, 1]\n") => Expect::WrongType("kind");
    kind_every_wrong: "kind" / "every_wrong", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: [1, 2]\n") => Expect::WrongType("kind");
    kind_empty: "kind" / "empty", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: ''\n") => Expect::Empty("kind");
    kind_duplicate: "kind" / "duplicate", Catalog, |_, t| replace_once(t, "kind: schema\n", "kind: schema\nkind: schema\n") => Expect::Yaml;
    kind_trailing: "kind" / "trailing", Catalog, |_, t| format!("{t}---\nkind: schema\n") => Expect::Yaml;
    // ── $schema ──
    schema_control: "$schema" / "control", Catalog, |_, t| t.to_string() => Expect::Loads;
    schema_absent: "$schema" / "absent", Catalog, |_, t| tail(t, "$schema:\n", "") => Expect::Missing("$schema");
    schema_null: "$schema" / "null", Catalog, |_, t| tail(t, "$schema:\n", "$schema: null\n") => Expect::Null("$schema");
    schema_wrong_type: "$schema" / "wrong_type", Catalog, |_, t| tail(t, "$schema:\n", "$schema: ./partials/fm.yaml\n") => Expect::WrongType("$schema");
    schema_one_wrong: "$schema" / "one_wrong", Catalog, |_, t| replace_once(t, "    timing: timing@./partials", "    timing: timing@@./partials") => Expect::Resolve("$schema.timing");
    schema_every_wrong: "$schema" / "every_wrong", Catalog, |_, t| tail(t, "$schema:\n", "$schema:\n    fm: 1\n    ctx: 1\n    current: 1\n    env: 1\n    err: 1\n    timing: 1\n") => Expect::WrongType("$schema.fm");
    schema_empty: "$schema" / "empty", Catalog, |_, t| tail(t, "$schema:\n", "$schema: {}\n") => Expect::Empty("$schema");
    schema_duplicate: "$schema" / "duplicate", Catalog, |_, t| format!("{t}$schema:\n    env: object\n") => Expect::Yaml;
    schema_trailing: "$schema" / "trailing", Catalog, |_, t| format!("{t}  : [\n") => Expect::Yaml;
    // ── fm (the `fm` partial) ──
    fm_control: "fm" / "control", Fm, |_, t| t.to_string() => Expect::Loads;
    fm_absent: "fm" / "absent", Catalog, |_, t| tail(t, "    fm: ", "") + &t[t.find("    ctx: ").unwrap()..] => Expect::Missing("$schema.fm");
    fm_null: "fm" / "null", Catalog, |_, t| replace_once(t, "    fm: fm@./partials/fm.yaml", "    fm: null #") => Expect::Null("$schema.fm");
    fm_wrong_type: "fm" / "wrong_type", Catalog, |_, t| replace_once(t, "    fm: fm@./partials/fm.yaml", "    fm: 42 #") => Expect::WrongType("$schema.fm");
    fm_one_wrong: "fm" / "one_wrong", Fm, |_, t| replace_once(t, "title: \"string -> Human", "title: \"strng -> Human") => Expect::Resolve("$schema.fm");
    fm_every_wrong: "fm" / "every_wrong", Fm, |_, t| tail(t, "    fm:\n", "    fm:\n        title: strng\n        tags: nmber[]\n        draft: bolish\n") => Expect::Resolve("$schema.fm");
    fm_empty: "fm" / "empty", Fm, |_, t| tail(t, "    fm:\n", "    fm: {}\n") => Expect::Empty("$schema.fm");
    fm_duplicate: "fm" / "duplicate", Fm, |_, t| replace_once(t, "        tags: \"string[] -> Freeform document tags.\"\n", "        tags: \"string[] -> Freeform document tags.\"\n        tags: string\n") => Expect::Resolve("$schema.fm");
    fm_trailing: "fm" / "trailing", Fm, |_, t| format!("{t}---\nextra: 1\n") => Expect::Resolve("$schema.fm");
    // ── ctx (its declaration, inlined where a member is edited) ──
    ctx_control: "ctx" / "control", Catalog, |_, t| t.to_string() => Expect::Loads;
    ctx_absent: "ctx" / "absent", Catalog, |_, t| replace_once(t, CTX_LINE, "") => Expect::Missing("$schema.ctx");
    ctx_null: "ctx" / "null", Catalog, |_, t| replace_once(t, CTX_LINE, "    ctx: null\n") => Expect::Null("$schema.ctx");
    ctx_wrong_type: "ctx" / "wrong_type", Catalog, |_, t| replace_once(t, CTX_LINE, "    ctx: true\n") => Expect::WrongType("$schema.ctx");
    ctx_one_wrong: "ctx" / "one_wrong", Catalog, |texts, t| replace_once(t, CTX_LINE, &context_block(texts, "ctx", |line| line.replacen("cwd: \"string(", "cwd: \"strng(", 1))) => Expect::Resolve("$schema.ctx");
    ctx_every_wrong: "ctx" / "every_wrong", Catalog, |texts, t| replace_once(t, CTX_LINE, &context_block(texts, "ctx", |line| format!("{}: strng", line.split(':').next().unwrap()))) => Expect::Resolve("$schema.ctx");
    ctx_empty: "ctx" / "empty", Catalog, |_, t| replace_once(t, CTX_LINE, "    ctx: {}\n") => Expect::Empty("$schema.ctx");
    ctx_duplicate: "ctx" / "duplicate", Catalog, |texts, t| replace_once(t, CTX_LINE, &context_block(texts, "ctx", |line| if line.trim_start().starts_with("cwd:") { format!("{line}\n{line}") } else { line.to_string() })) => Expect::Yaml;
    ctx_trailing: "ctx" / "trailing", Catalog, |_, t| replace_once(t, CTX_LINE, "    ctx: [context@./partials/context-variables.yaml\n") => Expect::Yaml;
    // ── current (must equal ctx) ──
    current_control: "current" / "control", Catalog, |_, t| t.to_string() => Expect::Loads;
    current_absent: "current" / "absent", Catalog, |_, t| replace_once(t, CURRENT_LINE, "") => Expect::Missing("$schema.current");
    current_null: "current" / "null", Catalog, |_, t| replace_once(t, CURRENT_LINE, "    current: null\n") => Expect::Null("$schema.current");
    current_wrong_type: "current" / "wrong_type", Catalog, |_, t| replace_once(t, CURRENT_LINE, "    current: 7\n") => Expect::WrongType("$schema.current");
    current_one_wrong: "current" / "one_wrong", Catalog, |texts, t| replace_once(t, CURRENT_LINE, &context_block(texts, "current", |line| line.replacen("cwd: \"string(", "cwd: \"strng(", 1))) => Expect::Resolve("$schema.current");
    current_every_wrong: "current" / "every_wrong", Catalog, |texts, t| replace_once(t, CURRENT_LINE, &context_block(texts, "current", |line| format!("{}: strng", line.split(':').next().unwrap()))) => Expect::Resolve("$schema.current");
    current_empty: "current" / "empty", Catalog, |_, t| replace_once(t, CURRENT_LINE, "    current: \"\"\n") => Expect::Empty("$schema.current");
    current_duplicate: "current" / "duplicate", Catalog, |_, t| replace_once(t, CURRENT_LINE, &format!("{CURRENT_LINE}{CURRENT_LINE}")) => Expect::Yaml;
    current_trailing: "current" / "trailing", Catalog, |_, t| replace_once(t, CURRENT_LINE, "    current: [context@./partials/context-variables.yaml\n") => Expect::Yaml;
    // ── env ──
    env_control: "env" / "control", Catalog, |_, t| t.to_string() => Expect::Loads;
    env_absent: "env" / "absent", Catalog, |_, t| replace_once(t, "    env: object -> environment variables captured once at the start of the document's lifecycle\n", "") => Expect::Missing("$schema.env");
    env_null: "env" / "null", Catalog, |_, t| replace_once(t, "    env: object", "    env: null #") => Expect::Null("$schema.env");
    env_wrong_type: "env" / "wrong_type", Catalog, |_, t| replace_once(t, "    env: object", "    env: string") => Expect::WrongType("$schema.env");
    env_one_wrong: "env" / "one_wrong", Catalog, |_, t| replace_once(t, "    env: object", "    env: \"{ HOME: strng, PATH: string }\" #") => Expect::Resolve("$schema.env");
    env_every_wrong: "env" / "every_wrong", Catalog, |_, t| replace_once(t, "    env: object", "    env: \"{ HOME: strng, PATH: nmber }\" #") => Expect::Resolve("$schema.env");
    env_empty: "env" / "empty", Catalog, |_, t| replace_once(t, "    env: object", "    env: \"\" #") => Expect::Empty("$schema.env");
    env_duplicate: "env" / "duplicate", Catalog, |_, t| replace_once(t, "    env: object", "    env: object\n    env: object #") => Expect::Yaml;
    env_trailing: "env" / "trailing", Catalog, |_, t| replace_once(t, "    env: object", "    env: [object") => Expect::Yaml;
    // ── err (the `err` partial) ──
    err_control: "err" / "control", Error, |_, t| t.to_string() => Expect::Loads;
    err_absent: "err" / "absent", Catalog, |_, t| replace_once(t, "    err: err@./partials/error.yaml -> the active failure\n", "") => Expect::Missing("$schema.err");
    err_null: "err" / "null", Catalog, |_, t| replace_once(t, "    err: err@./partials/error.yaml", "    err: null #") => Expect::Null("$schema.err");
    err_wrong_type: "err" / "wrong_type", Catalog, |_, t| replace_once(t, "    err: err@./partials/error.yaml", "    err: string #") => Expect::WrongType("$schema.err");
    err_one_wrong: "err" / "one_wrong", Error, |_, t| replace_once(t, "msg: \"string(generated; required)", "msg: \"strng(generated; required)") => Expect::Resolve("$schema.err");
    err_every_wrong: "err" / "every_wrong", Error, |_, t| tail(t, "    err:\n", "    err:\n        kind: strng\n        msg: bolean\n") => Expect::Resolve("$schema.err");
    err_empty: "err" / "empty", Error, |_, t| tail(t, "    err:\n", "    err: {}\n") => Expect::Empty("$schema.err");
    err_duplicate: "err" / "duplicate", Error, |_, t| format!("{t}        msg: string\n") => Expect::Resolve("$schema.err");
    err_trailing: "err" / "trailing", Error, |_, t| format!("{t}---\nextra: 1\n") => Expect::Resolve("$schema.err");
    // ── timing (the `timing` partial) ──
    timing_control: "timing" / "control", Timing, |_, t| t.to_string() => Expect::Loads;
    timing_absent: "timing" / "absent", Catalog, |_, t| replace_once(t, "    timing: timing@./partials/timing.yaml -> observed durations\n", "") => Expect::Missing("$schema.timing");
    timing_null: "timing" / "null", Catalog, |_, t| replace_once(t, "    timing: timing@./partials/timing.yaml", "    timing: null #") => Expect::Null("$schema.timing");
    timing_wrong_type: "timing" / "wrong_type", Catalog, |_, t| replace_once(t, "    timing: timing@./partials/timing.yaml", "    timing: number #") => Expect::WrongType("$schema.timing");
    timing_one_wrong: "timing" / "one_wrong", Timing, |_, t| replace_once(t, "step_ms: \"number(integer; min(0); generated)", "step_ms: \"number(integer; min(0); generatd)") => Expect::Resolve("$schema.timing");
    timing_every_wrong: "timing" / "every_wrong", Timing, |_, t| tail(t, "    timing:\n", "    timing:\n        document_ms: nmber\n        total_ms: nmber\n        step_ms: nmber\n") => Expect::Resolve("$schema.timing");
    timing_empty: "timing" / "empty", Timing, |_, t| tail(t, "    timing:\n", "    timing: {}\n") => Expect::Empty("$schema.timing");
    timing_duplicate: "timing" / "duplicate", Timing, |_, t| format!("{t}        step_ms: number\n") => Expect::Resolve("$schema.timing");
    timing_trailing: "timing" / "trailing", Timing, |_, t| format!("{t}  : [\n") => Expect::Resolve("$schema.timing");
}

/// `current` declared with a valid shape that differs from `ctx` fails on the
/// shared-shape rule, not on its own definition.
#[test]
fn a_current_shape_differing_from_ctx_is_a_mismatch() {
    let mut texts = shipped!();
    let block = context_block(&texts, "current", |line| line.replacen("cwd: \"string(", "cwd: \"number(", 1));
    texts.0[0] = replace_once(&texts.0[0], CURRENT_LINE, &block);
    let error = texts.load().expect_err("current differs from ctx");
    assert_eq!(error.field, "$schema.current");
    assert!(matches!(&error.reason, CatalogErrorReason::Mismatch(target) if target == "ctx"), "{error}");
}

#[test]
fn a_kind_other_than_schema_is_refused() {
    let mut texts = shipped!();
    texts.0[0] = replace_once(&texts.0[0], "kind: schema\n", "kind: schema-trigger\n");
    let error = texts.load().expect_err("not a schema catalog");
    assert_eq!(error.field, "kind");
    assert!(matches!(error.reason, CatalogErrorReason::WrongKind(_)), "{error}");
}

#[test]
fn an_unknown_global_or_header_field_is_refused() {
    let mut texts = shipped!();
    texts.0[0] = format!("{}    tracking: object\n", texts.0[0]);
    let error = texts.load().expect_err("tracking is not a Darkmatter global");
    assert_eq!(error.field, "$schema.tracking");
    assert!(matches!(error.reason, CatalogErrorReason::Unknown));
    let mut texts = shipped!();
    texts.0[0] = format!("name: globals\n{}", texts.0[0]);
    assert_eq!(texts.load().expect_err("name is not a header field").field, "name");
}

// ── shape versus availability ──────────────────────────────────────────────

/// The catalog declares `err`'s shape, but `md`'s `initialize` view declares
/// it unavailable: a read fails with the view's reason and never falls back
/// to a document `err` property, which `fm.err` still reads.
#[test]
fn an_unavailable_err_fails_with_the_views_reason_not_fm_err() {
    use darkmatter::lifecycle::registry::EventRegistry;
    use darkmatter::markdown::MarkdownError;
    use darkmatter::markdown::compose::EffectiveStateBuilder;
    use darkmatter::markdown::compose::expression::{
        AuthoredMode, Availability, BindingError, EvaluationSession, ExpressionError, RuntimeBinding,
        evaluate_prepared, prepare_value,
    };

    assert!(GlobalCatalog::darkmatter().global("err").is_some());
    let registry = EventRegistry::md();
    let initialize = registry.id("initialize").expect("md registers initialize");
    let view = registry.globals(initialize).clone();
    let Some(Availability::Unavailable(reason)) = registry.err_availability(initialize).cloned() else {
        panic!("`err` is unavailable in initialize");
    };
    let frontmatter = serde_json::json!({ "err": { "msg": "from the document" } });
    let document = EffectiveStateBuilder::new()
        .with_frontmatter(serde_json::from_value(frontmatter).expect("map"))
        .build()
        .expect("state");
    let runtime = view.globals().map(|global| {
        let binding = match global.availability() {
            Availability::Unavailable(reason) => RuntimeBinding::unavailable(reason.clone()),
            _ => RuntimeBinding::eager(serde_json::json!({})),
        };
        (global.root().to_string(), binding)
    });
    let session = EvaluationSession::associate(view.clone(), &document, runtime.collect::<Vec<_>>()).expect("session");
    let render = |text: &str| {
        let prepared = prepare_value(&serde_json::json!(text), AuthoredMode::InterpolatedValue).expect("prepares");
        evaluate_prepared(&prepared, &session)
    };
    let error = render("{{ err.msg }}").expect_err("`err` is unavailable");
    let MarkdownError::Interpolation { cause, .. } = &error else { panic!("{error:?}") };
    let ExpressionError::Binding(binding) = cause.as_ref() else { panic!("{cause:?}") };
    let BindingError::Unavailable(read) = binding.as_ref() else { panic!("{binding:?}") };
    assert_eq!((read.root.as_str(), read.reason.code()), ("err", reason.code()));
    assert_eq!(render("{{ fm.err.msg }}").expect("document data"), serde_json::json!("from the document"));
}
