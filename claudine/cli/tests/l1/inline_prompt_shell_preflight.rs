//! Shell pre-flight for runs that compose a document's `prompt` property
//! instead of its body: `inline-compose`, an inline sequence step, a proxy
//! into an inline target, and a retried inline attempt. Discovery must walk the text the run composes, so a
//! permitted `prompt` command runs, an unpermitted one is refused as needing
//! approval, and a body command (which an inline run never composes) is never
//! asked about.

use std::fs;
use std::path::Path;

use crate::common;
use common::{CliProcessFixture, InlineAgentStub, strip_ansi, write_executable};

/// An inline document whose `prompt` runs `prompt_command` and whose body
/// runs a command no rule permits.
fn inline_document(prompt_command: &str, extra_frontmatter: &str) -> String {
    format!(
        "---\nagent: goose\nprompt: |\n  Update the body.\n\n  ::shell {prompt_command}\n{extra_frontmatter}---\n# Body\n\n::shell echo body-only-command\n"
    )
}

/// An agent that records its argv (the delivered prompt) in `argv` and then
/// rewrites `document`'s body.
fn install_recording_agent(fixture: &CliProcessFixture, document: &Path, argv: &Path) {
    let prelude = format!("printf '%s\\n' \"$@\" >> '{}'\n", argv.display());
    InlineAgentStub::new(document)
        .prelude(&prelude)
        .body("Replacement body\n")
        .install(fixture.bin_dir(), "goose");
}

fn run_inline(fixture: &CliProcessFixture, document: &Path) -> (Option<i32>, String) {
    let output = fixture
        .command()
        .args(["inline-compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    (output.status.code(), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
}

#[test]
fn inline_prompt_permitted_command_runs_into_the_delivered_prompt() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-permitted");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact echo inline-prompt-marker\n");
    let document = fixture.cwd().join("doc.md");
    fs::write(&document, inline_document("echo inline-prompt-marker", "")).unwrap();
    let argv = fixture.cwd().join("argv.txt");
    install_recording_agent(&fixture, &document, &argv);

    let (code, stderr) = run_inline(&fixture, &document);

    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stderr.contains("pre-flight scanner"), "{stderr}");
    let prompt = fs::read_to_string(&argv).unwrap();
    assert!(prompt.contains("inline-prompt-marker"), "the command's output is in the prompt:\n{prompt}");
    assert!(!prompt.contains("::shell"), "the directive was expanded:\n{prompt}");
}

/// A status block wrapped at any character (a hyphen included), with its
/// rule and whitespace removed.
fn unwrapped(stderr: &str) -> String {
    stderr.chars().filter(|c| *c != '┃' && !c.is_whitespace()).collect()
}

/// The refusal names the real file and its `prompt` property, never a line of
/// the generated document the run composes (which would be past the end of
/// this file), on the direct, initialized, and loop routes alike.
#[test]
fn inline_prompt_unpermitted_command_without_a_terminal_is_refused_as_needing_approval() {
    let routes = [
        ("direct", ""),
        ("initialized", "initialize:\n  - {stdout: initialized}\n"),
        ("loop", "loop: {until: 'false', max: 1}\n"),
    ];
    for (route, extra_frontmatter) in routes {
        let fixture = CliProcessFixture::named(&format!("inline-prompt-shell-refused-{route}"));
        fixture.seed_user_config();
        let document = fixture.cwd().join("doc.md");
        fs::write(&document, inline_document("echo inline-prompt-unpermitted", extra_frontmatter)).unwrap();
        let argv = fixture.cwd().join("argv.txt");
        install_recording_agent(&fixture, &document, &argv);

        let (code, stderr) = run_inline(&fixture, &document);

        assert_ne!(code, Some(0), "{route}: {stderr}");
        let unwrapped = unwrapped(&stderr);
        assert!(
            unwrapped.contains("'echoinline-prompt-unpermitted'") && unwrapped.contains("requiresapproval"),
            "{route}: the ordinary approval refusal names the prompt's command:\n{stderr}"
        );
        assert!(unwrapped.contains("doc.md(frontmatter`prompt`)"), "{route}: names the prompt:\n{stderr}");
        assert!(!unwrapped.contains("doc.md:"), "{route}: no generated line:\n{stderr}");
        assert!(!stderr.contains("body-only-command"), "{route}: the body is not audited:\n{stderr}");
        assert!(!stderr.contains("pre-flight scanner"), "{route}: {stderr}");
        assert!(!argv.exists(), "{route}: the agent must not launch");
    }
}

