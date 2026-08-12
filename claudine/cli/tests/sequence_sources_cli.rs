//! The sequence *source* matrix, exercised through the real
//! `claudine sequence` invocation path.
//!
//! Every case runs under `--dry-run`, which performs the full preflight and
//! composes each step but launches no provider. That is what lets one fixture
//! prove "this source produced these steps" without a provider stub — and
//! therefore without a `unix` gate. The library owns the same matrix at unit
//! level (`composition::sequence::tests`); this binary proves the shipped
//! artifact and the normal invocation path agree with it.
//!
//! Not duplicated here: `@` magic and `!` package references have a repo-aware
//! home in `sequence_magic_reference.rs`; typed rejections live in
//! `sequence_errors_cli.rs`.

use std::fs;
use std::path::Path;
use tempfile::{TempDir, tempdir};
mod common;
use common::strip_ansi;

/// Dry-run `file` and return the composed step bodies, in order.
///
/// A step body is emitted on stdout; status framing goes to stderr. Reading
/// only stdout is therefore both the assertion and a standing check that the
/// stream split holds.
fn composed_steps(workspace: &TempDir, file: &Path, extra_args: &[&str]) -> Vec<String> {
    let (stdout, _) = dry_run(workspace, file, extra_args, true);
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("Step "))
        .map(|line| line.trim_start_matches("Step ").trim_end_matches('.').to_string())
        .collect()
}

