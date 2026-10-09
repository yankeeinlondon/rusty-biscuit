//! The Goose stub and compose runner shared by the caller-file provenance
//! tests, in Level 1 and in the opt-in `prompts` binary.

use super::{CliProcessFixture, strip_ansi};
#[cfg(windows)]
use super::write;
#[cfg(unix)]
use super::write_executable;

/// A Goose stub that records its delivered prompt and, when
/// `CLAUDINE_INLINE_TARGET` names a document, edits that document's body the
/// way a file-aware inline agent does.
pub fn install_goose(fixture: &CliProcessFixture) {
    #[cfg(unix)]
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" > \"$HOME/provider-prompt\"\n\
             if [ -n \"$CLAUDINE_INLINE_TARGET\" ]; then\n\
             CLAUDINE_DOC=\"$CLAUDINE_INLINE_TARGET\"\n\
             CLAUDINE_ADD=''\n\
             CLAUDINE_BODY='provider reached\n'\n\
             {rewrite}\
             fi\n\
             printf 'provider reached\\n'\n",
            rewrite = super::INLINE_BODY_REWRITE
        ),
    );
    // A `.cmd` stub cannot receive the multi-line shipped prompts: Rust refuses
    // to spawn a batch file whose arguments contain newlines. Compile a tiny
    // native provider instead, mirroring `inline_compose_hash.rs`.
    #[cfg(windows)]
    {
        let source = fixture.bin_dir().join("goose-fixture.rs");
        write(
            &source,
            r##"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .expect("a home directory for the provider prompt");
    std::fs::write(
        std::path::Path::new(&home).join("provider-prompt"),
        format!("{}\n", args.join(" ")),
    )
    .expect("write provider prompt");
    if let Ok(target) = std::env::var("CLAUDINE_INLINE_TARGET") {
        let current = std::fs::read_to_string(&target).expect("read the inline target");
        let mut delimiters = 0;
        let mut head = String::new();
        for line in current.split_inclusive('\n') {
            if delimiters >= 2 {
                break;
            }
            if line.trim_end_matches(['\r', '\n']) == "---" {
                delimiters += 1;
            }
            head.push_str(line);
        }
        std::fs::write(&target, format!("{head}provider reached\n"))
            .expect("write the inline target");
    }
    println!("provider reached");
}
"##,
        );
        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(fixture.bin_dir().join("goose.exe"))
            .output()
            .expect("rustc must build the Windows provider fixture");
        assert!(
            output.status.success(),
            "provider fixture compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

pub fn run_compose(
    fixture: &CliProcessFixture,
    cwd: &std::path::Path,
    document: &std::path::Path,
    setters: &[&str],
) -> String {
    // Escape: caller-relative references must resolve from this fixture directory.
    let mut command = fixture.command_builder().ambient_context(cwd).build();
    let audio_spool = fixture.cwd().join("provenance-audio-spool");
    // Shipped templates retain their lifecycle actions; provenance tests must not play them.
    command
        .env("PLAYA_DRY_RUN", "1")
        .env("PLAYA_SPOOL_DIR", &audio_spool)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["compose", "--goose", document.to_str().unwrap()]);
    command.args(setters);
    let assertion = command.assert().success();
    assert!(!audio_spool.exists(), "provenance tests must not publish audio");
    strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr))
}
