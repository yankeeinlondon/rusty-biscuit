//! Integration test for inline-compose hash stamping.
//!
//! Verifies that a real `claudine inline-compose` run writes a valid
//! Darkmatter `Simple` hash and that Darkmatter's canonical comparison reports
//! no difference for the resulting file.

use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;
use tempfile::tempdir;
mod common;
use common::augmented_path;
use common::wrap::seed_minimal_config;

/// Write a fake `goose` provider that prints a fixed, deterministic replacement
/// body and exits 0, discoverable on `PATH` on every platform.
///
/// Uses a shell script on Unix and compiles a tiny native executable on
/// Windows. A batch file is not a valid stand-in here because the composed
/// prompt intentionally contains newlines. The replacement body is
/// intentionally *dirty* — a heading immediately followed by a paragraph with no
/// blank line between them — so the test also covers the cleanup→hash
/// consistency path (the document is normalized before the hash is stamped, and
/// `md hash --diff` must still match the normalized result).
fn write_goose_provider(bin_dir: &Path) {
    #[cfg(unix)]
    {
        common::write_executable(
            &bin_dir.join("goose"),
            "#!/bin/sh\nprintf '# Replacement heading\\nReplacement body content\\n'\nexit 0\n",
        );
    }
    #[cfg(windows)]
    {
        let source = bin_dir.join("goose-fixture.rs");
        common::write(
            &source,
            r##"fn main() {
    println!("# Replacement heading");
    println!("Replacement body content");
}
"##,
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
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate the body\nlast_updated: 2026-01-01\n---\nOriginal body\n",
    )
    .unwrap();

    write_goose_provider(&path_dir);

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        // `dirs::home_dir()` reads `HOME` on Unix and `USERPROFILE` on Windows;
        // set both so the wrapper's config home resolves to the temp workspace
        // on every platform.
        .env("HOME", workspace.path())
        .env("USERPROFILE", workspace.path())
        .env("PATH", augmented_path(&path_dir))
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