/// Dry-run `file`, returning `(stdout, stderr)` with ANSI stripped.
fn dry_run(
    workspace: &TempDir,
    file: &Path,
    extra_args: &[&str],
    expect_success: bool,
) -> (String, String) {
    let mut args: Vec<&str> = vec!["sequence", "--dry-run"];
    args.extend_from_slice(extra_args);
    let file_arg = file.to_str().unwrap();
    args.push(file_arg);

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .current_dir(workspace.path())
        .args(&args)
        .assert();
    let assert = if expect_success { assert.success() } else { assert.failure() };
    let output = assert.get_output().clone();

    (
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

fn write(workspace: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = workspace.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, body).unwrap();
    path
}

/// A Markdown document whose body renders each step's name on its own line.
fn source_doc(workspace: &TempDir, name: &str, source: &str) -> std::path::PathBuf {
    write(
        workspace,
        name,
        &format!("---\nsequence: {source}\n---\nStep {{{{ state.name }}}}.\n"),
    )
}

// ============================================================================
// Data-file formats
// ============================================================================

/// The five supported data formats are one code path above the loader, so a
/// format that parses must produce the same plan as any other. Asserting
/// equality between them — rather than five independent expectations — is what
/// makes a future format-specific divergence visible.
#[test]
fn yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan() {
    let workspace = tempdir().unwrap();
    write(&workspace, "d.yaml", "wrap:\n  items:\n    - one\n    - two\n");
    write(&workspace, "d.json", r#"{"wrap": {"items": ["one", "two"]}}"#);
    write(
        &workspace,
        "d.json5",
        "{ wrap: { items: ['one', 'two'] } } // trailing comment\n",
    );
    write(
        &workspace,
        "d.jsonl",
        "{\"label\": \"one\"}\n{\"label\": \"two\"}\n",
    );
    write(
        &workspace,
        "d.ndjson",
        "{\"label\": \"one\"}\n{\"label\": \"two\"}\n",
    );

    let expected = vec!["one".to_string(), "two".to_string()];
    for (name, source) in [
        ("yaml.md", "\"d.yaml -> wrap.items\""),
        ("json.md", "\"d.json -> wrap.items\""),
        ("json5.md", "\"d.json5 -> wrap.items\""),
        // JSONL/NDJSON are always a list at the root, so they take an operator
        // but never an offset.
        ("jsonl.md", "\"d.jsonl::name(label)\""),
        ("ndjson.md", "\"d.ndjson::name(label)\""),
    ] {
        let doc = source_doc(&workspace, name, source);
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            expected,
            "`{source}` should normalize to the same plan as every other format"
        );
    }
}

#[test]
fn the_three_operators_each_produce_a_usable_name() {
    let workspace = tempdir().unwrap();
    write(
        &workspace,
        "d.yaml",
        "colors:\n  data:\n    - color: blue\n      rank: 5\n    - color: green\n      rank: 3\n",
    );

    // `map` renames (the source key is gone), `name` copies (it is retained),
    // and `template` computes. All three answer the same produce-a-name
    // question, which is why v1 allows exactly one per reference.
    let mapped = source_doc(&workspace, "map.md", "\"d.yaml -> colors.data::map(color, name)\"");
    assert_eq!(composed_steps(&workspace, &mapped, &[]), ["blue", "green"]);

    let named = source_doc(&workspace, "name.md", "\"d.yaml -> colors.data::name(color)\"");
    assert_eq!(composed_steps(&workspace, &named, &[]), ["blue", "green"]);

    let templated = source_doc(
        &workspace,
        "template.md",
        "\"d.yaml -> colors.data::template(color + '-' + rank)\"",
    );
    assert_eq!(composed_steps(&workspace, &templated, &[]), ["blue-5", "green-3"]);
}

#[test]
fn map_removes_the_source_key_while_name_retains_it() {
    let workspace = tempdir().unwrap();
    write(&workspace, "d.yaml", "items:\n  - color: blue\n");

    // The distinction is invisible in the step name and only observable in the
    // surviving state, which is exactly why it is asserted on the state.
    let mapped = write(
        &workspace,
        "map.md",
        "---\nsequence: \"d.yaml -> items::map(color, name)\"\n---\nStep [{{ state.color }}].\n",
    );
    assert_eq!(composed_steps(&workspace, &mapped, &[]), ["[]"], "map renames");

    let named = write(
        &workspace,
        "name.md",
        "---\nsequence: \"d.yaml -> items::name(color)\"\n---\nStep [{{ state.color }}].\n",
    );
    assert_eq!(composed_steps(&workspace, &named, &[]), ["[blue]"], "name copies");
}

// ============================================================================
// `ListFormat` classification, through the shipped classifier
// ============================================================================

/// Every classified string form reaches the sequence through the same
/// `biscuit_file::ListFormat` classifier. Driving them through the CLI proves
/// the wiring, not just the classifier's own unit tests.
#[test]
fn every_string_list_form_classifies_through_the_real_invocation_path() {
    let workspace = tempdir().unwrap();
    let cases: &[(&str, &str, &[&str])] = &[
        ("csv", "\"alpha, beta, gamma\"", &["alpha", "beta", "gamma"]),
        ("tsv", "\"alpha\tbeta\"", &["alpha", "beta"]),
        ("lines", "\"alpha\nbeta\"", &["alpha", "beta"]),
        ("spaces", "\"alpha beta gamma\"", &["alpha", "beta", "gamma"]),
        ("scalar", "\"solo\"", &["solo"]),
    ];

    for (name, raw, expected) in cases {
        let doc = write(
            &workspace,
            &format!("{name}.md"),
            &format!("---\nraw: {raw}\nsequence: \"{{{{ raw }}}}\"\n---\nStep {{{{ state.name }}}}.\n"),
        );
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            *expected,
            "`{name}` did not classify as expected"
        );
    }
}

#[test]
fn markdown_list_markers_win_over_line_splitting() {
    let workspace = tempdir().unwrap();
    // Both bodies are multi-line, so line-separation would also "work" — the
    // marker precedence rule is what strips the bullets and numbers.
    let unordered = write(
        &workspace,
        "ul.md",
        "---\nraw: |\n  - alpha\n  - beta\nsequence: \"{{ raw }}\"\n---\nStep {{ state.name }}.\n",
    );
    assert_eq!(composed_steps(&workspace, &unordered, &[]), ["alpha", "beta"]);

    let ordered = write(
        &workspace,
        "ol.md",
        "---\nraw: |\n  1. alpha\n  2. beta\nsequence: \"{{ raw }}\"\n---\nStep {{ state.name }}.\n",
    );
    assert_eq!(composed_steps(&workspace, &ordered, &[]), ["alpha", "beta"]);
}