/// A permitted `prompt` command that fails reports the real file and the
/// `prompt` property: no `unknown` source, no generated line or excerpt.
#[test]
fn inline_prompt_command_failure_names_the_real_file_and_prompt() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-failure");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact mkdir taken\n");
    // `mkdir` of an existing directory fails under `sh` and `cmd` alike.
    fs::create_dir(fixture.cwd().join("taken")).unwrap();
    let document = fixture.cwd().join("doc.md");
    fs::write(&document, inline_document("mkdir taken", "")).unwrap();
    let argv = fixture.cwd().join("argv.txt");
    install_recording_agent(&fixture, &document, &argv);

    let (code, stderr) = run_inline(&fixture, &document);

    assert_ne!(code, Some(0), "{stderr}");
    assert!(stderr.contains("mkdir taken"), "the failure names the command:\n{stderr}");
    assert!(stderr.contains("doc.md"), "the failure names the real file:\n{stderr}");
    assert!(stderr.contains("frontmatter.prompt"), "the origin is the prompt property:\n{stderr}");
    assert!(!stderr.contains("unknown"), "no unknown source:\n{stderr}");
    assert!(!stderr.contains("line 10") && !stderr.contains("Origin: line"), "no generated line:\n{stderr}");
    assert!(!argv.exists(), "the agent must not launch");
}

/// An inline document with `word: taken` whose `prompt` is `prompt`
/// (indented into the frontmatter).
fn inline_prompt_document(prompt: &str) -> String {
    let indented: String = prompt.lines().map(|line| format!("  {line}\n")).collect();
    format!("---\nagent: goose\nword: taken\nprompt: |\n{indented}---\n# Body\n")
}

/// Runs `prompt` inline with `mkdir taken` permitted and `taken` existing;
/// the failure must name the real file and its `prompt` property, publish no
/// position of the generated body (which would be past the end of this short
/// file) and no `unknown` source, and launch no agent.
fn assert_prompt_failure_names_the_prompt(name: &str, prompt: &str) {
    let fixture = CliProcessFixture::named(&format!("inline-prompt-failure-{name}"));
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact mkdir taken\n");
    // `mkdir` of an existing directory fails under `sh` and `cmd` alike.
    fs::create_dir(fixture.cwd().join("taken")).unwrap();
    let document = fixture.cwd().join("doc.md");
    fs::write(&document, inline_prompt_document(prompt)).unwrap();
    let argv = fixture.cwd().join("argv.txt");
    install_recording_agent(&fixture, &document, &argv);

    let (code, stderr) = run_inline(&fixture, &document);

    assert_ne!(code, Some(0), "{stderr}");
    let flat = unwrapped(&stderr);
    assert!(flat.contains("doc.md"), "the failure names the real file:\n{stderr}");
    // The run's header names `prompt` too; the failure itself must name it
    // (`unwrapped` removed the spaces).
    assert!(
        flat.contains("doc.md(frontmatter`prompt`)") || flat.contains("Origin:frontmatter.prompt"),
        "the failure names the prompt property:\n{stderr}"
    );
    assert!(!stderr.contains("unknown"), "no unknown source:\n{stderr}");
    for generated in ["Origin: line", "at line", "Line:", "Opened at line", "Block opened at line", "Command at line"] {
        assert!(!stderr.contains(generated), "no generated position ({generated}):\n{stderr}");
    }
    assert!(!argv.exists(), "the agent must not launch");
}

/// The forms a review found publishing generated positions: a command after
/// body interpolation, a failing `::shell-block` command, an invalid block
/// option, an unclosed block, a `::map` over an undeclared source, a bad
/// `::toc-linking` level, and a bad `::file-links --depth`.
macro_rules! prompt_failure_names_the_prompt {
    ($($name:ident: $prompt:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_prompt_failure_names_the_prompt(stringify!($name), $prompt);
        }
    )*};
}

