//! AC29 / R37 migration guard for the nesting the clean break removed
//! (feature `2026-09-09-more-context`, ruling R33): `current.ctx.` and
//! `current.env.` may appear only where `ALLOWLIST` expects them.
//!
//! Scope rule. Starting at the workspace root — the nearest ancestor of this
//! crate's `CARGO_MANIFEST_DIR` whose `Cargo.toml` declares `[workspace]` —
//! every regular file is scanned whose extension is `rs`, `md`, `toml`,
//! `yaml`, `yml`, or `json`, plus every regular file under a directory named
//! `prompts` regardless of extension. That covers code and tests, shipped
//! prompts, schemas and templates, user docs, READMEs, and the
//! `.claude/skills` tree. Directories named `target`, `node_modules`,
//! `features`, or `fixes` are skipped at any depth, as is every dot-directory
//! other than `.claude` (`.git`, `.gitnexus`, ...), and the repository's
//! internal prompt folders (`INTERNAL_PROMPTS`), which are tools CI never
//! tests; symlinks are never followed. Skipping `features/` and `fixes/`
//! excludes planning documents — specs, plans, reviews, logs, decisions, and
//! their `_completed` and `_unscheduled` lifecycle directories — which are
//! neither code, shipped prompts, user docs, nor skills; every in-flight spec
//! the migration list names lives there.
//!
//! Each occurrence of either literal counts once, so a line naming both
//! spellings counts twice. The scan must equal `ALLOWLIST` exactly: a file
//! outside it with any occurrence is a new use of the removed spelling, and
//! an entry whose count moved is stale. An allowlisted occurrence is a
//! negative test proving the spelling is rejected, or documentation / error
//! text explaining the removal; the one exception is annotated inline. This
//! file lists itself because its documentation names the spellings.
//!
//! Git answers which tracked files hold no occurrence, so the scan opens only
//! the rest; without a repository it opens every file. Either way the result
//! is the same, which `the_git_prefilter_skips_only_clean_tracked_files_without_a_needle`
//! checks.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const NEEDLES: [&str; 2] = ["current.ctx.", "current.env."];
const EXTENSIONS: [&str; 6] = ["rs", "md", "toml", "yaml", "yml", "json"];
const SKIPPED_DIRS: [&str; 4] = ["target", "node_modules", "features", "fixes"];
/// Workspace-relative internal prompt folders. A prompt may be a draft, and
/// editing one must never fail the normal suite.
const INTERNAL_PROMPTS: [&str; 3] = ["prompts", "darkmatter/prompts", "claudine/prompts"];

/// Workspace-relative `/`-separated path, expected occurrence count.
const ALLOWLIST: &[(&str, usize)] = &[
    // Skills explaining the clean break (R29–R33).
    (".claude/skills/claudine/cli-commands.md", 2),
    (".claude/skills/darkmatter/compose.md", 2),
    // Exception: a Rust field chain, `current.env.push(..)` on a local named
    // `current`, not the document spelling. The literal scan cannot tell
    // them apart.
    ("claudine/cli/tests/level2/level2_lifecycle_control.rs", 1),
    // Negative test: `{{current.ctx.branch}}` fails under Claudine.
    ("claudine/lib/src/composition/lifecycle/context/tests.rs", 2),
    // Negative test through the `md` binary.
    ("darkmatter/cli/tests/l1/compose_lazy_roots.rs", 2),
    // User docs: "There is no nesting".
    ("darkmatter/docs/topics/darkmatter-expressions.md", 2),
    // Negative test: the nesting names no capability and no eager requirement.
    ("darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs", 2),
    // Doc comment on the eager-requirement scanner.
    ("darkmatter/lib/src/markdown/compose/context/capture/groups.rs", 1),
    // Doc comment plus negative test on the lazy-root member lookup.
    ("darkmatter/lib/src/markdown/compose/context/current.rs", 4),
    // `LazyRootMemberUnknown` docs and error text.
    ("darkmatter/lib/src/markdown/compose/expression/error.rs", 4),
    // Negative test through the compose pipeline.
    ("darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs", 2),
    // This guard: the module doc, `NEEDLES`, the annotations above, and the
    // fixtures below.
    ("darkmatter/lib/tests/l1/current_root_migration_guard.rs", 24),
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

fn is_skipped_dir(name: &str) -> bool {
    SKIPPED_DIRS.contains(&name) || (name.starts_with('.') && name != ".claude")
}

fn is_internal_prompts(root: &Path, dir: &Path) -> bool {
    dir.strip_prefix(root).is_ok_and(|relative| {
        INTERNAL_PROMPTS.iter().any(|prompts| relative == Path::new(prompts))
    })
}

fn has_scanned_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| EXTENSIONS.contains(&extension))
}