#[test]
fn quoted_csv_delimiters_and_crlf_survive_classification() {
    let workspace = tempdir().unwrap();
    // A naive `split(',')` would produce four entries here, and a naive line
    // split would leave a trailing `\r` on every name.
    let doc = write(
        &workspace,
        "quoted.md",
        "---\nraw: \"\\\"alpha, with comma\\\", beta\"\nsequence: \"{{ raw }}\"\n---\nStep {{ state.name }}.\n",
    );
    assert_eq!(
        composed_steps(&workspace, &doc, &[]),
        ["alpha, with comma", "beta"]
    );

    let crlf = write(
        &workspace,
        "crlf.md",
        "---\nraw: \"alpha\\r\\nbeta\"\nsequence: \"{{ raw }}\"\n---\nStep [{{ state.name }}].\n",
    );
    assert_eq!(composed_steps(&workspace, &crlf, &[]), ["[alpha]", "[beta]"]);
}

// ============================================================================
// Strictness by provenance
// ============================================================================

/// Foreign data is lenient where an authored inline list is strict: scalars
/// coerce and a nameless object gets an ordinal name. `sequence_errors_cli.rs`
/// pins the strict half of the same boundary.
#[test]
fn a_foreign_source_coerces_scalars_and_names_nameless_objects() {
    let workspace = tempdir().unwrap();
    let scalars = write(
        &workspace,
        "scalars.md",
        "---\nraw:\n  - 1\n  - 2.5\n  - true\nsequence: \"{{ raw }}\"\n---\nStep {{ state.name }}.\n",
    );
    assert_eq!(composed_steps(&workspace, &scalars, &[]), ["1", "2.5", "true"]);

    let objects = write(
        &workspace,
        "objects.md",
        "---\nraw:\n  - name: alpha\n  - topic: parsing\nsequence: \"{{ raw }}\"\n---\nStep {{ state.name }}.\n",
    );
    assert_eq!(
        composed_steps(&workspace, &objects, &[]),
        ["alpha", "2"],
        "a nameless object takes its one-based ordinal"
    );
}

#[test]
fn a_shell_expanded_source_becomes_a_classified_list() {
    let workspace = tempdir().unwrap();
    // `echo` is one of the few commands with the same surface on `sh` and
    // `cmd`, which keeps this case off a platform gate. `--yolo` stands in for
    // the approval the preflight would otherwise ask for.
    let doc = source_doc(&workspace, "shell.md", "\"$(echo alpha,beta)\"");
    assert_eq!(composed_steps(&workspace, &doc, &["--yolo"]), ["alpha", "beta"]);
}

// ============================================================================
// `FileReference` families
// ============================================================================

/// Resolution is delegated to `biscuit_file::FileReference::resolve_from` with
/// the *authoring document's* directory, so the families below work without the
/// sequence code reimplementing any of them — including the two that a
/// hand-rolled suffix splitter would break: a path holding a space, and a path
/// holding an `@`. The home-pinned case uses the same explicit request-context
/// seam because native Windows home discovery cannot be isolated by overriding
/// environment variables in a child process.
#[test]
fn file_references_resolve_from_the_authoring_document() {
    let workspace = tempdir().unwrap();
    write(&workspace, "nested/data.yaml", "items:\n  - nested-hit\n");
    write(&workspace, "my data/data.yaml", "items:\n  - spaced-hit\n");
    write(&workspace, "a@b/data.yaml", "items:\n  - at-hit\n");
    write(&workspace, "tilde.yaml", "items:\n  - tilde-hit\n");

    for (name, source, expected) in [
        ("relative.md", "\"nested/data.yaml -> items\"", "nested-hit"),
        ("spaced.md", "\"my data/data.yaml -> items\"", "spaced-hit"),
        ("at.md", "\"a@b/data.yaml -> items\"", "at-hit"),
    ] {
        let doc = source_doc(&workspace, name, source);
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            [expected],
            "`{source}` did not resolve"
        );
    }

    let doc = source_doc(&workspace, "tilde.md", "\"~/tilde.yaml -> items\"");
    let request_context = biscuit_file::FileResolutionContext::new(workspace.path())
        .with_home_dir(workspace.path())
        .with_source_path(&doc);
    let source = claudine::composition::resolve_composition_source_in_context(
        doc.to_str().unwrap(),
        &request_context,
    )
    .unwrap();
    let plan = claudine::composition::resolve_sequence_plan_with(
        &source,
        claudine::composition::SequenceSourceOptions {
            shell_runner: None,
            file_resolution_context: Some(&request_context),
        },
    )
    .unwrap()
    .expect("tilde fixture declares a sequence");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].name, "tilde-hit");
}

