//! [`ResolvedSchema::searched`]: every location resolution probed, including
//! absent nearer candidates, for each local reference form.
//!
//! Each fixture is a repository `{tmp}/repo` (with `.git`) whose schemas live
//! in `repo/schemas/`. Implicit paths authored there try `repo/schemas/…`
//! before `repo/…`, so a reference can select a repository-root file while
//! its nearer candidate is absent.

use std::fs;
use std::path::PathBuf;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use serde_yaml_ng::Value as YamlValue;

use super::{ResolvedSchema, SchemaError, canonical_path, resolve_yaml_schema_with_roots};

const VALID_EXAMPLE: &str = "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

struct Repo {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(tmp.path()).unwrap().join("repo");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        Self { _tmp: tmp, root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Resolves `$schema: {reference}` authored in `repo/schemas/` with the
    /// given schema roots.
    fn resolve(&self, reference: &str, roots: &[PathBuf]) -> Result<ResolvedSchema, SchemaError> {
        let base = self.path("schemas");
        let context = FileResolutionContext::new(&base).with_repository_root(self.root.clone());
        resolve_yaml_schema_with_roots(&YamlValue::String(reference.into()), &base, roots, &context)
    }

    fn paths(&self, relatives: &[&str]) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = relatives.iter().map(|relative| self.path(relative)).collect();
        paths.sort();
        paths
    }
}

const FIELDS: &str = "$schema:\n  Fields:\n    type: object\n    properties:\n      shared: string\n";

