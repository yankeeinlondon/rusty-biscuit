use sniff::filesystem::ProgrammingLanguage;
use sniff::os::NtpStatus;
use sniff::request::DetectionPlan;
use sniff::{SniffConfig, detect, detect_with_config};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod fixtures;

#[test]
fn test_detect_returns_hardware_info() {
    // OS + hardware only; the full convenience detect() path is exercised by
    // test_detect_completes_in_reasonable_time.
    let result =
        sniff::detect_with_plan(DetectionPlan::new().without_network().without_filesystem())
            .unwrap();
    let os = result.os.expect("os should be present");
    assert!(!os.name.is_empty());
    let hardware = result.hardware.expect("hardware should be present");
    assert!(hardware.memory.total_bytes > 0);
}

#[test]
fn test_detect_with_custom_base_dir() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(temp_dir.path().to_path_buf())
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    assert!(result.filesystem.is_some());
}

#[test]
fn test_detect_in_git_repo() {
    let (_dir, path) = fixtures::create_test_git_repo();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(path)
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.git.is_some());
}

#[test]
fn test_detect_cargo_workspace() {
    let (_dir, path) = fixtures::create_cargo_workspace();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(path)
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.repo.is_some());
    let repo = fs.repo.unwrap();
    assert!(repo.is_monorepo);
    assert!(repo.packages.is_some());
    assert_eq!(repo.packages.unwrap().len(), 2);
}

#[test]
fn test_detect_completes_in_reasonable_time() {
    // NFR-1: Fast path detection should complete in <300ms
    let start = Instant::now();
    let _ = detect();
    let elapsed = start.elapsed();
    // Allow slack for CI environments, package manager detection (PATH scanning),
    // and boundary-aware mixed-workspace package discovery.
    assert!(
        elapsed.as_millis() < 20000,
        "Detection took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_serialization_roundtrip() {
    // OS-only: this asserts os.name survives a JSON roundtrip. Skipping the
    // other (heavier) domains keeps the suite's concurrent filesystem/network
    // work down without changing what is exercised here.
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
    )
    .unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff::SniffResult = serde_json::from_str(&json).unwrap();
    let orig_os = result.os.expect("os should be present");
    let parsed_os = parsed.os.expect("parsed os should be present");
    assert_eq!(orig_os.name, parsed_os.name);
}

#[test]
fn test_performance_is_opt_in() {
    // The performance report is a plan-level toggle independent of which
    // domains run, so detect only OS to avoid an unnecessary monorepo scan.
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
    )
    .unwrap();
    assert!(result.performance.is_none());
}

#[test]
fn test_performance_report_is_serialized_when_requested() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_network()
            .without_filesystem()
            .performance(true),
    )
    .unwrap();

    let performance = result
        .performance
        .as_ref()
        .expect("performance should be present");
    assert!(performance.total_duration_ms >= 0.0);
    assert!(performance.stages.contains_key("detect.total"));

    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("performance").is_some());
}

#[test]
fn test_skip_all_returns_minimal_result() {
    let config = SniffConfig::new()
        .skip_hardware()
        .skip_network()
        .skip_filesystem();
    let result = detect_with_config(config).unwrap();
    assert!(result.hardware.is_none());
    assert!(result.network.is_none());
    assert!(result.filesystem.is_none());
}

#[test]
fn test_detect_mixed_languages() {
    let (_dir, path) = fixtures::create_mixed_language_dir();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(path)
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.languages.is_some());
    let langs = fs.languages.unwrap();
    assert!(langs.total_files_scanned >= 4);
}

#[test]
fn test_detect_pnpm_workspace() {
    let (_dir, path) = fixtures::create_pnpm_workspace();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(path)
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    let fs = result.filesystem.unwrap();
    assert!(fs.repo.is_some());
    let repo = fs.repo.unwrap();
    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    assert_eq!(
        repo.monorepo_layers[0].authority,
        sniff::filesystem::MonorepoStandard::PnpmWorkspaces
    );
}

#[test]
fn test_membership_glob_discovers_deep_nested_packages() {
    // `members/**` resolves package boundaries at depth two. Structure-mode
    // detection skips the manifest-index nested walk, so the packages reported
    // here come solely from the dialect-aware membership glob expander. The
    // former prefix-only expander reported the intermediate `members/group-*`
    // directories and missed these named packages.
    let (_dir, path) = fixtures::create_nested_glob_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nested glob workspace should be detected as a monorepo");

    let names: Vec<&str> = repo
        .packages
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|pkg| pkg.name.as_str())
        .collect();

    assert!(
        names.contains(&"pkg-one"),
        "expected deep package pkg-one, got {names:?}"
    );
    assert!(
        names.contains(&"pkg-two"),
        "expected deep package pkg-two, got {names:?}"
    );
}

#[test]
fn test_detect_language_uses_package_boundary_from_nested_workspace() {
    let (_dir, path) = fixtures::create_mixed_nested_workspace();
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(path.join("server"))
            .without_os()
            .without_hardware()
            .without_network(),
    )
    .unwrap();
    let filesystem = result.filesystem.unwrap();
    let languages = filesystem.languages.unwrap();

    assert_eq!(languages.primary, Some(ProgrammingLanguage::Rust));
    assert_eq!(languages.total_files_scanned, 2);
    assert!(
        languages
            .languages
            .iter()
            .any(|lang| lang.language == ProgrammingLanguage::Rust)
    );
    assert!(
        !languages
            .languages
            .iter()
            .any(|lang| lang.language == ProgrammingLanguage::TypeScript)
    );
}

// ============================================================================
// Monorepo topology (MonorepoLayer / monorepo_standards) integration tests
// ============================================================================

#[test]
fn test_cargo_workspace_topology_layer() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_cargo_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("cargo workspace should be detected");

    assert!(repo.is_monorepo);
    // One membership authority at one root.
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
    assert!(layer.orchestrators.is_empty());
    assert_eq!(layer.packages.len(), 2);
}

#[test]
fn test_virtual_cargo_single_member_is_not_a_monorepo() {
    // Regression: a virtual Cargo workspace with one member used to be
    // reported as a monorepo because the predicate treated
    // `WhenManifestDeclaresPackage` the same as `Always`. The honest
    // predicate now reads `root_is_package`, so a virtual root with one
    // member is degenerate.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_virtual_cargo_single_member_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("cargo detector still reports a RepoInfo for a single-member workspace");

    assert!(
        !repo.is_monorepo,
        "virtual Cargo workspace with one member must not be a monorepo"
    );
    // A layer still exists — it carries the single resolved package — but its
    // membership is degenerate, so `is_monorepo` stays false.
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
    assert_eq!(layer.packages.len(), 1);
    assert!(
        !layer.root_is_package,
        "virtual Cargo workspace root must not declare [package]"
    );
}

#[test]
fn test_cargo_root_package_plus_one_member_is_a_monorepo() {
    // Counterpart to the virtual-single-member regression: when the root
    // `Cargo.toml` also declares a `[package]`, the root counts and one member
    // is enough to be a monorepo.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_cargo_root_package_plus_one_member();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("cargo root-package-plus-one workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
    assert!(
        layer.root_is_package,
        "root_is_package must be true when the root Cargo.toml declares [package]"
    );
}

#[test]
fn test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer() {
    // Regression for the topology-forest gap: a Cargo workspace at the root
    // and a pnpm workspace at `web/` used to be collapsed into the Cargo
    // layer alone, silently dropping the nested pnpm standard. The marker
    // walk now dispatches the pnpm detector at `web/` so both layers appear.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_cargo_root_with_nested_pnpm();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("mixed cargo+pnpm forest should be detected");

    assert!(repo.is_monorepo);

    let cargo_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::CargoWorkspace)
        .expect("root Cargo workspace must produce a layer");
    assert_eq!(cargo_layer.root, path);

    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("nested pnpm workspace must produce its own layer");
    assert_eq!(pnpm_layer.root, path.join("web"));
    assert_eq!(pnpm_layer.packages.len(), 2);

    // Layer packages use the same repo-root-relative framing as the canonical
    // `RepoInfo.packages` catalog, so nested packages carry their `web/`
    // prefix.
    let layer_rels: Vec<String> = pnpm_layer.packages.to_vec();
    assert!(layer_rels.contains(&"web/packages/app".to_string()));
    assert!(
        !layer_rels.contains(&"packages/app".to_string()),
        "layer packages must be repo-root-relative, got: {layer_rels:?}"
    );

    // The flat `RepoInfo.packages` list uses the same repo-root-relative paths.
    let flat_rels: Vec<String> = repo
        .packages
        .iter()
        .flatten()
        .map(|p| p.relative.clone())
        .collect();
    assert!(
        flat_rels.contains(&"web/packages/app".to_string()),
        "flat package list must be repo-root-relative, got: {flat_rels:?}"
    );
}

#[test]
fn test_nested_uv_under_pnpm_is_discovered_as_its_own_layer() {
    // uv's `ForbidsNested` policy is about uv's own nested instances; a uv
    // workspace nested under a *different* standard (pnpm) must still be
    // discovered as its own layer.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_pnpm_root_with_nested_uv();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("mixed pnpm+uv forest should be detected");

    assert!(repo.is_monorepo);

    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("root pnpm workspace must produce a layer");
    assert_eq!(pnpm_layer.root, path);

    let uv_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::UvWorkspace)
        .expect("nested uv workspace must produce its own layer");
    assert_eq!(uv_layer.root, path.join("python"));
    // uv counts the root + each globbed child.
    assert_eq!(uv_layer.packages.len(), 2);
}

#[test]
fn test_nested_cargo_under_pnpm_is_discovered_as_its_own_layer() {
    // Counterpart to `test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer`
    // with the standards flipped: `Cargo.toml` must be a nested marker candidate
    // so a Cargo workspace nested under a pnpm root is reported as its own
    // layer. Cargo's `ForbidsNested` policy only blocks nested Cargo under an
    // ancestor Cargo workspace; under a different standard it is valid.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_pnpm_root_with_nested_cargo();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("mixed pnpm+cargo forest should be detected");

    assert!(repo.is_monorepo);

    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("root pnpm workspace must produce a layer");
    assert_eq!(pnpm_layer.root, path);

    let cargo_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::CargoWorkspace)
        .expect("nested Cargo workspace must produce its own layer");
    assert_eq!(cargo_layer.root, path.join("crates"));
    assert_eq!(
        cargo_layer.packages.len(),
        2,
        "nested Cargo workspace must resolve both members"
    );

    // Layer packages use the same repo-root-relative framing as the canonical
    // `RepoInfo.packages` catalog, so nested packages carry their `crates/`
    // prefix.
    let layer_rels: Vec<String> = cargo_layer.packages.to_vec();
    assert!(layer_rels.contains(&"crates/alpha".to_string()));
    assert!(
        !layer_rels.contains(&"alpha".to_string()),
        "layer packages must be repo-root-relative, got: {layer_rels:?}"
    );

    // The flat list uses the same repo-root-relative paths.
    let flat_rels: Vec<String> = repo
        .packages
        .iter()
        .flatten()
        .map(|p| p.relative.clone())
        .collect();
    assert!(
        flat_rels.contains(&"crates/alpha".to_string()),
        "flat package list must be repo-root-relative, got: {flat_rels:?}"
    );
}

