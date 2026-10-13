use super::*;
use tempfile::TempDir;
use crate::classes::MemberFilter;
use tree_hugger::NamePattern;

fn write(root: &Path, file: &str, text: &str) -> PathBuf {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    path
}

fn scan(root: &Path, command: Declarations) -> DeclarationScan {
    DeclarationScan {
        command,
        files: Vec::new(),
        scan_root: root.to_path_buf(),
        git_root: root.to_path_buf(),
        selection: Selection::default(),
        prelude: false,
        listing: ListingOptions { group_by_file: true, group_by_module: false, sort: Default::default() },
        format: ReportFormat::Stats(None),
        verbose: false,
    }
}

fn json(stats: &Stats) -> serde_json::Value {
    let mut out = Vec::new();
    stats.write_json(&mut out, ".", ".", &[], &[]).unwrap();
    serde_json::from_slice::<serde_json::Value>(&out).unwrap()["stats"].clone()
}

fn collect(scan: &DeclarationScan, files: &[PathBuf], catalog: &Catalog, prelude: Option<&PreludeFilter>) -> (Tally, Vec<ReportError>) {
    let analyzer = Analyzer::new(None, None, false);
    let query = scan.command.query(scan.format, &scan.selection).scan_root(&scan.scan_root);
    let mut out = Vec::new();
    let mut stderr = Vec::new();
    let mut errors = Vec::new();
    let mut cache = CacheStats::new(false);
    let mut sink = Sink { out: &mut out, errors: &mut errors, stats: &mut cache, stderr: &mut stderr };
    let tally = tally(scan, files, |file| {
        let analyzed = analyzer.analyze(file, &query);
        FileOutcome {
            display: portable_relative(file, &scan.scan_root),
            cache: Some(CacheUse::Disabled),
            prepared: analyzed.map(|analyzed| prepare(scan, prelude, &query, analyzed.result)),
        }
    }, Some(catalog), &mut sink);
    (tally, errors)
}

