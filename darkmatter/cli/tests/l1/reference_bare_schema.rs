//! The standalone reference commands resolve a bare `$schema` name from the
//! document's schema roots, as `md compose` does, and still refuse a name no
//! root holds; `--disable schema-triggers` leaves that lookup on.
//!
//! A note in the fixture home, outside every repository, refuses a bare name
//! and skips root triggers even when a repository document includes it:
//! schema-root eligibility belongs to each source document.

use crate::common;

use common::CliProcessFixture;

fn review_repository(name: &str, schema: &str) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/schemas/review.yaml", "$schema:\n  description: string(required)\n");
    fixture.write_file("cwd/sibling.md", "# Sibling\n");
    let input = fixture.write_file(
        "cwd/review.md",
        &format!("---\n$schema: {schema}\ndescription: Review heading\n---\n\n[sibling](./sibling.md)\n"),
    );
    (fixture, input)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn reference_commands_resolve_a_bare_schema_name() {
    let (fixture, input) = review_repository("reference_bare_schema", "review.yaml");

    let refs = fixture.command().args(["validate", "refs"]).arg(&input).args(["--output", "plain"]).output().unwrap();
    assert_eq!(refs.status.code(), Some(0), "{}{}", text(&refs.stdout), text(&refs.stderr));
    assert_eq!(text(&refs.stdout), "Checked 1 reference across 1 document: all ok\n");

    let graph = fixture.command().arg("graph").arg(&input).args(["--validate", "--output", "plain"]).output().unwrap();
    assert_eq!(graph.status.code(), Some(0), "{}{}", text(&graph.stdout), text(&graph.stderr));
    assert!(text(&graph.stdout).ends_with("Checked 1 reference across 1 document: all ok\n"), "{}", text(&graph.stdout));

    let schema = fixture
        .command()
        .args(["schema", "validate", "--disable", "schema-triggers"])
        .arg(&input)
        .args(["--output", "plain"])
        .output()
        .unwrap();
    assert_eq!(schema.status.code(), Some(0), "{}{}", text(&schema.stdout), text(&schema.stderr));
}

#[test]
fn reference_commands_refuse_a_schema_no_root_holds() {
    let (fixture, input) = review_repository("reference_missing_schema", "absent.yaml");

    for args in [&["validate", "refs"][..], &["graph", "--validate"][..]] {
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = text(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}{}", text(&output.stderr));
        assert!(stdout.contains("absent.yaml"), "{args:?}: {stdout}");
    }
}

/// A repository whose `root.md` includes `~/review.md`, a note in the fixture
/// home next to its `schemas/` root; the note's `$schema` is `schema`.
fn home_note(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    fixture.write_file("home/schemas/review.yaml", "$schema:\n  description: string(required)\n");
    fixture.write_file("cwd/root.md", "::file ~/review.md\n");
    fixture
}

fn write_home_note(fixture: &CliProcessFixture, frontmatter: &str) {
    fixture.write_file(
        "home/review.md",
        &format!("---\n{frontmatter}description: Review heading\n---\n\n# {{{{ fm.description }}}}\n"),
    );
}

#[test]
fn reference_commands_refuse_a_bare_schema_in_an_included_home_note() {
    let fixture = home_note("reference_home_note_schema");
    for (schema, accepted) in [("review.yaml", false), ("./schemas/review.yaml", true), ("absent.yaml", false)] {
        write_home_note(&fixture, &format!("$schema: '{schema}'\n"));
        for args in [&["validate", "refs"][..], &["graph", "--follow", "--validate"][..]] {
            let output = fixture.command().args(args).arg("root.md").args(["--output", "json"]).output().unwrap();
            let stdout = text(&output.stdout);
            if accepted {
                assert_eq!(output.status.code(), Some(0), "{schema} {args:?}: {stdout}{}", text(&output.stderr));
            } else {
                assert_eq!(output.status.code(), Some(1), "{schema} {args:?}: {stdout}{}", text(&output.stderr));
                assert!(stdout.contains(schema), "{schema} {args:?}: {stdout}");
            }
        }
    }
}

