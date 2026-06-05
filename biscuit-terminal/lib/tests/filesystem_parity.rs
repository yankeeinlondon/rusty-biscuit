//! Parity tests for the `FileSystem` component's IR migration.
//!
//! Stage 3a.1 established that [`TerminalRenderable::render_tree_node`] must
//! delegate to [`TreeRenderable::render_tree`] so cross-target adapters consume
//! `FileSystem` structurally. Stage 3a.3 uses this file as the **decision
//! gate** for flipping `FileSystem::render` itself from the bespoke renderer
//! to the canonical tree path (`render_via_tree`).
//!
//! The fixtures below cover every category called out in §3a.3 of the
//! Stage 3 spec:
//!
//! - connector geometry
//! - gitignore styling
//! - errors and permissions
//! - depth limits
//! - highlight precedence
//! - metric annotations
//! - dotfile italic
//! - symlink styling
//! - link behavior (OSC8)
//!
//! Each fixture renders the same `FileSystem` instance twice — once through
//! the bespoke `TerminalRenderable::render` path, and once by hand-driving
//! the canonical tree projection through `render_terminal_node` — and then
//! either asserts byte-for-byte parity (on stripped ANSI) or records the
//! exact divergence as a `#[test]` that pins the gap.
//!
//! The Stage 3a.1 contract tests live at the bottom of the file and are
//! retained verbatim so nothing in the structural projection regresses.

mod parity_helpers;

use std::fs;
use std::path::Path;

use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderStrictness, TreeRenderable};

use parity_helpers::{strip_ansi, test_terminal};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Render the canonical tree projection through the terminal renderer.
///
/// This is the path `FileSystem::render` would route through if Stage 3a.3
/// resolved to outcome (i) (flip to `render_via_tree`). It is hand-driven
/// here so the comparison does not depend on any production wiring.
fn render_via_tree(fs: &FileSystem, term: &Terminal) -> String {
    let node = <FileSystem as TreeRenderable>::render_tree(fs);
    let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
    render_terminal_node(&node, &opts)
        .expect("tree render should succeed")
        .output
}

/// Build a small directory fixture with `a.txt`, `b.txt`, and `sub/c.txt`.
fn make_connector_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::create_dir(root.join("sub")).expect("create sub");
    fs::write(root.join("a.txt"), "alpha").expect("a.txt");
    fs::write(root.join("b.txt"), "beta").expect("b.txt");
    fs::write(root.join("sub/c.txt"), "gamma").expect("c.txt");
    dir
}

/// Build a fixture containing a `.env` dotfile so the italic-dotfile fixture
/// has something to attach styling to.
fn make_dotfile_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::write(dir.path().join(".env"), "X=1").expect("create .env");
    dir
}

/// Build a fixture with `TODO-marker.txt` to exercise highlight precedence.
fn make_highlight_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::create_dir(dir.path().join("TODO-dir")).expect("TODO-dir");
    fs::write(dir.path().join("TODO-dir").join("kept.txt"), "x").expect("kept.txt");
    dir
}

/// Build a fixture for symlink styling. On platforms without symlink support
/// (or when symlinks cannot be created without elevation) this returns `None`
/// so the caller can skip the test rather than fail spuriously.
#[cfg(unix)]
fn make_symlink_fixture() -> Option<tempfile::TempDir> {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::write(dir.path().join("target.txt"), "t").expect("write target");
    std::os::unix::fs::symlink(dir.path().join("target.txt"), dir.path().join("link.txt")).ok()?;
    Some(dir)
}

#[cfg(not(unix))]
fn make_symlink_fixture() -> Option<tempfile::TempDir> {
    None
}

/// Build a fixture for depth-limit testing: root/a/b/c with files at each
/// level. The caller picks `max_depth` to stop the traversal early.
fn make_depth_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("a/b/c")).expect("nested dirs");
    fs::write(root.join("a/level1.txt"), "1").expect("level1");
    fs::write(root.join("a/b/level2.txt"), "2").expect("level2");
    fs::write(root.join("a/b/c/level3.txt"), "3").expect("level3");
    dir
}

