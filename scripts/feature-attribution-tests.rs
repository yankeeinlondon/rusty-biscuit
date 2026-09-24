use super::*;

use std::path::PathBuf;

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("scripts/ has a parent")
        .to_path_buf()
}

/// The feature-attribution fixture workspace.
fn fixture() -> PathBuf {
    repo_root().join("scripts/ci/fixtures/feature-attribution")
}

/// The fixture's sidecar table.
fn fixture_sidecars() -> SidecarTable {
    let text = fs::read_to_string(repo_root().join("scripts/ci/fixtures/feature-attribution/sidecars.json"))
        .expect("the fixture sidecar table is readable");
    serde_json::from_str(&text).expect("the fixture sidecar table parses")
}

/// Attribute `owners` over a checked-in fixture workspace, optionally joined
/// with a timed pass and modeling an alignment.
fn fixture_report_aligned(workspace: &Path, owners: &[&str], events: Option<&Path>, aligned: &[&str]) -> Report {
    let owners: Vec<String> = owners.iter().map(|owner| (*owner).to_owned()).collect();
    let target = host_triple().expect("rustc reports a host triple");
    let sidecars = fixture_sidecars();
    let selection = collect(workspace, &target, &owners, Some(&sidecars)).expect("the fixture resolves");
    let compiles = events.map(|dir| read_compiles(dir, &selection.members).expect("events parse"));
    attribute(
        &target,
        &selection.owners,
        &selection.invocations,
        &selection.members,
        &selection.tables,
        compiles.as_deref(),
        &aligned.iter().map(|name| (*name).to_owned()).collect(),
    )
    .expect("attribution succeeds")
}

fn fixture_report(workspace: &Path, owners: &[&str], events: Option<&Path>) -> Report {
    fixture_report_aligned(workspace, owners, events, &[])
}

fn causes_fixture() -> Report {
    fixture_report(
        &repo_root().join("scripts/ci/fixtures/feature-attribution"),
        // Deliberately unsorted: the producer's order is by package name.
        &["fa-owner-wsdep", "fa-owner-third", "fa-core", "fa-owner-plain", "fa-owner-own"],
        None,
    )
}

fn crate_report<'a>(report: &'a Report, name: &str) -> &'a CrateReport {
    report
        .crates
        .iter()
        .find(|krate| krate.krate == name)
        .unwrap_or_else(|| panic!("{name} is in the report: {report:#?}"))
}

