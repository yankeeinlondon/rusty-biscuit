//! A `retry.with` overlay reaches the retried attempt's composed prompt through
//! `claudine compose`, for that attempt only, without touching the document on
//! disk.

use std::fs;

use crate::common;
use common::{CliProcessFixture, strip_ansi};

/// The first provider attempt fails; `failure` retries once with an overlay
/// built from the source's own frontmatter, and the second attempt's prompt
/// reads the overlaid `fm.reason`.
#[test]
fn a_retry_overlay_reaches_the_second_attempts_prompt() {
    let fixture = CliProcessFixture::named("compose-retry-overlay");
    // Launch 0 fails, every later launch succeeds.
    let launches = common::launch_recorder::install(&fixture, "goose", "[ \"$n\" = 0 ] && exit 1\nexit 0");
    let md_file = fixture.cwd().join("retry.md");
    let document = r#"---
reason: first try
failure:
    - retry:
        max_attempts: 1
        with:
            reason: "{{ fm.reason }}, then retried"
---
reason={{ fm.reason }}
"#;
    fs::write(&md_file, document).unwrap();

    let output = fixture
        .command()
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "the retried attempt succeeds:\n{stderr}");

    // Goose receives the composed prompt as the argument after `-t`.
    let prompts: Vec<String> = common::launch_recorder::launches(&launches)
        .into_iter()
        .map(|argv| {
            let at = argv.iter().position(|arg| arg == "-t").expect("goose run -t <prompt>");
            argv[at + 1].trim().to_string()
        })
        .collect();
    assert_eq!(
        prompts,
        ["reason=first try", "reason=first try, then retried"],
        "the retried attempt, and only it, reads the overlay\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(&md_file).unwrap(),
        document,
        "the overlay is never written to disk"
    );
}
