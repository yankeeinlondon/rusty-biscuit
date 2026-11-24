//! Whole-YAML kind selection through `select` and `select_with_registered`:
//! the registered tier is consulted only for a kind the local registry
//! establishes is unclaimed, and never shadows a shipped claim, a local
//! claim, or a local failure.
//!
//! Every registry is built over the test's own temporary directory (its tree
//! root holds the `schemas/` root); no home, environment, or process working
//! directory is read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::{ResolvedSchema, resolve_yaml_schema};
use simplified_schema::triggers::discovery::SchemaRootSource;
use simplified_schema::yaml_documents::{
    DefinitionSource, KindHost, KindRegistry, RegisteredDefinition, RegisteredKinds, RegisteredSource, RegistryScope,
    Selection, ShippedDefinition, prepare_shipped_definitions, select, select_with_registered,
};
use simplified_schema::{KindSelectionCause, SchemaError};

const ROSTER: &str = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    team: string(required)\n";

/// A temporary tree whose `schemas/` folder is a schema root.
struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let tree = Self { dir: tempfile::tempdir().expect("temp dir") };
        std::fs::create_dir_all(tree.schemas()).expect("schemas folder");
        tree
    }

    fn root(&self) -> PathBuf {
        biscuit_file::canonicalize_simplified(self.dir.path()).unwrap_or_else(|_| self.dir.path().to_path_buf())
    }

    fn schemas(&self) -> PathBuf {
        self.root().join("schemas")
    }

    fn define(&self, name: &str, text: &str) -> PathBuf {
        let file = self.schemas().join(name);
        std::fs::write(&file, text).expect("write definition");
        file
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(self.root()).without_home_dir()
    }

    fn registry(&self, shipped: &[ShippedDefinition]) -> KindRegistry {
        self.registry_from(shipped, &mut DiskRoots { unreadable: false })
    }

    fn registry_from(&self, shipped: &[ShippedDefinition], source: &mut dyn SchemaRootSource) -> KindRegistry {
        let host = KindHost { sources: ShippedSources::NONE, shipped };
        KindRegistry::build(&self.context(), source, RegistryScope::WithRoots, host)
    }
}

/// Root files read from disk, sorted; `unreadable` fails every root.
struct DiskRoots {
    unreadable: bool,
}

impl SchemaRootSource for DiskRoots {
    fn root_files(&mut self, root: &Path) -> Result<Vec<PathBuf>, SchemaError> {
        if self.unreadable {
            return Err(SchemaError::Io {
                path: root.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
            });
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(root)
            .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
            .unwrap_or_default();
        files.sort();
        Ok(files)
    }

    fn root_file_text(&mut self, path: &Path) -> Result<Arc<str>, SchemaError> {
        Ok(std::fs::read_to_string(path).expect("fixture text").into())
    }
}

fn shipped() -> Vec<ShippedDefinition> {
    prepare_shipped_definitions(ShippedSources::NONE)
}

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

/// The `$schema` declaration `declaration` resolved in `dir`.
fn resolved(dir: &Path, declaration: &str) -> ResolvedSchema {
    resolve_yaml_schema(&yaml(declaration), dir, &FileResolutionContext::new(dir), ShippedSources::NONE)
        .unwrap_or_else(|error| panic!("`{declaration}` resolves: {error}"))
}

fn source(host: &str, path: &str) -> RegisteredSource {
    RegisteredSource { host: host.to_string(), path: path.to_string(), file: PathBuf::from(format!("{host}/{path}")) }
}

/// A registered definition from `host` at `path` claiming `kind`.
fn registered(dir: &Path, host: &str, path: &str, kind: &str, sameness: &str) -> RegisteredDefinition {
    let declaration = format!("kind: literal({kind}; required)\nteam: string(required)\n");
    RegisteredDefinition::new(source(host, path), resolved(dir, &declaration), sameness).expect("registered definition")
}

/// The definition source a `kind` selection chose.
#[track_caller]
fn selected_source(selection: Result<Selection<'_>, SchemaError>) -> DefinitionSource {
    match selection {
        Ok(Selection::Kind { definition, .. }) => definition.source().clone(),
        Ok(Selection::NoSchema) => panic!("expected a kind selection, got no schema"),
        Ok(Selection::RootSchema(_)) => panic!("expected a kind selection, got the root $schema"),
        Err(error) => panic!("expected a kind selection, got {error}"),
    }
}

#[track_caller]
fn refusal(selection: Result<Selection<'_>, SchemaError>) -> (String, KindSelectionCause) {
    match selection {
        Err(SchemaError::KindSelection { kind, cause }) => (kind, cause),
        Err(other) => panic!("expected a kind selection error, got {other}"),
        Ok(_) => panic!("expected a kind selection error, got a selection"),
    }
}

#[test]
fn registered_definition_selects_an_unclaimed_kind() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let selection = select_with_registered(&registry, Some(&registered), &document);

