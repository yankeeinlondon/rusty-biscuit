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
//! Not duplicated here: `@` magic and `&`/`^` repository references have a
//! repo-aware home in `sequence_magic_reference.rs`; typed rejections live in
//! `sequence_errors_cli.rs`.

use std::fs;
use std::path::Path;
use crate::common;
use common::{CliProcessFixture, strip_ansi};

/// Dry-run `file` and return the composed step bodies, in order.
///
/// A step body is emitted on stdout; status framing goes to stderr. Reading
/// only stdout is therefore both the assertion and a standing check that the
/// stream split holds.
fn composed_steps(workspace: &CliProcessFixture, file: &Path, extra_args: &[&str]) -> Vec<String> {
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
    workspace: &CliProcessFixture,
    file: &Path,
    extra_args: &[&str],
    expect_success: bool,
) -> (String, String) {
    dry_run_command(workspace.command(), file, extra_args, expect_success)
}

/// [`dry_run`] over a command the caller has already shaped, for the one
/// case whose subject needs something other than the fixture's default PATH.
fn dry_run_command(
    mut command: assert_cmd::Command,
    file: &Path,
    extra_args: &[&str],
    expect_success: bool,
) -> (String, String) {
    let mut args: Vec<&str> = vec!["sequence", "--dry-run"];
    args.extend_from_slice(extra_args);
    let file_arg = file.to_str().unwrap();
    args.push(file_arg);

    let assert = command.args(&args).assert();
    let assert = if expect_success { assert.success() } else { assert.failure() };
    let output = assert.get_output().clone();

    (
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

fn write(workspace: &CliProcessFixture, name: &str, body: &str) -> std::path::PathBuf {
    let path = workspace.cwd().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, body).unwrap();
    path
}

/// A Markdown document whose body renders each step's name on its own line.
fn source_doc(workspace: &CliProcessFixture, name: &str, source: &str) -> std::path::PathBuf {
    write(
        workspace,
        name,
        &format!("---\nsequence: {source}\n---\nStep {{{{ seq.name }}}}.\n"),
    )
}

// ============================================================================
// Data-file formats
// ============================================================================

/// The five supported data formats are one code path above the loader, so a
/// format that parses must produce the same plan as any other. Every row
/// asserts that one shared plan rather than an expectation of its own, which
/// is what makes a future format-specific divergence visible.
mod yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan {
    use super::*;

    /// Each format's document name and its `sequence:` source. JSONL/NDJSON
    /// are always a list at the root, so they take an operator but never an
    /// offset.
    const ROWS: [(&str, &str); 5] = [
        ("yaml.md", "\"d.yaml -> wrap.items\""),
        ("json.md", "\"d.json -> wrap.items\""),
        ("json5.md", "\"d.json5 -> wrap.items\""),
        ("jsonl.md", "\"d.jsonl::name(label)\""),
        ("ndjson.md", "\"d.ndjson::name(label)\""),
    ];

    macro_rules! format_tests {
        ($($test:ident => $name:literal),* $(,)?) => {
            $(#[test] fn $test() { assert_row_produces_the_shared_plan($name); })*

            #[test]
            fn every_row_has_a_listed_test() {
                let rows: Vec<&str> = ROWS.iter().map(|(name, _)| *name).collect();
                assert_eq!(rows, [$($name),*]);
            }
        };
    }

    format_tests! {
        yaml => "yaml.md",
        json => "json.md",
        json5 => "json5.md",
        jsonl => "jsonl.md",
        ndjson => "ndjson.md",
    }

    /// Every source format the loader classifies has a row, and both
    /// line-delimited extensions keep theirs.
    #[test]
    fn every_source_format_has_a_row() {
        use claudine::composition::sequence::data::SourceFormat;

        let data_files: Vec<String> =
            ROWS.iter().map(|(name, _)| format!("d.{}", name.trim_end_matches(".md"))).collect();
        // Exhaustive on purpose: a new format fails to compile here until it
        // has a row above.
        let all = [
            SourceFormat::Yaml,
            SourceFormat::Json,
            SourceFormat::Json5,
            SourceFormat::LineDelimited,
        ];
        for format in all {
            match format {
                SourceFormat::Yaml | SourceFormat::Json | SourceFormat::Json5 | SourceFormat::LineDelimited => {}
            }
            assert!(
                data_files.iter().any(|file| SourceFormat::for_path(Path::new(file)) == format),
                "{format:?} has no same-plan row"
            );
        }
        assert_eq!(data_files, ["d.yaml", "d.json", "d.json5", "d.jsonl", "d.ndjson"]);
    }

    fn assert_row_produces_the_shared_plan(name: &str) {
        let (_, source) = ROWS.iter().find(|(row, _)| *row == name).expect("a listed row");
        let workspace = CliProcessFixture::named("sequence-sources-cli");
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

        let doc = source_doc(&workspace, name, source);
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            ["one", "two"],
            "`{source}` should normalize to the same plan as every other format"
        );
    }
}

/// `map` renames (the source key is gone), `name` copies (it is retained),
/// and `template` computes. All three answer the same produce-a-name
/// question, which is why v1 allows exactly one per reference.
mod the_three_operators_each_produce_a_usable_name {
    use super::*;

    fn operator_steps(name: &str, source: &str) -> Vec<String> {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(
            &workspace,
            "d.yaml",
            "colors:\n  data:\n    - color: blue\n      rank: 5\n    - color: green\n      rank: 3\n",
        );
        let doc = source_doc(&workspace, name, source);
        composed_steps(&workspace, &doc, &[])
    }

    #[test]
    fn map() {
        assert_eq!(
            operator_steps("map.md", "\"d.yaml -> colors.data::map(color, name)\""),
            ["blue", "green"]
        );
    }

    #[test]
    fn name() {
        assert_eq!(
            operator_steps("name.md", "\"d.yaml -> colors.data::name(color)\""),
            ["blue", "green"]
        );
    }

    #[test]
    fn template() {
        assert_eq!(
            operator_steps("template.md", "\"d.yaml -> colors.data::template(color + '-' + rank)\""),
            ["blue-5", "green-3"]
        );
    }
}

#[test]
fn map_removes_the_source_key_while_name_retains_it() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(&workspace, "d.yaml", "items:\n  - color: blue\n");

    // The distinction is invisible in the step name and only observable in the
    // surviving state, which is exactly why it is asserted on the state.
    let mapped = write(
        &workspace,
        "map.md",
        "---\nsequence: \"d.yaml -> items::map(color, name)\"\n---\nStep [{{ seq.color }}].\n",
    );
    assert_eq!(composed_steps(&workspace, &mapped, &[]), ["[]"], "map renames");

    let named = write(
        &workspace,
        "name.md",
        "---\nsequence: \"d.yaml -> items::name(color)\"\n---\nStep [{{ seq.color }}].\n",
    );
    assert_eq!(composed_steps(&workspace, &named, &[]), ["[blue]"], "name copies");
}