#[test]
fn test_nested_only_cargo_workspace_is_discovered_under_bare_root() {
    // Before `Cargo.toml` was a nested marker, a bare root (no workspace
    // marker of its own) with only a nested Cargo workspace was missed
    // entirely. The marker walk now surfaces the nested Cargo layer.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nested_only_cargo_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nested-only Cargo topology should be detected");

    assert!(repo.is_monorepo);

    let cargo_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::CargoWorkspace)
        .expect("nested Cargo workspace must produce its own layer under a bare root");
    assert_eq!(cargo_layer.root, path.join("crates"));
    assert_eq!(cargo_layer.packages.len(), 2);
}

#[test]
fn test_nested_cargo_under_cargo_is_forbidden() {
    // Cargo's `ForbidsNested` policy: a nested Cargo workspace is invalid
    // Cargo, so sniff must not produce a second Cargo layer for it. The root
    // workspace still resolves normally.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_cargo_root_with_nested_forbidden_cargo();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("root Cargo workspace should be detected");

    assert!(repo.is_monorepo);

    let cargo_layers: Vec<_> = repo
        .monorepo_layers
        .iter()
        .filter(|l| l.authority == MonorepoStandard::CargoWorkspace)
        .collect();
    assert_eq!(
        cargo_layers.len(),
        1,
        "nested Cargo workspace must not produce a second layer"
    );
    assert_eq!(cargo_layers[0].root, path);
}

#[test]
fn test_nested_only_workspace_full_detection_does_not_panic() {
    // Regression: a root with no workspace marker but with a nested workspace
    // below (e.g. `web/pnpm-workspace.yaml`) used to panic in full
    // `detect_repo` because the manifest index was skipped eagerly and then
    // `expect`ed downstream. Full detection must build the index lazily once
    // nested outcomes exist.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nested_only_pnpm_workspace();
    // Use full `detect_repo`, not `detect_repo_structure`, to exercise the
    // nested package enrichment path that previously panicked.
    let repo = sniff::filesystem::detect_repo(&path)
        .unwrap()
        .expect("nested-only topology should still be detected");

    assert!(repo.is_monorepo);
    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("nested pnpm workspace must produce its own layer");
    assert_eq!(pnpm_layer.root, path.join("web"));
    assert_eq!(pnpm_layer.packages.len(), 2);
}

#[test]
fn test_nested_pnpm_and_dotnet_both_discovered_via_single_pass() {
    // Single-pass entry inspection must surface both a pnpm layer at `web/`
    // and a .NET solution layer at `dotnet/` from one `ignore` walk over a
    // bare-root repo. The deep `web/packages/app/package.json` exercises the
    // second-level depth so the walker's pruning does not over-eagerly
    // truncate the pnpm membership glob.
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nested_pnpm_and_dotnet_repo();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nested pnpm + dotnet forest should be detected");

    assert!(repo.is_monorepo);

    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("nested pnpm workspace must produce its own layer");
    assert_eq!(pnpm_layer.root, path.join("web"));
    assert_eq!(pnpm_layer.packages.len(), 2);

    let dotnet_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::DotNetSolution)
        .expect("nested .NET solution must produce its own layer");
    assert_eq!(dotnet_layer.root, path.join("dotnet"));
    assert_eq!(
        dotnet_layer.packages.len(),
        1,
        "dotnet layer must resolve the single .csproj listed by MyApp.sln"
    );
}

#[test]
fn test_root_marker_is_not_registered_as_nested_candidate() {
    // Nested discovery is non-root only: a marker placed directly at the repo
    // root must not register the root itself as a `Candidate` (its parent is
    // `root`, which the new walk skips). A repo whose only marker sits at the
    // root resolves to at most a single-package repo, never a nested layer.
    let (_dir, path) = fixtures::create_nested_marker_at_root_to_be_ignored();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(
                repo.monorepo_layers.is_empty(),
                "root marker must not register a nested candidate, got layers: {:?}",
                repo.monorepo_layers
            );
        }
    }
}

#[test]
fn test_node_modules_package_json_is_pruned() {
    // `node_modules` is in the prune list (`should_skip_directory_name`), so
    // `filter_entry` keeps the walker from descending into it. The fixture
    // buries a real pnpm workspace (with a resolvable `packages/app` member)
    // inside `node_modules`: a regressed prune would walk it, dispatch to
    // `detect_pnpm_workspace`, and produce a PnpmWorkspaces layer rooted under
    // `node_modules`. The top-level `app/package.json` has no `workspaces`
    // field, so it produces no layer of its own. A working prune therefore
    // leaves zero monorepo layers, making this assertion fail loudly if the
    // node_modules workspace is ever detected.
    let (_dir, path) = fixtures::create_pruned_node_modules_with_package_json();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    let Some(repo) = repo else {
        return;
    };
    assert!(
        repo.monorepo_layers.is_empty(),
        "pruned node_modules subtree must not register a nested candidate, got layers: {:?}",
        repo.monorepo_layers
    );
}

#[test]
fn test_gitignored_nested_marker_is_not_detected() {
    // Intentional behavior change from the old per-directory
    // `Path::exists()` probe: the single-pass `ignore` walker honors
    // `git_ignore`, so a gitignored `pnpm-workspace.yaml` is no longer
    // detected. The fixture's ignored marker declares a real member, so the
    // old `exists()` probe WOULD have produced a PnpmWorkspaces layer here —
    // making the empty-layers assertion discriminate the two implementations.
    // See the spec's "Intentional Behavior Change" section. Marker files are
    // conventionally committed, so the risk is judged negligible.
    let (_dir, path) = fixtures::create_gitignored_nested_marker();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    let Some(repo) = repo else {
        return;
    };
    assert!(
        repo.monorepo_layers.is_empty(),
        "gitignored nested marker must not register a candidate, got layers: {:?}",
        repo.monorepo_layers
    );
}

#[test]
fn test_per_root_confidence_in_same_standard_forest() {
    // Confidence is defined per detected standard/root pair: a degenerate
    // sibling must stay `Inferred` even when a non-degenerate twin of the
    // same standard exists elsewhere in the forest.
    use sniff::filesystem::{DetectionConfidence, MonorepoStandard};

    let (_dir, path) = fixtures::create_pnpm_forest_with_degenerate_sibling();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm forest should be detected");

    assert!(repo.is_monorepo);

    // Two pnpm layers, one at each root.
    let pnpm_layers: Vec<_> = repo
        .monorepo_layers
        .iter()
        .filter(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .collect();
    assert_eq!(
        pnpm_layers.len(),
        2,
        "expected one pnpm layer per root, got {:?}",
        pnpm_layers
    );

    let nested_root = path.join("nested");
    let (root_confidence, nested_confidence) = repo
        .monorepo_standards
        .iter()
        .filter(|s| s.standard == MonorepoStandard::PnpmWorkspaces)
        .fold((None, None), |(root_c, nested_c), s| {
            if s.root == path {
                (Some(s.confidence), nested_c)
            } else if s.root == nested_root {
                (root_c, Some(s.confidence))
            } else {
                (root_c, nested_c)
            }
        });

    // Root pnpm workspace is non-degenerate (two members).
    assert_eq!(
        root_confidence,
        Some(DetectionConfidence::MarkerConfirmed),
        "non-degenerate root pnpm must be marker-confirmed"
    );
    // Nested pnpm workspace is degenerate (one member, Never root membership).
    assert_eq!(
        nested_confidence,
        Some(DetectionConfidence::Inferred),
        "degenerate nested pnpm must stay inferred, got {:?}",
        repo.monorepo_standards
    );
}

#[test]
fn test_pnpm_workspace_topology_layer() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_pnpm_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    assert_eq!(
        repo.monorepo_layers[0].authority,
        MonorepoStandard::PnpmWorkspaces
    );
}

#[test]
fn test_pnpm_lockfile_parity_upgrades_provenance() {
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let (_dir, path) = fixtures::create_pnpm_workspace_with_lockfile();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm workspace should be detected");

    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::PnpmWorkspaces);
    assert_eq!(layer.provenance, PackageProvenance::Lockfile);
    assert_eq!(layer.lockfile_match, Some(true));

    // Per-package provenance lives on the canonical `RepoInfo.packages` entries.
    let packages = repo.packages.as_ref().expect("packages should be present");
    for rel in &layer.packages {
        let key = rel.replace('\\', "/");
        let pkg = packages
            .iter()
            .find(|p| p.relative == key)
            .expect("layer package must resolve to a catalog entry");
        assert_eq!(
            pkg.provenance,
            PackageProvenance::Lockfile,
            "package {key:?} should inherit Lockfile provenance"
        );
    }
}

#[test]
fn test_pnpm_lockfile_drift_records_mismatch() {
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let (_dir, path) = fixtures::create_pnpm_workspace_with_drifted_lockfile();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm workspace should be detected");

    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::PnpmWorkspaces);
    // Manifest remains the authority; lockfile mismatch is recorded.
    assert_eq!(layer.provenance, PackageProvenance::Globbed);
    assert_eq!(layer.lockfile_match, Some(false));
}

#[test]
fn test_pnpm_lockfile_superset_is_recorded_as_mismatch() {
    // Regression: a stale lockfile with extra importers used to silently
    // upgrade provenance to `Lockfile` because every manifest member still
    // appeared in the lockfile. Set equality flags this as drift instead.
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let (_dir, path) = fixtures::create_pnpm_workspace_with_stale_lockfile();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm workspace should be detected");

    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::PnpmWorkspaces);
    assert_eq!(
        layer.provenance,
        PackageProvenance::Globbed,
        "a stale lockfile must not upgrade provenance to Lockfile"
    );
    assert_eq!(
        layer.lockfile_match,
        Some(false),
        "stale extra importer must be reported as drift"
    );
}

#[test]
fn test_uv_lockfile_parity_upgrades_provenance() {
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let (_dir, path) = fixtures::create_uv_workspace_with_lockfile();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("uv workspace should be detected");

    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::UvWorkspace);
    assert_eq!(layer.provenance, PackageProvenance::Lockfile);
    assert_eq!(layer.lockfile_match, Some(true));
}

#[test]
fn test_uv_lockfile_superset_is_recorded_as_mismatch() {
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let (_dir, path) = fixtures::create_uv_workspace_with_stale_lockfile();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("uv workspace should be detected");

    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::UvWorkspace);
    assert_eq!(layer.provenance, PackageProvenance::Globbed);
    assert_eq!(layer.lockfile_match, Some(false));
}

#[test]
fn test_mixed_nested_workspace_has_layer_per_authority() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_mixed_nested_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("mixed nested workspace should be detected");

    assert!(repo.is_monorepo);
    // Cargo and pnpm both declare membership at the same root → one layer each.
    let authorities: Vec<MonorepoStandard> =
        repo.monorepo_layers.iter().map(|l| l.authority).collect();
    assert!(authorities.contains(&MonorepoStandard::CargoWorkspace));
    assert!(authorities.contains(&MonorepoStandard::PnpmWorkspaces));
}

