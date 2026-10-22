//! The process-wide trigger-payload memo, observed through trigger
//! discovery ([`scan`]) and the thread-scoped [`PayloadMemoTally`].
//!
//! The memo is keyed by what payload resolution reads: the trigger, its
//! payload, the schema roots, the requesting context derived onto the
//! trigger's folder and onto each schema root, and the requesting context's
//! tree. Documents in different package folders of one repository share a
//! resolution; any request-side state that survives derivation keeps two
//! requests apart. Every fixture is a fresh temporary directory and counts are
//! per thread, so no test can hit another's entry. The eviction tests also
//! rely on nextest's process per test: another test's inserts in the same
//! process would change which entry is least recently used.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, PackageAreaFallback, RepositoryScopeCatalog, canonicalize_simplified};
use super::{PayloadMemoCounts, PayloadMemoTally, TriggerRegistry, scan};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: claudine.yaml\n";
const PAYLOAD: &str = "$schema:\n  model: string(required)\n";

/// A repository `{dir}/repo` with packages `pkg-a` and `pkg-b` (neither has a
/// `schemas/` folder, so every document searches the same roots), the
/// trigger in `repo/schemas/`, and empty fixture homes `{dir}/home-a` and
/// `{dir}/home-b`.
struct Fixture {
    _dir: tempfile::TempDir,
    dir: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let base = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let root = base.join("repo");
        for folder in [".git", "pkg-a", "pkg-b"] {
            std::fs::create_dir_all(root.join(folder)).expect("fixture folder");
        }
        for home in ["home-a", "home-b"] {
            std::fs::create_dir_all(base.join(home)).expect("fixture home");
        }
        let fixture = Self { _dir: dir, dir: base, root };
        fixture.write("schemas/claudine.trigger.yaml", TRIGGER);
        fixture.write("schemas/claudine.yaml", PAYLOAD);
        fixture
    }

    fn write(&self, relative: &str, contents: &str) {
        write(&self.root.join(relative), contents);
    }

    fn catalog(&self) -> RepositoryScopeCatalog {
        RepositoryScopeCatalog::new(
            &self.root,
            Vec::new(),
            vec![self.root.join("pkg-a"), self.root.join("pkg-b")],
            PackageAreaFallback::FirstComponent,
        )
        .expect("fixture catalog")
    }

    /// The repository context DMLS builds once per repository: launched at
    /// the repository root with its scope catalog.
    fn repository_context(&self) -> FileResolutionContext {
        self.repository_context_with(&self.root, Some(self.dir.join("home-a")), HashMap::new())
    }

    fn repository_context_with(
        &self,
        launch: &Path,
        home: Option<PathBuf>,
        env: HashMap<String, String>,
    ) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(launch, home, env).with_repository_scope_catalog(self.catalog())
    }

    fn document(&self, context: &FileResolutionContext, relative: &str) -> FileResolutionContext {
        context.for_source(self.root.join(relative))
    }
}

/// Scans `context` under a fresh tally and returns the registry with the
/// tally's counts.
fn tallied_scan(context: &FileResolutionContext) -> (TriggerRegistry, PayloadMemoCounts) {
    let tally = PayloadMemoTally::start();
    let registry = scan(context).unwrap_or_else(|error| panic!("trigger scan: {error}"));
    (registry, tally.counts())
}

const HIT: PayloadMemoCounts = PayloadMemoCounts { hits: 1, misses: 0 };
const MISS: PayloadMemoCounts = PayloadMemoCounts { hits: 0, misses: 1 };

