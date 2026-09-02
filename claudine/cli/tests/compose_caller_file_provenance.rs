//! Level 1 coverage for immutable caller file origins across a proxy handoff.

mod common;

use common::{CliProcessFixture, strip_ansi, write, write_executable};

fn install_goose(fixture: &CliProcessFixture) {
    #[cfg(unix)]
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf 'provider reached\\n'\n",
    );
    #[cfg(windows)]
    write_executable(
        &fixture.bin_dir().join("goose.cmd"),
        "@echo off\r\necho provider reached\r\nexit /b 0\r\n",
    );
}

#[cfg(unix)]
fn install_retrying_goose(fixture: &CliProcessFixture) {
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nif [ ! -f \"$HOME/provider-attempted\" ]; then\n  : > \"$HOME/provider-attempted\"\n  exit 1\nfi\nprintf 'provider reached\\n'\n",
    );
}

fn run_compose(
    fixture: &CliProcessFixture,
    cwd: &std::path::Path,
    document: &std::path::Path,
    setters: &[&str],
) -> String {
    let mut command = fixture.command();
    command
        .current_dir(cwd)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["compose", "--goose", document.to_str().unwrap()]);
    command.args(setters);
    let assertion = command.assert().success();
    strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr))
}

#[test]
fn shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target() {
    let fixture = CliProcessFixture::named("caller-file-provenance");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let package = fixture.cwd().join("packages/example");
    let spec = package.join("fixes/case/spec.md");
    write(
        &spec,
        "---\nimplemented: true\nreview_iterations: 4\n---\nCase.\n",
    );

    let router = fixture.cwd().join("prompts/implement.md");
    write(
        &router,
        include_str!("../../../prompts/implement.md"),
    );
    write(
        &fixture
            .cwd()
            .join("prompts/_implement/implement-suggestions.md"),
        include_str!("../../../prompts/_implement/implement-suggestions.md"),
    );

    let stderr = run_compose(
        &fixture,
        &package,
        &router,
        &["spec=fixes/case/spec.md"],
    );

    assert!(
        stderr.contains("Iteration: 4"),
        "the literal shipped lazy target must read the caller's package-relative specification; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("fixes/case/log.md"),
        "the shipped target must derive its log beside the caller's spec; stderr:\n{stderr}"
    );
}

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
    write(&router, include_str!("../../../prompts/implement.md"));
    write(
        &fixture
            .cwd()
            .join("prompts/_implement/implement-suggestions.md"),
        include_str!("../../../prompts/_implement/implement-suggestions.md"),
    );
    write(
        &fixture
            .cwd()
            .join("prompts/_implement/implement-plan.md"),
        include_str!("fixtures/shipped_implement_route/_implement/implement-plan.md"),
    );

    let stderr = run_compose(
        &fixture,
        &package,
        &router,
        &["spec=fixes/case/spec.md"],
    );

    assert!(
        stderr.contains("Implement Review Suggestions"),
        "an existing unimplemented review must outrank the already-executed plan; stderr:\n{stderr}"
    );
    assert!(stderr.contains("review-1.md"), "stderr:\n{stderr}");
    assert!(
        !stderr.contains("Implement Phase 5 of 5"),
        "the router must not resume the original plan once a review exists; stderr:\n{stderr}"
    );
}

#[test]
fn direct_and_proxy_targets_agree_and_caller_values_outrank_proxy_with() {
    let fixture = CliProcessFixture::named("caller-file-direct-proxy-equivalence");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    write(
        &fixture.cwd().join("cases/decoy/spec.md"),
        "---\nmarker: decoy\n---\n",
    );
    let target = fixture.cwd().join("prompts/target.md");
    write(
        &target,
        "---\n$schema:\n  spec: 'file(required)'\n  choice: 'string(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\n---\nROUTE={{ marker }}/{{ choice }}\n",
    );
    let router = fixture.cwd().join("prompts/router.md");
    write(
        &router,
        "---\ninitialize:\n  stack:\n    - action: {action: proxy, target: './target.md', with: {spec: 'cases/decoy/spec.md', choice: overlay}}\n---\nRouter.\n",
    );
    let setters = ["spec=cases/original/spec.md", "choice=caller"];

    let direct = run_compose(&fixture, fixture.cwd(), &target, &setters);
    let proxied = run_compose(&fixture, fixture.cwd(), &router, &setters);

    for output in [&direct, &proxied] {
        assert!(
            output.contains("ROUTE=original/caller"),
            "caller values must produce the same prepared target and outrank proxy.with; stderr:\n{output}"
        );
        assert!(!output.contains("ROUTE=decoy/overlay"), "{output}");
    }
}

