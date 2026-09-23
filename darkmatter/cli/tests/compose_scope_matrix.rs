//! The empty-string scope table (AC21) and its conditional truthiness (AC23),
//! through the ambient `md compose` path.
//!
//! AC21 asks for both legs of a dual path. `darkmatter/lib/tests/empty_package_area.rs`
//! is the *supplied-evidence* leg: it hands composition a `sniff` repository
//! observation it built in process. That cannot say what a real `md compose`
//! discovers for itself, which is the ambient leg — the launch directory, the
//! repository walk, and the scope projection assembled by the binary. These
//! cases are that leg, and they add the fifth tabled position (outside a
//! monorepo) that the supplied-evidence leg does not reach.
//!
//! ## Why the assertions read frontmatter rather than the body
//!
//! `{{ ctx.area }}` in the body renders the empty string for `""` *and* for
//! `null`, so a body assertion cannot see the difference — and AC21's contract
//! is precisely that these are strings, never `null` and never the retired
//! `"root"` sentinel. A whole-value `"{{ expr }}"` frontmatter key keeps the
//! evaluated JSON type, so `--frontmatter` emits `area: ''` for an empty string
//! and would emit `area: null` for the defect. The body is not the subject
//! here; `compose_lazy_roots.rs` and the library suite cover rendering.

mod common;

use std::path::{Path, PathBuf};

use common::CliProcessFixture;

/// Whole-value interpolations, so each key keeps its evaluated JSON type.
const SCOPE_DOCUMENT: &str = "---\narea: \"{{ ctx.area }}\"\ncpa: \"{{ ctx.current_package_area }}\"\ncp: \"{{ ctx.current_package }}\"\n---\nbody\n";

/// Two complementary blocks, so a position that renders neither marker fails
/// loudly instead of looking like the false branch.
const WHEN_DOCUMENT: &str = "::block when=\"ctx.current_package_area\"\nIN-AREA\n::end-block\n\n::block when=\"!ctx.current_package_area\"\nNO-AREA\n::end-block\n";

/// A Cargo monorepo with area `zeta` (holding package `zeta-lib`) and
/// top-level package `aaa`, plus a sibling directory outside it.
///
/// `aaa` sorts before `zeta` so an area that wrongly claimed the repository
/// root would be shadowed by the top-level package rather than showing up.
///
/// ## Returns
///
/// `None` when `git` is unavailable to the *test* process, which is the only
/// way the repository cannot be created; the composed child needs no `git`.
fn monorepo(fixture: &CliProcessFixture) -> Option<PathBuf> {
    let root = fixture.workspace_path().join("monorepo");
    fixture.write_file(
        "monorepo/Cargo.toml",
        "[workspace]\nmembers = [\"aaa\", \"zeta/lib\"]\n",
    );
    for (relative, name) in [("aaa", "aaa"), ("zeta/lib", "zeta-lib")] {
        fixture.write_file(
            format!("monorepo/{relative}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        );
        fixture.write_file(format!("monorepo/{relative}/src/lib.rs"), "");
    }
    // Outside the repository, but still inside the fixture workspace, so the
    // fifth position is a launch directory `ambient_context` will accept.
    fixture.write_file("elsewhere/keep", "");

    fixture.initialize_repository_at(&root).then_some(root)
}

/// Compose `document` from `launch_dir` and return the composed stdout.
fn compose_from(
    fixture: &CliProcessFixture,
    launch_dir: &Path,
    document: &str,
    content: &str,
) -> String {
    let path = launch_dir.join(document);
    std::fs::write(&path, content).unwrap();

    let mut command = fixture
        .command_builder()
        // The subject is what composition discovers from the launch directory,
        // which is the whole of the ambient leg.
        .ambient_context(launch_dir)
        .build();
    command.arg("compose");
    if document == "scope.md" {
        command.arg("--frontmatter");
    }
    let output = command.arg(document).assert().success();
    String::from_utf8(output.get_output().stdout.clone()).expect("md emits UTF-8")
}

/// The five tabled positions, as `(label, directory relative to the monorepo
/// root or the workspace, expected area / package-area / package)`.
fn positions(root: &Path, workspace: &Path) -> Vec<(&'static str, PathBuf, [&'static str; 3])> {
    vec![
        (
            "inside a package under an area",
            root.join("zeta/lib/src"),
            ["zeta-lib", "zeta", "zeta-lib"],
        ),
        (
            "inside a top-level package",
            root.join("aaa/src"),
            ["aaa", "", "aaa"],
        ),
        (
            "in an area directory but not a package",
            root.join("zeta"),
            ["zeta", "zeta", ""],
        ),
        ("at the monorepo root", root.to_path_buf(), ["", "", ""]),
        (
            "outside a monorepo",
            workspace.join("elsewhere"),
            ["", "", ""],
        ),
    ]
}

/// AC21: every tabled position projects the tabled value through a real
/// `md compose`, as a string — never `null`, never the retired `"root"`.
#[test]
fn every_scope_position_projects_its_tabled_string_through_md_compose() {
    let fixture = CliProcessFixture::named("scope_matrix_values");
    let Some(root) = monorepo(&fixture) else {
        panic!("the fixture repository needs `git` on the test process's PATH");
    };
    let workspace = fixture.workspace_path().to_path_buf();

    for (label, dir, [area, cpa, cp]) in positions(&root, &workspace) {
        let stdout = compose_from(&fixture, &dir, "scope.md", SCOPE_DOCUMENT);
        let yaml = |key: &str, value: &str| {
            // An empty string round-trips through YAML as `''`; `null` — the
            // defect AC21 names — would render as `null` and not match.
            if value.is_empty() {
                format!("{key}: ''")
            } else {
                format!("{key}: {value}")
            }
        };
        // Exact-line equality is what carries the typing clause: `area: null`
        // and `area: root` — the two defects AC21 names — are different lines
        // from `area: ''`, so neither can satisfy this.
        for line in [yaml("area", area), yaml("cpa", cpa), yaml("cp", cp)] {
            assert!(
                stdout.lines().any(|emitted| emitted == line),
                "{label}: expected `{line}`; composed frontmatter was:\n{stdout}"
            );
        }
    }
}

/// AC23: `when="ctx.current_package_area"` is true inside an area and false
/// both at the monorepo root and inside a top-level package — the two
/// positions whose package area is the empty string.
#[test]
fn the_package_area_condition_is_false_exactly_where_the_area_is_empty() {
    let fixture = CliProcessFixture::named("scope_matrix_condition");
    let Some(root) = monorepo(&fixture) else {
        panic!("the fixture repository needs `git` on the test process's PATH");
    };
    let workspace = fixture.workspace_path().to_path_buf();

    for (label, dir, [_, cpa, _]) in positions(&root, &workspace) {
        let stdout = compose_from(&fixture, &dir, "when.md", WHEN_DOCUMENT);
        let (kept, dropped) = if cpa.is_empty() {
            ("NO-AREA", "IN-AREA")
        } else {
            ("IN-AREA", "NO-AREA")
        };
        assert!(
            stdout.contains(kept) && !stdout.contains(dropped),
            "{label}: `when=\"ctx.current_package_area\"` must select `{kept}`; \
             composed body was:\n{stdout}"
        );
    }
}