fn payload_properties(registry: &TriggerRegistry) -> Vec<String> {
    registry.payloads[0].json_schema["properties"]
        .as_object()
        .expect("payload properties")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn documents_in_two_package_folders_share_one_resolution() {
    let fixture = Fixture::new();
    let repository = fixture.repository_context();
    let first_document = fixture.document(&repository, "pkg-a/doc.md");
    let second_document = fixture.document(&repository, "pkg-b/doc.md");
    assert_ne!(first_document.package_root(), second_document.package_root(), "the documents sit in different packages");

    let (first, first_counts) = tallied_scan(&first_document);
    let (second, second_counts) = tallied_scan(&second_document);

    assert_eq!(first_counts, MISS);
    assert_eq!(second_counts, HIT);
    assert!(Arc::ptr_eq(&first.payloads[0].json_schema, &second.payloads[0].json_schema));
    assert_eq!(first.payloads[0].dependencies, second.payloads[0].dependencies);
    assert_eq!(first.payloads[0].dependencies, [fixture.root.join("schemas/claudine.yaml")]);
    // Sharing stops at the payload: each registry keeps its own document context.
    assert_eq!(first.context(), &first_document);
    assert_eq!(second.context(), &second_document);
}

/// One edit per cell to the repository context of
/// [`documents_in_two_package_folders_share_one_resolution`]. The control
/// row proves the unedited context hits, so each edit is the reason its row
/// misses.
macro_rules! incompatible_request_cases {
    ($($name:ident: |$fixture:ident| $context:expr, $expected:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let $fixture = Fixture::new();
                let warm = $fixture.document(&$fixture.repository_context(), "pkg-a/doc.md");
                assert_eq!(tallied_scan(&warm).1, MISS, "the first scan resolves");

                let (registry, counts) = tallied_scan(&$context);

                assert_eq!(counts, $expected);
                assert_eq!(payload_properties(&registry), ["model"]);
            }
        )+
    };
}

incompatible_request_cases! {
    control_same_repository_context_hits: |fixture|
        fixture.document(&fixture.repository_context(), "pkg-b/doc.md"), HIT;
    a_different_request_boundary_misses: |fixture|
        FileResolutionContext::from_snapshot(&fixture.root, Some(fixture.dir.join("home-a")), HashMap::new())
            .with_repository_root(&fixture.root)
            .for_source(fixture.root.join("pkg-b/doc.md")),
        MISS;
    a_different_launch_scope_misses: |fixture| {
        let launched_in_package = fixture.repository_context_with(
            &fixture.root.join("pkg-b"),
            Some(fixture.dir.join("home-a")),
            HashMap::new(),
        );
        fixture.document(&launched_in_package, "pkg-b/doc.md")
    }, MISS;
    a_different_home_misses: |fixture| {
        let other_home = fixture.repository_context_with(&fixture.root, Some(fixture.dir.join("home-b")), HashMap::new());
        fixture.document(&other_home, "pkg-b/doc.md")
    }, MISS;
    a_different_environment_misses: |fixture| {
        let env = HashMap::from([("FIXTURE_ONLY".to_string(), "1".to_string())]);
        let other_env = fixture.repository_context_with(&fixture.root, Some(fixture.dir.join("home-a")), env);
        fixture.document(&other_env, "pkg-b/doc.md")
    }, MISS;
    an_explicit_tree_control_misses: |fixture|
        fixture.document(&fixture.repository_context().allow_external_relative(), "pkg-b/doc.md"), MISS;
    an_explicit_base_dir_misses: |fixture|
        fixture.document(&fixture.repository_context().with_base_dir(&fixture.root), "pkg-b/doc.md"), MISS;
}

/// A requester whose own `cwd` escaped its tree carries that failure into
/// every derived context. Its scan must not reuse the passing requester's
/// payload: the memo records a miss, and resolution reports the escape.
#[test]
fn an_inherited_validation_failure_is_never_cleared_by_a_hit() {
    let fixture = Fixture::new();
    let warm = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    assert_eq!(tallied_scan(&warm).1, MISS);
    let escaped = fixture.repository_context().for_cwd(fixture.dir.join("home-b"));
    assert!(escaped.validate().is_err(), "the fixture context must fail validation");

    let tally = PayloadMemoTally::start();
    let result = scan(&escaped);

    assert_eq!(tally.counts().hits, 0, "a failing requester never reuses a payload: {result:?}");
}