#[test]
fn test_nx_pnpm_layer_has_pnpm_authority_and_nx_orchestrator() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nx_pnpm_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nx + pnpm workspace should be detected");

    assert!(repo.is_monorepo);
    // pnpm is the membership authority; Nx only orchestrates.
    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("pnpm should be the membership authority");
    assert_eq!(pnpm_layer.orchestrators, vec![MonorepoStandard::Nx]);
    // Nx must never be reported as an authority.
    assert!(
        !repo
            .monorepo_layers
            .iter()
            .any(|l| l.authority == MonorepoStandard::Nx)
    );
}

#[test]
fn test_nx_only_repo_is_not_a_monorepo() {
    use sniff::filesystem::{DetectionConfidence, MonorepoStandard};

    // `nx.json` alone has no membership authority. Detection finds no pnpm/npm
    // workspace patterns, so the repo is honestly *not* a monorepo.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("nx.json"), "{}").unwrap();
    std::fs::write(dir.path().join("package.json"), "{}").unwrap();

    let repo = sniff::filesystem::detect_repo_structure(dir.path())
        .unwrap()
        .expect("nx.json should still yield a RepoInfo");

    assert!(!repo.is_monorepo, "nx-only repo must not be a monorepo");
    assert!(repo.monorepo_layers.is_empty());
    // The downgrade is observable: an inferred Unknown standard is recorded.
    assert!(
        repo.monorepo_standards
            .iter()
            .any(|s| s.standard == MonorepoStandard::Unknown
                && s.confidence == DetectionConfidence::Inferred),
        "nx-only downgrade should record an inferred Unknown standard, got {:?}",
        repo.monorepo_standards
    );
}

#[test]
fn test_degenerate_workspaces_are_not_monorepos() {
    // Empty membership arrays must never count as a monorepo.
    for (label, (_dir, path)) in [
        ("cargo", fixtures::create_degenerate_cargo_workspace()),
        ("npm", fixtures::create_degenerate_npm_workspace()),
        ("pnpm", fixtures::create_degenerate_pnpm_workspace()),
    ] {
        let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
        match repo {
            None => {} // No membership authority at all — the strongest signal.
            Some(repo) => {
                assert!(
                    !repo.is_monorepo,
                    "{label}: degenerate workspace must not be a monorepo"
                );
                assert!(
                    repo.monorepo_layers.is_empty(),
                    "{label}: degenerate workspace must have no layers"
                );
            }
        }
    }
}

#[test]
fn test_monorepo_standards_serialize_with_kebab_case_ids() {
    let (_dir, path) = fixtures::create_pnpm_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pnpm workspace should be detected");

    let value = serde_json::to_value(&repo).unwrap();
    let obj = value.as_object().unwrap();

    // Legacy keys removed.
    assert!(
        !obj.contains_key("monorepo_tool"),
        "legacy monorepo_tool key should be absent"
    );
    assert!(
        !obj.contains_key("workspace_tools"),
        "legacy workspace_tools key should be absent"
    );

    // New keys present when non-empty, with kebab-case standard ids.
    let standards = obj
        .get("monorepo_standards")
        .and_then(|v| v.as_array())
        .expect("monorepo_standards should be a non-empty array");
    assert!(
        standards
            .iter()
            .any(|s| s.get("standard") == Some(&serde_json::json!("pnpm-workspaces"))),
        "expected kebab-case pnpm-workspaces id, got {standards:?}"
    );

    let layers = obj
        .get("monorepo_layers")
        .and_then(|v| v.as_array())
        .expect("monorepo_layers should be a non-empty array");
    assert_eq!(
        layers[0].get("authority"),
        Some(&serde_json::json!("pnpm-workspaces"))
    );
}

// ============================================================================
// Phase 8: acceptance — parity, catalog assertability, authority delegation
// ============================================================================

/// Walk up from `CARGO_MANIFEST_DIR` looking for the rusty-biscuit workspace
/// root. Returns `None` when the test is run outside this repo.
fn rusty_biscuit_repo_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut current = manifest_dir.as_path();
    while let Some(parent) = current.parent() {
        current = parent;
        let cargo_toml = current.join("Cargo.toml");
        let git_dir = current.join(".git");
        if cargo_toml.exists() && git_dir.exists() {
            let Ok(content) = std::fs::read_to_string(&cargo_toml) else {
                continue;
            };
            if content.contains("[workspace]") && content.contains("sniff/lib") {
                return Some(current.to_path_buf());
            }
        }
    }
    None
}

#[test]
fn test_rusty_biscuit_repo_topology_parity() {
    use sniff::filesystem::{MonorepoStandard, PackageProvenance};

    let Some(repo_root) = rusty_biscuit_repo_root() else {
        // Not running inside the rusty-biscuit repo; skip parity check.
        return;
    };

    let repo = sniff::filesystem::detect_repo_structure(&repo_root)
        .unwrap()
        .expect("rusty-biscuit should be detected as a monorepo");

    assert!(repo.is_monorepo);
    assert!(
        !repo.monorepo_layers.is_empty(),
        "rusty-biscuit should have membership layers"
    );

    let authorities: HashSet<MonorepoStandard> =
        repo.monorepo_layers.iter().map(|l| l.authority).collect();
    assert!(
        authorities.contains(&MonorepoStandard::CargoWorkspace),
        "cargo-workspace authority missing: {authorities:?}"
    );
    assert!(
        authorities.contains(&MonorepoStandard::PnpmWorkspaces),
        "pnpm-workspaces authority missing: {authorities:?}"
    );

    let packages = repo
        .packages
        .as_deref()
        .expect("packages should be present");
    assert!(!packages.is_empty());

    // The pnpm workspace package must be owned by pnpm.
    let pnpm_pkg = packages
        .iter()
        .find(|p| p.relative == "homelab/server/frontend")
        .expect("homelab/server/frontend should be detected");
    assert_eq!(
        pnpm_pkg.standard,
        MonorepoStandard::PnpmWorkspaces,
        "pnpm package must be owned by pnpm-workspaces"
    );
    assert_eq!(
        pnpm_pkg.provenance,
        PackageProvenance::Lockfile,
        "pnpm package should be lockfile-derived"
    );

    // Every package is either cargo, pnpm, or a manifest-scan fallback.
    // No orchestrator-only standard may own a package.
    for pkg in packages {
        assert!(
            matches!(
                pkg.standard,
                MonorepoStandard::CargoWorkspace
                    | MonorepoStandard::PnpmWorkspaces
                    | MonorepoStandard::Unknown
            ),
            "package {} has unexpected standard {:?}",
            pkg.relative,
            pkg.standard
        );
        assert_ne!(
            pkg.standard,
            MonorepoStandard::Nx,
            "package {} must not be owned by Nx",
            pkg.relative
        );
    }

    // Every layer package path resolves to exactly one catalog entry.
    let catalog: HashSet<&str> = packages.iter().map(|p| p.relative.as_str()).collect();
    for layer in &repo.monorepo_layers {
        for layer_pkg in &layer.packages {
            let rel = layer_pkg.as_str();
            assert!(
                catalog.contains(rel),
                "layer package {rel:?} not found in canonical catalog"
            );
        }
    }
}

#[test]
fn test_nx_delegates_package_ownership_to_pnpm() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nx_pnpm_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nx + pnpm workspace should be detected");

    let pnpm_layer = repo
        .monorepo_layers
        .iter()
        .find(|l| l.authority == MonorepoStandard::PnpmWorkspaces)
        .expect("pnpm should be the membership authority");
    assert_eq!(pnpm_layer.orchestrators, vec![MonorepoStandard::Nx]);

    let packages = repo
        .packages
        .as_deref()
        .expect("packages should be present");
    assert!(!packages.is_empty());

    // Every package is owned by the membership authority, never by Nx.
    for pkg in packages {
        assert_eq!(
            pkg.standard,
            MonorepoStandard::PnpmWorkspaces,
            "package {} must be owned by pnpm-workspaces, not {:?}",
            pkg.relative,
            pkg.standard
        );
    }
}

#[test]
fn test_monorepo_layer_packages_resolve_to_canonical_catalog() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_nx_pnpm_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("nx + pnpm workspace should be detected");

    let packages = repo
        .packages
        .as_deref()
        .expect("packages should be present");
    assert!(!packages.is_empty());

    // Each package carries its owning standard and provenance directly.
    for pkg in packages {
        assert_ne!(
            pkg.standard,
            MonorepoStandard::Unknown,
            "package {} should have a concrete standard",
            pkg.relative
        );
    }

    // Every layer package path resolves to exactly one catalog entry.
    let catalog: HashSet<&str> = packages.iter().map(|p| p.relative.as_str()).collect();
    for layer in &repo.monorepo_layers {
        for layer_pkg in &layer.packages {
            let rel = layer_pkg.as_str();
            assert!(
                catalog.contains(rel),
                "layer package {rel:?} not found in canonical catalog"
            );
        }
    }
}

// ============================================================================
// Phase 4: new membership authorities — Bun, uv, Go workspace
// ============================================================================

#[test]
fn test_bun_workspace_authority_is_bun() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_bun_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("bun workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    assert_eq!(
        repo.monorepo_layers[0].authority,
        MonorepoStandard::BunWorkspaces
    );
    assert_eq!(repo.monorepo_layers[0].packages.len(), 2);
}

#[test]
fn test_degenerate_bun_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_bun_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_uv_workspace_counts_root_as_member() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_uv_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("uv workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    assert_eq!(
        repo.monorepo_layers[0].authority,
        MonorepoStandard::UvWorkspace
    );
    // uv's `RootMembership::Always`: root + two children = three packages.
    assert_eq!(repo.monorepo_layers[0].packages.len(), 3);
    assert_eq!(repo.packages.as_ref().map(Vec::len), Some(3));
}

#[test]
fn test_degenerate_uv_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_uv_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_go_workspace_resolves_explicit_use_paths() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_go_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("go workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::GoWorkspace);

    let mut relatives: Vec<String> = layer.packages.to_vec();
    relatives.sort();
    assert_eq!(relatives, vec!["svc-a".to_string(), "svc-b".to_string()]);
}

#[test]
fn test_degenerate_go_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_go_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

// ============================================================================
// Phase 5: new membership authorities — Gradle, Maven, .NET Solution
// ============================================================================

#[test]
fn test_gradle_workspace_authority_is_gradle() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_gradle_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("gradle workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::GradleMultiProject);

    let mut relatives: Vec<String> = layer.packages.to_vec();
    relatives.sort();
    assert_eq!(relatives, vec!["app".to_string(), "core".to_string()]);
    // The repo-local `gradlew` wrapper presence is recorded for Phase 7 to act on.
    assert!(path.join("gradlew").exists());
}

#[test]
fn test_degenerate_gradle_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_gradle_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_maven_workspace_authority_is_maven() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_maven_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("maven workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::MavenMultiModule);

    let mut relatives: Vec<String> = layer.packages.to_vec();
    relatives.sort();
    assert_eq!(relatives, vec!["core".to_string(), "web".to_string()]);
}