prompt_failure_names_the_prompt! {
    inline_prompt_failure_after_interpolation_names_the_prompt: "Update {{ word }}.\n\n::shell mkdir taken\n";
    inline_prompt_block_command_failure_names_the_prompt: "Update.\n::shell-block\nmkdir taken\n::end-block\n";
    inline_prompt_invalid_block_option_names_the_prompt: "Update.\n::shell-block timeout=bad\nmkdir taken\n::end-block\n";
    inline_prompt_unclosed_block_names_the_prompt: "Update.\n::shell-block\nmkdir taken\n";
    inline_prompt_collection_failure_names_the_prompt: "Update {{ word }}.\n\n::map word \"w => w\"\n- item\n::end-map\n";
    inline_prompt_toc_linking_failure_names_the_prompt: "Update {{ word }}.\n\n::toc-linking ./x.md level=h9\n";
    inline_prompt_file_links_failure_names_the_prompt: "Update {{ word }}.\n\n::file-links --dir . --depth x\n";
}

/// The staged route: `initialize` runs first, then the stabilized reread's
/// audit (`reread_and_audit`) approves the `prompt` the run composes.
#[test]
fn inline_prompt_permitted_command_runs_after_initialize() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-initialized");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact echo initialized-prompt-marker\n");
    let document = fixture.cwd().join("doc.md");
    fs::write(
        &document,
        inline_document("echo initialized-prompt-marker", "initialize:\n  - {stdout: initialized}\n"),
    )
    .unwrap();
    let argv = fixture.cwd().join("argv.txt");
    install_recording_agent(&fixture, &document, &argv);

    let (code, stderr) = run_inline(&fixture, &document);

    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("initialized"), "initialize ran:\n{stderr}");
    let prompt = fs::read_to_string(&argv).unwrap();
    assert!(prompt.contains("initialized-prompt-marker"), "{prompt}");
}

/// A sequence step whose document declares `prompt` is an inline run: the
/// graph audit approves its `prompt`'s command, and the body's unpermitted
/// command, which never runs, is not asked about.
#[test]
fn inline_sequence_step_prompt_command_is_approved_by_the_graph_audit() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-sequence-step");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact echo step-prompt-marker\n");
    let step = fixture.cwd().join("step.md");
    fs::write(&step, inline_document("echo step-prompt-marker", "")).unwrap();
    let sequence = fixture.cwd().join("seq.md");
    fs::write(&sequence, "---\nsequence:\n  - name: one\n    prompt: step.md\n---\nSequence\n").unwrap();
    let argv = fixture.cwd().join("argv.txt");
    install_recording_agent(&fixture, &step, &argv);

    let output = fixture
        .command()
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.code(), Some(0), "{stderr}");
    let prompt = fs::read_to_string(&argv).unwrap();
    assert!(prompt.contains("step-prompt-marker"), "{prompt}");
}

/// A proxy into an inline target is a fresh run whose pre-flight audits the
/// target's `prompt`, not its body.
#[test]
fn inline_proxy_target_prompt_command_is_audited_before_it_runs() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-proxy-target");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact echo proxy-target-marker\n");
    let source = fixture.cwd().join("source.md");
    fs::write(
        &source,
        "---\nagent: goose\nprompt: Route the task\nsuccess:\n  - {proxy: './target.md'}\n---\nsource body\n",
    )
    .unwrap();
    let target = fixture.cwd().join("target.md");
    fs::write(&target, inline_document("echo proxy-target-marker", "")).unwrap();
    // One stub for both documents: it learns which file it owns from the
    // delivered prompt header.
    let argv = fixture.cwd().join("argv.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{argv}'\n{doc_from_prompt}\
             CLAUDINE_ADD=''\n\
             CLAUDINE_BODY='Agent wrote this.\n'\n\
             {rewrite}\
             printf 'wrote %s\\n' \"$CLAUDINE_DOC\"\n\
             exit 0\n",
            argv = argv.display(),
            doc_from_prompt = common::INLINE_DOC_FROM_PROMPT,
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );

    let (code, stderr) = run_inline(&fixture, &source);

    assert_eq!(code, Some(0), "{stderr}");
    let prompts = fs::read_to_string(&argv).unwrap();
    assert!(prompts.contains("proxy-target-marker"), "the target's prompt ran its command:\n{prompts}");
    assert!(fs::read_to_string(&target).unwrap().contains("Agent wrote this."));
}