/// A schema root outside the repository (`SCHEMAS_DIR`) is derived with
/// `for_trusted_external_cwd`; documents of one repository still share it,
/// and a different launch scope still keeps them apart.
#[test]
fn a_trusted_external_root_is_shared_within_one_repository_context() {
    let fixture = Fixture::new();
    std::fs::remove_dir_all(fixture.root.join("schemas")).expect("drop the repository root");
    let external = fixture.dir.join("external-schemas");
    write(&external.join("claudine.trigger.yaml"), TRIGGER);
    write(&external.join("claudine.yaml"), "$schema:\n  owner: string(required)\n");
    let env = || HashMap::from([("SCHEMAS_DIR".to_string(), external.to_string_lossy().into_owned())]);
    let repository = fixture.repository_context_with(&fixture.root, Some(fixture.dir.join("home-a")), env());

    let (first, first_counts) = tallied_scan(&fixture.document(&repository, "pkg-a/doc.md"));
    let (second, second_counts) = tallied_scan(&fixture.document(&repository, "pkg-b/doc.md"));
    let launched_in_package = fixture.repository_context_with(&fixture.root.join("pkg-b"), Some(fixture.dir.join("home-a")), env());
    let (third, third_counts) = tallied_scan(&fixture.document(&launched_in_package, "pkg-b/doc.md"));

    assert_eq!(first.roots.search_paths(), std::slice::from_ref(&external));
    assert_eq!((first_counts, second_counts, third_counts), (MISS, HIT, MISS));
    assert!(Arc::ptr_eq(&first.payloads[0].json_schema, &second.payloads[0].json_schema));
    assert_eq!(payload_properties(&third), ["owner"]);
}

/// An inline payload that imports a named type from a nested folder, whose
/// definition carries an example in that nested folder: resolution derives
/// contexts for folders other than the trigger's.
#[test]
fn an_inline_payload_with_nested_imports_and_examples_is_shared() {
    let fixture = Fixture::new();
    fixture.write(
        "schemas/claudine.trigger.yaml",
        "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  owner: \"Person@./types/person.yaml\"\n",
    );
    fixture.write("schemas/types/person.yaml", "$schema:\n  Person: \"string(example(./person-example.yaml))\"\n");
    fixture.write(
        "schemas/types/person-example.yaml",
        "kind: example\ninvocation: \"x\"\nreturns: y\ndescription: d\n",
    );
    let repository = fixture.repository_context();

    let (first, first_counts) = tallied_scan(&fixture.document(&repository, "pkg-a/doc.md"));
    let (second, second_counts) = tallied_scan(&fixture.document(&repository, "pkg-b/doc.md"));

    assert_eq!((first_counts, second_counts), (MISS, HIT));
    assert!(Arc::ptr_eq(&first.payloads[0].json_schema, &second.payloads[0].json_schema));
    assert_eq!(
        first.payloads[0].dependencies,
        [fixture.root.join("schemas/types/person-example.yaml"), fixture.root.join("schemas/types/person.yaml")]
    );
    assert_eq!(payload_properties(&second), ["owner"]);
}

/// A repository requester keeps its boundary tree for every derived folder,
/// while a requester outside any repository selects a tree per folder. They
/// search the same root and resolve the same payload, but never share it.
#[test]
fn requesters_with_different_tree_kinds_never_share() {
    let fixture = Fixture::new();
    let boundary = fixture.repository_context().for_source(fixture.root.join("doc.md"));
    let fallback = FileResolutionContext::from_snapshot(&fixture.root, Some(fixture.dir.join("home-a")), HashMap::new())
        .for_source(fixture.root.join("doc.md"));
    assert_ne!(boundary.base_dir_origin(), fallback.base_dir_origin(), "the fixture needs two tree kinds");

    let (first, first_counts) = tallied_scan(&boundary);
    let (second, second_counts) = tallied_scan(&fallback);

    assert_eq!(first.roots.search_paths(), second.roots.search_paths());
    assert_eq!((first_counts, second_counts), (MISS, MISS));
    assert_eq!(payload_properties(&second), ["model"]);
}

