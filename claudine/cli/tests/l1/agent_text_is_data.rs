//! Red reproductions for `2026-09-27-agent-text-is-data`: one test per row of
//! the spec's "Where agent text re-enters" table.
//!
//! Each assertion states the post-fix contract — agent text arrives as exact
//! raw text, is never evaluated, never asks for shell approval, and the run
//! succeeds. A test stays ignored until the plan phase that fixes its row
//! removes the marker.

use std::fs;

use crate::common;
use common::{CliProcessFixture, sh_quote, strip_ansi, write_executable};

/// Install a fake `goose` that appends each call's argv and stdin to
/// `$HOME/prompts.txt`, runs `first_call` and prints `first_output` on its
/// first call, and prints `done` on every later call.
fn counting_goose(fixture: &CliProcessFixture, first_call: &str, first_output: &str) {
    let first_output = sh_quote(first_output);
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\n\
             n=0\n\
             if [ -f \"$HOME/n.txt\" ]; then IFS= read -r n < \"$HOME/n.txt\"; fi\n\
             n=$((n + 1))\n\
             printf '%s' \"$n\" > \"$HOME/n.txt\"\n\
             {{ printf '=== call %s ===\\n' \"$n\"; printf '%s\\n' \"$*\"; /bin/cat; }} >> \"$HOME/prompts.txt\"\n\
             if [ \"$n\" = 1 ]; then\n\
             {first_call}\n\
             printf '%s\\n' {first_output}\n\
             else\n\
             printf '%s\\n' done\n\
             fi\n\
             exit 0\n"
        ),
    );
}

/// Everything the fake agent received on calls after the first.
fn later_prompts(fixture: &CliProcessFixture) -> String {
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    log.split("=== call ")
        .filter(|call| !call.starts_with("1 ===") && !call.is_empty())
        .collect::<Vec<_>>()
        .join("=== call ")
}