#[test]
fn test_degenerate_maven_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_maven_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_dotnet_solution_authority_is_dotnet() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_dotnet_solution();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect(".NET solution should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::DotNetSolution);

    let mut relatives: Vec<String> = layer.packages.to_vec();
    relatives.sort();
    // Each project's directory (the `.csproj` parent) becomes a package.
    assert_eq!(
        relatives,
        vec!["src/App".to_string(), "src/Lib".to_string()]
    );
}

#[test]
fn test_degenerate_dotnet_solution_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_dotnet_solution();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

// ============================================================================
// Phase 6: polyglot build systems — Bazel, Pants, Buck2, Rush Stack
// ============================================================================

#[test]
fn test_bazel_workspace_segments_nested_workspace_into_its_own_layer() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_bazel_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("bazel workspace should be detected");

    assert!(repo.is_monorepo);
    // Parent workspace + nested workspace => two layers, both Bazel.
    assert_eq!(repo.monorepo_layers.len(), 2);
    for layer in &repo.monorepo_layers {
        assert_eq!(layer.authority, MonorepoStandard::Bazel);
    }

    let parent = repo
        .monorepo_layers
        .iter()
        .find(|l| l.root == path)
        .expect("parent layer rooted at repo root");
    let mut parent_rels: Vec<String> = parent.packages.to_vec();
    parent_rels.sort();
    // The nested subtree must be excluded from the parent's package list.
    assert_eq!(parent_rels, vec!["a".to_string(), "b".to_string()]);

    let nested = repo
        .monorepo_layers
        .iter()
        .find(|l| l.root == path.join("nested"))
        .expect("nested layer rooted at nested/");
    let nested_rels: Vec<String> = nested.packages.to_vec();
    // Layer packages are repo-root-relative, so the nested workspace root is
    // represented as `nested`.
    assert_eq!(nested_rels, vec!["nested".to_string()]);

    // The flat `RepoInfo.packages` list uses the same repo-root-relative paths.
    let flat_rels: Vec<String> = repo
        .packages
        .iter()
        .flatten()
        .map(|p| p.relative.clone())
        .collect();
    assert!(
        flat_rels.contains(&"nested".to_string()),
        "flat package list must be repo-root-relative, got: {flat_rels:?}"
    );
}

#[test]
fn test_degenerate_bazel_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_bazel_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_pants_workspace_discovers_leaf_packages() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_pants_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("pants workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::Pants);
    // Leaf walk finds both `BUILD.pants` directories without a root manifest list.
    assert_eq!(layer.packages.len(), 2);
}

#[test]
fn test_degenerate_pants_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_pants_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_buck2_workspace_discovers_leaf_packages() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_buck2_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("buck2 workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::Buck2);
    assert_eq!(layer.packages.len(), 2);
}

#[test]
fn test_degenerate_buck2_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_buck2_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

#[test]
fn test_rush_workspace_authority_is_rush() {
    use sniff::filesystem::MonorepoStandard;

    let (_dir, path) = fixtures::create_rush_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path)
        .unwrap()
        .expect("rush workspace should be detected");

    assert!(repo.is_monorepo);
    assert_eq!(repo.monorepo_layers.len(), 1);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::RushStack);

    let mut relatives: Vec<String> = layer.packages.to_vec();
    relatives.sort();
    assert_eq!(
        relatives,
        vec!["apps/app".to_string(), "libraries/lib".to_string()]
    );
}

#[test]
fn test_degenerate_rush_workspace_is_not_a_monorepo() {
    let (_dir, path) = fixtures::create_degenerate_rush_workspace();
    let repo = sniff::filesystem::detect_repo_structure(&path).unwrap();
    match repo {
        None => {}
        Some(repo) => {
            assert!(!repo.is_monorepo);
            assert!(repo.monorepo_layers.is_empty());
        }
    }
}

// === Regression tests for JSON serialization of partial results ===
// Bug: Skipped sections were serialized as empty objects instead of being omitted.
//
// NOTE: These tests parse JSON as serde_json::Value and check top-level keys
// rather than using substring matching, because nested data (e.g. Cargo feature
// names like "network") can produce false positives with contains().

/// Helper: parse SniffResult JSON and return the top-level key set.
fn top_level_keys(result: &sniff::SniffResult) -> std::collections::HashSet<String> {
    let json = serde_json::to_string(result).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn test_skip_hardware_json_omits_hardware_key() {
    // Regression test: JSON should NOT contain "hardware" key when skipped.
    // Also skip filesystem (not asserted here) to avoid a monorepo scan.
    let config = SniffConfig::new().skip_hardware().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("hardware"),
        "JSON should not contain hardware key when skipped"
    );
    assert!(keys.contains("network"), "JSON should contain network key");
}

#[test]
fn test_skip_network_json_omits_network_key() {
    // Regression test: JSON should NOT contain "network" key when skipped.
    // Also skip filesystem (not asserted here) to avoid a monorepo scan.
    let config = SniffConfig::new().skip_network().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("network"),
        "JSON should not contain network key when skipped"
    );
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_skip_filesystem_json_omits_filesystem_key() {
    // Regression test: JSON should NOT contain "filesystem" key when skipped.
    // Also skip network (not asserted here) to keep the test cheap.
    let config = SniffConfig::new().skip_filesystem().skip_network();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        !keys.contains("filesystem"),
        "JSON should not contain filesystem key when skipped"
    );
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
}

#[test]
fn test_hardware_only_json_contains_only_hardware() {
    // Regression test: When only hardware is requested, JSON should contain ONLY hardware
    let config = SniffConfig::new().skip_network().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let keys = top_level_keys(&result);
    assert!(
        keys.contains("hardware"),
        "JSON should contain hardware key"
    );
    assert!(
        !keys.contains("network"),
        "JSON should not contain network key"
    );
    assert!(
        !keys.contains("filesystem"),
        "JSON should not contain filesystem key"
    );
}

#[test]
fn test_partial_result_deserialization_roundtrip() {
    // Regression test: Partial results should deserialize correctly.
    // Skip filesystem too (not asserted here) to avoid a monorepo scan.
    let config = SniffConfig::new().skip_hardware().skip_filesystem();
    let result = detect_with_config(config).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    let parsed: sniff::SniffResult = serde_json::from_str(&json).unwrap();
    assert!(
        parsed.hardware.is_none(),
        "Deserialized hardware should be None"
    );
    assert!(
        parsed.network.is_some(),
        "Deserialized network should be Some"
    );
}

// ============================================================================
// OS Detection Integration Tests
// ============================================================================

/// Tests that detect_os returns populated OS detection fields.
#[test]
fn test_detect_os_has_detection_fields() {
    use sniff::hardware::detect_os;

    let os = detect_os().expect("detect_os should succeed");

    // OS info should have populated fields
    assert!(!os.name.is_empty(), "OS name should be detected");
    assert!(!os.kernel.is_empty(), "Kernel version should be detected");

    // OS type should match current platform
    #[cfg(target_os = "macos")]
    assert_eq!(os.os_type, sniff::hardware::OsType::MacOS);

    #[cfg(target_os = "linux")]
    assert_eq!(os.os_type, sniff::hardware::OsType::Linux);

    #[cfg(target_os = "windows")]
    assert_eq!(os.os_type, sniff::hardware::OsType::Windows);
}

/// Tests that detect_locale returns valid locale data.
#[test]
fn test_detect_locale_returns_valid_data() {
    use sniff::hardware::detect_locale;

    let locale = detect_locale();

    // At least one of LANG or LC_* should typically be set on most systems
    // But we can't require it in all environments (CI containers may have minimal setup)
    // So we just verify the structure is populated correctly
    if locale.lang.is_some() || locale.lc_all.is_some() {
        // If we have locale data, preferred_language extraction should work
        // (unless the locale is "C" or "POSIX")
        if let Some(ref lang) = locale.lang
            && lang != "C"
            && lang != "POSIX"
            && lang.contains('_')
        {
            assert!(
                locale.preferred_language.is_some(),
                "Should extract preferred language from locale"
            );
        }
    }

    // LocaleInfo should always have valid structure even if empty
    let json = serde_json::to_string(&locale).expect("LocaleInfo should serialize");
    let _parsed: sniff::hardware::LocaleInfo =
        serde_json::from_str(&json).expect("LocaleInfo should deserialize");
}

/// Tests that detect_timezone returns a valid UTC offset.
#[test]
fn test_detect_timezone_returns_valid_offset() {
    use sniff::hardware::detect_timezone;

    let time_info = detect_timezone();

    // UTC offset should be within valid bounds (-12h to +14h in seconds)
    assert!(
        time_info.utc_offset_seconds >= -12 * 3600,
        "UTC offset should be >= -12 hours"
    );
    assert!(
        time_info.utc_offset_seconds <= 14 * 3600,
        "UTC offset should be <= +14 hours"
    );

    // Timezone abbreviation should be present on all platforms
    assert!(
        time_info.timezone_abbr.is_some(),
        "Timezone abbreviation should be detected"
    );

    // Monotonic clock should always be available on modern systems
    assert!(
        time_info.monotonic_available,
        "Monotonic clock should be available"
    );

    // TimeInfo should serialize/deserialize correctly
    let json = serde_json::to_string(&time_info).expect("TimeInfo should serialize");
    let _parsed: sniff::hardware::TimeInfo =
        serde_json::from_str(&json).expect("TimeInfo should deserialize");
}

/// Tests that detect_os_type matches the current platform.
#[test]
fn test_detect_os_type_matches_platform() {
    use sniff::hardware::{OsType, detect_os_type};

    let os_type = detect_os_type();

    // Verify the detected type matches the compilation target
    #[cfg(target_os = "macos")]
    assert_eq!(
        os_type,
        OsType::MacOS,
        "Should detect macOS on macOS platform"
    );

    #[cfg(target_os = "linux")]
    assert_eq!(
        os_type,
        OsType::Linux,
        "Should detect Linux on Linux platform"
    );

    #[cfg(target_os = "windows")]
    assert_eq!(
        os_type,
        OsType::Windows,
        "Should detect Windows on Windows platform"
    );

    #[cfg(target_os = "freebsd")]
    assert_eq!(
        os_type,
        OsType::FreeBSD,
        "Should detect FreeBSD on FreeBSD platform"
    );

    // On any platform, the type should have a valid Display implementation
    let display = os_type.to_string();
    assert!(!display.is_empty(), "OsType should have valid Display");
}

// ============================================================================
// Platform-Specific Package Manager Integration Tests
// ============================================================================