#[test]
fn compose_skips_root_triggers_and_bare_names_for_an_included_home_note() {
    let fixture = home_note("compose_home_note_schema");
    fixture.write_file(
        "home/schemas/gate.trigger.yaml",
        "kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  version: string(required)\n",
    );
    let explicit = fixture.home().join("schemas/review.yaml");
    write_home_note(&fixture, &format!("$schema: '{}'\ntopic: review\n", explicit.display()));
    let composed = fixture.command().args(["compose", "root.md"]).output().unwrap();
    assert_eq!(composed.status.code(), Some(0), "{}{}", text(&composed.stdout), text(&composed.stderr));
    assert!(text(&composed.stdout).contains("# Review heading"), "{}", text(&composed.stdout));

    write_home_note(&fixture, "$schema: review.yaml\n");
    let bare = fixture.command().args(["compose", "root.md"]).output().unwrap();
    assert_eq!(bare.status.code(), Some(1), "{}{}", text(&bare.stdout), text(&bare.stderr));

    // The same home trigger still applies to a child inside the repository.
    fixture.write_file("cwd/child.md", "---\ntopic: review\n---\n\n# In repository\n");
    fixture.write_file("cwd/inner.md", "::file ./child.md\n");
    let control = fixture.command().args(["compose", "inner.md"]).output().unwrap();
    assert_eq!(control.status.code(), Some(1), "{}{}", text(&control.stdout), text(&control.stderr));
    assert!(text(&control.stderr).contains("child.md"), "{}", text(&control.stderr));
}

// ── A note in a second checkout ───────────────────────────────────────────
//
// The launch checkout (the fixture's working directory) reaches
// `~/notes/review.md`, a note in a separate Git checkout whose own
// `schemas/` root holds its schema: every command resolves the note's bare
// `$schema` from that root, as when the note is opened directly.

/// The launch checkout's documents that reach the note, and the notes
/// checkout with its schema (an enum field for completion) and a
/// types-only library.
fn two_checkouts(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    assert!(fixture.initialize_repository_at(&fixture.home().join("notes")));
    fixture.write_file(
        "home/notes/schemas/review.yaml",
        "$schema:\n  description: string(required)\n  state: enum(open, done)\n",
    );
    fixture.write_file("home/notes/schemas/types.yaml", "kind: schema\ntypes:\n  entry: string\n");
    fixture.write_file("cwd/body.md", "::file ~/notes/review.md\n");
    fixture.write_file("cwd/prologue.md", "---\nprologue: ~/notes/review.md\n---\n\n# Root\n");
    fixture.write_file("cwd/epilogue.md", "---\nepilogue: ~/notes/review.md\n---\n\n# Root\n");
    fixture.write_file("cwd/link.md", "[go](~/notes/review.md#review-heading)\n");
    fixture.write_file("cwd/toc.md", "::toc-linking ~/notes/review.md\n");
    fixture
}

fn write_second_note(fixture: &CliProcessFixture, frontmatter: &str) {
    fixture.write_file(
        "home/notes/review.md",
        &format!("---\n{frontmatter}description: Review heading\n---\n\n# {{{{ fm.description }}}}\n\n## Static\n"),
    );
}

/// `md <args>` from the launch checkout: `(exit code, stdout and stderr)`.
fn run(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, String) {
    let output = fixture.command().args(args).output().unwrap();
    (output.status.code(), format!("{}{}", text(&output.stdout), text(&output.stderr)))
}