/// 18 distinct keys (6 package folders × 3 triggers, each folder with its own
/// root and payload), then a forced repeat scan of all six: every repeat
/// lookup is a hit.
#[test]
fn eighteen_distinct_keys_fit_and_a_repeat_scan_resolves_nothing() {
    let fixture = Fixture::new();
    std::fs::remove_dir_all(fixture.root.join("schemas")).expect("one root per package");
    let packages: Vec<PathBuf> = (0..6).map(|index| fixture.root.join(format!("pkg-{index}"))).collect();
    for package in &packages {
        for trigger in ["claudine", "inline-compose", "sequence"] {
            write(
                &package.join(format!("schemas/{trigger}.trigger.yaml")),
                &format!("kind: schema-trigger\nmatch:\n  {trigger}: string(required)\n$schema: {trigger}.yaml\n"),
            );
            write(&package.join(format!("schemas/{trigger}.yaml")), PAYLOAD);
        }
    }
    let catalog = RepositoryScopeCatalog::new(&fixture.root, Vec::new(), packages.clone(), PackageAreaFallback::None)
        .expect("fixture catalog");
    let repository = FileResolutionContext::from_snapshot(&fixture.root, None, HashMap::new())
        .with_repository_scope_catalog(catalog);
    let documents: Vec<_> = packages.iter().map(|package| repository.for_source(package.join("doc.md"))).collect();

    let cold = PayloadMemoTally::start();
    for document in &documents {
        assert_eq!(scan(document).expect("cold scan").payloads.len(), 3);
    }
    assert_eq!(cold.counts(), PayloadMemoCounts { hits: 0, misses: 18 });
    drop(cold);

    let (_, repeat) = {
        let tally = PayloadMemoTally::start();
        for document in &documents {
            scan(document).expect("repeat scan");
        }
        ((), tally.counts())
    };
    assert_eq!(repeat, PayloadMemoCounts { hits: 18, misses: 0 });
}

/// The memo holds 32 entries. Reusing the oldest entry protects it, so
/// filling past the bound evicts the next-oldest instead.
#[test]
fn a_hit_protects_an_entry_and_the_least_recently_used_is_evicted() {
    let fixture = Fixture::new();
    std::fs::remove_dir_all(fixture.root.join("schemas")).expect("one root per package");
    let packages: Vec<PathBuf> = (0..33).map(|index| fixture.root.join(format!("pkg-{index:02}"))).collect();
    for package in &packages {
        write(&package.join("schemas/claudine.trigger.yaml"), TRIGGER);
        write(&package.join("schemas/claudine.yaml"), PAYLOAD);
    }
    let catalog = RepositoryScopeCatalog::new(&fixture.root, Vec::new(), packages.clone(), PackageAreaFallback::None)
        .expect("fixture catalog");
    let repository = FileResolutionContext::from_snapshot(&fixture.root, None, HashMap::new())
        .with_repository_scope_catalog(catalog);
    let document = |index: usize| repository.for_source(packages[index].join("doc.md"));

    for index in 0..32 {
        assert_eq!(tallied_scan(&document(index)).1, MISS, "filling entry {index}");
    }
    assert_eq!(tallied_scan(&document(0)).1, HIT, "the oldest entry is still held at the bound");
    assert_eq!(tallied_scan(&document(32)).1, MISS, "the 33rd key evicts one entry");

    assert_eq!(tallied_scan(&document(0)).1, HIT, "the refreshed entry survived");
    assert_eq!(tallied_scan(&document(1)).1, MISS, "the least recently used entry was evicted");
}

#[test]
fn an_edited_dependency_is_resolved_again() {
    let fixture = Fixture::new();
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    assert_eq!(tallied_scan(&document).1, MISS);

    fixture.write("schemas/claudine.yaml", "$schema:\n  owner: string(required)\n");
    let (registry, counts) = tallied_scan(&document);

    assert_eq!(counts, MISS);
    assert_eq!(payload_properties(&registry), ["owner"]);
}

#[test]
fn a_file_added_to_a_schema_root_is_resolved_again() {
    let fixture = Fixture::new();
    let document = fixture.document(&fixture.repository_context(), "pkg-b/doc.md");
    assert_eq!(tallied_scan(&document).1, MISS);

    fixture.write("schemas/unrelated.yaml", "$schema:\n  other: string\n");
    let (registry, counts) = tallied_scan(&document);

    assert_eq!(counts, MISS);
    assert_eq!(payload_properties(&registry), ["model"]);
}

