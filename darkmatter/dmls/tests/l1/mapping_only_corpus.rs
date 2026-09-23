//! Level-1 regression guard: frontmatter diagnostics on mapping-only documents
//! stay byte-identical across the sequence-descent change to the entry arena.
//!
//! The corpus is every `prompts/` document whose frontmatter holds no YAML
//! sequence at commit `3df43cc67`, frozen under
//! `fixtures/mapping_only_corpus/` (`/` in the source path spelled `__`), plus
//! the two sequence-free `suggest_constraint` fixtures. `baseline.json` was
//! captured from the unchanged overlay **before** `lower_mapping` learned to
//! descend into sequences (see
//! `claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md`).
//!
//! A deliberate diagnostic change on these documents must re-bless the baseline
//! with `DMLS_BLESS_MAPPING_CORPUS=1` and record why in the fix's verification
//! record; the bless run still fails so it can never pass silently.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dmls::capabilities::ClientProfile;
use dmls::config::{DmlsConfig, SchemaExtensionConfig};
use dmls::graph::WorkspaceGraph;
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{InitializeParams, Uri};
use serde_json::Value;

fn manifest_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
}

/// Every corpus document as `(stable name, absolute path)`, sorted by name.
fn corpus() -> Vec<(String, PathBuf)> {
    let fixtures = manifest_dir().join("tests/fixtures");
    let mut documents: Vec<(String, PathBuf)> = std::fs::read_dir(fixtures.join("mapping_only_corpus"))
        .expect("corpus directory")
        .map(|entry| entry.expect("corpus entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, path)
        })
        .collect();
    for name in ["inline.md", "raw-consumer.md"] {
        documents.push((
            format!("suggest_constraint/{name}"),
            fixtures.join("suggest_constraint").join(name),
        ));
    }
    documents.sort();
    documents
}

/// The configuration the repository's editors use for these documents: the
/// shipped Claudine extension activated for every Markdown file.
fn config() -> DmlsConfig {
    let mut config = DmlsConfig::default();
    config.schema.extensions.insert(
        "claudine".to_string(),
        SchemaExtensionConfig {
            path: manifest_dir().join("../docs/schemas/claudine.yaml"),
            globs: vec!["**/*.md".to_string()],
        },
    );
    config
}

/// All provider diagnostics for one document, as normalized JSON.
fn diagnostics_json(path: &Path, config: &DmlsConfig) -> Value {
    let text = std::fs::read_to_string(path).expect("read corpus document");
    let root = path.parent().expect("document directory").to_path_buf();
    let uri: Uri = url::Url::from_file_path(path).unwrap().as_str().parse().unwrap();
    let state = OverlayState::default();
    let overlay = state.for_document(&uri, &text, path, config, std::slice::from_ref(&root));
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text.as_str()));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
    let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let ctx = DocumentContext {
        uri: &uri,
        path,
        text: &text,
        source_map: &source_map,
        graph: &graph,
        doc_id: None,
        config,
        profile: &profile,
        overlay: overlay.as_ref(),
    };
    let diagnostics = ProviderRegistry::with_substrate().diagnostics(&ctx);
    let mut json = serde_json::to_value(diagnostics).expect("serialize diagnostics");
    normalize_paths(&mut json, &manifest_dir());
    json
}

/// Replaces host-specific absolute paths in messages so the baseline is
/// portable across checkouts and operating systems.
fn normalize_paths(value: &mut Value, manifest: &Path) {
    match value {
        Value::String(text) => {
            for spelling in [manifest.to_string_lossy().into_owned(), manifest.to_string_lossy().replace('\\', "/")] {
                if text.contains(&spelling) {
                    *text = text.replace(&spelling, "<DMLS>").replace('\\', "/");
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| normalize_paths(item, manifest)),
        Value::Object(map) => map.values_mut().for_each(|item| normalize_paths(item, manifest)),
        _ => {}
    }
}

#[test]
fn mapping_only_corpus_diagnostics_match_pre_descent_baseline() {
    let config = config();
    let mut observed = serde_json::Map::new();
    for (name, path) in corpus() {
        observed.insert(name, diagnostics_json(&path, &config));
    }
    let observed = Value::Object(observed);
    let baseline_path = manifest_dir().join("tests/fixtures/mapping_only_corpus/baseline.json");

    if std::env::var_os("DMLS_BLESS_MAPPING_CORPUS").is_some() {
        let rendered = serde_json::to_string_pretty(&observed).unwrap() + "\n";
        std::fs::write(&baseline_path, rendered).expect("write baseline");
        panic!("re-blessed {}; unset DMLS_BLESS_MAPPING_CORPUS and re-run", baseline_path.display());
    }

    let baseline: Value =
        serde_json::from_str(&std::fs::read_to_string(&baseline_path).expect("read baseline"))
            .expect("parse baseline");
    let baseline_names: Vec<&String> = baseline.as_object().unwrap().keys().collect();
    let observed_names: Vec<&String> = observed.as_object().unwrap().keys().collect();
    assert_eq!(observed_names, baseline_names, "corpus membership changed");
    for (name, expected) in baseline.as_object().unwrap() {
        assert_eq!(
            &observed[name], expected,
            "diagnostics for mapping-only document `{name}` drifted from the pre-descent baseline"
        );
    }
}

#[test]
fn mapping_only_corpus_contains_no_sequences() {
    // The guard is only meaningful if descent cannot reach these documents.
    for (name, path) in corpus() {
        let text = std::fs::read_to_string(&path).unwrap();
        let parse = dmls::overlay::FrontmatterAst::parse(&text).expect("frontmatter");
        let ast = parse.ast.expect("parses");
        assert!(
            ast.entries().iter().all(|entry| entry.kind != dmls::overlay::FmValueKind::Sequence),
            "`{name}` holds a sequence and does not belong in the mapping-only corpus"
        );
    }
}
