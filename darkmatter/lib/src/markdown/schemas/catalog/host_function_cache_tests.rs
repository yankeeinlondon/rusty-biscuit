//! Host function discovery: one cached result per catalog path, replaced
//! when the catalog or its facts artifact changes, with replaced catalogs
//! freed once no caller holds them. Every test goes through
//! `host_function_catalogs`, the production lookup, in its own temp roots,
//! and inspects only its own paths' cache entries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::discovery_probe::{CachedArtifact, DiscoveryProbe, cached_host_functions, host_function_entries_under};
use super::*;
use crate::markdown::compose::expression::host::facts_artifact_path;
use crate::markdown::compose::expression::{
    CallProblem, DeclaredTypes, HostFunctionCatalog, HostFunctions, TypeScope, check_expression_calls,
    incomplete_host_calls, parse_spanned,
};

/// A repository whose tree root (`repo/schemas`) is searched before the
/// `SCHEMAS_DIR` root (`external`).
struct Roots {
    _dir: tempfile::TempDir,
    /// The canonical temp dir every path below is in.
    base: PathBuf,
    tree: PathBuf,
    external: PathBuf,
    context: biscuit_file::FileResolutionContext,
}

fn roots() -> Roots {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let base = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let repo = base.join("repo");
    let tree = repo.join("schemas");
    let external = base.join("external");
    std::fs::create_dir_all(repo.join(".git")).expect("repository marker");
    std::fs::create_dir_all(&tree).expect("tree root");
    std::fs::create_dir_all(&external).expect("SCHEMAS_DIR root");
    let env = HashMap::from([("SCHEMAS_DIR".to_string(), external.to_string_lossy().into_owned())]);
    let context = biscuit_file::FileResolutionContext::from_snapshot(&repo, None, env).with_repository_root(&repo);
    Roots { _dir: dir, base, tree, external, context }
}

impl Roots {
    fn discover(&self) -> Vec<Result<HostFunctions, HostFunctionCatalogError>> {
        host_function_catalogs(&self.context)
    }

    /// The one discovered set, which must have loaded.
    fn only_set(&self) -> HostFunctions {
        let mut sets = self.discover();
        assert_eq!(sets.len(), 1, "{sets:?}");
        sets.remove(0).expect("the catalog loads")
    }

    fn entries(&self) -> usize {
        host_function_entries_under(&self.base)
    }
}

/// A host catalog declaring `function(parameter: string) -> string`, with
/// an inline display-only example before a verified one.
fn host_catalog(function: &str, parameter: &str, description: &str) -> String {
    format!(
        r#"kind: schema
categories:
    - Greeting -> Greetings.
types:
    {function}: |-
        function(
            parameters({parameter}: string);
            returns(string);
            category(Greeting);
            example({function}("a") => "hello a"; display-only: depends on the host);
            example({function}("b") => "hello b");
        ) -> {description}
"#
    )
}

/// A host catalog whose first function loads, with an inline example, and
/// whose second fails projection: a host catalog cannot embed a file-form
/// example.
fn failing_catalog() -> String {
    let mut text = host_catalog("greet", "name", "Greets.");
    text.push_str(
        "    later: |-\n        function(\n            parameters(x: string);\n            returns(string);\n            category(Greeting);\n            example(./later.yaml);\n        ) -> Later.\n",
    );
    text
}

/// The facts artifact a host registering every function of `text` (without
/// facts) writes.
fn facts_for(path: &Path, text: &str) -> String {
    let catalog = HostFunctionCatalog::load(text, path).expect("the catalog loads");
    let names: Vec<String> = catalog.function_names().into_iter().map(str::to_string).collect();
    let mut builder = HostFunctions::builder(catalog);
    for name in names {
        builder = builder.function(name, &[], |registration| registration, Arc::new(|_| Ok(serde_json::Value::Null)));
    }
    builder.build().expect("the host registers").facts_artifact()
}

/// Writes `text` at `path` with its valid facts artifact beside it.
fn write_with_facts(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write the catalog");
    std::fs::write(facts_artifact_path(path), facts_for(path, text)).expect("write the facts");
}