/// A failed resolution counts as a miss and is never kept, so the next
/// lookup tries again.
#[test]
fn a_failed_resolution_is_a_miss_and_is_not_kept() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.root.join("schemas/claudine.yaml")).expect("drop the payload");
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");

    for attempt in 0..2 {
        let tally = PayloadMemoTally::start();
        assert!(scan(&document).is_err(), "attempt {attempt} must fail");
        assert_eq!(tally.counts(), MISS, "attempt {attempt}");
    }
}

/// Only the innermost tally counts: an interrupting unit of work is
/// excluded from the outer tally, and work on another thread is never
/// counted here.
#[test]
fn a_nested_tally_excludes_the_interruption_from_the_outer_one() {
    let fixture = Fixture::new();
    let first_document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    let second_document = fixture.document(&fixture.repository_context(), "pkg-b/doc.md");

    let pass = PayloadMemoTally::start();
    scan(&first_document).expect("pass scan");
    {
        let interruption = PayloadMemoTally::start();
        scan(&second_document).expect("interrupting scan");
        assert_eq!(interruption.counts(), HIT);
    }
    let other_thread = std::thread::spawn({
        let first_document = first_document.clone();
        move || scan(&first_document).map(|_| ())
    });
    other_thread.join().expect("other thread").expect("other-thread scan");

    assert_eq!(pass.counts(), MISS);
}

/// A registry reports the locations its payload searched, including an
/// absent nearer candidate, apart from the files it depends on; a memo hit
/// reports the same set.
#[test]
fn a_registry_reports_searched_locations_apart_from_its_dependencies() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", "$schema:\n  Fields: string\n");
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");

    let (registry, counts) = tallied_scan(&document);
    assert_eq!(counts, MISS);

    let searched: Vec<&Path> = registry.searched_locations().collect();
    let nearer = fixture.root.join("schemas/shared/types.yaml");
    for expected in [&fixture.root.join("schemas/claudine.yaml"), &nearer, &fixture.root.join("shared/types.yaml")] {
        assert!(searched.contains(&expected.as_path()), "{expected:?} missing from {searched:?}");
    }
    assert!(
        registry.dependencies().all(|dependency| dependency != nearer),
        "an absent candidate is never a dependency file"
    );

    let (repeat, counts) = tallied_scan(&document);
    assert_eq!(counts, HIT);
    assert_eq!(repeat.searched_locations().collect::<Vec<_>>(), searched);
}

/// The non-null JSON Schema type the optional payload property `property`
/// resolved to.
fn property_type(registry: &TriggerRegistry, property: &str) -> String {
    let schema = &registry.payloads[0].json_schema["properties"][property];
    schema["anyOf"]
        .as_array()
        .expect("an optional property is a null union")
        .iter()
        .map(|arm| arm["type"].as_str().expect("typed arm"))
        .find(|kind| *kind != "null")
        .expect("a non-null arm")
        .to_string()
}

const SHARED_FIELDS: &str = "$schema:\n  Fields: string\n";
const NEARER_FIELDS: &str = "$schema:\n  Fields: number\n";

/// The concrete stale case: `claudine.yaml` in `schemas/` imports
/// `Fields@shared/types.yaml`, which resolves in the authoring folder first
/// (`schemas/shared/types.yaml`, absent with its folder) and then the
/// repository root (`shared/types.yaml`). Creating the nearer file must win
/// over a held payload, and deleting it must fall back again.
#[test]
fn a_nearer_import_candidate_created_in_an_absent_folder_is_resolved_again() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    let shared = fixture.root.join("shared/types.yaml");
    let nearer = fixture.root.join("schemas/shared/types.yaml");

    let (first, first_counts) = tallied_scan(&document);
    let (repeat, repeat_counts) = tallied_scan(&document);
    assert_eq!((first_counts, repeat_counts), (MISS, HIT));
    assert!(Arc::ptr_eq(&first.payloads[0].json_schema, &repeat.payloads[0].json_schema));
    assert_eq!(property_type(&repeat, "fields"), "string");
    assert!(repeat.dependencies().any(|dependency| dependency == shared));

    fixture.write("schemas/shared/types.yaml", NEARER_FIELDS);
    let (shadowed, shadowed_counts) = tallied_scan(&document);
    assert_eq!(shadowed_counts, MISS);
    assert_eq!(property_type(&shadowed, "fields"), "number");
    assert!(shadowed.dependencies().any(|dependency| dependency == nearer));
    assert!(shadowed.dependencies().all(|dependency| dependency != shared), "the root file is no longer selected");
    assert_eq!(tallied_scan(&document).1, HIT, "the shadowed result is held");

    std::fs::remove_file(&nearer).expect("drop the nearer file");
    let (fallback, fallback_counts) = tallied_scan(&document);
    assert_eq!(fallback_counts, MISS);
    assert_eq!(property_type(&fallback, "fields"), "string");
    assert!(fallback.dependencies().any(|dependency| dependency == shared));
}

