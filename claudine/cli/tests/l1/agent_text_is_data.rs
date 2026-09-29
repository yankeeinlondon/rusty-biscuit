//! Red reproductions for `2026-09-27-agent-text-is-data`: one test per row of
//! the spec's "Where agent text re-enters" table.
//!
//! Each assertion states the post-fix contract — agent text arrives as exact
//! raw text, is never evaluated, never asks for shell approval, and the run
//! succeeds. A test stays ignored until the plan phase that fixes its row
//! removes the marker. The regression tests at the end pin what must not
//! change: a value a person typed is still a template, and the authoring
//! guards still refuse before any provider starts.

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
fn loop_last_output_with_template_syntax_stays_raw() {
    loop_carries_output_raw("see {{…}} siblings");
}

/// Spec row "captured agent output in a loop": a whole-value shell form.
#[test]
fn loop_last_output_with_whole_value_shell_stays_raw() {
    loop_carries_output_raw("$(echo INJECTED)");
}

/// Spec row "captured agent output in a loop": the acceptance-test text.
#[test]
fn loop_last_output_with_template_and_shell_stays_raw() {
    loop_carries_output_raw("see {{…}} and $(rm -rf x)");
}

/// Spec row "captured agent output in a loop": the loop predicate reads the
/// same raw text. Iteration 1's output ends the loop only when its length is
/// exactly that of the raw text; a rewritten value would run a second call.
#[test]
fn loop_predicate_reads_the_raw_output() {
    let agent_text = "see {{…}} and $(rm -rf x)";
    let fixture = CliProcessFixture::named("agent-text-loop-predicate");
    counting_goose(&fixture, "", agent_text);
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        format!(
            "---\ntitle: t\nloop:\n  max: 3\n  until: 'length(_loop_last_output) == {}'\n---\nGo.\n",
            agent_text.chars().count()
        ),
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_no_shell_approval(&stderr);
    assert_eq!(
        later_prompts(&fixture),
        "",
        "the predicate must have seen the raw text after one call; stderr:\n{stderr}"
    );
}

// ============================================================================
// Row 2: captured task output in a sequence (`outputs`, step overlay)
// ============================================================================

/// Spec row "captured task output in a sequence": an agent task's output reaches
/// the next step raw through a param and through `last(outputs)`.
#[test]
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

/// Install a fake `goose` that runs `prelude` and then replaces `document`
/// with exactly `content`, the way an agent that rewrites the whole file would.
fn whole_file_agent(fixture: &CliProcessFixture, document: &std::path::Path, content: &str) {
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\nprintf '%s' {} > {}\nprintf '%s\\n' 'Wrote the document.'\nexit 0\n",
            sh_quote(content),
            sh_quote(&document.display().to_string())
        ),
    );
}

fn frontmatter_value(document: &str, key: &str) -> serde_json::Value {
    let markdown: darkmatter::markdown::Markdown = document.to_string().into();
    markdown.frontmatter().as_map().get(key).cloned().unwrap_or_default()
}

