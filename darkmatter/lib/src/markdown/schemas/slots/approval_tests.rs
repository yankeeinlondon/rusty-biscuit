//! Pre-flight's passive collection of the reads a template's slots name.
//!
//! The approval model itself (permissions, approvals for this run, the
//! handler, saving an "always allow", listings, URLs) is exercised through
//! the public API in `tests/l1/slot_permissions.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use simplified_schema::SchemaError;
use simplified_schema::slots::PendingSchema;

use super::grants::targets::{CollectedRead, collect_targets};

const SCHEMA: &str = "schemas/sandbox.yaml";
const DATA: &str = "data/os.yaml";

/// A folder holding the sandbox schema.
struct Repo {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn with_schema(schema: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("repo");
        let repo = Self { _temp: temp, root };
        let path = repo.path(SCHEMA);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, schema).unwrap();
        repo
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }
}

fn context(root: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(root).with_env(HashMap::new())
}

fn pending_at(root: &Path, schema: &Path) -> PendingSchema {
    let reference = serde_yaml_ng::Value::String(biscuit_file::to_forward_slash_string(schema));
    match simplified_schema::resolve::resolve_yaml_schema(
        &reference,
        root,
        &context(root),
        crate::markdown::schemas::embedded::SHIPPED_SOURCES,
    ) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("a pending handle"),
        other => panic!("expected a template, got {other:?}"),
    }
}

#[test]
fn a_target_of_literals_and_pure_functions_is_known_before_evaluation() {
    let repo = Repo::with_schema(
        "kind: schema\n$schema:\n    a: enum({{ load_data(lower(\"../DATA/OS.YAML\"), \"string[]\") }})\n    b: enum({{ load_data(env.DATA, \"string[]\") }})\n    c: 'enum({{ true ? [\"x\"] : load_data(\"https://example.com/x.yaml\") }})'\n",
    );
    let reads: Vec<CollectedRead> = collect_targets(&pending_at(&repo.root, &repo.path(SCHEMA)), &context(&repo.root))
        .into_iter()
        .flat_map(|slot| slot.reads)
        .collect();
    use crate::markdown::compose::expression::read_gate::ReadLocation;
    let locations: Vec<Option<ReadLocation>> = reads
        .iter()
        .map(|read| match read {
            CollectedRead::Known(target) => Some(target.location.clone()),
            CollectedRead::Computed { .. } => None,
        })
        .collect();
    assert_eq!(
        locations,
        [
            Some(ReadLocation::Local(repo.path(DATA))),
            None,
            Some(ReadLocation::Remote(url::Url::parse("https://example.com/x.yaml").unwrap())),
        ],
        "a branch that would not run still counts"
    );
}
