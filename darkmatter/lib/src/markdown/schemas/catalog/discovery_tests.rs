//! Lifecycle discovery: owned, reclaimable profiles, one cached observation
//! per catalog path, the observed-bytes loader, and lazy candidate order.
//! The probe observes the production read and build path on this thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use super::discovery_probe::{CachedOutcome, DiscoveryProbe, cached_outcome};
use super::*;
use crate::lifecycle::host_profile::{HostLifecycleProfile, lifecycle_artifact, lifecycle_artifact_path};
use crate::lifecycle::registry::EventRegistry;
use crate::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy, VerbList};

/// A repository whose tree root (`repo/schemas`) is searched before the
/// `SCHEMAS_DIR` root (`external`).
struct Roots {
    _dir: tempfile::TempDir,
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
    Roots { _dir: dir, tree, external, context }
}

/// A host catalog declaring `run`; `description` makes distinct revisions.
fn host_catalog(description: &str) -> String {
    format!("kind: schema\ndescription: {description}\n$schema:\n    run:\n        count: \"number -> The pass.\"\n")
}

/// The artifact a host registering the owned global `run`, its catalog, and
/// an owned `ignore_unprovided` list writes for `catalog_text`.
fn host_artifact(catalog_text: &str, effects: &[&str]) -> String {
    let catalog =
        GlobalCatalog::from_documents("host.yaml", &[("host.yaml", catalog_text)], &[GlobalRequirement::shape("run")])
            .expect("the host catalog loads");
    let mut registry = EventRegistry::new("test");
    registry.declare_host_global(String::from("run")).expect("run is unreserved");
    registry.register_global_catalog(Arc::new(catalog)).expect("not shared yet");
    registry.validate().expect("no outcome events");
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::IgnoreUnprovided(vec![String::from("say")].into()),
        registry: &registry,
    };
    lifecycle_artifact(&options, effects, catalog_text)
}

/// Writes a usable host catalog at `dir/name` with a fresh artifact.
fn usable(dir: &Path, name: &str, description: &str) -> PathBuf {
    let path = dir.join(name);
    let text = host_catalog(description);
    std::fs::write(&path, &text).expect("catalog");
    std::fs::write(lifecycle_artifact_path(&path), host_artifact(&text, &["chime"])).expect("artifact");
    path
}

fn discover(context: &biscuit_file::FileResolutionContext) -> Vec<HostLifecycleDiscovery> {
    host_lifecycle_profiles(context).collect()
}

fn only_profile(context: &biscuit_file::FileResolutionContext) -> Arc<HostLifecycleProfile> {
    match discover(context).as_slice() {
        [only] => only.profile.clone().expect("the artifact is fresh"),
        other => panic!("one host catalog: {other:?}"),
    }
}

fn only_reason(context: &biscuit_file::FileResolutionContext) -> String {
    match discover(context).as_slice() {
        [only] => only.profile.clone().expect_err("the artifact is refused"),
        other => panic!("one host catalog: {other:?}"),
    }
}

/// Weak references to a profile and everything it owns.
struct Owned {
    profile: Weak<HostLifecycleProfile>,
    registry: Weak<crate::lifecycle::registry::EventRegistry>,
    catalog: Weak<GlobalCatalog>,
}

impl Owned {
    fn of(profile: &Arc<HostLifecycleProfile>) -> Self {
        let catalog = match profile.registry().global_catalog_handle() {
            Some(CatalogHandle::Shared(catalog)) => Arc::downgrade(catalog),
            other => panic!("a discovered profile owns its catalog: {other:?}"),
        };
        Self { profile: Arc::downgrade(profile), registry: Arc::downgrade(profile.registry()), catalog }
    }

    fn released(&self) -> bool {
        self.profile.upgrade().is_none() && self.registry.upgrade().is_none() && self.catalog.upgrade().is_none()
    }
}