// ============================================================================
// `ListFormat` classification, through the shipped classifier
// ============================================================================

/// Every classified string form reaches the sequence through the same
/// `biscuit_file::ListFormat` classifier. Driving them through the CLI proves
/// the wiring, not just the classifier's own unit tests.
mod every_string_list_form_classifies_through_the_real_invocation_path {
    use super::*;

    /// Each form's name, its raw frontmatter value, and the steps it must
    /// classify into.
    const ROWS: [(&str, &str, &[&str]); 5] = [
        ("csv", "\"alpha, beta, gamma\"", &["alpha", "beta", "gamma"]),
        ("tsv", "\"alpha\tbeta\"", &["alpha", "beta"]),
        // YAML's `\n` escape: a raw newline inside a double-quoted scalar
        // folds to a space, which would classify as space-separated instead.
        ("lines", "\"alpha\\nbeta\"", &["alpha", "beta"]),
        ("spaces", "\"alpha beta gamma\"", &["alpha", "beta", "gamma"]),
        ("scalar", "\"solo\"", &["solo"]),
    ];

    macro_rules! form_tests {
        ($($test:ident => $name:literal),* $(,)?) => {
            $(#[test] fn $test() { assert_form_classifies($name); })*

            #[test]
            fn every_row_has_a_listed_test() {
                let rows: Vec<&str> = ROWS.iter().map(|(name, _, _)| *name).collect();
                assert_eq!(rows, [$($name),*]);
            }
        };
    }

    form_tests! {
        csv => "csv",
        tsv => "tsv",
        lines => "lines",
        spaces => "spaces",
        scalar => "scalar",
    }

    /// Each row classifies as the format it names once YAML has read it, and
    /// every `ListFormat` has a row here or in
    /// `markdown_list_markers_win_over_line_splitting`.
    #[test]
    fn every_list_format_has_a_row() {
        use biscuit_file::{ListFormat, Yaml};

        let intended = |name: &str| match name {
            "csv" => ListFormat::Csv,
            "tsv" => ListFormat::Tsv,
            "lines" => ListFormat::LineSeparated,
            "spaces" => ListFormat::SpaceSeparated,
            "scalar" => ListFormat::Scalar,
            _ => panic!("row {name} names no format"),
        };
        for (name, raw, _) in ROWS {
            let yaml = Yaml::from_str(format!("raw: {raw}")).unwrap();
            let value = yaml.value().get("raw").and_then(|v| v.as_str()).unwrap().to_string();
            assert_eq!(ListFormat::classify(&value), intended(name), "row {name} reads as {value:?}");
        }

        // Exhaustive on purpose: a new format fails to compile here until it
        // has a test.
        let all = [
            ListFormat::MarkdownOrdered,
            ListFormat::MarkdownUnordered,
            ListFormat::LineSeparated,
            ListFormat::Tsv,
            ListFormat::Csv,
            ListFormat::SpaceSeparated,
            ListFormat::Scalar,
        ];
        for format in all {
            let covered = match format {
                ListFormat::MarkdownOrdered | ListFormat::MarkdownUnordered => true,
                ListFormat::LineSeparated
                | ListFormat::Tsv
                | ListFormat::Csv
                | ListFormat::SpaceSeparated
                | ListFormat::Scalar => ROWS.iter().any(|(name, _, _)| intended(name) == format),
            };
            assert!(covered, "{format:?} has no string-form row");
        }
    }

    fn assert_form_classifies(name: &str) {
        let (_, raw, expected) = ROWS.iter().find(|(row, _, _)| *row == name).expect("a listed row");
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let doc = write(
            &workspace,
            &format!("{name}.md"),
            &format!("---\nraw: {raw}\nsequence: \"{{{{ raw }}}}\"\n---\nStep {{{{ seq.name }}}}.\n"),
        );
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            *expected,
            "`{name}` did not classify as expected"
        );
    }
}

// Both bodies are multi-line, so line-separation would also "work" — the
// marker precedence rule is what strips the bullets and numbers.
mod markdown_list_markers_win_over_line_splitting {
    use super::*;

    #[test]
    fn unordered() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let unordered = write(
            &workspace,
            "ul.md",
            "---\nraw: |\n  - alpha\n  - beta\nsequence: \"{{ raw }}\"\n---\nStep {{ seq.name }}.\n",
        );
        assert_eq!(composed_steps(&workspace, &unordered, &[]), ["alpha", "beta"]);
    }

    #[test]
    fn ordered() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let ordered = write(
            &workspace,
            "ol.md",
            "---\nraw: |\n  1. alpha\n  2. beta\nsequence: \"{{ raw }}\"\n---\nStep {{ seq.name }}.\n",
        );
        assert_eq!(composed_steps(&workspace, &ordered, &[]), ["alpha", "beta"]);
    }
}

mod quoted_csv_delimiters_and_crlf_survive_classification {
    use super::*;

    /// A naive `split(',')` would produce four entries here.
    #[test]
    fn quoted_delimiters() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let doc = write(
            &workspace,
            "quoted.md",
            "---\nraw: \"\\\"alpha, with comma\\\", beta\"\nsequence: \"{{ raw }}\"\n---\nStep {{ seq.name }}.\n",
        );
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            ["alpha, with comma", "beta"]
        );
    }

    /// A naive line split would leave a trailing `\r` on every name.
    #[test]
    fn crlf() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let crlf = write(
            &workspace,
            "crlf.md",
            "---\nraw: \"alpha\\r\\nbeta\"\nsequence: \"{{ raw }}\"\n---\nStep [{{ seq.name }}].\n",
        );
        assert_eq!(composed_steps(&workspace, &crlf, &[]), ["[alpha]", "[beta]"]);
    }
}

// ============================================================================
// Strictness by provenance
// ============================================================================

/// Foreign data is lenient where an authored inline list is strict: scalars
/// coerce and a nameless object gets an ordinal name. `sequence_errors_cli.rs`
/// pins the strict half of the same boundary.
mod a_foreign_source_coerces_scalars_and_names_nameless_objects {
    use super::*;

