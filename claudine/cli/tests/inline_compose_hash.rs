//! Integration test for inline-compose hash stamping.
//!
//! Verifies that a real `claudine inline-compose` run writes a valid
//! Darkmatter `Simple` hash and that Darkmatter's canonical comparison reports
//! no difference for the resulting file.

use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;
mod common;
use common::CliProcessFixture;

/// Write a fake `goose` provider that edits `document` the way a file-aware
/// inline agent does, discoverable on `PATH` on every platform.
///
/// Uses a shell script on Unix and compiles a tiny native executable on
/// Windows. A batch file is not a valid stand-in here because the composed
/// prompt intentionally contains newlines. The body it writes is intentionally
/// *dirty* — a heading immediately followed by a paragraph with no blank line
/// between them — so the test also covers the cleanup→hash consistency path
/// (the document is normalized before the hash is stamped, and `md hash --diff`
/// must still match the normalized result).
fn write_goose_provider(bin_dir: &Path, document: &Path) {
    #[cfg(unix)]
    {
        common::InlineAgentStub::new(document)
            .body("# Replacement heading\nReplacement body content\n")
            .summary("Replaced the body.")
            .install(bin_dir, "goose");
    }
    #[cfg(windows)]
    {
        let source = bin_dir.join("goose-fixture.rs");
        common::write(
            &source,
            &format!(
                r##"fn main() {{
    let target = std::path::PathBuf::from(r"{document}");
    let current = std::fs::read_to_string(&target).expect("read the inline target");
    let mut delimiters = 0;
    let mut head = String::new();
    for line in current.split_inclusive('\n') {{
        if delimiters >= 2 {{
            break;
        }}
        if line.trim_end_matches(['\r', '\n']) == "---" {{
            delimiters += 1;
        }}
        head.push_str(line);
    }}
    std::fs::write(
        &target,
        format!("{{head}}# Replacement heading\nReplacement body content\n"),
    )
    .expect("write the inline target");
    println!("Replaced the body.");
}}
"##,
                document = document.display()
            ),
        );
        let output = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(bin_dir.join("goose.exe"))
            .output()
            .expect("rustc must build the Windows provider fixture");
        assert!(
            output.status.success(),
            "provider fixture compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn inline_compose_writes_hash_that_passes_md_diff() {
    let fixture = CliProcessFixture::named("inline-compose-hash");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate the body\nlast_updated: 2026-01-01\n---\nOriginal body\n",
    )
    .unwrap();

    write_goose_provider(fixture.bin_dir(), &md_file);

    fixture
        .command()
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("hash:"),
        "inline-compose must stamp a hash property; file:\n{final_content}"
    );
    assert!(
        final_content.contains("Replacement body content"),
        "inline-compose must write the replacement body; file:\n{final_content}"
    );

    // This is the canonical library path behind `md hash --diff`, and unlike a
    // sibling CLI process it remains available in a nextest archive.
    let markdown: darkmatter::markdown::Markdown = final_content.into();
    let options = claudine::composition::closure::inline_hash_options();
    let stored_value = markdown
        .frontmatter()
        .as_map()
        .get(&options.property)
        .expect("inline-compose should write the hash property");
    let stored = darkmatter::markdown::hash::StoredHash::parse(stored_value, &options.property)
        .expect("inline-compose should write a valid stored hash");
    let comparison = markdown
        .compare_hash(&stored, &options)
        .expect("stored hash should be comparable");
    assert!(
        !comparison.frontmatter_changed && !comparison.body_changed,
        "the stored hash must match the final document"
    );
}