/// Replacing the artifact, and separately the catalog, with another valid
/// revision releases the old profile, registry, and catalog once nothing
/// outside the cache holds them. A held profile, registry, or cloned policy
/// keeps its old semantics until its last holder drops.
#[test]
fn a_replaced_revision_releases_the_old_profile_once_its_holders_drop() {
    let roots = roots();
    let path = usable(&roots.tree, "host.yaml", "first");
    let text = std::fs::read_to_string(&path).expect("catalog");

    let first = only_profile(&roots.context);
    assert!(Arc::ptr_eq(&first, &only_profile(&roots.context)), "an unchanged revision is reused");
    let first_owned = Owned::of(&first);
    let held_registry = first.registry().clone();
    let held_policy = first.parse_options(None).unknown_verbs.clone();
    let UnknownVerbPolicy::IgnoreUnprovided(VerbList::Shared(verbs)) = &held_policy else {
        panic!("the artifact's verbs are owned: {held_policy:?}");
    };
    let verbs = Arc::downgrade(verbs);

    // Another valid artifact for the same catalog.
    std::fs::write(lifecycle_artifact_path(&path), host_artifact(&text, &["chime", "bell"])).expect("artifact");
    let second = only_profile(&roots.context);
    assert!(!Arc::ptr_eq(&first, &second));
    assert!(second.is_known_effect("bell"));
    assert!(!first.is_known_effect("bell"), "a held profile keeps its old analysis");
    drop(first);
    assert!(first_owned.profile.upgrade().is_none(), "the cache no longer holds the old profile");
    assert!(first_owned.registry.upgrade().is_some(), "a held registry keeps itself alive");
    assert!(held_registry.global_catalog().and_then(|catalog| catalog.global("run")).is_some());
    drop(held_registry);
    assert!(first_owned.released(), "the last holder of the registry released it and its catalog");
    assert!(held_policy.ignores("say"), "a cloned policy keeps its owned verbs");
    drop(held_policy);
    assert!(verbs.upgrade().is_none(), "and releases them when dropped");

    // A new catalog revision with its own fresh artifact.
    let second_owned = Owned::of(&second);
    usable(&roots.tree, "host.yaml", "second");
    let third = only_profile(&roots.context);
    assert!(!Arc::ptr_eq(&second, &third));
    assert!(!second_owned.released(), "the caller still holds the old profile");
    drop(second);
    assert!(second_owned.released());
    assert_eq!(cached_outcome(&path), Some(CachedOutcome::Profile));
}

/// An absent, unreadable, stale, or invalid artifact keeps its reason, is
/// not rebuilt while unchanged, and leaves a cache entry holding only that
/// reason. A repair is observed on the next discovery.
#[test]
fn a_refused_artifact_is_cached_by_reason_and_rebuilt_only_when_it_changes() {
    let roots = roots();
    let path = usable(&roots.tree, "host.yaml", "host");
    let artifact = lifecycle_artifact_path(&path);
    let fresh = std::fs::read_to_string(&artifact).expect("artifact");
    let probe = DiscoveryProbe::install();

    /// A reason's needle, how to break the artifact, and how to undo an
    /// injected failure.
    type Case<'a> = (&'a str, &'a dyn Fn(), &'a dyn Fn());
    let cases: [Case<'_>; 4] = [
        ("is absent", &|| std::fs::remove_file(&artifact).expect("remove artifact"), &|| {}),
        (
            "cannot be read: injected failure",
            &|| probe.fail_reads_of(&artifact, "injected failure"),
            &|| probe.restore_reads_of(&artifact),
        ),
        (
            "regenerate the artifact",
            &|| std::fs::write(&artifact, host_artifact(&host_catalog("another"), &["chime"])).expect("stale"),
            &|| {},
        ),
        ("instead of an object", &|| std::fs::write(&artifact, "[]").expect("invalid"), &|| {}),
    ];
    for (needle, break_it, unbreak_it) in cases {
        only_profile(&roots.context);
        break_it();
        let builds = probe.builds();
        let reason = only_reason(&roots.context);
        assert!(reason.contains(needle), "{needle}: {reason}");
        assert_eq!(only_reason(&roots.context), reason, "{needle}");
        assert_eq!(probe.builds(), builds + 1, "{needle}: an unchanged refusal is not rebuilt");
        assert_eq!(cached_outcome(&path), Some(CachedOutcome::Refused(reason)), "{needle}");

        unbreak_it();
        std::fs::write(&artifact, &fresh).expect("repair");
        only_profile(&roots.context);
        assert_eq!(probe.builds(), builds + 2, "{needle}: the repair is built on the next discovery");
    }
}