    #[test]
    fn scalars_coerce() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let scalars = write(
            &workspace,
            "scalars.md",
            "---\nraw:\n  - 1\n  - 2.5\n  - true\nsequence: \"{{ raw }}\"\n---\nStep {{ seq.name }}.\n",
        );
        assert_eq!(composed_steps(&workspace, &scalars, &[]), ["1", "2.5", "true"]);
    }

    #[test]
    fn nameless_objects_take_their_ordinal() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let objects = write(
            &workspace,
            "objects.md",
            "---\nraw:\n  - name: alpha\n  - topic: parsing\nsequence: \"{{ raw }}\"\n---\nStep {{ seq.name }}.\n",
        );
        assert_eq!(
            composed_steps(&workspace, &objects, &[]),
            ["alpha", "2"],
            "a nameless object takes its one-based ordinal"
        );
    }
}

#[test]
fn a_shell_expanded_source_becomes_a_classified_list() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    // The shell source is executed as a program, and `echo` is one of the few
    // with the same surface everywhere — but on Windows it exists only as
    // Git's echo.exe outside System32, so this case declares the host PATH
    // rather than a platform gate. `--yolo` stands in for the approval the
    // preflight would otherwise ask for.
    let doc = source_doc(&workspace, "shell.md", "\"$(echo alpha,beta)\"");
    let (stdout, _) = dry_run_command(
        workspace.command_builder().host_path().build(),
        &doc,
        &["--yolo"],
        true,
    );
    let steps: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("Step "))
        .map(|line| line.trim_start_matches("Step ").trim_end_matches('.').to_string())
        .collect();
    assert_eq!(steps, ["alpha", "beta"]);
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
mod file_references_resolve_from_the_authoring_document {
    use super::*;

    /// Each family's document name, its `sequence:` source, and the one step
    /// its data file holds.
    const ROWS: [(&str, &str, &str); 3] = [
        ("relative.md", "\"nested/data.yaml -> items\"", "nested-hit"),
        ("spaced.md", "\"my data/data.yaml -> items\"", "spaced-hit"),
        ("at.md", "\"a@b/data.yaml -> items\"", "at-hit"),
    ];

    macro_rules! family_tests {
        ($($test:ident => $name:literal),* $(,)?) => {
            $(#[test] fn $test() { assert_family_resolves($name); })*

            #[test]
            fn every_row_has_a_listed_test() {
                let rows: Vec<&str> = ROWS.iter().map(|(name, _, _)| *name).collect();
                assert_eq!(rows, [$($name),*]);
            }
        };
    }

    family_tests! {
        relative => "relative.md",
        path_with_a_space => "spaced.md",
        path_with_an_at_sign => "at.md",
    }

    fn workspace() -> CliProcessFixture {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "nested/data.yaml", "items:\n  - nested-hit\n");
        write(&workspace, "my data/data.yaml", "items:\n  - spaced-hit\n");
        write(&workspace, "a@b/data.yaml", "items:\n  - at-hit\n");
        write(&workspace, "tilde.yaml", "items:\n  - tilde-hit\n");
        workspace
    }

    fn assert_family_resolves(name: &str) {
        let (_, source, expected) = ROWS.iter().find(|(row, _, _)| *row == name).expect("a listed row");
        let workspace = workspace();
        let doc = source_doc(&workspace, name, source);
        assert_eq!(
            composed_steps(&workspace, &doc, &[]),
            [*expected],
            "`{source}` did not resolve"
        );
    }

    #[test]
    fn home_pinned() {
        let workspace = workspace();
        let doc = source_doc(&workspace, "tilde.md", "\"~/tilde.yaml -> items\"");
        let request_context = biscuit_file::FileResolutionContext::new(workspace.cwd())
            .with_home_dir(workspace.cwd())
            .with_source_path(&doc);
        let source = claudine::composition::resolve_composition_source_in_context(
            doc.to_str().unwrap(),
            &request_context,
        )
        .unwrap();
        let plan = claudine::composition::resolve_sequence_plan_with(
            &source,
            claudine::composition::SequenceSourceOptions::new(&request_context),
        )
        .unwrap()
        .expect("tilde fixture declares a sequence");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].name, "tilde-hit");
    }
}

#[test]
fn a_reference_resolves_relative_to_its_own_document_not_the_process_cwd() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(&workspace, "deep/data.yaml", "items:\n  - deep-hit\n");
    // A sibling reference, authored one directory down. Resolving from the
    // process CWD (the workspace root) would miss it entirely.
    let doc = source_doc(&workspace, "deep/seq.md", "\"data.yaml -> items\"");

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["deep-hit"]);
}

#[test]
fn a_quoted_operator_argument_keeps_its_delimiters() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(
        &workspace,
        "formal.yaml",
        "kind: sequence\nsequence:\n  - name: blue\n    color: blue\n    rank: 5\ntemplate:\n  desc: \"{{ color }}({{ rank }})\"\n",
    );
    let doc = write(
        &workspace,
        "ref.md",
        "---\nsequence: formal.yaml\n---\nStep {{ seq.desc }}/{{ seq.index }}.\n",
    );

    // A templated key is ordinary authored state by the time `index` is
    // generated, so both are readable from the same state object.
    assert_eq!(composed_steps(&workspace, &doc, &[]), ["blue(5)/1"]);
}

/// A referenced formal document's root `$state` types its step state: a
/// conforming step composes the referring document's body with the typed
/// value, and one edit to the value is a located step-state violation.
#[test]
fn a_referenced_formal_documents_state_declaration_validates_the_step_state() {
    const FORMAL: &str = "kind: sequence\nsequence:\n  - name: blue\n    rank: 5\n$state:\n  rank: number(required)\n";
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(&workspace, "formal.yaml", FORMAL);
    let doc = write(&workspace, "ref.md", "---\nsequence: formal.yaml\n---\nStep {{ seq.name }}/{{ seq.rank }}.\n");
    assert_eq!(composed_steps(&workspace, &doc, &[]), ["blue/5"]);

    write(&workspace, "formal.yaml", &FORMAL.replace("rank: 5", "rank: not-a-number"));
    let (stdout, stderr) = dry_run(&workspace, &doc, &[], false);
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("sequence step 0 (`blue`) failed schema validation at `/rank`"),
        "the violation must name the step index, its id, and the failing property; stderr:\n{flat}"
    );
    assert!(!stdout.contains("Step blue"), "no step composed: {stdout}");
}