fn cause(category: Category, krate: &str, added: &[&str], removed: &[&str]) -> Cause {
    Cause {
        category,
        krate: krate.to_owned(),
        version: "0.1.0".to_owned(),
        host: false,
        added: added.iter().map(|feature| (*feature).to_owned()).collect(),
        removed: removed.iter().map(|feature| (*feature).to_owned()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

const TREE: &str = "\
root v0.1.0 (/ws/root)|default
|-- libc v0.2.186|default,std
|-- serde v1.0.228|default,derive,std
|   `-- serde_derive v1.0.228 (proc-macro)|default
|       `-- proc-macro2 v1.0.106|proc-macro,span-locations
|           `-- unicode-ident v1.0.19|
|-- widget v0.1.0 (/ws/widget)|
|   |-- libc v0.2.186|default,std
|   `-- serde v1.0.228|default,derive,std (*)
|   [build-dependencies]
|   `-- cc v1.2.0|
|       `-- libc v0.2.186|default,std
[dev-dependencies]
`-- serde v1.0.228|default,derive,std (*)
";

#[test]
fn parse_tree_links_repeated_and_marked_packages_to_one_node() {
    let tree = parse_tree(TREE).expect("parses");
    let find = |name: &str, host: bool| {
        tree.nodes
            .iter()
            .position(|node| node.package.name == name && node.host == host)
            .unwrap_or_else(|| panic!("{name} (host={host}) is a node"))
    };

    let serde = find("serde", false);
    let widget = find("widget", false);
    assert!(tree.nodes[0].deps.contains(&(serde, EdgeKind::Normal)));
    assert!(tree.nodes[0].deps.contains(&(serde, EdgeKind::Dev)), "the dev header applies");
    assert!(tree.nodes[widget].deps.contains(&(serde, EdgeKind::Normal)), "(*) resolves to the first node");
    assert_eq!(
        tree.nodes.iter().filter(|node| node.package.name == "serde").count(),
        1,
        "one serde node, not one per occurrence"
    );
    // The repeated, unmarked target-side `libc` is one node too.
    assert_eq!(tree.nodes[widget].deps.iter().filter(|(dep, _)| *dep == find("libc", false)).count(), 1);
}

#[test]
fn parse_tree_marks_proc_macros_and_build_dependencies_as_host_side() {
    let tree = parse_tree(TREE).expect("parses");
    let node = |name: &str| {
        tree.nodes
            .iter()
            .filter(|node| node.package.name == name)
            .collect::<Vec<_>>()
    };

    assert!(node("serde_derive")[0].host, "a proc-macro is compiled for the host");
    assert!(node("proc-macro2")[0].host, "a proc-macro's dependency is host-side");
    assert!(node("cc")[0].host, "a build dependency is host-side");
    let libc = node("libc");
    assert_eq!(libc.len(), 2, "libc under a build dependency is a separate host node");
    assert_eq!(libc.iter().filter(|node| node.host).count(), 1);
    assert_eq!(
        node("proc-macro2")[0].features,
        BTreeSet::from(["proc-macro".to_owned(), "span-locations".to_owned()])
    );
}

#[test]
fn parse_package_reads_version_source_and_proc_macro_marker() {
    let (package, proc_macro) = parse_package("serde_derive v1.0.228 (proc-macro)").expect("parses");
    assert_eq!(
        (package.name.as_str(), package.version.as_str(), package.source.as_str(), proc_macro),
        ("serde_derive", "1.0.228", "", true)
    );
    let (package, proc_macro) = parse_package("widget v0.1.0 (/ws/a b/widget)").expect("parses");
    assert_eq!(
        (package.version.as_str(), package.source.as_str(), proc_macro),
        ("0.1.0", "/ws/a b/widget", false)
    );
}

#[test]
fn parse_tree_rejects_malformed_input() {
    assert!(parse_tree("").is_err(), "no packages");
    assert!(parse_tree("root v0.1.0\n").is_err(), "no feature separator");
    assert!(parse_tree("root v0.1.0|\n|   |-- deep v1.0.0|\n").is_err(), "skipped level");
    assert!(parse_tree("root v0.1.0|\n[target-dependencies]\n").is_err(), "unknown header");
}

// ---------------------------------------------------------------------------
// Attribution over the fixture workspace
// ---------------------------------------------------------------------------

#[test]
fn own_feature_cause_names_the_crate_and_flag() {
    let report = causes_fixture();
    let core = crate_report(&report, "fa-core");
    let own = core
        .configurations
        .iter()
        .find(|config| config.first_owner == "fa-owner-own")
        .expect("fa-owner-own builds its own fa-core configuration");
    assert!(own.features.contains(&"fast".to_owned()));
    assert_eq!(own.causes, vec![cause(Category::Own, "fa-core", &["fast"], &[])]);
}

#[test]
fn workspace_dependency_cause_is_the_dependency_not_the_dependent() {
    let report = causes_fixture();
    let core = crate_report(&report, "fa-core");
    let wsdep = core
        .configurations
        .iter()
        .find(|config| config.first_owner == "fa-owner-wsdep")
        .expect("fa-owner-wsdep re-identifies fa-core");
    assert_eq!(
        wsdep.causes,
        vec![cause(Category::WorkspaceDependency, "fa-util", &["wide"], &[])],
        "fa-core's own features match; the root is fa-util's"
    );
    let util = crate_report(&report, "fa-util");
    let wide = util
        .configurations
        .iter()
        .find(|config| config.first_owner == "fa-owner-wsdep")
        .expect("fa-util/wide is its own configuration");
    assert_eq!(wide.causes, vec![cause(Category::Own, "fa-util", &["wide"], &[])]);
}

#[test]
fn third_party_cause_reaches_every_workspace_crate_above_it() {
    let report = causes_fixture();
    for name in ["fa-core", "fa-util"] {
        let third = crate_report(&report, name)
            .configurations
            .iter()
            .find(|config| config.first_owner == "fa-owner-third")
            .unwrap_or_else(|| panic!("a dev-dependency flag re-identifies {name}"));
        assert_eq!(
            third.causes,
            vec![cause(Category::ThirdParty, "fa-base", &["extra"], &[])],
            "{name}: compared with its nearest earlier configuration"
        );
    }

    let ranked = report
        .causes
        .iter()
        .find(|ranked| ranked.cause.krate == "fa-base")
        .expect("the third-party cause is ranked");
    assert_eq!(ranked.configurations, 2);
    assert_eq!(ranked.crates, vec!["fa-core".to_owned(), "fa-util".to_owned()]);
}

#[test]
fn every_configuration_beyond_the_first_is_explained() {
    let report = causes_fixture();
    assert_eq!(report.totals.workspace_crates, 6, "four owner packages, fa-core, fa-util: {report:#?}");
    for krate in &report.crates {
        for (index, config) in krate.configurations.iter().enumerate() {
            assert_eq!(index == 0, config.causes.is_empty(), "{}#{index}: {config:?}", krate.krate);
            assert_eq!(index == 0, config.compared_with.is_none());
        }
    }
    // fa-core: first (fa-core's own archive), +fast, +extra beneath, +wide beneath.
    assert_eq!(crate_report(&report, "fa-core").configurations.len(), 4);
    assert_eq!(crate_report(&report, "fa-util").configurations.len(), 3);
    assert_eq!(report.totals.divergent_configurations, 5);
    assert_eq!(report.totals.divergent_by_origin.get("third-party"), Some(&2));
    assert_eq!(report.totals.divergent_by_origin.get("workspace-own"), Some(&3));
}

#[test]
fn a_root_package_shares_the_configuration_its_dependents_build() {
    // `fa-core` declares `default = []`. Its own archive and `fa-owner-plain`
    // must resolve the same unit: a comparison that read the root's display
    // as a feature difference would fabricate a configuration here.
    let report = causes_fixture();
    let first = &crate_report(&report, "fa-core").configurations[0];
    assert_eq!(first.first_owner, "fa-core", "owners are modeled in producer order");
    assert_eq!(first.features, vec!["default".to_owned()]);
    assert!(
        first.owners.contains(&"fa-owner-plain".to_owned()),
        "fa-owner-plain reuses the root's configuration: {first:?}"
    );
}

#[test]
fn two_versions_of_one_crate_are_not_a_divergence() {
    // `fa-twin` 1.0.0 (with `x`) and 2.0.0 (without) are two packages; keyed on
    // the name alone they would look like one crate with two feature sets.
    let report = causes_fixture();
    assert!(
        report.third_party_divergence.iter().all(|row| row.krate != "fa-twin"),
        "{:#?}",
        report.third_party_divergence
    );
    let base = report
        .third_party_divergence
        .iter()
        .find(|row| row.krate == "fa-base")
        .expect("fa-base resolves two ways");
    assert_eq!(base.variants.len(), 2);
    assert_eq!(base.variants[1].features, vec!["extra".to_owned()]);
    assert_eq!(base.variants[1].owners, vec!["fa-owner-third".to_owned()]);
}

#[test]
fn a_deliberately_divergent_dependency_is_two_configurations() {
    // Mirrors `one_owner_tree_shares_a_dependency_compile_without_unifying_features`
    // over the same fixture: `common` compiles once for both packages,
    // `divergent` twice, and the second is explained by its own feature.
    let report = fixture_report(
        &repo_root().join("scripts/ci/fixtures/shared-deps"),
        &["shared-deps-beta", "shared-deps-alpha"],
        None,
    );
    let common = crate_report(&report, "shared-deps-common");
    assert_eq!(common.configurations.len(), 1);
    assert_eq!(
        common.configurations[0].owners,
        vec!["shared-deps-alpha".to_owned(), "shared-deps-beta".to_owned()]
    );

    let divergent = crate_report(&report, "shared-deps-divergent");
    assert_eq!(divergent.configurations.len(), 2);
    assert_eq!(divergent.configurations[0].first_owner, "shared-deps-alpha");
    let second = &divergent.configurations[1];
    assert_eq!(second.first_owner, "shared-deps-beta");
    assert_eq!(second.compared_with, Some(0));
    assert_eq!(
        second.causes,
        vec![Cause {
            category: Category::Own,
            krate: "shared-deps-divergent".to_owned(),
            version: "0.1.0".to_owned(),
            host: false,
            added: Vec::new(),
            removed: vec!["extra".to_owned()],
        }]
    );
}

// ---------------------------------------------------------------------------
// Timed-pass join
// ---------------------------------------------------------------------------

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "feature-attribution-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Write a target-side compile event that starts at `index` milliseconds.
fn write_event(dir: &Path, index: usize, owner: &str, package: &str, crate_types: &[&str], ms: u64) {
    let event = serde_json::json!({
        "schema_version": 1,
        "package": owner,
        "configuration": "timed",
        "crate_name": package.replace('-', "_"),
        "cargo_package": package,
        "primary": false,
        "crate_types": crate_types,
        "target": "t",
        "probe": false,
        "argv_digest": format!("{index:016x}"),
        "started_ms": index,
        "finished_ms": index as u64 + ms,
        "duration_ms": ms,
        "pid": 1,
    });
    fs::write(dir.join(format!("{index:04}.json")), event.to_string()).expect("event written");
}

#[test]
fn timed_pass_seconds_are_charged_to_the_configurations_an_owner_built_first() {
    let events = Scratch::new("events");
    write_event(&events.0, 0, "fa-core", "fa-core", &["lib"], 4_000);
    write_event(&events.0, 1, "fa-core", "fa-util", &["lib"], 2_000);
    // The owner's own test harness is never divergence.
    write_event(&events.0, 2, "fa-core", "fa-core", &[], 9_000);
    write_event(&events.0, 3, "fa-owner-third", "fa-core", &["lib"], 3_000);
    write_event(&events.0, 4, "fa-owner-third", "fa-util", &["lib"], 1_000);
    // fa-owner-plain reuses fa-core's configuration: a compile here is one
    // the model cannot explain, and must be reported rather than absorbed.
    write_event(&events.0, 5, "fa-owner-plain", "fa-util", &["lib"], 500);

    let report = fixture_report(
        &repo_root().join("scripts/ci/fixtures/feature-attribution"),
        &["fa-core", "fa-owner-plain", "fa-owner-third"],
        Some(&events.0),
    );

    let core = crate_report(&report, "fa-core");
    assert_eq!(core.configurations[0].seconds, Some(4.0), "the harness is excluded");
    assert_eq!(core.configurations[1].seconds, Some(3.0));
    assert_eq!(report.totals.seconds_first, Some(6.0), "fa-core 4s + fa-util 2s");
    assert_eq!(report.totals.seconds_divergent, Some(4.0));
    assert_eq!(report.totals.seconds_by_category.get(&Category::ThirdParty), Some(&4.0));

    let ranked = &report.causes[0];
    assert_eq!(ranked.cause.krate, "fa-base");
    assert_eq!(ranked.seconds, 4.0);
    assert_eq!((ranked.sole_seconds, ranked.involved_seconds), (4.0, 4.0), "the only cause of both");

    assert_eq!(report.unexplained_compiles.len(), 1, "{:#?}", report.unexplained_compiles);
    assert_eq!(report.unexplained_compiles[0].owner, "fa-owner-plain");
    assert_eq!(report.unexplained_compiles[0].krate, "fa-util");
}

#[test]
fn the_json_report_round_trips() {
    let report = causes_fixture();
    let text = serde_json::to_string(&report).expect("serializes");
    let back: Report = serde_json::from_str(&text).expect("deserializes");
    assert_eq!(serde_json::to_string(&back).expect("serializes again"), text);
    assert_eq!(back.schema_version, REPORT_SCHEMA_VERSION);
}

#[test]
fn the_rendered_report_names_each_cause() {
    let report = causes_fixture();
    let rendered = render(&report, &Terminal::new());
    for expected in ["fa-base +extra", "fa-core +fast", "fa-util +wide", "first"] {
        assert!(rendered.contains(expected), "missing {expected:?} in:\n{rendered}");
    }
}

#[test]
fn owners_are_required() {
    let error = parse_args(&[]).expect_err("no owners");
    assert!(error.to_string().contains("--owners is required"));
    let error = parse_args(&["--target".to_owned()]).expect_err("a flag without its value");
    assert!(error.to_string().contains("--target needs a value"));
}

#[test]
fn co_causes_split_seconds_and_bound_what_each_removes() {
    let owner = |name: &str, a: &str, b: &str| Invocation {
        owner: name.to_owned(),
        label: "archive".to_owned(),
        tree: parse_tree(&format!(
            "{name} v0.1.0 (/ws/{name})|\n`-- w v0.1.0 (/ws/w)|\n    |-- a v1.0.0|{a}\n    `-- b v1.0.0|{b}\n"
        ))
        .expect("parses"),
    };
    let invocations = [owner("o1", "", ""), owner("o2", "x", "y")];
    let workspace: BTreeSet<PackageKey> = ["o1", "o2", "w"]
        .into_iter()
        .map(|name| PackageKey {
            name: name.to_owned(),
            version: "0.1.0".to_owned(),
            source: format!("/ws/{name}"),
        })
        .collect();
    let compile = |owner: &str, started_ms: u64, seconds: f64| Compile {
        owner: owner.to_owned(),
        package: "w".to_owned(),
        host: false,
        build_script: false,
        started_ms,
        seconds,
    };
    let compiles = [compile("o1", 0, 1.0), compile("o2", 1, 6.0)];
    let timings = Some(compiles.as_slice());
    let tables = FeatureTables::new();
    let owners = vec![("o1".to_owned(), Vec::new()), ("o2".to_owned(), Vec::new())];

    let report = attribute("t", &owners, &invocations, &workspace, &tables, timings, &BTreeSet::new())
        .expect("attributes");

    assert_eq!(report.causes.len(), 2, "{:#?}", report.causes);
    for ranked in &report.causes {
        assert_eq!(ranked.cause.category, Category::ThirdParty);
        assert_eq!(ranked.seconds, 3.0, "{}: an even share", ranked.cause.flag());
        assert_eq!(ranked.sole_seconds, 0.0, "{}: removing it alone saves nothing", ranked.cause.flag());
        assert_eq!(ranked.involved_seconds, 6.0);
    }
    assert_eq!(report.totals.seconds_divergent, Some(6.0));
    assert_eq!(report.totals.divergent_by_origin.get("third-party"), Some(&1));
    assert_eq!(report.totals.seconds_removed, None, "no what-if was asked for");

    // Aligning one co-cause leaves the configuration divergent on the other.
    let one = BTreeSet::from(["a".to_owned()]);
    let report = attribute("t", &owners, &invocations, &workspace, &tables, timings, &one).expect("attributes");
    assert_eq!(report.totals.divergent_configurations, 1);
    assert_eq!(report.causes.len(), 1);
    assert_eq!(report.causes[0].cause.krate, "b");
    assert_eq!(report.causes[0].sole_seconds, 6.0, "b is now the only cause");
    assert_eq!(report.totals.seconds_removed, Some(0.0));

    // Aligning both collapses it: o2 reuses o1's `w`, and its 6s go.
    let both = BTreeSet::from(["a".to_owned(), "b".to_owned()]);
    let report = attribute("t", &owners, &invocations, &workspace, &tables, timings, &both).expect("attributes");
    assert_eq!(report.totals.divergent_configurations, 0);
    assert_eq!(report.totals.seconds_removed, Some(6.0));
    assert!(report.unexplained_compiles.is_empty());
    assert_eq!(report.aligned, vec!["a".to_owned(), "b".to_owned()]);
}

#[test]
fn a_workspace_crate_is_never_aligned() {
    let tree = parse_tree("o v0.1.0 (/ws/o)|\n`-- w v0.1.0 (/ws/w)|x\n").expect("parses");
    let invocations = [Invocation { owner: "o".to_owned(), label: "archive".to_owned(), tree }];
    let workspace: BTreeSet<PackageKey> = ["o", "w"]
        .into_iter()
        .map(|name| PackageKey {
            name: name.to_owned(),
            version: "0.1.0".to_owned(),
            source: format!("/ws/{name}"),
        })
        .collect();
    let owners = vec![("o".to_owned(), Vec::new())];
    let tables = FeatureTables::new();
    let error = attribute("t", &owners, &invocations, &workspace, &tables, None, &BTreeSet::from(["w".to_owned()]))
        .expect_err("aligning a workspace crate is refused");
    assert!(error.to_string().contains("never aligned"), "{error}");
}

#[test]
fn aligning_a_fixture_flag_collapses_the_configurations_it_caused() {
    // End to end over real `cargo tree` output: aligning `fa-base` removes
    // exactly the two configurations `fa-owner-third`'s dev-dependency caused.
    let workspace = repo_root().join("scripts/ci/fixtures/feature-attribution");
    let owners: Vec<String> = ["fa-core", "fa-owner-third"].map(str::to_owned).to_vec();
    let target = host_triple().expect("host triple");
    let selection = collect(&workspace, &target, &owners, None).expect("resolves");
    let run = |aligned: &BTreeSet<String>| {
        attribute(&target, &selection.owners, &selection.invocations, &selection.members, &selection.tables, None, aligned)
            .expect("attributes")
    };
    let as_is = run(&BTreeSet::new());
    let what_if = run(&BTreeSet::from(["fa-base".to_owned()]));
    assert_eq!(as_is.totals.divergent_configurations, 2);
    assert_eq!(what_if.totals.divergent_configurations, 0);
    assert!(what_if.third_party_divergence.iter().any(|row| row.krate == "fa-base"),
        "the as-resolved feature sets are still reported");
}

// ---------------------------------------------------------------------------
// Reconciliation against a wrapped build
// ---------------------------------------------------------------------------

/// A private copy of a freshly built `ci-build`, the wrapper CI's owner and the
/// timed pass both use.
///
/// Mirrors `shipped_wrapper` in `ci-build-tests.rs`, which lives in another
/// binary's test crate: rebuild unconditionally (a `[[bin]]` test harness does
/// not build its sibling, so an existing file may be stale), take the path
/// Cargo reports, and exec a private link so a concurrent rebuild by another
/// test process cannot swap the file out from under this build.
fn wrapper_in(scratch: &Path) -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["build", "--message-format=json-render-diagnostics"])
        .args(["--no-default-features", "--features", "build-tools", "--bin", "ci-build"])
        .arg("--manifest-path")
        .arg(repo_root().join("scripts/Cargo.toml"))
        .output()
        .expect("building ci-build");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let built = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact" && message["target"]["name"] == "ci-build")
        .find_map(|message| message["executable"].as_str().map(PathBuf::from))
        .expect("cargo reports the ci-build executable");
    let private = scratch.join(format!("ci-build{}", std::env::consts::EXE_SUFFIX));
    if fs::hard_link(&built, &private).is_err() {
        fs::copy(&built, &private).expect("copying ci-build");
    }
    private
}

