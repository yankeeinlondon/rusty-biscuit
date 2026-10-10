//! The shipped implement router, read live, routing a case through its
//! proxy targets. Opt-in through `prompt-tests`; CI never runs these.

use crate::common;

use common::prompt_staging::{stage_shipped_prompts, workspace_root};
use common::provenance::{install_goose, run_compose};
use common::{CliProcessFixture, strip_ansi, write};

#[test]
fn shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan() {
    let fixture = CliProcessFixture::named("implement-router-unimplemented-review");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let package = fixture.cwd().join("packages/example");
    let case = package.join("fixes/case");
    write(
        &case.join("spec.md"),
        "---\nstatus: draft\n---\nSpecification.\n",
    );
    write(
        &case.join("plan.md"),
        "---\ntotal_phases: 5\nphase: 5\n---\n# Plan\n",
    );
    write(
        &case.join("review-1.md"),
        "---\nready: false\nimplemented: false\n---\n# Review 1\n\nA finding to implement.\n",
    );

    let router = fixture.cwd().join("prompts/implement.md");
    let repository = workspace_root();
    stage_shipped_prompts(
        &repository,
        &fixture.cwd().join("prompts"),
        &[
            "prompts/implement.md",
            "prompts/_implement/implement-suggestions.md",
            "prompts/_implement/implement-plan.md",
        ],
    );
    write(
        &fixture.cwd().join("prompts/_implement/implement-plan.md"),
        include_str!("../fixtures/shipped_implement_route/_implement/implement-plan.md"),
    );

    let stderr = run_compose(&fixture, &package, &router, &["spec=fixes/case/spec.md"]);

    assert!(
        stderr.contains("Implement Review Suggestions"),
        "an existing unimplemented review must outrank the already-executed plan; stderr:\n{stderr}"
    );
    // The stderr panel previews only the first 20 rendered rows, and the
    // route's leading rule block fills most of them, so the review path is
    // asserted on the prompt the provider received rather than the preview.
    let provider_prompt =
        std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    assert!(
        provider_prompt.contains("review-1.md"),
        "the routed prompt must name the unimplemented review; prompt:\n{provider_prompt}"
    );
    assert!(
        !stderr.contains("Implement Phase 5 of 5"),
        "the router must not resume the original plan once a review exists; stderr:\n{stderr}"
    );
}

/// An archived case — an implemented spec whose reviews are all implemented —
/// matches no route. The router must reach its own routing error rather than
/// raising on the `review` guard, which is an optional input nobody supplied.
#[test]
fn shipped_implement_router_refuses_an_archived_case_through_its_own_error() {
    let fixture = CliProcessFixture::named("implement-router-archived-case");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let package = fixture.cwd().join("packages/example");
    let case = package.join("fixes/case");
    write(
        &case.join("spec.md"),
        "---\nimplemented: true\nreview_iterations: 4\n---\nCase.\n",
    );
    write(
        &case.join("review-4.md"),
        "---\nimplemented: true\n---\n# Review 4\n",
    );

    let router = fixture.cwd().join("prompts/implement.md");
    write(&router, include_str!("../../../../prompts/implement.md"));

    // Not `run_compose_failure`: an authored `error:` action is a routing
    // decision, not a typed diagnostic facet, so it writes no snapshot.
    // Escape: caller-relative references must resolve from this fixture directory.
    let mut command = fixture.command_builder().ambient_context(&package).build();
    let audio_spool = fixture.cwd().join("provenance-audio-spool");
    let assertion = command
        .env("PLAYA_DRY_RUN", "1")
        .env("PLAYA_SPOOL_DIR", &audio_spool)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["compose", "--goose", router.to_str().unwrap()])
        .args(["spec=fixes/case/spec.md"])
        .assert()
        .failure();
    assert!(
        !audio_spool.exists(),
        "provenance tests must not publish audio"
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));

    assert!(
        stderr.contains("Unable to route the implementation to an appropriate prompt"),
        "an archived case must fail through the router's own error; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("references undefined variable"),
        "an unsupplied optional input must not crash a routing guard; stderr:\n{stderr}"
    );
    assert!(
        !fixture.home().join("provider-prompt").exists(),
        "an unroutable case must not launch a provider"
    );
}