/// The whole formal-document contract in one fixture, reached both ways.
///
/// The file is written once and invoked twice — directly, and through a
/// Markdown document that references it — because the spec's "both entry modes
/// accept the identical document shape" is only meaningful if the *same bytes*
/// normalize the same way. It exercises scalar shorthand (`- blue`), an
/// interpolated template value, a typed literal template default, a template
/// default an item overrides, and generated fields, all of which the root
/// `$state` must then accept as step state.
///
/// The two entry modes differ in exactly one respect: a directly invoked YAML
/// has no Markdown body, so its steps render through a document-level `prompt`
/// the referencing document supplies as its body instead. Both strings are
/// therefore identical, and so are both results.
fn write_parity_fixture(workspace: &CliProcessFixture) -> (std::path::PathBuf, std::path::PathBuf) {
    const STEP_BODY: &str = "Step {{ seq.name }}/{{ seq.desc }}/{{ seq.rank }}.";

    let direct = write(
        workspace,
        "formal.yaml",
        &format!(
            "kind: sequence\nprompt: \"{STEP_BODY}\"\n\
             sequence:\n  - blue\n  - name: red\n    rank: 3\n\
             template:\n  desc: \"{{{{ name }}}}!\"\n  rank: 5\n\
             $state:\n  desc: string(required)\n  rank: number(required)\n"
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
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
/// [`a_referenced_formal_documents_state_declaration_validates_the_step_state`]: a
/// directly invoked document's root `$state` types the step state the same
/// way: a conforming step composes the document's `prompt` with the typed
/// value. Its root `$schema` would describe the document instead.
#[test]
fn a_directly_invoked_documents_state_declaration_validates_the_step_state() {
    const FORMAL: &str = "kind: sequence\nprompt: \"Step {{ seq.name }}/{{ seq.rank }}.\"\nsequence:\n  - name: blue\n    rank: 5\n$state:\n  rank: number(required)\n";
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    let doc = write(&workspace, "formal.yaml", FORMAL);
    assert_eq!(composed_steps(&workspace, &doc, &[]), ["blue/5"]);

    write(&workspace, "formal.yaml", &FORMAL.replace("rank: 5", "rank: not-a-number"));
    let (stdout, stderr) = dry_run(&workspace, &doc, &[], false);
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("sequence step 0 (`blue`) failed schema validation at `/rank`"),
        "the direct mode must report the same step-state violation the referenced mode does; \
         stderr:\n{flat}"
    );
    assert!(!stdout.contains("Step blue"), "no step composed: {stdout}");
}

/// A setter supplies a value, never the sequence's plan: a `template=`
/// setter (before or after the file) or a `--set` key naming `$state` or
/// `template`, or a path under one, is refused before any step runs. (A
/// setter name holds no `.` or `$`, so only `--set` can spell those.)
#[test]
fn a_setter_cannot_write_a_sequence_declaration() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    let doc = write(
        &workspace,
        "formal.yaml",
        "kind: sequence\nprompt: \"Handle {{ seq.name }}\"\ntemplate:\n  desc: x\nsequence:\n  - blue\n",
    );
    let file = doc.to_str().unwrap();
    for args in [
        vec!["sequence", "--dry-run", file, "template={}"],
        vec!["sequence", "--dry-run", "template=y", file],
        vec!["sequence", "--dry-run", "--set", "{\"$state\": {}}", file],
        vec!["sequence", "--dry-run", file, "--set", "{\"template.desc\": \"y\"}"],
    ] {
        let output = workspace.command().args(&args).assert().failure().get_output().clone();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains("is not accepted here"), "{args:?}: {flat}");
        assert!(flat.contains("a setter supplies values, not declarations"), "{args:?}: {flat}");
        assert!(!flat.contains("Handle blue"), "{args:?}: no step composed: {flat}");
    }
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(&workspace, "p.md", "---\ntitle: p\n---\nStep {{ seq.name }}.\n");
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    let doc = write(
        &workspace,
        "headless.yaml",
        "kind: sequence\ntopic: from-document\nprompt: \"Step {{ seq.name }}/{{ topic }}.\"\nsequence:\n  - alpha\n  - beta\n",
    );

    assert_eq!(
        composed_steps(&workspace, &doc, &[]),
        ["alpha/from-document", "beta/from-document"]
    );
}

#[test]
fn a_bodyless_step_that_declares_no_executable_is_still_rejected() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");

    let document_level = write(
        &workspace,
        "doc-level.md",
        "---\nprompt: \"Step {{ seq.name }}.\"\nsequence:\n  - alpha\n---\nOriginal body\n",
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

    write(&workspace, "referenced.md", "---\ntitle: r\n---\nStep {{ seq.name }}.\n");
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    let original =
        "---\nprompt: \"Step {{ seq.name }}.\"\nsequence:\n  - alpha\n---\nOriginal body\n";
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    write(&workspace, "p.md", "---\ntitle: p\n---\nStep {{ seq.name }}.\n");
    write(&workspace, "task.yaml", "kind: task\nprompt: p.md\n");
    write(&workspace, "group.yaml", "kind: group\nname: G\ntasks:\n  - name: render\n    prompt: p.md\n");
    write(
        &workspace,
        "catalog.yaml",
        "kind: group-catalog\ngroups:\n  - name: build\n    tasks:\n      - name: render\n        prompt: p.md\n",
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
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    // The prompt sits beside the *task* file, not beside the sequence. Only
    // resolving from the task's own directory finds it.
    write(&workspace, "tasks/p.md", "---\ntitle: p\n---\nStep {{ seq.name }}.\n");
    write(&workspace, "tasks/task.yaml", "kind: task\nprompt: p.md\n");
    let doc = write(
        &workspace,
        "refs.md",
        "---\nsequence:\n  - name: alpha\n    task: tasks/task.yaml\n---\nUnused body\n",
    );

    assert_eq!(composed_steps(&workspace, &doc, &[]), ["alpha"]);
}

/// A sequence document in `sub/` resolves the four executable references of
/// its rows beside itself, however it is reached: invoked directly, named by
/// a root `sequence:`, or nested as a task. Nothing it references exists in
/// the launch folder, so a wrong base fails the run.
#[test]
fn a_sequence_documents_relative_references_resolve_beside_it_on_every_route() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    for (name, body) in [("p.md", "prompt"), ("tp.md", "task"), ("sp.md", "sequence"), ("gp.md", "group")] {
        write(&workspace, &format!("sub/{name}"), &format!("---\ntitle: {body}\n---\nStep {body}.\n"));
    }
    write(&workspace, "sub/t.yaml", "kind: task\nprompt: ./tp.md\n");
    write(&workspace, "sub/s.yaml", "kind: sequence\nsequence:\n  - name: inner\n    prompt: ./sp.md\n");
    write(&workspace, "sub/g.yaml", "kind: group\nname: G\ntasks:\n  - name: leg\n    prompt: ./gp.md\n");
    let child = write(
        &workspace,
        "sub/child.yaml",
        "kind: sequence\nsequence:\n  - name: a\n    prompt: ./p.md\n  - name: b\n    task: ./t.yaml\n  - name: c\n    sequence: ./s.yaml\n  - name: d\n    group: ./g.yaml\n",
    );
    let root = write(&workspace, "root.yaml", "kind: sequence\nsequence: sub/child.yaml\n");
    let nested = write(&workspace, "nested.yaml", "kind: sequence\nsequence:\n  - name: outer\n    sequence: sub/child.yaml\n");

    for document in [&child, &root, &nested] {
        let (stdout, stderr) = dry_run(&workspace, document, &["--yolo"], true);
        let steps: Vec<&str> = stdout.lines().map(str::trim).filter(|line| line.starts_with("Step ")).collect();
        assert_eq!(
            steps,
            ["Step prompt.", "Step task.", "Step sequence.", "Step group."],
            "{}: stdout:\n{stdout}\nstderr:\n{stderr}",
            document.display(),
        );
    }
    assert!(!workspace.audio_spool().exists());
}