/// A wrapped build's compiles of fixture workspace packages (every member, not
/// only the ones the model keeps), counted per `(owner, package)`.
///
/// Build-script compiles are counted apart from library compiles: the model
/// charges a build script's seconds to a configuration, but a build-script
/// unit is never a configuration of its own.
#[derive(Default)]
struct Observed {
    libraries: BTreeMap<(String, String), usize>,
    build_scripts: BTreeMap<(String, String), usize>,
}

/// Build `selection` in `workspace` as the owner does — every owner's archive
/// compile, then its sidecar builds, in producer order, in one target
/// directory — with every rustc timed by the `ci-build` wrapper.
///
/// Returns the event directory, what the wrapper observed, and the owners whose
/// own test harness compiled.
fn wrapped_build(
    workspace: &Path,
    selection: &Selection,
    sidecars: Option<&SidecarTable>,
    scratch: &Path,
) -> (PathBuf, Observed, BTreeSet<String>) {
    let wrapper = wrapper_in(scratch);
    let target = host_triple().expect("host triple");
    let events = scratch.join("events");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let run = |owner: &str, args: &[&str], package: &str, features: &[String]| {
        let mut command = Command::new(&cargo);
        command
            .args(args)
            .args(["--quiet", "--offline", "--locked", "--target", &target, "--package", package])
            .arg("--manifest-path")
            .arg(workspace.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(scratch.join("target"))
            .env("RUSTC_WRAPPER", &wrapper)
            .env("BISCUIT_CI_BUILD_WRAP", "1")
            .env("BISCUIT_CI_BUILD_COUNTER_DIR", &events)
            .env("BISCUIT_CI_BUILD_PACKAGE", owner)
            .env("BISCUIT_CI_BUILD_CONFIGURATION", "reconcile");
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
        let output = command.output().expect("running cargo");
        assert!(output.status.success(), "{owner} {package}: {}", String::from_utf8_lossy(&output.stderr));
    };
    for (owner, features) in &selection.owners {
        run(owner, &["test", "--no-run"], owner, features);
        for invocation in selection.invocations.iter().filter(|invocation| &invocation.owner == owner) {
            let Some(name) = invocation.label.strip_prefix("sidecar ") else {
                continue;
            };
            let sidecar = &sidecars.expect("a sidecar table").sidecars[name];
            run(owner, &["build", "--profile", "test"], &sidecar.package, &sidecar.features);
        }
    }

    let fixture_members: BTreeSet<String> = cargo_metadata::MetadataCommand::new()
        .manifest_path(workspace.join("Cargo.toml"))
        .no_deps()
        .exec()
        .expect("fixture metadata")
        .workspace_packages()
        .iter()
        .map(|package| package.name.to_string())
        .collect();
    let mut observed = Observed::default();
    let mut harnesses: BTreeSet<String> = BTreeSet::new();
    for entry in fs::read_dir(&events).expect("the wrapper recorded events") {
        let path = entry.expect("event entry").path();
        let event: Event = serde_json::from_str(&fs::read_to_string(&path).expect("event read")).expect("event parses");
        if event.probe {
            continue;
        }
        let Some(package) = event.cargo_package.filter(|name| fixture_members.contains(name)) else {
            continue;
        };
        let build_script = event.crate_name.as_deref().is_some_and(|name| name.starts_with("build_script_"));
        let library = event.crate_types.iter().any(|kind| LIBRARY_KINDS.contains(&kind.as_str()));
        if build_script {
            *observed.build_scripts.entry((event.package, package)).or_default() += 1;
        } else if library {
            *observed.libraries.entry((event.package, package)).or_default() += 1;
        } else if package == event.package {
            harnesses.insert(package);
        }
    }
    (events, observed, harnesses)
}

/// Each configuration of `report`, counted per `(first owner, crate)`.
fn predicted(report: &Report) -> BTreeMap<(String, String), usize> {
    let mut predicted = BTreeMap::new();
    for krate in &report.crates {
        for config in &krate.configurations {
            *predicted.entry((config.first_owner.clone(), krate.krate.clone())).or_default() += 1;
        }
    }
    predicted
}

#[test]
fn a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations() {
    // The model's claim is about what Cargo compiles, so it is checked against
    // a build. `fa-owner-bin` has no library or build script; before the model
    // excluded such packages it predicted a configuration for that root that
    // no compile produced. `fa-util` has a build script, whose compiles must be
    // charged to a configuration without being counted as one.
    let workspace = fixture();
    let scratch = Scratch::new("wrapped");
    let target = host_triple().expect("host triple");
    let owners: Vec<String> = ["fa-core", "fa-owner-bin", "fa-owner-own", "fa-owner-plain", "fa-owner-third", "fa-owner-wsdep"]
        .map(str::to_owned)
        .to_vec();
    let selection = collect(&workspace, &target, &owners, None).expect("the fixture resolves");
    let (events, observed, harnesses) = wrapped_build(&workspace, &selection, None, &scratch.0);
    assert!(harnesses.contains("fa-owner-bin"), "the bin-only owner was built: {harnesses:?}");

    let report = attribute(&target, &selection.owners, &selection.invocations, &selection.members, &selection.tables, None, &BTreeSet::new())
        .expect("attributes");
    // Both directions at once: a prediction with no compile, or a compile with
    // no prediction, makes the maps differ.
    assert_eq!(predicted(&report), observed.libraries, "predicted (left) vs library compiles (right)");
    assert!(report.crates.iter().all(|krate| krate.krate != "fa-owner-bin"), "{:#?}", report.crates);

    // Cargo compiles a build script once per feature set of its own package,
    // so how many compiles an owner records is Cargo's business; every one
    // must belong to a configuration that owner's library compile provides.
    assert!(
        observed.build_scripts.contains_key(&("fa-core".to_owned(), "fa-util".to_owned())),
        "the first owner compiled fa-util's build script: {:?}",
        observed.build_scripts
    );
    for key in observed.build_scripts.keys() {
        assert_eq!(key.1, "fa-util", "only fa-util has a build script: {:?}", observed.build_scripts);
        assert!(observed.libraries.contains_key(key), "{key:?} compiled a build script but no library");
    }

    let compiles = read_compiles(&events, &selection.members).expect("events parse");
    let timed = attribute(
        &target,
        &selection.owners,
        &selection.invocations,
        &selection.members,
        &selection.tables,
        Some(&compiles),
        &BTreeSet::new(),
    )
    .expect("attributes");
    assert!(timed.unexplained_compiles.is_empty(), "{:#?}", timed.unexplained_compiles);
    // Each owner first builds at most one fa-util configuration here, so an
    // owner's fa-util compiles, build script included, are that configuration's.
    for config in &crate_report(&timed, "fa-util").configurations {
        let owned: Vec<&Compile> = compiles
            .iter()
            .filter(|compile| compile.package == "fa-util" && compile.owner == config.first_owner)
            .collect();
        let expected: f64 = owned.iter().map(|compile| compile.seconds).sum();
        let seconds = config.seconds.expect("a timed configuration has seconds");
        assert!((seconds - expected).abs() < 1e-9, "{}: {seconds} vs {owned:#?}", config.first_owner);
    }
    assert!(
        compiles
            .iter()
            .any(|compile| compile.build_script && compile.owner == "fa-core" && compile.package == "fa-util"),
        "the join saw the build-script compile fa-core's fa-util configuration is charged with"
    );
}

// ---------------------------------------------------------------------------
// The `--align` what-if
// ---------------------------------------------------------------------------

#[test]
fn one_owner_two_configurations_keep_their_own_seconds_and_merge_under_alignment() {
    // `fa-fwd-user` builds itself twice: its archive without `fa-fwd/feat`,
    // and its sidecar (`fa-owner-fwd`) with it. Both are first built by the
    // same owner. An equal split of that owner's seconds charged 5.5s to each
    // and, because the owner still has a configuration left under alignment,
    // counted nothing removed.
    let events = Scratch::new("one-owner");
    // Out of start order on disk: the join orders by start time.
    write_event(&events.0, 30, "fa-fwd-user", "fa-owner-fwd", &["lib"], 2_000);
    write_event(&events.0, 20, "fa-fwd-user", "fa-fwd-user", &["lib"], 1_500);
    write_event(&events.0, 10, "fa-fwd-user", "fa-fwd-user", &["lib"], 9_500);
    let workspace = fixture();

    let as_is = fixture_report(&workspace, &["fa-fwd-user"], Some(&events.0));
    let user = crate_report(&as_is, "fa-fwd-user");
    let seconds: Vec<_> = user.configurations.iter().map(|config| config.seconds).collect();
    assert_eq!(seconds, vec![Some(9.5), Some(1.5)], "archive first, then the sidecar");
    assert!(user.configurations.iter().all(|config| config.first_owner == "fa-fwd-user"));
    assert_eq!(as_is.totals.seconds_divergent, Some(1.5));
    assert!(as_is.unexplained_compiles.is_empty(), "{:#?}", as_is.unexplained_compiles);

    let aligned = fixture_report_aligned(&workspace, &["fa-fwd-user"], Some(&events.0), &["fa-fwd"]);
    let user = crate_report(&aligned, "fa-fwd-user");
    assert_eq!(user.configurations.len(), 1, "{user:#?}");
    assert_eq!(user.configurations[0].seconds, Some(9.5), "the surviving compile keeps its own seconds");
    assert_eq!(aligned.totals.seconds_removed, Some(1.5), "the merged-away compile is credited");
    assert_eq!(aligned.totals.seconds_first, Some(11.5), "fa-fwd-user 9.5s + fa-owner-fwd 2s");
    assert!(aligned.unexplained_compiles.is_empty(), "{:#?}", aligned.unexplained_compiles);
}

#[test]
fn a_build_script_is_charged_with_its_packages_next_library_compile() {
    let built = |host| Built {
        owner: "o".to_owned(),
        package: PackageKey { name: "w".to_owned(), version: "0.1.0".to_owned(), source: String::new() },
        host,
        provides: None,
    };
    let compile = |started_ms, build_script, host, seconds| Compile {
        owner: "o".to_owned(),
        package: "w".to_owned(),
        host,
        build_script,
        started_ms,
        seconds,
    };
    let compiles = [
        compile(0, false, false, 4.0),
        compile(1, true, true, 0.5),
        compile(2, false, false, 3.0),
        // A third target-side compile the model has no slot for.
        compile(3, false, false, 1.0),
        compile(4, true, true, 0.25),
    ];
    let (spent, unexplained) = join(&compiles, &[built(false), built(false)]);
    assert_eq!(spent, vec![4.0, 3.5], "the build script before the second compile joins it");
    assert_eq!(unexplained.len(), 1, "{unexplained:#?}");
    assert_eq!(unexplained[0].seconds, 1.25, "the extra compile, and a build script nothing followed");
}

/// Copy the fixture workspace's manifests and sources into `dir`.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("copy destination");
    for entry in fs::read_dir(from).expect("fixture directory") {
        let entry = entry.expect("fixture entry");
        let path = entry.path();
        if entry.file_name() == "target" {
            continue;
        }
        if path.is_dir() {
            copy_tree(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).expect("copying a fixture file");
        }
    }
}