#[test]
fn a_reference_resolves_relative_to_its_own_document_not_the_process_cwd() {
    let workspace = tempdir().unwrap();
    write(&workspace, "deep/data.yaml", "items:\n  - deep-hit\n");
    // A sibling reference, authored one directory down. Resolving from the
    // process CWD (the workspace root) would miss it entirely.
    let doc = source_doc(&workspace, "deep/seq.md", "\"data.yaml -> items\"");

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["deep-hit"]);
}

#[test]
fn a_quoted_operator_argument_keeps_its_delimiters() {
    let workspace = tempdir().unwrap();
    // The field name holds a comma — the argument separator — so an unquoted
    // splitter would read this as a two-argument `name(...)` and fail arity.
    write(&workspace, "d.yaml", "items:\n  - \"k, v\": quoted-hit\n");
    let doc = source_doc(&workspace, "quoted-arg.md", "\"d.yaml -> items::name(\\\"k, v\\\")\"");

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["quoted-hit"]);
}

// ============================================================================
// Formal sequence documents
// ============================================================================

#[test]
fn a_referenced_formal_document_applies_template_before_generated_fields() {
    let workspace = tempdir().unwrap();
    write(
        &workspace,
        "formal.yaml",
        "kind: sequence\nsequence:\n  - name: blue\n    color: blue\n    rank: 5\ntemplate:\n  desc: \"{{ color }}({{ rank }})\"\n",
    );
    let doc = write(
        &workspace,
        "ref.md",
        "---\nsequence: formal.yaml\n---\nStep {{ state.desc }}/{{ state.index }}.\n",
    );

    // A templated key is ordinary authored state by the time `index` is
    // generated, so both are readable from the same state object.
    assert_eq!(composed_steps(&workspace, &doc, &[]), ["blue(5)/1"]);
}

#[test]
fn a_referenced_formal_documents_schema_validates_the_step_state() {
    let workspace = tempdir().unwrap();
    write(
        &workspace,
        "formal.yaml",
        "kind: sequence\nsequence:\n  - name: blue\n    rank: not-a-number\n$schema:\n  rank: number(required)\n",
    );
    let doc = source_doc(&workspace, "ref.md", "formal.yaml");

    let (_, stderr) = dry_run(&workspace, &doc, &[], false);
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("sequence step 0 (`blue`) failed schema validation at `/rank`"),
        "the violation must name the step index, its id, and the failing property; stderr:\n{flat}"
    );
}

/// The whole formal-document contract in one fixture, reached both ways.
///
/// The file is written once and invoked twice — directly, and through a
/// Markdown document that references it — because the spec's "both entry modes
/// accept the identical document shape" is only meaningful if the *same bytes*
/// normalize the same way. It exercises scalar shorthand (`- blue`), an
/// interpolated template value, a typed literal template default, a template
/// default an item overrides, and generated fields, all of which the root
/// `$schema` must then accept as step state.
///
/// The two entry modes differ in exactly one respect: a directly invoked YAML
/// has no Markdown body, so its steps render through a document-level `prompt`
/// the referencing document supplies as its body instead. Both strings are
/// therefore identical, and so are both results.
fn write_parity_fixture(workspace: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    const STEP_BODY: &str = "Step {{ state.name }}/{{ state.desc }}/{{ state.rank }}.";

    let direct = write(
        workspace,
        "formal.yaml",
        &format!(
            "kind: sequence\nprompt: \"{STEP_BODY}\"\n\
             sequence:\n  - blue\n  - name: red\n    rank: 3\n\
             template:\n  desc: \"{{{{ name }}}}!\"\n  rank: 5\n\
             $schema:\n  desc: string(required)\n  rank: number(required)\n"
        ),
    );
    let referenced = write(
        workspace,
        "ref.md",
        &format!("---\nsequence: formal.yaml\n---\n{STEP_BODY}\n"),
    );
    (direct, referenced)
}

