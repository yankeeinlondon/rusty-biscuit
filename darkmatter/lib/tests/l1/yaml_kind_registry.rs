//! The registry of YAML document kinds, read through its public results: the
//! claims a document context's registry holds, and the outcome of validating
//! a YAML document through `DarkmatterSchemas::validate_yaml_document`.
//!
//! Every context is built from a `RequestSnapshot` anchored in the test's
//! own temporary directory, with the fixture `HOME`; nothing is read from the
//! process environment or the checkout, except the shipped definitions'
//! compiled-in text and the repository copies the source-path identity test
//! names.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::yaml_documents::{Claim, DefinitionSource, shipped_kind_definition_paths};
use darkmatter::markdown::schemas::{DarkmatterSchemas, KindRegistry, RegistryScope, Route, YamlOutcome};
use darkmatter::markdown::schemas::triggers::discovery::SchemaRootSource;
use simplified_schema::resolve::buffers::{self, SourceBuffers};
use simplified_schema::{KindSelectionCause, SchemaError, ValidationProblemCode};

pub(crate) fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

/// `{dir}/repo` is a Git repository (a bare `.git` folder, no `git` process)
/// holding package `area/pkg` in package area `area`; `{dir}/home` is the
/// fixture `HOME`; `{dir}/dm-defs` lies outside both and is the
/// `SCHEMAS_DIR` folder when a test sets one.
pub(crate) struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = Self { dir };
        let repo = fixture.repo();
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
        std::fs::create_dir_all(repo.join(".git/refs")).unwrap();
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            &repo.join("area/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&repo.join("area/pkg/src/lib.rs"), "");
        std::fs::create_dir_all(fixture.home()).unwrap();
        fixture
    }

    pub(crate) fn root(&self) -> PathBuf {
        canonicalize_simplified(self.dir.path()).unwrap_or_else(|_| self.dir.path().to_path_buf())
    }

    pub(crate) fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    pub(crate) fn package(&self) -> PathBuf {
        self.repo().join("area/pkg")
    }

    pub(crate) fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    pub(crate) fn defs(&self) -> PathBuf {
        self.root().join("dm-defs")
    }

    /// The context of a document at `document`, with the fixture home and
    /// `env` as the snapshot's environment.
    pub(crate) fn context(&self, document: &Path, env: &[(&str, &str)]) -> FileResolutionContext {
        let env: HashMap<String, String> = env.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
        let folder = document.parent().expect("document folder");
        std::fs::create_dir_all(folder).expect("document folder");
        let snapshot = RequestSnapshot::new(folder).with_home(Some(self.home())).with_env(env);
        build_resolution_context(&snapshot).expect("fixture context")
    }

    /// Validates `text` as the YAML document `document` (written first).
    pub(crate) fn validate(&self, document: &Path, text: &str) -> Result<YamlOutcome, SchemaError> {
        write(document, text);
        let context = self.context(document, &[]);
        DarkmatterSchemas::new(context).validate_yaml_document(text, document)
    }
}

pub(crate) const ROSTER_DEFINITION: &str = "\
kind: yaml-schema
types:
    platform:
        name: string(required)
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
    platforms: platform[](required)
    $constraints: { closed: true }
";

pub(crate) const ROSTER_DOCUMENT: &str = "kind: roster\nroster_version: 1\nplatforms:\n    - name: slack\n";

fn registry(context: &FileResolutionContext) -> KindRegistry {
    KindRegistry::from_filesystem(context, RegistryScope::WithRoots)
}

fn healthy_source(registry: &KindRegistry, kind: &str) -> DefinitionSource {
    match registry.claim(kind) {
        Some(Claim::Healthy(definition)) => definition.source().clone(),
        other => panic!("`{kind}` is not healthy: {other:?}"),
    }
}

fn selection_cause(result: Result<YamlOutcome, SchemaError>) -> (String, KindSelectionCause) {
    match result {
        Err(SchemaError::KindSelection { kind, cause }) => (kind, cause),
        other => panic!("expected a kind-selection error, got {other:?}"),
    }
}

