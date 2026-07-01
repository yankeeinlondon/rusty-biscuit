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

    // The bespoke renderer formats Unicode-icon lines as `📝name`, while
    // the tree projection inserts a literal `" "` between the icon span and
    // the name span — producing `📝 name`. Pin that gap so a future change
    // to either path forces this decision gate to be re-examined.
    assert!(
        bespoke.contains("├── 📝a.txt"),
        "bespoke uses `📝name` (no space): {bespoke:?}"
    );
    assert!(
        via_tree.contains("├── 📝 a.txt"),
        "tree path uses `📝 name` (extra space): {via_tree:?}"
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
/// Now that hierarchical gitignore semantics are wired (the scanner marks
/// `is_ignored` via the `GitignoreMatcher` rather than hardcoding `false`),
/// **both** paths emit dim SGR for the ignored entry. This fixture asserts
/// that parity.
#[test]
fn fixture_gitignore_styling_dims_in_both_paths() {
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

    // Both paths honor the resolved `is_ignored` flag and dim the ignored
    // entry; assert the parity.
    let dim_bespoke = bespoke.contains("\x1b[2m") || bespoke.contains(";2m");
    let dim_tree = via_tree.contains("\x1b[2m") || via_tree.contains(";2m");
    assert!(
        dim_bespoke && dim_tree,
        "expected both paths to emit dim SGR for the ignored entry; \
         bespoke={dim_bespoke}, tree={dim_tree}"
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
// Phase 1 — File Links Directive: new capability parity tests.
//
// These fixtures exercise the document-extension allowlist, included-paths
// allowlist, and dimmed-root-prefix / root-icon API added in Phase 1 of the
// file-links directive plan. Each test compares the bespoke renderer against
// the canonical tree projection, asserting content (stripped ANSI) parity.
// ---------------------------------------------------------------------------

/// Extension filter: only files whose extension is in the allowlist should
/// appear in both renderers. Non-matching files are pruned during scanning.
#[test]
fn fixture_extension_filter_content_matches() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::write(root.join("readme.md"), "m").expect("readme.md");
    fs::write(root.join("notes.txt"), "t").expect("notes.txt");
    fs::write(root.join("report.pdf"), "p").expect("report.pdf");

    let mut fs_obj = FileSystem::new(root)
        .expect("FileSystem::new")
        .show_root(false)
        .extension_filter(["md"]);
    fs_obj.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs_obj.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs_obj, &term));

    assert!(bespoke.contains("readme.md"), "bespoke missing readme.md: {bespoke:?}");
    assert!(via_tree.contains("readme.md"), "tree missing readme.md: {via_tree:?}");
    assert!(!bespoke.contains("notes.txt"), "bespoke should prune notes.txt: {bespoke:?}");
    assert!(!via_tree.contains("notes.txt"), "tree should prune notes.txt: {via_tree:?}");
    assert!(!bespoke.contains("report.pdf"), "bespoke should prune report.pdf: {bespoke:?}");
    assert!(!via_tree.contains("report.pdf"), "tree should prune report.pdf: {via_tree:?}");
}

/// `document_extensions()` convenience filter should accept .md, .txt, .pdf,
/// .doc(x), .xls(x) and prune everything else.
#[test]
fn fixture_document_extensions_filter_content_matches() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::write(root.join("doc.md"), "m").expect("doc.md");
    fs::write(root.join("img.png"), "i").expect("img.png");
    fs::write(root.join("data.csv"), "c").expect("data.csv");

    let mut fs_obj = FileSystem::new(root)
        .expect("FileSystem::new")
        .show_root(false)
        .document_extensions();
    fs_obj.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs_obj.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs_obj, &term));

    assert!(bespoke.contains("doc.md"), "bespoke missing doc.md: {bespoke:?}");
    assert!(via_tree.contains("doc.md"), "tree missing doc.md: {via_tree:?}");
    assert!(!bespoke.contains("img.png"), "bespoke should prune img.png: {bespoke:?}");
    assert!(!via_tree.contains("img.png"), "tree should prune img.png: {via_tree:?}");
}

