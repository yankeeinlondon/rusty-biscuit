use super::*;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use claudine::diagnostics::Diagnostic;
use std::cell::Cell;
use std::fs;
use std::rc::Rc;
use tempfile::TempDir;

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn seed_repo(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
}

#[test]
fn operation_file_autocomplete_eligibility_is_grammar_driven() {
    for eligible in ["access", "access.md"] {
        assert!(
            is_operation_file_autocomplete_eligible(eligible),
            "expected eligible: {eligible:?}"
        );
    }

    for explicit in [
        "docs/access.md",
        r"docs\access.md",
        "./access.md",
        r".\access.md",
        "../access.md",
        r"..\access.md",
        "/tmp/access.md",
        r"C:\tmp\access.md",
        "C:/tmp/access.md",
        r"\\server\share\access.md",
        "//server/share/access.md",
        "~/access.md",
        r"~\access.md",
        "@access.md",
        "@/access.md",
        "&access.md",
        "^access.md",
        "vault:access.md",
        "vault::access.md",
        "https://example.com/access.md",
        "%access.md",
        "{{NAME}}",
        "prefix-{{NAME}}.md",
        "",
    ] {
        assert!(
            !is_operation_file_autocomplete_eligible(explicit),
            "expected ineligible: {explicit:?}"
        );
    }
    assert!(is_operation_file_autocomplete_eligible("literal}}.md"));
    assert!(!is_operation_file_autocomplete_eligible("literal{{.md"));
}

#[test]
fn operation_file_basename_is_separator_portable() {
    for (reference, expected) in [
        ("./docs/access.md", "access.md"),
        (r".\docs\access.md", "access.md"),
        (r"C:\docs\access.md", "access.md"),
        (r"\\server\share\access.md", "access.md"),
        ("@docs/access.md", "access.md"),
        ("&access.md", "access.md"),
        ("^/docs/access.md", "access.md"),
        ("vault:notes/access.md", "access.md"),
        ("%./docs/access.md", "access.md"),
    ] {
        assert_eq!(operation_file_basename(reference), Some(OsStr::new(expected)));
    }
    assert_eq!(operation_file_basename("~/"), None);
    assert_eq!(operation_file_basename("https://example.com/access.md"), None);
}

#[test]
fn repository_suggestions_match_exact_filename_and_sort() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("z/access.md"), "z");
    write(&tmp.path().join("a/access.md"), "a");
    write(&tmp.path().join("b/ACCESS.md"), "b");

    assert_eq!(
        repository_basename_suggestions(Some(tmp.path()), Some(OsStr::new("access.md"))),
        vec!["a/access.md", "z/access.md"]
    );
}

#[test]
fn repository_suggestion_empty_iterator_returns_empty() {
    assert!(
        collect_repository_suggestions(
            OsStr::new("access.md"),
            std::iter::empty::<SuggestionWalkItem>(),
        )
        .is_empty()
    );
}

#[test]
fn repository_suggestions_are_deduplicated_and_capped_at_five() {
    let entries = [
        "a/access.md",
        "a/access.md",
        "b/access.md",
        "c/access.md",
        "d/access.md",
        "e/access.md",
        "f/access.md",
        "z/access.md",
    ]
    .into_iter()
    .map(|path| {
        SuggestionWalkItem::Entry(SuggestionEntry {
            relative_path: PathBuf::from(path),
            file_name: OsString::from("access.md"),
            is_file: true,
        })
    })
    .chain(std::iter::once(SuggestionWalkItem::Error));

    assert_eq!(
        collect_repository_suggestions(OsStr::new("access.md"), entries),
        vec![
            "a/access.md",
            "b/access.md",
            "c/access.md",
            "d/access.md",
            "e/access.md"
        ]
    );
}

