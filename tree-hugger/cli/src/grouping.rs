//! Request-scoped package attribution for stats. Manifest identity never enters
//! symbol records or persistent cache keys. Output writers consume sorted counters.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sniff::filesystem::git::GitRepo;
use sniff::filesystem::repo::{
    PackageNameSource, PackageOwnershipLookup, RepoInfo, detect_repo_structure_or_root_package,
};

use crate::declarations::ReportFormat;
use crate::stats::Stats;

#[derive(Debug)]
pub(crate) struct Catalog {
    pub(crate) root: PathBuf,
    repo: Option<RepoInfo>,
}

impl Catalog {
    pub(crate) fn capture(format: ReportFormat, scan_root: &Path) -> Result<Option<Self>, String> {
        Self::capture_with(format, scan_root, |root| {
            detect_repo_structure_or_root_package(root).map_err(|error| error.to_string())
        })
    }

    fn capture_with(
        format: ReportFormat,
        scan_root: &Path,
        discover: impl FnOnce(&Path) -> Result<Option<RepoInfo>, String>,
    ) -> Result<Option<Self>, String> {
        if !matches!(format, ReportFormat::Stats(_)) {
            return Ok(None);
        }
        let capture = || {
            let git = GitRepo::discover(scan_root).map_err(|error| error.to_string())?;
            let root = git.as_ref().map_or(scan_root, |repo| repo.repo_root());
            let root = biscuit_file::canonicalize_simplified(root).map_err(|error| error.to_string())?;
            let repo = discover(&root)?;
            Ok(Self { root, repo })
        };
        capture().map(Some).map_err(|error: String| format!(
            "cannot capture package catalog for {}: {error}; check the repository and package manifests",
            biscuit_file::to_portable_string(scan_root),
        ))
    }
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct PackageIdentity {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Debug)]
pub(crate) struct Group {
    pub(crate) package: Option<PackageIdentity>,
    pub(crate) stats: Stats,
}

pub(crate) struct Grouping<'a> {
    ownership: Option<PackageOwnershipLookup<'a>>,
    groups: BTreeMap<PathBuf, Group>,
    unassigned: Option<Stats>,
}

impl<'a> Grouping<'a> {
    pub(crate) fn new(catalog: &'a Catalog) -> Self {
        Self {
            ownership: catalog.repo.as_ref().map(PackageOwnershipLookup::new),
            groups: BTreeMap::new(),
            unassigned: None,
        }
    }

    /// Called before settling analysis or iterating records, so a failed or
    /// zero-match selected file still contributes its package's zero counters.
    pub(crate) fn select_file(&mut self, file: &Path) -> &mut Stats {
        let owner = self.ownership.as_ref().and_then(|lookup| lookup.package_for_path(file));
        match owner.filter(|package| package.name_source == PackageNameSource::Manifest) {
            Some(package) => &mut self.groups.entry(package.path.clone()).or_insert_with(|| Group {
                package: Some(PackageIdentity {
                    name: package.name.clone(),
                    path: if package.relative.is_empty() {
                        ".".to_string()
                    } else {
                        biscuit_file::to_portable_string(Path::new(&package.relative))
                    },
                }),
                stats: Stats::default(),
            }).stats,
            None => self.unassigned.get_or_insert_with(Stats::default),
        }
    }