#[test]
fn tally_preserves_groups_and_overall() {
    let root = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(root.path()).unwrap();
    write(&root, "Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\", \"empty\", \"failed\"]\n");
    for name in ["a", "b", "empty", "failed"] {
        write(&root, &format!("{name}/Cargo.toml"), &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"));
    }
    let files = vec![
        write(&root, "a/src/lib.rs", "/// Docs.\npub fn a() {}\n/// Docs.\npub fn b() {}\npub fn c() {}\n"),
        write(&root, "b/src/lib.rs", "pub fn d() {}\n"),
        write(&root, "empty/src/lib.rs", "pub struct Empty;\n"),
        root.join("failed/src/missing.rs"),
        write(&root, "loose.sh", "loose() { echo hello; }\n"),
    ];
    let catalog = Catalog::capture(ReportFormat::Stats(None), &root).unwrap().unwrap();
    let mut scan = scan(&root, Declarations::Functions);
    let (tally, errors) = collect(&scan, &files, &catalog, None);
    assert_eq!(errors.len(), 1, "one analysis failure, not one per group");
    assert_eq!(errors[0].file, "failed/src/missing.rs");
    assert_eq!(tally.count, 5);
    assert_eq!(tally.groups.len(), 5);
    let overall = json(&tally.stats);
    assert_eq!(overall["doc_coverage"]["documented"], 2);
    assert_eq!(overall["doc_coverage"]["total"], 4);
    assert_eq!(overall["doc_coverage"]["unavailable"], 1);
    assert_eq!(overall["doc_coverage"]["ratio"], 0.5);
    let groups: Vec<_> = tally.groups.iter().map(|group| json(&group.stats)).collect();
    for map in ["by_kind", "by_visibility", "by_language"] {
        let mut sums = BTreeMap::<String, u64>::new();
        for group in &groups {
            for (key, count) in group[map].as_object().unwrap() {
                *sums.entry(key.clone()).or_default() += count.as_u64().unwrap();
            }
        }
        assert_eq!(serde_json::to_value(sums).unwrap(), overall[map], "{map}");
    }
    for field in ["documented", "total", "unavailable"] {
        let sum: u64 = groups.iter().map(|group| group["doc_coverage"][field].as_u64().unwrap()).sum();
        assert_eq!(sum, overall["doc_coverage"][field].as_u64().unwrap(), "{field}");
    }
    let synthetic: u64 = groups.iter().map(|group| group["synthetic_exports"].as_u64().unwrap()).sum();
    assert_eq!(synthetic, overall["synthetic_exports"].as_u64().unwrap());
    for group in &groups[2..4] {
        assert_eq!(group["by_kind"], serde_json::json!({}));
        assert!(group["doc_coverage"]["ratio"].is_null());
    }
    assert!(tally.groups[4].package.is_none());
    scan.selection.names = vec![NamePattern::exact("no_match")];
    let (filtered, errors) = collect(&scan, &files, &catalog, None);
    assert_eq!(filtered.count, 0);
    assert_eq!(filtered.groups.len(), 5);
    assert_eq!(errors.len(), 1);
    for group in &filtered.groups { assert!(json(&group.stats)["doc_coverage"]["ratio"].is_null()); }
    let (empty, errors) = collect(&scan, &[], &catalog, None);
    assert_eq!(empty.count, 0);
    assert!(empty.groups.is_empty());
    assert!(errors.is_empty());
    assert_eq!(json(&empty.stats)["by_kind"], serde_json::json!({}));
}

#[test]
fn classes_tally_counts_only_containers() {
    let root = TempDir::new().unwrap();
    write(root.path(), "Cargo.toml", "[package]\nname = \"classes\"\nversion = \"0.1.0\"\n");
    let file = write(root.path(), "src/lib.rs", "pub struct Item { pub field: u8 }\nimpl Item { pub fn method(&self) {} }\n");
    let catalog = Catalog::capture(ReportFormat::Stats(None), root.path()).unwrap().unwrap();
    let command = Declarations::Classes(Box::new(ClassSelection::new(&Selection::default(), MemberFilter::default())));
    let (tally, errors) = collect(&scan(root.path(), command), &[file], &catalog, None);
    assert!(errors.is_empty());
    assert_eq!(tally.count, 1);
    assert_eq!(json(&tally.stats)["by_kind"], serde_json::json!({"Type": 1}));
    assert_eq!(json(&tally.groups[0].stats), json(&tally.stats));
}

#[test]
fn synthetic_export_belongs_to_prelude_not_target_location() {
    let root = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(root.path()).unwrap();
    write(&root, "Cargo.toml", "[package]\nname = \"exporter\"\nversion = \"0.1.0\"\n[workspace]\nmembers = [\".\", \"src/target\"]\n");
    write(&root, "src/target/Cargo.toml", "[package]\nname = \"target\"\nversion = \"0.1.0\"\n");
    let target = write(&root, "src/target/mod.rs", "/// Target documentation.\npub struct Exported;\n");
    let prelude_file = write(&root, "src/prelude.rs", "pub use crate::target::Exported;\n");
    let mut scan = scan(&root, Declarations::Symbols);
    scan.prelude = true;
    let analyzer = Analyzer::new(None, None, false);
    let query = SymbolQuery::new().capabilities([Capability::Metrics]);
    let prelude = resolve_prelude_symbols(&root, &analyzer, &query, &mut HashMap::new()).unwrap();
    let export = &prelude.exports_by_file[&prelude_file][0];
    assert_eq!(export.source.file_path, target, "target location must differ from attribution path");
    assert!(export.visibility.is_synthetic);
    let catalog = Catalog::capture(ReportFormat::Stats(None), &root).unwrap().unwrap();
    let (tally, errors) = collect(&scan, &[prelude_file], &catalog, Some(&prelude));
    assert!(errors.is_empty());
    assert_eq!(tally.count, 1);
    assert_eq!(tally.groups.len(), 1);
    assert_eq!(tally.groups[0].package.as_ref().unwrap().name, "exporter");
    let stats = json(&tally.stats);
    assert_eq!(stats["synthetic_exports"], 1);
    assert_eq!(stats["doc_coverage"]["total"], 0);
    assert!(stats["doc_coverage"]["ratio"].is_null());
    assert_eq!(json(&tally.groups[0].stats), stats);
}
