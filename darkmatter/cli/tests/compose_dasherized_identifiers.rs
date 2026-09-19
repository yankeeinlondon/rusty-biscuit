//! End-to-end `md compose` coverage for Requirement 1 of the
//! dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): kebab-case
//! identifiers through the binary, `--set` overrides of a kebab key, and a
//! shipped prompt whose dashes live in string literals.

mod common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const KEBAB_DOCUMENT: &str = "---\nspec-name: alpha\nlabel: \"v={{ spec-name }}\"\n_loop_count: 3\n---\n\
Bare: [{{ spec-name }}] Doc: [{{ doc.spec-name }}] Bracket: [{{ doc['spec-name'] }}] \
Label: [{{ label }}] Prev: [{{ _loop_count - 1 }}|{{ 4-2 }}]\n";

#[test]
fn compose_resolves_a_kebab_key_three_ways_through_the_binary() {
    let fixture = CliProcessFixture::named("compose_resolves_a_kebab_key_three_ways_through_the_binary");
    fixture
        .command()
        .args(["compose", "-", "--frontmatter"])
        .write_stdin(KEBAB_DOCUMENT)
        .assert()
        .success()
        .stdout(predicate::str::contains("label: v=alpha"))
        .stdout(predicate::str::contains(
            "Bare: [alpha] Doc: [alpha] Bracket: [alpha] Label: [v=alpha] Prev: [2|2]",
        ))
        .stdout(predicate::str::contains("{{").not());
}

#[test]
fn compose_set_overrides_a_kebab_key_for_every_spelling() {
    let fixture = CliProcessFixture::named("compose_set_overrides_a_kebab_key_for_every_spelling");
    fixture
        .command()
        .args(["compose", "-", "--set", r#"{"spec-name":"override"}"#])
        .write_stdin(KEBAB_DOCUMENT)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Bare: [override] Doc: [override] Bracket: [override] Label: [v=override]",
        ));
}

/// Copies the shipped `review-findings-plan.md` prompt into a fixture
/// repository with a spec that records two review iterations.
fn shipped_review_plan_repository(process: &CliProcessFixture) -> PathBuf {
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("darkmatter CLI crate should live two levels below the repository root")
        .to_path_buf();
    let root = process.cwd().to_path_buf();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("prompts/_implement")).unwrap();
    for shipped in ["prompts/_implement/review-findings-plan.md", "prompts/_no_formatting.md"] {
        fs::copy(checkout.join(shipped), root.join(shipped)).unwrap();
    }
    fs::create_dir_all(root.join("feature")).unwrap();
    fs::write(root.join("feature/spec.md"), "---\nreview_iterations: 2\n---\n# Spec\n").unwrap();
    root
}

/// The shipped prompt builds paths with `'review-plan-' + iteration`; the dash
/// sits inside a string literal and must not join into an identifier.
#[test]
fn shipped_prompt_with_dashes_in_string_literals_composes_unchanged() {
    let process = CliProcessFixture::new();
    let root = shipped_review_plan_repository(&process);

    process
        .command_builder()
        .ambient_context(&root)
        .build()
        .args([
            "compose",
            "prompts/_implement/review-findings-plan.md",
            "spec=feature/spec.md",
            "--no-baseline-schema",
            "--no-trigger-schemas",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("review-plan-2.md"))
        .stdout(predicate::str::contains("review-2.md"))
        .stdout(predicate::str::contains("{{").not());
}