/// A nested sequence document rereads its rows when it starts, and resolves
/// their references as the launch's walk did even when the sequence is named
/// through a symlinked folder, whose spelling differs from the canonical
/// one every referenced file takes.
#[cfg(unix)] // unix symlink API
#[test]
fn a_nested_document_reached_through_a_symlinked_folder_starts_and_runs_its_rows() {
    let workspace = CliProcessFixture::named("sequence-sources-cli");
    // A repository bounds resolution, so a spelling outside its root fails.
    workspace.initialize_repository();
    write(&workspace, "real/sub/t.yaml", "kind: task\nshell: echo ran-task\n");
    write(&workspace, "real/sub/child.yaml", "kind: sequence\nsequence:\n  - name: a\n    task: ./t.yaml\n");
    write(&workspace, "real/nested.yaml", "kind: sequence\nsequence:\n  - name: outer\n    sequence: sub/child.yaml\n");
    let link = workspace.cwd().join("link");
    std::os::unix::fs::symlink(workspace.cwd().join("real"), &link).unwrap();

    let output = workspace
        .command()
        .args(["sequence", "--yolo", link.join("nested.yaml").to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("ran-task"), "stdout:\n{stdout}\nstderr:\n{stderr}");
}

#[test]
fn shipped_runner_sequence_renders_lexical_rows_through_the_cli() {
    let fixture = CliProcessFixture::named("shipped-sequence-bindings");
    write(&fixture, "runners.yaml", include_str!("../../../docs/local-runners.yaml"));
    let source = write(&fixture, "seq.md", "---\nsequence: runners.yaml\n---\nStep {{ seq.index }}:{{ seq.name }}:{{ seq.run_id }}.\n");
    let rows = composed_steps(&fixture, &source, &[]);
    assert_eq!(rows.len(), 5, "{rows:?}");
    assert!(rows[0].starts_with("1:Ollama:"), "{rows:?}");
    assert!(rows[1].starts_with("2:LM Studio:"), "{rows:?}");
    let run_ids: std::collections::BTreeSet<_> = rows.iter().map(|row| row.rsplit(':').next().unwrap()).collect();
    assert_eq!(run_ids.len(), 1);
    assert_eq!(run_ids.first().unwrap().len(), 16);
    assert!(!fixture.audio_spool().exists());
}

// ============================================================================
// Document inputs and inline checks at the invocation boundary
// ============================================================================

/// Every owning document's required inputs join the one collection pass
/// before the graph is frozen, and an inline container's `$schema` and a
/// `$state` never do. Detailed shapes live in the library
/// (`preflight::inputs::tests`, `task::tests::inline_check`); these prove the
/// CLI wiring.
mod document_inputs {
    use super::*;

    fn flat(text: &str) -> String {
        text.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn run(workspace: &CliProcessFixture, args: &[&str], expect_success: bool) -> String {
        let assert = workspace.command().args(args).assert();
        let assert = if expect_success { assert.success() } else { assert.failure() };
        let output = assert.get_output().clone();
        flat(&strip_ansi(&String::from_utf8_lossy(&output.stderr)))
    }

    const CHILD: &str = "kind: sequence\n$schema:\n  reviewer: string(required) -> who signs off\nsequence:\n  - name: sign\n    shell: echo signed by {{ reviewer }}\n";

    #[test]
    fn a_nested_referenced_documents_missing_input_is_reported_as_that_document() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "nested/child.yaml", CHILD);
        let root = write(&workspace, "flow.yaml", "kind: sequence\nsequence:\n  - name: review\n    sequence: ./nested/child.yaml\n");

        let stderr = run(&workspace, &["sequence", "--dry-run", root.to_str().unwrap()], false);
        assert!(stderr.contains("missing properties"), "{stderr}");
        assert!(stderr.contains("Document") && stderr.contains("child.yaml"), "{stderr}");
        assert!(stderr.contains("sequence[0].sequence"), "the container path names the reference: {stderr}");
        assert!(stderr.contains("reviewer"), "{stderr}");
        assert!(!stderr.contains("Step 1"), "a document input is not a step's: {stderr}");
    }

    /// A schema-aware setter belongs to the sequence wherever it sits.
    #[test]
    fn a_setter_before_or_after_the_file_supplies_a_nested_documents_input() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "nested/child.yaml", CHILD);
        let root = write(&workspace, "flow.yaml", "kind: sequence\nsequence:\n  - name: review\n    sequence: ./nested/child.yaml\n");
        let file = root.to_str().unwrap();
        for args in [
            vec!["sequence", "--dry-run", "--yolo", file, "reviewer=ada"],
            vec!["sequence", "--dry-run", "--yolo", "reviewer=ada", file],
        ] {
            let stderr = run(&workspace, &args, true);
            assert!(stderr.contains("echo signed by ada"), "{args:?}: {stderr}");
        }
    }

    #[test]
    fn an_empty_root_list_still_requires_its_inputs() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let root = write(&workspace, "empty.yaml", "kind: sequence\n$schema:\n  reviewer: string(required)\nsequence: []\n");
        let file = root.to_str().unwrap();

        let stderr = run(&workspace, &["sequence", "--dry-run", file], false);
        assert!(stderr.contains("missing properties") && stderr.contains("Document"), "{stderr}");
        assert!(stderr.contains("reviewer"), "{stderr}");
        run(&workspace, &["sequence", "--dry-run", file, "reviewer=ada"], true);
    }

    #[test]
    fn an_empty_root_referenced_document_still_requires_its_inputs() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "lists/empty.yaml", "kind: sequence\n$schema:\n  reviewer: string(required)\nsequence: []\n");
        let root = write(&workspace, "flow.yaml", "kind: sequence\nsequence: ./lists/empty.yaml\n");

        let stderr = run(&workspace, &["sequence", "--dry-run", root.to_str().unwrap()], false);
        assert!(stderr.contains("missing properties") && stderr.contains("empty.yaml"), "{stderr}");
        assert!(stderr.contains("reviewer"), "{stderr}");
    }

    /// The report comes before the root starts: no event action or task runs.
    #[test]
    fn collecting_a_missing_input_starts_nothing() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "nested/child.yaml", CHILD);
        let root = write(
            &workspace,
            "flow.yaml",
            "kind: sequence\nstart:\n  - append_line: ['events.log', 'root-start']\nsequence:\n  - name: review\n    sequence: ./nested/child.yaml\n",
        );

        let stderr = run(&workspace, &["sequence", root.to_str().unwrap()], false);
        assert!(stderr.contains("missing properties"), "{stderr}");
        assert!(!workspace.cwd().join("events.log").exists(), "nothing started: {stderr}");
    }

    /// A required `$state` field types step data; it is never asked for.
    #[test]
    fn a_required_state_field_is_a_state_violation_not_a_missing_input() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let root = write(
            &workspace,
            "flow.yaml",
            "kind: sequence\n$state:\n  color: string(required)\nsequence:\n  - name: blue\n    shell: echo blue\n",
        );

        let stderr = run(&workspace, &["sequence", "--dry-run", root.to_str().unwrap()], false);
        assert!(stderr.contains("sequence step 0 (`blue`) failed schema validation at `color`"), "{stderr}");
        assert!(!stderr.contains("missing properties"), "{stderr}");
    }

    /// An inline container's `$schema` is checked at its turn: a preview
    /// names it, and a run refuses the container, never asking for input.
    #[test]
    fn an_inline_check_is_never_an_input_and_refuses_its_container_at_its_turn() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let root = write(
            &workspace,
            "flow.yaml",
            "kind: sequence\nsequence:\n  - name: review\n    $schema:\n      reviewer: string(required)\n    start:\n      - append_line: ['events.log', 'review-start']\n    sequence:\n      - name: sign\n        shell: echo signed\n",
        );
        let file = root.to_str().unwrap();

        let preview = run(&workspace, &["sequence", "--dry-run", "--yolo", file], true);
        assert!(preview.contains("checks its `$schema` at its turn"), "{preview}");

        let stderr = run(&workspace, &["sequence", "--yolo", file], false);
        assert!(stderr.contains("failed its `$schema` check at `reviewer`"), "{stderr}");
        assert!(!stderr.contains("missing properties"), "{stderr}");
        assert!(!workspace.cwd().join("events.log").exists(), "the refused container never started: {stderr}");
    }

    /// The plan is filled after the root's `initialize`, so a template reads
    /// a value `initialize` wrote, through the same `seq.run_id` its events
    /// saw.
    #[test]
    fn a_root_template_reads_what_initialize_wrote() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let root = write(
            &workspace,
            "flow.yaml",
            "kind: sequence\nwho: nobody\ninitialize:\n  - set:\n      who: ada\ntemplate:\n  desc: \"signed by {{ who }}\"\nsequence:\n  - name: sign\n    side_effect:\n      append_line: ['events.log', '{{ seq.desc }}']\n",
        );

        run(&workspace, &["sequence", "--yolo", root.to_str().unwrap()], true);
        let events = fs::read_to_string(workspace.cwd().join("events.log")).unwrap();
        assert_eq!(events.trim(), "signed by ada");
    }

    /// A malformed declaration fails before the root's `initialize` runs.
    #[test]
    fn a_malformed_declaration_fails_before_initialize() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        let root = write(
            &workspace,
            "flow.yaml",
            "kind: sequence\ninitialize:\n  - append_line: ['events.log', 'initialized']\nsequence:\n  - name: review\n    when: \"false\"\n    $state: 42\n    sequence:\n      - name: sign\n        shell: echo signed\n",
        );

        let stderr = run(&workspace, &["sequence", root.to_str().unwrap()], false);
        assert!(stderr.contains("$state"), "{stderr}");
        assert!(!workspace.cwd().join("events.log").exists(), "initialize never ran: {stderr}");
    }

    /// A referenced document's authored `file(eager)` value resolves from
    /// that document's folder, not the invoking document's.
    #[test]
    fn a_nested_documents_authored_file_value_resolves_from_its_own_folder() {
        let workspace = CliProcessFixture::named("sequence-sources-cli");
        write(&workspace, "nested/work/spec.md", "the nested spec text\n");
        write(&workspace, "nested/step.md", "---\n$schema:\n  spec: file(required;eager)\n---\n::file {{ spec }}\n");
        write(
            &workspace,
            "nested/child.yaml",
            "kind: sequence\n$schema:\n  spec: file(required;eager)\nspec: work/spec.md\nsequence:\n  - name: read\n    prompt: ./step.md\n    with:\n      spec: \"{{ absolute(spec) }}\"\n",
        );
        let root = write(&workspace, "caller/flow.yaml", "kind: sequence\nsequence:\n  - name: review\n    sequence: ../nested/child.yaml\n");
        assert!(!workspace.cwd().join("caller/work/spec.md").exists());

        let (stdout, stderr) = dry_run(&workspace, &root, &["--yolo"], true);
        assert!(stdout.contains("the nested spec text"), "stdout: {stdout}\nstderr: {stderr}");
    }
}