    pub(crate) fn finish(self) -> Vec<Group> {
        let mut groups: Vec<_> = self.groups.into_values().collect();
        groups.sort_by(|left, right| {
            left.package.as_ref().map(|package| (&package.path, &package.name))
                .cmp(&right.package.as_ref().map(|package| (&package.path, &package.name)))
        });
        if let Some(stats) = self.unassigned {
            groups.push(Group { package: None, stats });
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tempfile::TempDir;
    use crate::declarations::ReportFormat;
    use crate::output::View;
    use crate::render::RenderOptions;

    fn write(root: &Path, file: &str, text: &str) -> PathBuf {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn workspace(root: &Path) {
        write(root, "Cargo.toml", "[workspace]\nmembers = [\"pkg-a\", \"pkg-a/nested\", \"pkg-a2\", \"unused\"]\n");
        for (path, name) in [("pkg-a", Some("same")), ("pkg-a2", Some("same")), ("pkg-a/nested", None), ("unused", Some("unused"))] {
            let name = name.map_or(String::new(), |name| format!("name = \"{name}\"\n"));
            write(root, &format!("{path}/Cargo.toml"), &format!("[package]\n{name}version = \"0.1.0\"\n"));
        }
    }

    fn stats_json(stats: &Stats) -> serde_json::Value {
        let mut out = Vec::new();
        stats.write_json(&mut out, ".", ".", &[], &[]).unwrap();
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["stats"].clone()
    }

    #[test]
    fn catalog_requests_only_stats_once() {
        let root = TempDir::new().unwrap();
        let calls = Cell::new(0);
        let render = RenderOptions { styled: false, show_comments: false, width: None, signatures: Default::default() };
        for format in [ReportFormat::Count, ReportFormat::Json, ReportFormat::JsonFull, ReportFormat::JsonLines, ReportFormat::Terminal { render, view: View::List }] {
            let catalog = Catalog::capture_with(format, root.path(), |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            }).unwrap();
            assert!(catalog.is_none());
        }
        assert_eq!(calls.get(), 0);
        for format in [ReportFormat::Stats(None), ReportFormat::Stats(Some(render))] {
            let catalog = Catalog::capture_with(format, root.path(), |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            }).unwrap().unwrap();
            assert_eq!(catalog.root, biscuit_file::canonicalize_simplified(root.path()).unwrap());
            assert!(catalog.repo.is_none());
        }
        assert_eq!(calls.get(), 2);
        let error = Catalog::capture_with(ReportFormat::Stats(None), root.path(), |_| Err("broken manifest".into())).unwrap_err();
        assert!(error.contains("package catalog") && error.contains("broken manifest"), "{error}");
    }

    #[test]
    fn shipped_manifests_form_declared_groups() {
        let root = TempDir::new().unwrap();
        write(root.path(), "Cargo.toml", "[workspace]\nmembers = [\"lib\", \"cli\"]\n");
        for (dir, manifest) in [
            ("lib", include_str!("../../lib/Cargo.toml")),
            ("cli", include_str!("../Cargo.toml")),
        ] {
            write(root.path(), &format!("{dir}/Cargo.toml"), manifest);
            write(root.path(), &format!("{dir}/src/lib.rs"), "pub fn selected() {}\n");
        }
        let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
        let mut grouping = Grouping::new(&catalog);
        grouping.select_file(&root.path().join("lib/src/lib.rs"));
        grouping.select_file(&root.path().join("cli/src/lib.rs"));
        let groups = grouping.finish();
        let packages: Vec<_> = groups.iter().map(|g| g.package.as_ref().map(|p| (p.name.as_str(), p.path.as_str()))).collect();
        assert_eq!(packages, [Some(("tree-hugger-cli", "cli")), Some(("tree-hugger", "lib"))]);
    }

    #[test]
    fn ownership_ordering_and_zero_groups() {
        let root = TempDir::new().unwrap();
        workspace(root.path());
        let a = write(root.path(), "pkg-a/src/lib.rs", "pub fn a() {}\n");
        let nested = write(root.path(), "pkg-a/nested/src/lib.rs", "pub fn nested() {}\n");
        let a2 = write(root.path(), "pkg-a2/src/lib.rs", "pub fn a2() {}\n");
        let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
        let mut grouping = Grouping::new(&catalog);
        for file in [&a2, &nested, &a, &a] { grouping.select_file(file); }
        let groups = grouping.finish();
        assert_eq!(groups.len(), 3, "unused package absent, selected files retained once");
        let identities: Vec<_> = groups.iter().map(|group| group.package.as_ref().map(|package| (package.name.as_str(), package.path.as_str()))).collect();
        assert_eq!(identities, [Some(("same", "pkg-a")), Some(("same", "pkg-a2")), None]);
        for group in &groups {
            let stats = stats_json(&group.stats);
            assert_eq!(stats["by_kind"], serde_json::json!({}));
            assert_eq!(stats["doc_coverage"]["total"], 0);
            assert!(stats["doc_coverage"]["ratio"].is_null());
        }
        let mut reversed = Grouping::new(&catalog);
        for file in [&a, &nested, &a2] { reversed.select_file(file); }
        let reversed = reversed.finish();
        assert_eq!(groups.iter().map(|g| &g.package).collect::<Vec<_>>(), reversed.iter().map(|g| &g.package).collect::<Vec<_>>());
    }

    #[test]
    fn absence_outside_and_empty_selection() {
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
        assert!(Grouping::new(&catalog).finish().is_empty());
        let mut grouping = Grouping::new(&catalog);
        grouping.select_file(&write(root.path(), "lib.rs", ""));
        grouping.select_file(&write(outside.path(), "lib.rs", ""));
        let groups = grouping.finish();
        assert_eq!(groups.len(), 1);
        assert!(groups[0].package.is_none());
        write(root.path(), "Cargo.toml", "[package]\nname = \"root\"\nversion = \"0.1.0\"\n");
        let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
        let mut grouping = Grouping::new(&catalog);
        grouping.select_file(&root.path().join("lib.rs"));
        grouping.select_file(&outside.path().join("lib.rs"));
        let groups = grouping.finish();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].package.as_ref().unwrap().path, ".");
        assert!(groups[1].package.is_none());
    }

    #[test]
    fn catalog_roots_follow_active_worktree() {
        let root = TempDir::new().unwrap();
        workspace(root.path());
        let repo = git2::Repository::init(root.path()).unwrap();
        let nested = root.path().join("pkg-a");
        let catalog = Catalog::capture(ReportFormat::Stats(None), &nested).unwrap().unwrap();
        assert_eq!(catalog.root, biscuit_file::canonicalize_simplified(root.path()).unwrap());
        let tree = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree).unwrap();
        let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[]).unwrap();
        let linked_parent = TempDir::new().unwrap();
        let linked = linked_parent.path().join("linked");
        repo.worktree("linked", &linked, None).unwrap();
        workspace(&linked);
        let linked_catalog = Catalog::capture(ReportFormat::Stats(None), &linked.join("pkg-a")).unwrap().unwrap();
        assert_eq!(linked_catalog.root, biscuit_file::canonicalize_simplified(&linked).unwrap());
        assert_ne!(linked_catalog.root, catalog.root);
    }

    #[cfg(unix)]
    #[test]
    fn aliases_share_one_group() {
        let root = TempDir::new().unwrap();
        write(root.path(), "Cargo.toml", "[package]\nname = \"root\"\nversion = \"0.1.0\"\n");
        let file = write(root.path(), "src/lib.rs", "");
        std::os::unix::fs::symlink(root.path().join("src"), root.path().join("alias")).unwrap();
        let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
        let mut grouping = Grouping::new(&catalog);
        grouping.select_file(&file);
        grouping.select_file(&root.path().join("alias/lib.rs"));
        let groups = grouping.finish();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].package.as_ref().unwrap().name, "root");
    }
}