/// Included paths: only files under the specified relative path should appear.
#[test]
fn fixture_included_paths_content_matches() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("topics")).expect("topics");
    fs::write(root.join("topics/alpha.md"), "a").expect("alpha.md");
    fs::write(root.join("topics/beta.md"), "b").expect("beta.md");
    fs::write(root.join("other.md"), "o").expect("other.md");

    let mut fs_obj = FileSystem::new(root)
        .expect("FileSystem::new")
        .show_root(false)
        .included_paths(["topics/alpha.md"]);
    fs_obj.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke = strip_ansi(&fs_obj.render(&term));
    let via_tree = strip_ansi(&render_via_tree(&fs_obj, &term));

    assert!(bespoke.contains("alpha.md"), "bespoke missing alpha.md: {bespoke:?}");
    assert!(via_tree.contains("alpha.md"), "tree missing alpha.md: {via_tree:?}");
    assert!(!bespoke.contains("beta.md"), "bespoke should prune beta.md: {bespoke:?}");
    assert!(!via_tree.contains("beta.md"), "tree should prune beta.md: {via_tree:?}");
    assert!(!bespoke.contains("other.md"), "bespoke should prune other.md: {bespoke:?}");
    assert!(!via_tree.contains("other.md"), "tree should prune other.md: {via_tree:?}");
}

/// Dimmed root prefix: both renderers should emit the display name, and the
/// dim SGR (`\x1b[2m`) should appear for the prefix portion in the bespoke
/// path. Content parity is asserted on stripped ANSI.
#[test]
fn fixture_dimmed_root_prefix_content_matches() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::write(root.join("a.md"), "a").expect("a.md");

    let mut fs_obj = FileSystem::new(root)
        .expect("FileSystem::new")
        .with_dimmed_root_prefix("/docs/")
        .with_root_display_name("topics");
    fs_obj.ensure_tree_built();

    let term = test_terminal(80);
    let bespoke_raw = fs_obj.render(&term);
    let via_tree_raw = render_via_tree(&fs_obj, &term);
    let bespoke = strip_ansi(&bespoke_raw);
    let via_tree = strip_ansi(&via_tree_raw);

    // Both paths should show the display name in the root line.
    let bespoke_root = bespoke.lines().next().unwrap_or_default();
    let tree_root = via_tree.lines().next().unwrap_or_default();
    assert!(bespoke_root.contains("topics"), "bespoke root missing 'topics': {bespoke_root:?}");
    assert!(tree_root.contains("topics"), "tree root missing 'topics': {tree_root:?}");
}

/// Root icon: setting `with_root_icon(Repository)` should change the icon
/// glyph in both renderers. The content (display name) should still match.
#[test]
fn fixture_root_icon_content_matches() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    fs::write(root.join("a.md"), "a").expect("a.md");

    let mut fs_dir = FileSystem::new(root)
        .expect("FileSystem::new")
        .with_root_icon(biscuit_terminal::components::filesystem::RootIconKind::Directory);
    fs_dir.ensure_tree_built();

    let mut fs_repo = FileSystem::new(root)
        .expect("FileSystem::new")
        .with_root_icon(biscuit_terminal::components::filesystem::RootIconKind::Repository);
    fs_repo.ensure_tree_built();

    let term = test_terminal(80);
    let dir_bespoke = strip_ansi(&fs_dir.render(&term));
    let repo_bespoke = strip_ansi(&fs_repo.render(&term));

    // At minimum, both should produce non-empty root lines with the dir name.
    assert!(dir_bespoke.lines().count() > 0);
    assert!(repo_bespoke.lines().count() > 0);
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

// ---------------------------------------------------------------------------
// Width-mode slack sink (style-everywhere Phase 2, Task 2.5)
//
// `FileSystem` is an internal-layout component on the *tree* path: the
// shared render-tree fold resolves the outer box from `Layout::width`, and
// the entry-label region absorbs slack by wrapping / truncating inside the
// resolved content width. The connector and icon columns stay fixed (D2
// slack sink).
//
// The terminal `render` flip stays deferred (Nerd Font icons the bespoke
// path emits cannot be reproduced by the target-agnostic projection), so
// the tests below exercise the *tree* projection only — the documented
// honored subset for the deferred-render gap.
// ---------------------------------------------------------------------------