#[test]
fn a_formal_document_normalizes_identically_through_both_entry_paths() {
    let workspace = tempdir().unwrap();
    let (direct, referenced) = write_parity_fixture(&workspace);

    let expected = ["blue/blue!/5", "red/red!/3"];
    assert_eq!(
        composed_steps(&workspace, &direct, &[]),
        expected,
        "direct invocation"
    );
    assert_eq!(
        composed_steps(&workspace, &referenced, &[]),
        expected,
        "the same file, referenced"
    );
}

/// The counterpart to
/// [`a_referenced_formal_documents_schema_validates_the_step_state`]: a
/// directly invoked document's root `$schema` is the *step state* schema too,
/// not its own frontmatter schema. Before this was unified, the direct mode
/// reported `rank` merely missing from the document root.
#[test]
fn a_directly_invoked_documents_schema_validates_the_step_state() {
    let workspace = tempdir().unwrap();
    let doc = write(
        &workspace,
        "formal.yaml",
        "kind: sequence\nprompt: \"Handle {{ state.name }}\"\nsequence:\n  - name: blue\n    rank: not-a-number\n$schema:\n  rank: number(required)\n",
    );

    let (_, stderr) = dry_run(&workspace, &doc, &[], false);
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("sequence step 0 (`blue`) failed schema validation at `/rank`"),
        "the direct mode must report the same step-state violation the referenced mode does; \
         stderr:\n{flat}"
    );
}

/// A directly invoked YAML file has no Markdown body, so every step must carry
/// an executable — the degenerate case the unified step/task model predicts.
///
/// Regression: the just-in-time re-read parsed the source as plain Markdown,
/// which for a `.yaml` file yields an empty frontmatter. That silently discarded
/// the document's entire configuration at step-compose time and broke this entry
/// mode outright.
#[test]
fn a_directly_invoked_yaml_runs_executable_steps_without_a_body() {
    let workspace = tempdir().unwrap();
    write(&workspace, "p.md", "---\ntitle: p\n---\nStep {{ state.name }}.\n");
    let doc = write(
        &workspace,
        "headless.yaml",
        "kind: sequence\nsequence:\n  - name: alpha\n    prompt: p.md\n  - name: beta\n    prompt: p.md\n",
    );

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["alpha", "beta"]);
}

/// The second step is the one that proves the fix: it composes against the
/// re-read source, so a reload that lost the YAML frontmatter would drop
/// `topic` even though the first step rendered it.
#[test]
fn a_directly_invoked_yaml_keeps_its_frontmatter_across_step_boundaries() {
    let workspace = tempdir().unwrap();
    let doc = write(
        &workspace,
        "headless.yaml",
        "kind: sequence\ntopic: from-document\nprompt: \"Step {{ state.name }}/{{ topic }}.\"\nsequence:\n  - alpha\n  - beta\n",
    );

    assert_eq!(
        composed_steps(&workspace, &doc, &[]),
        ["alpha/from-document", "beta/from-document"]
    );
}

#[test]
fn a_bodyless_step_that_declares_no_executable_is_still_rejected() {
    let workspace = tempdir().unwrap();
    // The empty-body guard is relaxed only for a step that runs a task; a step
    // that would send an empty prompt to a provider must still say so.
    let doc = write(&workspace, "empty.md", "---\nsequence:\n  - alpha\n---\n");

    let (_, stderr) = dry_run(&workspace, &doc, &[], false);
    assert!(
        stderr.contains("empty prompt body"),
        "a step with neither body nor executable has nothing to run; stderr:\n{stderr}"
    );
}