/// R7-F1.1–.5: one test per command that reaches the note. The bare schema
/// name succeeds; a missing one fails, naming `missing`.
macro_rules! second_checkout_command_tests {
    ($($test:ident => [$($arg:literal),+], $missing:literal);* $(;)?) => {$(
        #[test]
        fn $test() {
            let fixture = two_checkouts(stringify!($test));
            let args = [$($arg),+];
            write_second_note(&fixture, "$schema: review.yaml\n");
            let (code, output) = run(&fixture, &args);
            assert_eq!(code, Some(0), "bare {args:?}: {output}");
            write_second_note(&fixture, "$schema: absent.yaml\n");
            let (code, output) = run(&fixture, &args);
            assert_eq!(code, Some(1), "missing {args:?}: {output}");
            assert!(output.contains($missing), "missing {args:?}: {output}");
        }
    )*};
}

second_checkout_command_tests! {
    second_checkout_refs_of_a_body_inclusion => ["validate", "refs", "body.md"], "absent.yaml";
    second_checkout_refs_of_a_prologue => ["validate", "refs", "prologue.md"], "absent.yaml";
    second_checkout_refs_of_an_epilogue => ["validate", "refs", "epilogue.md"], "absent.yaml";
    second_checkout_mermaid_graph => ["graph", "body.md"], "absent.yaml";
    second_checkout_validated_graph => ["graph", "--validate", "body.md"], "absent.yaml";
    second_checkout_followed_graph => ["graph", "--follow", "--validate", "body.md"], "absent.yaml";
    second_checkout_compose => ["compose", "body.md"], "absent.yaml";
    second_checkout_compose_without_triggers => ["compose", "--disable", "schema-triggers", "body.md"], "absent.yaml";
    second_checkout_command_discovery => ["compose", "--shell", "body.md"], "absent.yaml";
    second_checkout_toc_target_compose => ["compose", "toc.md"], "absent.yaml";
    second_checkout_fragment_target => ["validate", "refs", "--fragments", "link.md"], "review-heading";
}

/// R7-F1.5: a bare `Name@file` import in the note resolves from the notes
/// checkout through the reference report and composition.
#[test]
fn second_checkout_bare_named_type_import_resolves() {
    let fixture = two_checkouts("second_checkout_import");
    write_second_note(&fixture, "$schema:\n  description: 'entry@types.yaml'\n");
    for args in [&["validate", "refs", "body.md"][..], &["compose", "body.md"][..]] {
        let (code, output) = run(&fixture, args);
        assert_eq!(code, Some(0), "{args:?}: {output}");
    }
}

/// R7-F1.6: `md compose` of a launch-checkout parent applies the notes
/// checkout's trigger to the note (its missing `version` is refused, and
/// nothing names the launch checkout's `parent_only` trigger); another topic
/// and `--disable schema-triggers` compose.
#[test]
fn second_checkout_compose_applies_the_notes_trigger() {
    let fixture = two_checkouts("second_checkout_trigger");
    let trigger = |field: &str| {
        format!("kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  {field}: string(required)\n")
    };
    fixture.write_file("home/notes/schemas/gate.trigger.yaml", &trigger("version"));
    fixture.write_file("cwd/schemas/gate.trigger.yaml", &trigger("parent_only"));
    let schema = fixture.home().join("notes/schemas/review.yaml");
    write_second_note(&fixture, &format!("$schema: '{}'\ntopic: review\n", schema.display()));
    let (code, output) = run(&fixture, &["compose", "body.md"]);
    assert_eq!(code, Some(1), "{output}");
    assert!(output.contains("version") && !output.contains("parent_only"), "{output}");
    let (code, output) = run(&fixture, &["compose", "--disable", "schema-triggers", "body.md"]);
    assert_eq!(code, Some(0), "{output}");

    write_second_note(&fixture, &format!("$schema: '{}'\ntopic: other\n", schema.display()));
    let (code, output) = run(&fixture, &["compose", "body.md"]);
    assert_eq!(code, Some(0), "{output}");
}

