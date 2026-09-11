//! Shipped-review-router fixture shared by the provided-partial terminal tests.
//!
//! `fixes/2026-09-10-no-interactive-completion` reproduces its report through
//! the real `prompts/review.md`: a supplied `spec=` partial must be offered for
//! completion *before* the router's first `initialize` guard dereferences it
//! with `frontmatter(spec, ...)`. Two test binaries drive that flow — the PTY
//! suite (`level2_provided_partial_file_pty.rs`) for ordering and data flow,
//! and the real-terminal capture (`level2_provided_partial_file_capture.rs`)
//! for what a terminal emulator actually draws — and both must see byte-identical
//! topology, so the fixture lives here rather than in either binary.

use std::path::PathBuf;

use super::{CliProcessFixture, write, write_executable};

/// The partial the report supplied, matching one spec inside the launch area.
pub const PARTIAL: &str = "fixes/2026-09-10-local";

/// Seed the shipped router, its proxy target, spec candidates under the
/// `packages/example` launch area, and a same-substring decoy at the repository
/// root that only a mis-anchored candidate scope would find.
///
/// The `goose` stub records the prompt it receives at `$HOME/provider-prompt`
/// and prints `provider reached`, so a test can prove which selection crossed
/// the proxy hop without a real provider.
pub fn review_router_fixture(multiple: bool) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("review-router-partial");
    fixture.initialize_repository();
    fixture.seed_user_config();
    let router = fixture.cwd().join("prompts/review.md");
    write(&router, include_str!("../../../../prompts/review.md"));
    write(
        &fixture.cwd().join("prompts/_reviews/feature-review.md"),
        "---\n$schema:\n  spec: file(required;eager;match(**/*spec*.md))\nselected: \"{{ frontmatter(spec, 'marker') }}\"\n---\nSELECTED={{ selected }}\nSPEC={{ spec }}\nTOKEN={{ token }}\n",
    );
    write(
        &fixture.cwd().join("packages/example/fixes/2026-09-10-local-a/spec.md"),
        "---\nreviewed: true\nmarker: alpha\n---\nAlpha specification.\n",
    );
    if multiple {
        write(
            &fixture.cwd().join("packages/example/fixes/2026-09-10-local-b/spec.md"),
            "---\nreviewed: true\nmarker: beta\n---\nBeta specification.\n",
        );
    }
    // Decoy outside the launch area (`packages/example`) that matches the same
    // `fixes/2026-09-10-local` substring. Candidate discovery must walk from the
    // frozen launch origin, so this file is invisible to a package-area launch;
    // a scope anchored at the repository root — or recaptured from the ambient
    // CWD after the wrapper switches there — would pull it in and turn the
    // single-match confirmation into a chooser.
    write(
        &fixture.cwd().join("fixes/2026-09-10-local-decoy/spec.md"),
        "---\nreviewed: true\nmarker: decoy\n---\nDecoy specification outside the launch area.\n",
    );
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$HOME/provider-prompt\"\nprintf 'provider reached\\n'\n",
    );
    (fixture, router)
}

/// The launch area the report ran from: a package directory, not the root.
pub fn launch_area(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("packages/example")
}

/// Where the `goose` stub records the prompt it was handed.
pub fn provider_prompt(fixture: &CliProcessFixture) -> PathBuf {
    fixture.home().join("provider-prompt")
}

/// Assert the prompt the provider received names exactly the selected spec.
pub fn assert_provider_received(prompt: &str, selected: &str, directory: &str) {
    assert!(prompt.contains(&format!("SELECTED={selected}")), "prompt: {prompt}");
    assert!(prompt.contains(directory), "prompt: {prompt}");
    assert!(prompt.contains("TOKEN=retained"), "prompt: {prompt}");
    assert!(!prompt.contains("local-decoy"), "prompt: {prompt}");
}