/// The spec's inline acceptance case, end to end through `inline-compose`.
///
/// The agent adds four values. After run 1 the file is valid YAML: `summary`
/// and `cmd` are literal tokens, `note` and `title` are quoted strings, the
/// completion schema judged the decoded `summary` (its pattern rejects the
/// token spelling), and the stamped hash agrees with the bytes. Run 2 reads
/// every value back as the agent's exact text, the unchanged authored
/// `{{ area }}` still fills in, and the stored tokens keep their bytes.
#[test]
fn inline_agent_values_are_stored_as_data_and_read_back_exactly() {
    let fixture = CliProcessFixture::named("agent-text-inline-accept");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        concat!(
            "---\n",
            "area: here\n",
            "area_note: \"in {{ area }}\"\n",
            "$schema:\n",
            "  summary: 'string(pattern(^fixed ); required)'\n",
            "prompt: \"S=[{{ summary || 'none' }}] N=[{{ note || 'none' }}] C=[{{ cmd || 'none' }}] T=[{{ title || 'none' }}] A=[{{ area_note }}]\"\n",
            "---\n",
            "old body\n",
        ),
    )
    .unwrap();

    common::InlineAgentStub::new(&md)
        .frontmatter_additions(
            "summary: fixed {{…}} parsing\nnote: see issue #42\ncmd: \"$(echo X)\"\ntitle: Fix: colons\n",
        )
        .body("first body\n")
        .install(fixture.bin_dir(), "goose");
    let (first_code, first_stderr) =
        run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);
    assert_eq!(first_code, 0, "run 1 (the completion schema sees decoded text); stderr:\n{first_stderr}");
    assert_no_shell_approval(&first_stderr);

    let stored = fs::read_to_string(&md).unwrap();
    use darkmatter::markdown::literal_token::encode;
    assert_eq!(frontmatter_value(&stored, "summary"), serde_json::json!(encode("fixed {{…}} parsing")));
    assert_eq!(frontmatter_value(&stored, "cmd"), serde_json::json!(encode("$(echo X)")));
    assert!(stored.contains("note: \"see issue #42\"\n"), "{stored}");
    assert!(stored.contains("title: \"Fix: colons\"\n"), "{stored}");
    assert!(stored.contains("area_note: \"in {{ area }}\"\n"), "authored bytes kept:\n{stored}");

    // `md hash --diff` compares with the same library call.
    let markdown: darkmatter::markdown::Markdown = stored.clone().into();
    let options = darkmatter::markdown::MdHashOptions {
        forced_kind: Some(darkmatter::markdown::hash::MdHashKind::Simple),
        ..Default::default()
    };
    let hash = frontmatter_value(&stored, "hash");
    let stored_hash = darkmatter::markdown::hash::StoredHash::parse(&hash, &options.property).unwrap();
    let comparison = markdown.compare_hash(&stored_hash, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed, "{comparison:?}");

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

    assert_eq!(second_code, 0, "run 2; stderr:\n{second_stderr}");
    assert_no_shell_approval(&second_stderr);
    let prompt = fs::read_to_string(&prompt_log).unwrap_or_default();
    assert!(
        prompt.contains("S=[fixed {{…}} parsing] N=[see issue #42] C=[$(echo X)] T=[Fix: colons] A=[in here]"),
        "run 2 must read back the exact text; prompt:\n{prompt}"
    );
    let rewritten = fs::read_to_string(&md).unwrap();
    for key in ["summary", "cmd", "note", "title"] {
        assert_eq!(frontmatter_value(&rewritten, key), frontmatter_value(&stored, key), "{key}");
    }
}

/// Spec "inline, unrepairable YAML": a duplicate key or a malformed nested
/// value names the line and the agent, and the rollback restores the pre-run
/// bytes exactly.
#[test]
fn unrepairable_agent_frontmatter_names_the_line_and_rolls_back() {
    let original = "---\ntitle: t\nprompt: write it\n---\nold body\n";
    for (label, agent_wrote, line) in [
        ("duplicate key", "---\ntitle: t\nprompt: write it\ntitle: again\n---\nnew body\n", 4),
        ("bad nesting", "---\ntitle: t\nprompt: write it\nmeta:\n  a: b: c\n---\nnew body\n", 5),
    ] {
        let fixture = CliProcessFixture::named("agent-text-inline-unrepairable");
        let md = fixture.cwd().join("doc.md");
        fs::write(&md, original).unwrap();
        whole_file_agent(&fixture, &md, agent_wrote);

        let (code, stderr) = run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);

        assert_ne!(code, 0, "{label}: stderr:\n{stderr}");
        let flat = stderr.split_whitespace().filter(|word| *word != "┃").collect::<Vec<_>>().join(" ");
        assert!(flat.contains(&format!("(line {line})")), "{label}: names the line:\n{stderr}");
        assert!(flat.contains("The agent wrote this line"), "{label}: names the agent:\n{stderr}");
        assert_eq!(fs::read_to_string(&md).unwrap(), original, "{label}: rolled back");
    }
}