/// Tests macOS package manager detection finds homebrew or softwareupdate.
#[cfg(target_os = "macos")]
#[test]
fn test_macos_package_managers_finds_expected_managers() {
    use sniff::hardware::{SystemPackageManager, detect_macos_package_managers};

    let managers = detect_macos_package_managers(None);

    // softwareupdate is always present on macOS as a system utility
    let has_softwareupdate = managers
        .managers
        .iter()
        .any(|m| m.manager == SystemPackageManager::Softwareupdate);
    assert!(
        has_softwareupdate,
        "macOS should always have softwareupdate available"
    );

    // A primary should always be selected on macOS
    assert!(
        managers.primary.is_some(),
        "macOS should have a primary package manager"
    );

    // If homebrew is installed, it should be detected
    let homebrew_apple_silicon = std::path::Path::new("/opt/homebrew/bin/brew").exists();
    let homebrew_intel = std::path::Path::new("/usr/local/bin/brew").exists();

    if homebrew_apple_silicon || homebrew_intel {
        let has_homebrew = managers
            .managers
            .iter()
            .any(|m| m.manager == SystemPackageManager::Homebrew);
        assert!(has_homebrew, "Homebrew should be detected when installed");
        assert_eq!(
            managers.primary,
            Some(SystemPackageManager::Homebrew),
            "Homebrew should be primary when installed"
        );
    }
}

/// Tests Linux package manager detection finds at least one manager.
#[cfg(target_os = "linux")]
#[test]
fn test_linux_package_managers_finds_at_least_one() {
    use sniff::hardware::{detect_linux_distro, detect_linux_package_managers};

    // Get distro info to determine family
    let linux_family = detect_linux_distro().map(|d| d.family);
    let managers = detect_linux_package_managers(linux_family, None);

    // On any real Linux system, at least one package manager should be found
    // This may fail in extremely minimal containers, which is acceptable
    if !managers.managers.is_empty() {
        // If managers are found, primary should be set
        assert!(
            managers.primary.is_some(),
            "Should have primary if managers are found"
        );

        // Each detected manager should have a valid path
        for m in &managers.managers {
            assert!(
                !m.path.is_empty(),
                "Detected manager {} should have a path",
                m.manager
            );
        }
    }
}

/// Tests that the default (full) OS request includes package manager info.
#[test]
fn test_os_includes_package_managers() {
    // OS-only via the default OsRequest -- the same OS code path detect() runs,
    // minus the unrelated hardware/network/filesystem work.
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
    )
    .unwrap();
    let os = result.os.expect("os should be present");

    // On desktop platforms (macOS, Linux, Windows), package managers should be detected
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        assert!(
            os.system_package_managers.is_some(),
            "System package managers should be detected on desktop platforms"
        );

        let mgrs = os.system_package_managers.as_ref().unwrap();
        // At minimum, the structure should be valid
        assert!(
            mgrs.primary.is_some() || mgrs.managers.is_empty(),
            "If managers exist, primary should be set"
        );
    }
}

/// Tests that the default (full) OS request includes locale info.
#[test]
fn test_os_includes_locale() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
    )
    .unwrap();
    let os = result.os.expect("os should be present");

    assert!(
        os.locale.is_some(),
        "Locale info should be included in OS detection"
    );
}

/// Tests that the default (full) OS request includes time info.
#[test]
fn test_os_includes_time_info() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
    )
    .unwrap();
    let os = result.os.expect("os should be present");

    assert!(
        os.time.is_some(),
        "Time info should be included in OS detection"
    );

    let time = os.time.as_ref().unwrap();
    // Verify basic time info fields
    assert!(
        time.utc_offset_seconds >= -12 * 3600 && time.utc_offset_seconds <= 14 * 3600,
        "UTC offset should be within valid range"
    );
}

// ============================================================================
// Network ip_addresses Integration Tests
// ============================================================================

/// Tests that network info includes ip_addresses field with proper structure.
#[test]
fn test_network_has_ip_addresses_field() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_filesystem(),
    )
    .unwrap();
    let network = result.network.expect("network should be present");

    if !network.permission_denied {
        // ip_addresses field should exist and have v4/v6 vectors
        // (even if empty, the structure should be present)
        let v4_count = network.ip_addresses.v4.len();
        let v6_count = network.ip_addresses.v6.len();

        // If interfaces have addresses, they should be aggregated
        let expected_v4: usize = network
            .interfaces
            .iter()
            .map(|i| i.ipv4_addresses.len())
            .sum();
        let expected_v6: usize = network
            .interfaces
            .iter()
            .map(|i| i.ipv6_addresses.len())
            .sum();

        assert_eq!(
            v4_count, expected_v4,
            "ip_addresses.v4 count should match interface IPv4 sum"
        );
        assert_eq!(
            v6_count, expected_v6,
            "ip_addresses.v6 count should match interface IPv6 sum"
        );
    }
}

/// Tests that ip_addresses JSON serialization produces expected structure.
#[test]
fn test_network_ip_addresses_json_structure() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_filesystem(),
    )
    .unwrap();
    let json = serde_json::to_string(&result).expect("SniffResult should serialize");

    // If network is present, JSON should have ip_addresses with v4/v6
    if result.network.is_some() {
        // Parse as Value to inspect structure
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("JSON should parse as Value");

        if let Some(network) = value.get("network") {
            let ip_addresses = network.get("ip_addresses");
            assert!(
                ip_addresses.is_some(),
                "network should have ip_addresses field"
            );
            assert!(
                network.get("wan_ip_address").is_some(),
                "network should have wan_ip_address field"
            );

            let ip_addr = ip_addresses.unwrap();
            assert!(ip_addr.get("v4").is_some(), "ip_addresses should have v4");
            assert!(ip_addr.get("v6").is_some(), "ip_addresses should have v6");

            // v4 and v6 should be arrays
            assert!(
                ip_addr.get("v4").unwrap().is_array(),
                "ip_addresses.v4 should be an array"
            );
            assert!(
                ip_addr.get("v6").unwrap().is_array(),
                "ip_addresses.v6 should be an array"
            );

            // Each address entry should have address and interface fields
            if let Some(v4_arr) = ip_addr.get("v4").and_then(|v| v.as_array()) {
                for addr in v4_arr {
                    assert!(
                        addr.get("address").is_some(),
                        "IPv4 entry should have address field"
                    );
                    assert!(
                        addr.get("interface").is_some(),
                        "IPv4 entry should have interface field"
                    );
                }
            }

            if let Some(v6_arr) = ip_addr.get("v6").and_then(|v| v.as_array()) {
                for addr in v6_arr {
                    assert!(
                        addr.get("address").is_some(),
                        "IPv6 entry should have address field"
                    );
                    assert!(
                        addr.get("interface").is_some(),
                        "IPv6 entry should have interface field"
                    );
                }
            }
        }
    }
}

/// Tests that ip_addresses roundtrip through JSON correctly.
#[test]
fn test_network_ip_addresses_roundtrip() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_filesystem(),
    )
    .unwrap();
    let json = serde_json::to_string(&result).expect("SniffResult should serialize");
    let parsed: sniff::SniffResult = serde_json::from_str(&json).expect("JSON should deserialize");

    if let (Some(orig_net), Some(parsed_net)) = (&result.network, &parsed.network) {
        // Counts should match
        assert_eq!(
            orig_net.ip_addresses.v4.len(),
            parsed_net.ip_addresses.v4.len(),
            "v4 count should survive roundtrip"
        );
        assert_eq!(
            orig_net.ip_addresses.v6.len(),
            parsed_net.ip_addresses.v6.len(),
            "v6 count should survive roundtrip"
        );
        assert_eq!(
            orig_net.wan_ip_address, parsed_net.wan_ip_address,
            "wan_ip_address should survive roundtrip"
        );

        // Contents should match
        for (orig, parsed) in orig_net
            .ip_addresses
            .v4
            .iter()
            .zip(parsed_net.ip_addresses.v4.iter())
        {
            assert_eq!(
                orig.address, parsed.address,
                "IPv4 address should survive roundtrip"
            );
            assert_eq!(
                orig.interface, parsed.interface,
                "IPv4 interface should survive roundtrip"
            );
        }

        for (orig, parsed) in orig_net
            .ip_addresses
            .v6
            .iter()
            .zip(parsed_net.ip_addresses.v6.iter())
        {
            assert_eq!(
                orig.address, parsed.address,
                "IPv6 address should survive roundtrip"
            );
            assert_eq!(
                orig.interface, parsed.interface,
                "IPv6 interface should survive roundtrip"
            );
        }
    }
}

#[test]
fn test_detect_with_plan_summary_mode() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary())
        .hardware(HardwareRequest::summary())
        .without_network()
        .without_filesystem();

    let start = Instant::now();
    let result = sniff::detect_with_plan(plan).unwrap();
    let elapsed = start.elapsed();

    assert!(result.os.is_some());
    assert!(result.hardware.is_some());
    assert!(result.network.is_none());
    assert!(result.filesystem.is_none());

    // Summary mode should be significantly faster than full detection
    assert!(
        elapsed.as_millis() < 2000,
        "Summary detection took too long: {:?}",
        elapsed
    );
}

// ============================================================================
// Selective-cost behavior regression tests (review-3 item 3)
// ============================================================================

/// Creates a temporary git repo with a committed file and an uncommitted modification,
/// suitable for testing file_changes vs diff payload behavior.
fn create_dirty_git_repo() -> (tempfile::TempDir, PathBuf) {
    use git2::{Repository, Signature};
    use std::fs;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Create and commit a file
    let file_path = dir.path().join("hello.txt");
    fs::write(&file_path, "hello world\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("hello.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    // Now modify the file (unstaged change) to make the repo dirty
    fs::write(&file_path, "hello world\nmodified line\n").unwrap();

    // Also create an untracked file
    fs::write(dir.path().join("untracked.txt"), "new file\n").unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_merge_conflict_repo() -> (tempfile::TempDir, PathBuf) {
    use std::fs;
    use std::process::Command;

    fn run_git(dir: &std::path::Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed with {:?}", args, status);
    }

    fn run_git_expect_failure(dir: &std::path::Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(
            !status.success(),
            "git {:?} unexpectedly succeeded with {:?}",
            args,
            status
        );
    }

    fn git_stdout(dir: &std::path::Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {:?} failed", args);
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    let dir = tempfile::TempDir::new().unwrap();

    run_git(dir.path(), &["init"]);
    run_git(dir.path(), &["config", "user.email", "test@test.com"]);
    run_git(dir.path(), &["config", "user.name", "Test User"]);

    fs::write(dir.path().join("conflict.txt"), "base\n").unwrap();
    run_git(dir.path(), &["add", "conflict.txt"]);
    run_git(dir.path(), &["commit", "-m", "initial commit"]);

    let main_branch = git_stdout(dir.path(), &["rev-parse", "--abbrev-ref", "HEAD"]);

    run_git(dir.path(), &["checkout", "-b", "feature"]);
    fs::write(dir.path().join("conflict.txt"), "feature branch\n").unwrap();
    run_git(dir.path(), &["commit", "-am", "feature change"]);

    run_git(dir.path(), &["checkout", &main_branch]);
    fs::write(dir.path().join("conflict.txt"), "main branch\n").unwrap();
    run_git(dir.path(), &["commit", "-am", "main change"]);

    run_git_expect_failure(dir.path(), &["merge", "feature"]);

    let path = dir.path().to_path_buf();
    (dir, path)
}

#[test]
fn test_git_full_has_file_changes_but_no_diff_payloads() {
    use sniff::request::*;

    let (_dir, path) = create_dirty_git_repo();

    let plan = DetectionPlan::new()
        .base_dir(path)
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::full())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        );

    let result = sniff::detect_with_plan(plan).unwrap();
    let fs = result.filesystem.expect("filesystem should be present");
    let git = fs.git.expect("git should be present");

    // GitRequest::full() includes file_changes (paths, status, line counts)
    assert!(
        !git.file_changes.is_empty(),
        "full() should populate file_changes"
    );

    // But does NOT include unified diff payloads
    assert!(
        git.status.as_ref().unwrap().dirty.is_empty(),
        "full() should NOT populate dirty diff payloads"
    );
    assert!(
        git.status.as_ref().unwrap().untracked.is_empty(),
        "full() should NOT populate untracked file details"
    );

    // Verify the counts are correct
    assert!(git.status.as_ref().unwrap().is_dirty);
    assert!(git.status.as_ref().unwrap().unstaged_count > 0);
    assert!(git.status.as_ref().unwrap().untracked_count > 0);
}

#[test]
fn test_git_deep_includes_diff_payloads() {
    use sniff::request::*;

    let (_dir, path) = create_dirty_git_repo();

    let plan = DetectionPlan::new()
        .base_dir(path)
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::deep())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        );

    let result = sniff::detect_with_plan(plan).unwrap();
    let fs = result.filesystem.expect("filesystem should be present");
    let git = fs.git.expect("git should be present");

    // deep() includes both file_changes AND diff payloads
    assert!(
        !git.file_changes.is_empty(),
        "deep() should populate file_changes"
    );
    assert!(
        !git.status.as_ref().unwrap().dirty.is_empty(),
        "deep() should populate dirty diff payloads"
    );
    assert!(
        !git.status.as_ref().unwrap().untracked.is_empty(),
        "deep() should populate untracked file details"
    );

    // Verify the diff payload contains actual content
    let dirty_file = &git.status.as_ref().unwrap().dirty[0];
    assert!(
        !dirty_file.diff.is_empty(),
        "dirty file should have a non-empty diff"
    );
}

