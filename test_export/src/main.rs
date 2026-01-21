use tree_hugger_lib::TreeFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tree_file = TreeFile::new("tree-hugger/lib/tests/fixtures/sample.js")?;
    let exports = tree_file.exported_symbols()?;

    println!("Found {} exported symbols:", exports.len());
    for export in &exports {
        println!(
            "  - {} ({}) at line {}",
            export.name, export.kind, export.range.start_line
        );
    }

    Ok(())
}