/// A cached success is dropped, never served again, when its catalog is
/// observed removed, unreadable, invalid, or no longer a host catalog.
#[test]
fn a_cached_success_is_dropped_when_its_catalog_stops_loading() {
    let roots = roots();
    let path = roots.tree.join("host.yaml");
    let probe = DiscoveryProbe::install();
    type Mutation<'a> = (&'a str, Box<dyn Fn() + 'a>, Option<CachedOutcome>);
    let mutations: Vec<Mutation<'_>> = vec![
        ("removed", Box::new(|| std::fs::remove_file(&path).expect("remove")), None),
        ("unreadable", Box::new(|| probe.fail_reads_of(&path, "injected failure")), None),
        (
            "invalid",
            Box::new(|| std::fs::write(&path, "kind: schema\n$schema:\n    run: 42\n").expect("invalid catalog")),
            None,
        ),
        (
            "no longer a host catalog",
            Box::new(|| std::fs::write(&path, "title: plain\n").expect("plain YAML")),
            Some(CachedOutcome::NotHostCatalog),
        ),
    ];
    for (label, mutate, after) in mutations {
        probe.restore_reads_of(&path);
        usable(&roots.tree, "host.yaml", label);
        let owned = Owned::of(&only_profile(&roots.context));
        assert_eq!(cached_outcome(&path), Some(CachedOutcome::Profile), "{label}");
        mutate();
        assert!(discover(&roots.context).is_empty(), "{label}: the old profile is never returned");
        assert_eq!(cached_outcome(&path), after, "{label}");
        assert!(owned.released(), "{label}: the dropped success is released");
    }
}

/// A file that is not a host catalog is cached as such, one entry per path:
/// its next revision replaces the entry.
#[test]
fn a_non_catalog_file_s_new_revision_replaces_its_entry() {
    let roots = roots();
    let path = roots.tree.join("host.yaml");
    std::fs::write(&path, "title: plain\n").expect("plain YAML");
    assert!(discover(&roots.context).is_empty());
    assert_eq!(cached_outcome(&path), Some(CachedOutcome::NotHostCatalog));

    usable(&roots.tree, "host.yaml", "now a catalog");
    only_profile(&roots.context);
    assert_eq!(cached_outcome(&path), Some(CachedOutcome::Profile), "the new revision replaced the entry");
}

/// The shared text loader builds from the bytes it is given, never from
/// the file at its path, and validates them as strictly as the file loader.
#[test]
fn the_host_text_loader_builds_from_the_supplied_bytes() {
    let roots = roots();
    let path = roots.tree.join("host.yaml");
    let supplied = host_catalog("supplied");

    let absent = GlobalCatalog::from_host_text(&path, &supplied, &roots.context).expect("no file is read");
    assert_eq!(absent.description(), Some("supplied"));

    std::fs::write(&path, "kind: schema\ndescription: on disk\n$schema:\n    other:\n        x: string\n").expect("disk");
    let loaded = GlobalCatalog::from_host_text(&path, &supplied, &roots.context).expect("supplied bytes load");
    assert_eq!(loaded.description(), Some("supplied"));
    assert!(loaded.global("run").is_some() && loaded.global("other").is_none());

    for invalid in [
        "kind: schema\n$schema:\n    run: 42\n",
        "kind: schema\n$schema:\n    run:\n        count: number\n    7: {x: string}\n",
        "kind: schema\n$schema:\n    run:\n        count: number\n    run:\n        count: number\n",
        "kind: schema\n$schema:\n    fm:\n        title: string\n",
    ] {
        std::fs::write(&path, invalid).expect("invalid catalog");
        let from_text = GlobalCatalog::from_host_text(&path, invalid, &roots.context).expect_err(invalid);
        let from_file = GlobalCatalog::from_host_file(&path, &roots.context).expect_err(invalid);
        assert_eq!(from_text.to_string(), from_file.to_string(), "{invalid}");
    }
}

