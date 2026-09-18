//! Corpus guard: no archive-executed target may resolve a path at compile time.
//!
//! Every hosted L1, L2, browser, and WSL2 cell runs binaries a *different* job
//! compiled, extracted somewhere the producer never wrote, against a checkout at
//! a different absolute path. `env!("CARGO_MANIFEST_DIR")` and
//! `env!("CARGO_BIN_EXE_…")` freeze the producer's directories into the binary,
//! so a test that reads either is green on every machine that builds it and red
//! only on the consumer. `biscuit_test_harness::manifest_dir!` and
//! `biscuit_test_harness::bin_exe!` ask the environment first —
//! `--workspace-remap` and `NEXTEST_BIN_EXE_*` are what the consumer rewrites.
//!
//! This scans the real repository, not a fixture. `ALLOWED` is the complete set
//! of files that may still bake a path in, each with the reason it is not
//! archive-executed; an entry that matches no live site fails too, so the list
//! burns down instead of becoming a grandfather table.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// A compile-time path form that does not survive relocation.
struct Form {
    /// Matched literally against the source, after comments are blanked.
    needle: &'static str,
    /// Named in the failure message, with what to use instead.
    remedy: &'static str,
}

const FORMS: &[Form] = &[
    Form {
        needle: "env!(\"CARGO_MANIFEST_DIR\")",
        remedy: "use `biscuit_test_harness::manifest_dir!()`",
    },
    Form {
        needle: "env!(\"CARGO_BIN_EXE_",
        remedy: "use `biscuit_test_harness::bin_exe!(\"<bin>\")`",
    },
];

/// A hosted runner's workspace root, which names the producer and nothing else.
///
/// Deliberately the only absolute-path form checked. `"/Users/…"` and
/// `"/Volumes/…"` are overwhelmingly synthetic *inputs* to pure path functions
/// (`document_within_workspace("/Users/ken/repo/x.md", …)`) rather than lookups,
/// and no textual rule separates the two; a check that flagged them would be
/// suppressed rather than obeyed. A baked producer path that survives this
/// guard is caught instead by the relocation fixtures in
/// `scripts/ci-build-archive-tests.rs`, which run with that path deleted.
const HOST_ROOTS: &[&str] = &["\"/home/runner/work/"];

struct AllowEntry {
    /// Repository-relative, `/`-separated.
    file: &'static str,
    reason: &'static str,
}

const ALLOWED: &[AllowEntry] = &[
    AllowEntry {
        file: "biscuit-test-harness/src/bin_exe.rs",
        reason: "defines the runtime lookups; the compile-time value is their documented fallback",
    },
    AllowEntry {
        file: "darkmatter/cli/src/commands/schema/about.rs",
        reason: "`include_str!` embeds the bytes at compile time, so the archived binary carries \
                 the document and opens no path at run time",
    },
    AllowEntry {
        file: "biscuit-icon/lib/src/bin/populate_assets.rs",
        reason: "dev-only `just populate-assets` bin with no test module; it regenerates \
                 committed assets from the checkout it is run in",
    },
    AllowEntry {
        file: "unchained-ai/gen/src/main.rs",
        reason: "`default_output_dir` for the manually-run `gen-models` bin; unreachable from \
                 that bin's own test module and from every test target",
    },
    AllowEntry {
        file: "unchained-ai/gen/src/bin/emit_catalog.rs",
        reason: "same, for the manually-run `emit-catalog` bin: it writes generated source back \
                 into the checkout it was launched from",
    },
];

/// Directories that are never archive-executed, and why.
///
/// Matched against any path component, so `examples` excludes every package's.
const SKIPPED_DIRS: &[(&str, &str)] = &[
    ("target", "build output"),
    (".git", "not source"),
    ("node_modules", "not Rust"),
    (".gitnexus", "index, not source"),
    (
        "scripts",
        "`repo-deps` runs from an archive like every other member, but its baked \
         `CARGO_MANIFEST_DIR` sites resolve on every hosted native consumer because those \
         share the producer's checkout path; `ci-rollup-tests.rs::repo_root` already falls \
         back to the run-time checkout, and the rest are the WSL2 leg's problem, not a \
         pull request's",
    ),
    (
        "examples",
        "examples are not test targets and nextest does not archive them",
    ),
    (
        "fuzz",
        "fuzz targets run on nightly from the checkout that built them",
    ),
];

fn repo_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .ancestors()
        .find(|dir| dir.join("just").join("devops.just").is_file())
        .map(Path::to_path_buf)
        .expect("could not locate the repo root")
}

/// Blank out `//` line comments so prose mentioning a form does not trip it.
///
/// Byte-for-byte length-preserving, so reported line numbers stay true. Block
/// comments and string literals are left alone: a form inside either is still a
/// form worth reporting, and the module docs above live in a file that is itself
/// excluded.
fn blank_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => {
                let mut kept = line[..at].to_owned();
                kept.push_str(&" ".repeat(line.len() - at));
                kept
            }
            None => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if SKIPPED_DIRS.iter().any(|(skip, _)| *skip == name) {
                continue;
            }
            collect_rust_files(&path, out);
        } else if name == "build.rs" {
            // A build script runs on the producer, where the compile-time value
            // is the only correct answer.
            continue;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("scanned path is below the repo root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

struct Violation {
    file: String,
    line: usize,
    remedy: &'static str,
}

fn scan(root: &Path) -> Vec<Violation> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for path in files {
        let file = relative(root, &path);
        // The guard's own constants spell every form it looks for.
        if file.ends_with("tests/archive_path_guard.rs") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in blank_line_comments(&source).lines().enumerate() {
            for form in FORMS {
                if line.contains(form.needle) {
                    violations.push(Violation {
                        file: file.clone(),
                        line: index + 1,
                        remedy: form.remedy,
                    });
                }
            }
            if HOST_ROOTS.iter().any(|root| line.contains(root)) {
                violations.push(Violation {
                    file: file.clone(),
                    line: index + 1,
                    remedy: "an absolute path to this host's checkout resolves nowhere else; \
                             build it from a runtime-resolved root",
                });
            }
        }
    }
    violations
}

#[test]
fn no_archive_executed_target_bakes_in_a_producer_path() {
    let root = repo_root();
    let violations = scan(&root);
    let allowed: BTreeSet<&str> = ALLOWED.iter().map(|entry| entry.file).collect();

    let offenders: Vec<&Violation> = violations
        .iter()
        .filter(|violation| !allowed.contains(violation.file.as_str()))
        .collect();

    assert!(
        offenders.is_empty(),
        "compile-time paths do not survive an archived run; {} site(s):\n{}\n\n\
         If a site is genuinely never archive-executed, add it to ALLOWED in \
         {} with a one-line reason.",
        offenders.len(),
        offenders
            .iter()
            .map(|violation| format!(
                "  {}:{} — {}",
                violation.file, violation.line, violation.remedy
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        file!(),
    );
}

#[test]
fn every_allowlist_entry_still_names_a_live_site() {
    let root = repo_root();
    let live: BTreeSet<String> = scan(&root)
        .into_iter()
        .map(|violation| violation.file)
        .collect();

    let stale: Vec<&AllowEntry> = ALLOWED
        .iter()
        .filter(|entry| !live.contains(entry.file))
        .collect();

    assert!(
        stale.is_empty(),
        "these ALLOWED entries no longer match any site and must be deleted:\n{}",
        stale
            .iter()
            .map(|entry| format!("  {} — {}", entry.file, entry.reason))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}