    match selected_source(selection) {
        DefinitionSource::Registered(source) => {
            assert_eq!(source.host, "claudine");
            assert_eq!(source.path, "schemas/roster.yaml");
        }
        other => panic!("expected the registered definition, got {other:?}"),
    }
    let label = DefinitionSource::Registered(self::source("claudine", "schemas/roster.yaml")).label();
    assert_eq!(label, "claudine:schemas/roster.yaml");
}

#[test]
fn shipped_kind_wins_over_a_registered_claim() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/schema.yaml", "schema", "c1")]);
    let document = yaml("kind: schema\ntypes:\n    name: string\n");

    let source = selected_source(select_with_registered(&registry, Some(&registered), &document));

    assert!(matches!(source, DefinitionSource::Shipped(_)), "the shipped definition wins: {source:?}");
}

#[test]
fn local_definition_wins_over_a_registered_claim() {
    let tree = Tree::new();
    let local = tree.define("roster.yaml", ROSTER);
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let source = selected_source(select_with_registered(&registry, Some(&registered), &document));

    assert_eq!(source, DefinitionSource::File(local));
}

#[test]
fn local_duplicate_claim_errors_without_consulting_registered_definitions() {
    let tree = Tree::new();
    let first = tree.define("a.yaml", ROSTER);
    let second = tree.define("b.yaml", ROSTER);
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let (kind, cause) = refusal(select_with_registered(&registry, Some(&registered), &document));

    assert_eq!(kind, "roster");
    assert_eq!(cause, KindSelectionCause::Duplicate { files: vec![first, second] });
}

#[test]
fn local_incomplete_definition_stays_unavailable_over_a_registered_claim() {
    let tree = Tree::new();
    let broken = tree.define("broken.yaml", "kind: yaml-schema\n$schema: [unclosed\n");
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let (_, cause) = refusal(select_with_registered(&registry, Some(&registered), &document));

    assert!(
        matches!(&cause, KindSelectionCause::Unavailable { file, .. } if *file == broken),
        "the failed local definition decides: {cause:?}"
    );
}

#[test]
fn unreadable_schema_root_errors_before_registered_definitions() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry_from(&shipped, &mut DiskRoots { unreadable: true });
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let (_, cause) = refusal(select_with_registered(&registry, Some(&registered), &document));

    assert!(matches!(cause, KindSelectionCause::RootUnreadable { .. }), "{cause:?}");
}

#[test]
fn different_registered_definitions_claiming_one_kind_are_a_duplicate_sorted_by_host_then_path() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let dir = tree.root();
    let registered = RegisteredKinds::new([
        registered(&dir, "zeta", "a/roster.yaml", "roster", "z1"),
        registered(&dir, "alpha", "z/roster.yaml", "roster", "a2"),
        registered(&dir, "alpha", "b/roster.yaml", "roster", "a1"),
    ]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let error = select_with_registered(&registry, Some(&registered), &document).err().expect("a duplicate claim");
    let message = error.to_string();
    let (kind, cause) = refusal(Err(error));

    assert_eq!(kind, "roster");
    let expected: Vec<PathBuf> =
        ["alpha/b/roster.yaml", "alpha/z/roster.yaml", "zeta/a/roster.yaml"].into_iter().map(PathBuf::from).collect();
    assert_eq!(cause, KindSelectionCause::Duplicate { files: expected });
    for file in ["alpha/b/roster.yaml", "alpha/z/roster.yaml", "zeta/a/roster.yaml"] {
        assert!(message.contains(file), "`{message}` names `{file}`");
    }
}