// ============================================================================
// The frozen `complete.yaml` reproduction
// ============================================================================

/// A YAML sequence's root `$schema` describes the document: the caller's
/// `spec` is a document input, coerced as an eager file from the caller's
/// directory, and the derived `plan` reaches the nested planning step.
///
/// The frozen files live under
/// `tests/fixtures/frozen_prompts/sequence-state-schema/`; each test includes
/// them by literal path, so no test reads the live `prompts/` tree. The
/// sequence and its prompt sit in `flow/`, the spec in `work/feature/`, and
/// the caller runs from the workspace root, so a value resolved from the
/// sequence's directory cannot pass. The provider is the compiled
/// `claudine-fake-goose` in marker mode, so these run on every OS.
#[cfg(feature = "test-fixtures")]
mod frozen_complete_reproduction {
    use super::*;

    /// The caller-relative spec path every setter supplies.
    const SPEC_ARG: &str = "spec=work/feature/spec.md";

    struct Reproduction {
        workspace: CliProcessFixture,
        marker: std::path::PathBuf,
    }

    impl Reproduction {
        /// Lays out the frozen files and installs the marker provider.
        fn new(complete: &str, plan: &str, spec: &str) -> Self {
            let workspace = CliProcessFixture::named("sequence-frozen-complete");
            write(&workspace, "flow/complete.yaml", complete);
            write(&workspace, "flow/plan.md", plan);
            write(&workspace, "work/feature/spec.md", spec);
            fs::copy(
                biscuit_test_harness::bin_exe!("claudine-fake-goose"),
                workspace.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
            )
            .expect("the fake provider is built with `test-fixtures`");
            let marker = workspace.cwd().join("provider-launched.txt");
            Self { workspace, marker }
        }