/// Build a fixture for permission-error / error-dir testing. On Unix we drop
/// read bits on a subdirectory so the scanner records an error node. Returns
/// `None` on platforms or filesystems where this cannot be arranged.
#[cfg(unix)]
fn make_error_fixture() -> Option<tempfile::TempDir> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("create tempdir");
    let secret = dir.path().join("secret");
    fs::create_dir(&secret).ok()?;
    fs::write(secret.join("hidden.txt"), "hide me").ok()?;
    // `0o000` strips execute/read on the directory so the scanner records
    // an error node. Some CI sandboxes will not honor this and the test
    // skips itself.
    fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).ok()?;
    Some(dir)
}

#[cfg(not(unix))]
fn make_error_fixture() -> Option<tempfile::TempDir> {
    None
}

/// Restore permissions on an error fixture so `tempfile` can remove it
/// without leaving droppings in `$TMPDIR`.
#[cfg(unix)]
fn restore_error_fixture_perms(root: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(root.join("secret"), fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn restore_error_fixture_perms(_root: &Path) {}

// ---------------------------------------------------------------------------
// §3a.3 Decision-gate fixtures
//
// Each fixture is paired with a `_records_divergence` test that documents
// the exact mismatch found between the bespoke renderer and the tree path.
// The fixtures collectively form the evidence backing the decision recorded
// in `renderable/features/.../stage1-and-2/lessons-learned.md`.
// ---------------------------------------------------------------------------

/// Connector geometry: `├──` for non-last children, `└──` for the last child,
/// and `│   ` continuation rails for non-last ancestors. Both paths should
/// produce the same connector glyphs.
#[test]
fn fixture_connector_geometry_records_divergence() {
    let fixture = make_connector_fixture();
    let mut fs = FileSystem::new(fixture.path())
        .expect("FileSystem::new")
        .show_root(false);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs, &term));

    // The bespoke renderer formats Unicode-icon lines as `📂name`, while
    // the tree projection inserts a literal `" "` between the icon span and
    // the name span — producing `📂 name`. Pin that gap so a future change
    // to either path forces this decision gate to be re-examined.
    assert!(
        bespoke.contains("├── 📄a.txt"),
        "bespoke uses `📄name` (no space): {bespoke:?}"
    );
    assert!(
        via_tree.contains("├── 📄 a.txt"),
        "tree path uses `📄 name` (extra space): {via_tree:?}"
    );

    // Connector glyphs themselves match.
    for needle in ["├── ", "└── "] {
        assert!(bespoke.contains(needle), "bespoke missing {needle:?}");
        assert!(via_tree.contains(needle), "tree missing {needle:?}");
    }
}

/// Gitignore styling: ignored entries are rendered with `\x1b[2m` dim by the
/// bespoke renderer. The tree projection lowers `dim_gitignore` into typed
/// `Style { dim: true }` on the entry paragraph.
///
/// **Observed gap (Stage 3a.3):** The scanner currently hardcodes
/// `is_ignored: false` on every entry (see `FileSystem::scan_dir`: "Will be
/// set properly with ignore crate in Phase 8"). Neither path actually emits
/// dim SGR for any entry today, so this fixture can only assert that both
/// paths agree on the **absence** of dim. When Phase 8 lands, this fixture
/// will flip to a real divergence — the tree path's
/// `render_tree_connector_list` drops the per-item paragraph `Style` (see
/// the dotfile fixture's recorded gap) and will not emit `\x1b[2m` until
/// that is fixed.
#[test]
fn fixture_gitignore_styling_records_divergence() {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::write(dir.path().join(".gitignore"), "ignored.txt\n").expect(".gitignore");
    fs::write(dir.path().join("ignored.txt"), "x").expect("ignored.txt");
    fs::write(dir.path().join("kept.txt"), "y").expect("kept.txt");

    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false)
        .dim_gitignore(true);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    // Both paths currently emit zero dim SGR because `is_ignored` is always
    // false. Document the joint absence so a future Phase-8 commit forces
    // this fixture to be revisited.
    let dim_bespoke = bespoke.contains("\x1b[2m") || bespoke.contains(";2m");
    let dim_tree = via_tree.contains("\x1b[2m") || via_tree.contains(";2m");
    assert!(
        !dim_bespoke && !dim_tree,
        "expected both paths to emit no dim SGR today (is_ignored is hardcoded \
         to false); bespoke={dim_bespoke}, tree={dim_tree}"
    );

    // Content (icon-name spacing aside) is otherwise present in both paths.
    assert!(strip_ansi(&bespoke).contains("ignored.txt"));
    assert!(strip_ansi(&via_tree).contains("ignored.txt"));
}