#[test]
fn equal_registered_copies_collapse_into_one_claim() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let dir = tree.root();
    let registered = RegisteredKinds::new([
        registered(&dir, "claudine", "schemas/roster.yaml", "roster", "claudine@r1:schemas/roster.yaml"),
        registered(&dir, "claudine", "schemas/roster.yaml", "roster", "claudine@r1:schemas/roster.yaml"),
    ]);
    let document = yaml("kind: roster\nteam: Platform\n");

    let source = selected_source(select_with_registered(&registry, Some(&registered), &document));

    assert!(matches!(source, DefinitionSource::Registered(ref source) if source.host == "claudine"), "{source:?}");
}

#[test]
fn missing_or_non_string_kind_never_consults_registered_definitions() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);

    for text in ["team: Platform\n", "kind: 3\nteam: Platform\n", "kind: null\n", "kind: [roster]\n"] {
        let document = yaml(text);
        let selection = select_with_registered(&registry, Some(&registered), &document);
        assert!(matches!(selection, Ok(Selection::NoSchema)), "`{text}` selects nothing");
    }
    let document = yaml("kind: [roster]\n$schema:\n    team: string\n");
    let selection = select_with_registered(&registry, Some(&registered), &document);
    assert!(matches!(selection, Ok(Selection::RootSchema(_))), "a non-string kind falls through to $schema");
}

#[test]
fn selected_registered_kind_takes_precedence_over_root_schema() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let registered = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/roster.yaml", "roster", "c1")]);
    let document = yaml("kind: roster\nteam: Platform\n$schema:\n    team: number\n");

    let source = selected_source(select_with_registered(&registry, Some(&registered), &document));

    assert!(matches!(source, DefinitionSource::Registered(_)), "{source:?}");
}

#[test]
fn select_without_registered_definitions_is_unchanged() {
    let tree = Tree::new();
    let shipped = shipped();
    let registry = tree.registry(&shipped);
    let empty = RegisteredKinds::default();
    let other = RegisteredKinds::new([registered(&tree.root(), "claudine", "schemas/crew.yaml", "crew", "c1")]);

    for text in ["kind: roster\nteam: Platform\n$schema:\n    team: string\n", "kind: roster\n", "team: x\n"] {
        let document = yaml(text);
        let route = |selection: Result<Selection<'_>, SchemaError>| match selection {
            Ok(Selection::NoSchema) => "none",
            Ok(Selection::RootSchema(_)) => "root",
            Ok(Selection::Kind { .. }) => "kind",
            Err(_) => "error",
        };
        let plain = route(select(&registry, &document));
        assert_eq!(plain, route(select_with_registered(&registry, None, &document)), "`{text}`");
        assert_eq!(plain, route(select_with_registered(&registry, Some(&empty), &document)), "`{text}`");
        assert_eq!(plain, route(select_with_registered(&registry, Some(&other), &document)), "`{text}`");
    }
    let document = yaml("kind: roster\n$schema:\n    team: string\n");
    assert!(matches!(select(&registry, &document), Ok(Selection::RootSchema(_))));
}

#[test]
fn registered_definition_claiming_no_kind_fails_like_a_local_one() {
    let tree = Tree::new();
    let export = resolved(&tree.root(), "kind: string(required)\nteam: string\n");

    let error = RegisteredDefinition::new(source("claudine", "schemas/open.yaml"), export, "c1")
        .expect_err("an open `kind` claims no finite set");

    match error {
        SchemaError::SchemaDocument { path, message, .. } => {
            assert_eq!(path, PathBuf::from("claudine/schemas/open.yaml"));
            assert!(message.contains("claims no document kind"), "{message}");
        }
        other => panic!("expected a schema document error, got {other}"),
    }
}
