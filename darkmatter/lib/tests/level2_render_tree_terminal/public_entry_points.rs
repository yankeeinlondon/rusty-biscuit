use super::support::*;

#[test]
fn level2_render_probe_entrypoint() {
    let Ok(variant) = std::env::var(RENDER_PROBE_ENV) else {
        return;
    };
    render_probe_to_stdout(&variant);
    // Flush before exiting so libtest's trailing summary never races the
    // rendered bytes the pane capture asserts on.
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

#[test]
#[serial(level2_terminal)]
fn level2_public_as_terminal_entry_renders_in_real_terminal() {
    let body = "# Public Entry\n\nBody paragraph via the public API.\n\n\
                ```rust\nfn demo() {}\n```\n";
    let Some((frame, _dir)) = drive_pane(body, "public_as_terminal", render_public_as_terminal_to_tempfile)
    else {
        return;
    };

    for token in &["Public Entry", "Body paragraph via the public API.", "demo"] {
        assert!(
            frame.plain.contains(token),
            "public as_terminal token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
    // The cutover emits a language-label header pill for every fenced block.
    assert!(
        frame.plain.to_lowercase().contains("rust"),
        "fenced-code language header missing from public as_terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.raw.contains("\u{1b}["),
        "expected SGR styling in the public as_terminal capture. raw:\n{}",
        frame.raw
    );
}
