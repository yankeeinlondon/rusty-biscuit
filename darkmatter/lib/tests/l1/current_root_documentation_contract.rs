//! AC29 positive documentation contract for the binding-time model of feature
//! `2026-09-09-more-context` (rulings R29–R33, decision D3). The sibling
//! `current_root_migration_guard` proves only that the removed nesting did not
//! return; this test proves the required pages actually explain the model:
//!
//! - the direct lazy-root spelling — `current.<key>` mirrors `ctx.<key>` and
//!   `current_env.<key>` mirrors `env.<key>`, with no nesting under either
//!   root;
//! - the variable/function pair rule, through its first pair
//!   (`ctx.recent_commits` and `recent_commits(count)`);
//! - the fixed-versus-refreshable distinction — repository metadata and
//!   topology are fixed by the request's repository observation, and only
//!   mutable Git and filesystem facts refresh;
//! - Claudine's run rule (fix `2026-09-20-lifecycle-handoff-gaps`, R1): `ctx`
//!   is evaluated once for each composition run, stated in the same words on
//!   every Claudine page that explains binding time, and `current` is
//!   observed once per lifecycle event.
//!
//! `PAGES` is the spec's "## Documentation" list, less the Claudine skill's
//! `lifecycle.md` and `composition.md`: those were snapshot copies, now
//! replaced by the skill's `topics/` symlink to the docs listed here. The spec says the pages
//! describe the model *collectively*, so each entry names only the clauses
//! that page is responsible for; the phrases are the page's own wording, so a
//! stray word cannot satisfy an entry and a rewrite that drops the claim
//! fails here. Whitespace is collapsed before matching so a reflowed
//! paragraph or a CRLF checkout does not break a phrase.

use std::path::{Path, PathBuf};

/// Claudine's per-composition-run rule, which every Claudine page that
/// explains binding time states in exactly these words.
const RUN_RULE: &str = "**`ctx` is evaluated once for each composition run.** A composition run is one document being composed and executed: a document invoked directly or adopted through `proxy`, each sequence step (each task of a serial group; a parallel group shares one capture until a sibling re-enters), each loop iteration, and each retry or resume attempt. Transclusion does not start a run: a document and every file it includes share one `ctx`. `current` is observed once per lifecycle event.";

/// Workspace-relative `/`-separated page path, phrases it must contain.
const PAGES: &[(&str, &[&str])] = &[
    (
        // The user page AC29 names for the eager/lazy pairing: every clause.
        "claudine/docs/topics/state-management/context-variables.md",
        &[
            "| `ctx.<key>` | Eager",
            "| `current.<key>` | Lazy — the same key as `ctx.<key>`",
            "| `current_env.<key>` | Lazy — the same key as `env.<key>`",
            "`current` does not contain a `ctx` member and `current_env` does not contain an `env` member",
            "`ctx.recent_commits` holds the last 10 commits captured at the start of the run; `recent_commits(count)`",
            "are fixed by the request's repository observation",
            "Only mutable Git and filesystem facts refresh from one event to the next",
            RUN_RULE,
        ],
    ),
    (
        ".claude/skills/claudine/cli-commands.md",
        &[
            "`current.<key>` is the same key as `ctx.<key>` read lazily",
            "`current_env.<key>` is the lazy mirror of `env.<key>`",
            "(`ctx.recent_commits` / `recent_commits(count)`) shares one definition",
            "are fixed by the request's repository observation, so `current.repo` always reads what `ctx.repo` does",
            "refresh between events",
            RUN_RULE,
        ],
    ),
    (
        ".claude/skills/darkmatter/compose.md",
        &[
            "`current.<key>` where `<key>` is cataloged",
            "`current_env.<KEY>` is `std::env::var` at reference time",
            "`ctx.recent_commits` is the eager snapshot of the same descriptor",
            "from the request's one repository observation (D3)",
        ],
    ),
    (
        "darkmatter/docs/topics/darkmatter-expressions.md",
        &[
            "| `current.*` | the same keys as `ctx.*`",
            "| `current_env.*` | the same keys as `env.*`",
            "`current` mirrors `ctx` key for key and `current_env` mirrors `env` key for key",
            "`ctx.recent_commits` holds the last 10, captured at the start of execution; `recent_commits(count)` returns the newest `count`, evaluated at call time",
            "are fixed by the request's repository observation, made once when the request is created",
        ],
    ),
    (
        "claudine/docs/topics/flow-control/lifecycle.md",
        &[
            "`current.<key>` for every `ctx.<key>`",
            "`current_env.<KEY>` for every `env.<KEY>`",
            "are fixed by the request's repository observation, so `current.repo` always reads what `ctx.repo` does",
        ],
    ),
    (
        "claudine/docs/topics/composition.md",
        &[
            "`current.<key>` observes a fact through the invocation's refresh capability once per lifecycle event",
            "`current_env.<KEY>` rereads the live process environment",
            "are fixed by the request's repository observation, so `current.repo` always reads what `ctx.repo` does",
            RUN_RULE,
        ],
    ),
];

fn workspace_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    let mut dir = manifest_dir.as_path();
    loop {
        let manifest = dir.join("Cargo.toml");
        if std::fs::read_to_string(&manifest).is_ok_and(|text| text.contains("[workspace]")) {
            return dir.to_path_buf();
        }
        dir = dir
            .parent()
            .expect("a workspace Cargo.toml above CARGO_MANIFEST_DIR");
    }
}

/// Runs of whitespace (including CRLF and LF) collapse to one space so a
/// phrase matches regardless of line wrapping or checkout line endings.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One line per defect: an unreadable page, or a page missing a phrase.
fn check(root: &Path, pages: &[(&str, &[&str])]) -> Vec<String> {
    let mut problems = Vec::new();
    for (page, phrases) in pages {
        let path = root.join(page.replace('/', std::path::MAIN_SEPARATOR_STR));
        let text = match std::fs::read(&path) {
            Ok(bytes) => collapse_whitespace(&String::from_utf8_lossy(&bytes)),
            Err(error) => {
                problems.push(format!("{page}: cannot read ({error})"));
                continue;
            }
        };
        for phrase in *phrases {
            if !text.contains(phrase) {
                problems.push(format!("{page}: missing {phrase:?}"));
            }
        }
    }
    problems
}

#[test]
fn every_required_page_explains_the_binding_time_model() {
    let problems = check(&workspace_root(), PAGES);
    assert!(
        problems.is_empty(),
        "AC29 documentation contract:\n  {}",
        problems.join("\n  ")
    );
}

/// The check names every missing phrase and every unreadable page, and
/// matches across line wrapping and CRLF, so the contract above cannot pass
/// vacuously.
#[test]
fn the_check_reports_each_missing_phrase_and_tolerates_reflow() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "docs/wrapped.md",
        "keys are fixed by the\r\nrequest's repository\n  observation, so\n",
    );
    write("docs/partial.md", "`current.<key>` mirrors `ctx.<key>`\n");
    let pages: &[(&str, &[&str])] = &[
        (
            "docs/wrapped.md",
            &["fixed by the request's repository observation"],
        ),
        (
            "docs/partial.md",
            &[
                "`current.<key>` mirrors `ctx.<key>`",
                "`recent_commits(count)`",
            ],
        ),
        ("docs/absent.md", &["anything"]),
    ];

    let problems = check(root.path(), pages);

    assert_eq!(problems.len(), 2, "{problems:?}");
    assert_eq!(
        problems[0],
        "docs/partial.md: missing \"`recent_commits(count)`\""
    );
    assert!(
        problems[1].starts_with("docs/absent.md: cannot read ("),
        "{}",
        problems[1]
    );
}