#[test]
fn implicit_import_records_its_absent_nearer_candidate_and_the_selection() {
    let repo = Repo::new();
    repo.write("schemas/payload.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    repo.write("shared/types.yaml", FIELDS);

    let resolved = repo.resolve("./payload.yaml", &[]).expect("payload resolves");

    assert_eq!(
        resolved.searched,
        repo.paths(&["schemas/payload.yaml", "schemas/shared/types.yaml", "shared/types.yaml"]),
    );
    assert!(!repo.path("schemas/shared").exists(), "the nearer candidate's folder stays absent");
    // The content-hashed dependency lists are unchanged: only existing files.
    assert_eq!(resolved.imports, vec![canonical_path(&repo.path("shared/types.yaml"))]);
    assert_eq!(resolved.referenced_files, vec![canonical_path(&repo.path("schemas/payload.yaml"))]);
}

#[test]
fn nearer_candidate_that_is_a_directory_is_recorded_before_the_fallback() {
    let repo = Repo::new();
    repo.write("schemas/payload.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    repo.write("shared/types.yaml", FIELDS);
    fs::create_dir_all(repo.path("schemas/shared/types.yaml")).unwrap();

    let resolved = repo.resolve("./payload.yaml", &[]).expect("a directory candidate advances");

    assert!(resolved.searched.contains(&repo.path("schemas/shared/types.yaml")));
    assert_eq!(resolved.imports, vec![canonical_path(&repo.path("shared/types.yaml"))]);
}

#[test]
fn explicit_relative_references_record_only_their_one_location() {
    let repo = Repo::new();
    repo.write("schemas/payload.yaml", "$schema:\n  fields: Fields@../shared/types.yaml\n");
    repo.write("shared/types.yaml", FIELDS);

    let resolved = repo.resolve("./payload.yaml", &[]).expect("payload resolves");

    assert_eq!(resolved.searched, repo.paths(&["schemas/payload.yaml", "shared/types.yaml"]));
}

#[test]
fn bare_name_records_each_schema_root_tried_through_the_selection() {
    let repo = Repo::new();
    repo.write("far/claudine.yaml", "$schema:\n  model: string\n");
    let roots = vec![repo.path("near"), repo.path("far"), repo.path("farther")];

    let resolved = repo.resolve("claudine.yaml", &roots).expect("far root has it");

    assert_eq!(
        resolved.searched,
        repo.paths(&["near/claudine.yaml", "far/claudine.yaml"]),
        "roots after the selected one are never tried",
    );
}

#[test]
fn whole_file_delegation_merges_each_hop() {
    let repo = Repo::new();
    repo.write("schemas/a.yaml", "$schema: shared/b.yaml\n");
    repo.write("shared/b.yaml", "$schema:\n  model: string\n");

    let resolved = repo.resolve("./a.yaml", &[]).expect("delegation resolves");

    assert_eq!(
        resolved.searched,
        repo.paths(&["schemas/a.yaml", "schemas/shared/b.yaml", "shared/b.yaml"]),
    );
}

#[test]
fn root_union_arms_merge_their_searches() {
    let repo = Repo::new();
    repo.write("shared/one.yaml", "$schema:\n  one: string\n");
    repo.write("shared/two.yaml", "$schema:\n  two: string\n");
    let base = repo.path("schemas");
    let context = FileResolutionContext::new(&base).with_repository_root(repo.root.clone());
    let union: YamlValue = serde_yaml_ng::from_str("[shared/one.yaml, shared/two.yaml]").unwrap();

    let resolved = resolve_yaml_schema_with_roots(&union, &base, &[], &context).expect("union resolves");

    assert_eq!(
        resolved.searched,
        repo.paths(&[
            "schemas/shared/one.yaml",
            "schemas/shared/two.yaml",
            "shared/one.yaml",
            "shared/two.yaml",
        ]),
    );
}

#[test]
fn example_file_records_its_absent_nearer_candidate() {
    let repo = Repo::new();
    repo.write("schemas/payload.yaml", "$schema:\n  demo: \"string(example(examples/demo.yaml))\"\n");
    repo.write("examples/demo.yaml", VALID_EXAMPLE);

    let resolved = repo.resolve("./payload.yaml", &[]).expect("example resolves");

    assert_eq!(
        resolved.searched,
        repo.paths(&["examples/demo.yaml", "schemas/examples/demo.yaml", "schemas/payload.yaml"]),
    );
    assert_eq!(resolved.examples, vec![canonical_path(&repo.path("examples/demo.yaml"))]);
}

#[test]
fn transitive_import_and_its_rehomed_example_are_recorded() {
    let repo = Repo::new();
    // `lib/types.yaml` imports again from its own folder and names an example.
    repo.write(
        "schemas/lib/types.yaml",
        "$schema:\n  T: \"string(example(examples/demo.yaml))\"\n  U: Fields@shared/types.yaml\n",
    );
    repo.write("examples/demo.yaml", VALID_EXAMPLE);
    repo.write("schemas/payload.yaml", "$schema:\n  value: T@./lib/types.yaml\n  other: U@./lib/types.yaml\n");
    repo.write("shared/types.yaml", FIELDS);

    let resolved = repo.resolve("./payload.yaml", &[]).expect("transitive import resolves");

    for expected in [
        "schemas/lib/types.yaml",
        "schemas/lib/examples/demo.yaml",
        "examples/demo.yaml",
        "schemas/lib/shared/types.yaml",
        "shared/types.yaml",
    ] {
        assert!(
            resolved.searched.contains(&repo.path(expected)),
            "{expected} missing from {:?}",
            resolved.searched
        );
    }
    assert!(
        !resolved.searched.contains(&repo.path("schemas/shared/types.yaml")),
        "a transitive import searches from its own file's folder",
    );
}

#[test]
fn second_import_of_a_cached_namespace_still_records_its_own_search() {
    let repo = Repo::new();
    // The first site loads `shared/types.yaml`; the second, from
    // `schemas/sub/`, reuses that loaded namespace.
    repo.write(
        "schemas/payload.yaml",
        "$schema:\n  first: Fields@shared/types.yaml\n  second: B@./sub/mid.yaml\n",
    );
    repo.write("schemas/sub/mid.yaml", "$schema:\n  B: Fields@shared/types.yaml\n");
    repo.write("shared/types.yaml", FIELDS);

    let resolved = repo.resolve("./payload.yaml", &[]).expect("payload resolves");

    assert!(resolved.searched.contains(&repo.path("schemas/shared/types.yaml")));
    assert!(
        resolved.searched.contains(&repo.path("schemas/sub/shared/types.yaml")),
        "the cache-hit site's own nearer candidate: {:?}",
        resolved.searched
    );
}

#[test]
fn failed_resolution_still_errors_and_returns_no_partial_record() {
    let repo = Repo::new();
    repo.write("schemas/payload.yaml", "$schema:\n  fields: Fields@shared/missing.yaml\n");

    let error = repo.resolve("./payload.yaml", &[]).expect_err("the import matches nothing");

    assert!(
        matches!(&error, SchemaError::NoMatch { reference, bare_name: false } if reference == "shared/missing.yaml"),
        "{error:?}"
    );
}

#[test]
fn inline_mapping_records_the_searches_of_its_imports() {
    let repo = Repo::new();
    repo.write("shared/types.yaml", FIELDS);
    let base = repo.path("schemas");
    let context = FileResolutionContext::new(&base).with_repository_root(repo.root.clone());
    let inline: YamlValue = serde_yaml_ng::from_str("fields: Fields@shared/types.yaml").unwrap();

    let resolved = resolve_yaml_schema_with_roots(&inline, &base, &[], &context).expect("inline resolves");

    assert_eq!(resolved.searched, repo.paths(&["schemas/shared/types.yaml", "shared/types.yaml"]));
    assert!(resolved.referenced_files.is_empty());
}