fn validated(result: Result<YamlOutcome, SchemaError>) -> darkmatter::markdown::schemas::YamlValidation {
    match result {
        Ok(YamlOutcome::Validated(validation)) => validation,
        other => panic!("expected a validated document, got {other:?}"),
    }
}

// ── Shipped definitions ────────────────────────────────────────────────────

#[test]
fn the_shipped_registry_claims_schema_yaml_schema_and_schema_trigger() {
    let registry = KindRegistry::shipped();
    let claimed: Vec<&str> = registry.claims().map(|(kind, _)| kind).collect();
    assert_eq!(claimed, ["schema", "schema-trigger", "yaml-schema"]);
    for (kind, path) in [
        ("schema", "simplified-schema/schemas/schema.yaml"),
        ("yaml-schema", "simplified-schema/schemas/yaml-schema.yaml"),
        ("schema-trigger", "darkmatter/schemas/schema-trigger.yaml"),
    ] {
        assert_eq!(healthy_source(&registry, kind), DefinitionSource::Shipped(path));
    }
    assert!(registry.incomplete().is_empty() && registry.root_failures().is_empty());
    assert_eq!(
        shipped_kind_definition_paths(),
        [
            PathBuf::from("simplified-schema/schemas/schema.yaml"),
            PathBuf::from("simplified-schema/schemas/yaml-schema.yaml"),
            PathBuf::from("darkmatter/schemas/schema-trigger.yaml"),
        ]
    );
}

/// Each shipped file, as compiled in, validates against the shipped
/// registry: all three through `yaml-schema.yaml`, which therefore validates
/// itself.
#[test]
fn each_shipped_definition_validates_against_the_shipped_yaml_schema_definition() {
    let schemas = DarkmatterSchemas::new(Fixture::new().context(&std::env::temp_dir().join("x.yaml"), &[]))
        .with_kind_registry(KindRegistry::shipped());
    for (path, text) in [
        ("simplified-schema/schemas/schema.yaml", include_str!("../../../../simplified-schema/schemas/schema.yaml")),
        (
            "simplified-schema/schemas/yaml-schema.yaml",
            include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml"),
        ),
        ("darkmatter/schemas/schema-trigger.yaml", include_str!("../../../schemas/schema-trigger.yaml")),
    ] {
        let validation = validated(schemas.validate_yaml_document(text, Path::new(path)));
        assert_eq!(validation.route, Route::Kind("yaml-schema".into()), "{path}");
        assert_eq!(validation.definition, "built-in simplified-schema/schemas/yaml-schema.yaml");
        assert!(validation.report.valid, "{path}: {:?}", validation.report.problems);
    }
}

#[test]
fn a_registry_built_outside_a_repository_holds_the_shipped_kinds_only() {
    let fixture = Fixture::new();
    let outside = fixture.root().join("loose/doc.yaml");
    write(&fixture.root().join("loose/schemas/roster.yaml"), ROSTER_DEFINITION);
    let context = fixture.context(&outside, &[]);
    assert_eq!(RegistryScope::for_context(&context), RegistryScope::ShippedOnly);
    let registry = KindRegistry::from_filesystem(&context, RegistryScope::for_context(&context));
    assert!(registry.claim("roster").is_none());
    assert!(registry.claim("schema").is_some());
    // Unclaimed and the registry is complete: not validated.
    assert!(matches!(fixture.validate(&outside, ROSTER_DOCUMENT), Ok(YamlOutcome::NoSchema)));
}