/// The arity verdict on `greet("a", "b")` under the current thread's scopes:
/// the signatures analysis offers.
fn greet_signatures() -> Vec<String> {
    let declared = DeclaredTypes::new(None);
    let checks =
        check_expression_calls(&parse_spanned(r#"greet("a", "b")"#).expect("parses"), &TypeScope::unguarded(&declared));
    let check = checks.into_iter().find(|check| check.function == "greet").expect("the call is checked");
    match check.problem {
        Some(CallProblem::Arity { signatures, .. }) => signatures,
        other => panic!("an arity problem: {other:?}"),
    }
}

/// A catalog-only change replaces the path's entry; the replaced catalog is
/// freed once its set, clones, and guard are gone, and inline examples and
/// display-only reasons are owned by the catalog it frees.
#[test]
fn a_catalog_only_change_replaces_the_entry_and_frees_the_old_catalog() {
    let roots = roots();
    let path = roots.tree.join("greet.yaml");
    let first_text = host_catalog("greet", "name", "Greets.");
    write_with_facts(&path, &first_text);
    let facts = std::fs::read_to_string(facts_artifact_path(&path)).expect("the facts");
    let probe = DiscoveryProbe::install();

    let first = roots.only_set();
    let owner = first.catalog().owner_probe();
    let guard = first.enter();
    let clone = first.catalog().clone();
    assert_eq!(first.incomplete_functions(), Vec::<(&str, &str)>::new());
    let cached = cached_host_functions(&path).expect("an entry");
    assert_eq!(cached.catalog_hash, biscuit_hash::blake3_hash(&first_text));
    assert_eq!(cached.artifact, CachedArtifact::Text(facts.clone()));
    assert_eq!(cached.result.expect("loaded"), ["greet"]);

    // Unchanged: reused, not rebuilt.
    let again = roots.only_set();
    assert_eq!(probe.host_function_builds(), 1);
    assert_eq!(roots.entries(), 1);

    // Only the description changes; the artifact bytes stay the same, so
    // its recorded hash is now stale.
    let second_text = host_catalog("greet", "name", "Greets warmly.");
    std::fs::write(&path, &second_text).expect("rewrite the catalog");
    let second = roots.only_set();
    assert_eq!(probe.host_function_builds(), 2);
    assert_eq!(roots.entries(), 1);
    let cached = cached_host_functions(&path).expect("an entry");
    assert_eq!(cached.catalog_hash, biscuit_hash::blake3_hash(&second_text));
    assert_eq!(cached.artifact, CachedArtifact::Text(facts));
    assert_eq!(second.catalog().version(), biscuit_hash::blake3_hash(&second_text));

    // The held old values keep the old catalog alive.
    assert!(owner.is_alive());
    drop((first, again, clone));
    assert!(owner.is_alive(), "the scope guard still holds the old set");
    drop(guard);
    assert!(!owner.is_alive(), "nothing retains the replaced catalog");
    assert!(second.catalog().owner_probe().is_alive(), "the cache holds the current revision");
}

/// An artifact-only change replaces the entry under the same catalog hash.
#[test]
fn an_artifact_only_change_replaces_the_entry_and_frees_the_old_catalog() {
    let roots = roots();
    let path = roots.tree.join("greet.yaml");
    let text = host_catalog("greet", "name", "Greets.");
    write_with_facts(&path, &text);
    let probe = DiscoveryProbe::install();

    let first = roots.only_set();
    let owner = first.catalog().owner_probe();
    assert!(first.registration("greet").is_some());

    std::fs::remove_file(facts_artifact_path(&path)).expect("remove the facts");
    let second = roots.only_set();
    assert_eq!(probe.host_function_builds(), 2);
    assert_eq!(roots.entries(), 1);
    let cached = cached_host_functions(&path).expect("an entry");
    assert_eq!(cached.catalog_hash, biscuit_hash::blake3_hash(&text));
    assert_eq!(cached.artifact, CachedArtifact::Absent);
    assert!(second.registration("greet").is_none(), "the new revision has no facts");
    assert!(first.registration("greet").is_some(), "the held set keeps its facts");

    drop(first);
    assert!(!owner.is_alive());
}

/// A catalog that fails to load replaces a loaded one: the old set is never
/// served for the failing revision, the error is reused while unchanged,
/// and a valid catalog recovers.
#[test]
fn a_failing_catalog_replaces_a_loaded_one_and_a_valid_one_recovers() {
    let roots = roots();
    let path = roots.tree.join("greet.yaml");
    write_with_facts(&path, &host_catalog("greet", "name", "Greets."));
    let probe = DiscoveryProbe::install();

    let first = roots.only_set();
    let owner = first.catalog().owner_probe();
    drop(first);
    assert!(owner.is_alive(), "the cache holds the loaded revision");

    let failing = failing_catalog();
    std::fs::write(&path, &failing).expect("write the failing catalog");
    let error = roots.discover().remove(0).expect_err("the catalog does not load");
    assert_eq!(error.path, path);
    assert!(error.message.contains("./later.yaml"), "{}", error.message);
    assert!(!owner.is_alive(), "the failing revision released the loaded one");
    assert_eq!(probe.host_function_builds(), 2);
    let cached = cached_host_functions(&path).expect("an entry");
    assert_eq!(cached.catalog_hash, biscuit_hash::blake3_hash(&failing));
    assert_eq!(cached.result.expect_err("the cached error"), error);

    // Unchanged: the cached error, not a rebuild.
    let again = roots.discover().remove(0).expect_err("still failing");
    assert_eq!(again, error);
    assert_eq!(probe.host_function_builds(), 2);
    assert_eq!(roots.entries(), 1);

    let recovered_text = host_catalog("greet", "name", "Greets again.");
    write_with_facts(&path, &recovered_text);
    let recovered = roots.only_set();
    assert_eq!(probe.host_function_builds(), 3);
    assert_eq!(roots.entries(), 1);
    assert_eq!(recovered.incomplete_functions(), Vec::<(&str, &str)>::new());
    let cached = cached_host_functions(&path).expect("an entry");
    assert_eq!(cached.catalog_hash, biscuit_hash::blake3_hash(&recovered_text));
    assert_eq!(cached.result.expect("loaded"), ["greet"]);
}

/// A loaded set's facts follow the artifact through every state: valid,
/// absent, invalid JSON, a stale catalog hash, and bytes that are not
/// UTF-8. Each is a successful set with the reason its functions are
/// incomplete, cached under its own artifact state, and valid facts recover.
#[test]
fn facts_transitions_classify_each_artifact_state_and_recover() {
    let roots = roots();
    let path = roots.tree.join("greet.yaml");
    let text = host_catalog("greet", "name", "Greets.");
    write_with_facts(&path, &text);
    let facts_path = facts_artifact_path(&path);
    let valid = std::fs::read_to_string(&facts_path).expect("the facts");
    let file = format!("the facts artifact `{}`", facts_path.display());
    let probe = DiscoveryProbe::install();

    let incomplete = |set: &HostFunctions| -> Option<String> {
        match set.incomplete_functions().as_slice() {
            [] => None,
            [("greet", reason)] => Some((*reason).to_string()),
            other => panic!("one function: {other:?}"),
        }
    };
    let artifact = || cached_host_functions(&path).expect("an entry").artifact;

    assert_eq!(incomplete(&roots.only_set()), None);
    assert_eq!(artifact(), CachedArtifact::Text(valid.clone()));

    std::fs::remove_file(&facts_path).expect("remove the facts");
    assert_eq!(incomplete(&roots.only_set()), Some(format!("{file} is absent")));
    assert_eq!(artifact(), CachedArtifact::Absent);

    std::fs::write(&facts_path, "{ not json").expect("invalid JSON");
    let reason = incomplete(&roots.only_set()).expect("incomplete");
    assert!(reason.starts_with(&file) && reason != format!("{file} is absent"), "{reason}");
    assert_eq!(artifact(), CachedArtifact::Text("{ not json".to_string()));

    let stale = valid.replace(&biscuit_hash::blake3_hash(&text), &"0".repeat(64));
    std::fs::write(&facts_path, &stale).expect("stale facts");
    let reason = incomplete(&roots.only_set()).expect("incomplete");
    assert!(reason.contains("records catalog hash `0000") && reason.ends_with("regenerate the artifact"), "{reason}");
    assert_eq!(artifact(), CachedArtifact::Text(stale));

    std::fs::write(&facts_path, [0xff, 0xfe, b'{']).expect("non-UTF-8 bytes");
    assert_eq!(incomplete(&roots.only_set()), Some(format!("{file} cannot be read: the file is not UTF-8 text")));
    assert_eq!(artifact(), CachedArtifact::Unreadable("the file is not UTF-8 text".to_string()));

    std::fs::write(&facts_path, &valid).expect("valid facts again");
    assert_eq!(incomplete(&roots.only_set()), None);
    assert_eq!(artifact(), CachedArtifact::Text(valid));

    assert_eq!(probe.host_function_builds(), 6, "every artifact state is a new revision");
    assert_eq!(roots.entries(), 1);
}

/// A set held with its scope guard across a replacement keeps its own
/// signatures, registrations, and call analysis; the new set has the
/// replacement's, a newer guard takes precedence only while it lives, and
/// the old catalog is freed once its set and guard drop.
#[test]
fn a_held_set_and_guard_survive_replacement_with_their_own_analysis() {
    let roots = roots();
    let path = roots.tree.join("greet.yaml");
    write_with_facts(&path, &host_catalog("greet", "name", "Greets."));

    let old = roots.only_set();
    let owner = old.catalog().owner_probe();
    let old_guard = old.enter();
    assert_eq!(greet_signatures(), ["greet(name: string): string"]);

    write_with_facts(&path, &host_catalog("greet", "person", "Greets a person."));
    let new = roots.only_set();
    assert_eq!(roots.entries(), 1);

    let parameter = |set: &HostFunctions| {
        let registration = set.registration("greet").expect("complete");
        registration.overloads()[0].parameters()[0].name().to_string()
    };
    assert_eq!(parameter(&old), "name");
    assert_eq!(parameter(&new), "person");
    let call = parse_spanned(r#"greet("a")"#).expect("parses");
    assert!(incomplete_host_calls(&call).is_empty(), "the old set's facts are complete");
    assert_eq!(greet_signatures(), ["greet(name: string): string"], "only the old set is in scope");

    {
        let _new_guard = new.enter();
        assert_eq!(greet_signatures(), ["greet(person: string): string"], "a later guard is consulted first");
    }
    assert_eq!(greet_signatures(), ["greet(name: string): string"], "the old set again once the newer guard drops");

    drop(old);
    assert!(owner.is_alive(), "the guard holds the old set");
    drop(old_guard);
    assert!(!owner.is_alive());
}

/// Two catalog paths keep independent entries in discovery order; a file
/// shadowed by an earlier root's file of the same name is never loaded, and
/// replacing one path's revision leaves the other's set and entry intact.
#[test]
fn distinct_paths_keep_independent_entries_in_discovery_order() {
    let roots = roots();
    let alpha = roots.tree.join("alpha.yaml");
    let beta = roots.tree.join("beta.yaml");
    let shadowed = roots.external.join("alpha.yaml");
    write_with_facts(&alpha, &host_catalog("greet_alpha", "name", "Alpha."));
    write_with_facts(&beta, &host_catalog("greet_beta", "name", "Beta."));
    write_with_facts(&shadowed, &host_catalog("greet_shadowed", "name", "Shadowed."));
    let probe = DiscoveryProbe::install();

    let names = |sets: &[Result<HostFunctions, HostFunctionCatalogError>]| -> Vec<String> {
        sets.iter().map(|set| set.as_ref().expect("loads").function_names().join(",")).collect()
    };
    let first = roots.discover();
    assert_eq!(names(&first), ["greet_alpha", "greet_beta"]);
    assert_eq!(roots.entries(), 2);
    assert!(cached_host_functions(&shadowed).is_none(), "the shadowed file is never loaded");
    let beta_owner = first[1].as_ref().expect("loads").catalog().owner_probe();
    let beta_hash = cached_host_functions(&beta).expect("an entry").catalog_hash;

    write_with_facts(&alpha, &host_catalog("greet_alpha", "name", "Alpha, revised."));
    drop(first);
    let second = roots.discover();
    assert_eq!(names(&second), ["greet_alpha", "greet_beta"]);
    assert_eq!(probe.host_function_builds(), 3, "only alpha was rebuilt");
    assert_eq!(roots.entries(), 2);
    assert_eq!(cached_host_functions(&beta).expect("an entry").catalog_hash, beta_hash);
    assert!(beta_owner.is_alive(), "beta's cached set is the one already loaded");
    let beta_set = second[1].as_ref().expect("loads");
    assert_eq!(beta_set.incomplete_functions(), Vec::<(&str, &str)>::new());
    assert!(beta_set.registration("greet_beta").is_some());
}