/// With a usable first profile, discovery reads nothing of any later
/// candidate: neither its catalog nor its artifact.
#[test]
fn discovery_stops_reading_at_the_first_usable_profile() {
    let roots = roots();
    let first = usable(&roots.tree, "a.yaml", "first");
    let later = usable(&roots.tree, "b.yaml", "later");
    let external = usable(&roots.external, "c.yaml", "external");
    let probe = DiscoveryProbe::install();

    let found = host_lifecycle_profiles(&roots.context).find_map(|discovery| discovery.profile.ok());
    assert!(found.is_some());
    assert_eq!(probe.take_reads(), [first.clone(), lifecycle_artifact_path(&first)]);

    // And again from the cache: the winner is reread and hashed, nothing else.
    host_lifecycle_profiles(&roots.context).find_map(|discovery| discovery.profile.ok()).expect("cached");
    let reads = probe.take_reads();
    for unread in [&later, &external] {
        assert!(!reads.contains(unread) && !reads.contains(&lifecycle_artifact_path(unread)), "{reads:?}");
    }
}

/// Roots in order, file names sorted within each, and a name in an earlier
/// root shadowing the same name in later ones even when the earlier file is
/// unusable. With no usable profile, every reason arrives, in that order.
#[test]
fn discovery_keeps_root_order_sorted_names_and_shadowing() {
    let roots = roots();
    for name in ["b.yaml", "a.yml"] {
        std::fs::write(roots.tree.join(name), host_catalog(name)).expect("catalog without artifact");
    }
    std::fs::write(roots.tree.join("shadow.yaml"), "kind: schema\n$schema:\n    run: 42\n").expect("unusable");
    let shadowed = usable(&roots.external, "shadow.yaml", "shadowed but valid");
    std::fs::write(roots.external.join("c.yaml"), host_catalog("c")).expect("catalog without artifact");
    let probe = DiscoveryProbe::install();

    let found = discover(&roots.context);
    let catalogs: Vec<_> = found.iter().map(|discovery| discovery.catalog.clone()).collect();
    assert_eq!(catalogs, [roots.tree.join("a.yml"), roots.tree.join("b.yaml"), roots.external.join("c.yaml")]);
    for (discovery, catalog) in found.iter().zip(&catalogs) {
        let reason = discovery.profile.as_ref().expect_err("no artifact");
        let artifact = lifecycle_artifact_path(catalog).display().to_string();
        assert!(reason.contains(&artifact) && reason.ends_with("is absent"), "{reason}");
    }
    let reads = probe.take_reads();
    assert!(!reads.contains(&shadowed), "a shadowed file is never read: {reads:?}");

    // An earlier root's file shadows a later root's valid catalog of the same name.
    std::fs::write(roots.external.join("a.yml"), host_catalog("later")).expect("catalog");
    std::fs::write(
        lifecycle_artifact_path(&roots.external.join("a.yml")),
        host_artifact(&host_catalog("later"), &[]),
    )
    .expect("artifact");
    assert!(discover(&roots.context).iter().all(|discovery| discovery.profile.is_err()));
}
