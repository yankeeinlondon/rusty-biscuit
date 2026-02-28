use std::fs;

use tempfile::TempDir;

use crate::TreeFile;
use crate::analysis::analyze_file;
use crate::shared::AnalysisPass;

#[test]
fn pipeline_marks_completed_passes() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("sample.rs");
    fs::write(
        &file,
        "/// says hello\nfn hello(name: &str) -> String { format!(\"hi {name}\") }",
    )
    .expect("write fixture");

    let tree_file = TreeFile::new(&file).expect("parse");
    let index = analyze_file(
        &tree_file,
        &[
            AnalysisPass::Parse,
            AnalysisPass::Bind,
            AnalysisPass::Semantic,
            AnalysisPass::Docs,
        ],
    )
    .expect("analyze");

    assert!(index.completed_passes.contains(&AnalysisPass::Parse));
    assert!(index.completed_passes.contains(&AnalysisPass::Bind));
    assert!(index.completed_passes.contains(&AnalysisPass::Semantic));
    assert!(index.completed_passes.contains(&AnalysisPass::Docs));
    assert!(!index.symbols.is_empty());
}