#[test]
fn a_second_proxy_hop_drops_the_first_overlay_but_keeps_caller_records() {
    let fixture = CliProcessFixture::named("caller-file-multi-hop");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    write(
        &fixture.cwd().join("cases/decoy/spec.md"),
        "---\nmarker: decoy\n---\n",
    );
    let first = fixture.cwd().join("prompts/first.md");
    write(
        &first,
        "---\ninitialize:\n  stack:\n    - action: {action: proxy, target: './middle.md', with: {spec: 'cases/decoy/spec.md', ephemeral: first-hop}}\n---\nFirst.\n",
    );
    write(
        &fixture.cwd().join("prompts/middle.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './final.md'}\n---\nMiddle.\n",
    );
    write(
        &fixture.cwd().join("prompts/final.md"),
        "---\n$schema:\n  spec: 'file(required)'\n  stable: 'string(required)'\n  ephemeral: 'string(default(dropped))'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\n---\nMULTIHOP={{ marker }}/{{ stable }}/{{ ephemeral }}\n",
    );

    let output = run_compose(
        &fixture,
        fixture.cwd(),
        &first,
        &["spec=cases/original/spec.md", "stable=caller"],
    );

    assert!(
        output.contains("MULTIHOP=original/caller/"),
        "the first overlay must be gone while immutable caller records reach the final target; stderr:\n{output}"
    );
    assert!(!output.contains("first-hop"), "{output}");
}

#[cfg(unix)]
#[test]
fn a_proxied_retry_rematerializes_from_the_original_caller_record() {
    let fixture = CliProcessFixture::named("caller-file-proxy-retry");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_retrying_goose(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    let router = fixture.cwd().join("prompts/router.md");
    write(
        &router,
        "---\ninitialize:\n  stack:\n    - action: {proxy: './retry-target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("prompts/retry-target.md"),
        "---\n$schema:\n  spec: 'file(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'retry={{ marker }}']}\nfailure:\n  stack:\n    - action: {retry: 1}\n---\nRETRY={{ marker }}\n",
    );

    let output = run_compose(
        &fixture,
        fixture.cwd(),
        &router,
        &["spec=cases/original/spec.md"],
    );
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();

    assert_eq!(
        events.lines().filter(|line| *line == "retry=original").count(),
        2,
        "the original attempt and fresh retry must both materialize the caller file; events:\n{events}\nstderr:\n{output}"
    );
}

#[test]
fn inline_compose_proxy_uses_the_caller_origin_and_closes_over_the_target() {
    let fixture = CliProcessFixture::named("caller-file-inline-proxy");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    let router = fixture.cwd().join("prompts/inline-router.md");
    write(
        &router,
        "---\nprompt: Router.\ninitialize:\n  stack:\n    - action: {proxy: './inline-target.md'}\n---\nRouter body.\n",
    );
    let target = fixture.cwd().join("prompts/inline-target.md");
    write(
        &target,
        "---\n$schema:\n  spec: 'file(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nprompt: 'INLINE={{ marker }}'\n---\nOld body.\n",
    );

    let assertion = fixture
        .command()
        .current_dir(fixture.cwd())
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args([
            "inline-compose",
            "--goose",
            router.to_str().unwrap(),
            "spec=cases/original/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let rewritten = std::fs::read_to_string(&target).unwrap();

    assert!(
        stderr.contains("INLINE=original"),
        "the inline target must compose its prompt from the caller's file origin; stderr:\n{stderr}"
    );
    assert!(
        rewritten.contains("provider reached"),
        "the inline closure must rewrite the adopted target, not the router; target:\n{rewritten}"
    );
    assert!(
        std::fs::read_to_string(&router).unwrap().contains("Router body."),
        "the router must remain unchanged after handing off"
    );
}