/// One reference shape per row, each with a nearer candidate in an absent
/// folder `deep/` below an existing `shared/` folder that holds no
/// dependency, so no schema-root or dependency-folder listing changes when
/// the candidate appears; only the searched-candidate observations can. The
/// first scan selects the repository-root file, a repeat hits, and creating
/// the nearer file (`$nearer`, with `$nearer_contents`) misses and selects it.
macro_rules! nearer_candidate_cases {
    ($($name:ident: files $files:expr, nearer $nearer:literal => $nearer_contents:expr, property $property:literal;)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = Fixture::new();
                fixture.write("schemas/shared/readme.md", "unrelated\n");
                for (relative, contents) in $files {
                    fixture.write(relative, contents);
                }
                let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
                assert_eq!(tallied_scan(&document).1, MISS);
                let (held, held_counts) = tallied_scan(&document);
                assert_eq!(held_counts, HIT);
                let nearer = fixture.root.join($nearer);
                assert!(held.dependencies().all(|dependency| dependency != nearer));
                assert!(held.searched_locations().any(|searched| searched == nearer), "{nearer:?} was searched");

                fixture.write($nearer, $nearer_contents);
                let (registry, counts) = tallied_scan(&document);

                assert_eq!(counts, MISS);
                assert!(registry.dependencies().any(|dependency| dependency == nearer), "{nearer:?} is selected");
                assert_eq!(payload_properties(&registry), [$property]);
            }
        )+
    };
}

nearer_candidate_cases! {
    a_nearer_payload_path_is_resolved_again:
        files [
            ("schemas/claudine.trigger.yaml", "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: shared/deep/payload.yaml\n"),
            ("shared/deep/payload.yaml", PAYLOAD),
        ],
        nearer "schemas/shared/deep/payload.yaml" => "$schema:\n  owner: string(required)\n",
        property "owner";
    a_nearer_import_is_resolved_again:
        files [
            ("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/deep/types.yaml\n"),
            ("shared/deep/types.yaml", SHARED_FIELDS),
        ],
        nearer "schemas/shared/deep/types.yaml" => NEARER_FIELDS,
        property "fields";
    a_nearer_transitive_import_is_resolved_again:
        files [
            ("schemas/claudine.yaml", "$schema:\n  fields: Fields@./types/fields.yaml\n"),
            ("schemas/types/fields.yaml", "$schema:\n  Fields:\n    inner: Inner@shared/deep/inner.yaml\n"),
            ("schemas/types/shared/readme.md", "unrelated\n"),
            ("shared/deep/inner.yaml", "$schema:\n  Inner: string\n"),
        ],
        nearer "schemas/types/shared/deep/inner.yaml" => "$schema:\n  Inner: number\n",
        property "fields";
    a_nearer_example_file_is_resolved_again:
        files [
            ("schemas/claudine.yaml", "$schema:\n  model: \"string(example(shared/deep/model.yaml))\"\n"),
            ("shared/deep/model.yaml", "kind: example\ninvocation: \"x\"\nreturns: y\ndescription: root\n"),
        ],
        nearer "schemas/shared/deep/model.yaml" => "kind: example\ninvocation: \"x\"\nreturns: y\ndescription: nearer\n",
        property "model";
}