/// R7-F1.7 control: opened directly, the note's enum is offered by
/// completion for the bare spelling and not for a missing schema.
#[test]
fn second_checkout_note_completion_reads_its_bare_schema() {
    let fixture = two_checkouts("second_checkout_completion");
    for (schema, offered) in [("review.yaml", true), ("absent.yaml", false)] {
        write_second_note(&fixture, &format!("$schema: {schema}\n"));
        let output = fixture
            .command()
            .env("COMPLETE", "fish")
            .args(["--", "md", "compose", "~/notes/review.md", "state="])
            .output()
            .unwrap();
        assert!(output.status.success(), "{schema}: {output:?}");
        assert_eq!(text(&output.stdout).contains("state=open"), offered, "{schema}: {}", text(&output.stdout));
    }
}

// ── A nested checkout ─────────────────────────────────────────────────────
//
// `cwd/vendor` is a component checkout nested in the launch checkout, made
// as `git init --separate-git-dir` and submodules make one (a `.git` file
// naming a Git directory elsewhere); `~/notes/vendor` is a checkout nested in
// the separate `notes` checkout, beside an unrelated `~/notes/seed.md`. Git
// membership decides a document's repository, so each component resolves its
// schema from its own `schemas/` root whichever directory `md` is launched
// from and whichever note of the enclosing checkout is visited first.

/// The launch checkout with both nested components, each holding a review
/// schema (an enum field for completion and `md set`), a types-only
/// library, and an enclosing-checkout trigger that must never reach them.
fn nested_checkouts(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    let separate = fixture.workspace_path().join("component-git");
    assert!(fixture.initialize_repository_at(&separate));
    fixture.write_file("cwd/vendor/.git", &format!("gitdir: {}\n", separate.join(".git").display()));
    assert!(fixture.initialize_repository_at(&fixture.home().join("notes")));
    assert!(fixture.initialize_repository_at(&fixture.home().join("notes/vendor")));
    for component in ["cwd/vendor", "home/notes/vendor"] {
        fixture.write_file(
            format!("{component}/schemas/review.yaml"),
            "$schema:\n  description: string(required)\n  state: enum(open, done)\n",
        );
        fixture.write_file(format!("{component}/schemas/types.yaml"), "kind: schema\ntypes:\n  entry: string\n");
    }
    fixture.write_file("home/notes/seed.md", "# Seed\n");
    for (name, text) in [
        ("body.md", "::file ./vendor/review.md\n"),
        ("prologue.md", "---\nprologue: ./vendor/review.md\n---\n\n# Root\n"),
        ("epilogue.md", "---\nepilogue: ./vendor/review.md\n---\n\n# Root\n"),
        ("link.md", "[go](./vendor/review.md#review-heading)\n"),
        ("toc.md", "::toc-linking ./vendor/review.md\n"),
        ("seed_first.md", "::file ~/notes/seed.md\n\n::file ~/notes/vendor/review.md\n"),
        ("seed_last.md", "::file ~/notes/vendor/review.md\n\n::file ~/notes/seed.md\n"),
        ("seed_link.md", "[seed](~/notes/seed.md#seed)\n[go](~/notes/vendor/review.md#review-heading)\n"),
        ("seed_toc.md", "::file ~/notes/seed.md\n\n::toc-linking ~/notes/vendor/review.md\n"),
    ] {
        fixture.write_file(format!("cwd/{name}"), text);
    }
    fixture
}

/// The component directory (`cwd/vendor` or `home/notes/vendor`).
fn component(fixture: &CliProcessFixture, foreign: bool) -> std::path::PathBuf {
    if foreign { fixture.home().join("notes/vendor") } else { fixture.cwd().join("vendor") }
}

/// Writes the component's note with `frontmatter` and the review heading.
fn write_component_note(fixture: &CliProcessFixture, foreign: bool, frontmatter: &str) {
    let note = component(fixture, foreign).join("review.md");
    std::fs::write(
        note,
        format!("---\n{frontmatter}description: Review heading\n---\n\n# {{{{ fm.description }}}}\n\n## Static\n"),
    )
    .unwrap();
}