/// Review 1, "Malformed structured YAML is silently converted into text": a
/// value that starts a quoted scalar or flow collection but does not close it
/// is refused on its line and attributed to the agent, and the rollback
/// restores the pre-run bytes; plain text with a `: ` is still repaired.
#[test]
fn malformed_structured_agent_values_are_refused_not_saved_as_text() {
    let original = "---\nprompt: write it\ntitle: t\n---\nold body\n";
    for value in ["\"half\" quoted", "'half' quoted", "[a, b", "{k: v"] {
        let fixture = CliProcessFixture::named("agent-text-inline-malformed");
        let md = fixture.cwd().join("doc.md");
        fs::write(&md, original).unwrap();
        whole_file_agent(
            &fixture,
            &md,
            &format!("---\nprompt: write it\ntitle: t\nadded: {value}\n---\nnew body\n"),
        );

        let (code, stderr) = run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);

        assert_ne!(code, 0, "{value}: stderr:\n{stderr}");
        let flat = stderr.split_whitespace().filter(|word| *word != "┃").collect::<Vec<_>>().join(" ");
        assert!(flat.contains("(line 4, `added`)"), "{value}: names the line:\n{stderr}");
        assert!(flat.contains("The agent wrote this line"), "{value}: names the agent:\n{stderr}");
        assert_eq!(fs::read_to_string(&md).unwrap(), original, "{value}: rolled back");
    }

    // Control: plain text the agent meant as a string is still repaired.
    let fixture = CliProcessFixture::named("agent-text-inline-malformed");
    let md = fixture.cwd().join("doc.md");
    fs::write(&md, original).unwrap();
    whole_file_agent(&fixture, &md, "---\nprompt: write it\ntitle: t\nadded: Fix: colons\n---\nnew body\n");
    let (code, stderr) = run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);
    assert_eq!(code, 0, "plain text repairs; stderr:\n{stderr}");
    assert!(fs::read_to_string(&md).unwrap().contains("added: \"Fix: colons\"\n"));
}