/// An explicit `./` reference names one location: a file of the same name
/// at the repository root is never a fallback, and the failure is not held.
#[test]
fn an_explicit_relative_reference_gains_no_fallback() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@./shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");

    for attempt in 0..2 {
        let tally = PayloadMemoTally::start();
        assert!(scan(&document).is_err(), "attempt {attempt}: `./` must not fall back to the root");
        assert_eq!(tally.counts(), MISS, "attempt {attempt}");
    }

    fixture.write("schemas/shared/types.yaml", NEARER_FIELDS);
    let (registry, counts) = tallied_scan(&document);
    assert_eq!(counts, MISS);
    assert_eq!(property_type(&registry, "fields"), "number");
}

/// The nearer candidate's folder already exists (with an unrelated file), so
/// only its listing changes when the candidate appears.
#[test]
fn a_nearer_candidate_in_an_existing_folder_is_resolved_again() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    fixture.write("schemas/shared/readme.md", "unrelated\n");
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    assert_eq!(tallied_scan(&document).1, MISS);
    assert_eq!(tallied_scan(&document).1, HIT);

    fixture.write("schemas/shared/types.yaml", NEARER_FIELDS);
    let (registry, counts) = tallied_scan(&document);

    assert_eq!(counts, MISS);
    assert_eq!(property_type(&registry, "fields"), "number");
}

/// A nearer candidate that is a directory is skipped. Replacing it with a
/// regular file of the same name leaves its folder's entry names unchanged,
/// so only the candidate's kind observation can notice the swap.
#[test]
fn a_nearer_candidate_swapped_from_directory_to_file_is_resolved_again() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    let nearer = fixture.root.join("schemas/shared/types.yaml");
    std::fs::create_dir_all(&nearer).expect("nearer candidate as a directory");
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    let (held, _) = tallied_scan(&document);
    assert_eq!(property_type(&held, "fields"), "string");
    assert_eq!(tallied_scan(&document).1, HIT);

    std::fs::remove_dir(&nearer).expect("drop the directory");
    std::fs::write(&nearer, NEARER_FIELDS).expect("same name as a file");
    let (registry, counts) = tallied_scan(&document);

    assert_eq!(counts, MISS);
    assert_eq!(property_type(&registry, "fields"), "number");
}

/// A failed observation of a searched folder (other than its absence)
/// neither validates the held entry nor seeds a new one; once the folder
/// reads again, the next resolution is kept.
#[test]
fn a_failed_observation_is_never_a_hit() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    assert_eq!(tallied_scan(&document).1, MISS);
    assert_eq!(tallied_scan(&document).1, HIT, "control: the unedited state hits");

    {
        let _failing = super::assemble::fail_observing(&fixture.root.join("schemas/shared"));
        for attempt in 0..2 {
            let (registry, counts) = tallied_scan(&document);
            assert_eq!(counts, MISS, "attempt {attempt}");
            assert_eq!(property_type(&registry, "fields"), "string", "attempt {attempt} still resolves");
        }
    }

    assert_eq!(tallied_scan(&document).1, MISS, "nothing was kept while observation failed");
    assert_eq!(tallied_scan(&document).1, HIT);
}

/// Files outside every searched folder, and new content in a searched folder
/// that changes no listed name or dependency, keep the held payload.
#[test]
fn unrelated_edits_keep_the_held_payload() {
    let fixture = Fixture::new();
    fixture.write("schemas/claudine.yaml", "$schema:\n  fields: Fields@shared/types.yaml\n");
    fixture.write("shared/types.yaml", SHARED_FIELDS);
    fixture.write("shared/notes.md", "first\n");
    let document = fixture.document(&fixture.repository_context(), "pkg-a/doc.md");
    let (first, first_counts) = tallied_scan(&document);
    assert_eq!(first_counts, MISS);

    fixture.write("pkg-a/unrelated.yaml", "$schema:\n  other: string\n");
    fixture.write("pkg-b/nested/unrelated.yaml", "$schema:\n  other: string\n");
    fixture.write("shared/notes.md", "second\n");
    let (repeat, counts) = tallied_scan(&document);

    assert_eq!(counts, HIT);
    assert!(Arc::ptr_eq(&first.payloads[0].json_schema, &repeat.payloads[0].json_schema));
}