/// A retry is a fresh attempt whose pre-flight (`preflight_harness_document`)
/// audits the `prompt` it composes: a `retry.with` overlay changes the
/// command's bytes, and the retried attempt runs the new ones.
#[test]
fn inline_retry_prompt_command_with_new_bytes_is_audited_before_it_runs() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-retry");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "prefix echo\n");
    let document = fixture.cwd().join("doc.md");
    fs::write(
        &document,
        "---\nagent: goose\nword: first\nprompt: |\n  Update the body.\n\n  ::shell echo attempt-{{ word }}\nfailure:\n  - retry:\n      max_attempts: 1\n      with:\n        word: second\n---\n# Body\n",
    )
    .unwrap();
    let argv = fixture.cwd().join("argv.txt");
    let counter = fixture.cwd().join("launches");
    // The first launch fails before editing; the retried one edits.
    let prelude = format!(
        "printf '%s\\n' \"$@\" >> '{argv}'\nn=$(cat '{counter}' 2>/dev/null || echo 0)\necho $((n + 1)) > '{counter}'\n[ \"$n\" = 0 ] && exit 1\n",
        argv = argv.display(),
        counter = counter.display(),
    );
    InlineAgentStub::new(&document)
        .prelude(&prelude)
        .body("Replacement body\n")
        .install(fixture.bin_dir(), "goose");

    let (code, stderr) = run_inline(&fixture, &document);

    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stderr.contains("pre-flight scanner"), "{stderr}");
    let prompts = fs::read_to_string(&argv).unwrap();
    assert!(prompts.contains("attempt-first"), "{prompts}");
    assert!(prompts.contains("attempt-second"), "the retried attempt ran its own bytes:\n{prompts}");
}

/// A retried attempt whose `retry.with` overlay produces bytes no rule covers
/// is refused at the retried document's `prompt`, not at a line of the
/// generated document.
#[test]
fn inline_retry_prompt_refusal_names_the_prompt_property() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-retry-refused");
    fixture.seed_user_config();
    common::write_shell_permissions(fixture.home(), "exact echo attempt-first\n");
    let document = fixture.cwd().join("doc.md");
    fs::write(
        &document,
        "---\nagent: goose\nword: first\nprompt: |\n  Update the body.\n\n  ::shell echo attempt-{{ word }}\nfailure:\n  - retry:\n      max_attempts: 1\n      with:\n        word: second\n---\n# Body\n",
    )
    .unwrap();
    let argv = fixture.cwd().join("argv.txt");
    // Every launch fails before editing, so the retry is attempted.
    let prelude = format!("printf '%s\\n' \"$@\" >> '{}'\nexit 1\n", argv.display());
    InlineAgentStub::new(&document).prelude(&prelude).body("Replacement body\n").install(fixture.bin_dir(), "goose");

    let (code, stderr) = run_inline(&fixture, &document);

    assert_ne!(code, Some(0), "{stderr}");
    let unwrapped = unwrapped(&stderr);
    assert!(unwrapped.contains("'echoattempt-second'"), "the retried command is refused:\n{stderr}");
    assert!(unwrapped.contains("doc.md(frontmatter`prompt`)"), "names the prompt:\n{stderr}");
    assert!(!unwrapped.contains("doc.md:"), "no generated line:\n{stderr}");
}

/// A proxy target's refused `prompt` command names the target's `prompt`.
#[test]
fn inline_proxy_target_prompt_refusal_names_the_prompt_property() {
    let fixture = CliProcessFixture::named("inline-prompt-shell-proxy-refused");
    fixture.seed_user_config();
    let source = fixture.cwd().join("source.md");
    fs::write(
        &source,
        "---\nagent: goose\nprompt: Route the task\nsuccess:\n  - {proxy: './target.md'}\n---\nsource body\n",
    )
    .unwrap();
    let target = fixture.cwd().join("target.md");
    fs::write(&target, inline_document("echo proxy-target-unpermitted", "")).unwrap();
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\n{doc_from_prompt}CLAUDINE_ADD=''\nCLAUDINE_BODY='Agent wrote this.\n'\n{rewrite}exit 0\n",
            doc_from_prompt = common::INLINE_DOC_FROM_PROMPT,
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );

    let (code, stderr) = run_inline(&fixture, &source);

    assert_ne!(code, Some(0), "{stderr}");
    let unwrapped = unwrapped(&stderr);
    assert!(unwrapped.contains("'echoproxy-target-unpermitted'"), "the target's command is refused:\n{stderr}");
    assert!(unwrapped.contains("target.md(frontmatter`prompt`)"), "names the target's prompt:\n{stderr}");
    assert!(!unwrapped.contains("target.md:"), "no generated line:\n{stderr}");
}