/// Shell that writes `log.md` with an agent-authored `message_to_agent`.
fn write_agent_log(message: &str) -> String {
    let content = format!("---\nmessage_to_agent: {message}\n---\nLog.\n");
    format!("printf '%s' {} > log.md", sh_quote(&content))
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String) {
    let output = fixture.command().args(args).assert().get_output().clone();
    (
        output.status.code().unwrap_or(-1),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

fn assert_no_shell_approval(stderr: &str) {
    assert!(
        !stderr.contains("ShellExpansionError") && !stderr.contains("not pre-approved"),
        "agent text must never be offered as a shell command; stderr:\n{stderr}"
    );
}

// ============================================================================
// Row 1: captured agent output in a loop (`_loop_last_output`)
// ============================================================================

fn loop_carries_output_raw(agent_text: &str) {
    let fixture = CliProcessFixture::named("agent-text-loop");
    counting_goose(&fixture, "", agent_text);
    let sentinel = fixture.cwd().join("x");
    fs::create_dir(&sentinel).unwrap();
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: t\ncounter: 0\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\n---\nPrevious: [{{ _loop_last_output }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "iteration 2 must prepare; stderr:\n{stderr}");
    assert_no_shell_approval(&stderr);
    assert!(sentinel.is_dir(), "agent text must never run as a command");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains(&format!("Previous: [{agent_text}]")),
        "iteration 2 must render the prior output byte-for-byte; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

/// Spec row "captured agent output in a loop": malformed template syntax.
#[test]
#[ignore = "red until phase 4"]
fn loop_last_output_with_template_syntax_stays_raw() {
    loop_carries_output_raw("see {{…}} siblings");
}

/// Spec row "captured agent output in a loop": a whole-value shell form.
#[test]
#[ignore = "red until phase 4"]
fn loop_last_output_with_whole_value_shell_stays_raw() {
    loop_carries_output_raw("$(echo INJECTED)");
}

/// Spec row "captured agent output in a loop": the acceptance-test text.
#[test]
#[ignore = "red until phase 4"]
fn loop_last_output_with_template_and_shell_stays_raw() {
    loop_carries_output_raw("see {{…}} and $(rm -rf x)");
}

// ============================================================================
// Row 2: captured task output in a sequence (`outputs`, step overlay)
// ============================================================================

/// Spec row "captured task output in a sequence": an agent task's output reaches
/// the next step raw through a param and through `last(outputs)`.
#[test]
#[ignore = "red until phase 4"]
fn sequence_outputs_with_template_syntax_stay_raw() {
    let fixture = CliProcessFixture::named("agent-text-sequence");
    counting_goose(&fixture, "", "{{ ctx.repo }}");
    fs::write(fixture.cwd().join("producer.md"), "Produce.\n").unwrap();
    fs::write(
        fixture.cwd().join("reader.md"),
        "---\ntitle: reader\n---\nEntry: [{{ doc.entry }}]\nLast: [{{ last(outputs) }}]\n",
    )
    .unwrap();
    let md = fixture.cwd().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - name: alpha\n    prompt: producer.md\n  - name: beta\n    prompt: reader.md\n    params:\n      entry: \"{{ last(outputs) }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["sequence", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Entry: [{{ ctx.repo }}]") && prompts.contains("Last: [{{ ctx.repo }}]"),
        "the next step must see the agent's raw text; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

/// Spec row "captured task output in a sequence": a parallel group's nested
/// `outputs` entry stays raw.
#[test]
#[ignore = "red until phase 4"]
fn sequence_parallel_group_nested_output_stays_raw() {
    let fixture = CliProcessFixture::named("agent-text-sequence");
    counting_goose(&fixture, "", "unused");
    fs::write(fixture.cwd().join("emit.sh"), "printf '%s\\n' '{{ ctx.repo }}'\n").unwrap();
    fs::write(
        fixture.cwd().join("reader.md"),
        "---\ntitle: reader\n---\nEntry: [{{ doc.entry }}]\nNested: [{{ last(outputs)[0] }}]\n",
    )
    .unwrap();
    let md = fixture.cwd().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: templated
          shell: "/bin/sh emit.sh"
        - name: plain
          shell: "printf 'plain\n'"
  - name: beta
    prompt: reader.md
    params:
      entry: "{{ last(outputs)[0] }}"
---
Body.
"#,
    )
    .unwrap();

    let (code, stderr) = run(
        &fixture,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    assert!(
        log.contains("Entry: [{{ ctx.repo }}]") && log.contains("Nested: [{{ ctx.repo }}]"),
        "the nested entry must stay raw; prompts:\n{log}\nstderr:\n{stderr}"
    );
}

// ============================================================================
// Row 3: frontmatter keys an agent adds in an inline document
// ============================================================================

/// Spec row "frontmatter keys an agent adds or changes in an inline document":
/// the rerun reads the agent's value back as the original string.
#[test]
#[ignore = "red until phase 5"]
fn inline_agent_added_frontmatter_survives_a_second_run() {
    let fixture = CliProcessFixture::named("agent-text-inline");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nprompt: \"Summary so far: [{{ summary || 'none' }}]\"\n---\nold body\n",
    )
    .unwrap();

    common::InlineAgentStub::new(&md)
        .frontmatter_additions("summary: fixed {{…}} parsing\n")
        .body("first body\n")
        .install(fixture.bin_dir(), "goose");
    let (first_code, first_stderr) =
        run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);
    assert_eq!(first_code, 0, "first run; stderr:\n{first_stderr}");

    let prompt_log = fixture.home().join("prompt.txt");
    let capture = format!(
        "{{ printf '%s\\n' \"$*\"; /bin/cat; }} > {}\n",
        sh_quote(&prompt_log.display().to_string())
    );
    common::InlineAgentStub::new(&md)
        .prelude(&capture)
        .body("second body\n")
        .install(fixture.bin_dir(), "goose");
    let (second_code, second_stderr) =
        run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(
        second_code, 0,
        "the rerun must prepare the agent-written key as data; stderr:\n{second_stderr}"
    );
    let prompt = fs::read_to_string(&prompt_log).unwrap_or_default();
    assert!(
        prompt.contains("Summary so far: [fixed {{…}} parsing]"),
        "the rerun must read back the agent's exact text; prompt:\n{prompt}"
    );
}

// ============================================================================
// Row 5: agent-written files read by an authored expression
// ============================================================================

/// Spec row "agent-written files read by expression": a top-level lifecycle
/// field sends the agent's exact text instead of refusing it as a surviving span.
///
/// Green since Darkmatter stopped converting `{{{ … }}}` literals in inserted
/// text; a `{{ … }}` payload in the same field still depends on N10.
#[test]
fn lifecycle_field_from_agent_written_file_is_sent_verbatim() {
    let fixture = CliProcessFixture::named("agent-text-message");
    counting_goose(&fixture, &write_agent_log("'see {{{ title }}} siblings'"), "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: \"agent says: {{ frontmatter('log.md', 'message_to_agent') }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(
        stderr.contains("agent says: see {{{ title }}} siblings"),
        "the message must carry the agent's exact text; stderr:\n{stderr}"
    );
}

/// Spec row "agent-written files read by expression": a stack action operand
/// sends the agent's exact text instead of re-expanding it.
#[test]
#[ignore = "red until phase 4"]
fn lifecycle_stack_message_from_agent_written_file_is_sent_verbatim() {
    let fixture = CliProcessFixture::named("agent-text-message");
    counting_goose(&fixture, &write_agent_log("'see {{…}} siblings'"), "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  stack:\n    - action: {info: \"agent says: {{ frontmatter('log.md', 'message_to_agent') }}\"}\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(
        stderr.contains("agent says: see {{…}} siblings"),
        "the message must carry the agent's exact text; stderr:\n{stderr}"
    );
}

// ============================================================================
// Row 4: lifecycle `set:` and `proxy.with:` values derived from agent data
// ============================================================================

/// Spec row "lifecycle values derived from the above": a `set:` value read
/// from an agent-written file stays inert in the next loop iteration.
#[test]
#[ignore = "red until phase 4"]
fn lifecycle_set_from_agent_data_stays_inert_on_next_preparation() {
    let fixture = CliProcessFixture::named("agent-text-set");
    counting_goose(&fixture, &write_agent_log("'agent said {{ title }}'"), "ok");
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: authored-title\ncarried: none\ncounter: 0\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\nsuccess:\n  stack:\n    - action: {set: {carried: \"{{ frontmatter('log.md', 'message_to_agent') }}\"}}\n---\nCarried: [{{ carried }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Carried: [agent said {{ title }}]"),
        "the `set:` value must not be filled in again; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

/// Spec row "lifecycle values derived from the above": a `proxy.with:` value
/// read from an agent-written file stays inert when the target prepares.
#[test]
#[ignore = "red until phase 4"]
fn lifecycle_proxy_with_from_agent_data_stays_inert_in_target() {
    let fixture = CliProcessFixture::named("agent-text-proxy");
    counting_goose(&fixture, &write_agent_log("'agent said {{ title }}'"), "ok");
    fs::write(
        fixture.cwd().join("target.md"),
        "---\ntitle: target-title\nnote: none\n---\nNote: [{{ note }}]\n",
    )
    .unwrap();
    let md = fixture.cwd().join("source.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  stack:\n    - action: {action: proxy, target: './target.md', with: {note: \"{{ frontmatter('log.md', 'message_to_agent') }}\"}}\n---\nSource.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Note: [agent said {{ title }}]"),
        "the overlay value must not be filled in again; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

// ============================================================================
// Inventory I2, Table B: further re-entry points
// ============================================================================

/// Row B3: a lifecycle shell command built from file data runs the approved
/// bytes, not a late re-resolution of the data.
#[test]
#[ignore = "red until phase 4"]
fn lifecycle_shell_from_file_data_runs_approved_bytes() {
    let fixture = CliProcessFixture::named("agent-text-shell");
    counting_goose(&fixture, "", "ok");
    fs::write(
        fixture.cwd().join("data.md"),
        "---\nv: '{{ err.msg }}'\n---\nData.\n",
    )
    .unwrap();
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  stack:\n    - action: {shell: \"printf '%s' '{{ frontmatter(\\\"data.md\\\", \\\"v\\\") }}' > out.txt\"}\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(
        &fixture,
        &["compose", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let out = fs::read_to_string(fixture.cwd().join("out.txt")).unwrap_or_default();
    assert_eq!(
        out, "{{ err.msg }}",
        "the command must receive the data verbatim; stderr:\n{stderr}"
    );
}

/// Row B8: a loop control variable seeded from composed frontmatter is not
/// re-evaluated as an authored override. The body leaves it unrendered so the
/// seed, not the body rescan, is what fails.
#[test]
#[ignore = "red until phase 4"]
fn loop_seed_from_composed_frontmatter_stays_raw() {
    let fixture = CliProcessFixture::named("agent-text-seed");
    counting_goose(&fixture, "", "ok");
    fs::write(
        fixture.cwd().join("plan.md"),
        "---\ntotal_phases: 'see {{…}} siblings'\n---\nPlan.\n",
    )
    .unwrap();
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: authored-title\ncounter: 0\ntotal_phases: \"{{ frontmatter('plan.md', 'total_phases') }}\"\nloop:\n  while: \"counter < 1 && total_phases != ''\"\n  actions:\n    - 'increment(counter)'\n---\nIteration counter={{ counter }}\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Iteration counter=1"),
        "every iteration must prepare; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

/// Row B9: a mixed-render loop action result stays a string even when it
/// parses as JSON.
#[test]
#[ignore = "red until phase 4"]
fn loop_action_result_is_not_reparsed_as_json() {
    let fixture = CliProcessFixture::named("agent-text-action");
    counting_goose(&fixture, "", "true");
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: t\ncounter: 0\nflag: none\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\n    - op: set\n      prop: flag\n      value: \" {{ _loop_last_output }}\"\n---\nFlag: [{{ flag }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Flag: [ true"),
        "the leading space shows the value stayed a string; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}

/// Row B10: a JSONL item field reaches the step prompt as raw `state` data.
#[test]
#[ignore = "red until phase 4"]
fn sequence_state_from_jsonl_item_stays_raw() {
    let fixture = CliProcessFixture::named("agent-text-state");
    counting_goose(&fixture, "", "ok");
    fs::write(
        fixture.cwd().join("items.jsonl"),
        "{\"name\":\"a\",\"note\":\"see {{ title }}\"}\n",
    )
    .unwrap();
    let md = fixture.cwd().join("seq.md");
    fs::write(
        &md,
        "---\ntitle: authored-title\nsequence: items.jsonl\n---\nNote: [{{ state.note }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["sequence", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    assert!(
        log.contains("Note: [see {{ title }}]"),
        "the step must see the item's raw text; prompts:\n{log}\nstderr:\n{stderr}"
    );
}

/// Row B12: group `variables:` and task `params:` evaluated from a prior
/// task's output reach the member prompt raw.
#[test]
#[ignore = "red until phase 4"]
fn task_params_and_group_variables_from_outputs_stay_raw() {
    let fixture = CliProcessFixture::named("agent-text-params");
    counting_goose(&fixture, "", "ok");
    fs::write(fixture.cwd().join("emit.sh"), "printf '%s\\n' 'see {{ title }}'\n").unwrap();
    fs::write(
        fixture.cwd().join("reader.md"),
        "---\ntitle: reader-title\n---\nGroup: [{{ group.prev }}]\nParam: [{{ doc.entry }}]\n",
    )
    .unwrap();
    let md = fixture.cwd().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    shell: "/bin/sh emit.sh"
  - name: beta
    group:
      name: bundle
      variables:
        prev: "{{ last(outputs) }}"
      tasks:
        - name: read
          prompt: reader.md
          params:
            entry: "{{ last(outputs) }}"
---
Body.
"#,
    )
    .unwrap();

    let (code, stderr) = run(
        &fixture,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    assert!(
        log.contains("Group: [see {{ title }}]") && log.contains("Param: [see {{ title }}]"),
        "the member prompt must see the prior output raw; prompts:\n{log}\nstderr:\n{stderr}"
    );
}

/// Row B15: a positional side-effect argument read from an agent-written file
/// is written without being resolved a second time.
#[test]
#[ignore = "red until phase 4"]
fn set_frontmatter_argument_from_agent_data_is_not_reresolved() {
    let fixture = CliProcessFixture::named("agent-text-effect");
    counting_goose(&fixture, &write_agent_log("'agent said {{ title }}'"), "ok");
    fs::write(fixture.cwd().join("state.md"), "---\nnote: none\n---\nState.\n").unwrap();
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: authored-title\nsuccess:\n  stack:\n    - action: {set_frontmatter: [\"state.md\", \"note\", \"{{ frontmatter('log.md', 'message_to_agent') }}\"]}\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let state = fs::read_to_string(fixture.cwd().join("state.md")).unwrap_or_default();
    assert!(
        state.contains("agent said {{ title }}"),
        "the written value must be the agent's exact text; state.md:\n{state}\nstderr:\n{stderr}"
    );
}

/// Rows B14 and B15: data persisted into the active document's frontmatter is
/// read back as data when the next iteration prepares.
#[test]
#[ignore = "red until phase 5"]
fn set_frontmatter_persisted_agent_data_survives_next_preparation() {
    let fixture = CliProcessFixture::named("agent-text-persist");
    counting_goose(&fixture, &write_agent_log("'see {{…}} siblings'"), "ok");
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: t\nnote: none\ncounter: 0\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\nsuccess:\n  stack:\n    - action: {set_frontmatter: [\"loop.md\", \"note\", \"{{ frontmatter('log.md', 'message_to_agent') }}\"]}\n---\nNote: [{{ note }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "the next iteration must prepare; stderr:\n{stderr}");
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Note: [see {{…}} siblings]"),
        "the persisted value must render verbatim; prompts:\n{prompts}\nstderr:\n{stderr}"
    );
}
