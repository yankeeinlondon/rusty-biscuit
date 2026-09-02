//! Level 1 coverage for immutable caller file origins across a proxy handoff.

mod common;

use common::{CliProcessFixture, strip_ansi, write, write_executable};

fn install_goose(fixture: &CliProcessFixture) {
    #[cfg(unix)]
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$HOME/provider-prompt\"\nprintf 'provider reached\\n'\n",
    );
    #[cfg(windows)]
    write_executable(
        &fixture.bin_dir().join("goose.cmd"),
        "@echo off\r\necho %* > \"%USERPROFILE%\\provider-prompt\"\r\necho provider reached\r\nexit /b 0\r\n",
    );
}

#[cfg(unix)]
fn install_retrying_goose(fixture: &CliProcessFixture) {
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nif [ ! -f \"$HOME/provider-attempted\" ]; then\n  : > \"$HOME/provider-attempted\"\n  exit 1\nfi\nprintf 'provider reached\\n'\n",
    );
}

#[cfg(unix)]
fn install_resumable_claude(fixture: &CliProcessFixture) {
    write_executable(
        &fixture.bin_dir().join("claude"),
        r#"#!/bin/sh
prompt=$(cat)
printf 'provider-ran\n' >> "$HOME/provider-events"
case " $* " in
  *" -r caller-origin-session "*)
    printf 'resume-session-ok\n' >> "$HOME/provider-events"
    printf '%s\n' '{"type":"init","session_id":"caller-origin-session","model":"claude-test"}'
    printf '%s\n' '{"type":"assistant","content":[{"type":"text","text":"resumed"}]}'
    exit 0
    ;;
  *)
    printf '%s\n' "$prompt" >> "$HOME/provider-prompts"
    printf '%s\n' '{"type":"init","session_id":"caller-origin-session","model":"claude-test"}'
    exit 99
    ;;
esac
"#,
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