        /// Runs `claudine sequence --goose` with `args` from the workspace
        /// root, returning the output with ANSI stripped.
        fn run(&self, args: &[&str]) -> (std::process::ExitStatus, String, String) {
            let output = self
                .workspace
                .command()
                .env("PROVIDER_MARKER", &self.marker)
                .args(["sequence", "--goose"])
                .args(args)
                .output()
                .unwrap();
            assert!(!self.workspace.audio_spool().exists());
            (
                output.status,
                strip_ansi(&String::from_utf8_lossy(&output.stdout)),
                strip_ansi(&String::from_utf8_lossy(&output.stderr)),
            )
        }

        fn launches(&self) -> usize {
            fs::read_to_string(&self.marker).map_or(0, |log| log.lines().count())
        }
    }

    /// The bracketed value the composed `plan.md` rendered after `label`,
    /// with separators normalized so the assertion holds on Windows.
    fn rendered(stdout: &str, label: &str) -> String {
        let line = stdout
            .lines()
            .find_map(|line| line.trim().strip_prefix(label))
            .unwrap_or_else(|| panic!("`{label}` was not composed; stdout:\n{stdout}"));
        line.trim().trim_start_matches('[').trim_end_matches(']').replace('\\', "/")
    }

    /// The composed nested prompt received the coerced `spec` and the `plan`
    /// derived from it: both absolute and under the caller's `work/feature/`.
    fn assert_composed_from_the_caller_spec(stdout: &str, stderr: &str) {
        let plan = rendered(stdout, "Plan target:");
        let spec = rendered(stdout, "Spec source:");
        for (name, value, file) in [("plan", &plan, "plan.md"), ("spec", &spec, "spec.md")] {
            assert!(
                Path::new(value).is_absolute() && value.ends_with(&format!("/work/feature/{file}")),
                "`{name}` must resolve from the caller's directory, got `{value}`; stderr:\n{stderr}"
            );
            assert!(!value.contains("/flow/"), "`{name}` resolved from the sequence's directory: `{value}`");
        }
        // `plan` is derived from the coerced, absolute `spec`, never the raw setter.
        assert_eq!(plan.trim_end_matches("plan.md"), spec.trim_end_matches("spec.md"));
    }

    #[test]
    fn a_setter_after_the_file_is_judged_as_a_document_input_and_reaches_the_nested_step() {
        let repro = Reproduction::new(
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/complete.yaml"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/plan.md"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/spec.md"),
        );

        let (status, stdout, stderr) = repro.run(&["flow/complete.yaml", SPEC_ARG, "--dry-run"]);

        assert!(status.success(), "stderr:\n{stderr}");
        assert!(!stderr.contains("failed schema validation"), "the document schema judged a step: {stderr}");
        assert_composed_from_the_caller_spec(&stdout, &stderr);
        assert_eq!(repro.launches(), 0, "a dry run launches nothing; stderr:\n{stderr}");
    }

    /// The golden rule: a setter means the same before or after the file.
    #[test]
    fn a_setter_before_the_file_is_judged_the_same_way() {
        let repro = Reproduction::new(
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/complete.yaml"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/plan.md"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/spec.md"),
        );

        let (status, stdout, stderr) = repro.run(&[SPEC_ARG, "flow/complete.yaml", "--dry-run"]);

        assert!(status.success(), "stderr:\n{stderr}");
        assert_composed_from_the_caller_spec(&stdout, &stderr);
        assert_eq!(repro.launches(), 0, "stderr:\n{stderr}");
    }

    /// Without the setter a non-interactive run reports `spec` as a missing
    /// input of the sequence, composes nothing, and launches nothing.
    #[test]
    fn a_missing_spec_is_reported_as_a_missing_input_without_composing() {
        let repro = Reproduction::new(
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/complete.yaml"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/plan.md"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/spec.md"),
        );

        let (status, stdout, stderr) = repro.run(&["flow/complete.yaml", "--dry-run"]);

        assert!(!status.success(), "stdout:\n{stdout}");
        let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains("sequence missing properties"), "{flat}");
        assert!(flat.contains("`spec`: file — the specification to complete"), "{flat}");
        assert!(!flat.contains("failed schema validation"), "not a step-state verdict: {flat}");
        assert!(!stdout.contains("Plan target"), "no prompt was composed: {stdout}");
        assert_eq!(repro.launches(), 0, "{flat}");
    }

    /// Outside a dry run, the step's guard reads the derived `plan`: the step
    /// launches once while the plan is absent beside the spec, and is skipped
    /// once the plan exists there.
    #[test]
    fn the_step_guard_reads_the_plan_derived_from_the_coerced_spec() {
        let repro = Reproduction::new(
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/complete.yaml"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/plan.md"),
            include_str!("../fixtures/frozen_prompts/sequence-state-schema/spec.md"),
        );

        let (status, _, stderr) = repro.run(&["--silent", "flow/complete.yaml", SPEC_ARG]);
        assert!(status.success(), "stderr:\n{stderr}");
        assert_eq!(repro.launches(), 1, "the plan is absent, so the step runs; stderr:\n{stderr}");

        write(&repro.workspace, "work/feature/plan.md", "planned\n");
        let (status, _, stderr) = repro.run(&["--silent", "flow/complete.yaml", SPEC_ARG]);
        assert!(status.success(), "stderr:\n{stderr}");
        assert_eq!(repro.launches(), 1, "the plan exists beside the spec, so the step is skipped; stderr:\n{stderr}");
    }
}

// ============================================================================
// Declaration wiring through each CLI loader
// ============================================================================

/// `$state` and `template:` reach the declaration judge through both loaders
/// the CLI invokes directly: Markdown frontmatter and YAML. Each loader has a
/// dry-run control whose step composes with its template-filled, typed state,
/// and invalid edits that fail a real run with the file and declaration
/// named, launching nothing. Every declaration and
/// loader shape is the library's (`composition::sequence::tests`); these
/// prove the route only. The provider is the compiled `claudine-fake-goose`
/// in marker mode, so these run on every OS.
#[cfg(feature = "test-fixtures")]
mod declaration_routes {
    use super::*;