/// A bare root `$schema` searches the schema roots exactly where `md`
/// searches them for Markdown frontmatter: inside a repository. Outside one
/// the bare name is a file next to the document, and an explicit path still
/// names the contract.
#[test]
fn a_bare_root_schema_searches_the_schema_roots_only_inside_a_repository() {
    let fixture = Fixture::new();
    let contract = fixture.defs().join("contract.yaml");
    write(&contract, "kind: schema\n$schema:\n    owner: string(required)\n");
    let schemas_dir = fixture.defs().display().to_string();
    let env = [("SCHEMAS_DIR", schemas_dir.as_str())];
    let validate = |document: &Path, text: &str| {
        write(document, text);
        DarkmatterSchemas::new(fixture.context(document, &env)).validate_yaml_document(text, document)
    };
    let bare = "$schema: contract.yaml\nowner: ops\n";

    let inside = validated(validate(&fixture.package().join("docs/service.yaml"), bare));
    assert!(inside.report.valid, "{:?}", inside.report.problems);

    let outside = fixture.root().join("loose/service.yaml");
    let error = validate(&outside, bare).expect_err("no schema root is searched outside a repository");
    assert!(error.to_string().contains("contract.yaml"), "{error}");

    let explicit = format!("$schema: {}\nowner: ops\n", contract.display());
    let outside_explicit = validated(validate(&outside, &explicit));
    assert!(outside_explicit.report.valid, "{:?}", outside_explicit.report.problems);
}

// ── Discovery in the five roots ────────────────────────────────────────────

#[test]
fn a_definition_in_each_of_the_five_roots_claims_its_kind() {
    let fixture = Fixture::new();
    let document = fixture.package().join("docs/roster.yaml");
    for (label, folder, env) in [
        ("package", fixture.package().join("schemas"), None),
        ("package area", fixture.repo().join("area/schemas"), None),
        ("tree", fixture.repo().join("schemas"), None),
        ("home", fixture.home().join("schemas"), None),
        ("SCHEMAS_DIR", fixture.defs(), Some(fixture.defs())),
    ] {
        let definition = folder.join("roster.yaml");
        write(&definition, ROSTER_DEFINITION);
        let env: Vec<(&str, String)> = env.map(|dir| ("SCHEMAS_DIR", dir.display().to_string())).into_iter().collect();
        let env: Vec<(&str, &str)> = env.iter().map(|(key, value)| (*key, value.as_str())).collect();
        let context = fixture.context(&document, &env);
        let registry = registry(&context);
        assert_eq!(
            healthy_source(&registry, "roster"),
            DefinitionSource::File(definition.clone()),
            "{label} root"
        );
        write(&document, ROSTER_DOCUMENT);
        let validation = validated(
            DarkmatterSchemas::new(context).validate_yaml_document(ROSTER_DOCUMENT, &document),
        );
        assert_eq!(validation.route, Route::Kind("roster".into()), "{label}");
        assert!(validation.report.valid, "{label}: {:?}", validation.report.problems);
        std::fs::remove_file(&definition).unwrap();
    }
}

#[test]
fn two_definitions_claiming_one_kind_are_a_duplicate_naming_both_files_in_any_root_order() {
    let fixture = Fixture::new();
    let document = fixture.package().join("roster.yaml");
    let package = fixture.package().join("schemas/a.yaml");
    let tree = fixture.repo().join("schemas/b.yaml");
    write(&package, ROSTER_DEFINITION);
    write(&tree, ROSTER_DEFINITION);
    let mut expected = vec![package.clone(), tree.clone()];
    expected.sort();
    let (kind, cause) = selection_cause(fixture.validate(&document, ROSTER_DOCUMENT));
    assert_eq!(kind, "roster");
    assert_eq!(cause, KindSelectionCause::Duplicate { files: expected.clone() });

    // The same two files the other way round across the roots: the same
    // error, sorted by path rather than by search order.
    std::fs::remove_file(&package).unwrap();
    std::fs::remove_file(&tree).unwrap();
    let package = fixture.package().join("schemas/b.yaml");
    let tree = fixture.repo().join("schemas/a.yaml");
    write(&package, ROSTER_DEFINITION);
    write(&tree, ROSTER_DEFINITION);
    let mut expected = vec![package, tree];
    expected.sort();
    let (_, cause) = selection_cause(fixture.validate(&document, ROSTER_DOCUMENT));
    assert_eq!(cause, KindSelectionCause::Duplicate { files: expected });
}