fn run_compose_failure(
    fixture: &CliProcessFixture,
    cwd: &std::path::Path,
    document: &std::path::Path,
    setters: &[&str],
) -> (String, serde_json::Value) {
    let snapshot = fixture
        .cwd()
        .join(format!("diagnostic-{}.json", document.file_stem().unwrap().to_string_lossy()));
    let mut command = fixture.command();
    command
        .current_dir(cwd)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .env("CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT", &snapshot)
        .args(["compose", "--goose", document.to_str().unwrap()]);
    command.args(setters);
    let assertion = command.assert().failure();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let diagnostic = serde_json::from_str(
        &std::fs::read_to_string(&snapshot)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}\nstderr:\n{stderr}", snapshot.display())),
    )
    .unwrap_or_else(|error| panic!("invalid diagnostic snapshot: {error}\nstderr:\n{stderr}"));
    (stderr, diagnostic)
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
    // The suggestions target implements an existing review; without it the
    // derived `design` probe anchors at the prompt directory and never fires.
    // It is already implemented so the router takes the implemented-spec
    // branch and the target derives `review` itself rather than receiving the
    // router's absolute `pending_review` through `proxy.with`.
    write(
        &package.join("fixes/case/review-4.md"),
        "---\nimplemented: true\n---\n# Review 4\n",
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
    let absent_design_prompt =
        std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    let portable_package = biscuit_file::to_portable_string(&std::fs::canonicalize(&package).unwrap());
    // Paths derived from the eager caller value project to the repository-
    // relative display form, whatever spelling the temp repository carries.
    let derived_package = "packages/example";
    for expected in [
        format!("**Specification:** @{portable_package}/fixes/case/spec.md"),
        format!("**Review:** @{derived_package}/fixes/case/review-4.md"),
        format!("**Log File:** {derived_package}/fixes/case/log.md"),
    ] {
        assert!(
            absent_design_prompt.contains(&expected),
            "the shipped target omitted {expected:?}; prompt:\n{absent_design_prompt}"
        );
    }
    assert!(
        !absent_design_prompt.contains("**Design:**"),
        "an absent optional design must not render; prompt:\n{absent_design_prompt}"
    );

    write(
        &fixture
            .cwd()
            .join(derived_package)
            .join("fixes/case/design.md"),
        "# Design\n",
    );
    let _ = run_compose(
        &fixture,
        &package,
        &router,
        &["spec=fixes/case/spec.md"],
    );
    let present_design_prompt =
        std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    assert!(
        present_design_prompt.contains(&format!(
            "**Design:** @{derived_package}/fixes/case/design.md"
        )),
        "the shipped target must derive a present design beside the spec; prompt:\n{present_design_prompt}"
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

#[cfg(unix)]
#[test]
fn a_proxied_resume_rematerializes_from_the_original_caller_record() {
    let fixture = CliProcessFixture::named("caller-file-proxy-resume");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_resumable_claude(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    let router = fixture.cwd().join("prompts/router.md");
    write(
        &router,
        "---\ninitialize:\n  stack:\n    - action: {proxy: './resume-target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("prompts/resume-target.md"),
        "---\n$schema:\n  spec: 'file(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'resume={{ marker }}']}\nfailure:\n  stack:\n    - action: {resume: 'continue'}\n---\nRESUME={{ marker }}\n",
    );

    let assertion = fixture
        .command()
        .current_dir(fixture.cwd())
        .args([
            "compose",
            "--claude",
            router.to_str().unwrap(),
            "spec=cases/original/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    let provider_events = std::fs::read_to_string(fixture.home().join("provider-events")).unwrap();

    assert_eq!(
        events.lines().filter(|line| *line == "resume=original").count(),
        2,
        "both the opening and resumed attempts must materialize the original caller file; events:\n{events}\nstderr:\n{stderr}"
    );
    assert!(provider_events.contains("resume-session-ok"), "{provider_events}");
}

#[test]
fn a_proxied_loop_reuses_the_same_materialized_caller_identity() {
    let fixture = CliProcessFixture::named("caller-file-proxy-loop");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("cases/original/spec.md"),
        "---\nmarker: original\n---\n",
    );
    let router = fixture.cwd().join("prompts/router.md");
    write(
        &router,
        "---\ninitialize:\n  stack:\n    - action: {proxy: './loop-target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("prompts/loop-target.md"),
        "---\n$schema:\n  spec: 'file(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nphase: 1\nloop:\n  until: 'phase > 2'\n  action: 'increment(phase)'\n  max: 5\nfinalize:\n  stack:\n    - action: {append_line: ['events.log', 'loop={{ marker }}:{{ phase }}']}\n---\nLOOP={{ marker }}\n",
    );

    let stderr = run_compose(
        &fixture,
        fixture.cwd(),
        &router,
        &["spec=cases/original/spec.md"],
    );
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert_eq!(
        events.lines().filter(|line| line.starts_with("loop=original:")).count(),
        3,
        "every reused loop plan must retain the caller file identity; events:\n{events}\nstderr:\n{stderr}"
    );
}

#[test]
fn a_sequence_prompt_task_proxy_reads_the_invocation_wide_caller_file() {
    let fixture = CliProcessFixture::named("caller-file-sequence-proxy");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let package = fixture.cwd().join("packages/example");
    write(
        &package.join("cases/original/spec.md"),
        "---\nmarker: caller\n---\n",
    );
    let sequence = package.join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: routed\n    prompt: task-router.md\n---\nSequence.\n",
    );
    write(
        &package.join("task-router.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './task-target.md'}\n---\nRouter.\n",
    );
    write(
        &package.join("task-target.md"),
        "---\n$schema:\n  spec: 'file(required)'\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'sequence={{ marker }}']}\n---\nTARGET={{ marker }}\n",
    );

    let assertion = fixture
        .command()
        .current_dir(&package)
        .args([
            "sequence",
            "--goose",
            sequence.to_str().unwrap(),
            "spec=cases/original/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert!(events.contains("sequence=caller"), "events:\n{events}\nstderr:\n{stderr}");
}

#[test]
fn sequence_task_params_and_cli_file_inputs_keep_distinct_authoring_origins() {
    let fixture = CliProcessFixture::named("caller-file-sequence-layer-origins");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let package = fixture.cwd().join("packages/example");
    write(
        &package.join("caller/spec.md"),
        "---\nmarker: caller\n---\n",
    );
    write(
        &package.join("task/spec.md"),
        "---\nmarker: task\n---\n",
    );
    let sequence = package.join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: mixed\n    prompt: prompts/router.md\n    params:\n      task_spec: task/spec.md\n---\nSequence.\n",
    );
    write(
        &package.join("prompts/router.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
    );
    write(
        &package.join("prompts/target.md"),
        "---\n$schema:\n  caller_spec: 'file(required)'\n  task_spec: 'file(required)'\ncaller_marker: \"{{ frontmatter(caller_spec, 'marker') }}\"\ntask_marker: \"{{ frontmatter(task_spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'mixed={{ caller_marker }}:{{ task_marker }}']}\n---\nMIXED.\n",
    );

    let assertion = fixture
        .command()
        .current_dir(&package)
        .args([
            "sequence",
            "--goose",
            sequence.to_str().unwrap(),
            "caller_spec=caller/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert!(events.contains("mixed=caller:task"), "events:\n{events}\nstderr:\n{stderr}");
}

#[test]
fn a_sequence_cli_setter_shadows_the_same_named_task_file_param() {
    let fixture = CliProcessFixture::named("caller-file-sequence-cli-winner");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("task/spec.md"),
        "---\nmarker: task\n---\n",
    );
    write(
        &fixture.cwd().join("cli/spec.md"),
        "---\nmarker: cli\n---\n",
    );
    let sequence = fixture.cwd().join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: consume\n    prompt: router.md\n    params:\n      spec: task/spec.md\n---\nSequence.\n",
    );
    write(
        &fixture.cwd().join("router.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("target.md"),
        "---\n$schema:\n  spec: file(required)\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'winner={{ marker }}']}\n---\nTarget.\n",
    );

    let assertion = fixture
        .command()
        .current_dir(fixture.cwd())
        .args([
            "sequence",
            "--goose",
            sequence.to_str().unwrap(),
            "spec=cli/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert!(events.contains("winner=cli"), "events:\n{events}\nstderr:\n{stderr}");
}

#[test]
fn a_sequence_runtime_mutation_shadows_the_same_named_task_file_param() {
    let fixture = CliProcessFixture::named("caller-file-sequence-runtime-winner");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("task/spec.md"),
        "---\nmarker: task\n---\n",
    );
    write(
        &fixture.cwd().join("runtime/spec.md"),
        "---\nmarker: runtime\n---\n",
    );
    let sequence = fixture.cwd().join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: mutate\n    prompt: mutator.md\n  - name: consume\n    prompt: router.md\n    params:\n      spec: task/spec.md\n---\nSequence.\n",
    );
    write(
        &fixture.cwd().join("mutator.md"),
        "---\nsuccess:\n  stack:\n    - action: {set: [spec, runtime/spec.md]}\n---\nMutator.\n",
    );
    write(
        &fixture.cwd().join("router.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("target.md"),
        "---\n$schema:\n  spec: file(required)\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'winner={{ marker }}']}\n---\nTarget.\n",
    );

    let assertion = fixture
        .command()
        .current_dir(fixture.cwd())
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert!(
        events.contains("winner=runtime"),
        "events:\n{events}\nstderr:\n{stderr}"
    );
}

#[test]
fn a_sequence_reserved_overlay_shadows_a_same_named_caller_file_input() {
    let fixture = CliProcessFixture::named("caller-file-sequence-overlay-winner");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    write(
        &fixture.cwd().join("task/spec.md"),
        "---\nmarker: task\n---\n",
    );
    let sequence = fixture.cwd().join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: consume\n    prompt: router.md\n---\nSequence.\n",
    );
    write(
        &fixture.cwd().join("router.md"),
        "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
    );
    write(
        &fixture.cwd().join("target.md"),
        "---\n$schema:\n  state:\n    - file(required)\n    - object(required)\nreserved_name: \"{{ state.name }}\"\nstart:\n  stack:\n    - action: {append_line: ['events.log', 'winner={{ reserved_name }}']}\n---\nTarget.\n",
    );

    let assertion = fixture
        .command()
        .current_dir(fixture.cwd())
        .args([
            "sequence",
            "--goose",
            sequence.to_str().unwrap(),
            "state=task/spec.md",
        ])
        .assert()
        .success();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap();
    assert!(
        events.contains("winner=consume"),
        "events:\n{events}\nstderr:\n{stderr}"
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

#[test]
fn direct_and_proxy_file_failures_keep_equivalent_caller_diagnostics() {
    for (case, schema, expression, raw) in [
        ("malformed", "file(eager; required)", "", "@//invalid"),
        ("eager-missing", "file(eager; required)", "", "cases/missing/spec.md"),
        (
            "lazy-read-missing",
            "file(required)",
            "value: \"{{ frontmatter(spec, 'value') }}\"\n",
            "cases/missing/spec.md",
        ),
    ] {
        let fixture = CliProcessFixture::named(&format!("caller-file-{case}"));
        fixture.initialize_repository();
        fixture.seed_user_config();
        install_goose(&fixture);

        let target = fixture.cwd().join("prompts/target.md");
        write(
            &target,
            &format!(
                "---\n$schema:\n  spec: '{schema}'\n{expression}---\nTarget.\n"
            ),
        );
        let router = fixture.cwd().join("prompts/router.md");
        write(
            &router,
            "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
        );
        let setter = format!("spec={raw}");

        let (direct, direct_diagnostic) =
            run_compose_failure(&fixture, fixture.cwd(), &target, &[&setter]);
        let (proxied, proxied_diagnostic) =
            run_compose_failure(&fixture, fixture.cwd(), &router, &[&setter]);
        let diagnostic_headline = if case == "lazy-read-missing" {
            "invalid file path"
        } else {
            "schema validation"
        };

        for (route, output) in [("direct", &direct), ("proxy", &proxied)] {
            assert!(
                output.contains(diagnostic_headline),
                "{case} {route} lost the expected typed diagnostic: {output}"
            );
            assert!(
                output.contains(raw),
                "{case} {route} diagnostic lost the raw caller spelling; stderr:\n{output}"
            );
            assert!(
                output.contains("target.md"),
                "{case} {route} diagnostic must retain the target error locus: {output}"
            );
        }
        assert_eq!(
            direct_diagnostic["code"], proxied_diagnostic["code"],
            "{case} changed diagnostic code across proxy: direct={direct_diagnostic} proxy={proxied_diagnostic}"
        );
        assert_eq!(
            direct_diagnostic["code"],
            serde_json::json!("composition.invalid_file_reference"),
            "{case} lost its typed file-reference identity: {direct_diagnostic}"
        );
        for (route, diagnostic) in [
            ("direct", &direct_diagnostic),
            ("proxy", &proxied_diagnostic),
        ] {
            assert_eq!(diagnostic["detail"]["reference"], raw, "{case} {route}");
            assert_eq!(diagnostic["detail"]["property"], "spec", "{case} {route}");
            assert!(
                diagnostic["detail"]["base_dir"].as_str().is_some(),
                "{case} {route} lost the caller base: {diagnostic}"
            );
        }
        assert_eq!(
            direct_diagnostic["detail"]["base_dir"],
            proxied_diagnostic["detail"]["base_dir"],
            "{case} changed caller base across proxy"
        );
        assert_eq!(
            direct_diagnostic["detail"]["repository_root"],
            proxied_diagnostic["detail"]["repository_root"],
            "{case} changed caller origin across proxy"
        );
        assert_eq!(
            direct_diagnostic["detail"]["candidates"],
            proxied_diagnostic["detail"]["candidates"],
            "{case} changed selected candidate evidence across proxy"
        );
        if case == "malformed" {
            assert!(direct_diagnostic["detail"]["candidates"].is_null());
        } else {
            let expected_candidate = std::path::Path::new(
                direct_diagnostic["detail"]["base_dir"].as_str().unwrap(),
            )
            .join(raw);
            assert_eq!(
                direct_diagnostic["detail"]["candidates"][0]["path"],
                biscuit_file::to_portable_string(&expected_candidate),
                "{case} retained the wrong selected candidate: {direct_diagnostic}"
            );
        }
        if case == "lazy-read-missing" {
            let projected = fixture.cwd().join(raw).to_string_lossy().into_owned();
            for output in [&direct, &proxied] {
                assert!(
                    !output.contains(&format!("invalid file path: {projected}")),
                    "lazy diagnostic exposed only the projected candidate instead of caller evidence: {output}"
                );
            }
        }
    }
}

#[test]
fn dynamic_array_selection_keeps_complete_direct_and_proxy_diagnostics() {
    let fixture = CliProcessFixture::named("caller-file-dynamic-array-diagnostic");
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);

    let target = fixture.cwd().join("prompts/target.md");
    let router = fixture.cwd().join("prompts/router.md");
    write(
        &router,
        "---\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter.\n",
    );
    let files = r#"files=["missing.md","./missing.md"]"#;
    let caller_root = std::fs::canonicalize(fixture.cwd()).unwrap();
    let mut selected_candidates = Vec::new();

    for (index, raw) in [(0, "missing.md"), (1, "./missing.md")] {
        write(
            &target,
            &format!(
                "---\n$schema:\n  files: 'file(required)[]'\nindex: {index}\nvalue: \"{{{{ frontmatter(files[index], 'value') }}}}\"\n---\nTarget.\n"
            ),
        );

        let (direct, direct_diagnostic) =
            run_compose_failure(&fixture, fixture.cwd(), &target, &[files]);
        let (proxied, proxied_diagnostic) =
            run_compose_failure(&fixture, fixture.cwd(), &router, &[files]);

        for (route, stderr, diagnostic) in [
            ("direct", &direct, &direct_diagnostic),
            ("proxy", &proxied, &proxied_diagnostic),
        ] {
            assert!(stderr.contains("invalid file path"), "{index} {route}: {stderr}");
            assert!(stderr.contains(raw), "{index} {route}: {stderr}");
            assert_eq!(diagnostic["code"], "composition.invalid_file_reference");
            assert_eq!(diagnostic["detail"]["reference"], raw, "{index} {route}");
            assert_eq!(diagnostic["detail"]["property"], "files", "{index} {route}");
            assert_eq!(
                diagnostic["detail"]["base_dir"],
                biscuit_file::to_portable_string(&caller_root),
                "{index} {route} lost the caller base"
            );
            assert_eq!(
                diagnostic["detail"]["repository_root"],
                biscuit_file::to_portable_string(&caller_root),
                "{index} {route} lost the caller repository root"
            );
            assert_eq!(
                diagnostic["detail"]["candidates"][0]["path"],
                biscuit_file::to_portable_string(&caller_root.join(raw)),
                "{index} {route} lost the selected candidate"
            );
        }
        assert_eq!(
            direct_diagnostic, proxied_diagnostic,
            "dynamic array diagnostic changed across proxy for index {index}"
        );
        selected_candidates.push(std::path::PathBuf::from(
            direct_diagnostic["detail"]["candidates"][0]["path"]
                .as_str()
                .unwrap(),
        ));
    }

    assert_eq!(
        selected_candidates[0], selected_candidates[1],
        "aliased raw spellings must retain distinct references for one semantic candidate"
    );
}