#[test]
fn repository_suggestion_raw_walk_budget_includes_root_errors_and_entries() {
    let visits = Rc::new(Cell::new(0));
    let observed_visits = Rc::clone(&visits);
    let entries = std::iter::from_fn(move || {
        let index = visits.get();
        visits.set(index + 1);
        assert!(
            index < SUGGESTION_ENTRY_BUDGET,
            "collector polled item 20,001"
        );

        if index == 0 {
            return Some(SuggestionWalkItem::Root);
        }
        if index == 1 {
            return Some(SuggestionWalkItem::Error);
        }
        Some(SuggestionWalkItem::Entry(SuggestionEntry {
            relative_path: PathBuf::from(format!("dir-{index}/access.md")),
            file_name: OsString::from(if index + 1 == SUGGESTION_ENTRY_BUDGET {
                "access.md"
            } else {
                "not-access.md"
            }),
            is_file: true,
        }))
    });

    assert_eq!(
        collect_repository_suggestions(OsStr::new("access.md"), entries),
        vec![format!(
            "dir-{}/access.md",
            SUGGESTION_ENTRY_BUDGET - 1
        )]
    );
    assert_eq!(observed_visits.get(), SUGGESTION_ENTRY_BUDGET);
}

#[test]
fn repository_suggestion_first_error_keeps_later_match() {
    let entries = [
        SuggestionWalkItem::Error,
        SuggestionWalkItem::Entry(SuggestionEntry {
            relative_path: PathBuf::from("a/access.md"),
            file_name: OsString::from("access.md"),
            is_file: true,
        }),
    ];
    assert_eq!(
        collect_repository_suggestions(OsStr::new("access.md"), entries),
        vec!["a/access.md"]
    );
}

#[test]
fn repository_suggestion_middle_error_keeps_later_match() {
    let entries = [
        SuggestionWalkItem::Entry(SuggestionEntry {
            relative_path: PathBuf::from("a/other.md"),
            file_name: OsString::from("other.md"),
            is_file: true,
        }),
        SuggestionWalkItem::Error,
        SuggestionWalkItem::Entry(SuggestionEntry {
            relative_path: PathBuf::from("z/access.md"),
            file_name: OsString::from("access.md"),
            is_file: true,
        }),
    ];
    assert_eq!(
        collect_repository_suggestions(OsStr::new("access.md"), entries),
        vec!["z/access.md"]
    );
}

#[test]
fn repository_suggestions_reuse_completion_tree_filters() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join(".gitignore"), "ignored/\n");
    write(&tmp.path().join("visible/access.md"), "visible");
    write(&tmp.path().join("ignored/access.md"), "ignored");
    write(&tmp.path().join("_drafts/access.md"), "draft");
    write(&tmp.path().join("target/access.md"), "target");

    assert_eq!(
        repository_basename_suggestions(Some(tmp.path()), Some(OsStr::new("access.md"))),
        vec!["visible/access.md"]
    );
}

#[test]
fn repository_suggestions_handle_absent_inputs_and_no_hits() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("docs/other.md"), "other");

    assert!(repository_basename_suggestions(None, Some(OsStr::new("access.md"))).is_empty());
    assert!(repository_basename_suggestions(Some(tmp.path()), None).is_empty());
    assert!(
        repository_basename_suggestions(Some(tmp.path()), Some(OsStr::new("access.md"))).is_empty()
    );
    assert!(
        repository_basename_suggestions(
            Some(&tmp.path().join("missing")),
            Some(OsStr::new("access.md"))
        )
        .is_empty()
    );
}

#[test]
fn recovery_attempts_autocomplete_only_for_bare_no_match() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let context = biscuit_file::FileResolutionContext::new(tmp.path())
        .with_repository_root(tmp.path());
    let detailed = FileReference::new("access.md")
        .unwrap()
        .resolve_detailed(&context);
    let error = CompositionError::from_detailed_no_match(&detailed);

    assert!(matches!(
        recover_operation_file("access.md", error),
        OperationFileRecovery::AttemptAutocomplete
    ));
}