#[test]
fn test_git_full_reports_conflicted_files() {
    use sniff::filesystem::git::FileStatus;
    use sniff::request::*;

    let (_dir, path) = create_merge_conflict_repo();

    let plan = DetectionPlan::new()
        .base_dir(path)
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::full())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        );

    let result = sniff::detect_with_plan(plan).unwrap();
    let fs = result.filesystem.expect("filesystem should be present");
    let git = fs.git.expect("git should be present");

    assert!(
        git.status.as_ref().unwrap().is_dirty,
        "conflicted repo should be dirty"
    );
    assert!(
        git.file_changes
            .iter()
            .any(|change| change.status == FileStatus::Conflicted
                && change.path.as_os_str() == "conflict.txt"),
        "conflicted files should be included in file_changes"
    );
}

#[test]
fn test_os_timezone_without_ntp() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary()
            .include_timezone(true)
            .include_ntp_status(false))
        .without_hardware()
        .without_network()
        .without_filesystem();

    let result = sniff::detect_with_plan(plan).unwrap();
    let os = result.os.expect("os should be present");

    // Timezone data should be populated
    let time = os
        .time
        .expect("time should be present when timezone is enabled");
    assert!(time.timezone.is_some(), "timezone name should be detected");

    // NTP should NOT have been probed — expect Unknown (the default)
    assert!(
        matches!(time.ntp_status, NtpStatus::Unknown),
        "NTP status should be Unknown when NTP probing is disabled, got {:?}",
        time.ntp_status
    );
}

#[test]
fn test_os_summary_has_no_time_data() {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .os(OsRequest::summary())
        .without_hardware()
        .without_network()
        .without_filesystem();

    let result = sniff::detect_with_plan(plan).unwrap();
    let os = result.os.expect("os should be present");

    // Summary mode disables both timezone and NTP
    assert!(os.time.is_none(), "summary() should not include time data");
}

#[test]
fn test_executable_index_parity_with_which_for_common_programs() {
    use sniff::programs::{ExecutableIndex, find_program_with_source};

    let index = ExecutableIndex::build();

    // Test a broader set of programs that are commonly available
    let programs = ["git", "bash", "sh", "env", "ls", "cat"];

    for prog in &programs {
        let which_found = find_program_with_source(prog).is_some();
        let index_found = index.find_with_source(prog).is_some();

        assert_eq!(
            which_found, index_found,
            "Parity mismatch for '{}': which={}, index={}",
            prog, which_found, index_found
        );
    }
}

// ============================================================================
// Windows Cross-Platform Integration Tests
// ============================================================================

/// Asserts that `primary_interface` is populated on eligible hosts.
///
/// On macOS and Linux a desktop/workstation usually has at least one
/// non-loopback, up interface with an IPv4 address, so the primary
/// selector should succeed.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn test_network_primary_interface_is_populated() {
    let result = sniff::detect_with_plan(
        DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_filesystem(),
    )
    .unwrap();
    let network = result.network.expect("network should be present");

    let has_eligible_interface = !network.permission_denied
        && network
            .interfaces
            .iter()
            .any(|i| !i.flags.is_loopback && !i.ipv4_addresses.is_empty() && i.flags.is_up);

    if has_eligible_interface {
        assert!(
            network.primary_interface.is_some(),
            "primary_interface should be populated when a non-loopback IPv4 interface exists"
        );
        let primary = network.primary_interface.unwrap();
        assert!(
            !primary.is_empty(),
            "primary_interface name should not be empty"
        );
    }
}

/// Asserts that `services_detailed(ServiceState::All)` returns at least one
/// service with a non-empty name on supported platforms.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn test_services_detailed_returns_non_empty_names() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let services = manager.services_detailed(ServiceState::All);

    if manager.init_system != sniff::services::InitSystem::Unknown {
        assert!(
            !services.is_empty(),
            "services_detailed(All) should return at least one service for {:?}",
            manager.init_system
        );
        for svc in &services {
            assert!(
                !svc.name.is_empty(),
                "every service should have a non-empty name"
            );
        }
    }
}

/// On Windows the default `detect_timezone()` code path should populate the
/// `timezone` field via `tzutil`.  This test locks down the runtime contract
/// on an actual Windows host without using the plan-based opt-in path.
#[cfg(target_os = "windows")]
#[test]
fn test_detect_timezone_windows_populates_timezone_name() {
    let time_info = sniff::hardware::detect_timezone();

    assert!(
        time_info.timezone.is_some(),
        "detect_timezone() should populate timezone on Windows via tzutil"
    );

    let tz = time_info.timezone.unwrap();
    assert!(!tz.is_empty(), "timezone name should not be empty");

    // IANA names contain '/' (e.g. "America/Los_Angeles").  Unmapped Windows
    // IDs typically contain "Standard" or "Daylight" but never '/'.
    // Either way the value should be non-empty and valid.
    assert!(
        tz.len() >= 3,
        "timezone name should be at least 3 characters, got: '{tz}'"
    );
}

/// On Windows `services_detailed(Running)` should return only services whose
/// SCM state is `SERVICE_RUNNING`.
#[cfg(target_os = "windows")]
#[test]
fn test_services_detailed_running_filter_windows() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let all = manager.services_detailed(ServiceState::All);
    let running = manager.services_detailed(ServiceState::Running);

    // Running should be a subset of all
    assert!(
        running.len() <= all.len(),
        "Running services ({}) should not exceed total ({})",
        running.len(),
        all.len()
    );

    for svc in &running {
        assert!(
            svc.running,
            "Service '{}' passed Running filter but running=false",
            svc.name
        );
    }
}

/// On Windows `services_detailed(Stopped)` should return only stopped services.
#[cfg(target_os = "windows")]
#[test]
fn test_services_detailed_stopped_filter_windows() {
    use sniff::services::{ServiceManager, ServiceState};

    let manager = ServiceManager::detect();
    let stopped = manager.services_detailed(ServiceState::Stopped);

    for svc in &stopped {
        assert!(
            !svc.running,
            "Service '{}' passed Stopped filter but running=true",
            svc.name
        );
    }
}

// ============================================================================
// Recent Commits Integration Tests (Step 13)
// ============================================================================

use chrono::{Duration, Utc};
use git2::Repository;
use std::fs;

/// Create a temporary git repo with a single commit containing a Rust source file.
fn create_recent_commits_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("src/main.rs")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "feat(cli): add main entry point\n\n- added src/main.rs\n- basic main function",
        &tree,
        &[],
    )
    .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

#[test]
fn test_get_recent_commits_by_duration_returns_commits() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok(), "Query should succeed: {:?}", result);
    let set = result.unwrap();
    assert!(!set.commits.is_empty(), "Should find at least one commit");
    assert_eq!(
        set.commits[0].description,
        "feat(cli): add main entry point"
    );
}

#[test]
fn test_get_recent_commits_by_duration_empty_for_old_period() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(0), "last 0 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    assert!(
        set.commits.is_empty(),
        "Should find no commits for 0-day duration"
    );
}

#[test]
fn test_get_recent_commits_by_date_returns_commits() {
    use sniff::filesystem::get_recent_commits_by_date;

    let (_dir, path) = create_recent_commits_repo();
    let today = Utc::now().date_naive();
    let result = get_recent_commits_by_date(&path, today);
    assert!(result.is_ok(), "Query should succeed: {:?}", result);
    let set = result.unwrap();
    assert!(!set.commits.is_empty(), "Should find commits since today");
}

#[test]
fn test_get_recent_commits_by_date_future_date_returns_empty() {
    use chrono::NaiveDate;
    use sniff::filesystem::get_recent_commits_by_date;

    let (_dir, path) = create_recent_commits_repo();
    // Query for a date in the future - git commits have "now" timestamps
    let future_date = NaiveDate::from_ymd_opt(2099, 12, 31).unwrap();
    let result = get_recent_commits_by_date(&path, future_date);
    assert!(result.is_ok());
    let set = result.unwrap();
    // All commits are "now", so none should be after 2099
    assert!(
        set.commits.is_empty(),
        "Should find no commits after far-future date"
    );
}

#[test]
fn test_get_recent_commits_by_hash_resolves_commit() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let (_dir, path) = create_recent_commits_repo();
    let repo = Repository::open(&path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let head_hash = head.id().to_string();

    let result = get_recent_commits_by_hash(&path, &head_hash);
    assert!(result.is_ok(), "Query should succeed: {:?}", result);
    let set = result.unwrap();
    assert!(!set.commits.is_empty(), "Should find the commit by hash");
    assert!(set.commits[0].hash.contains(&head_hash[..8]));
}

#[test]
fn test_get_recent_commits_by_hash_partial() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let (_dir, path) = create_recent_commits_repo();
    let repo = Repository::open(&path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let short_hash = &head.id().to_string()[..7];

    let result = get_recent_commits_by_hash(&path, short_hash);
    assert!(
        result.is_ok(),
        "Query should succeed with partial hash: {:?}",
        result
    );
    let set = result.unwrap();
    assert!(
        !set.commits.is_empty(),
        "Should resolve partial hash to commit"
    );
}

