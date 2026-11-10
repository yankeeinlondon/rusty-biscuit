//! The one controlled-PTY wiring test of render's no-input path: `md` and
//! `md render` with a terminal on stdin and no input print the root help on
//! stderr, nothing on stdout, and exit 1, without reading stdin.
//!
//! The decision itself is tested in process with an injected stdin state
//! (`render_dispatch::no_input`); only this test proves the binary reads the
//! real stream's terminal state. A detached tmux session supplies the
//! terminal, so no window takes focus; stdout and stderr go to files, so the
//! assertions read bytes rather than a screen.

use crate::common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use common::CliProcessFixture;
use common::level2::{link_or_copy, md_bin};
use std::path::Path;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

const WAIT: Duration = Duration::from_secs(20);

/// Waits until the pane's script has written `done`.
fn wait_for_done(cwd: &Path) {
    let deadline = Instant::now() + WAIT;
    while !cwd.join("done").exists() {
        assert!(Instant::now() < deadline, "the pane did not finish within {WAIT:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn read(cwd: &Path, name: &str) -> String {
    std::fs::read_to_string(cwd.join(name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn level2_render_no_input_on_a_terminal_prints_root_help_in_both_spellings() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let fixture = CliProcessFixture::named("level2-render-no-input");
    let md = fixture.bin_dir().join("md");
    link_or_copy(md_bin(), &md).expect("link md");
    let cwd = fixture.cwd();

    // Only stdin is the pane's terminal. Each status is written before
    // `done`, so the test waits on the file it asserts.
    let script = format!(
        "cd '{}' || exit 1; \
         if [ -t 0 ]; then echo terminal > stdin; fi; \
         md > root.out 2> root.err; echo $? > root.status; \
         md render > render.out 2> render.err; echo $? > render.status; \
         : > done",
        cwd.display()
    );
    let path = format!("PATH={}:/usr/bin:/bin", fixture.bin_dir().display());
    let home = format!("HOME={}", fixture.home().display());
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program("/usr/bin/env", &["-i", &path, &home, "TERM=xterm-256color", "LANG=C.UTF-8", "sh", "-c", &script])
        .expect("spawn the pane");
    wait_for_done(cwd);

    assert_eq!(read(cwd, "stdin").trim(), "terminal", "stdin is the pane's terminal");
    let root_help = read(cwd, "root.err");
    for expected in ["`md [ARGS] [OPTIONS]` is `md render [ARGS] [OPTIONS]`", "Commands:", "Shared controls:"] {
        assert!(root_help.contains(expected), "no {expected:?} in:\n{root_help}");
    }
    for spelling in ["root", "render"] {
        assert_eq!(read(cwd, &format!("{spelling}.status")).trim(), "1", "{spelling}");
        assert_eq!(read(cwd, &format!("{spelling}.out")), "", "{spelling}: stdout");
    }
    assert_eq!(read(cwd, "render.err"), root_help, "`md render` prints the root help too");
}
