//! Level-2 (real-terminal) tests for the render-tree terminal renderer.
//!
//! These tests run inside a WezTerm pane via the shared `biscuit-test-harness`
//! so we observe the actual rendered bytes — ANSI styling, visible widths,
//! line wrapping — for output produced through the tree pipeline
//! (`fold_markdown_to_document` + `render_terminal_document`).
//!
//! Without Level 2 coverage the parity suite can only assert that the tokens
//! survive ANSI-stripped string comparison; it cannot catch regressions where
//! the rendered bytes *look* correct as a string but fail in an actual
//! terminal pane (column width, SGR ordering, wrapping behaviour).
//!
//! ## Mechanism
//!
//! Most tests pre-render in-process and replay the bytes into the pane:
//! 1. Fold a small fixture through the render-tree pipeline directly (so we
//!    test the canonical tree → terminal path, not the legacy
//!    `for_terminal` event-stream serializer).
//! 2. Write the rendered output to a temp file.
//! 3. Spawn a WezTerm pane and run `cat <tempfile>` so the bytes are
//!    actually emitted into a real terminal.
//! 4. Capture the rendered frame and assert on visible structure.
//!
//! ### Capability-detection tests render *inside* the pane
//!
//! Pre-rendering in the cargo-test process is wrong for any capability the
//! renderer resolves from the ambient terminal at render time — most notably
//! OSC8 hyperlink support, which `DarkmatterPage`'s no-geometry path decides
//! via `Terminal::default()` detection that short-circuits to "no OSC8"
//! whenever `is_tty()` is false. A cargo-test process has no controlling tty,
//! so an in-process render has already decided OSC8 is off before its bytes
//! ever reach the pane. These tests therefore execute the *renderer itself*
//! inside the terminal: [`drive_render_probe`] re-execs this test binary as a
//! foreground command in the pane (with `DM_L2_RENDER_PROBE` set so the
//! [`level2_render_probe_entrypoint`] test renders the page straight to the
//! real-tty stdout), giving terminal detection a real TTY to observe. Keeping
//! the probe inside the integration-test executable avoids adding a production
//! `bin` target to the Darkmatter library package.
//!
//! Tests skip cleanly when `WEZTERM_UNIX_SOCKET` is not set or the
//! `wezterm` binary is missing. Set `BISCUIT_TEST_LEVEL_REQUIRED=2` in the
//! environment to convert that into a hard failure — CI jobs that provision
//! WezTerm should set this so Level 2 coverage is enforced, not nominal.
//!
//! ## Bespoke page-path coverage
//!
//! Beyond the render-tree pipeline, this file also drives the **bespoke
//! [`DarkmatterPage`] path** — the path the reported code-block defects live in
//! (theme inversion, right-margin gap, pill/body boundary). Those tests render
//! the real repro layout at the pane's true width and assert on the captured
//! *cell grid* (inverted code-panel background, contiguous rectangle, shared
//! right boundary, blank-line rhythm) — properties an in-process ANSI string
//! cannot verify. See `run_page_in_pane` and the `level2_page_*` tests.

// Whitebox: these tests wire the deprecated `TerminalCodeRenderer` adapter
// directly to exercise the render-tree code path.
#![allow(deprecated)]

#[path = "level2_render_tree_terminal/code_panel.rs"]
mod code_panel;
#[path = "level2_render_tree_terminal/images.rs"]
mod images;
#[path = "level2_render_tree_terminal/file_links.rs"]
mod file_links;
#[path = "level2_render_tree_terminal/layout_policy.rs"]
mod layout_policy;
#[path = "level2_render_tree_terminal/public_entry_points.rs"]
mod public_entry_points;
#[path = "level2_render_tree_terminal/basic_spans.rs"]
mod basic_spans;
#[path = "level2_render_tree_terminal/support/mod.rs"]
mod support;