#[test]
fn a_same_named_file_in_a_later_root_is_still_a_candidate() {
    let fixture = Fixture::new();
    let document = fixture.package().join("roster.yaml");
    write(&fixture.package().join("schemas/defs.yaml"), ROSTER_DEFINITION);
    write(
        &fixture.repo().join("schemas/defs.yaml"),
        &ROSTER_DEFINITION.replace("literal(roster; required)", "literal(inventory; required)"),
    );
    let registry = registry(&fixture.context(&document, &[]));
    assert!(matches!(registry.claim("roster"), Some(Claim::Healthy(_))));
    assert!(matches!(registry.claim("inventory"), Some(Claim::Healthy(_))));
}

#[test]
fn a_repository_file_claiming_a_shipped_kind_is_a_duplicate() {
    let fixture = Fixture::new();
    let document = fixture.package().join("trigger.yaml");
    let mine = fixture.repo().join("schemas/my-trigger.yaml");
    write(
        &mine,
        "kind: yaml-schema\n$schema:\n    kind: literal(schema-trigger; required)\n    match: unknown(required)\n",
    );
    let text = "kind: schema-trigger\nmatch:\n    title: string(required)\n";
    let (kind, cause) = selection_cause(fixture.validate(&document, text));
    assert_eq!(kind, "schema-trigger");
    let mut files = vec![PathBuf::from("darkmatter/schemas/schema-trigger.yaml"), mine];
    files.sort();
    assert_eq!(cause, KindSelectionCause::Duplicate { files });
}

#[test]
fn kinds_compare_exactly_including_case() {
    let fixture = Fixture::new();
    let document = fixture.package().join("roster.yaml");
    write(&fixture.repo().join("schemas/roster.yaml"), ROSTER_DEFINITION);
    let upper = ROSTER_DOCUMENT.replace("kind: roster", "kind: Roster");
    assert!(matches!(fixture.validate(&document, &upper), Ok(YamlOutcome::NoSchema)));
    assert!(validated(fixture.validate(&document, ROSTER_DOCUMENT)).report.valid);
}

// ── Source-path identity ───────────────────────────────────────────────────

/// A checkout copy at a shipped definition's repository-relative path that
/// claims the same kind replaces the compiled-in copy; editing it changes
/// validation with no Rust change.
#[test]
fn a_modified_copy_at_the_shipped_path_replaces_the_built_in_definition() {
    let fixture = Fixture::new();
    let copy = fixture.repo().join("darkmatter/schemas/schema-trigger.yaml");
    // In this repository `darkmatter/schemas/` is a package-area root; here
    // `SCHEMAS_DIR` names it, which puts the copy in a root the same way.
    let roots = fixture.repo().join("darkmatter/schemas").display().to_string();
    let env = [("SCHEMAS_DIR", roots.as_str())];
    let document = fixture.repo().join("docs/trigger.yaml");
    // A matcher-only trigger: no `$schema`.
    let trigger = "kind: schema-trigger\nmatch:\n    title: string(required)\n";
    let validate = |text: &str| {
        write(&document, text);
        DarkmatterSchemas::new(fixture.context(&document, &env)).validate_yaml_document(text, &document)
    };

    // Unedited copy: `$schema` is optional, so the trigger passes.
    write(&copy, include_str!("../../../schemas/schema-trigger.yaml"));
    let registry = registry(&fixture.context(&document, &env));
    assert_eq!(healthy_source(&registry, "schema-trigger"), DefinitionSource::File(copy.clone()));
    let validation = validated(validate(trigger));
    assert_eq!(PathBuf::from(&validation.definition), copy, "named by the copy's path");
    assert!(validation.report.valid, "{:?}", validation.report.problems);

    // The copy edited to require `$schema`: the same trigger now fails, at
    // its root mapping, with no Rust change.
    let edited = include_str!("../../../schemas/schema-trigger.yaml").replace("schema(non-null)", "schema(required; non-null)");
    assert_ne!(edited, include_str!("../../../schemas/schema-trigger.yaml"), "the edit applies");
    write(&copy, &edited);
    let validation = validated(validate(trigger));
    assert!(!validation.report.valid);
    assert_eq!(validation.report.problems[0].code, ValidationProblemCode::MissingRequired);
    assert_eq!(validation.report.problems[0].property.as_deref(), Some("$schema"));
}