#[test]
fn recovery_enriches_explicit_no_match_without_selecting_suggestion() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("homelab/docs/unifi/apps/protect.md"),
        "apps",
    );
    write(
        &tmp
            .path()
            .join("homelab/docs/unifi/service-offerings/protect.md"),
        "services",
    );
    for directory in ["alpha", "beta", "gamma", "zeta"] {
        write(
            &tmp.path().join(directory).join("protect.md"),
            directory,
        );
    }
    let context = biscuit_file::FileResolutionContext::new(tmp.path())
        .with_repository_root(tmp.path());
    let detailed = FileReference::new("./docs/unifi/protect.md")
        .unwrap()
        .resolve_detailed(&context);
    let error = CompositionError::from_detailed_no_match(&detailed);

    let OperationFileRecovery::ExplicitNoMatch(error) =
        recover_operation_file("./docs/unifi/protect.md", error)
    else {
        panic!("explicit reference must not enter autocomplete");
    };
    let (reference, resolution, suggestions) = error.file_reference_no_match().unwrap();
    let expected = [
        "alpha/protect.md",
        "beta/protect.md",
        "gamma/protect.md",
        "homelab/docs/unifi/apps/protect.md",
        "homelab/docs/unifi/service-offerings/protect.md",
    ];
    assert_eq!(reference, "./docs/unifi/protect.md");
    assert_eq!(suggestions, expected);
    assert_eq!(error.code(), "composition.invalid_file_reference");

    let detail = error.detail();
    assert_eq!(detail["failure"], "no_match");
    assert_eq!(detail["suggestions"], serde_json::json!(expected));
    assert_eq!(detail["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(resolution.candidates().len(), 1);
    assert_eq!(detail["candidates"][0]["provenance"], "source");
    assert_eq!(detail["candidates"][0]["disposition"], "missing");

    let rendered =
        strip_escape_codes(error.report_block_error(&Terminal::new_optimistic(100)));
    assert!(rendered.contains("Did you mean:"), "got:\n{rendered}");
    for suggestion in expected {
        assert!(rendered.contains(suggestion), "got:\n{rendered}");
    }
    assert!(!rendered.contains("zeta/protect.md"), "got:\n{rendered}");
}

#[test]
fn recovery_does_not_reclassify_non_no_match_errors() {
    let error = CompositionError::FileNotFound("access.md".to_string());
    assert!(matches!(
        recover_operation_file("access.md", error),
        OperationFileRecovery::ExplicitNoMatch(CompositionError::FileNotFound(reference))
            if reference == "access.md"
    ));
}

#[cfg(unix)]
#[test]
fn repository_suggestions_do_not_follow_directory_symlinks() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    seed_repo(repo.path());
    write(&external.path().join("access.md"), "external");
    symlink(external.path(), repo.path().join("linked")).unwrap();

    assert!(
        repository_basename_suggestions(Some(repo.path()), Some(OsStr::new("access.md"))).is_empty()
    );
}

#[cfg(unix)]
#[test]
fn repository_suggestions_skip_earlier_dangling_symlink() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join(".aaa")).unwrap();
    symlink("missing", repo.path().join(".aaa/broken")).unwrap();
    write(&repo.path().join("zzz/access.md"), "match");

    assert_eq!(
        repository_basename_suggestions(Some(repo.path()), Some(OsStr::new("access.md"))),
        vec!["zzz/access.md"]
    );
}

#[cfg(windows)]
#[test]
fn repository_suggestions_do_not_follow_directory_symlinks() {
    use std::os::windows::fs::symlink_dir;

    let repo = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    seed_repo(repo.path());
    write(&external.path().join("access.md"), "external");
    if symlink_dir(external.path(), repo.path().join("linked")).is_err() {
        return;
    }

    assert!(
        repository_basename_suggestions(Some(repo.path()), Some(OsStr::new("access.md"))).is_empty()
    );
}