/// The three resolvable spellings of `file` in the component's `schemas/`
/// root, then a name no root holds.
fn component_spellings(fixture: &CliProcessFixture, foreign: bool, file: &str) -> [(String, bool); 4] {
    let absolute = component(fixture, foreign).join("schemas").join(file);
    [
        (file.to_owned(), true),
        (format!("./schemas/{file}"), true),
        (absolute.display().to_string(), true),
        ("absent.yaml".to_owned(), false),
    ]
}

/// `md <args>` launched in the enclosing checkout, or in the component.
fn run_in(fixture: &CliProcessFixture, in_component: bool, args: &[&str]) -> (Option<i32>, String) {
    let dir = component(fixture, false);
    let mut builder = fixture.command_builder();
    if in_component {
        builder = builder.ambient_context(&dir);
    }
    let output = builder.build().args(args).output().unwrap();
    (output.status.code(), format!("{}{}", text(&output.stdout), text(&output.stderr)))
}

/// R8-F1.1–.5, .8: one test per command, component, and launch directory.
/// Every resolvable spelling of the component's schema succeeds and the
/// missing one fails, naming `missing`.
macro_rules! nested_checkout_command_tests {
    ($($test:ident => foreign: $foreign:literal, in_component: $in_component:literal, [$($arg:literal),+], $missing:literal);* $(;)?) => {$(
        #[test]
        fn $test() {
            let fixture = nested_checkouts(stringify!($test));
            let args = [$($arg),+];
            for (schema, resolvable) in component_spellings(&fixture, $foreign, "review.yaml") {
                write_component_note(&fixture, $foreign, &format!("$schema: '{schema}'\n"));
                let (code, output) = run_in(&fixture, $in_component, &args);
                if resolvable {
                    assert_eq!(code, Some(0), "{schema} {args:?}: {output}");
                } else {
                    assert_eq!(code, Some(1), "{schema} {args:?}: {output}");
                    assert!(output.contains($missing), "{schema} {args:?}: {output}");
                }
            }
        }
    )*};
}

nested_checkout_command_tests! {
    nested_checkout_schema_validate_from_the_enclosing_checkout => foreign: false, in_component: false, ["schema", "validate", "vendor/review.md"], "absent.yaml";
    nested_checkout_schema_validate_from_the_component => foreign: false, in_component: true, ["schema", "validate", "review.md"], "absent.yaml";
    nested_checkout_compose_from_the_enclosing_checkout => foreign: false, in_component: false, ["compose", "vendor/review.md"], "absent.yaml";
    nested_checkout_compose_from_the_component => foreign: false, in_component: true, ["compose", "review.md"], "absent.yaml";
    nested_checkout_refs_from_the_enclosing_checkout => foreign: false, in_component: false, ["validate", "refs", "vendor/review.md"], "absent.yaml";
    nested_checkout_validated_graph_from_the_enclosing_checkout => foreign: false, in_component: false, ["graph", "--validate", "vendor/review.md"], "absent.yaml";
    nested_checkout_set_from_the_enclosing_checkout => foreign: false, in_component: false, ["set", "vendor/review.md", "state", "open"], "absent.yaml";
    nested_checkout_set_from_the_component => foreign: false, in_component: true, ["set", "review.md", "state", "open"], "absent.yaml";
    nested_checkout_clean_from_the_enclosing_checkout => foreign: false, in_component: false, ["clean", "vendor/review.md"], "absent.yaml";
    foreign_nested_checkout_schema_validate => foreign: true, in_component: false, ["schema", "validate", "~/notes/vendor/review.md"], "absent.yaml";
    nested_checkout_refs_of_a_body_inclusion => foreign: false, in_component: false, ["validate", "refs", "body.md"], "absent.yaml";
    nested_checkout_refs_of_a_prologue => foreign: false, in_component: false, ["validate", "refs", "prologue.md"], "absent.yaml";
    nested_checkout_refs_of_an_epilogue => foreign: false, in_component: false, ["validate", "refs", "epilogue.md"], "absent.yaml";
    nested_checkout_followed_graph => foreign: false, in_component: false, ["graph", "--follow", "--validate", "body.md"], "absent.yaml";
    nested_checkout_compose_of_a_body_inclusion => foreign: false, in_component: false, ["compose", "body.md"], "absent.yaml";
    nested_checkout_compose_without_triggers => foreign: false, in_component: false, ["compose", "--disable", "schema-triggers", "prologue.md"], "absent.yaml";
    nested_checkout_command_discovery => foreign: false, in_component: false, ["compose", "--shell", "epilogue.md"], "absent.yaml";
    nested_checkout_toc_target_compose => foreign: false, in_component: false, ["compose", "toc.md"], "absent.yaml";
    nested_checkout_fragment_target => foreign: false, in_component: false, ["validate", "refs", "--fragments", "link.md"], "review-heading";
    foreign_nested_checkout_refs_after_the_seed => foreign: true, in_component: false, ["validate", "refs", "seed_first.md"], "absent.yaml";
    foreign_nested_checkout_refs_before_the_seed => foreign: true, in_component: false, ["validate", "refs", "seed_last.md"], "absent.yaml";
    foreign_nested_checkout_compose_after_the_seed => foreign: true, in_component: false, ["compose", "seed_first.md"], "absent.yaml";
    foreign_nested_checkout_compose_before_the_seed => foreign: true, in_component: false, ["compose", "seed_last.md"], "absent.yaml";
    foreign_nested_checkout_command_discovery_after_the_seed => foreign: true, in_component: false, ["compose", "--shell", "seed_first.md"], "absent.yaml";
    foreign_nested_checkout_fragment_after_the_seed => foreign: true, in_component: false, ["validate", "refs", "--fragments", "seed_link.md"], "review-heading";
    foreign_nested_checkout_toc_after_the_seed => foreign: true, in_component: false, ["compose", "seed_toc.md"], "absent.yaml";
}

