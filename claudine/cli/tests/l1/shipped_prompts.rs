use crate::common;
use common::lifecycle_set_corpus::mapping_only_set_findings;
use common::prompt_staging::workspace_root;
use common::write;

/// Claudine's own lifecycle artifacts. The repository's internal `prompts/`
/// get the same check from the opt-in `prompts` binary, never from CI.
#[test]
fn shipped_lifecycle_artifacts_use_mapping_only_set() {
    let root = workspace_root();
    let (mapping_witnesses, defects) = mapping_only_set_findings(&[
        root.join("claudine/cli/tests/fixtures"),
        root.join("claudine/gen/tests/fixtures"),
        root.join("claudine/schemas"),
        root.join("claudine/docs/schemas"),
        root.join("darkmatter/dmls/tests/fixtures/sequence_descent"),
    ]);

    assert!(
        mapping_witnesses > 0,
        "the corpus must contain a lifecycle `set` mapping witness"
    );
    assert!(
        defects.is_empty(),
        "shipped artifacts with removed or invalid lifecycle `set` syntax:\n{}",
        defects.join("\n")
    );
}

/// Staging follows `::file` directives through every spelling the shipped
/// prompts use, so a fragment added to a prompt reaches each test that stages
/// it without a hand-kept snippet list.
#[test]
fn staging_a_prompt_brings_every_file_it_transcludes() {
    let repository = tempfile::tempdir().expect("temp dir");
    let staged = tempfile::tempdir().expect("temp dir");
    for (path, content) in [
        (
            "_implement/entry.md",
            "::file \"../_quoted.md\"\n    ::file ../_bare.md\n::file {{dynamic}}\n::filed not-a-directive.md\n",
        ),
        ("_quoted.md", "::file ./_nested/leaf.md actor={{actor}}\n"),
        ("_bare.md", "bare\n"),
        ("_nested/leaf.md", "leaf\n"),
        ("_unreached.md", "unreached\n"),
    ] {
        write(&repository.path().join("prompts").join(path), content);
    }

    let paths = common::prompt_staging::stage_shipped_prompts(
        repository.path(),
        staged.path(),
        &["prompts/_implement/entry.md"],
    );

    assert_eq!(
        paths,
        ["_bare.md", "_implement/entry.md", "_nested/leaf.md", "_quoted.md"]
    );
    assert_eq!(
        std::fs::read_to_string(staged.path().join("_nested/leaf.md")).unwrap(),
        "leaf\n"
    );
    assert!(!staged.path().join("_unreached.md").exists());
}