/// The editor's view of the schema roots: only the unsaved buffers directly
/// inside each root, spelled under the root as the registry names it and
/// read from the buffers snapshot.
struct UnsavedRoots<'a>(&'a SourceBuffers);

impl SchemaRootSource for UnsavedRoots<'_> {
    fn root_files(&mut self, root: &Path) -> Result<Vec<PathBuf>, SchemaError> {
        let folder = SourceBuffers::key(root);
        let inside = self.0.paths().filter(|path| path.parent() == Some(folder.as_path()));
        Ok(inside.filter_map(|path| path.file_name()).map(|name| root.join(name)).collect())
    }

    fn root_file_text(&mut self, path: &Path) -> Result<Arc<str>, SchemaError> {
        Ok(Arc::clone(self.0.get(path).expect("only buffers are listed")))
    }
}

/// A copy at a shipped definition's repository-relative path that exists only
/// in the editor still replaces the compiled-in copy. The document and its
/// roots are spelled through a symlinked alias of the checkout (as
/// `/var/…` is of `/private/var/…` on macOS), so the identity is decided on
/// the canonical spelling of the copy's nearest existing folder rather than
/// of the file, which has none.
#[test]
fn an_unsaved_copy_at_the_shipped_path_replaces_the_built_in_definition() {
    let fixture = Fixture::new();
    #[cfg(unix)]
    let checkout = {
        let alias = fixture.root().join("alias");
        std::os::unix::fs::symlink(fixture.repo(), &alias).expect("checkout alias");
        alias
    };
    #[cfg(not(unix))]
    let checkout = fixture.repo();
    let folder = checkout.join("darkmatter/schemas");
    std::fs::create_dir_all(&folder).unwrap();
    let copy = folder.join("schema-trigger.yaml");
    assert!(!copy.exists(), "the copy exists only in the editor");
    let edited = include_str!("../../../schemas/schema-trigger.yaml").replace("schema(non-null)", "schema(required; non-null)");
    let mut buffers = SourceBuffers::default();
    buffers.insert(&copy, Arc::from(edited.as_str()));

    let roots = folder.display().to_string();
    let document = checkout.join("docs/trigger.yaml");
    let context = fixture.context(&document, &[("SCHEMAS_DIR", roots.as_str())]);
    let registry = buffers::with(&buffers, || {
        KindRegistry::build(&context, &mut UnsavedRoots(&buffers), RegistryScope::WithRoots)
    });
    match healthy_source(&registry, "schema-trigger") {
        DefinitionSource::File(file) => assert_eq!(SourceBuffers::key(&file), SourceBuffers::key(&copy)),
        other => panic!("the unsaved copy replaces the shipped definition: {other:?}"),
    }

    // The unsaved edit requires `$schema`, so a matcher-only trigger fails.
    let trigger = "kind: schema-trigger\nmatch:\n    title: string(required)\n";
    write(&document, trigger);
    let validation = validated(
        DarkmatterSchemas::new(context).with_kind_registry(Arc::new(registry)).validate_yaml_document(trigger, &document),
    );
    assert!(!validation.report.valid);
    assert_eq!(validation.report.problems[0].code, ValidationProblemCode::MissingRequired);
    assert_eq!(validation.report.problems[0].property.as_deref(), Some("$schema"));
}