/// R8-F1.5: an inline `Name@file` import in the component note resolves
/// from the component's root for every resolvable spelling, directly from
/// the enclosing checkout, through an inclusion, and after the seed; a
/// missing library fails. One test per command, as each spawns `md` once per
/// spelling.
macro_rules! nested_checkout_import_tests {
    ($($test:ident => foreign: $foreign:literal, [$($arg:literal),+]);* $(;)?) => {$(
        #[test]
        fn $test() {
            let fixture = nested_checkouts(stringify!($test));
            let args = [$($arg),+];
            for (library, resolvable) in component_spellings(&fixture, $foreign, "types.yaml") {
                write_component_note(&fixture, $foreign, &format!("$schema:\n  description: 'entry@{library}'\n"));
                let (code, output) = run_in(&fixture, false, &args);
                assert_eq!(code, Some(if resolvable { 0 } else { 1 }), "{library} {args:?}: {output}");
            }
        }
    )*};
}

nested_checkout_import_tests! {
    nested_checkout_named_type_import_resolves_directly => foreign: false, ["schema", "validate", "vendor/review.md"];
    nested_checkout_named_type_import_resolves_in_references => foreign: false, ["validate", "refs", "body.md"];
    nested_checkout_named_type_import_resolves_in_composition => foreign: false, ["compose", "body.md"];
    foreign_nested_checkout_named_type_import_resolves_after_the_seed => foreign: true, ["compose", "seed_first.md"];
}