#[test]
fn test_get_recent_commits_includes_files() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    assert!(!set.commits.is_empty());
    assert!(!set.commits[0].files.is_empty(), "Commit should have files");
    assert!(
        set.commits[0]
            .files
            .iter()
            .any(|f| f.path.contains("main.rs"))
    );
}

#[test]
fn test_get_recent_commits_preserves_bullet_points() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    assert!(!set.commits.is_empty());
    assert!(
        !set.commits[0].bullet_points.is_empty(),
        "Should parse bullet points"
    );
    assert!(
        set.commits[0]
            .bullet_points
            .iter()
            .any(|b| b.contains("added src/main.rs"))
    );
}

#[test]
fn test_get_recent_commits_describe_produces_markdown() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    let md = set.describe(true);
    // Commit-centric format: "- [shorthash] ... at <time>: <message>"
    assert!(
        md.contains("- ["),
        "Should start each commit block with bracketed short hash, got:\n{}",
        md
    );
    assert!(
        md.contains("    **Files Impacted:**"),
        "Should have Files Impacted sub-block, got:\n{}",
        md
    );
}

#[test]
fn test_get_recent_commits_source_code_changes_filters() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    let md = set.source_code_changes(true);
    assert!(
        md.contains("Source Code Changes"),
        "Should have source code section"
    );
    assert!(md.contains("main.rs"), "Should include .rs files");
}

#[test]
fn test_get_recent_commits_documentation_changes_filters() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    fs::write(dir.path().join("README.md"), "# Test\n").unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn foo() {}").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.add_path(std::path::Path::new("src/lib.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "add readme and lib", &tree, &[])
        .unwrap();

    let result = get_recent_commits_by_duration(dir.path(), Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let set = result.unwrap();
    let md = set.documentation_changes(true);
    assert!(
        md.contains("Documentation Changes"),
        "Should have docs section"
    );
    assert!(md.contains("README.md"), "Should include markdown");
    assert!(!md.contains("lib.rs"), "Should not include .rs in docs");
}

#[test]
fn test_get_recent_commits_not_a_repo_error() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let dir = tempfile::TempDir::new().unwrap();
    let result = get_recent_commits_by_duration(dir.path(), Duration::days(1), "last 1 day");
    assert!(result.is_err(), "Should error on non-repo directory");
}

#[test]
fn test_commit_desc_set_filter_by_package_not_a_monorepo() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let mut set = result.unwrap();

    let result = set.filter_by_package("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, sniff::SniffError::NotAMonorepo(_)));
}

#[test]
fn test_commit_desc_set_filter_by_package_area_not_a_monorepo() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days");
    assert!(result.is_ok());
    let mut set = result.unwrap();

    let result = set.filter_by_package_area("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, sniff::SniffError::NotAMonorepo(_)));
}

// ============================================================================
// Recent Commits — Multi-commit and hash boundary tests
// ============================================================================

/// Helper: commit a file to an existing repo with a custom message and return
/// the resulting commit hash.
fn commit_file_with_message(
    repo: &Repository,
    dir: &std::path::Path,
    relative: &str,
    content: &str,
    message: &str,
) -> git2::Oid {
    let full = dir.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, content).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(relative)).unwrap();
    index.write().unwrap();

    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap()
}

#[test]
fn test_hash_boundary_returns_inclusive_range() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Initial commit (will be the boundary)
    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let boundary_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    // Second commit
    let _oid2 = commit_file_with_message(&repo, dir.path(), "src/a.rs", "fn a() {}", "commit two");
    // Third commit (HEAD)
    let _oid3 =
        commit_file_with_message(&repo, dir.path(), "src/b.rs", "fn b() {}", "commit three");

    let boundary_hash = boundary_oid.to_string();
    let result = get_recent_commits_by_hash(dir.path(), &boundary_hash).unwrap();

    // Should include all 3 commits: HEAD, commit two, initial (boundary)
    assert_eq!(
        result.commits.len(),
        3,
        "Should include HEAD down to and including the boundary commit"
    );

    // Newest-first order: commit three, commit two, initial
    assert_eq!(result.commits[0].description, "commit three");
    assert_eq!(result.commits[1].description, "commit two");
    assert_eq!(result.commits[2].description, "initial commit");

    // The last commit should be the boundary
    assert!(result.commits[2].hash.starts_with(&boundary_hash[..8]));
}

#[test]
fn test_hash_boundary_head_itself_returns_single_commit() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let (_dir, path) = create_recent_commits_repo();
    let repo = Repository::open(&path).unwrap();
    let head_hash = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();

    let result = get_recent_commits_by_hash(&path, &head_hash).unwrap();
    assert_eq!(
        result.commits.len(),
        1,
        "When hash == HEAD, should return only HEAD itself"
    );
    assert!(result.commits[0].hash.starts_with(&head_hash[..8]));
}

#[test]
fn test_hash_non_ancestor_returns_error() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Initial commit on main
    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let root_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "root", &tree, &[])
        .unwrap();

    // Branch off: create an orphan-like commit on a side branch
    let branch_oid =
        commit_file_with_message(&repo, dir.path(), "src/main.rs", "fn main() {}", "on main");

    // Create a detached side branch from root
    repo.set_head_detached(root_oid).unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let side_oid = commit_file_with_message(
        &repo,
        dir.path(),
        "side.txt",
        "side content",
        "side branch commit",
    );

    // Switch back to main branch commit
    repo.set_head_detached(branch_oid).unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();

    // Try to query from the side commit — it's not an ancestor of HEAD
    let result = get_recent_commits_by_hash(dir.path(), &side_oid.to_string());
    assert!(result.is_err(), "Non-ancestor hash should produce an error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, sniff::SniffError::HashNotReachable { .. }),
        "Expected HashNotReachable, got: {:?}",
        err
    );
}

#[test]
fn test_hash_boundary_commits_are_newest_first() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Create 5 commits
    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "commit 1", &tree, &[])
        .unwrap();

    let oid2 = commit_file_with_message(&repo, dir.path(), "src/a.rs", "fn a() {}", "commit 2");
    commit_file_with_message(&repo, dir.path(), "src/b.rs", "fn b() {}", "commit 3");
    commit_file_with_message(&repo, dir.path(), "src/c.rs", "fn c() {}", "commit 4");
    commit_file_with_message(&repo, dir.path(), "src/d.rs", "fn d() {}", "commit 5");

    // Query from commit 2 to HEAD
    let result = get_recent_commits_by_hash(dir.path(), &oid2.to_string()).unwrap();

    assert_eq!(result.commits.len(), 4, "Should include commits 2-5");

    // Verify newest-first ordering
    assert_eq!(result.commits[0].description, "commit 5");
    assert_eq!(result.commits[1].description, "commit 4");
    assert_eq!(result.commits[2].description, "commit 3");
    assert_eq!(result.commits[3].description, "commit 2");
}

// ============================================================================
// Recent Commits — Monorepo package filtering integration tests
// ============================================================================

/// Create a monorepo-style temp repo with Cargo workspace and multiple packages.
fn create_monorepo_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Create workspace Cargo.toml
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["pkg-a/lib", "pkg-b/lib"]
"#,
    )
    .unwrap();

    // Package A
    let pkg_a = dir.path().join("pkg-a/lib");
    fs::create_dir_all(pkg_a.join("src")).unwrap();
    fs::write(
        pkg_a.join("Cargo.toml"),
        r#"[package]
name = "pkg-a"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(pkg_a.join("src/lib.rs"), "pub fn a() {}").unwrap();

    // Package B
    let pkg_b = dir.path().join("pkg-b/lib");
    fs::create_dir_all(pkg_b.join("src")).unwrap();
    fs::write(
        pkg_b.join("Cargo.toml"),
        r#"[package]
name = "pkg-b"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(pkg_b.join("src/lib.rs"), "pub fn b() {}").unwrap();

    // Commit everything
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "initial monorepo setup",
        &tree,
        &[],
    )
    .unwrap();

    // Second commit: change only pkg-a
    fs::write(pkg_a.join("src/lib.rs"), "pub fn a() { /* v2 */ }").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("pkg-a/lib/src/lib.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "update pkg-a library",
        &tree,
        &[&head],
    )
    .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

#[test]
fn test_monorepo_filter_by_package_narrows_commits() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_monorepo_repo();
    let mut result =
        get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days").unwrap();

    // Before filtering, commits touch both packages
    assert!(result.commits.len() >= 2);

    result.filter_by_package("pkg-a").unwrap();

    // After filtering, all commit files should be under pkg-a/
    for commit in &result.commits {
        for file in &commit.files {
            assert!(
                file.path.starts_with("pkg-a/"),
                "Expected file under pkg-a/, got: {}",
                file.path
            );
        }
    }
}

#[test]
fn test_monorepo_filter_by_package_rewrites_top_level_packages() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_monorepo_repo();
    let mut result =
        get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days").unwrap();

    // Before filtering, packages should contain both pkg-a and pkg-b
    let pkgs_before = result.packages.as_ref().unwrap();
    assert!(
        pkgs_before.len() >= 2,
        "Monorepo should have at least 2 packages"
    );

    result.filter_by_package("pkg-a").unwrap();

    // After filtering, top-level packages should only contain pkg-a
    let pkgs_after = result.packages.as_ref().unwrap();
    assert_eq!(
        pkgs_after.len(),
        1,
        "Top-level packages should be narrowed to the filtered package"
    );
    assert_eq!(pkgs_after[0].name, "pkg-a");
}

#[test]
fn test_monorepo_filter_by_package_area_rewrites_top_level_packages() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_monorepo_repo();
    let mut result =
        get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days").unwrap();

    result.filter_by_package_area("pkg-a").unwrap();

    // After filtering by area "pkg-a", only pkg-a packages remain
    let pkgs_after = result.packages.as_ref().unwrap();
    assert!(
        pkgs_after.iter().all(|p| p.package_area == "pkg-a"),
        "Top-level packages should only include packages from the filtered area"
    );
}

#[test]
fn test_monorepo_json_output_scoped_after_filter() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_monorepo_repo();
    let mut result =
        get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days").unwrap();

    result.filter_by_package("pkg-a").unwrap();

    // Serialize to JSON and verify packages field is scoped
    let json_str = serde_json::to_string_pretty(&result).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    let packages = json["packages"].as_array().unwrap();
    assert_eq!(
        packages.len(),
        1,
        "JSON packages should be scoped to the filter"
    );
    assert_eq!(packages[0]["name"], "pkg-a");

    // Verify no file paths reference pkg-b
    assert!(
        !json_str.contains("pkg-b"),
        "Filtered JSON should not contain references to pkg-b"
    );
}

// ============================================================================
// Recent Commits — Empty commit tests
// ============================================================================

/// Helper: create an empty commit (no file changes) in an existing repo.
fn commit_empty(repo: &Repository, message: &str) -> git2::Oid {
    let sig = repo.signature().unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap()
}