#[test]
fn a_copy_at_the_shipped_path_claiming_another_kind_is_an_ordinary_candidate() {
    let fixture = Fixture::new();
    let copy = fixture.repo().join("darkmatter/schemas/schema-trigger.yaml");
    write(&copy, ROSTER_DEFINITION);
    let roots = fixture.repo().join("darkmatter/schemas").display().to_string();
    let registry = registry(&fixture.context(&fixture.repo().join("docs/a.yaml"), &[("SCHEMAS_DIR", roots.as_str())]));
    assert_eq!(healthy_source(&registry, "roster"), DefinitionSource::File(copy));
    assert_eq!(
        healthy_source(&registry, "schema-trigger"),
        DefinitionSource::Shipped("darkmatter/schemas/schema-trigger.yaml")
    );
}

#[test]
fn a_copy_with_the_shipped_file_name_elsewhere_is_a_duplicate() {
    let fixture = Fixture::new();
    let document = fixture.package().join("a.yaml");
    let elsewhere = fixture.home().join("schemas/schema.yaml");
    write(&elsewhere, include_str!("../../../../simplified-schema/schemas/schema.yaml"));
    let (_, cause) = selection_cause(fixture.validate(&document, "kind: schema\ntypes:\n    a: string\n"));
    let mut files = vec![elsewhere, PathBuf::from("simplified-schema/schemas/schema.yaml")];
    files.sort();
    assert_eq!(cause, KindSelectionCause::Duplicate { files });
}

// ── Failed definitions (reserved and incomplete) ───────────────────────────

#[test]
fn a_definition_that_fails_to_import_reserves_its_inline_claim() {
    let fixture = Fixture::new();
    let document = fixture.package().join("roster.yaml");
    let broken = fixture.repo().join("schemas/roster.yaml");
    let text = ROSTER_DEFINITION.replace("    platform:\n        name: string(required)\n", "    platform: platform@./absent.yaml\n");
    assert_ne!(text, ROSTER_DEFINITION);
    write(&broken, &text);
    let registry = registry(&fixture.context(&document, &[]));
    assert!(matches!(registry.claim("roster"), Some(Claim::Reserved { file, .. }) if *file == broken));
    let (kind, cause) = selection_cause(fixture.validate(&document, ROSTER_DOCUMENT));
    assert_eq!(kind, "roster");
    let KindSelectionCause::Reserved { file, defect, code } = cause else { panic!("reserved: {cause:?}") };
    assert_eq!(file, broken);
    assert_eq!(code, None, "only a slot defect carries a code");
    assert!(defect.contains("absent.yaml"), "{defect}");

    // Other documents are unaffected: a shipped kind, and no kind.
    let schema = "kind: schema\ntypes:\n    a: string\n";
    assert!(validated(fixture.validate(&fixture.package().join("s.yaml"), schema)).report.valid);
    assert!(matches!(fixture.validate(&fixture.package().join("n.yaml"), "a: 1\n"), Ok(YamlOutcome::NoSchema)));

    // The defect itself is reported once, when the definition is validated.
    let own = validated(fixture.validate(&broken, &std::fs::read_to_string(&broken).unwrap()));
    assert_eq!(own.report.problems.len(), 1, "{:?}", own.report.problems);
    assert_eq!(own.report.problems[0].code, ValidationProblemCode::KindCheck);
}

