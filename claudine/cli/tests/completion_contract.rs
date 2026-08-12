//! Contract regression sweep for shell completions and autocomplete.
//!
//! Phase 5 of the `2026-06-14-auto-complete` feature. These tests lock in
//! the cross-cutting acceptance criteria that span the TAB and ENTER paths:
//!
//! - `claudine __complete` is the dynamic completion surface.
//! - `claudine completions <shell>` remains the bootstrap install command.
//! - `@`-magic candidates insert concrete paths without the `@` sigil.
//! - Bare `file`/`file[]` schema properties fall back to the default glob.
//! - Comma-continuation is a TAB-only behavior.
//! - YAML `sequence` candidates derive their detail block from top-level keys.
//!
//! Layout/type-driven chooser coverage lives in the Level-2/Level-3 harness
//! tests; this file covers the subprocess-visible contract.


mod common;
use common::TestWorkspace;
use common::completion::{
    run_complete, seed_cargo_workspace_members as seed_cargo_workspace, write_file,
};

// ----------------------------------------------------------------------
// Bootstrap contract
// ----------------------------------------------------------------------

#[test]
fn completions_subcommand_outputs_bootstrap_script() {
    // `claudine completions <shell>` must print a registration script that
    // shells out to the hidden `__complete` subcommand on every `<TAB>`.
    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["completions", "bash"])
        .output()
        .expect("completions subprocess to run");

    assert!(output.status.success(), "completions bash failed");
    let script = String::from_utf8_lossy(&output.stdout);
    assert!(
        script.contains("__complete"),
        "bash completion script must reference __complete; got: {script}"
    );
}

#[test]
fn completions_subcommand_supports_all_three_shells() {
    for shell in ["bash", "zsh", "fish"] {
        let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .args(["completions", shell])
            .output()
            .expect("completions subprocess to run");

        assert!(output.status.success(), "completions {shell} failed");
        let script = String::from_utf8_lossy(&output.stdout);
        assert!(
            script.contains("__complete"),
            "{shell} completion script must reference __complete; got: {script}"
        );
    }
}

// ----------------------------------------------------------------------
// Dynamic completion surface
// ----------------------------------------------------------------------

#[test]
fn complete_subcommand_drives_dynamic_completion() {
    let ws = TestWorkspace::named("contract-complete-surface");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("prompts").join("plan.md"), "# Plan\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "__complete must still drive dynamic completion; got: {got:?}"
    );
}

// ----------------------------------------------------------------------
// @-sigil kept; filename-only render
// ----------------------------------------------------------------------

#[test]
fn magic_at_inserts_filename_with_sigil() {
    let ws = TestWorkspace::named("contract-magic-keeps-sigil");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("prompts").join("review.md"), "# Review\n");

    let got = run_complete(ws.path(), &["compose", "@rev"]);
    assert!(
        got.iter().any(|c| c == "@review.md"),
        "@ magic must keep the sigil and render the filename; got: {got:?}"
    );
    assert!(
        got.iter().all(|c| c.starts_with('@') && !c.contains('/')),
        "every magic candidate must be `@<basename>`; got: {got:?}"
    );
}

// ----------------------------------------------------------------------
// Bare file / file[] default-glob fallback
// ----------------------------------------------------------------------

#[test]
fn bare_file_property_resolves_to_default_glob() {
    let ws = TestWorkspace::named("contract-bare-file-default-glob");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  cover: file\n",
            "---\n",
            "Cover at {{cover}}.\n",
        ),
    );
    write_file(&ws.path().join("readme.md"), "# Readme\n");
    write_file(&ws.path().join("draft.txt"), "plain text\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "cover="]);
    assert!(
        got.iter().any(|c| c == "cover='readme.md'"),
        "bare file property must fall back to default glob; got: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("draft.txt")),
        "non-markdown files must not surface in default glob; got: {got:?}"
    );
}

#[test]
fn bare_file_array_property_resolves_to_default_glob() {
    let ws = TestWorkspace::named("contract-bare-file-array-default-glob");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("notes.md"), "# Notes\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments="]);
    assert!(
        got.iter().any(|c| c == "attachments='notes.md'"),
        "bare file[] property must fall back to default glob; got: {got:?}"
    );
}

// ----------------------------------------------------------------------
// Comma-continuation is TAB-only
// ----------------------------------------------------------------------

#[test]
fn file_array_trailing_comma_reopens_on_tab_path() {
    // TAB path: a trailing comma after one selected file re-opens the
    // completion for the next file and excludes the already-named one.
    let ws = TestWorkspace::named("contract-comma-continuation-tab");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("a.md"), "# A\n");
    write_file(&ws.path().join("b.md"), "# B\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments=a.md,"]);
    assert!(
        got.iter().any(|c| c == "attachments='a.md,b.md'"),
        "trailing comma must append the next default-glob file; got: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "attachments='a.md,a.md'"),
        "already-selected file must be excluded on continuation; got: {got:?}"
    );
}

#[test]
fn file_array_first_value_excludes_nothing_on_tab_path() {
    // Before a comma appears, the first-file candidate set is not filtered.
    let ws = TestWorkspace::named("contract-comma-first-file-unfiltered");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("a.md"), "# A\n");
    write_file(&ws.path().join("b.md"), "# B\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments="]);
    assert!(
        got.iter().any(|c| c == "attachments='a.md'"),
        "first-file completion must include a.md; got: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "attachments='b.md'"),
        "first-file completion must include b.md; got: {got:?}"
    );
}

// ----------------------------------------------------------------------
// YAML sequence detail contract (library unit + integration identity)
// ----------------------------------------------------------------------

#[test]
fn yaml_sequence_candidates_surface_in_sequence_completion() {
    // TAB path: sequence mode accepts raw YAML files with a top-level
    // `sequence` key. This test verifies the candidate surface; the detail
    // block populated from top-level keys is covered by the library unit
    // tests for `extract_yaml_sequence_detail` and by the Level-2 chooser
    // harness tests for the ENTER path.
    let ws = TestWorkspace::named("contract-yaml-sequence-surface");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("steps.yaml"),
        "name: 'Deploy Pipeline'\ndescription: 'Deployment steps'\nsequence:\n  - one\n",
    );
    write_file(&ws.path().join("prompts").join("other.yaml"), "other:\n  - x\n");

    let got = run_complete(ws.path(), &["sequence", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/steps.yaml"),
        "sequence must surface YAML files with top-level sequence key; got: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("other.yaml")),
        "sequence must reject YAML without top-level sequence key; got: {got:?}"
    );
}

#[test]
fn yaml_sequence_detail_extracts_top_level_keys() {
    // Direct library assertion: the detail block that the ENTER-path chooser
    // renders for a YAML sequence file comes from top-level `name`,
    // `description`, and `$schema` keys, exactly like Markdown frontmatter.
    let ws = TestWorkspace::named("contract-yaml-detail-extraction");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    let path = ws.path().join("prompts").join("steps.yaml");
    write_file(
        &path,
        "name: 'Release'\ndescription: 'Release steps'\n$schema:\n  env: 'enum(dev, prod)'\nsequence:\n  - one\n",
    );

    let detail = claudine::composition::extract_yaml_sequence_detail(&path, "SEQUENCE");
    assert_eq!(detail.name, "Release");
    assert_eq!(detail.description.as_deref(), Some("Release steps"));
    assert!(
        detail.schema_lines.iter().any(|l| l.contains("env")),
        "YAML sequence detail must include $schema lines; got: {detail:?}"
    );
    assert_eq!(detail.badge, "SEQUENCE");
}