/// Errors and permissions: an error directory should render the warning
/// glyph and red styling in both paths.
///
/// **Observed gap (Stage 3a.3):** Where the scanner actually marks a node
/// `has_error: true`, the bespoke path emits `\x1b[31m` for it. The tree
/// path emits **no** red SGR for that entry because
/// `render_tree_connector_list` discards the per-item `Paragraph` `Style`
/// (see the dotfile fixture). When the sandbox does not honor `0o000`
/// permission stripping (some CI tmpfs), no error node is produced and the
/// fixture self-skips. Even on platforms that do produce the error node,
/// the recorded divergence is "bespoke emits red; tree emits no styling at
/// all" — the same root-cause gap as the other styling fixtures.
#[test]
fn fixture_errors_and_permissions_records_divergence() {
    let Some(dir) = make_error_fixture() else {
        eprintln!("skipping: cannot create permission-error fixture on this platform");
        return;
    };

    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    let stripped_bespoke = strip_ansi(&bespoke);
    let stripped_tree = strip_ansi(&via_tree);

    let red_in_bespoke = bespoke.contains("\x1b[31m") || bespoke.contains(";31m");
    let red_in_tree = via_tree.contains("\x1b[31m") || via_tree.contains(";31m");

    if !red_in_bespoke {
        // Sandbox did not honor permission stripping; no error node was
        // recorded. Skip rather than fail spuriously, but still pin the
        // shared-content assertion so we know the entry was projected.
        assert!(stripped_bespoke.contains("secret"));
        assert!(stripped_tree.contains("secret"));
        restore_error_fixture_perms(dir.path());
        eprintln!(
            "skipping red-SGR assertion: sandbox did not produce a has_error node; \
             both paths rendered the entry without error styling"
        );
        return;
    }

    // Bespoke produced an error node; the tree path **must** match by
    // emitting red SGR. Today it does not — the gap is the same connector
    // styling issue, just exposed on a different fixture. The assertion is
    // intentionally one-sided: it pins the production capability.
    assert!(
        red_in_tree,
        "GAP: bespoke emitted red for error dir but tree path did not. \
         bespoke={bespoke:?} tree={via_tree:?}"
    );

    restore_error_fixture_perms(dir.path());
}

/// Depth limit: at `max_depth = 1` only the first level of children should
/// appear; deeper entries are hidden and the parent directory is marked as
/// at-depth-limit by the scanner.
#[test]
fn fixture_depth_limit_records_divergence() {
    let dir = make_depth_fixture();
    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false)
        .depth(1);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs, &term));

    // `a` is at depth 1 and visible in both paths. Anything below `a` is
    // hidden by the depth limiter.
    assert!(bespoke.contains('a'));
    assert!(via_tree.contains('a'));
    assert!(!bespoke.contains("level2"));
    assert!(!via_tree.contains("level2"));
}