// ============================================================================
// The two meanings of `prompt`
// ============================================================================

/// Document-level `prompt:` is prose that flips the sequence to inline-compose;
/// step/task-level `prompt:` is a file reference. The words collide but the
/// levels never do, and this pins both halves in one fixture.
#[test]
fn document_level_prompt_is_prose_while_task_level_prompt_is_a_file_reference() {
    let workspace = tempdir().unwrap();

    let document_level = write(
        &workspace,
        "doc-level.md",
        "---\nprompt: \"Step {{ state.name }}.\"\nsequence:\n  - alpha\n---\nOriginal body\n",
    );
    let (stdout, _) = dry_run(&workspace, &document_level, &[], true);
    assert!(
        stdout.contains("Step alpha."),
        "the composed `prompt` is what runs; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Original body"),
        "the body is replaced, not sent; stdout:\n{stdout}"
    );

    write(&workspace, "referenced.md", "---\ntitle: r\n---\nStep {{ state.name }}.\n");
    let task_level = write(
        &workspace,
        "task-level.md",
        "---\nsequence:\n  - name: alpha\n    prompt: referenced.md\n---\nUnused body\n",
    );
    let (stdout, _) = dry_run(&workspace, &task_level, &[], true);
    assert!(
        stdout.contains("Step alpha."),
        "the referenced document composes in place of the body; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Unused body"),
        "a step with an executable does not run the source body; stdout:\n{stdout}"
    );
}

#[test]
fn a_dry_run_leaves_an_inline_compose_source_untouched() {
    let workspace = tempdir().unwrap();
    let original =
        "---\nprompt: \"Step {{ state.name }}.\"\nsequence:\n  - alpha\n---\nOriginal body\n";
    let doc = write(&workspace, "inline.md", original);

    dry_run(&workspace, &doc, &[], true);
    assert_eq!(
        fs::read_to_string(&doc).unwrap(),
        original,
        "dry-run performs no write-back"
    );
}

// ============================================================================
// External task, group, and catalog references
// ============================================================================

/// The three group definition sites converge during preflight, so execution
/// never learns which one a group came from. Externalized tasks resolve through
/// the same loader.
#[test]
fn external_task_group_and_catalog_references_all_load_and_run() {
    let workspace = tempdir().unwrap();
    write(&workspace, "p.md", "---\ntitle: p\n---\nStep {{ state.name }}.\n");
    write(&workspace, "task.yaml", "kind: task\nprompt: p.md\n");
    write(&workspace, "group.yaml", "kind: group\nname: G\ntasks:\n  - prompt: p.md\n");
    write(
        &workspace,
        "catalog.yaml",
        "kind: group-catalog\ngroups:\n  - name: build\n    tasks:\n      - prompt: p.md\n",
    );

    let doc = write(
        &workspace,
        "refs.md",
        "---\nsequence:\n  - name: a\n    task: task.yaml\n  - name: b\n    group: group.yaml\n  - name: c\n    group: build@catalog.yaml\n---\nUnused body\n",
    );

    assert_eq!(
        composed_steps(&workspace, &doc, &[]),
        ["a", "b", "c"],
        "each reference form should compose the same prompt document"
    );
}

#[test]
fn a_reference_inside_an_external_task_resolves_from_that_tasks_directory() {
    let workspace = tempdir().unwrap();
    // The prompt sits beside the *task* file, not beside the sequence. Only
    // resolving from the task's own directory finds it.
    write(&workspace, "tasks/p.md", "---\ntitle: p\n---\nStep {{ state.name }}.\n");
    write(&workspace, "tasks/task.yaml", "kind: task\nprompt: p.md\n");
    let doc = write(
        &workspace,
        "refs.md",
        "---\nsequence:\n  - name: alpha\n    task: tasks/task.yaml\n---\nUnused body\n",
    );

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["alpha"]);
}