/// Spec "inline, unrepairable YAML" formatting clause: CRLF and block scalar
/// documents keep their formatting through repair and encoding.
#[test]
fn crlf_and_block_scalar_documents_keep_their_formatting() {
    let fixture = CliProcessFixture::named("agent-text-inline-format");
    let md = fixture.cwd().join("doc.md");
    let original = "---\r\nprompt: write it\r\nkeep: |\r\n    four spaces: {{ x }}\r\n\r\n    # not a comment\r\n---\r\nold body\r\n";
    fs::write(&md, original).unwrap();
    whole_file_agent(
        &fixture,
        &md,
        "---\r\nprompt: write it\r\nkeep: |\r\n    four spaces: {{ x }}\r\n\r\n    # not a comment\r\nnote: see issue #42\r\nlog: |\r\n  one {{ x }}\r\n  two\r\n---\r\nnew body\r\n",
    );

    let (code, stderr) = run(&fixture, &["inline-compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let stored = fs::read_to_string(&md).unwrap();
    // The body goes through Darkmatter's cleanup pass, which has its own
    // line-ending rules; the frontmatter is edited in place.
    let frontmatter = &stored[..stored.find("new body").unwrap()];
    assert!(
        !frontmatter.replace("\r\n", "").contains('\n'),
        "every frontmatter line ending is CRLF: {stored:?}"
    );
    assert!(
        stored.contains("keep: |\r\n    four spaces: {{ x }}\r\n\r\n    # not a comment\r\n"),
        "the unchanged block scalar keeps its bytes: {stored:?}"
    );
    assert!(stored.contains("note: \"see issue #42\"\r\n"), "{stored:?}");
    assert_eq!(
        stored_frontmatter_text(&stored, "log"),
        // A clipped block ending the frontmatter reads without its final
        // newline, which is what composition reads too.
        "one {{ x }}\ntwo",
        "the agent's block scalar is stored as its exact text: {stored:?}"
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

/// A log an earlier inline run persisted holds `message_to_agent` as a literal
/// token. `frontmatter()` reads it back as the agent's text, so the message
/// carries that text and never the token spelling.
#[test]
fn lifecycle_field_from_a_stored_literal_token_is_sent_as_its_text() {
    use darkmatter::markdown::literal_token::encode_yaml_scalar;
    let fixture = CliProcessFixture::named("agent-text-message");
    counting_goose(&fixture, &write_agent_log(&encode_yaml_scalar("see {{ title }} and $(echo X)")), "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: \"agent says: {{ frontmatter('log.md', 'message_to_agent') }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(
        stderr.contains("agent says: see {{ title }} and $(echo X)"),
        "the message must carry the decoded text; stderr:\n{stderr}"
    );
    assert!(!stderr.contains("{{!data:"), "the token spelling must not leak; stderr:\n{stderr}");
    assert_no_shell_approval(&stderr);
}

/// Spec row "agent-written files read by expression": a stack action operand
/// sends the agent's exact text instead of re-expanding it.
#[test]
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

/// Spec row "lifecycle values derived from the above": an `initialize` proxy
/// is hoisted to the command coordinator, which prepares the target itself —
/// for a looping target, the loop seed and pre-flight compose from the
/// coordinator's options. The overlay stays inert on that route too.
#[test]
fn initialize_proxy_with_from_file_data_stays_inert_in_a_looping_target() {
    let fixture = CliProcessFixture::named("agent-text-proxy-initialize");
    counting_goose(&fixture, "", "ok");
    fs::write(
        fixture.cwd().join("data.md"),
        "---\nv: 'agent said {{…}} and $(echo INJECTED)'\nc: '$(echo INJECTED)'\n---\nData.\n",
    )
    .unwrap();
    fs::write(
        fixture.cwd().join("target.md"),
        "---\ntitle: target-title\nnote: none\ncounter: 0\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\n---\nNote: [{{ note }}]\nCmd: {{ cmd }} end\n",
    )
    .unwrap();
    let md = fixture.cwd().join("router.md");
    fs::write(
        &md,
        "---\ntitle: t\ninitialize:\n  stack:\n    - action: {action: proxy, target: './target.md', with: {note: \"{{ frontmatter('data.md', 'v') }}\", cmd: \"{{ frontmatter('data.md', 'c') }}\"}}\n---\nRouter.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_no_shell_approval(&stderr);
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    assert_eq!(
        log.matches("Note: [agent said {{…}} and $(echo INJECTED)]").count(),
        2,
        "both iterations must receive the overlay raw; prompts:\n{log}\nstderr:\n{stderr}"
    );
    assert_eq!(
        log.matches("Cmd: $(echo INJECTED) end").count(),
        2,
        "a whole-value command shape from data is text; prompts:\n{log}\nstderr:\n{stderr}"
    );
}

/// Row B16: the overlay stays inert when a retry re-prepares the proxied
/// target, the harness path that re-applies `proxy.with:` on every attempt.
#[test]
fn lifecycle_proxy_with_from_agent_data_stays_inert_on_a_retry() {
    let fixture = CliProcessFixture::named("agent-text-proxy-retry");
    // Call 1 is the source (it writes the agent log), call 2 is the target's
    // first attempt and fails, call 3 is the target's retry.
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\n\
             n=0\n\
             if [ -f \"$HOME/n.txt\" ]; then IFS= read -r n < \"$HOME/n.txt\"; fi\n\
             n=$((n + 1))\n\
             printf '%s' \"$n\" > \"$HOME/n.txt\"\n\
             {{ printf '=== call %s ===\\n' \"$n\"; printf '%s\\n' \"$*\"; /bin/cat; }} >> \"$HOME/prompts.txt\"\n\
             if [ \"$n\" = 1 ]; then {}; fi\n\
             if [ \"$n\" = 2 ]; then exit 1; fi\n\
             printf '%s\\n' done\n",
            write_agent_log("'agent said {{ title }}'")
        ),
    );
    fs::write(
        fixture.cwd().join("target.md"),
        "---\ntitle: target-title\nnote: none\nfailure:\n  stack:\n    - action: {retry: 1}\n---\nNote: [{{ note }}]\n",
    )
    .unwrap();
    let md = fixture.cwd().join("source.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  stack:\n    - action: {action: proxy, target: './target.md', with: {note: \"{{ frontmatter('log.md', 'message_to_agent') }}\"}}\n---\nSource.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(code, 0, "the retry must succeed; stderr:\n{stderr}");
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    let retry = log.split("=== call 3 ===").nth(1).unwrap_or_default();
    assert!(
        retry.contains("Note: [agent said {{ title }}]"),
        "the retried target must receive the overlay raw; prompts:\n{log}\nstderr:\n{stderr}"
    );
}

// ============================================================================
// Inventory I2, Table B: further re-entry points
// ============================================================================

/// Row B3: a lifecycle shell command built from file data runs the approved
/// bytes, not a late re-resolution of the data.
#[test]
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
    assert_eq!(
        stored_frontmatter_text(&state, "note"),
        "agent said {{ title }}",
        "the written value must be the agent's exact text; state.md:\n{state}\nstderr:\n{stderr}"
    );
}

/// The text a frontmatter value holds as Darkmatter reads it: a stored literal
/// token decoded, anything else as written.
fn stored_frontmatter_text(document: &str, key: &str) -> String {
    let markdown: darkmatter::markdown::Markdown = document.to_string().into();
    let value = markdown.frontmatter().as_map().get(key).cloned().unwrap_or_default();
    darkmatter::markdown::literal_token::decode_literal_tokens(&value)
        .expect("a well-formed token")
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// Rows B14 and B15: data a lifecycle effect persists into the active
/// document's frontmatter is stored as a literal token, so the next run, which
/// reads the file afresh, renders the agent's exact text instead of scanning
/// it as a template.
#[test]
fn set_frontmatter_persisted_agent_data_survives_next_preparation() {
    let fixture = CliProcessFixture::named("agent-text-persist");
    counting_goose(&fixture, &write_agent_log("'see {{…}} siblings'"), "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nnote: none\nsuccess:\n  stack:\n    - action: {set_frontmatter: [\"doc.md\", \"note\", \"{{ frontmatter('log.md', 'message_to_agent') }}\"]}\n---\nNote: [{{ note }}]\n",
    )
    .unwrap();

    let (first_code, first_stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);
    assert_eq!(first_code, 0, "first run; stderr:\n{first_stderr}");
    let stored = fs::read_to_string(&md).unwrap();
    let markdown: darkmatter::markdown::Markdown = stored.clone().into();
    assert_eq!(
        markdown.frontmatter().as_map().get("note"),
        Some(&serde_json::json!(darkmatter::markdown::literal_token::encode("see {{…}} siblings"))),
        "the persisted value is a literal token on disk:\n{stored}"
    );

    let (second_code, second_stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_eq!(second_code, 0, "the rerun must prepare; stderr:\n{second_stderr}");
    assert_no_shell_approval(&second_stderr);
    let prompts = later_prompts(&fixture);
    assert!(
        prompts.contains("Note: [see {{…}} siblings]"),
        "the persisted value must render verbatim; prompts:\n{prompts}\nstderr:\n{second_stderr}"
    );
    assert_eq!(stored_frontmatter_text(&fs::read_to_string(&md).unwrap(), "note"), "see {{…}} siblings");
}

// ============================================================================
// Regressions: what a person typed is still a template; guards still fire
// ============================================================================

/// `--set` stays a template (ruling N1): the setter fills in on every loop
/// iteration, even though the loop carries it through its seed.
#[test]
fn set_value_template_still_fills_in_on_every_iteration() {
    let fixture = CliProcessFixture::named("agent-text-set-template");
    counting_goose(&fixture, "", "ok");
    let md = fixture.cwd().join("loop.md");
    fs::write(
        &md,
        "---\ntitle: t\ncounter: 0\nloop:\n  while: 'counter < 1'\n  actions:\n    - 'increment(counter)'\n---\nX: [{{ x }}]\n",
    )
    .unwrap();

    let (code, stderr) = run(
        &fixture,
        &["compose", "--goose", "--set", r#"{"x":"{{ title }}"}"#, md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let log = fs::read_to_string(fixture.home().join("prompts.txt")).unwrap_or_default();
    assert_eq!(
        log.matches("X: [t]").count(),
        2,
        "both iterations must fill in the typed template; prompts:\n{log}"
    );
}

/// The nested-span-in-literal guard still refuses before the provider starts.
#[test]
fn nested_span_in_a_lifecycle_literal_is_still_refused() {
    let fixture = CliProcessFixture::named("agent-text-nested-guard");
    counting_goose(&fixture, "", "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: \"{{ 'in {{ title }}' }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_ne!(code, 0, "stderr:\n{stderr}");
    assert!(stderr.contains("success.info"), "the guard names the property:\n{stderr}");
    assert!(
        !fixture.home().join("prompts.txt").exists(),
        "no provider may start; stderr:\n{stderr}"
    );
}

/// Strict whole-value evaluation still fails closed: an authored lifecycle
/// span with an unknown root is an error, not an empty message.
#[test]
fn an_unknown_root_in_an_authored_lifecycle_span_still_fails() {
    let fixture = CliProcessFixture::named("agent-text-strict-guard");
    counting_goose(&fixture, "", "ok");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: \"{{ titel }}\"\n---\nBody.\n",
    )
    .unwrap();

    let (code, stderr) = run(&fixture, &["compose", "--goose", md.to_str().unwrap()]);

    assert_ne!(code, 0, "stderr:\n{stderr}");
    assert!(stderr.contains("lifecycle evaluation error"), "{stderr}");
    assert!(stderr.contains("success.info"), "{stderr}");
}