/// A kind definition never holds `{{ … }}` slots: its claim is broken with
/// the slot code, whether the slot is written in it or reached through an
/// import, and the same definition without the slot is healthy.
#[test]
fn a_kind_definition_holding_a_slot_is_a_broken_claim_with_the_slot_code() {
    use darkmatter::markdown::schemas::slots::codes::{SLOT_NOT_ALLOWED_IN_KIND_CODE, slot_prohibition_code};
    let slotted = "    platform:\n        name: enum({{ load_data(\"./names.yaml\") }}; required)\n";
    let inline = ROSTER_DEFINITION.replace("    platform:\n        name: string(required)\n", slotted);
    let imported = ROSTER_DEFINITION.replace("    platform:\n        name: string(required)\n", "    platform: platform@./types.yaml\n");
    assert_ne!(inline, ROSTER_DEFINITION);
    assert_ne!(imported, ROSTER_DEFINITION);
    let types = format!("kind: schema\ntypes:\n{slotted}");
    for (case, text) in [("control", ROSTER_DEFINITION.to_string()), ("inline", inline), ("import", imported)] {
        let fixture = Fixture::new();
        let document = fixture.package().join("roster.yaml");
        let definition = fixture.repo().join("schemas/roster.yaml");
        write(&definition, &text);
        write(&fixture.repo().join("schemas/types.yaml"), &types);
        write(&fixture.repo().join("schemas/names.yaml"), "- slack\n");
        let result = fixture.validate(&document, ROSTER_DOCUMENT);
        if case == "control" {
            assert!(validated(result).report.valid, "{case}");
            continue;
        }
        let error = result.expect_err(case);
        assert_eq!(slot_prohibition_code(&error), Some(SLOT_NOT_ALLOWED_IN_KIND_CODE), "{case}: {error}");
        let (kind, cause) = selection_cause(Err(error));
        assert_eq!(kind, "roster", "{case}");
        let KindSelectionCause::Reserved { file, defect, code } = cause else { panic!("{case}: reserved: {cause:?}") };
        assert_eq!(file, definition, "{case}");
        assert_eq!(code, Some(SLOT_NOT_ALLOWED_IN_KIND_CODE), "{case}");
        assert!(defect.contains("a kind definition cannot hold `{{ … }}` slots"), "{case}: {defect}");
        assert!(defect.contains("load_data"), "{case}: {defect}");
        // Nothing the slot names was read: the slot was never evaluated.
        // Other documents are unaffected.
        assert!(matches!(fixture.validate(&fixture.package().join("n.yaml"), "a: 1\n"), Ok(YamlOutcome::NoSchema)));
        // The definition's own check names the prohibition.
        let own = validated(fixture.validate(&definition, &text));
        assert_eq!(own.report.problems.len(), 1, "{case}: {:?}", own.report.problems);
        assert_eq!(own.report.problems[0].code, ValidationProblemCode::KindCheck, "{case}");
        assert!(own.report.problems[0].message.contains("a kind definition cannot hold"), "{case}: {:?}", own.report.problems);
    }
}

#[test]
fn malformed_yaml_that_claims_yaml_schema_makes_unclaimed_kinds_unavailable() {
    let fixture = Fixture::new();
    let broken = fixture.repo().join("schemas/broken.yaml");
    write(&broken, "kind: yaml-schema\n$schema: [unclosed\n");
    let registry = registry(&fixture.context(&fixture.package().join("a.yaml"), &[]));
    assert_eq!(registry.incomplete().len(), 1);
    assert_eq!(registry.incomplete()[0].file, broken);

    let (kind, cause) = selection_cause(fixture.validate(&fixture.package().join("inv.yaml"), "kind: inventory\n"));
    assert_eq!(kind, "inventory");
    assert!(matches!(cause, KindSelectionCause::Unavailable { file, .. } if file == broken));
    // A claimed kind and a document with no kind are unaffected.
    assert!(validated(fixture.validate(&fixture.package().join("s.yaml"), "kind: schema\ntypes:\n    a: string\n"))
        .report
        .valid);
    assert!(matches!(fixture.validate(&fixture.package().join("n.yaml"), "a: 1\n"), Ok(YamlOutcome::NoSchema)));
}

#[test]
fn an_unrelated_malformed_file_in_a_root_leaves_the_registry_complete() {
    let fixture = Fixture::new();
    write(&fixture.repo().join("schemas/notes.yaml"), "title: [unclosed\n");
    let registry = registry(&fixture.context(&fixture.package().join("a.yaml"), &[]));
    assert!(registry.incomplete().is_empty());
    assert!(matches!(fixture.validate(&fixture.package().join("inv.yaml"), "kind: inventory\n"), Ok(YamlOutcome::NoSchema)));
}