/// Highlight precedence: `highlight_red("TODO")` makes a directory red and
/// strips the bold/blue treatment that directories normally receive.
///
/// Both paths now emit `\x1b[31m` for the `TODO-dir` entry. The connector-list
/// terminal renderer applies the per-item paragraph `Style` projected by
/// `fs_entry_style` (tree-cutover Phase 4 connector-list `Style` lowering), so
/// the styling divergence recorded in Stage 3a.3 is closed.
#[test]
fn fixture_highlight_precedence_styling_matches() {
    let dir = make_highlight_fixture();
    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false)
        .highlight_red("TODO");
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    let red_in_bespoke = bespoke.contains("\x1b[31m") || bespoke.contains(";31m");
    let red_in_tree = via_tree.contains("\x1b[31m") || via_tree.contains(";31m");

    assert!(
        red_in_bespoke,
        "bespoke must emit highlight red: {bespoke:?}"
    );
    assert!(
        red_in_tree,
        "tree path must emit highlight red now that the connector list applies \
         the per-item paragraph Style: {via_tree:?}"
    );

    assert!(strip_ansi(&bespoke).contains("TODO-dir"));
    assert!(strip_ansi(&via_tree).contains("TODO-dir"));
}

/// Metric annotations: with `show_file_size()` enabled, both paths should
/// emit a `( file size: N B )` suffix after the file name.
#[test]
fn fixture_metric_annotations_records_divergence() {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::write(dir.path().join("data.bin"), vec![0u8; 42]).expect("data.bin");

    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false)
        .show_file_size();
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs, &term));

    // Both paths must surface the metric pair text.
    assert!(
        bespoke.contains("file size:") && bespoke.contains("42 B"),
        "bespoke missing metric pair: {bespoke:?}"
    );
    assert!(
        via_tree.contains("file size:") && via_tree.contains("42 B"),
        "tree missing metric pair: {via_tree:?}"
    );
}

/// Dotfile italic: `italicize_dot_files(true)` should make `.env` italic in
/// both renderers.
///
/// Both paths now emit `\x1b[3m` for `.env`. The connector-list terminal
/// renderer applies the per-item paragraph `Style` (tree-cutover Phase 4), so
/// the styling divergence recorded in Stage 3a.3 is closed.
#[test]
fn fixture_dotfile_italic_styling_matches() {
    let dir = make_dotfile_fixture();
    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false)
        .italicize_dot_files(true);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    let italic_in_bespoke = bespoke.contains("\x1b[3m") || bespoke.contains(";3m");
    let italic_in_tree = via_tree.contains("\x1b[3m") || via_tree.contains(";3m");

    assert!(
        italic_in_bespoke,
        "bespoke must emit italic SGR: {bespoke:?}"
    );
    assert!(
        italic_in_tree,
        "tree path must emit italic now that the connector list applies the \
         per-item paragraph Style: {via_tree:?}"
    );

    assert!(strip_ansi(&bespoke).contains(".env"));
    assert!(strip_ansi(&via_tree).contains(".env"));
}

/// Symlink styling: cyan (`\x1b[36m`) on the entry. Skipped on platforms
/// where symlink creation is not possible without elevation.
///
/// When the scanner marks the symlink, both paths now emit `\x1b[36m`: the
/// connector-list terminal renderer applies the per-item paragraph `Style`
/// (tree-cutover Phase 4), so the styling divergence recorded in Stage 3a.3 is
/// closed.
#[test]
fn fixture_symlink_styling_matches() {
    let Some(dir) = make_symlink_fixture() else {
        eprintln!("skipping: symlink creation unsupported on this platform");
        return;
    };

    let mut fs = FileSystem::new(dir.path())
        .expect("FileSystem::new")
        .show_root(false);
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    let cyan_in_bespoke = bespoke.contains("\x1b[36m") || bespoke.contains(";36m");
    let cyan_in_tree = via_tree.contains("\x1b[36m") || via_tree.contains(";36m");

    if cyan_in_bespoke {
        // Scanner marked the symlink; the tree path now emits cyan too because
        // the connector list applies the per-item Style.
        assert!(
            cyan_in_tree,
            "tree path must emit cyan now that the connector list applies the \
             per-item Style: {via_tree:?}"
        );
    } else {
        // Scanner did not mark the symlink (some platforms / fs combos);
        // both paths agree on plain content, no styling.
        assert!(
            !cyan_in_tree,
            "tree unexpectedly emitted cyan: {via_tree:?}"
        );
        eprintln!("note: scanner did not flag the symlink; both paths emitted plain text");
    }

    assert!(strip_ansi(&bespoke).contains("link.txt"));
    assert!(strip_ansi(&via_tree).contains("link.txt"));
}