    const MARKDOWN: &str = "---\n$state:\n  rank: number(required)\ntemplate:\n  desc: \"{{ name }}-{{ rank }}\"\nsequence:\n  - { name: blue, rank: 5 }\n---\nStep {{ seq.desc }}.\n";
    const YAML: &str = "kind: sequence\nprompt: \"Step {{ seq.desc }}.\"\n$state:\n  rank: number(required)\ntemplate:\n  desc: \"{{ name }}-{{ rank }}\"\nsequence:\n  - { name: blue, rank: 5 }\n";

    /// Runs `file` (holding `contents`) as a sequence with `flags`, returning
    /// whether it succeeded, its stdout, its flattened stderr, and the
    /// provider launch count.
    fn launch(file: &str, contents: &str, flags: &[&str]) -> (bool, String, String, usize) {
        let workspace = CliProcessFixture::named("sequence-declaration-routes");
        let path = write(&workspace, file, contents);
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            workspace.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("the fake provider is built with `test-fixtures`");
        let marker = workspace.cwd().join("provider-launched.txt");
        let output = workspace.command().env("PROVIDER_MARKER", &marker)
            .args(["sequence", "--goose"]).args(flags).arg(&path).output().unwrap();
        assert!(!workspace.audio_spool().exists());
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
        (output.status.success(), strip_ansi(&String::from_utf8_lossy(&output.stdout)), flat,
         fs::read_to_string(&marker).map_or(0, |log| log.lines().count()))
    }

    macro_rules! route {
        ($name:ident, $file:literal, $contents:expr, composes) => {
            #[test]
            fn $name() {
                let (success, stdout, stderr, launches) = launch($file, &$contents, &["--dry-run"]);
                assert!(success, "{stderr}");
                assert!(stdout.lines().any(|line| line.trim() == "Step blue-5."), "stdout:\n{stdout}\nstderr: {stderr}");
                assert_eq!(launches, 0, "{stderr}");
            }
        };
        ($name:ident, $file:literal, $contents:expr, refuses: [$($needle:expr),+ $(,)?]) => {
            #[test]
            fn $name() {
                let (success, _, stderr, launches) = launch($file, &$contents, &[]);
                assert!(!success, "{stderr}");
                $(assert!(stderr.contains($needle), "expected `{}` in: {stderr}", $needle);)+
                assert_eq!(launches, 0, "{stderr}");
            }
        };
    }

    route!(a_markdown_sequences_declarations_compose_its_step, "flow.md", MARKDOWN, composes);
    route!(a_markdown_state_of_the_wrong_shape_is_refused_with_its_file, "flow.md",
        MARKDOWN.replace("$state:\n  rank: number(required)\n", "$state: 42\n"),
        refuses: ["invalid sequence declaration `$state`", "flow.md", "must be a schema mapping"]);
    route!(a_markdown_state_written_as_null_is_refused_not_ignored, "flow.md",
        MARKDOWN.replace("$state:\n  rank: number(required)\n", "$state:\n"),
        refuses: ["invalid sequence declaration `$state`", "flow.md", "must not be null"]);
    route!(a_yaml_sequences_declarations_compose_its_step, "flow.yaml", YAML, composes);
    route!(a_yaml_template_of_the_wrong_shape_is_refused_with_its_file, "flow.yaml",
        YAML.replace("template:\n  desc: \"{{ name }}-{{ rank }}\"\n", "template: [desc]\n"),
        refuses: ["invalid sequence declaration `template`", "flow.yaml", "must be an object, got array"]);
    route!(a_yaml_duplicate_declaration_key_is_refused_with_its_file, "flow.yaml",
        YAML.replace("sequence:\n", "$state:\n  rank: string\nsequence:\n"),
        refuses: ["flow.yaml", "duplicate entry with key \"$state\""]);
}

// ============================================================================
// Provider reads through derived values
// ============================================================================

/// A task's provider read through derived sequence values that a runtime
/// `set` feeds is decided at its turn, and a prompt's own property is never
/// another owner's of the same name. The writer and the derived reader use
/// the compiled `claudine-fake-goose` in marker mode; Claude is absent from
/// the fixture `PATH`, so any launch of it fails as not installed.
#[cfg(feature = "test-fixtures")]
mod derived_provider_reads {
    use super::*;

    const WRITES: &str = "---\nagent: goose\nsuccess: [{ set: { pick: goose } }]\n---\nWrite.\n";

    /// Runs `seq.yaml` (`root`) beside `files` with `--yolo`, returning
    /// whether it succeeded, its flattened stderr, and the provider launch
    /// count.
    fn run(root: &str, files: &[(&str, &str)]) -> (bool, String, usize) {
        let workspace = CliProcessFixture::named("sequence-derived-provider-reads");
        let path = write(&workspace, "seq.yaml", root);
        write(&workspace, "writes.md", WRITES);
        for (name, body) in files {
            write(&workspace, name, body);
        }
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            workspace.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("the fake provider is built with `test-fixtures`");
        let marker = workspace.cwd().join("provider-launched.txt");
        let output = workspace.command().env("PROVIDER_MARKER", &marker)
            .args(["sequence", "--yolo"]).arg(&path).output().unwrap();
        assert!(!workspace.audio_spool().exists());
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
        (output.status.success(), flat, fs::read_to_string(&marker).map_or(0, |log| log.lines().count()))
    }

    /// Two derived links from `with.agent` to the writer's `pick`: the reader
    /// waits for the write and launches Goose after the writer.
    #[test]
    fn a_provider_read_through_two_derived_values_waits_for_the_write() {
        let (success, stderr, launches) = run(
            "kind: sequence\npick: claude\nmid: \"{{ pick }}\"\nchoice: \"{{ mid }}\"\nsequence:\n  - { name: writer, prompt: writes.md }\n  - { name: reader, prompt: plain.md, with: { agent: \"{{ choice }}\" } }\n",
            &[("plain.md", "Plain.\n")],
        );
        assert!(success, "{stderr}");
        assert_eq!(launches, 2, "{stderr}");
    }

    /// The prompt's literal `choice` is its own, whatever the sequence's
    /// derived `choice` reads: its fixed, unavailable provider is refused
    /// before the writer runs.
    #[test]
    fn a_prompts_own_property_is_not_the_sequences_derived_one() {
        let (success, stderr, launches) = run(
            "kind: sequence\npick: goose\nchoice: \"{{ pick }}\"\nsequence:\n  - { name: writer, prompt: writes.md }\n  - { name: reader, prompt: reads.md }\n",
            &[("reads.md", "---\nchoice: claude\nagent: \"{{ choice }}\"\n---\nRead.\n")],
        );
        assert!(!success, "{stderr}");
        assert!(stderr.contains("Agent Not Installed:(Claude)"), "{stderr}");
        assert!(stderr.contains("No task has started"), "{stderr}");
        assert_eq!(launches, 0, "{stderr}");
    }
}