#[test]
fn a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared() {
    // `fa-fwd/feat` forwards to `fa-leaf/deep`, which implies `deeper`. Aligning
    // `fa-fwd` therefore re-identifies `fa-leaf` too, and only by modeling that
    // does `fa-fwd-user`'s second configuration collapse. The prediction is
    // checked against a real build of a copy that declares the alignment:
    // `fa-fwd-user` itself enables `fa-fwd/feat`.
    let workspace = fixture();
    let target = host_triple().expect("host triple");
    let owners: Vec<String> = ["fa-fwd-user", "fa-owner-fwd"].map(str::to_owned).to_vec();
    let sidecars = fixture_sidecars();
    let selection = collect(&workspace, &target, &owners, Some(&sidecars)).expect("the fixture resolves");
    let aligned = BTreeSet::from(["fa-fwd".to_owned()]);
    let what_if = |tables: &FeatureTables, compiles: Option<&[Compile]>| {
        attribute(&target, &selection.owners, &selection.invocations, &selection.members, tables, compiles, &aligned)
            .expect("attributes")
    };

    // As built: the model matches the build, and each compile is joined.
    let built = Scratch::new("fwd-as-built");
    let (events, observed, _) = wrapped_build(&workspace, &selection, Some(&sidecars), &built.0);
    let as_is = attribute(&target, &selection.owners, &selection.invocations, &selection.members, &selection.tables, None, &BTreeSet::new())
        .expect("attributes");
    assert_eq!(predicted(&as_is), observed.libraries, "as built: predicted (left) vs compiled (right)");
    assert_eq!(observed.libraries.get(&("fa-fwd-user".to_owned(), "fa-fwd-user".to_owned())), Some(&2));

    // With the alignment declared.
    let declared = Scratch::new("fwd-declared");
    let copy = declared.0.join("workspace");
    copy_tree(&workspace, &copy);
    let manifest = copy.join("fwd-user/Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("fwd-user manifest");
    let edited = text.replace(
        r#"fa-fwd = { path = "../vendor/fwd" }"#,
        r#"fa-fwd = { path = "../vendor/fwd", features = ["feat"] }"#,
    );
    assert_ne!(edited, text, "the alignment declaration applied");
    fs::write(&manifest, edited).expect("declaring the alignment");
    let copy_selection = collect(&copy, &target, &owners, Some(&sidecars)).expect("the copy resolves");
    let (_, aligned_observed, _) = wrapped_build(&copy, &copy_selection, Some(&sidecars), &declared.0);

    let compiles = read_compiles(&events, &selection.members).expect("events parse");
    let modeled = what_if(&selection.tables, Some(&compiles));
    assert_eq!(predicted(&modeled), aligned_observed.libraries, "what-if (left) vs aligned build (right)");
    let unmodeled = what_if(&FeatureTables::new(), None);
    assert_ne!(predicted(&unmodeled), aligned_observed.libraries, "without forwarding the what-if misses the collapse");

    // The removed compile is the sidecar's, the later of the owner's two.
    let sidecar_seconds = compiles
        .iter()
        .filter(|compile| compile.owner == "fa-fwd-user" && compile.package == "fa-fwd-user")
        .nth(1)
        .expect("two compiles of fa-fwd-user")
        .seconds;
    assert!(modeled.unexplained_compiles.is_empty(), "{:#?}", modeled.unexplained_compiles);
    assert_eq!(modeled.totals.seconds_removed, Some(sidecar_seconds));
}