/// Link behavior (OSC8): with `with_file_links()` enabled, both renderers
/// should emit an OSC8 hyperlink sequence pointing at the absolute file
/// path. The bespoke renderer composes the link via `Prose::new("<a
/// href=...>")`; the tree projection emits a `NodeKind::Link` whose URL
/// terminal rendering lowers to OSC8.
#[test]
fn fixture_link_osc8_records_divergence() {
    let fixture = make_connector_fixture();
    let mut fs = FileSystem::new(fixture.path())
        .expect("FileSystem::new")
        .show_root(false)
        .with_file_links();
    fs.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = fs.render(&term);
    let via_tree = render_via_tree(&fs, &term);

    // The OSC8 link introducer is `\x1b]8;;` followed by a URL and a `\x07`
    // (BEL) terminator. Both paths should emit at least one of these.
    let osc8 = "\x1b]8;;";
    assert!(
        bespoke.contains(osc8),
        "bespoke missing OSC8 hyperlink: {bespoke:?}"
    );
    assert!(
        via_tree.contains(osc8),
        "tree missing OSC8 hyperlink: {via_tree:?}"
    );

    // Both should target absolute `file://` URLs.
    assert!(
        bespoke.contains("file://") && via_tree.contains("file://"),
        "missing file:// scheme on at least one path"
    );
}

// ---------------------------------------------------------------------------
// Stage 3a.1 contract tests (retained verbatim from the prior file).
//
// `render_tree_node` must delegate to `TreeRenderable::render_tree` so the
// structural-projection hook used when this component is nested inside a
// parent component matches the canonical tree producer byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_matches_render_tree_for_filesystem() {
    let fixture = make_connector_fixture();
    let mut fs_component = FileSystem::new(fixture.path()).expect("build FileSystem");
    fs_component.ensure_tree_built();

    let from_tree = <FileSystem as TreeRenderable>::render_tree(&fs_component);
    let from_hook = <FileSystem as TerminalRenderable>::render_tree_node(&fs_component)
        .expect("render_tree_node should return Some");
    assert_eq!(
        from_tree, from_hook,
        "FileSystem::render_tree_node must delegate to TreeRenderable::render_tree"
    );
}

#[test]
fn render_tree_node_matches_render_tree_without_ensure_tree_built() {
    // Projection is read-only, so a component whose tree was never built
    // still produces a tree (the empty-root form). Both entry points must
    // agree on that empty projection too.
    let fixture = make_connector_fixture();
    let fs_component = FileSystem::new(fixture.path()).expect("build FileSystem");

    let from_tree = <FileSystem as TreeRenderable>::render_tree(&fs_component);
    let from_hook = <FileSystem as TerminalRenderable>::render_tree_node(&fs_component)
        .expect("render_tree_node should return Some");
    assert_eq!(from_tree, from_hook);
}

#[test]
fn render_tree_node_matches_render_tree_with_formatting_constructor() {
    let fixture = make_connector_fixture();
    let mut fs_component =
        FileSystem::new_with_formatting(fixture.path()).expect("build FileSystem");
    fs_component.ensure_tree_built();

    let from_tree = <FileSystem as TreeRenderable>::render_tree(&fs_component);
    let from_hook = <FileSystem as TerminalRenderable>::render_tree_node(&fs_component)
        .expect("render_tree_node should return Some");
    assert_eq!(from_tree, from_hook);
}