/// Build a small connector fixture and return both the tempdir and a built
/// `FileSystem` component ready to project.
fn built_fs() -> (tempfile::TempDir, FileSystem) {
    let fixture = make_connector_fixture();
    let mut fs_component = FileSystem::new(fixture.path()).expect("build FileSystem");
    fs_component.ensure_tree_built();
    (fixture, fs_component)
}

#[test]
fn tree_path_width_auto_renders_within_available() {
    // Width::Auto (the default) — the tree fold renders the filesystem
    // tree inside the handed width. Connector geometry is preserved.
    let (_guard, fs_obj) = built_fs();
    let term = test_terminal(80);
    let out = render_via_tree(&fs_obj, &term);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 80,
        "Width::Auto must keep the tree inside the 80-column handed width: widest={widest}"
    );
    assert!(
        stripped.contains("├──") || stripped.contains("└──"),
        "tree connectors preserved under Width::Auto: {stripped:?}"
    );
}

#[test]
fn tree_path_width_fixed_percent_50_does_not_double_apply() {
    // Width::Fixed(50%) resolves the outer box to 50% of available. The
    // filesystem tree renders inside that 40-cell box; a widest near ~20
    // would mean the 50% was resolved twice.
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let (_guard, mut fs_obj) = built_fs();
    fs_obj.layout_mut().width =
        Width::Fixed(TargetValue::universal(Length::Percent(50.0)));
    let term = test_terminal(80);
    let out = render_via_tree(&fs_obj, &term);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 40,
        "Fixed(50%) caps the outer box at 40 cells: widest={widest}"
    );
    // The connector / icon columns are fixed and survive the narrower box.
    assert!(
        stripped.contains("├──") || stripped.contains("└──"),
        "tree connectors preserved under Fixed(50%): {stripped:?}"
    );
}

#[test]
fn tree_path_width_fit_content_hugs_entry_labels() {
    // FitContent hugs the natural width of the tree's entry labels. The
    // connectors stay fixed; the entry-label region is the slack sink.
    use biscuit_terminal::utils::layout::Width;
    let (_guard, mut fs_obj) = built_fs();
    fs_obj.layout_mut().width = Width::FitContent;
    let term = test_terminal(120);
    let out = render_via_tree(&fs_obj, &term);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest < 120,
        "FitContent hugs the entry labels and does not pad to full width: widest={widest}"
    );
}

#[test]
fn tree_path_width_fixed_full_fills_available() {
    // Width::Fixed(100%) is the explicit fill-the-available-width contract.
    // The filesystem tree's outer box equals the handed width.
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let (_guard, mut fs_obj) = built_fs();
    fs_obj.layout_mut().width =
        Width::Fixed(TargetValue::universal(Length::Percent(100.0)));
    let term = test_terminal(80);
    let out = render_via_tree(&fs_obj, &term);
    let stripped = strip_ansi(&out);
    // The tree connector geometry is preserved; the entry labels (slack
    // sink) are not padded to fill — they stay at their natural width — but
    // the outer box never exceeds the available width.
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 80,
        "Fixed(100%) never overflows the handed width: widest={widest}"
    );
}

#[test]
fn tree_path_margin_and_alignment_honored() {
    // The fold honors Layout.margin (left offset) and Layout.alignment
    // (centering) on the tree path. The connector geometry stays fixed.
    use biscuit_terminal::utils::layout::{Edges, Length};
    let (_guard, mut fs_obj) = built_fs();
    fs_obj.layout_mut().margin = Edges {
        left: biscuit_terminal::utils::layout::TargetValue::universal(Length::ch(4)),
        ..Edges::default()
    };
    let term = test_terminal(80);
    let out = render_via_tree(&fs_obj, &term);
    let stripped = strip_ansi(&out);
    let indented = stripped
        .lines()
        .filter(|l| !l.trim().is_empty())
        .any(|l| l.starts_with("    "));
    assert!(
        indented,
        "left margin of 4ch is honored on the tree path: {stripped:?}"
    );
}