/// R8-F1.6: with an explicit schema path, a matching component note takes
/// its own checkout's trigger (`version`), never the enclosing checkout's
/// (`parent_only`), opened directly and through inclusions in both visit
/// orders; another topic and `--disable schema-triggers` compose.
fn assert_component_trigger(name: &str, foreign: bool, args: &[&str]) {
    let fixture = nested_checkouts(name);
    let trigger = |field: &str| {
        format!("kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  {field}: string(required)\n")
    };
    for dir in ["cwd/vendor", "home/notes/vendor"] {
        fixture.write_file(format!("{dir}/schemas/gate.trigger.yaml"), &trigger("version"));
    }
    for dir in ["cwd", "home/notes"] {
        fixture.write_file(format!("{dir}/schemas/gate.trigger.yaml"), &trigger("parent_only"));
    }
    write_component_note(&fixture, foreign, "$schema: './schemas/review.yaml'\ntopic: review\n");
    let (code, output) = run_in(&fixture, false, args);
    assert_eq!(code, Some(1), "{args:?}: {output}");
    assert!(output.contains("version") && !output.contains("parent_only"), "{args:?}: {output}");
    if args[0] == "compose" {
        let disabled: Vec<&str> = [&args[..1], &["--disable", "schema-triggers"][..], &args[1..]].concat();
        let (code, output) = run_in(&fixture, false, &disabled);
        assert_eq!(code, Some(0), "{disabled:?}: {output}");
    }
    write_component_note(&fixture, foreign, "$schema: './schemas/review.yaml'\ntopic: other\n");
    let (code, output) = run_in(&fixture, false, args);
    assert_eq!(code, Some(0), "other {args:?}: {output}");
}

macro_rules! nested_checkout_trigger_tests {
    ($($test:ident => foreign: $foreign:literal, [$($arg:literal),+]);* $(;)?) => {$(
        #[test]
        fn $test() {
            assert_component_trigger(stringify!($test), $foreign, &[$($arg),+]);
        }
    )*};
}

nested_checkout_trigger_tests! {
    nested_checkout_compose_applies_the_component_trigger => foreign: false, ["compose", "vendor/review.md"];
    nested_checkout_schema_validate_applies_the_component_trigger => foreign: false, ["schema", "validate", "vendor/review.md"];
    nested_checkout_inclusion_applies_the_component_trigger => foreign: false, ["compose", "body.md"];
    foreign_nested_checkout_applies_the_component_trigger_after_the_seed => foreign: true, ["compose", "seed_first.md"];
    foreign_nested_checkout_applies_the_component_trigger_before_the_seed => foreign: true, ["compose", "seed_last.md"];
}

/// R8-F1.7: Bash-style dynamic completion from the enclosing checkout offers
/// the component schema's enum for every resolvable spelling and nothing for
/// a missing schema.
#[test]
fn nested_checkout_completion_reads_the_component_schema() {
    let fixture = nested_checkouts("nested_checkout_completion");
    for (schema, resolvable) in component_spellings(&fixture, false, "review.yaml") {
        write_component_note(&fixture, false, &format!("$schema: '{schema}'\n"));
        let output = fixture
            .command()
            .env("COMPLETE", "bash")
            .env("_CLAP_COMPLETE_SPACE", "true")
            .env("_CLAP_COMPLETE_INDEX", "3")
            .args(["--", "md", "compose", "vendor/review.md", "state="])
            .output()
            .unwrap();
        assert!(output.status.success(), "{schema}: {output:?}");
        let offered = text(&output.stdout).lines().any(|line| line.trim() == "open");
        assert_eq!(offered, resolvable, "{schema}: {}", text(&output.stdout));
    }
}

/// R8-F1.7: `md schema detect` infers a `&` reference to a file at the
/// component's root as a file from the enclosing checkout, and a missing
/// one as a string.
#[test]
fn nested_checkout_schema_detect_sees_the_component_root() {
    let fixture = nested_checkouts("nested_checkout_detect");
    fixture.write_file("cwd/vendor/marker.md", "# Marker\n");
    for (target, kind) in [("&marker.md", "file"), ("&absent.md", "string")] {
        write_component_note(&fixture, false, &format!("attachment: '{target}'\n"));
        let (code, output) = run_in(&fixture, false, &["schema", "detect", "vendor/review.md"]);
        assert_eq!(code, Some(0), "{target}: {output}");
        assert!(output.contains(&format!("attachment: {kind}")), "{target}: {output}");
    }
}