#[cfg(unix)]
#[test]
fn an_unreadable_root_is_an_error_never_an_empty_registry() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }
    let fixture = Fixture::new();
    let root = fixture.repo().join("schemas");
    write(&root.join("roster.yaml"), ROSTER_DEFINITION);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000)).unwrap();
    let _restore = Restore(root.clone());
    if std::fs::read_dir(&root).is_ok() {
        // Running with privileges that ignore permissions: nothing to prove.
        return;
    }
    let document = fixture.package().join("roster.yaml");
    let registry = registry(&fixture.context(&document, &[]));
    assert_eq!(registry.root_failures().len(), 1);
    let (_, cause) = selection_cause(fixture.validate(&document, ROSTER_DOCUMENT));
    assert!(matches!(cause, KindSelectionCause::RootUnreadable { root: failed, .. } if failed == root));
    assert!(matches!(fixture.validate(&document, "a: 1\n"), Ok(YamlOutcome::NoSchema)));
}

// ── Claims declared by the definition ──────────────────────────────────────

#[test]
fn a_yaml_schema_whose_kind_is_a_plain_string_fails_on_the_kind_line() {
    let fixture = Fixture::new();
    let definition = fixture.repo().join("schemas/loose.yaml");
    let text = "kind: yaml-schema\n$schema:\n    kind: string(required)\n    name: string\n";
    write(&definition, text);
    let registry = registry(&fixture.context(&fixture.package().join("a.yaml"), &[]));
    assert_eq!(registry.incomplete().len(), 1, "a plain-string kind reveals no claim");
    let own = validated(fixture.validate(&definition, text));
    assert!(!own.report.valid);
    let problem = &own.report.problems[0];
    assert_eq!(problem.line, Some(3), "{problem:?}");
}

#[test]
fn root_union_definitions_claim_each_arm_s_kinds_and_reject_an_optional_arm() {
    let fixture = Fixture::new();
    let document = fixture.package().join("a.yaml");
    write(
        &fixture.repo().join("schemas/pair.yaml"),
        "kind: yaml-schema\n$schema:\n    - kind: literal(left; required)\n      a: string\n    - kind: enum(right, middle; required)\n      b: string\n",
    );
    write(
        &fixture.repo().join("schemas/bad.yaml"),
        "kind: yaml-schema\n$schema:\n    - kind: literal(up; required)\n    - kind: literal(down)\n",
    );
    let registry = registry(&fixture.context(&document, &[]));
    for kind in ["left", "right", "middle"] {
        assert!(matches!(registry.claim(kind), Some(Claim::Healthy(_))), "{kind}");
    }
    // The optional arm fails the definition; its inline claims are reserved.
    assert!(matches!(registry.claim("up"), Some(Claim::Reserved { .. })), "{:?}", registry.claim("up"));
    assert!(matches!(registry.claim("down"), Some(Claim::Reserved { .. })));
    assert!(validated(fixture.validate(&document, "kind: middle\nb: x\n")).report.valid);
}

#[test]
fn a_registry_can_be_shared_by_every_document_of_its_context() {
    let fixture = Fixture::new();
    let document = fixture.package().join("roster.yaml");
    write(&fixture.repo().join("schemas/roster.yaml"), ROSTER_DEFINITION);
    let context = fixture.context(&document, &[]);
    let shared = Arc::new(registry(&context));
    let schemas = DarkmatterSchemas::new(context).with_kind_registry(Arc::clone(&shared));
    for text in [ROSTER_DOCUMENT, "kind: roster\nroster_version: 1\nplatforms: []\n"] {
        assert!(validated(schemas.validate_yaml_document(text, &document)).report.valid);
    }
    assert!(shared.dependencies().iter().any(|path| path.ends_with("schemas/roster.yaml")));
    assert!(shared.searched_locations().iter().any(|path| path.ends_with("repo/schemas")));
}