/// 1-based line of every occurrence, per file, keyed by `/`-separated path
/// relative to `root`. Files without an occurrence are absent.
fn scan(root: &Path) -> BTreeMap<String, Vec<usize>> {
    scan_skipping(root, &clean_tracked_files_without_a_needle(root))
}

/// [`scan`] that opens every scoped file except those keyed in `skippable`.
fn scan_skipping(root: &Path, skippable: &HashSet<String>) -> BTreeMap<String, Vec<usize>> {
    scoped_files(root)
        .iter()
        .map(|path| (key(root, path), path))
        .filter(|(key, _)| !skippable.contains(key))
        .filter_map(|(key, path)| occurrences(path).map(|lines| (key, lines)))
        .collect()
}

/// Every regular file the scope rule admits.
fn scoped_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![(root.to_path_buf(), false)];
    while let Some((dir, under_prompts)) = pending.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry.file_type().expect("file type");
            if kind.is_dir() {
                if !is_skipped_dir(&name) && !is_internal_prompts(root, &path) {
                    pending.push((path, under_prompts || name == "prompts"));
                }
            } else if kind.is_file() && (under_prompts || has_scanned_extension(&path)) {
                files.push(path);
            }
        }
    }
    files
}

/// Keys of tracked files whose working-tree bytes Git reports free of every
/// needle, so the scan need not open them; empty when Git cannot answer
/// (no repository, as in an extracted CI archive), which makes the scan read
/// every file.
///
/// A test process's first open of each file costs about 0.8 ms on the native
/// Windows rig, about 8 s over this corpus, while a single-threaded
/// `git grep` reads the same bytes in about 2 s. Only entries `ls-files -v`
/// tags `H` qualify: Git greps the index blob rather than the working file
/// for assume-unchanged and skip-worktree entries, so those are read here.
/// The pathspecs only narrow both commands to a superset of the scope rule;
/// a scoped file they omit is read.
fn clean_tracked_files_without_a_needle(root: &Path) -> HashSet<String> {
    const PATHSPECS: [&str; 11] = [
        ":(glob)**/*.rs",
        ":(glob)**/*.md",
        ":(glob)**/*.toml",
        ":(glob)**/*.yaml",
        ":(glob)**/*.yml",
        ":(glob)**/*.json",
        ":(glob)**/prompts/**",
        ":(exclude,glob)**/target/**",
        ":(exclude,glob)**/node_modules/**",
        ":(exclude,glob)**/features/**",
        ":(exclude,glob)**/fixes/**",
    ];
    let paths = |output: Vec<u8>| -> Vec<String> {
        output
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect()
    };
    let Some(listed) = git(root, &["ls-files", "-z", "-v", "--"], &PATHSPECS) else {
        return HashSet::new();
    };
    // `grep.fullName` would print paths from the top of the repository
    // rather than relative to `root`, as `ls-files` does.
    let mut grep = vec!["-c", "grep.fullName=false", "grep", "--threads", "1", "--no-color", "--no-textconv", "-a", "-l", "-z", "-F"];
    for needle in NEEDLES {
        grep.extend(["-e", needle]);
    }
    grep.push("--");
    let Some(matching) = git(root, &grep, &PATHSPECS) else {
        return HashSet::new();
    };
    let matching: HashSet<String> = paths(matching).into_iter().collect();
    paths(listed)
        .into_iter()
        .filter_map(|entry| entry.strip_prefix("H ").map(str::to_string))
        .filter(|path| !matching.contains(path))
        .collect()
}

/// Stdout of `git <args> <pathspecs>` run in `root`, or `None` unless it exits
/// 0 or, for `grep`, 1 (no match). Inherited repository variables are
/// dropped so a hook's `GIT_DIR` cannot point the command elsewhere.
fn git(root: &Path, args: &[&str], pathspecs: &[&str]) -> Option<Vec<u8>> {
    let output = std::process::Command::new("git")
        .args(args)
        .args(pathspecs)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    match output.status.code() {
        Some(0) => Some(output.stdout),
        Some(1) if args.contains(&"grep") => Some(Vec::new()),
        _ => None,
    }
}