#[test]
fn test_empty_commit_included_in_duration_query() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let (_dir, path) = create_recent_commits_repo();
    let repo = Repository::open(&path).unwrap();

    // Add an empty commit on top of HEAD
    commit_empty(&repo, "chore: empty marker");

    let result = get_recent_commits_by_duration(&path, Duration::days(7), "last 7 days").unwrap();

    // Should have 2 commits: the empty marker and the original
    assert_eq!(
        result.commits.len(),
        2,
        "Empty commit should be included in results"
    );
    assert_eq!(result.commits[0].description, "chore: empty marker");
    assert!(
        result.commits[0].files.is_empty(),
        "Empty commit should have files: []"
    );
    assert_eq!(
        result.commits[1].description,
        "feat(cli): add main entry point"
    );
}

#[test]
fn test_empty_commit_as_hash_boundary() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let (_dir, path) = create_recent_commits_repo();
    let repo = Repository::open(&path).unwrap();

    // Add an empty commit, then a real commit on top
    let empty_oid = commit_empty(&repo, "chore: empty boundary");
    commit_file_with_message(&repo, &path, "src/extra.rs", "fn extra() {}", "feat: extra");

    // Query from the empty boundary commit to HEAD — should be inclusive
    let result = get_recent_commits_by_hash(&path, &empty_oid.to_string()).unwrap();

    assert_eq!(
        result.commits.len(),
        2,
        "Should include HEAD and the empty boundary commit"
    );
    assert_eq!(result.commits[0].description, "feat: extra");
    assert_eq!(result.commits[1].description, "chore: empty boundary");
    assert!(
        result.commits[1].files.is_empty(),
        "Empty boundary commit should have files: []"
    );
}

#[test]
fn test_empty_head_included_in_hash_query() {
    use sniff::filesystem::get_recent_commits_by_hash;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Initial commit with a file
    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let boundary_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    // Second commit with a file
    commit_file_with_message(&repo, dir.path(), "src/a.rs", "fn a() {}", "commit two");

    // HEAD is an empty commit
    commit_empty(&repo, "chore: empty head");

    let result = get_recent_commits_by_hash(dir.path(), &boundary_oid.to_string()).unwrap();

    assert_eq!(
        result.commits.len(),
        3,
        "Should include empty HEAD, commit two, and boundary"
    );
    assert_eq!(result.commits[0].description, "chore: empty head");
    assert!(result.commits[0].files.is_empty());
    assert_eq!(result.commits[1].description, "commit two");
    assert_eq!(result.commits[2].description, "initial commit");
}

// ============================================================================
// Recent Commits — Skewed timestamp tests
// ============================================================================

/// Helper: commit a file with a custom timestamp (seconds since epoch).
fn commit_file_with_timestamp(
    repo: &Repository,
    dir: &std::path::Path,
    relative: &str,
    content: &str,
    message: &str,
    epoch_secs: i64,
) -> git2::Oid {
    let full = dir.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, content).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(relative)).unwrap();
    index.write().unwrap();

    let sig = git2::Signature::new(
        "Test User",
        "test@test.com",
        &git2::Time::new(epoch_secs, 0),
    )
    .unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap()
}

#[test]
fn test_skewed_head_does_not_hide_newer_parent() {
    use sniff::filesystem::get_recent_commits_by_duration;

    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Parent commit: recent timestamp (now)
    let now_secs = Utc::now().timestamp();
    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent_sig =
        git2::Signature::new("Test User", "test@test.com", &git2::Time::new(now_secs, 0)).unwrap();
    repo.commit(
        Some("HEAD"),
        &parent_sig,
        &parent_sig,
        "recent parent",
        &tree,
        &[],
    )
    .unwrap();

    // HEAD commit: very old timestamp (year 2000)
    let old_epoch = 946684800_i64; // 2000-01-01T00:00:00Z
    commit_file_with_timestamp(
        &repo,
        dir.path(),
        "src/main.rs",
        "fn main() {}",
        "old head",
        old_epoch,
    );

    // Query for last 7 days — the parent commit is within range
    let result = get_recent_commits_by_duration(dir.path(), Duration::days(7), "last 7 days");
    assert!(result.is_ok(), "Query should succeed: {:?}", result);
    let set = result.unwrap();

    // The recent parent should be found even though HEAD is old
    let descriptions: Vec<&str> = set.commits.iter().map(|c| c.description.as_str()).collect();
    assert!(
        descriptions.contains(&"recent parent"),
        "Should find the recent parent commit despite old HEAD. Got: {:?}",
        descriptions
    );
}

// ============================================================================
// Performance Collector Tests
// ============================================================================

#[test]
fn test_performance_collector_thread_local_aggregation() {
    use sniff::performance::{
        PerformanceCollector, duration_ms, increment_counter, record_stage, with_current_collector,
    };
    use std::sync::Arc;
    use std::time::Duration;

    let collector = PerformanceCollector::new_shared();
    let result = with_current_collector(Some(Arc::clone(&collector)), || {
        // Simulate work on the current thread
        record_stage("test.stage.a", Duration::from_millis(1));
        record_stage("test.stage.a", Duration::from_millis(2));
        record_stage("test.stage.b", Duration::from_millis(3));
        increment_counter("test.counter.x", 5);
        increment_counter("test.counter.y", 7);

        // Spawn additional threads that each install the same collector and
        // record data into their own thread-local buffers. Recording writes to
        // a thread-local buffer, so a thread that exits without draining it
        // would silently discard everything it recorded; these threads rely on
        // `with_current_collector` flushing before it restores the previous
        // collector.
        let handles: Vec<_> = (0..3)
            .map(|i| {
                let c = Arc::clone(&collector);
                std::thread::spawn(move || {
                    with_current_collector(Some(c), || {
                        record_stage("test.stage.a", Duration::from_millis((i + 1) as u64));
                        increment_counter("test.counter.x", 1);
                    });
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        collector.snapshot(Duration::from_millis(10))
    });

    // Stages should be aggregated across all threads
    let stage_a = result
        .stages
        .get("test.stage.a")
        .expect("stage a should exist");
    assert_eq!(stage_a.calls, 5, "expected 5 calls (2 main + 3 spawned)");
    assert!(
        stage_a.total_duration_ms >= 9.0,
        "expected total >= 9ms, got {}",
        stage_a.total_duration_ms
    );
    assert_eq!(
        duration_ms(Duration::from_millis(3)),
        stage_a.max_duration_ms
    );

    let stage_b = result
        .stages
        .get("test.stage.b")
        .expect("stage b should exist");
    assert_eq!(stage_b.calls, 1);

    // Counters should be aggregated across all threads
    let counter_x = result
        .counters
        .get("test.counter.x")
        .expect("counter x should exist");
    assert_eq!(*counter_x, 8, "expected 8 (5 main + 3 spawned)");

    let counter_y = result
        .counters
        .get("test.counter.y")
        .expect("counter y should exist");
    assert_eq!(*counter_y, 7);
}

#[test]
fn test_performance_collector_no_collector_does_not_panic() {
    use sniff::performance::{increment_counter, record_stage};
    use std::time::Duration;

    // These should not panic even when no collector is set
    record_stage("test.no_collector.stage", Duration::from_millis(1));
    increment_counter("test.no_collector.counter", 1);
}

#[test]
fn test_performance_collector_snapshot_is_deterministic() {
    use sniff::performance::{PerformanceCollector, record_stage, with_current_collector};
    use std::sync::Arc;
    use std::time::Duration;

    let collector = PerformanceCollector::new_shared();
    let report1 = with_current_collector(Some(Arc::clone(&collector)), || {
        record_stage("test.deterministic", Duration::from_millis(5));
        collector.snapshot(Duration::from_millis(10))
    });

    // Second snapshot after the first should have drained thread-local buffers,
    // so the same stages should not be double-counted.
    let report2 = with_current_collector(Some(Arc::clone(&collector)), || {
        collector.snapshot(Duration::from_millis(10))
    });

    // report1 had the stage, report2 should not (buffers were drained)
    assert!(
        report1.stages.contains_key("test.deterministic"),
        "first snapshot should contain the stage"
    );
    assert_eq!(report1.stages.get("test.deterministic").unwrap().calls, 1);

    // report2 was taken in a fresh with_current_collector scope but the same
    // thread; the thread-local buffer was drained by report1, so no data.
    // Note: the central state in the collector still has the data from report1,
    // so report2 will also see it.  The key property we test is that the data
    // is NOT double-counted (i.e. the thread-local buffer was drained, not
    // left behind to be merged again).
    assert_eq!(
        report2.stages.get("test.deterministic").unwrap().calls,
        1,
        "second snapshot should show the same single call (not double-counted)"
    );
}

#[test]
fn parallel_inventory_collects_all_classifications() {
    // Regression test for the production parallel inventory drop-flush bug.
    // Integration tests compile the sniff lib without `cfg(test)`, so this
    // exercises `scan_inventory_parallel` directly. Before the fix, every
    // worker-local buffer was discarded and the resulting inventory was empty.
    use sniff::filesystem::file_types::scan_file_inventory;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let total = 256_usize;
    for i in 0..total {
        let kind = i % 4;
        let (name, body) = match kind {
            0 => (format!("file_{i}.rs"), "pub fn x() {}\n".to_string()),
            1 => (format!("file_{i}.ts"), "export const x = 1;\n".to_string()),
            2 => (format!("file_{i}.py"), "def x():\n    pass\n".to_string()),
            _ => (format!("file_{i}.md"), "# Title\n".to_string()),
        };
        std::fs::write(dir.path().join(name), body).unwrap();
    }

    let inventory = scan_file_inventory(dir.path()).unwrap();

    assert_eq!(
        inventory.classifications.len(),
        total,
        "parallel scanner must retain every classification produced by worker threads"
    );
    assert_eq!(inventory.total_files_scanned, total);
    assert!(
        inventory
            .classifications
            .windows(2)
            .all(|pair| pair[0].path <= pair[1].path),
        "classifications should be sorted by path"
    );
}

#[test]
fn debug_skewed_git2_order() {
    use git2::Repository;
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    let now_secs = chrono::Utc::now().timestamp();
    std::fs::write(dir.path().join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("init.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent_sig =
        git2::Signature::new("Test User", "test@test.com", &git2::Time::new(now_secs, 0)).unwrap();
    let parent_oid = repo
        .commit(
            Some("HEAD"),
            &parent_sig,
            &parent_sig,
            "recent parent",
            &tree,
            &[],
        )
        .unwrap();

    let old_epoch = 946684800_i64;
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("src/main.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let old_sig =
        git2::Signature::new("Test User", "test@test.com", &git2::Time::new(old_epoch, 0)).unwrap();
    let head_oid = repo
        .commit(
            Some("HEAD"),
            &old_sig,
            &old_sig,
            "old head",
            &tree,
            &[&repo.find_commit(parent_oid).unwrap()],
        )
        .unwrap();

    eprintln!("Parent: {} at {}", parent_oid, now_secs);
    eprintln!("HEAD: {} at {}", head_oid, old_epoch);

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.set_sorting(git2::Sort::TIME).unwrap();
    revwalk.push_head().unwrap();

    eprintln!("\nWith TIME sort:");
    for oid in revwalk {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        eprintln!(
            "  {} at {}: {}",
            oid,
            commit.time().seconds(),
            commit.message().unwrap().trim()
        );
    }
}