fn key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("under the root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The occurrence lines of `path`, or `None` when it has none.
fn occurrences(path: &Path) -> Option<Vec<usize>> {
    let text = std::fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let text = String::from_utf8_lossy(&text);
    // Almost every file has no occurrence; skip the per-line pass for them.
    if !NEEDLES.iter().any(|needle| text.contains(needle)) {
        return None;
    }
    let lines: Vec<usize> = text
        .lines()
        .enumerate()
        .flat_map(|(index, line)| {
            let hits: usize = NEEDLES.iter().map(|needle| line.matches(needle).count()).sum();
            std::iter::repeat_n(index + 1, hits)
        })
        .collect();
    (!lines.is_empty()).then_some(lines)
}

#[test]
fn the_removed_nesting_appears_only_where_the_allowlist_expects() {
    let allowed: BTreeMap<&str, usize> = ALLOWLIST.iter().copied().collect();
    assert_eq!(allowed.len(), ALLOWLIST.len(), "duplicate allowlist entry");

    let found = scan(&workspace_root());
    let mut problems = Vec::new();
    for (path, lines) in &found {
        match allowed.get(path.as_str()) {
            None => problems.push(format!("new occurrence outside the allowlist: {path} at lines {lines:?}")),
            Some(&expected) if expected != lines.len() => problems.push(format!(
                "stale allowlist count for {path}: expected {expected}, found {} at lines {lines:?}",
                lines.len()
            )),
            Some(_) => {}
        }
    }
    for (path, expected) in ALLOWLIST {
        if !found.contains_key(*path) {
            problems.push(format!("stale allowlist entry: {path} expects {expected} occurrences but has none"));
        }
    }
    assert!(problems.is_empty(), "AC29 migration guard:\n  {}", problems.join("\n  "));
}

/// The scanner counts every occurrence and honors the scope rule, so the
/// guard above cannot pass vacuously.
#[test]
fn the_scan_counts_occurrences_and_honors_the_scope_rule() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("src/a.rs", "let x = \"{{ current.ctx.a }} {{ current.env.B }}\";\n\n// current.ctx.c\n");
    write("pkg/prompts/format.tmpl", "{{ current.env.HOME }}\n");
    write("prompts/draft.md", "current.ctx.ignored: internal prompt\n");
    write("claudine/prompts/draft.md", "current.env.ignored: internal prompt\n");
    write(".claude/skills/topic/SKILL.md", "no `current.ctx.x` nesting\n");
    write("notes.txt", "current.ctx.ignored: extension out of scope\n");
    write("features/2026-01-01-x/spec.md", "current.ctx.ignored: planning document\n");
    write("fixes/_completed/y/spec.md", "current.env.ignored: planning document\n");
    write("target/debug/build.rs", "current.ctx.ignored: build output\n");
    write(".git/COMMIT_EDITMSG", "current.ctx.ignored: dot-directory\n");
    write("clean.md", "current_env.HOME and ctx.current are fine\n");

    let found = scan(root.path());

    assert_eq!(
        found,
        BTreeMap::from([
            (".claude/skills/topic/SKILL.md".to_string(), vec![1]),
            ("pkg/prompts/format.tmpl".to_string(), vec![1]),
            ("src/a.rs".to_string(), vec![1, 1, 3]),
        ]),
    );
}

/// Git's answer only ever skips a tracked file whose working-tree bytes hold
/// no occurrence, and the scan agrees with one that opens every file. The
/// working-tree edits after `git add` and the assume-unchanged and
/// skip-worktree entries are the cases where the index and the file differ.
#[test]
fn the_git_prefilter_skips_only_clean_tracked_files_without_a_needle() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root.path())
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", root.path().join("absent-global-config"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}: {status}");
    };
    write("tracked.rs", "// current.ctx.a\n");
    write("clean.md", "nothing here\n");
    write("edited.md", "nothing yet\n");
    write("skip_worktree.rs", "nothing yet\n");
    write("assumed.rs", "nothing yet\n");
    write("features/x/spec.md", "current.env.ignored: planning document\n");
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["update-index", "--skip-worktree", "skip_worktree.rs"]);
    git(&["update-index", "--assume-unchanged", "assumed.rs"]);
    write("edited.md", "current.env.A\n");
    write("skip_worktree.rs", "current.ctx.b\n");
    write("assumed.rs", "current.env.C\n");
    write("untracked.toml", "current.ctx.d\n");

    let skippable = clean_tracked_files_without_a_needle(root.path());
    let scanned = scan(root.path());

    assert_eq!(skippable, HashSet::from(["clean.md".to_string()]));
    let expected = BTreeMap::from(
        ["assumed.rs", "edited.md", "skip_worktree.rs", "tracked.rs", "untracked.toml"].map(|key| (key.to_string(), vec![1])),
    );
    assert_eq!(scanned, expected);
    assert_eq!(scan_skipping(root.path(), &HashSet::new()), expected);
}
